//! W1-03 — shapeless name collisions in a single-module SwiftPM target.
//!
//! Sensor (`loct twins`) still lists every group. The new fields tell W2-01
//! which groups must not feed the score.
//!
//! 𝚅𝚒𝚋𝚎𝚌𝚛𝚊𝚏𝚝𝚎𝚍. with AI Agents by Vetcoders (c)2024-2026 LibraxisAI

use assert_cmd::cargo::cargo_bin_cmd;
use serde_json::Value;
use std::path::{Path, PathBuf};
use tempfile::TempDir;

fn fixtures_path() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures")
}

fn copy_dir_all(src: &Path, dst: &Path) -> std::io::Result<()> {
    std::fs::create_dir_all(dst)?;
    for entry in std::fs::read_dir(src)? {
        let entry = entry?;
        let target = dst.join(entry.file_name());
        if entry.file_type()?.is_dir() {
            copy_dir_all(&entry.path(), &target)?;
        } else {
            std::fs::copy(entry.path(), &target)?;
        }
    }
    Ok(())
}

fn git(root: &Path, args: &[&str]) {
    let status = std::process::Command::new("git")
        .current_dir(root)
        .args(args)
        .status()
        .expect("git must be available");
    assert!(status.success(), "git {args:?} failed");
}

/// Isolated checkout of `swift_namesake` plus a private cache.
fn namesake_fixture() -> (TempDir, TempDir) {
    let temp = TempDir::new().unwrap();
    let cache = TempDir::new().unwrap();
    copy_dir_all(&fixtures_path().join("swift_namesake"), temp.path()).unwrap();
    git(
        temp.path(),
        &["-c", "init.defaultBranch=main", "init", "-q"],
    );
    git(temp.path(), &["add", "-A", "-f"]);
    cargo_bin_cmd!("loct")
        .current_dir(temp.path())
        .env("LOCT_CACHE_DIR", cache.path())
        .env("LOCT_OPEN_BROWSER", "0")
        .env("LOCT_NO_GITIGNORE", "1")
        .args(["scan", "--full-scan"])
        .assert()
        .success();
    (temp, cache)
}

fn twins_json(root: &Path, cache: &Path) -> Value {
    let output = cargo_bin_cmd!("loct")
        .current_dir(root)
        .env("LOCT_CACHE_DIR", cache)
        .env("LOCT_OPEN_BROWSER", "0")
        .env("LOCT_NO_GITIGNORE", "1")
        .args(["twins", "--json"])
        .assert()
        .success()
        .get_output()
        .clone();
    serde_json::from_slice(&output.stdout).expect("loct twins --json")
}

/// Single-module Package.swift: shapeless namesakes stay visible and are
/// flagged so W2-01 can skip them. Group count must not shrink.
#[test]
fn namesake_single_module_fixture_flags_shapeless_groups() {
    let (fixture, cache) = namesake_fixture();
    let twins = twins_json(fixture.path(), cache.path());
    let groups = twins["exact_twins"].as_array().expect("exact_twins array");

    assert_eq!(
        groups.len(),
        3,
        "sensor must still expose text/body/makeNSView, got {groups:?}"
    );

    let mut names: Vec<&str> = groups.iter().filter_map(|g| g["name"].as_str()).collect();
    names.sort_unstable();
    assert_eq!(names, ["body", "makeNSView", "text"]);

    for group in groups {
        assert_eq!(group["shape_match"], false, "{group}");
        assert_eq!(group["single_module_target"], true, "{group}");
        assert_eq!(group["exclude_from_score"], true, "{group}");
        assert_eq!(group["class"], "NAME_COLLISION", "{group}");
    }
}

/// Two product targets: shapeless collisions stay listed and are NOT excluded.
#[test]
fn namesake_multi_target_fixture_does_not_suppress() {
    let temp = TempDir::new().unwrap();
    let cache = TempDir::new().unwrap();
    copy_dir_all(&fixtures_path().join("swift_namesake"), temp.path()).unwrap();
    std::fs::write(
        temp.path().join("Package.swift"),
        r#"
import PackageDescription
let package = Package(
    name: "NamesakeApp",
    targets: [
        .target(name: "UI"),
        .target(name: "Kit"),
    ]
)
"#,
    )
    .unwrap();
    git(
        temp.path(),
        &["-c", "init.defaultBranch=main", "init", "-q"],
    );
    git(temp.path(), &["add", "-A", "-f"]);
    cargo_bin_cmd!("loct")
        .current_dir(temp.path())
        .env("LOCT_CACHE_DIR", cache.path())
        .env("LOCT_OPEN_BROWSER", "0")
        .env("LOCT_NO_GITIGNORE", "1")
        .args(["scan", "--full-scan"])
        .assert()
        .success();

    let twins = twins_json(temp.path(), cache.path());
    let groups = twins["exact_twins"].as_array().expect("exact_twins array");
    assert_eq!(groups.len(), 3, "sensor must still list every group");
    for group in groups {
        assert_eq!(group["shape_match"], false, "{group}");
        assert_eq!(
            group["single_module_target"], false,
            "two .target( declarations must not look single-module: {group}"
        );
        assert_eq!(group["exclude_from_score"], false, "{group}");
    }
}
