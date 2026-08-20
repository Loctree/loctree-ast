//! Help-examples-as-tests: every jq example `--help-full` advertises must be
//! EXECUTABLE against a real snapshot — no nulls, no filter errors.
//!
//! Born from a live dead-end (2026-08-19): help promised `.dead_parrots[]`
//! and `.cycles[]`, README promised `.summary.health_score`, while the
//! snapshot's top-level keys were barrels/edges/files/metadata/... — agents
//! got "cannot use null as iterable" from the product's own front door.
//! A help surface that advertises a contract the engine refuses is a layer
//! lying about the layer below; this gate makes that class unrepresentable
//! for the JQ QUERIES section.
//!
//! 𝚅𝚒𝚋𝚎𝚌𝚛𝚊𝚏𝚝𝚎𝚍. with AI Agents ⓒ 2025-2026 Loctree Team

use assert_cmd::Command;
use assert_cmd::cargo::cargo_bin_cmd;
use std::path::{Path, PathBuf};
use tempfile::TempDir;

fn fixtures_path() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures")
}

fn loct(cache: &std::path::Path) -> Command {
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

/// Parse one advertised `loct '<filter>' [flags]   <description>` help line
/// into the argv it promises.
///
/// The WHOLE argv is recovered, not just the quoted filter: an example that
/// needs `--artifact findings` makes a different promise than the bare filter,
/// and silently dropping the flag would let the help advertise a contract this
/// gate never actually runs. The help renders its description in a column
/// separated by two-or-more spaces — that column boundary ends the argv.
fn parse_example(line: &str) -> Option<Vec<String>> {
    let rest = line.trim().strip_prefix("loct '")?;
    let end = rest.find('\'')?;
    let mut args = vec![rest[..end].to_string()];
    let tail = &rest[end + 1..];
    let argv_tail = match tail.find("  ") {
        Some(i) => &tail[..i],
        None => tail,
    };
    args.extend(argv_tail.split_whitespace().map(str::to_string));
    Some(args)
}

/// Extract `loct '<query>' ...` examples from the JQ QUERIES help section.
fn advertised_jq_examples(help: &str) -> Vec<Vec<String>> {
    let section = help
        .split("=== JQ QUERIES")
        .nth(1)
        .expect("--help-full must carry a JQ QUERIES section")
        .split("\n=== ")
        .next()
        .unwrap();
    section.lines().filter_map(parse_example).collect()
}

#[test]
fn every_advertised_jq_example_executes_against_a_real_snapshot() {
    let help = loct(&TempDir::new().unwrap().path().join("cache"))
        .arg("--help-full")
        .output()
        .expect("run --help-full");
    let help_text = String::from_utf8_lossy(&help.stdout).to_string();
    let examples = advertised_jq_examples(&help_text);
    assert!(
        examples.len() >= 3,
        "JQ QUERIES section should advertise at least 3 runnable examples, found {examples:?}"
    );
    assert!(
        examples.iter().any(|args| args.len() > 1),
        "JQ QUERIES advertises no flagged example — the --artifact surface is unproven: {examples:?}"
    );

    let cache = TempDir::new().unwrap();
    // Work on a copy: the full pass writes a context atlas into the working
    // directory, and the checked-in fixture is not a scratch space.
    let work = TempDir::new().unwrap();
    let fixture = work.path().join("simple_ts");
    copy_tree(&fixtures_path().join("simple_ts"), &fixture);
    // Full pass, not `scan`: the advertised --artifact examples read the
    // sibling artifacts, which only a full run materializes.
    loct(cache.path()).current_dir(&fixture).assert().success();

    for args in &examples {
        let out = loct(cache.path())
            .current_dir(&fixture)
            .args(args)
            .output()
            .expect("run advertised jq example");
        let stdout = String::from_utf8_lossy(&out.stdout).trim().to_string();
        let stderr = String::from_utf8_lossy(&out.stderr).to_string();
        assert!(
            out.status.success(),
            "advertised example `loct {args:?}` failed: {stderr}"
        );
        assert!(
            !stdout.is_empty() && stdout != "null",
            "advertised example `loct {args:?}` answered {stdout:?} — \
             the help promises a contract the snapshot does not carry"
        );
    }
}
