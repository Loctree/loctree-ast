//! CLI JSON must expose named role-root buckets, not hide tests/scripts.
//!
//! Companion to `zz_swift_report_truth::role_root_files_are_never_orphans`:
//! that fence checks `orphan_files.total == 0`; this one checks the role
//! buckets are present and populated for the same fixture.
//!
//! 𝚅𝚒𝚋𝚎𝚌𝚛𝚊𝚏𝚝𝚎𝚍. with AI Agents by Vetcoders (c)2024-2026 LibraxisAI

use assert_cmd::Command;
use assert_cmd::cargo::cargo_bin_cmd;
use serde_json::Value;
use std::path::{Path, PathBuf};
use tempfile::TempDir;

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

fn loct_at(root: &Path, cache: &Path) -> Command {
    let mut cmd = cargo_bin_cmd!("loct");
    cmd.current_dir(root)
        .env("LOCT_CACHE_DIR", cache)
        .env("LOCT_OPEN_BROWSER", "0")
        .env("LOCT_NO_GITIGNORE", "1");
    cmd
}

#[test]
fn role_root_buckets_are_visible_in_audit_json() {
    let fixture_src =
        PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/swift_role_roots");
    let temp = TempDir::new().unwrap();
    let cache = TempDir::new().unwrap();
    copy_dir_all(&fixture_src, temp.path()).unwrap();
    let status = std::process::Command::new("git")
        .current_dir(temp.path())
        .args(["-c", "init.defaultBranch=main", "init", "-q"])
        .status()
        .unwrap();
    assert!(status.success());
    let status = std::process::Command::new("git")
        .current_dir(temp.path())
        .args(["add", "-A", "-f"])
        .status()
        .unwrap();
    assert!(status.success());
    loct_at(temp.path(), cache.path())
        .args(["scan", "--full-scan"])
        .assert()
        .success();

    let output = loct_at(temp.path(), cache.path())
        .args(["audit", "--json"])
        .assert()
        .success()
        .get_output()
        .clone();
    let audit: Value = serde_json::from_slice(&output.stdout).unwrap();

    assert_eq!(audit["orphan_files"]["total"].as_u64().unwrap(), 0);
    assert!(
        audit["test_orphans"]["total"].as_u64().unwrap() >= 1,
        "test-role files must land in test_orphans: {audit}"
    );
    assert!(
        audit["script_orphans"]["total"].as_u64().unwrap() >= 1,
        "script-role files must land in script_orphans: {audit}"
    );
    assert!(audit.get("doc_orphans").is_some());
    assert!(audit.get("manifest_orphans").is_some());

    let test_files = audit["test_orphans"]["files"]
        .as_array()
        .expect("test_orphans.files");
    assert!(
        test_files.iter().any(|f| {
            f["path"]
                .as_str()
                .is_some_and(|p| p.contains("AppTests/") || p.ends_with("Tests.swift"))
        }),
        "expected an XCTest role root in test_orphans.files: {test_files:?}"
    );
    let script_files = audit["script_orphans"]["files"]
        .as_array()
        .expect("script_orphans.files");
    assert!(
        script_files
            .iter()
            .any(|f| f["path"].as_str().is_some_and(|p| p.contains("scripts/"))),
        "expected scripts/*.sh in script_orphans.files: {script_files:?}"
    );
}
