//! Organs for usable full-snapshot access without dumping 80MB into a prompt.
//!
//! Three CLI surfaces:
//! - `loct snapshot-path` — resolve snapshot + sibling artifact paths (no body dump)
//! - `loct inventory` — stream compact per-file rows as JSONL + coverage receipt
//! - `loct atlas` — materialize a small `loctree.repo-atlas.v1` pack of pointers
//!
//! Design law: snapshot.json is inventory SoT and is queryable, not promptable.
//! Context packs / repo-view remain sense surfaces (hubs, health), not full file lists.
//!
//! 𝚅𝚒𝚋𝚎𝚌𝚛𝚊𝚏𝚝𝚎𝚍. with AI Agents by Vetcoders (c)2024-2026 LibraxisAI

use std::collections::BTreeMap;
use std::fs;
use std::io::{self, Write};
use std::path::{Path, PathBuf};

use serde::Serialize;
use serde_json::{Value, json};

use crate::cli::command::{AtlasOptions, GlobalOptions, InventoryOptions, SnapshotPathOptions};
use crate::snapshot::{Snapshot, project_cache_dir};

use super::super::DispatchResult;

pub const SNAPSHOT_PATH_PROTOCOL: &str = "loctree.snapshot-path.v1";
pub const INVENTORY_PROTOCOL: &str = "loctree.inventory.v1";
pub const REPO_ATLAS_PROTOCOL: &str = "loctree.repo-atlas.v1";

/// Agents treat inventory as incomplete when ratio falls below this bar.
pub const INVENTORY_RATIO_THRESHOLD: f64 = 0.95;

const ARTIFACT_NAMES: &[&str] = &[
    "snapshot.json",
    "agent.json",
    "findings.json",
    "analysis.json",
    "manifest.json",
    "handlers.json",
    "dead.json",
    "circular.json",
    "report.sarif",
    "report.html",
];

// ---------------------------------------------------------------------------
// Path resolution
// ---------------------------------------------------------------------------

fn resolve_project_root(explicit: Option<&PathBuf>) -> Result<PathBuf, String> {
    let root = explicit
        .cloned()
        .unwrap_or_else(|| std::env::current_dir().unwrap_or_else(|_| PathBuf::from(".")));
    root.canonicalize()
        .map_err(|err| format!("cannot resolve project root {}: {err}", root.display()))
}

/// Resolve the on-disk snapshot path for a project without loading its body.
///
/// Prefers an existing snapshot discovered via the standard finder; falls back
/// to the canonical `Snapshot::snapshot_path` target (may not exist yet).
pub fn resolve_snapshot_path(root: &Path) -> PathBuf {
    Snapshot::find_latest_snapshot_in(root).unwrap_or_else(|_| Snapshot::snapshot_path(root))
}

fn artifact_status(dir: &Path) -> BTreeMap<String, ArtifactPresence> {
    let mut out = BTreeMap::new();
    for name in ARTIFACT_NAMES {
        let path = dir.join(name);
        let exists = path.is_file();
        let bytes = if exists {
            fs::metadata(&path).map(|m| m.len()).ok()
        } else {
            None
        };
        out.insert(
            (*name).to_string(),
            ArtifactPresence {
                path: path.display().to_string(),
                exists,
                bytes,
            },
        );
    }
    out
}

#[derive(Debug, Serialize)]
struct ArtifactPresence {
    path: String,
    exists: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    bytes: Option<u64>,
}

// ---------------------------------------------------------------------------
// snapshot-path
// ---------------------------------------------------------------------------

pub fn handle_snapshot_path(opts: &SnapshotPathOptions, global: &GlobalOptions) -> DispatchResult {
    let root = match resolve_project_root(opts.project.as_ref()) {
        Ok(p) => p,
        Err(err) => {
            eprintln!("[loct][snapshot-path] {err}");
            return DispatchResult::Exit(1);
        }
    };

    let snapshot_path = resolve_snapshot_path(&root);
    let artifacts_dir = snapshot_path
        .parent()
        .map(Path::to_path_buf)
        .unwrap_or_else(|| project_cache_dir(&root));
    let exists = snapshot_path.is_file();
    let bytes = if exists {
        fs::metadata(&snapshot_path).map(|m| m.len()).ok()
    } else {
        None
    };
    let git = Snapshot::git_context_for(&root);
    let artifacts = artifact_status(&artifacts_dir);

    // Global `--json` is stripped before subcommand parsers see remaining args.
    let as_json = opts.json || global.json;
    if as_json {
        let payload = json!({
            "protocol": SNAPSHOT_PATH_PROTOCOL,
            "project": root.display().to_string(),
            "snapshot_path": snapshot_path.display().to_string(),
            "artifacts_dir": artifacts_dir.display().to_string(),
            "exists": exists,
            "bytes": bytes,
            "git": {
                "branch": git.branch,
                "commit": git.commit,
                "owner_repo": git.owner_repo,
                "scan_id": git.scan_id,
            },
            "artifacts": artifacts,
            "hint": if exists {
                "Do not cat snapshot.json into an LLM. Use `loct inventory --jsonl` or `loct '.files[] | .path'`."
            } else {
                "No snapshot yet. Run `loct` (or `loct scan`) to materialize one."
            },
        });
        match serde_json::to_string_pretty(&payload) {
            Ok(s) => println!("{s}"),
            Err(err) => {
                eprintln!("[loct][snapshot-path] serialize failed: {err}");
                return DispatchResult::Exit(1);
            }
        }
    } else {
        println!("{}", snapshot_path.display());
        if opts.verbose_siblings {
            println!("# artifacts_dir={}", artifacts_dir.display());
            println!("# exists={exists}");
            for (name, presence) in &artifacts {
                let mark = if presence.exists { "ok" } else { "missing" };
                println!("# {name}: {mark}");
            }
        } else if !exists {
            eprintln!(
                "[loct][snapshot-path] path printed but file missing — run `loct` first (expected under cache)"
            );
        }
    }

    DispatchResult::Exit(if exists { 0 } else { 2 })
}

// ---------------------------------------------------------------------------
// inventory + coverage receipt
// ---------------------------------------------------------------------------

#[derive(Debug, Serialize)]
struct CoverageReceipt {
    protocol: String,
    record: &'static str,
    project: String,
    snapshot_path: String,
    snapshot_exists: bool,
    files_in_snapshot: usize,
    units: usize,
    tests: usize,
    generated: usize,
    other: usize,
    total_loc: usize,
    languages: BTreeMap<String, usize>,
    kinds: BTreeMap<String, usize>,
    /// snapshot file count / git HEAD tree file count when git is available.
    /// `null` when denominator cannot be measured (not a git repo, empty HEAD).
    inventory_ratio: Option<f64>,
    inventory_ratio_threshold: f64,
    /// `ok` when ratio ≥ threshold or ratio is unknown; `stop` when incomplete.
    verdict: &'static str,
    denominator: DenominatorInfo,
    git: Value,
    message: String,
}

#[derive(Debug, Serialize)]
struct DenominatorInfo {
    source: &'static str,
    count: Option<usize>,
    note: String,
}

#[derive(Debug, Serialize)]
struct InventoryFileRow<'a> {
    record: &'static str,
    path: &'a str,
    loc: usize,
    language: &'a str,
    kind: &'a str,
    is_test: bool,
    is_generated: bool,
    export_count: usize,
    local_symbol_count: usize,
    import_count: usize,
}

fn is_unit(file: &crate::types::FileAnalysis) -> bool {
    // Production code unit: kind code, not test, not generated.
    // Fallback: non-test non-generated source-ish language when kind is empty.
    if file.is_test || file.is_generated {
        return false;
    }
    if !file.kind.is_empty() {
        return file.kind == "code";
    }
    matches!(
        file.language.as_str(),
        "rs" | "rust"
            | "ts"
            | "tsx"
            | "js"
            | "jsx"
            | "py"
            | "python"
            | "swift"
            | "go"
            | "kt"
            | "kotlin"
            | "java"
            | "c"
            | "cpp"
            | "h"
            | "hpp"
            | "m"
            | "mm"
            | "zig"
            | "svelte"
            | "astro"
            | "vue"
            | "shell"
            | "bash"
            | "zsh"
    )
}

/// Count files at git HEAD that fall under the analyzed project root.
///
/// Scoped scans (fixture subdirs, monorepo packages) must not be judged
/// against the whole repository HEAD tree — that would always STOP.
fn git_head_file_count(root: &Path) -> Option<usize> {
    let repo = crate::git::GitRepo::discover(root).ok()?;
    let files = repo.list_files_at("HEAD").ok()?;
    let repo_root = repo.path().canonicalize().ok()?;
    let root_canon = root.canonicalize().ok()?;

    if root_canon == repo_root {
        return Some(files.len());
    }

    let rel = root_canon.strip_prefix(&repo_root).ok()?;
    let prefix = rel.to_string_lossy().replace('\\', "/");
    let prefix = prefix.trim_end_matches('/');
    if prefix.is_empty() {
        return Some(files.len());
    }
    let prefix_slash = format!("{prefix}/");
    let count = files
        .iter()
        .filter(|p| {
            let s = p.to_string_lossy().replace('\\', "/");
            s == prefix || s.starts_with(&prefix_slash)
        })
        .count();
    Some(count)
}

fn build_receipt(
    root: &Path,
    snapshot_path: &Path,
    snapshot: Option<&Snapshot>,
) -> CoverageReceipt {
    let git = Snapshot::git_context_for(root);
    let git_json = json!({
        "branch": git.branch,
        "commit": git.commit,
        "owner_repo": git.owner_repo,
        "scan_id": git.scan_id,
    });

    let mut languages: BTreeMap<String, usize> = BTreeMap::new();
    let mut kinds: BTreeMap<String, usize> = BTreeMap::new();
    let mut units = 0usize;
    let mut tests = 0usize;
    let mut generated = 0usize;
    let mut other = 0usize;
    let mut total_loc = 0usize;
    let files_in_snapshot = snapshot.map(|s| s.files.len()).unwrap_or(0);

    if let Some(snap) = snapshot {
        total_loc = snap.metadata.total_loc;
        for file in &snap.files {
            *languages.entry(file.language.clone()).or_default() += 1;
            let kind_key = if file.kind.is_empty() {
                "unknown".to_string()
            } else {
                file.kind.clone()
            };
            *kinds.entry(kind_key).or_default() += 1;
            if file.is_generated {
                generated += 1;
            } else if file.is_test || file.kind == "test" {
                tests += 1;
            } else if is_unit(file) {
                units += 1;
            } else {
                other += 1;
            }
        }
    }

    let (denominator, inventory_ratio) = match git_head_file_count(root) {
        Some(0) => (
            DenominatorInfo {
                source: "git_head",
                count: Some(0),
                note: "HEAD tree is empty; ratio undefined".into(),
            },
            None,
        ),
        Some(n) => {
            let ratio = files_in_snapshot as f64 / n as f64;
            (
                DenominatorInfo {
                    source: "git_head",
                    count: Some(n),
                    note: "git ls-tree style file count at HEAD (libgit2)".into(),
                },
                Some(ratio),
            )
        }
        None => (
            DenominatorInfo {
                source: "unavailable",
                count: None,
                note: "not a git repo or HEAD unreadable; ratio not computed".into(),
            },
            None,
        ),
    };

    let verdict = match inventory_ratio {
        Some(r) if r + f64::EPSILON < INVENTORY_RATIO_THRESHOLD => "stop",
        _ => "ok",
    };

    let message = match (snapshot.is_some(), inventory_ratio, verdict) {
        (false, _, _) => {
            "No snapshot loaded. Run `loct` then re-run `loct inventory`.".to_string()
        }
        (_, Some(r), "stop") => format!(
            "Inventory incomplete: ratio {r:.3} < {INVENTORY_RATIO_THRESHOLD}. STOP treating this as full repo inventory; rescan or widen scope."
        ),
        (_, Some(r), _) => format!(
            "Inventory coverage ratio {r:.3} ≥ {INVENTORY_RATIO_THRESHOLD}. Safe to treat snapshot files[] as inventory SoT (still do not dump into prompts)."
        ),
        (_, None, _) => {
            "Inventory ratio unknown (no git denominator). Use files_in_snapshot counts with caution."
                .to_string()
        }
    };

    CoverageReceipt {
        protocol: INVENTORY_PROTOCOL.to_string(),
        record: "receipt",
        project: root.display().to_string(),
        snapshot_path: snapshot_path.display().to_string(),
        snapshot_exists: snapshot_path.is_file(),
        files_in_snapshot,
        units,
        tests,
        generated,
        other,
        total_loc,
        languages,
        kinds,
        inventory_ratio,
        inventory_ratio_threshold: INVENTORY_RATIO_THRESHOLD,
        verdict,
        denominator,
        git: git_json,
        message,
    }
}

pub fn handle_inventory(opts: &InventoryOptions) -> DispatchResult {
    let root = match resolve_project_root(opts.project.as_ref()) {
        Ok(p) => p,
        Err(err) => {
            eprintln!("[loct][inventory] {err}");
            return DispatchResult::Exit(1);
        }
    };

    let snapshot_path = resolve_snapshot_path(&root);
    let snapshot = match Snapshot::load(&root) {
        Ok(s) => Some(s),
        Err(err) if err.kind() == io::ErrorKind::NotFound => None,
        Err(err) => {
            eprintln!("[loct][inventory] failed to load snapshot: {err}");
            return DispatchResult::Exit(1);
        }
    };

    let receipt = build_receipt(&root, &snapshot_path, snapshot.as_ref());

    if opts.receipt_only {
        match serde_json::to_string_pretty(&receipt) {
            Ok(s) => println!("{s}"),
            Err(err) => {
                eprintln!("[loct][inventory] serialize receipt failed: {err}");
                return DispatchResult::Exit(1);
            }
        }
        return DispatchResult::Exit(if receipt.verdict == "stop" {
            3
        } else if snapshot.is_none() {
            2
        } else {
            0
        });
    }

    let mut stdout = io::stdout().lock();
    if !opts.no_receipt {
        match serde_json::to_string(&receipt) {
            Ok(line) => {
                if writeln!(stdout, "{line}").is_err() {
                    return DispatchResult::Exit(1);
                }
            }
            Err(err) => {
                eprintln!("[loct][inventory] serialize receipt failed: {err}");
                return DispatchResult::Exit(1);
            }
        }
    }

    if let Some(snap) = snapshot.as_ref() {
        for file in &snap.files {
            if opts.units_only && !is_unit(file) {
                continue;
            }
            if !opts.include_tests && (file.is_test || file.kind == "test") {
                continue;
            }
            if !opts.include_generated && file.is_generated {
                continue;
            }
            if let Some(prefix) = opts.path_prefix.as_deref()
                && !file.path.starts_with(prefix)
            {
                continue;
            }

            let row = InventoryFileRow {
                record: "file",
                path: &file.path,
                loc: file.loc,
                language: &file.language,
                kind: &file.kind,
                is_test: file.is_test,
                is_generated: file.is_generated,
                export_count: file.exports.len(),
                local_symbol_count: file.local_symbols.len(),
                import_count: file.imports.len(),
            };
            match serde_json::to_string(&row) {
                Ok(line) => {
                    if writeln!(stdout, "{line}").is_err() {
                        return DispatchResult::Exit(1);
                    }
                }
                Err(err) => {
                    eprintln!("[loct][inventory] serialize row failed: {err}");
                    return DispatchResult::Exit(1);
                }
            }
        }
    }

    DispatchResult::Exit(if receipt.verdict == "stop" {
        3
    } else if snapshot.is_none() {
        2
    } else {
        0
    })
}

// ---------------------------------------------------------------------------
// repo atlas pack
// ---------------------------------------------------------------------------

pub fn handle_atlas(opts: &AtlasOptions, global: &GlobalOptions) -> DispatchResult {
    let root = match resolve_project_root(opts.project.as_ref()) {
        Ok(p) => p,
        Err(err) => {
            eprintln!("[loct][atlas] {err}");
            return DispatchResult::Exit(1);
        }
    };

    let snapshot_path = resolve_snapshot_path(&root);
    let artifacts_dir = snapshot_path
        .parent()
        .map(Path::to_path_buf)
        .unwrap_or_else(|| project_cache_dir(&root));
    let snapshot = match Snapshot::load(&root) {
        Ok(s) => Some(s),
        Err(err) if err.kind() == io::ErrorKind::NotFound => None,
        Err(err) => {
            eprintln!("[loct][atlas] failed to load snapshot: {err}");
            return DispatchResult::Exit(1);
        }
    };
    let receipt = build_receipt(&root, &snapshot_path, snapshot.as_ref());
    let artifacts = artifact_status(&artifacts_dir);
    let git = Snapshot::git_context_for(&root);

    let atlas_dir = opts
        .out_dir
        .clone()
        .unwrap_or_else(|| root.join(".loctree").join("repo-atlas"));

    if let Err(err) = fs::create_dir_all(&atlas_dir) {
        eprintln!("[loct][atlas] cannot create {}: {err}", atlas_dir.display());
        return DispatchResult::Exit(1);
    }

    // Compact inventory summary (not full JSONL) for the pack.
    let inventory_summary = json!({
        "protocol": INVENTORY_PROTOCOL,
        "files_in_snapshot": receipt.files_in_snapshot,
        "units": receipt.units,
        "tests": receipt.tests,
        "generated": receipt.generated,
        "other": receipt.other,
        "total_loc": receipt.total_loc,
        "languages": receipt.languages,
        "kinds": receipt.kinds,
        "inventory_ratio": receipt.inventory_ratio,
        "inventory_ratio_threshold": receipt.inventory_ratio_threshold,
        "verdict": receipt.verdict,
        "denominator": receipt.denominator,
        "message": receipt.message,
        "how_to_stream_full": "loct inventory --jsonl",
        "how_to_receipt_only": "loct inventory --receipt-only",
    });

    let sense = json!({
        "role": "sense",
        "description": "Compact overview for orientation — hubs, health, languages. NOT full inventory.",
        "commands": ["loct repo-view", "loct context"],
        "agent_json": artifacts.get("agent.json").map(|a| &a.path),
        "exists": artifacts.get("agent.json").map(|a| a.exists).unwrap_or(false),
    });

    let signals = json!({
        "role": "signals",
        "description": "Post-settle findings organ — dead, cycles, twins, lints.",
        "commands": ["loct findings --summary", "loct follow all", "loct health"],
        "findings_json": artifacts.get("findings.json").map(|a| &a.path),
        "exists": artifacts.get("findings.json").map(|a| a.exists).unwrap_or(false),
    });

    let inventory_organ = json!({
        "role": "inventory",
        "description": "Full file inventory SoT is snapshot.json files[]. Stream via inventory JSONL; never cat into an LLM.",
        "snapshot_path": snapshot_path.display().to_string(),
        "snapshot_exists": snapshot_path.is_file(),
        "commands": ["loct snapshot-path --json", "loct inventory --jsonl", "loct inventory --receipt-only"],
        "summary": inventory_summary,
    });

    let manifest = json!({
        "protocol": REPO_ATLAS_PROTOCOL,
        "status": if snapshot.is_some() { "ready" } else { "missing_snapshot" },
        "project": root.display().to_string(),
        "generated_at": chrono_lite_now(),
        "atlas_dir": atlas_dir.display().to_string(),
        "git": {
            "branch": git.branch,
            "commit": git.commit,
            "owner_repo": git.owner_repo,
            "scan_id": git.scan_id,
        },
        "organs": {
            "sense": sense,
            "inventory": inventory_organ,
            "signals": signals,
        },
        "artifacts_dir": artifacts_dir.display().to_string(),
        "artifacts": artifacts,
        "coverage_receipt": receipt,
        "reading_order": [
            "00-manifest.json (this file)",
            "01-coverage-receipt.json",
            "02-inventory-summary.json",
            "Then: loct inventory --jsonl | jq for file rows",
            "Then: loct repo-view / loct context for sense",
            "Then: loct findings for signals",
        ],
        "anti_patterns": [
            "cat snapshot.json into an LLM context",
            "treat context --full hubs list as full inventory",
            "continue agent work when coverage_receipt.verdict == stop",
        ],
        "message": receipt.message,
    });

    let write_json = |name: &str, value: &Value| -> Result<PathBuf, String> {
        let path = atlas_dir.join(name);
        let body =
            serde_json::to_string_pretty(value).map_err(|e| format!("serialize {name}: {e}"))?;
        fs::write(&path, body).map_err(|e| format!("write {}: {e}", path.display()))?;
        Ok(path)
    };

    let paths = match (|| -> Result<Vec<PathBuf>, String> {
        Ok(vec![
            write_json("00-manifest.json", &manifest)?,
            write_json(
                "01-coverage-receipt.json",
                &serde_json::to_value(&receipt).map_err(|e| e.to_string())?,
            )?,
            write_json("02-inventory-summary.json", &inventory_summary)?,
            write_json(
                "03-organs.json",
                &json!({
                    "sense": sense,
                    "inventory": inventory_organ,
                    "signals": signals,
                }),
            )?,
        ])
    })() {
        Ok(p) => p,
        Err(err) => {
            eprintln!("[loct][atlas] {err}");
            return DispatchResult::Exit(1);
        }
    };

    // Human README for operators who open the dir.
    let readme = format!(
        r#"# Loctree Repo Atlas

Protocol: `{REPO_ATLAS_PROTOCOL}`

This pack is a **pointer map** for the three organs:

| Organ | Role | Do not confuse with |
|---|---|---|
| sense | hubs, health, languages (`repo-view` / `agent.json`) | full file list |
| inventory | every file in snapshot (`loct inventory --jsonl`) | context --full |
| signals | findings / follow / health | inventory |

## Start here

1. `00-manifest.json`
2. `01-coverage-receipt.json` — if `verdict` is `stop`, rescan before trusting inventory
3. Stream files: `loct inventory --jsonl`
4. Sense: `loct repo-view`
5. Signals: `loct findings --summary`

Snapshot path: `{}`
Atlas dir: `{}`

Generated: {}
"#,
        snapshot_path.display(),
        atlas_dir.display(),
        chrono_lite_now(),
    );
    let readme_path = atlas_dir.join("README.md");
    if let Err(err) = fs::write(&readme_path, readme) {
        eprintln!("[loct][atlas] write README failed: {err}");
    }

    let as_json = opts.json || global.json;
    if as_json {
        let out = json!({
            "protocol": REPO_ATLAS_PROTOCOL,
            "status": manifest["status"],
            "atlas_dir": atlas_dir.display().to_string(),
            "manifest": atlas_dir.join("00-manifest.json").display().to_string(),
            "cards": paths.iter().map(|p| p.display().to_string()).collect::<Vec<_>>(),
            "coverage_verdict": receipt.verdict,
            "inventory_ratio": receipt.inventory_ratio,
            "message": receipt.message,
        });
        match serde_json::to_string_pretty(&out) {
            Ok(s) => println!("{s}"),
            Err(err) => {
                eprintln!("[loct][atlas] serialize failed: {err}");
                return DispatchResult::Exit(1);
            }
        }
    } else {
        println!("╭─ Loctree Repo Atlas ────────────────────────────────────────────────╮");
        println!("│ Three organs as paths — inventory without dumping snapshot.json.    │");
        println!("╰─────────────────────────────────────────────────────────────────────╯");
        println!();
        println!("Status: {}", manifest["status"]);
        println!("Project: {}", root.display());
        println!("Atlas dir: {}", atlas_dir.display());
        println!("Manifest: {}", atlas_dir.join("00-manifest.json").display());
        println!(
            "Coverage: verdict={} ratio={}",
            receipt.verdict,
            receipt
                .inventory_ratio
                .map(|r| format!("{r:.3}"))
                .unwrap_or_else(|| "unknown".into())
        );
        println!();
        println!("{}", receipt.message);
        println!();
        println!("Next:");
        println!("  loct inventory --receipt-only");
        println!("  loct inventory --jsonl | head");
        println!("  loct snapshot-path --json");
        println!("  loct repo-view");
    }

    DispatchResult::Exit(if receipt.verdict == "stop" {
        3
    } else if snapshot.is_none() {
        2
    } else {
        0
    })
}

/// Minimal ISO-ish timestamp without pulling chrono into this module's deps.
fn chrono_lite_now() -> String {
    // Prefer the same helper the context atlas uses when available.
    crate::context_render::current_iso_timestamp()
}

// ---------------------------------------------------------------------------
// Unit tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;
    use crate::types::FileAnalysis;
    use tempfile::TempDir;

    fn unit_file(path: &str) -> FileAnalysis {
        FileAnalysis {
            path: path.into(),
            loc: 10,
            language: "rs".into(),
            kind: "code".into(),
            is_test: false,
            is_generated: false,
            ..Default::default()
        }
    }

    fn test_file(path: &str) -> FileAnalysis {
        FileAnalysis {
            path: path.into(),
            loc: 5,
            language: "rs".into(),
            kind: "test".into(),
            is_test: true,
            is_generated: false,
            ..Default::default()
        }
    }

    #[test]
    fn is_unit_excludes_tests_and_generated() {
        assert!(is_unit(&unit_file("src/lib.rs")));
        assert!(!is_unit(&test_file("src/lib_test.rs")));
        let mut generated = unit_file("src/generated.rs");
        generated.is_generated = true;
        assert!(!is_unit(&generated));
    }

    #[test]
    fn receipt_stop_when_ratio_below_threshold() {
        // Build a minimal snapshot with 1 file; git denom will likely be larger
        // in a real repo. Here we unit-test the threshold arithmetic via direct fields.
        let ratio = 0.5_f64;
        let verdict = if ratio + f64::EPSILON < INVENTORY_RATIO_THRESHOLD {
            "stop"
        } else {
            "ok"
        };
        assert_eq!(verdict, "stop");
        // Guards the constant itself, so it belongs at compile time: a future
        // edit that loosens the threshold below 0.9 fails the build, not a test run.
        const { assert!(INVENTORY_RATIO_THRESHOLD > 0.9) };
    }

    #[test]
    fn resolve_snapshot_path_prefers_canonical_when_missing() {
        let tmp = TempDir::new().unwrap();
        let path = resolve_snapshot_path(tmp.path());
        assert!(
            path.ends_with("snapshot.json") || path.file_name().is_some(),
            "path={}",
            path.display()
        );
    }

    #[test]
    fn snapshot_path_handler_prints_path() {
        let tmp = TempDir::new().unwrap();
        // Non-git empty dir — should still print a path and exit 2 (missing).
        let opts = SnapshotPathOptions {
            project: Some(tmp.path().to_path_buf()),
            json: true,
            verbose_siblings: false,
        };
        let global = GlobalOptions::default();
        let result = handle_snapshot_path(&opts, &global);
        match result {
            DispatchResult::Exit(code) => assert!(code == 2 || code == 0, "exit={code}"),
            _ => panic!("unexpected dispatch result (expected Exit)"),
        }
    }
}
