use std::fs;
use std::path::Path;
use std::process::Command;

use serde_json::Value;
use tempfile::TempDir;

fn git(root: &Path, args: &[&str]) {
    let output = Command::new("git")
        .args(args)
        .current_dir(root)
        .output()
        .expect("git should run");
    assert!(
        output.status.success(),
        "git {args:?} failed: {}",
        String::from_utf8_lossy(&output.stderr)
    );
}

fn fixture_repo() -> TempDir {
    let temp = tempfile::tempdir().expect("temp repo");
    fs::create_dir_all(temp.path().join("src")).expect("src dir");
    fs::copy(
        "tests/fixtures/anchors_rename/a.rs",
        temp.path().join("src/a.rs"),
    )
    .expect("copy fixture");
    fs::write(
        temp.path().join("Cargo.toml"),
        "[package]\nname = \"anchor-fixture\"\nversion = \"0.1.0\"\nedition = \"2024\"\n",
    )
    .expect("manifest");
    git(temp.path(), &["init", "-q"]);
    git(
        temp.path(),
        &["config", "user.email", "anchors@example.test"],
    );
    git(temp.path(), &["config", "user.name", "Anchors Test"]);
    git(temp.path(), &["add", "."]);
    git(temp.path(), &["commit", "-qm", "initial"]);
    temp
}

/// Binary-wide temp cache: anchors runs scan through the env-resolved cache,
/// so every spawned `loct` must point `LOCT_CACHE_DIR` away from the
/// operator-global cache (`~/Library/Caches/loctree`).
fn test_cache_dir() -> &'static Path {
    static CACHE: std::sync::LazyLock<TempDir> =
        std::sync::LazyLock::new(|| TempDir::new().expect("shared test cache dir"));
    CACHE.path()
}

fn anchors(root: &Path, fresh: bool) -> Vec<u8> {
    let mut command = Command::new(env!("CARGO_BIN_EXE_loct"));
    command
        .current_dir(root)
        .env("LOCT_CACHE_DIR", test_cache_dir())
        .args(["anchors", "--format", "json"]);
    if fresh {
        command.arg("--fresh");
    }
    let output = command.output().expect("loct anchors should run");
    assert!(
        output.status.success(),
        "loct anchors failed: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    output.stdout
}

fn assert_schema_contract(document: &Value) {
    let object = document.as_object().expect("catalog object");
    let allowed = [
        "schema",
        "repo_id",
        "snapshot_commit",
        "anchor_catalog_revision",
        "producer_version",
        "anchors",
    ];
    assert!(object.keys().all(|key| allowed.contains(&key.as_str())));
    assert_eq!(document["schema"], "loctree.anchors.v1");
    assert!(document["repo_id"].as_str().is_some_and(|value| {
        !value.is_empty() && !value.starts_with('/') && value.split('/').count() <= 2
    }));
    assert!(document["snapshot_commit"].as_str().is_some_and(|value| {
        (7..=40).contains(&value.len()) && value.bytes().all(|byte| byte.is_ascii_hexdigit())
    }));
    assert!(
        document["anchor_catalog_revision"]
            .as_str()
            .is_some_and(|value| value.starts_with("acr1:") && value.len() == 69)
    );
    assert!(document["producer_version"].as_str().is_some());

    let anchors = document["anchors"].as_array().expect("anchors array");
    assert!(!anchors.is_empty());
    for anchor in anchors {
        let anchor = anchor.as_object().expect("anchor object");
        let allowed = [
            "anchor_id",
            "normalized_path",
            "language",
            "qualified_symbol",
            "signature_hash",
            "aliases",
        ];
        assert!(anchor.keys().all(|key| allowed.contains(&key.as_str())));
        assert!(
            anchor["anchor_id"]
                .as_str()
                .is_some_and(|value| value.starts_with("anc1:") && value.len() == 69)
        );
        assert!(anchor["normalized_path"].as_str().is_some_and(|value| {
            !value.is_empty()
                && !value.starts_with('/')
                && !value.split('/').any(|part| part == "..")
        }));
        assert!(anchor["language"].as_str().is_some_and(|value| {
            value
                .chars()
                .next()
                .is_some_and(|first| first.is_ascii_lowercase())
        }));
        if anchor.contains_key("signature_hash") {
            assert!(anchor.contains_key("qualified_symbol"));
            assert!(
                anchor["signature_hash"]
                    .as_str()
                    .is_some_and(|value| value.starts_with("sig1:") && value.len() == 69)
            );
        }
    }
}

#[test]
fn anchors_are_schema_valid_deterministic_and_stable_after_touch() {
    let repo = fixture_repo();
    let first = anchors(repo.path(), true);
    let second = anchors(repo.path(), false);
    assert_eq!(first, second, "same snapshot must be byte-identical");

    let first_document: Value = serde_json::from_slice(&first).expect("catalog JSON");
    assert_schema_contract(&first_document);
    let ids_before: Vec<_> = first_document["anchors"]
        .as_array()
        .expect("anchors array")
        .iter()
        .map(|anchor| anchor["anchor_id"].as_str().expect("anchor id").to_string())
        .collect();

    let path = repo.path().join("src/a.rs");
    let content = fs::read(&path).expect("read fixture");
    fs::write(&path, content).expect("touch without content change");
    let after_touch: Value =
        serde_json::from_slice(&anchors(repo.path(), true)).expect("catalog after touch");
    let ids_after: Vec<_> = after_touch["anchors"]
        .as_array()
        .expect("anchors array after touch")
        .iter()
        .map(|anchor| {
            anchor["anchor_id"]
                .as_str()
                .expect("anchor id after touch")
                .to_string()
        })
        .collect();
    assert_eq!(ids_before, ids_after);
}

#[test]
fn anchors_git_mv_adds_the_previous_path_as_a_rename_alias() {
    let repo = fixture_repo();
    let before: Value =
        serde_json::from_slice(&anchors(repo.path(), true)).expect("catalog before rename");
    let old_path_id = before["anchors"]
        .as_array()
        .expect("anchors before rename")
        .iter()
        .find(|anchor| {
            anchor["normalized_path"] == "src/a.rs" && anchor.get("qualified_symbol").is_none()
        })
        .expect("old path anchor")["anchor_id"]
        .as_str()
        .expect("old path anchor id")
        .to_string();

    git(repo.path(), &["mv", "src/a.rs", "src/b.rs"]);
    let after: Value =
        serde_json::from_slice(&anchors(repo.path(), true)).expect("catalog after rename");
    let renamed = after["anchors"]
        .as_array()
        .expect("anchors after rename")
        .iter()
        .find(|anchor| {
            anchor["normalized_path"] == "src/b.rs" && anchor.get("qualified_symbol").is_none()
        })
        .expect("renamed path anchor");
    assert_ne!(renamed["anchor_id"], old_path_id);
    assert!(
        renamed["aliases"]
            .as_array()
            .expect("rename aliases")
            .iter()
            .any(|alias| { alias["kind"] == "path" && alias["value"] == "src/a.rs" })
    );
}
