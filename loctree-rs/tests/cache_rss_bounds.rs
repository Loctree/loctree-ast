//! W7-A resource-contract harness for cache/doctor enumeration.
//!
//! Audit class H (independent reproduction 2026-07-21, probe LCT-E01):
//! `loct doctor --cache --scope --json` on a temporary clone walked the
//! global cache and peaked at 20.8 GiB RSS in 54 s. The repaired contract:
//!
//! 1. `--cache`/`--scope` are project-local by default; the global walk
//!    requires the explicit `--list` (doctor) / `--all` (cache list) opt-in.
//! 2. Enumeration streams snapshot metadata (no whole-file reads), caps
//!    per-bucket walks, budgets wall clock, and reports truncation honestly.
//! 3. Ceiling constants are part of the product contract and enforced here
//!    against a live binary run, not just documented.

use assert_cmd::Command;
use serde_json::{Value, json};
use std::fs;
use std::path::Path;

use loctree::cli::dispatch::{DOCTOR_RSS_CEILING_BYTES, DOCTOR_WALL_CEILING_SECS};

/// Output-byte ceiling for one doctor probe, mirroring the audit harness
/// budget (20 MiB captured output).
const PROBE_OUTPUT_CEILING_BYTES: usize = 20 * 1024 * 1024;

fn loct(cwd: &Path, cache: &Path) -> Command {
    let mut command = Command::cargo_bin("loct").expect("loct binary");
    command
        .current_dir(cwd)
        .env("LOCT_CACHE_DIR", cache)
        .env("NO_COLOR", "1");
    command
}

fn write_cache_snapshot(path: &Path, body: Value) {
    fs::create_dir_all(path.parent().expect("snapshot parent")).expect("create snapshot dir");
    fs::write(
        path,
        serde_json::to_string(&json!({ "metadata": body })).unwrap(),
    )
    .expect("write snapshot");
}

/// Seed a foreign cache bucket (a project the current one must NOT walk).
fn seed_foreign_bucket(cache: &Path, bucket_id: &str, root: &str) {
    let bucket = cache.join("projects").join(bucket_id);
    write_cache_snapshot(
        &bucket.join("latest").join("snapshot.json"),
        json!({
            "schema_version": "0.9.0",
            "generated_at": "2026-07-22T00:00:00Z",
            "roots": [root],
            "git_owner_repo": "foreign/repo",
            "git_branch": "main",
            "git_commit": "abc123"
        }),
    );
    fs::write(bucket.join("latest").join("filler.bin"), vec![0u8; 4096])
        .expect("write filler artifact");
}

/// Minimal project the doctor can recognize: a `.loctree/` marker with a
/// well-formed snapshot rooted at the project itself.
fn seed_project(dir: &Path) {
    let snapshot_dir = dir.join(".loctree");
    fs::create_dir_all(&snapshot_dir).expect("create .loctree");
    let body = json!({
        "metadata": {
            "schema_version": "0.14.0",
            "generated_at": "2026-07-22T00:00:00Z",
            "roots": [dir.display().to_string()],
            "languages": [],
            "file_count": 0,
            "total_loc": 0,
            "scan_duration_ms": 0,
            "manifest_summary": [],
            "entrypoints": [],
            "entrypoint_drift": {
                "declared_missing": [],
                "declared_without_marker": [],
                "code_only_entrypoints": [],
                "declared_unresolved": []
            }
        },
        "files": [],
        "edges": [],
        "export_index": {},
        "command_bridges": [],
        "event_bridges": [],
        "barrels": []
    });
    fs::write(
        snapshot_dir.join("snapshot.json"),
        serde_json::to_string(&body).unwrap(),
    )
    .expect("write project snapshot");
}

/// The LCT-E01 shape: doctor `--cache --scope` inside a project must inspect
/// ONLY that project's bucket, never the global cache.
#[test]
fn doctor_cache_scope_is_project_local_by_default() {
    let cache = tempfile::tempdir().expect("cache tempdir");
    let project = tempfile::tempdir().expect("project tempdir");
    let project_root = project.path().canonicalize().expect("canonicalize project");
    seed_project(&project_root);
    seed_foreign_bucket(cache.path(), "feedfacecafe0001", "/tmp/foreign-one");
    seed_foreign_bucket(cache.path(), "feedfacecafe0002", "/tmp/foreign-two");

    let output = loct(&project_root, cache.path())
        .args(["doctor", "--cache", "--scope", "--json"])
        .assert()
        .success()
        .get_output()
        .clone();

    let report: Value = serde_json::from_slice(&output.stdout).expect("doctor JSON");
    assert_eq!(report["schema_version"], "1.2");
    assert_eq!(report["enumeration"]["scope"], "project-local");
    assert_eq!(report["enumeration"]["complete"], true);

    let entries = report["entries"].as_array().expect("entries array");
    for entry in entries {
        let id = entry["project_id"].as_str().unwrap_or_default();
        assert!(
            !id.starts_with("feedfacecafe"),
            "project-local doctor must not surface foreign buckets; got {id}"
        );
    }
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(
        !stdout.contains("/tmp/foreign-one") && !stdout.contains("/tmp/foreign-two"),
        "foreign roots leaked into a project-local report:\n{stdout}"
    );
}

/// Outside any project, `--scope`/`--cache` without `--project` fail closed
/// with a hint instead of expanding into the global walk.
#[test]
fn doctor_scope_outside_project_fails_closed() {
    let cache = tempfile::tempdir().expect("cache tempdir");
    let scratch = tempfile::tempdir().expect("scratch tempdir");
    seed_foreign_bucket(cache.path(), "feedfacecafe0003", "/tmp/foreign-three");

    let output = loct(scratch.path(), cache.path())
        .args(["doctor", "--scope", "--json"])
        .output()
        .expect("run doctor");

    assert!(
        !output.status.success(),
        "scope without project outside a repo must fail closed.\nstdout: {}\nstderr: {}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        stderr.contains("project-local") && stderr.contains("--list"),
        "stderr must explain the project-local default and --list opt-in:\n{stderr}"
    );
    assert!(
        !stderr.contains("/tmp/foreign-three"),
        "fail-closed path must not enumerate foreign buckets"
    );
}

/// Bare `loct doctor` outside a project hints and walks nothing — the
/// historical implicit global `--list` fallback is gone.
#[test]
fn doctor_bare_outside_project_walks_nothing() {
    let cache = tempfile::tempdir().expect("cache tempdir");
    let scratch = tempfile::tempdir().expect("scratch tempdir");
    seed_foreign_bucket(cache.path(), "feedfacecafe0004", "/tmp/foreign-four");

    let output = loct(scratch.path(), cache.path())
        .args(["doctor"])
        .assert()
        .success()
        .get_output()
        .clone();

    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(
        stdout.contains("nothing was walked") && stdout.contains("--list"),
        "bare doctor outside a project must state the opt-in:\n{stdout}"
    );
    assert!(
        !stdout.contains("feedfacecafe0004"),
        "bare doctor must not enumerate the global cache:\n{stdout}"
    );
}

/// Explicit `--list` remains available and now declares its global scope.
#[test]
fn doctor_list_opt_in_declares_global_scope() {
    let cache = tempfile::tempdir().expect("cache tempdir");
    let scratch = tempfile::tempdir().expect("scratch tempdir");
    seed_foreign_bucket(cache.path(), "feedfacecafe0005", "/tmp/foreign-five");

    let output = loct(scratch.path(), cache.path())
        .args(["doctor", "--list", "--json"])
        .assert()
        .success()
        .get_output()
        .clone();

    let report: Value = serde_json::from_slice(&output.stdout).expect("doctor JSON");
    assert_eq!(report["enumeration"]["scope"], "global");
    let ids: Vec<&str> = report["entries"]
        .as_array()
        .expect("entries")
        .iter()
        .filter_map(|entry| entry["project_id"].as_str())
        .collect();
    assert!(
        ids.contains(&"feedfacecafe0005"),
        "explicit --list must still inventory the global cache; got {ids:?}"
    );
}

/// `loct cache list` is project-local by default and points at `--all`;
/// the global inventory requires the explicit opt-in.
#[test]
fn cache_list_default_is_project_local() {
    let cache = tempfile::tempdir().expect("cache tempdir");
    let project = tempfile::tempdir().expect("project tempdir");
    let project_root = project.path().canonicalize().expect("canonicalize project");
    seed_project(&project_root);
    seed_foreign_bucket(cache.path(), "feedfacecafe0006", "/tmp/foreign-six");

    let local = loct(&project_root, cache.path())
        .args(["cache", "list"])
        .assert()
        .success()
        .get_output()
        .clone();
    let local_stdout = String::from_utf8_lossy(&local.stdout);
    assert!(
        local_stdout.contains("project-local"),
        "default cache list must declare its project-local scope:\n{local_stdout}"
    );
    assert!(
        local_stdout.contains("--all"),
        "default cache list must point at the --all opt-in:\n{local_stdout}"
    );
    assert!(
        !local_stdout.contains("/tmp/foreign-six"),
        "default cache list must not walk foreign buckets:\n{local_stdout}"
    );

    let global = loct(&project_root, cache.path())
        .args(["cache", "list", "--all"])
        .assert()
        .success()
        .get_output()
        .clone();
    let global_stdout = String::from_utf8_lossy(&global.stdout);
    assert!(
        global_stdout.contains("/tmp/foreign-six"),
        "--all must inventory the global cache:\n{global_stdout}"
    );
}

/// LCT-E01 acceptance replay against the live binary: the doctor probe on a
/// synthetic multi-bucket cache stays under the documented RSS / wall /
/// output ceilings. RSS is measured with `/usr/bin/time` (`-l` on macOS,
/// `-v` on Linux); when the tool is unavailable the RSS clause is skipped
/// loudly rather than silently passed.
#[test]
fn doctor_probe_stays_within_documented_ceilings() {
    let cache = tempfile::tempdir().expect("cache tempdir");
    let project = tempfile::tempdir().expect("project tempdir");
    let project_root = project.path().canonicalize().expect("canonicalize project");
    seed_project(&project_root);

    // Multi-bucket cache with non-trivial snapshots: enough to prove the
    // streaming/bounded path, small enough for CI.
    let filler: Vec<u64> = (0..200_000).collect();
    for index in 0..5 {
        let bucket = cache
            .path()
            .join("projects")
            .join(format!("cafebabe000000{index:02}"));
        fs::create_dir_all(bucket.join("latest")).expect("create bucket");
        let body = json!({
            "metadata": {
                "schema_version": "0.9.0",
                "generated_at": "2026-07-22T00:00:00Z",
                "roots": [format!("/tmp/synthetic-{index}")]
            },
            "files": &filler,
            "edges": &filler
        });
        fs::write(
            bucket.join("latest").join("snapshot.json"),
            serde_json::to_string(&body).unwrap(),
        )
        .expect("write big snapshot");
    }

    let loct_bin = env!("CARGO_BIN_EXE_loct");
    let started = std::time::Instant::now();

    let time_tool = Path::new("/usr/bin/time");
    let (output, max_rss_bytes) = if time_tool.exists() {
        let flag = if cfg!(target_os = "macos") {
            "-l"
        } else {
            "-v"
        };
        let output = std::process::Command::new(time_tool)
            .arg(flag)
            .arg(loct_bin)
            .args(["doctor", "--list", "--json"])
            .current_dir(&project_root)
            .env("LOCT_CACHE_DIR", cache.path())
            .env("NO_COLOR", "1")
            .output()
            .expect("run doctor under /usr/bin/time");
        let stderr = String::from_utf8_lossy(&output.stderr).to_string();
        (output, parse_max_rss_bytes(&stderr))
    } else {
        eprintln!("[cache_rss_bounds] /usr/bin/time unavailable — RSS clause skipped");
        let output = std::process::Command::new(loct_bin)
            .args(["doctor", "--list", "--json"])
            .current_dir(&project_root)
            .env("LOCT_CACHE_DIR", cache.path())
            .env("NO_COLOR", "1")
            .output()
            .expect("run doctor");
        (output, None)
    };
    let wall = started.elapsed();

    assert!(
        output.status.success(),
        "doctor probe failed.\nstderr: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    assert!(
        wall.as_secs() < DOCTOR_WALL_CEILING_SECS,
        "doctor probe took {}s, ceiling is {}s",
        wall.as_secs(),
        DOCTOR_WALL_CEILING_SECS
    );
    assert!(
        output.stdout.len() < PROBE_OUTPUT_CEILING_BYTES,
        "doctor probe emitted {} bytes, ceiling is {}",
        output.stdout.len(),
        PROBE_OUTPUT_CEILING_BYTES
    );
    if let Some(rss) = max_rss_bytes {
        assert!(
            rss < DOCTOR_RSS_CEILING_BYTES,
            "doctor probe peaked at {rss} bytes RSS, ceiling is {DOCTOR_RSS_CEILING_BYTES}"
        );
    }

    let report: Value = serde_json::from_slice(&output.stdout).expect("doctor JSON");
    assert_eq!(report["enumeration"]["scope"], "global");
    assert_eq!(
        report["entries"].as_array().map(Vec::len),
        Some(5),
        "all synthetic buckets should be inventoried within the ceilings"
    );
}

/// Parse `/usr/bin/time` verbose output for max RSS, normalized to bytes.
/// macOS `-l` reports bytes ("maximum resident set size"); Linux `-v`
/// reports kbytes ("Maximum resident set size (kbytes)").
fn parse_max_rss_bytes(stderr: &str) -> Option<u64> {
    for line in stderr.lines() {
        let lower = line.to_lowercase();
        if !lower.contains("maximum resident set size") {
            continue;
        }
        let digits: String = line
            .chars()
            .filter(|character| character.is_ascii_digit())
            .collect();
        let value: u64 = digits.parse().ok()?;
        return Some(if lower.contains("kbytes") {
            value * 1024
        } else {
            value
        });
    }
    None
}
