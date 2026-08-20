#!/usr/bin/env python3
"""grep-impossible — the eval-set that measures what grep can NEVER answer.

Parity suites measure how well `loct find` imitates grep. This suite measures
the opposite axis: questions whose answers require a dependency graph, symbol
roles, runtime semantics, or coverage accounting — surfaces where text search
has no move at all. Every question is a live command against this repository
(or a bundled fixture) with a machine-checked assertion.

Run:  python3 evals/grep-impossible/run.py            # scorecard, exit 1 on fail
      python3 evals/grep-impossible/run.py --list     # questions only
"""

from __future__ import annotations

import json
import os
import subprocess
import sys
from pathlib import Path

REPO = Path(__file__).resolve().parents[2]
FIXTURES = REPO / "loctree-rs" / "tests" / "fixtures"


def loct_bin() -> str:
    """Resolve the binary UNDER TEST — the repo build, not whatever is on PATH.

    This suite measures THIS engine. Bare `loct` resolves through PATH, which
    on a fleet machine answers with whichever sibling cut installed last; a
    green run then proves nothing about the working tree. Same guard, same
    rationale as `tools/makieta-eval/run.sh`: LOCT_BIN, else the repo build,
    else PATH as a last resort.

    (Kept character-for-character identical to the sibling cut
    w1-a-body-redirect, ffadc32c — two cuts hit the same wall in the same wave;
    one spelling, so the integrator merges instead of choosing.)
    """
    env_bin = os.environ.get("LOCT_BIN")
    if env_bin and Path(env_bin).is_file() and os.access(env_bin, os.X_OK):
        return env_bin
    for profile in ("release", "debug"):
        candidate = REPO / "target" / profile / "loct"
        if candidate.is_file() and os.access(candidate, os.X_OK):
            return str(candidate)
    return "loct"


LOCT = loct_bin()


def loct(args: list[str], cwd: Path = REPO, timeout: int = 240) -> subprocess.CompletedProcess:
    return subprocess.run(
        [LOCT, *args],
        cwd=cwd,
        capture_output=True,
        text=True,
        timeout=timeout,
        check=False,
        env={**os.environ, "LOCT_DEBUG": ""},
    )


def loct_json(args: list[str], cwd: Path = REPO, timeout: int = 240):
    proc = loct([*args, "--json"], cwd=cwd, timeout=timeout)
    if proc.returncode not in (0, 1):  # 1 = findings present in CI mode
        raise AssertionError(f"exit {proc.returncode}: {proc.stderr[:300]}")
    try:
        return json.loads(proc.stdout)
    except json.JSONDecodeError as exc:
        raise AssertionError(f"non-JSON output: {proc.stdout[:200]!r}") from exc


QUESTIONS: list[dict] = []


def question(qid: str, text: str, why: str, gap: bool = False):
    """gap=True marks a KNOWN product gap: the assertion states the DESIRED
    behavior. A failing gap question reports GAP without failing the run; a
    PASSING gap question fails the run until it is promoted (gap=False) —
    strict-xfail, so fixed engine behavior cannot stay silently unpromoted."""

    def wrap(fn):
        QUESTIONS.append({"id": qid, "text": text, "why": why, "gap": gap, "fn": fn})
        return fn

    return wrap


# ── Blast radius & reverse dependencies ─────────────────────────────────────

@question(
    "blast-radius",
    "If loctree-rs/src/types.rs changes, how many files are affected — directly AND transitively?",
    "grep sees the string `types`, not the import graph; transitive closure needs edges.",
)
def q_blast_radius():
    d = loct_json(["impact", "loctree-rs/src/types.rs"])
    direct = len(d["direct_consumers"])
    total = d["total_affected"]
    assert direct > 0, "types.rs must have direct consumers"
    assert total > direct, f"transitive closure ({total}) must exceed direct ({direct})"
    assert d["max_depth"] >= 2, "depth accounting missing"


@question(
    "reverse-deps",
    "Which files import loctree-rs/src/analyzer/html.rs?",
    "an importer never has to mention the string `html.rs` — module paths, re-exports and aliases hide it from text search.",
)
def q_reverse_deps():
    d = loct_json(["find", "loctree-rs/src/analyzer/html.rs", "--who-imports"])
    total = d.get("total", 0)
    assert total >= 1, f"expected >=1 structured importer, got {total}"


# ── Symbol roles: definition vs mention ─────────────────────────────────────

@question(
    "definition-vs-mention",
    "Where is compute_env_truth DEFINED — as distinct from its dozens of textual mentions?",
    "grep returns every mention with equal weight; role separation needs an AST.",
)
def q_definition():
    where = loct_json(["query", "where-symbol", "compute_env_truth"])
    assert where["total"] == 1, f"expected exactly 1 definition site, got {where['total']}"
    hit = where["results"][0]
    assert hit["file"].endswith("env_truth/mod.rs")


@question(
    "symbol-body",
    "Give me the full source body of compute_env_truth — without knowing the file or offsets.",
    "grep prints matching LINES; a bounded multi-line body needs symbol ranges.",
)
def q_body():
    d = loct_json(["body", "compute_env_truth"])
    b = d["bodies"][0]
    span = b["end_line"] - b["start_line"]
    assert span > 50, f"body span suspiciously small: {span}"
    assert "pub fn compute_env_truth" in b["source"]


# ── Dead code, cycles, twins ────────────────────────────────────────────────

@question(
    "dead-with-reason",
    "Which exports are dead — and WHY does the engine believe that (what was checked)?",
    "grep cannot prove absence of callers, let alone enumerate which call-site classes were checked.",
)
def q_dead():
    d = loct_json(["follow", "dead"])
    assert isinstance(d, list) and d, "expected dead candidates on this repo"
    first = d[0]
    assert first.get("reason") and first.get("confidence"), "each candidate must carry reason + confidence"
    assert "Checked:" in first["reason"], "reason must enumerate what was verified"


@question(
    "cycles-classified",
    "Which import cycles exist, and which are hard bidirectional vs structurally benign?",
    "a cycle is a property of the graph's closure; no text pattern expresses it.",
)
def q_cycles():
    d = loct_json(["follow", "cycles"])
    cycles = d.get("classifiedCycles", [])
    assert cycles, "expected classified cycles"
    assert all("classification" in c for c in cycles)


@question(
    "twins",
    "Which structures are duplicated across files (twins / barrel chaos)?",
    "duplicates rarely share a literal string; shape comparison needs normalized symbols.",
)
def q_twins():
    d = loct_json(["follow", "twins"])
    assert isinstance(d, dict) and d, "expected twin/barrel structures"


# ── Aggregates with doctrine ────────────────────────────────────────────────

@question(
    "health-aggregate",
    "What is the repo's structural health — cycles, dead exports, twins — as one accounted summary?",
    "an aggregate over graph properties; grep has neither the properties nor the accounting.",
)
def q_health():
    d = loct_json(["health"])
    assert "cycles" in d and "dead_exports" in d
    assert d["scope"]["doctrine_note"], "scope doctrine must be explicit"


@question(
    "suppressions-inventory",
    "Every silencer in the repo (nosemgrep/ts-ignore/noqa/allow/unsafe) — by kind, file and line?",
    "grep finds ONE pattern per invocation and knows no kind taxonomy across 9 ecosystems.",
)
def q_suppressions():
    d = loct_json(["suppressions"])
    kinds = {e["kind"] for e in d}
    assert len(d) > 20 and len(kinds) >= 3, f"expected a real inventory, got {len(d)} entries / {len(kinds)} kinds"


# ── Runtime & env truth ─────────────────────────────────────────────────────

@question(
    "env-orphans",
    "Which env vars are DECLARED (dotenv/compose/k8s) but never READ anywhere in source?",
    "requires joining two different worlds — declaration files and code read-sites — grep can't join.",
)
def q_env_orphans():
    d = loct_json(["env-truth"])
    by_kind = d["summary"]["warnings_by_kind"]
    assert by_kind.get("orphan_declaration", 0) >= 1, f"expected orphan declarations, got {by_kind}"
    assert d["summary"]["precedence_table"], "orphan verdicts must rest on an explicit precedence model"


@question(
    "env-read-sites",
    "For every env var: WHERE is it read (file:line) and with what access kind?",
    "std::env::var wrappers, promoted keys and $VAR expansions have no common literal.",
)
def q_env_reads():
    d = loct_json(["env-truth"])
    decls = d["declarations"]
    with_reads = [x for x in decls if x.get("reads")]
    assert with_reads, "expected declarations with structured read sites"
    site = with_reads[0]["reads"][0]
    assert site.get("file"), "read site must carry a file"


# ── Framework semantics ─────────────────────────────────────────────────────

@question(
    "tauri-trace",
    "Is the Tauri handler wired end-to-end: Rust definition → invoke_handler! → frontend invoke()?",
    "the bridge spans two languages and a macro; no single text pattern crosses it.",
)
def q_trace():
    proc = loct(["trace", "greet"], cwd=FIXTURES / "tauri_app")
    out = proc.stdout + proc.stderr
    assert proc.returncode in (0, 1), f"trace failed: {out[:300]}"
    assert "greet" in out and ("register" in out.lower() or "invoke" in out.lower() or "handler" in out.lower()), (
        f"expected an end-to-end wiring verdict, got: {out[:300]}"
    )


@question(
    "prism-smear",
    "Are these two task framings the same work or a conceptual smear? Score it.",
    "semantic overlap of task framings against the code map — not a search at all.",
)
def q_prism():
    d = loct_json(["prism", "--task", "env truth audit", "--task", "env drift report"])
    payload = json.dumps(d)
    assert "score" in payload or "smear" in payload or "band" in payload, f"expected prism scoring, got keys {list(d)[:8]}"


# ── Coverage accounting ─────────────────────────────────────────────────────

@question(
    "absence-with-receipt",
    "Prove the identifier `definitely_not_here_xyzzy` does NOT exist — with a scanned/total receipt.",
    "grep's silence is not evidence (wrong dir? binary skip? glob miss); a denominator makes absence a fact.",
)
def q_absence():
    proc = loct(["find", "definitely_not_here_xyzzy"])
    out = proc.stdout + proc.stderr
    assert "scanned" in out or "universe" in out or "indexed" in out, "absence must ship a coverage receipt"


@question(
    "scoped-fingerprint",
    "Build context for ONE subsystem with a cache fingerprint proving what the scope covered.",
    "grep has no notion of 'what I looked at' — scope identity requires snapshot accounting.",
)
def q_scope():
    d = loct_json(["context", "--scope", "path:loctree-mcp/"], timeout=300)
    payload = json.dumps(d)
    assert "Scoped" in payload or "scope" in payload, "expected scope identity in the context pack"


# ── Known gaps (desired behavior asserted; observed 2026-08-19 by the
# vc-layouty session, verified live here — see README "Known gaps") ──────────

@question(
    "module-redirect",
    "Asked for the body of a MODULE name — do the surfaces converge on a redirect to its symbols?",
    "grep has no concept of 'this is a module, you meant its exports'; the graph knows both.",
)
def q_module_redirect():
    proc = loct(["body", "health_score"])
    out = proc.stdout + proc.stderr
    # Closed 2026-08-19: `mod x;` declarations became definition sites, so
    # where-symbol answers module names and body redirects to the symbols that
    # do have bodies (fuzzy_suggestions → HealthScore rides along).
    assert "HealthScore" in out or "module" in out.lower(), (
        f"dead-end hint instead of a redirect: {out[:200]}"
    )


@question(
    "shape-honesty",
    "Does the shape narrative stay silent when its only 'definition' is a module declaration?",
    "the confabulation is only visible when roles and shape come from the same graph — grep has neither.",
)
def q_shape_honesty():
    proc = loct(["find", "health_score"])
    out = proc.stdout
    # Answered 2026-08-19: the dataflow labels are gated on the sole definition
    # being a *value* declaration, so `pub mod health_score;` no longer gets a
    # state-flag story. Narrative authority must not exceed what the
    # definitions support.
    assert "state flag" not in out and "single_writer" not in out, (
        "module declaration narrated as a state flag"
    )


@question(
    "answers-ranked-first",
    "In 1600+ regex hits, do production definitions outrank docs on the first screen?",
    "ranking by role requires roles; grep emits file order and calls it a day.",
)
def q_ranking():
    d = loct_json(["find", "--regex", "twin\\w+"])
    occ = d["regex_matches"]["occurrences"][:10]
    assert occ, "expected hits"
    non_docs = [h for h in occ if not str(h.get("file", "")).endswith((".md", ".yml"))]
    # Answered 2026-08-19: emission is ranked (scope > role > path/line/col)
    # before paging, so the first screen carries code rather than CHANGELOG.
    assert len(non_docs) >= 3, (
        f"first screen is documentation, not answers: {[h.get('file') for h in occ]}"
    )


@question(
    "one-number-one-truth",
    "Every surface reporting a health number agrees — or names how its metric differs.",
    "reconciling scorers across artifacts is a semantics problem; grep does not even see the artifacts.",
)
def q_health_split_brain():
    f = json.loads(loct(["findings", "--summary", "--json"]).stdout)
    # `--for-ai` prints a human banner before the bundle, so slice from the
    # first brace instead of parsing the whole stream.
    raw = loct(["--for-ai"]).stdout
    a = json.loads(raw[raw.index("{") :])
    fs = f.get("health_score")
    asc = a.get("summary", {}).get("health_score") or a.get("health_score")
    # One scorer, one number. Both surfaces build their HealthMetrics through
    # `analyzer::health_inputs::structural_defects`; before that they shared
    # the formula but not the gates, and read 85 vs 72 on df35a677.
    # audit_report.md scores a strictly narrower collector and says so in the
    # report itself ("audit basis") rather than emitting a silent third number.
    assert fs == asc, f"split-brain scorer: findings={fs} vs agent.json={asc}"


def _provenance() -> str:
    """Name the binary AND whether it matches the checkout being questioned.

    Naming the path is half the guard: `~/.local/bin/loct` looks authoritative
    while being built from an unrelated commit. On 2026-08-19 that read as a
    FAIL on question 19 (85 vs 72) against a worktree that had already fixed
    the split-brain. Advisory, not fatal: measuring an installed binary is a
    legitimate release check — it just must never be silent.
    """
    try:
        version = subprocess.run(
            [LOCT, "--version"], capture_output=True, text=True, timeout=60, check=False
        ).stdout.strip()
    except OSError as exc:
        return f"binary: {LOCT} — UNUSABLE ({exc})"
    try:
        head = subprocess.run(
            ["git", "-C", str(REPO), "rev-parse", "HEAD"],
            capture_output=True,
            text=True,
            timeout=60,
            check=False,
        ).stdout.strip()
    except OSError:
        head = ""
    built = ""
    for token in version.split():
        if token.startswith("commit="):
            built = token.split("=", 1)[1]
    stale = ""
    if head and built and not (head.startswith(built) or built.startswith(head[: len(built)])):
        stale = (
            f"  <-- STALE: built from {built}, checkout is at {head[:8]}. "
            "Rebuild or set LOCT_BIN; failures below may belong to another build."
        )
    return f"binary: {LOCT}\n  {version}{stale}\n"


def main(argv: list[str]) -> int:
    if "--list" in argv:
        for q in QUESTIONS:
            print(f"[{q['id']}] {q['text']}\n    grep can't: {q['why']}")
        return 0
    print(_provenance())
    passed, failed, gaps, unpromoted = [], [], [], []
    for q in QUESTIONS:
        try:
            q["fn"]()
            if q["gap"]:
                unpromoted.append(q["id"])
                print(f"  FIXED?  {q['id']} — engine now answers this; promote gap=False")
            else:
                passed.append(q["id"])
                print(f"  PASS  {q['id']}")
        except Exception as exc:  # noqa: BLE001 — scorecard collects every failure
            if q["gap"]:
                gaps.append((q["id"], str(exc)[:200]))
                print(f"  GAP   {q['id']}: {str(exc)[:160]}")
            else:
                failed.append((q["id"], str(exc)[:200]))
                print(f"  FAIL  {q['id']}: {str(exc)[:200]}")
    hard = len([q for q in QUESTIONS if not q["gap"]])
    print(f"\ngrep-impossible: {len(passed)}/{hard} answered, {len(gaps)} known gaps")
    # Name the binary that was measured: a scorecard without provenance cannot
    # tell "this engine is green" apart from "some engine on PATH is green".
    print(f"engine under test: {LOCT}")
    if failed:
        print("failing questions are product gaps, not test noise:")
        for qid, msg in failed:
            print(f"  - {qid}: {msg}")
    if unpromoted:
        print("strict-xfail: these gaps now pass — flip gap=False to promote:")
        for qid in unpromoted:
            print(f"  - {qid}")
    return 1 if (failed or unpromoted) else 0


if __name__ == "__main__":
    raise SystemExit(main(sys.argv[1:]))
