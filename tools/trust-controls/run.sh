#!/usr/bin/env bash
# The disabled warning treats Markdown backticks as shell interpolation.
# shellcheck disable=SC2016
set -euo pipefail

ROOT=$(git -C "$(dirname "${BASH_SOURCE[0]}")" rev-parse --show-toplevel)
BIN_DIR=${LOCT_TRUST_BIN_DIR:-"$ROOT/target/release"}
RUN_ID=$(date -u +%Y%m%dT%H%M%SZ)-$$
EVIDENCE_DIR=${LOCT_TRUST_EVIDENCE_DIR:-"$ROOT/target/trust-controls/$RUN_ID"}
WORK_DIR=$(mktemp -d "${TMPDIR:-/tmp}/loctree-trust-controls.XXXXXX")
PASS_COUNT=0

cleanup() {
  case "$WORK_DIR" in
    "${TMPDIR:-/tmp}"/loctree-trust-controls.*) rm -rf -- "$WORK_DIR" ;;
    *) printf 'refusing to clean unexpected work directory: %s\n' "$WORK_DIR" >&2 ;;
  esac
}
trap cleanup EXIT

mkdir -p "$EVIDENCE_DIR"

pass() {
  PASS_COUNT=$((PASS_COUNT + 1))
  printf '[x] %s\n' "$1"
}

fail() {
  printf '[FAIL] %s\n' "$1" >&2
  printf 'evidence: %s\n' "$EVIDENCE_DIR" >&2
  exit 1
}

require() {
  command -v "$1" >/dev/null 2>&1 || fail "required command is unavailable: $1"
}

field() {
  local marker=$1 key=$2
  awk -v key="$key" '{ for (i = 1; i <= NF; i++) if ($i ~ ("^" key "=")) { sub("^" key "=", "", $i); print $i; exit } }' <<<"$marker"
}

init_fixture() {
  local fixture=$1
  git -C "$fixture" init -q
  git -C "$fixture" add .
  git -C "$fixture" -c user.name=trust-control -c user.email=trust-control@example.invalid commit -qm baseline
}

run_loct() {
  local fixture=$1 cache=$2
  shift 2
  (
    cd "$fixture"
    LOCT_CACHE_DIR="$cache" LOCT_NO_GITIGNORE=1 "$LOCT" "$@"
  )
}

run_suite() {
  (
    cd "$ROOT"
    LOCT_CACHE_DIR="$WORK_DIR/cache-suite" LOCT_NO_GITIGNORE=1 "$LOCT" "$@"
  )
}

require git
require jq
require awk

LOCT="$BIN_DIR/loct"
LOCTREE="$BIN_DIR/loctree"
MCP="$BIN_DIR/loctree-mcp"
for binary in "$LOCT" "$LOCTREE" "$MCP"; do
  [[ -x "$binary" ]] || fail "missing executable $binary; build the release triad first"
done

LOCT_MARKER=$($LOCT --version)
LOCTREE_MARKER=$($LOCTREE --version)
MCP_MARKER=$($MCP --version)
printf '%s\n%s\n%s\n' "$LOCT_MARKER" "$LOCTREE_MARKER" "$MCP_MARKER" \
  | tee "$EVIDENCE_DIR/binary-markers.txt"

EXPECTED_BUNDLE=$(field "$LOCT_MARKER" bundle_id)
EXPECTED_COMMIT=$(field "$LOCT_MARKER" commit)
[[ -n "$EXPECTED_BUNDLE" && -n "$EXPECTED_COMMIT" ]] || fail 'loct marker lacks bundle identity'
for marker in "$LOCT_MARKER" "$LOCTREE_MARKER" "$MCP_MARKER"; do
  [[ $(field "$marker" schema) == loctree.bundle.v1 ]] || fail "unexpected bundle schema: $marker"
  [[ $(field "$marker" bundle_id) == "$EXPECTED_BUNDLE" ]] || fail "bundle_id split: $marker"
  [[ $(field "$marker" commit) == "$EXPECTED_COMMIT" ]] || fail "commit split: $marker"
done
pass 'LCT-L03 release CLI/MCP triad shares loctree.bundle.v1 identity'

run_suite find --literal 'context atlas' --json > "$EVIDENCE_DIR/literal-context-atlas.json"
jq -e '
  .literal_matches.total > 0 and
  .literal_matches.universe.scan_complete == true and
  .literal_matches.scope.files_scanned == .literal_matches.scope.files_in_universe and
  .literal_matches.universe.scanned_files == .literal_matches.universe.indexed_files and
  (.literal_matches.scope_classifications | map(.scope_classification) | index("docs") != null)
' "$EVIDENCE_DIR/literal-context-atlas.json" >/dev/null \
  || fail 'literal phrase lacks complete, classified universe accounting'

run_suite find --literal 'cytoscape' --json > "$EVIDENCE_DIR/literal-asset-token.json"
jq -e '.literal_matches.total > 0 and .literal_matches.universe.scan_complete == true' \
  "$EVIDENCE_DIR/literal-asset-token.json" >/dev/null \
  || fail 'asset token is absent or coverage is incomplete'

run_suite find --discover result --limit 2 --json > "$EVIDENCE_DIR/discover-limit.json"
jq -e '
  .limit == 2 and .page.limit == 2 and .page.returned <= 2 and
  .page.semantics == "global" and .page.total == .total and .has_more == true
' "$EVIDENCE_DIR/discover-limit.json" >/dev/null \
  || fail 'discover did not enforce the global result limit'
pass 'LCT-A04/A06/A07 literal trust exposes its universe and discover is globally bounded'

PY_FIXTURE="$WORK_DIR/python"
cp -R "$ROOT/loctree-rs/tests/fixtures/python_project" "$PY_FIXTURE"
init_fixture "$PY_FIXTURE"
run_loct "$PY_FIXTURE" "$WORK_DIR/cache-python" scan --quiet > "$EVIDENCE_DIR/python-scan.txt"
run_loct "$PY_FIXTURE" "$WORK_DIR/cache-python" body normalize --file app/service.py --json \
  > "$EVIDENCE_DIR/body-qualified.json"
jq -e '
  (.bodies | length) == 1 and .bodies[0].file == "app/service.py" and
  .bodies[0].truncated == false and .bodies[0].extent == "indent" and
  (.bodies[0].source | contains("return value.strip().lower()"))
' "$EVIDENCE_DIR/body-qualified.json" >/dev/null \
  || fail 'qualified Python body is clipped, ambiguous, or mislabeled'
pass 'LCT-B03 qualified Python body is complete and reports honest extent metadata'

run_suite impact loctree-mcp/src/main.rs > "$EVIDENCE_DIR/impact-binary-entrypoint.txt"
if grep -Fq 'Safe to remove' "$EVIDENCE_DIR/impact-binary-entrypoint.txt"; then
  fail 'impact emitted destructive safe-to-remove advice for a binary entrypoint'
fi
grep -Fq 'coverage incomplete; cannot assess removal' "$EVIDENCE_DIR/impact-binary-entrypoint.txt" \
  || fail 'impact did not fail closed when the indexed graph had zero consumers'
pass 'LCT-C01 zero-consumer impact fails closed instead of advising deletion'

RUNTIME_FIXTURE="$WORK_DIR/runtime"
cp -R "$ROOT/loctree-rs/tests/fixtures/runtime_inventory" "$RUNTIME_FIXTURE"
init_fixture "$RUNTIME_FIXTURE"
RUNTIME_CACHE="$WORK_DIR/cache-runtime"
run_loct "$RUNTIME_FIXTURE" "$RUNTIME_CACHE" --fresh context --no-aicx --json \
  > "$EVIDENCE_DIR/context-initial.json" 2> "$EVIDENCE_DIR/context-initial.stderr"
printf '\n// trust-control HEAD move\n' >> "$RUNTIME_FIXTURE/src/main.rs"
git -C "$RUNTIME_FIXTURE" add src/main.rs
git -C "$RUNTIME_FIXTURE" -c user.name=trust-control -c user.email=trust-control@example.invalid commit -qm head-move
LIVE_HEAD=$(git -C "$RUNTIME_FIXTURE" rev-parse HEAD)

run_loct "$RUNTIME_FIXTURE" "$RUNTIME_CACHE" --no-scan context --no-aicx --json \
  > "$EVIDENCE_DIR/context-stale.json" 2> "$EVIDENCE_DIR/context-stale.stderr"
jq -e --arg head "$LIVE_HEAD" '
  .receipt.snapshot_commit as $snap |
  .receipt.authority == "stale" and .receipt.head_full == $head and
  ($head | startswith($snap) | not) and
  (.receipt.diagnostics | any(contains("does not match live HEAD")))
' "$EVIDENCE_DIR/context-stale.json" >/dev/null \
  || fail 'stale snapshot was not labeled against live HEAD'

run_loct "$RUNTIME_FIXTURE" "$RUNTIME_CACHE" --fresh context --no-aicx --json \
  > "$EVIDENCE_DIR/context-fresh.json" 2> "$EVIDENCE_DIR/context-fresh.stderr"
jq -e --arg head "$LIVE_HEAD" --arg binary "$EXPECTED_BUNDLE" '
  .receipt.snapshot_commit as $snap |
  .receipt.authority == "fresh" and .receipt.head_full == $head and
  ($head | startswith($snap)) and .receipt.binary_id == $binary and
  ([.runtime.env_contracts[].name] | index("FIXTURE_CACHE_ROOT") != null) and
  ([.runtime.env_contracts[].name] | index("FIXTURE_RUNTIME_ROOT") != null) and
  ([.runtime.framework_hints[].kind] | index("swiftpm_executable_target") != null) and
  ([.runtime.framework_hints[].kind] | index("cargo_default_run") != null)
' "$EVIDENCE_DIR/context-fresh.json" >/dev/null \
  || fail 'fresh context receipt/runtime inventory does not match live fixture truth'
pass 'LCT-D02 stale authority is explicit and --fresh receipt/runtime inventory matches live HEAD'

run_loct "$RUNTIME_FIXTURE" "$RUNTIME_CACHE" doctor --cache --scope --json \
  > "$EVIDENCE_DIR/doctor-cache.json"
jq -e '
  .schema_version == "1.2" and .enumeration.scope == "project-local" and
  .enumeration.complete == true
' "$EVIDENCE_DIR/doctor-cache.json" >/dev/null \
  || fail 'doctor cache enumeration escaped project-local scope'
pass 'LCT-E01 doctor cache enumeration is complete and project-local'

DEAD_FIXTURE="$WORK_DIR/dead"
cp -R "$ROOT/loctree-rs/tests/fixtures/dead_truth_framework" "$DEAD_FIXTURE"
init_fixture "$DEAD_FIXTURE"
run_loct "$DEAD_FIXTURE" "$WORK_DIR/cache-dead" scan --quiet > "$EVIDENCE_DIR/dead-scan.txt"
run_loct "$DEAD_FIXTURE" "$WORK_DIR/cache-dead" dead --all --json > "$EVIDENCE_DIR/dead-swift.json"
jq -e '
  ([.[] | select(.symbol == "FixtureApp" and .confidence == "high")] | length == 0) and
  ([.[] | select(.symbol == "DormantFeature")] | length > 0)
' "$EVIDENCE_DIR/dead-swift.json" >/dev/null \
  || fail 'Swift @main entrypoint was high-confidence dead or negative fixture disappeared'
pass 'LCT-G01 Swift @main is protected while a genuinely dormant symbol remains detectable'

{
  printf '# Loctree trust-control result\n\n'
  printf -- '- status: pass\n'
  printf -- '- controls: %s\n' "$PASS_COUNT"
  printf -- '- bundle_id: `%s`\n' "$EXPECTED_BUNDLE"
  printf -- '- commit: `%s`\n' "$EXPECTED_COMMIT"
  printf -- '- evidence: `%s`\n' "$EVIDENCE_DIR"
} > "$EVIDENCE_DIR/SUMMARY.md"

printf '\nPASS: %s controls\n' "$PASS_COUNT"
printf 'evidence: %s\n' "$EVIDENCE_DIR"
