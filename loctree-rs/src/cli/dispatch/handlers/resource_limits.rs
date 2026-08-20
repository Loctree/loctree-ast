//! Resource ceilings and bounded readers for cache/doctor enumeration.
//!
//! Audit class H (independent reproduction 2026-07-21, probe LCT-E01):
//! `loct doctor --cache --scope --json` on a temporary clone walked the
//! *global* cache, `fs::read` every `snapshot.json` fully into memory and
//! peaked at 20.8 GiB RSS in 54 s — more than ten times the declared 2 GiB
//! probe ceiling. Two structural causes, both fixed here:
//!
//! 1. Whole-file reads: snapshot metadata was obtained by loading the whole
//!    `snapshot.json` (potentially hundreds of MB per bucket) into a `Vec<u8>`
//!    plus a full serde DOM parse. [`read_snapshot_metadata_bounded`] replaces
//!    that with a streaming `from_reader` parse (memory bounded by the largest
//!    single JSON token) behind a hard file-size ceiling.
//! 2. Unbounded walks: per-bucket `walkdir` size sampling had no entry or
//!    time cap. [`bounded_dir_size`] caps both and reports truncation
//!    honestly instead of silently pretending completeness.
//!
//! The constants below are the product contract. `DOCTOR_RSS_CEILING_BYTES`
//! and `DOCTOR_WALL_CEILING_SECS` mirror the audit acceptance criteria
//! ("temp-clone doctor < 2 GiB and < 120 s") and are enforced end-to-end by
//! `loctree-rs/tests/cache_rss_bounds.rs`.

use std::fs;
use std::io::BufReader;
use std::path::Path;
use std::time::{Duration, Instant};

use serde::Deserialize;

use crate::snapshot::SnapshotMetadata;

/// Snapshots larger than this are not parsed for metadata during cache
/// enumeration; the entry is reported with an explicit oversize note instead
/// of stalling the whole inventory. Streaming keeps RSS flat regardless, so
/// this is a *time* guard, not a memory guard.
pub const MAX_SNAPSHOT_METADATA_PARSE_BYTES: u64 = 256 * 1024 * 1024;

/// Wall-clock ceiling for one cache enumeration pass (list / doctor).
/// When exceeded, remaining buckets are skipped and the output says so.
pub const CACHE_ENUM_TIME_CEILING: Duration = Duration::from_secs(30);

/// Maximum filesystem entries walked per cache bucket when sampling its
/// size. Beyond this the size is reported as a lower bound (`≥`).
pub const MAX_WALK_ENTRIES_PER_BUCKET: u64 = 250_000;

/// Output ceiling: maximum table rows a global `cache list` prints before
/// truncating with an explicit "omitted" note.
pub const MAX_LIST_ROWS: usize = 200;

/// Audit acceptance ceiling (LCT-E01): peak RSS for a doctor run.
/// Enforced by the test harness in `tests/cache_rss_bounds.rs`.
pub const DOCTOR_RSS_CEILING_BYTES: u64 = 2 * 1024 * 1024 * 1024;

/// Audit acceptance ceiling (LCT-E01): wall clock for a doctor run.
/// Enforced by the test harness in `tests/cache_rss_bounds.rs`.
pub const DOCTOR_WALL_CEILING_SECS: u64 = 120;

/// Wall-clock budget carried through one enumeration pass.
#[derive(Debug)]
pub struct EnumerationBudget {
    started: Instant,
    ceiling: Duration,
}

impl EnumerationBudget {
    pub fn start(ceiling: Duration) -> Self {
        Self {
            started: Instant::now(),
            ceiling,
        }
    }

    pub fn exceeded(&self) -> bool {
        self.started.elapsed() >= self.ceiling
    }

    pub fn ceiling_secs(&self) -> u64 {
        self.ceiling.as_secs()
    }
}

/// Outcome of a bounded snapshot-metadata read.
#[derive(Debug)]
pub enum SnapshotMetadataRead {
    Parsed(Box<SnapshotMetadata>),
    /// File exceeds `max_bytes`; parsing skipped to protect the time budget.
    OversizeSkipped {
        size_bytes: u64,
    },
    /// Missing, unreadable, or unparseable file. The caller reports the gap
    /// instead of aborting the whole enumeration.
    Unreadable,
}

/// Read `metadata` out of a `snapshot.json` without loading the whole file
/// into memory. Streaming `from_reader` keeps peak RSS bounded by the largest
/// single JSON token instead of the file size; `max_bytes` guards CPU time on
/// pathological snapshots.
pub fn read_snapshot_metadata_bounded(path: &Path, max_bytes: u64) -> SnapshotMetadataRead {
    #[derive(Default, Deserialize)]
    struct Envelope {
        #[serde(default)]
        metadata: SnapshotMetadata,
    }

    let Ok(file_meta) = fs::metadata(path) else {
        return SnapshotMetadataRead::Unreadable;
    };
    if file_meta.len() > max_bytes {
        return SnapshotMetadataRead::OversizeSkipped {
            size_bytes: file_meta.len(),
        };
    }
    let Ok(file) = fs::File::open(path) else {
        return SnapshotMetadataRead::Unreadable;
    };
    match serde_json::from_reader::<_, Envelope>(BufReader::new(file)) {
        Ok(envelope) => SnapshotMetadataRead::Parsed(Box::new(envelope.metadata)),
        Err(_) => SnapshotMetadataRead::Unreadable,
    }
}

/// Recursive directory size with hard entry and time caps. `truncated: true`
/// means `bytes` is a lower bound because a ceiling or filesystem error made
/// the measurement incomplete, never a silently-polished total.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct DirSizeSample {
    pub bytes: u64,
    pub truncated: bool,
}

pub fn bounded_dir_size(
    path: &Path,
    max_entries: u64,
    budget: Option<&EnumerationBudget>,
) -> DirSizeSample {
    bounded_dir_size_entries(walkdir::WalkDir::new(path), max_entries, budget)
}

fn bounded_dir_size_entries(
    entries: impl IntoIterator<Item = walkdir::Result<walkdir::DirEntry>>,
    max_entries: u64,
    budget: Option<&EnumerationBudget>,
) -> DirSizeSample {
    let mut bytes: u64 = 0;
    let mut walked: u64 = 0;
    let mut truncated = false;

    for entry in entries {
        walked += 1;
        if walked > max_entries || budget.is_some_and(EnumerationBudget::exceeded) {
            truncated = true;
            break;
        }

        let entry = match entry {
            Ok(entry) => entry,
            Err(_) => {
                truncated = true;
                continue;
            }
        };
        match entry.metadata() {
            Ok(metadata) if metadata.is_file() => {
                bytes = bytes.saturating_add(metadata.len());
            }
            Ok(_) => {}
            Err(_) => truncated = true,
        }
    }

    DirSizeSample { bytes, truncated }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    #[test]
    fn bounded_metadata_read_parses_wellformed_snapshot() {
        let temp = TempDir::new().expect("tempdir");
        let path = temp.path().join("snapshot.json");
        fs::write(
            &path,
            r#"{"metadata":{"schema_version":"0.9.0","generated_at":"2026-07-22T00:00:00Z","roots":["/tmp/demo"]},"files":[1,2,3],"edges":[]}"#,
        )
        .expect("write snapshot");

        match read_snapshot_metadata_bounded(&path, MAX_SNAPSHOT_METADATA_PARSE_BYTES) {
            SnapshotMetadataRead::Parsed(metadata) => {
                assert_eq!(metadata.schema_version, "0.9.0");
                assert_eq!(metadata.roots, vec!["/tmp/demo".to_string()]);
            }
            other => panic!("expected Parsed, got {other:?}"),
        }
    }

    #[test]
    fn bounded_metadata_read_skips_oversize_file_without_reading_it() {
        let temp = TempDir::new().expect("tempdir");
        let path = temp.path().join("snapshot.json");
        fs::write(&path, r#"{"metadata":{"schema_version":"0.9.0"}}"#).expect("write snapshot");

        // Ceiling below the file size: must skip, not parse.
        match read_snapshot_metadata_bounded(&path, 4) {
            SnapshotMetadataRead::OversizeSkipped { size_bytes } => {
                assert!(size_bytes > 4);
            }
            other => panic!("expected OversizeSkipped, got {other:?}"),
        }
    }

    #[test]
    fn bounded_metadata_read_reports_garbage_as_unreadable() {
        let temp = TempDir::new().expect("tempdir");
        let path = temp.path().join("snapshot.json");
        fs::write(&path, "{ not json").expect("write snapshot");

        assert!(matches!(
            read_snapshot_metadata_bounded(&path, MAX_SNAPSHOT_METADATA_PARSE_BYTES),
            SnapshotMetadataRead::Unreadable
        ));
        assert!(matches!(
            read_snapshot_metadata_bounded(&temp.path().join("missing.json"), u64::MAX),
            SnapshotMetadataRead::Unreadable
        ));
    }

    #[test]
    fn bounded_dir_size_truncates_at_entry_cap() {
        let temp = TempDir::new().expect("tempdir");
        for index in 0..10 {
            fs::write(temp.path().join(format!("file-{index}")), b"0123456789")
                .expect("write file");
        }

        let full = bounded_dir_size(temp.path(), u64::MAX, None);
        assert!(!full.truncated);
        assert_eq!(full.bytes, 100);

        let capped = bounded_dir_size(temp.path(), 3, None);
        assert!(capped.truncated, "entry cap must mark the sample truncated");
        assert!(capped.bytes < full.bytes);
    }

    #[test]
    fn bounded_dir_size_stops_on_exhausted_time_budget() {
        let temp = TempDir::new().expect("tempdir");
        fs::write(temp.path().join("file"), b"payload").expect("write file");

        let expired = EnumerationBudget::start(Duration::ZERO);
        let sample = bounded_dir_size(temp.path(), u64::MAX, Some(&expired));
        assert!(sample.truncated, "expired budget must truncate the walk");
    }

    #[test]
    fn bounded_dir_size_marks_walk_errors_incomplete() {
        let temp = TempDir::new().expect("tempdir");
        let missing = temp.path().join("missing");

        let sample = bounded_dir_size(&missing, u64::MAX, None);

        assert!(
            sample.truncated,
            "walk errors must make the sample incomplete"
        );
        assert_eq!(sample.bytes, 0);
    }

    #[test]
    fn bounded_dir_size_marks_metadata_errors_incomplete() {
        let temp = TempDir::new().expect("tempdir");
        let path = temp.path().join("removed-after-walk");
        fs::write(&path, b"payload").expect("write file");
        let entry = walkdir::WalkDir::new(&path)
            .into_iter()
            .next()
            .expect("root entry")
            .expect("walk file");
        fs::remove_file(&path).expect("remove file before metadata read");
        let sample = bounded_dir_size_entries(std::iter::once(Ok(entry)), u64::MAX, None);

        assert!(
            sample.truncated,
            "metadata errors must make the sample incomplete"
        );
        assert_eq!(sample.bytes, 0);
    }
}
