# makieta-eval — E1-01b value proof (live model + judge)

A/B harness measuring whether the makieta (dense cards + AICX intent overlay) makes the agent factually better on control tasks.

## Scope (per brief v4)
- Arm A: pre-makieta (cards regenerated from pinned commit before M1-01 / dense+intent; no intent-map).
- Arm B: full current makieta (post M1-01, this snapshot).
- Reuse: forcefeed-probe + atlas_factset_check.py for payload verification (both arms get full intended content, not truncation).
- Live model: same pinned version invoked via `claude -p` (non-interactive).
- Judge: second model call with this rubric; verdicts + justifications recorded.
- Canaries: seeded via fixture (this dir), injected only into B payload memory trail.
- Metrics: canary_recall (B), false_intent_rate (B), false_supersede_rate (B), decision_accuracy delta A→B.
- Thresholds for delivery: recall≥0.8, FI≤0.1, FS≤0.1, delta>0.
- Reduced matrix allowed when budget tight: note explicitly; do not fake full coverage.
- Territory: ONLY tools/forcefeed-probe/ + tools/makieta-eval/ . NEVER touch atlas.rs / pack.rs / overlay.rs / core context generators.

## Usage
```bash
# Dry-run with cost estimate (no model calls)
tools/makieta-eval/run.sh --out /tmp/makieta-eval.json --dry-run

# Full (or reduced) with live calls — budget conscious
tools/makieta-eval/run.sh --out /tmp/makieta-eval.json --live --tasks 3 --repos 1
```

The script prints cost estimate BEFORE any live calls.

## Acceptance (from operator brief)
- Harness runs live model on both arms; payloads verified by probe logic.
- ≥5 tasks (or reduced with note) × repos.
- Judge verdicts with justifications in artifact.
- Measured metrics + delta.
- Progs or [!] with numbers.
- Report table + cost.
- shellcheck clean.
- Rubric reviewable in repo.

## Pinned for A
8d5feffd (pre-dense markdown synthesis + intent overlay introduction). Verified via `git` + loctree that A payload carries no intent entries.

## Living Tree + Loctree
- Re-read touched files before edit.
- loct / loctree-mcp first for any structural question.
- Commit packs 5-6 files.
- Titles: `[<agent>/vc-workflow] ...`
- After tests: `loct scan` before any atlas regen in harness (pollution guard).

## Recovery
Blocked by E1-01a (forcefeed proof). This is the value measurement that E1-01 promised.

## Out of scope (this cut)
- Tuning cards from results.
- Runner fixes.
- Full 2-repo 500k scale run (use reduced + explicit extrapolation note).
- Operator push / publish of results.

## Files
- run.sh — orchestrator + live calls + verifier json
- rubric.md — judge rubric (binary criteria)
- tasks.json — control tasks (decision recall, anti-rec, supersede traps, canary)
- canaries.json — seeded theses (fixture store)
- (future) judge.py if extracted

See run.sh header for exact matrix and cost model.
