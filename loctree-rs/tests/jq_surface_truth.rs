//! The jq surface must answer "no such key, here is what IS here" instead of a
//! silent `null`, and must be able to reach the findings artifacts where
//! `.summary`, `.dead_parrots` and `.cycles` actually live.
//!
//! Live dead-end (2026-08-19): `loct '.summary | {...}'` answered nulls without
//! a word and `loct '.cycles[:2]'` answered "cannot use null as rangeable" —
//! both are the front door refusing to say that the key simply is not in
//! snapshot.json. The data existed the whole time, one file over.
//!
//! 𝚅𝚒𝚋𝚎𝚌𝚛𝚊𝚏𝚝𝚎𝚍. with AI Agents ⓒ 2025-2026 Loctree Team

use assert_cmd::Command;
use assert_cmd::cargo::cargo_bin_cmd;
use std::path::{Path, PathBuf};
use tempfile::TempDir;

fn loct(cache: &Path) -> Command {
    let mut cmd = cargo_bin_cmd!("loct");
    cmd.env("LOCT_OPEN_BROWSER", "0");
    cmd.env("LOCT_CACHE_DIR", cache);
    cmd.env(loctree::snapshot::LOCT_ALLOW_NON_GIT_ROOT_ENV, "1");
    cmd
}

fn copy_tree(from: &Path, to: &Path) {
    std::fs::create_dir_all(to).unwrap();
    for entry in std::fs::read_dir(from).unwrap() {
        let entry = entry.unwrap();
        let target = to.join(entry.file_name());
        if entry.file_type().unwrap().is_dir() {
            copy_tree(&entry.path(), &target);
        } else {
            std::fs::copy(entry.path(), &target).unwrap();
        }
    }
}

/// Scanned fixture copy with a full artifact set, plus its private cache.
struct Probe {
    cache: TempDir,
    _work: TempDir,
    root: PathBuf,
}

impl Probe {
    fn new() -> Self {
        let cache = TempDir::new().unwrap();
        let work = TempDir::new().unwrap();
        let root = work.path().join("simple_ts");
        copy_tree(
            &PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/simple_ts"),
            &root,
        );
        let mut cmd = cargo_bin_cmd!("loct");
        cmd.env("LOCT_OPEN_BROWSER", "0");
        cmd.env("LOCT_CACHE_DIR", cache.path());
        cmd.env(loctree::snapshot::LOCT_ALLOW_NON_GIT_ROOT_ENV, "1");
        cmd.current_dir(&root).assert().success();
        Self {
            cache,
            _work: work,
            root,
        }
    }

    fn run(&self, args: &[&str]) -> std::process::Output {
        loct(self.cache.path())
            .current_dir(&self.root)
            .args(args)
            .output()
            .expect("run loct")
    }
}

#[test]
fn missing_top_level_key_names_the_available_ones_and_fails() {
    let probe = Probe::new();
    let out = probe.run([".summary | {a: .health_score}"].as_ref());
    let stderr = String::from_utf8_lossy(&out.stderr).to_string();

    assert!(
        !out.status.success(),
        "a miss must be a miss: {stderr}\nstdout={}",
        String::from_utf8_lossy(&out.stdout)
    );
    assert!(
        stderr.contains("no key '.summary'"),
        "miss must name the key: {stderr}"
    );
    assert!(
        stderr.contains("available top-level keys"),
        "miss must list what IS queryable: {stderr}"
    );
    assert!(
        stderr.contains("files") && stderr.contains("metadata"),
        "available keys must be the real snapshot surface: {stderr}"
    );
    assert!(
        stderr.contains("--artifact"),
        "a key that lives one artifact over must carry an executable next step: {stderr}"
    );
}

/// The original error was "cannot use null as rangeable" — a jaq-level symptom
/// of a snapshot-level miss. It must be replaced, not merely accompanied.
#[test]
fn ranged_miss_reports_the_key_not_a_jaq_type_error() {
    let probe = Probe::new();
    let out = probe.run([".cycles[:2]"].as_ref());
    let stderr = String::from_utf8_lossy(&out.stderr).to_string();

    assert!(!out.status.success(), "{stderr}");
    assert!(
        stderr.contains("no key '.cycles'"),
        "expected a named miss, got: {stderr}"
    );
    assert!(
        !stderr.contains("rangeable"),
        "the jaq type error must not survive the miss report: {stderr}"
    );
}

/// Compatibility: filters that DO hit a real key are untouched, and jq's own
/// ways of asking for a quiet miss (`?`, `//`) keep answering quietly.
#[test]
fn existing_filters_and_explicit_silence_are_unchanged() {
    let probe = Probe::new();

    let out = probe.run([".files | length"].as_ref());
    assert!(out.status.success());
    let count: i64 = String::from_utf8_lossy(&out.stdout).trim().parse().unwrap();
    assert!(count > 0, "expected a real file count, got {count}");

    let out = probe.run([".metadata.git_repo?"].as_ref());
    assert!(
        out.status.success(),
        "`?` is the user asking for silence: {}",
        String::from_utf8_lossy(&out.stderr)
    );

    let out = probe.run([".summary?"].as_ref());
    assert!(
        out.status.success(),
        "`.summary?` opts out of the miss report: {}",
        String::from_utf8_lossy(&out.stderr)
    );

    let out = probe.run([".summary // \"absent\"", "-r"].as_ref());
    assert!(out.status.success());
    assert_eq!(String::from_utf8_lossy(&out.stdout).trim(), "absent");
}

#[test]
fn findings_artifacts_are_reachable_from_the_jq_surface() {
    let probe = Probe::new();

    let out = probe.run([".summary.health_score", "--artifact", "agent"].as_ref());
    assert!(
        out.status.success(),
        "{}",
        String::from_utf8_lossy(&out.stderr)
    );
    let score: i64 = String::from_utf8_lossy(&out.stdout).trim().parse().unwrap();
    assert!(
        (0..=100).contains(&score),
        "health_score out of range: {score}"
    );

    let out = probe.run([".dead_parrots | length", "--artifact", "findings"].as_ref());
    assert!(
        out.status.success(),
        "{}",
        String::from_utf8_lossy(&out.stderr)
    );
    let dead: i64 = String::from_utf8_lossy(&out.stdout).trim().parse().unwrap();
    assert!(dead >= 0, "dead parrot count must be a number: {dead}");

    let out = probe.run([".cycles | length", "--artifact", "findings"].as_ref());
    assert!(
        out.status.success(),
        "{}",
        String::from_utf8_lossy(&out.stderr)
    );
    assert!(
        !String::from_utf8_lossy(&out.stdout).trim().is_empty(),
        "cycles must answer a count, not nothing"
    );
}

/// `--artifact` reads naturally in front of the filter, so both orders must
/// answer the same thing — the flag position is not a hidden contract.
#[test]
fn artifact_flag_works_on_either_side_of_the_filter() {
    let probe = Probe::new();
    let before = probe.run(["--artifact", "findings", ".dead_parrots | length"].as_ref());
    let after = probe.run([".dead_parrots | length", "--artifact", "findings"].as_ref());
    assert!(
        before.status.success(),
        "flag-first form failed: {}",
        String::from_utf8_lossy(&before.stderr)
    );
    assert!(after.status.success());
    assert_eq!(before.stdout, after.stdout);
}

#[test]
fn unknown_artifact_name_is_refused_with_the_available_set() {
    let probe = Probe::new();
    let out = probe.run([".summary", "--artifact", "nonesuch"].as_ref());
    let stderr = String::from_utf8_lossy(&out.stderr).to_string();
    assert!(!out.status.success(), "{stderr}");
    assert!(
        stderr.contains("nonesuch") && stderr.contains("findings"),
        "refusal must name the available artifacts: {stderr}"
    );
}
