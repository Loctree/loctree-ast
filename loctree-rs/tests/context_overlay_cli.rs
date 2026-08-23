//! I1-01 acceptance gates for the C0-01 overlay seam on the bare
//! `loct context` path.
//!
//! - resilience gate: dead transport ⇒ explicit `intent layer stale (…) —
//!   refresh: …` marker, never a silent gap;
//! - feature gate: a warm cache renders ≥1 thesis in the spec grammar
//!   (`✓[U] 2026-07-13 · …`), not merely the word "intent";
//! - plan-B gate: the render NEVER waits for the producer (slow producer +
//!   warm cache ⇒ fast render from last correct data with a stale marker),
//!   and the detached refresh lands fresh data for the NEXT invocation.
//!
//! The full-key cache-miss matrix (store_revision / overlay_revision /
//! anchor_catalog_revision / snapshot_commit / repo_id / schema_version) is
//! covered by unit tests in `loctree::aicx::overlay`.

use assert_cmd::Command;
use assert_cmd::cargo::cargo_bin_cmd;
use regex::Regex;
use serde_json::Value;
use std::fs;
use std::path::Path;
use std::time::{Duration, Instant};
use tempfile::TempDir;

const THESIS_GRAMMAR: &str = r"[✓⊘✗]\[[VUR]\] \d{4}-\d{2}-\d{2} ·";

fn loct() -> Command {
    let mut cmd = cargo_bin_cmd!("loct");
    cmd.env("LOCT_OPEN_BROWSER", "0");
    cmd
}

/// Init a git repo with one committed source file and a loctree snapshot.
fn scanned_repo() -> TempDir {
    let dir = tempfile::tempdir().expect("tempdir");
    let root = dir.path();
    let git = |args: &[&str]| {
        let status = std::process::Command::new("git")
            .args(args)
            .current_dir(root)
            .stdout(std::process::Stdio::null())
            .stderr(std::process::Stdio::null())
            .status()
            .expect("run git");
        assert!(status.success(), "git {args:?} failed");
    };
    git(&["init", "-q"]);
    git(&["config", "user.email", "test@example.com"]);
    git(&["config", "user.name", "overlay-test"]);
    fs::write(
        root.join("lib.rs"),
        "pub fn seam() -> &'static str { \"overlay\" }\n",
    )
    .expect("write lib.rs");
    git(&["add", "."]);
    git(&["commit", "-q", "-m", "seed"]);

    loct()
        .arg("scan")
        .current_dir(root)
        // Keep every artifact inside the tempdir, away from the shared cache.
        .env("LOCT_CACHE_DIR", root.join(".loct-cache"))
        .assert()
        .success();
    dir
}

fn context_output(root: &Path, aicx_binary: &str) -> String {
    let output = loct()
        .arg("context")
        .current_dir(root)
        .env("LOCT_CACHE_DIR", root.join(".loct-cache"))
        .env("LOCT_AICX_BINARY", aicx_binary)
        .output()
        .expect("run loct context");
    assert!(
        output.status.success(),
        "loct context failed: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    String::from_utf8_lossy(&output.stdout).into_owned()
}

/// Read (repo_id, snapshot_commit, anchor_catalog_revision) from the real
/// `loct anchors` emission so planted caches can match local truth exactly.
fn local_identity(root: &Path) -> (String, String, String) {
    let output = loct()
        .arg("anchors")
        .arg("--format")
        .arg("json")
        .arg(root)
        .current_dir(root)
        .env("LOCT_CACHE_DIR", root.join(".loct-cache"))
        .output()
        .expect("run loct anchors");
    assert!(
        output.status.success(),
        "loct anchors failed: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    let catalog: Value = serde_json::from_slice(&output.stdout).expect("anchors emission is JSON");
    (
        catalog["repo_id"].as_str().expect("repo_id").to_string(),
        catalog["snapshot_commit"]
            .as_str()
            .expect("snapshot_commit")
            .to_string(),
        catalog["anchor_catalog_revision"]
            .as_str()
            .expect("anchor_catalog_revision")
            .to_string(),
    )
}

fn overlay_json(repo_id: &str, snapshot_commit: &str, acr: &str) -> String {
    format!(
        r#"{{
  "schema": "loctree.overlay.intent.v1",
  "repo_id": "{repo_id}",
  "snapshot_commit": "{snapshot_commit}",
  "anchor_catalog_revision": "{acr}",
  "store_revision": "sr1:{sr}",
  "overlay_revision": "ov1:{ov}",
  "producer_version": "0.11.0",
  "entries": [
    {{
      "intent_id": "int1:2371588e4469af0e",
      "content_hash": "ch1:{ch}",
      "target": {{ "kind": "path", "path": "lib.rs", "language": "rs" }},
      "thesis": "The seam function is the overlay integration witness",
      "status": "current",
      "authority": "agent_derived",
      "verification_status": "unverified",
      "valid_from": "2026-07-13T16:13:24.877Z",
      "refs": [
        {{
          "evidence_event_id": "ev1:claude:49a84e4c:002279:user:7dd32539a72ba6fc",
          "ref": "session:49a84e4c#turn-888"
        }}
      ]
    }}
  ]
}}"#,
        sr = "6".repeat(64),
        ov = "8".repeat(64),
        ch = "e".repeat(64),
    )
}

fn plant_cache(root: &Path, raw: &str) {
    let cache = root.join(".loctree").join("aicx-overlay.v1.json");
    fs::create_dir_all(cache.parent().unwrap()).unwrap();
    fs::write(cache, raw).unwrap();
}

/// Write an executable mock `aicx` that answers `--version` instantly and
/// serves the given overlay document after `sleep_secs`.
#[cfg(unix)]
fn write_mock_aicx(dir: &Path, overlay: &str, sleep_secs: u64) -> String {
    use std::os::unix::fs::PermissionsExt;
    let payload = dir.join("mock-overlay.json");
    fs::write(&payload, overlay).unwrap();
    let script = dir.join("mock-aicx.sh");
    fs::write(
        &script,
        format!(
            "#!/bin/sh\nif [ \"$1\" = \"--version\" ]; then echo mock-aicx 0.11.0; exit 0; fi\nsleep {sleep_secs}\ncat \"{}\"\n",
            payload.display()
        ),
    )
    .unwrap();
    fs::set_permissions(&script, fs::Permissions::from_mode(0o755)).unwrap();
    script.display().to_string()
}

#[test]
fn resilience_dead_transport_prints_explicit_stale_marker() {
    let repo = scanned_repo();
    let out = context_output(repo.path(), "/nonexistent/aicx-missing");
    assert!(
        out.contains("intent layer stale ("),
        "dead transport must degrade explicitly, got:\n{out}"
    );
    assert!(
        out.contains("— refresh: `"),
        "stale marker must carry the refresh command, got:\n{out}"
    );
    assert!(
        !out.contains("skipped (timeout)"),
        "the retired budgeted-transport timeout string must not resurface:\n{out}"
    );
}

#[test]
fn warm_fresh_cache_renders_theses_in_spec_grammar() {
    let repo = scanned_repo();
    let (repo_id, snapshot_commit, acr) = local_identity(repo.path());
    plant_cache(repo.path(), &overlay_json(&repo_id, &snapshot_commit, &acr));

    let out = context_output(repo.path(), "/nonexistent/aicx-missing");
    let grammar = Regex::new(THESIS_GRAMMAR).unwrap();
    assert!(
        grammar.is_match(&out),
        "warm cache must render ≥1 thesis in the spec grammar, got:\n{out}"
    );
    assert!(
        out.contains("intent layer fresh ("),
        "matching local truth must present as fresh, got:\n{out}"
    );
    assert!(
        !out.contains("intent layer stale ("),
        "fresh cache must not carry a stale marker, got:\n{out}"
    );
}

#[cfg(unix)]
#[test]
fn render_never_waits_for_a_slow_producer() {
    let repo = scanned_repo();
    // Mocks live OUTSIDE the repo: new files in the worktree would change
    // the snapshot (and with it the anchor catalog revision) on the next
    // auto-rescan, poisoning the freshness assertion.
    let mocks = tempfile::tempdir().expect("mock dir");
    let (repo_id, _snapshot_commit, acr) = local_identity(repo.path());
    // Stale on purpose: snapshot_commit mismatch forces a refresh
    // recommendation while the mock producer takes 10 s to answer.
    plant_cache(repo.path(), &overlay_json(&repo_id, "aaaaaa1", &acr));
    let slow_mock = write_mock_aicx(mocks.path(), &overlay_json(&repo_id, "aaaaaa1", &acr), 10);

    let started = Instant::now();
    let out = context_output(repo.path(), &slow_mock);
    let elapsed = started.elapsed();

    assert!(
        elapsed < Duration::from_secs(5),
        "render must not wait for the producer (plan B), took {elapsed:?}"
    );
    let grammar = Regex::new(THESIS_GRAMMAR).unwrap();
    assert!(
        grammar.is_match(&out),
        "stale render keeps the last correct data, got:\n{out}"
    );
    assert!(
        out.contains("intent layer stale ("),
        "stale cache must be marked explicitly, got:\n{out}"
    );
}

#[cfg(unix)]
#[test]
fn detached_refresh_lands_fresh_data_for_the_next_call() {
    let repo = scanned_repo();
    // Mocks live OUTSIDE the repo — see render_never_waits_for_a_slow_producer.
    let mocks = tempfile::tempdir().expect("mock dir");
    let (repo_id, snapshot_commit, acr) = local_identity(repo.path());
    let fresh_overlay = overlay_json(&repo_id, &snapshot_commit, &acr);
    let fast_mock = write_mock_aicx(mocks.path(), &fresh_overlay, 0);

    // First call: no cache yet → explicit marker + detached refresh spawn.
    let first = context_output(repo.path(), &fast_mock);
    assert!(
        first.contains("intent layer stale ("),
        "cold call must mark the missing cache, got:\n{first}"
    );

    // The refresh is detached — poll for its landing instead of waiting inline.
    let cache = repo.path().join(".loctree").join("aicx-overlay.v1.json");
    let deadline = Instant::now() + Duration::from_secs(15);
    while !cache.exists() && Instant::now() < deadline {
        std::thread::sleep(Duration::from_millis(100));
    }
    assert!(
        cache.exists(),
        "detached refresh must land the overlay cache for the next call"
    );

    let second = context_output(repo.path(), &fast_mock);
    let grammar = Regex::new(THESIS_GRAMMAR).unwrap();
    assert!(
        grammar.is_match(&second),
        "second call must serve refreshed theses, got:\n{second}"
    );
    assert!(
        second.contains("intent layer fresh ("),
        "refreshed cache matching local truth must present as fresh, got:\n{second}"
    );
}
