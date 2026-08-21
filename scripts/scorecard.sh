#!/usr/bin/env bash
# scorecard.sh — rg-vs-loct correctness and latency matrix, written to scorecard.json.
# Runs from .github/workflows/ci.yml; fixture correctness is the hard gate, while
# latency, output cost, and agent lift are recorded as trend signals only.
set -euo pipefail

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
OUTPUT="$ROOT/scorecard.json"
RUNS="${SCORECARD_RUNS:-3}"

# Prints the invocation banner and the LOCT_BIN / SCORECARD_RUNS environment contract.
usage() {
  cat <<'EOF'
Usage: scripts/scorecard.sh [--output PATH] [--runs N]

Runs the rg-vs-loct scorecard matrix and writes scorecard.json.

Environment:
  LOCT_BIN         Path to a prebuilt loct binary. Defaults to target/debug/loct,
                   target/release/loct, or builds target/debug/loct.
  SCORECARD_RUNS  Warm latency samples per query when --runs is omitted.
EOF
}

while [[ $# -gt 0 ]]; do
  case "$1" in
    --output)
      OUTPUT="$2"
      shift 2
      ;;
    --runs)
      RUNS="$2"
      shift 2
      ;;
    --help|-h)
      usage
      exit 0
      ;;
    *)
      echo "unknown argument: $1" >&2
      usage >&2
      exit 2
      ;;
  esac
done

if ! command -v rg >/dev/null 2>&1; then
  echo "error: ripgrep (rg) is required for the scorecard baseline" >&2
  exit 2
fi

if ! command -v python3 >/dev/null 2>&1; then
  echo "error: python3 is required to aggregate scorecard JSON" >&2
  exit 2
fi

python3 - "$ROOT" "$OUTPUT" "$RUNS" <<'PY'
import json
import os
import statistics
import subprocess
import sys
import tempfile
import time
from datetime import datetime, timezone
from pathlib import Path

root = Path(sys.argv[1]).resolve()
output_path = Path(sys.argv[2]).resolve()
runs = int(sys.argv[3])
if runs < 1:
    raise SystemExit("--runs must be >= 1")

fixture_root = root / "loctree-rs/tests/fixtures/scorecard_rg_parity"


def resolve_loct():
    env_bin = os.environ.get("LOCT_BIN")
    if env_bin:
        return [env_bin]
    for candidate in [root / "target/debug/loct", root / "target/release/loct"]:
        if candidate.exists() and os.access(candidate, os.X_OK):
            return [str(candidate)]
    subprocess.run(
        ["cargo", "build", "-p", "loctree", "--bin", "loct"],
        cwd=root,
        check=True,
    )
    return [str(root / "target/debug/loct")]


loct_bin = resolve_loct()
cache_dir = tempfile.mkdtemp(prefix="loct-scorecard-")
base_env = os.environ.copy()
base_env["LOCT_CACHE_DIR"] = cache_dir


def run_command(args, cwd):
    started = time.perf_counter_ns()
    proc = subprocess.run(
        args,
        cwd=cwd,
        env=base_env,
        text=True,
        stdout=subprocess.PIPE,
        stderr=subprocess.PIPE,
    )
    elapsed_ms = (time.perf_counter_ns() - started) / 1_000_000
    return proc, elapsed_ms


def checked_json(args, cwd):
    proc, elapsed = run_command(args, cwd)
    if proc.returncode != 0:
        raise RuntimeError(
            f"command failed ({proc.returncode}): {' '.join(args)}\n{proc.stderr}"
        )
    return json.loads(proc.stdout), proc.stdout, elapsed


def rg_args(probe):
    args = ["rg", "-o", "--hidden", "--glob", "!.git/**"]
    if probe["kind"] == "regex":
        return args + ["-e", probe["query"], "."]
    return args + ["--fixed-strings", probe["rg_query"], "."]


def loct_args(probe, count_only=False):
    kind = probe["kind"]
    if kind == "literal":
        args = ["find", "--literal", probe["query"]]
        if count_only:
            args.extend(["--group-by-file", "--count-only"])
        return loct_bin + args + ["--json"]
    if kind == "regex":
        return loct_bin + ["find", "--regex", probe["query"], "--json"]
    if kind == "where-symbol":
        return loct_bin + ["find", probe["query"], "--where-symbol", "--json"]
    if kind == "who-imports":
        return loct_bin + ["query", "who-imports", probe["target"], "--json"]
    raise ValueError(f"unknown probe kind {kind}")


def parse_rg_counts(stdout):
    counts = {}
    for line in stdout.splitlines():
        if ":" not in line:
            continue
        path = line.split(":", 1)[0]
        path = path[2:] if path.startswith("./") else path
        counts[path] = counts.get(path, 0) + 1
    return counts


def run_rg_counts(probe, cwd):
    args = rg_args(probe)
    proc, elapsed = run_command(args, cwd)
    if proc.returncode not in (0, 1):
        raise RuntimeError(f"rg failed ({proc.returncode}): {' '.join(args)}\n{proc.stderr}")
    return parse_rg_counts(proc.stdout), proc.stdout, elapsed


def by_file_counts(value):
    return {entry["file"]: int(entry["count"]) for entry in value or []}


def occurrence_counts(value):
    counts = {}
    for occurrence in value or []:
        file = occurrence["file"]
        counts[file] = counts.get(file, 0) + 1
    return counts


def result_counts(value):
    counts = {}
    for result in value or []:
        file = result["file"]
        counts[file] = counts.get(file, 0) + 1
    return counts


def file_presence(counts):
    return {file: 1 for file in counts}


def loct_counts(probe, data):
    kind = probe["kind"]
    if kind == "literal":
        literal = data["literal_matches"]
        if "by_file" in literal:
            return by_file_counts(literal["by_file"])
        return occurrence_counts(literal.get("occurrences", []))
    if kind == "regex":
        return occurrence_counts(data["regex_matches"].get("occurrences", []))
    if kind in ("where-symbol", "who-imports"):
        return result_counts(data.get("results", []))
    return {}


def correctness(probe, rg_counts_map, loct_counts_map):
    basis = probe["basis"]
    if basis == "loct_nonempty":
        total = sum(loct_counts_map.values())
        return {
            "basis": basis,
            "status": "pass" if total > 0 else "fail",
            "missing": [],
            "loct_total": total,
        }
    rg_for_compare = file_presence(rg_counts_map) if basis == "file_superset" else rg_counts_map
    missing = []
    for file, rg_count in rg_for_compare.items():
        loct_count = loct_counts_map.get(file, 0)
        if loct_count < rg_count:
            missing.append({"file": file, "rg": rg_count, "loct": loct_count})
    return {
        "basis": basis,
        "status": "pass" if not missing and bool(rg_for_compare) else "fail",
        "missing": missing,
        "rg_files": len(rg_for_compare),
        "loct_files": len(loct_counts_map),
    }


def median_latency(args, cwd):
    # Warm once, then sample. The warm output is deliberately ignored.
    run_command(args, cwd)
    samples = []
    stdout = ""
    for _ in range(runs):
        proc, elapsed = run_command(args, cwd)
        if proc.returncode not in (0, 1):
            raise RuntimeError(f"latency command failed: {' '.join(args)}\n{proc.stderr}")
        samples.append(elapsed)
        stdout = proc.stdout
    return statistics.median(samples), stdout


def lift(probe, data):
    kind = probe["kind"]
    if kind == "literal":
        literal = data.get("literal_matches", {})
        checks = {
            "role_summary": bool(literal.get("role_summary")),
            "scope_classifications": bool(literal.get("scope_classifications")),
            "file_context": bool(literal.get("file_context")),
        }
        checks["status"] = "pass" if all(checks.values()) else "fail"
        return checks
    if kind == "regex":
        regex = data.get("regex_matches", {})
        checks = {
            "scope_classifications": bool(regex.get("scope_classifications")),
            "suggested_next": bool(regex.get("suggested_next")),
        }
        checks["status"] = "pass" if any(checks.values()) else "warn"
        return checks
    checks = {"results": bool(data.get("results"))}
    checks["status"] = "pass" if checks["results"] else "fail"
    return checks


probes = [
    {
        "suite": "fixture",
        "class": "identifier",
        "kind": "literal",
        "query": "scorecard_worker_token",
        "rg_query": "scorecard_worker_token",
        "basis": "count_superset",
        "cwd": fixture_root,
    },
    {
        "suite": "fixture",
        "class": "prose",
        "kind": "literal",
        "query": "scorecard prose phrase literal parity stays honest",
        "rg_query": "scorecard prose phrase literal parity stays honest",
        "basis": "count_superset",
        "cwd": fixture_root,
    },
    {
        "suite": "fixture",
        "class": "regex",
        "kind": "regex",
        "query": "Scorecard[A-Za-z]+",
        "rg_query": "Scorecard[A-Za-z]+",
        "basis": "count_superset",
        "cwd": fixture_root,
    },
    {
        "suite": "fixture",
        "class": "symbol-definition",
        "kind": "where-symbol",
        "query": "ScorecardWorker",
        "rg_query": "ScorecardWorker",
        "basis": "file_superset",
        "cwd": fixture_root,
    },
    {
        "suite": "fixture",
        "class": "who-imports",
        "kind": "who-imports",
        "query": "src/alpha.rs",
        "target": "src/alpha.rs",
        "rg_query": "crate::alpha",
        "basis": "file_superset",
        "cwd": fixture_root,
    },
    {
        "suite": "loctree-suite",
        "class": "identifier",
        "kind": "literal",
        "query": "OccurrenceKind",
        "rg_query": "OccurrenceKind",
        "basis": "count_superset",
        "cwd": root,
    },
    {
        "suite": "loctree-suite",
        "class": "prose",
        "kind": "literal",
        "query": "literal mode must be self-describing",
        "rg_query": "literal mode must be self-describing",
        "basis": "count_superset",
        "cwd": root,
    },
    {
        "suite": "loctree-suite",
        "class": "regex",
        "kind": "regex",
        "query": "Occurrence[A-Za-z]+",
        "rg_query": "Occurrence[A-Za-z]+",
        "basis": "count_superset",
        "cwd": root,
    },
    {
        "suite": "loctree-suite",
        "class": "symbol-definition",
        "kind": "where-symbol",
        "query": "OccurrenceKind",
        "rg_query": "OccurrenceKind",
        "basis": "loct_nonempty",
        "cwd": root,
    },
    {
        "suite": "loctree-suite",
        "class": "who-imports",
        "kind": "who-imports",
        "query": "loctree-rs/src/analyzer/occurrences.rs",
        "target": "loctree-rs/src/analyzer/occurrences.rs",
        "rg_query": "crate::analyzer::occurrences",
        "basis": "loct_nonempty",
        "cwd": root,
    },
]

rows = []
for probe in probes:
    cwd = probe["cwd"]
    rg_count_map, rg_stdout, _ = run_rg_counts(probe, cwd)
    loct_count_data, loct_count_stdout, _ = checked_json(loct_args(probe, count_only=True), cwd)
    loct_full_data, loct_full_stdout, _ = checked_json(loct_args(probe, count_only=False), cwd)
    loct_count_map = loct_counts(probe, loct_count_data)

    rg_median, rg_last_stdout = median_latency(rg_args(probe), cwd)
    loct_median, loct_last_stdout = median_latency(loct_args(probe, count_only=False), cwd)
    ratio = loct_median / rg_median if rg_median > 0 else None

    row = {
        "suite": probe["suite"],
        "class": probe["class"],
        "kind": probe["kind"],
        "query": probe["query"],
        "loct_command": loct_args(probe, count_only=False),
        "rg_command": rg_args(probe),
        "correctness": correctness(probe, rg_count_map, loct_count_map),
        "latency_ms": {
            "runs": runs,
            "rg_median": round(rg_median, 3),
            "loct_median": round(loct_median, 3),
            "loct_vs_rg_ratio": round(ratio, 2) if ratio is not None else None,
            "status": "warn" if ratio is not None and ratio > 5 else "ok",
        },
        "output_cost": {
            "rg_bytes": len(rg_last_stdout.encode("utf-8")),
            "rg_chars": len(rg_last_stdout),
            "loct_bytes": len(loct_last_stdout.encode("utf-8")),
            "loct_chars": len(loct_last_stdout),
        },
        "lift": lift(probe, loct_full_data),
    }
    rows.append(row)

fixture_failures = [
    row for row in rows if row["suite"] == "fixture" and row["correctness"]["status"] != "pass"
]
warnings = [
    row for row in rows if row["latency_ms"]["status"] == "warn" or row["lift"]["status"] == "warn"
]

report = {
    "schema_version": "scorecard.rg_vs_loct.v1",
    "generated_at": datetime.now(timezone.utc).isoformat(),
    "repo": str(root),
    "fixture": str(fixture_root),
    "loct_bin": loct_bin,
    "rg_version": subprocess.run(
        ["rg", "--version"], text=True, stdout=subprocess.PIPE, check=True
    ).stdout.splitlines()[0],
    "runs": runs,
    "matrix": rows,
    "summary": {
        "fixture_correctness": "pass" if not fixture_failures else "fail",
        "fixture_failures": fixture_failures,
        "warnings": len(warnings),
        "hard_gate": "fixture correctness only; latency/output/lift are reported as trend signals",
    },
}

output_path.parent.mkdir(parents=True, exist_ok=True)
output_path.write_text(json.dumps(report, indent=2) + "\n", encoding="utf-8")

print(f"scorecard: wrote {output_path}")
print(f"scorecard: fixture correctness {report['summary']['fixture_correctness']}")
for row in rows:
    corr = row["correctness"]["status"]
    lat = row["latency_ms"]
    lift_status = row["lift"]["status"]
    print(
        f"- {row['suite']} {row['class']}: correctness={corr} "
        f"loct_median={lat['loct_median']}ms rg_median={lat['rg_median']}ms "
        f"ratio={lat['loct_vs_rg_ratio']} lift={lift_status}"
    )

if fixture_failures:
    raise SystemExit(1)
PY
