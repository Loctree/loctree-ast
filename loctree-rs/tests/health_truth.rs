//! One number, one truth — cross-surface health contract.
//!
//! Three surfaces published three health numbers for the same commit
//! (df35a677): `loct findings --summary` said 85, `loct --for-ai` said 72,
//! `audit_report.md` said something else again. All three already called the
//! one canonical scorer — ffc13063 made sure of that — so the drift could only
//! be in the argument, and it was: every defect gate a later wave added landed
//! in `findings.rs` alone.
//!
//! The pre-existing test for this (`audit_report::test_audit_health_matches_
//! canonical_scorer`) compared `audit_health(f)` against
//! `calculate_health_score(health_metrics_from_audit(f))` — the definition of
//! `audit_health`. A tautology cannot catch a split-brain. These tests compare
//! surfaces against *each other*, which is the only comparison that can.
//!
//! 𝚅𝚒𝚋𝚎𝚌𝚛𝚊𝚏𝚝𝚎𝚍. with AI Agents by Vetcoders (c)2024-2026 LibraxisAI

use assert_cmd::Command;
use assert_cmd::cargo::cargo_bin_cmd;
use serde_json::Value;
use std::path::PathBuf;
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

/// `loct --for-ai` prints a human banner before the JSON bundle.
fn json_tail(stdout: &str) -> Value {
    let start = stdout
        .find('{')
        .unwrap_or_else(|| panic!("no JSON object in output: {stdout}"));
    serde_json::from_str(&stdout[start..]).expect("for-ai bundle parses as JSON")
}

/// The behavioural half: two surfaces, one snapshot, one number.
///
/// This runs the real commands from the operator-facing path, not the library
/// internals — a split-brain that only shows up after CLI assembly (a
/// different snapshot, a different scan scope) still fails here.
///
/// Honest limit: measured against the pre-fix binary these fixtures agreed
/// already (82/82 on `dead_code`) — they are too small to hold classified
/// cycles, namesake duplicate groups or barrel chaos, which is where the real
/// 85-vs-72 drift lived. This is a smoke check on the operator-facing path.
/// The teeth are the source contract below and the gate-level unit tests in
/// `analyzer::health_inputs`, which pin the classification directly.
#[test]
fn findings_summary_and_agent_bundle_report_the_same_health() {
    // A fixture with real structure to score: cycles, duplicate exports and
    // dead code. Scoring an empty tree would agree trivially at 100.
    for fixture in ["circular_imports", "dead_code", "simple_ts"] {
        let cache = TempDir::new().expect("isolated cache");
        let root = fixtures_path().join(fixture);

        loct(cache.path())
            .arg("scan")
            .current_dir(&root)
            .assert()
            .success();

        let findings = loct(cache.path())
            .args(["findings", "--summary", "--json"])
            .current_dir(&root)
            .output()
            .expect("findings --summary --json");
        let findings: Value =
            serde_json::from_slice(&findings.stdout).expect("findings summary parses as JSON");

        let bundle = loct(cache.path())
            .arg("--for-ai")
            .current_dir(&root)
            .output()
            .expect("--for-ai");
        let bundle = json_tail(&String::from_utf8_lossy(&bundle.stdout));

        let from_findings = findings["health_score"].as_u64();
        let from_bundle = bundle["summary"]["health_score"].as_u64();

        assert!(
            from_findings.is_some() && from_bundle.is_some(),
            "[{fixture}] both surfaces must publish a health number, got \
             findings={from_findings:?} agent.json={from_bundle:?}"
        );
        assert_eq!(
            from_findings, from_bundle,
            "[{fixture}] split-brain scorer: `loct findings --summary` says \
             {from_findings:?} while `loct --for-ai` says {from_bundle:?} for the \
             same snapshot. Both must build their HealthMetrics through \
             `analyzer::health_inputs::structural_defects`."
        );
    }
}

/// The structural half: the reason the split came back twice.
///
/// A behavioural assertion catches today's drift. This catches tomorrow's: a
/// producer that hand-rolls its own `HealthMetrics` from local counters is
/// exactly how W1-03 and W2-01 hardened one surface and left the other
/// scoring ungated sensor rows. Building the vector anywhere but
/// `health_inputs.rs` is the defect, whatever number it happens to produce.
#[test]
fn every_scored_surface_builds_its_vector_in_health_inputs() {
    let findings = include_str!("../src/analyzer/findings.rs");
    let for_ai = include_str!("../src/analyzer/for_ai.rs");
    let audit = include_str!("../src/analyzer/audit_report.rs");
    let inputs = include_str!("../src/analyzer/health_inputs.rs");

    assert!(
        findings.contains("structural_defects(") && !findings.contains("HealthMetrics {"),
        "findings.rs must build its health vector via structural_defects(), \
         never inline"
    );
    assert!(
        for_ai.contains("structural_defects("),
        "for_ai.rs must build its health vector via structural_defects()"
    );

    // The dead-export defect gate had three homes and two of them agreed by
    // copy-paste. One definition only, in health_inputs.rs.
    assert!(
        inputs.contains("pub fn counts_as_dead_defect"),
        "the dead-export defect gate lives in health_inputs.rs"
    );
    for (name, src) in [("findings.rs", findings), ("audit_report.rs", audit)] {
        assert!(
            !src.contains("fn counts_as_dead_defect"),
            "{name} must import counts_as_dead_defect, not redefine it"
        );
    }

    // The audit collector genuinely cannot see the full vector. That is
    // allowed — a silent third number is not.
    assert!(
        audit.contains("AUDIT_BASIS") && audit.contains("STRUCTURAL_BASIS"),
        "audit_report.rs must name how its metric differs"
    );
}

/// The allowlist is the leak.
///
/// `every_scored_surface_builds_its_vector_in_health_inputs` names the three
/// producers it knows about. That is the same shape as the bug it guards: the
/// fix travels to the surfaces someone remembered, and a fourth one keeps
/// scoring raw counters. `analyzer/output.rs` was exactly that — it fed the
/// HTML report's health gauge from ungated section counts, so on 257a5f82 the
/// gauge read 71 while `findings --summary` and `--for-ai` both said 84.
///
/// So this test inverts the default: building a `HealthMetrics` anywhere under
/// `analyzer/` is a defect unless the file is on a list of *justified*
/// exceptions. A new producer fails this test on the day it is written, which
/// is the only moment the split is cheap to prevent.
#[test]
fn no_unlisted_analyzer_surface_builds_its_own_health_vector() {
    // Why each exception is allowed to construct the vector directly:
    //
    // * health_score.rs   — defines `HealthMetrics` and the formula's tests.
    // * health_inputs.rs  — the canonical builder itself (`metrics()`).
    // * audit_report.rs   — a strictly narrower collector that names its
    //                       difference via `AUDIT_BASIS` instead of publishing
    //                       a silent competing figure.
    // * for_ai.rs         — sectionless/unit-test fallback for when there is no
    //                       snapshot to read the canonical vector from. Known
    //                       residual: that path is not labelled the way the
    //                       audit basis is.
    const JUSTIFIED: &[&str] = &[
        "health_score.rs",
        "health_inputs.rs",
        "audit_report.rs",
        "for_ai.rs",
    ];

    let analyzer = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("src/analyzer");
    let mut offenders = Vec::new();
    let mut stack = vec![analyzer.clone()];
    while let Some(dir) = stack.pop() {
        for entry in std::fs::read_dir(&dir).expect("analyzer dir is readable") {
            let path = entry.expect("readable dir entry").path();
            if path.is_dir() {
                stack.push(path);
                continue;
            }
            if path.extension().and_then(|e| e.to_str()) != Some("rs") {
                continue;
            }
            let name = path
                .file_name()
                .and_then(|n| n.to_str())
                .unwrap_or_default()
                .to_string();
            if JUSTIFIED.contains(&name.as_str()) {
                continue;
            }
            let src = std::fs::read_to_string(&path).expect("source file is readable");
            if src.contains("HealthMetrics {") {
                offenders.push(
                    path.strip_prefix(&analyzer)
                        .unwrap_or(&path)
                        .display()
                        .to_string(),
                );
            }
        }
    }
    offenders.sort();
    assert!(
        offenders.is_empty(),
        "these analyzer surfaces build a health vector inline instead of \
         calling health_inputs::structural_defects(): {offenders:?} — one \
         scorer means one set of gates, and an inline vector skips them"
    );
}

/// The report gauge specifically: it must read the canonical builder.
///
/// Pinned separately from the sweep above so a regression names the surface
/// that actually regressed rather than a generic list.
#[test]
fn report_gauge_scores_the_canonical_vector() {
    let output = include_str!("../src/analyzer/output.rs");
    assert!(
        output.contains("structural_defects("),
        "output.rs must build the report gauge's vector via structural_defects()"
    );
    assert!(
        !output.contains("HealthMetrics {"),
        "output.rs must not hand-roll a HealthMetrics from section counters"
    );
}
