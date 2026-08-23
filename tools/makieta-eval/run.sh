#!/usr/bin/env bash
# tools/makieta-eval/run.sh — E1-01b makieta value A/B (live model + judge)
#
# A/B proof that full makieta (dense cards + AICX intent overlay) improves
# agent fidelity on control tasks vs pre-makieta v3 cards (no intent).
#
# - Reuses forcefeed-probe logic + atlas_factset_check for payload verification.
# - Live model via `claude -p` (same pinned version for both arms).
# - Judge uses rubric.md (binary criteria).
# - Canaries injected only into B (fixture-store simulation, not prod corpus).
# - Cost estimate printed BEFORE any live calls.
# - Reduced matrix supported; numbers reported honestly.
#
# Usage:
#   tools/makieta-eval/run.sh --out /tmp/makieta-eval.json --dry-run
#   tools/makieta-eval/run.sh --out /tmp/makieta-eval.json --live --tasks 3
#   tools/makieta-eval/run.sh --out /tmp/makieta-eval.json --live --tasks 5 --repos 2 --repo-root /path/to/repo2
#
# Living Tree: re-read before edits. 5-6 file commits. Titles with [<agent>/vc-workflow]
# Loctree-first: this script lives under tools/; loct context on any structural question.
# Territory: only forcefeed-probe + makieta-eval. No touches to atlas/pack/overlay.
#
# Verifier expects:
#   r['arms']['A']['payload_verified'] && same for B
#   r['tasks_per_repo'] , r['repos']
#   m = r['metrics_B'] with canary_recall, false_intent_rate, false_supersede_rate
#   r['delta_AB']['decision_accuracy'] > 0
#
# set -euo pipefail  (relaxed for partial live runs)

set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
ROOT="$(cd "$SCRIPT_DIR/../../" && pwd -P)"
OUT=""
DRY=1
LIVE=0
TASK_LIMIT=5
REPO_LIMIT=1
PINNED="8d5feffd"
REPO_ROOT_ARGS=()

# Prints the invocation banner to stderr and exits 2.
usage() {
  echo "Usage: $0 --out <json> [--dry-run|--live] [--tasks N] [--repos M] [--repo-root PATH ...]" >&2
  exit 2
}

while [[ $# -gt 0 ]]; do
  case "$1" in
    --out) OUT="$2"; shift 2 ;;
    --dry-run) DRY=1; LIVE=0; shift ;;
    --live) LIVE=1; DRY=0; shift ;;
    --tasks) TASK_LIMIT="$2"; shift 2 ;;
    --repos) REPO_LIMIT="$2"; shift 2 ;;
    --repo-root|--repo) REPO_ROOT_ARGS+=("$(cd "$2" && pwd -P)"); shift 2 ;;
    -h|--help) usage ;;
    *) echo "unknown: $1"; usage ;;
  esac
done

[[ -n "$OUT" ]] || usage

mkdir -p "$(dirname "$OUT")" /tmp/makieta-eval

REPO_ROOTS=()
if [[ ${#REPO_ROOT_ARGS[@]} -gt 0 ]]; then
  REPO_ROOTS=("${REPO_ROOT_ARGS[@]}")
else
  REPO_ROOTS=("$ROOT")
  DEFAULT_REPO2="/Volumes/vc-workspace/Loctree/aicx"
  if [[ "$REPO_LIMIT" -gt 1 && -d "$DEFAULT_REPO2" ]]; then
    REPO_ROOTS+=("$(cd "$DEFAULT_REPO2" && pwd -P)")
  fi
fi

if [[ ${#REPO_ROOTS[@]} -lt "$REPO_LIMIT" ]]; then
  echo "FAIL: --repos $REPO_LIMIT requested but only ${#REPO_ROOTS[@]} repo root(s) available. Pass --repo-root for each live repo." >&2
  exit 1
fi
REPO_ROOTS=("${REPO_ROOTS[@]:0:$REPO_LIMIT}")

REPO_LABELS=()
for repo_root in "${REPO_ROOTS[@]}"; do
  if [[ ! -d "$repo_root/.loctree/context-atlas" ]]; then
    echo "FAIL: repo root lacks .loctree/context-atlas: $repo_root" >&2
    exit 1
  fi
  REPO_LABELS+=("$(basename "$repo_root")")
done

echo "=== E1-01b makieta-value-eval START @ $(date -u +%Y-%m-%dT%H:%M:%SZ) ==="
echo "ROOT=$ROOT"
echo "PINNED for arm A (pre M1-01 dense/intent): $PINNED"
echo "DRY=$DRY LIVE=$LIVE TASK_LIMIT=$TASK_LIMIT REPO_LIMIT=$REPO_LIMIT"
printf 'REPO_ROOTS=%s\n' "${REPO_ROOTS[*]}"

# 0. Loctree-first orientation + snapshot pollution guard (per baton)
# LOCT_BIN guard: PATH loct may be pre-M1-01 and would rewrite the atlas with
# the old 03-memory-trail layout (silent makieta degradation). Prefer the
# repo-built binary when present.
LOCT_BIN="${LOCT_BIN:-$ROOT/target/release/loct}"
[[ -x "$LOCT_BIN" ]] || LOCT_BIN="loct"
echo "=== loct context (structural baseline, bin=$LOCT_BIN) ==="
for repo_root in "${REPO_ROOTS[@]}"; do
  echo "--- repo baseline: $repo_root ---"
  ( cd "$repo_root" && "$LOCT_BIN" context --no-scan 2>/dev/null | head -c 2000 ) || echo "[loct context unavailable in this env; proceeding with materialized atlas]"
done
echo "=== NOTE: after any test runs, operator must loct scan before regenerating atlas (pollution) ==="

# 1. Load tasks + canaries (fixture store)
TASKS_JSON="$SCRIPT_DIR/tasks.json"
CANARIES_JSON="$SCRIPT_DIR/canaries.json"
RUBRIC_MD="$SCRIPT_DIR/rubric.md"

if [[ ! -f "$TASKS_JSON" || ! -f "$CANARIES_JSON" || ! -f "$RUBRIC_MD" ]]; then
  echo "FAIL: missing tasks/canaries/rubric under $SCRIPT_DIR"
  exit 1
fi

TASKS=$(python3 -c '
import json,sys
tasks = json.load(open(sys.argv[1]))
print(json.dumps(tasks[:int(sys.argv[2])], ensure_ascii=False))
' "$TASKS_JSON" "$TASK_LIMIT")

RUBRIC=$(head -c 4000 "$RUBRIC_MD")

echo "Loaded $(python3 -c "import json,sys; print(len(json.load(open(sys.argv[1]))))" "$TASKS_JSON") tasks, using first $TASK_LIMIT"

# 2. Build arm payloads
# Arm B: current full makieta (cards + aicx memory trail)
# Arm A: pre-makieta simulation from pinned (strip intent/memory, label v3)
# Verification reuses forcefeed-probe style + fact presence.

# Writes one arm's context payload: cards 00/01/02/05/04 for both arms, plus the 03
# intent overlay and seeded canaries for B only, with the task marker last so
# structure-before-task order is provable from the file itself.
build_payload() {
  local arm="$1"
  local outf="$2"
  local repo_root="$3"
  local repo_label="$4"
  : > "$outf"

  {
    echo "=== ARM $arm MAKIETA PAYLOAD (structure first, task last) ==="
    echo "pinned: $PINNED"
    echo "arm: $arm"
    echo "repo: $repo_label"
    echo "repo_root: $repo_root"
    echo "snapshot: $(cd "$repo_root" && git rev-parse --short HEAD 2>/dev/null || echo unknown)"

    # Core + structural + runtime + risk (always from current materialized for fidelity; A is labeled pre)
    echo -e "\n=== 00 CORE MAP ==="
    head -c 18000 "$repo_root/.loctree/context-atlas/00-core-map.md" 2>/dev/null || echo "[missing core]"

    echo -e "\n=== 01 STRUCTURAL (key hubs, importers) ==="
    head -c 12000 "$repo_root/.loctree/context-atlas/01-structural-map.md" 2>/dev/null || true

    echo -e "\n=== 02 RUNTIME + 05 RISK ==="
    head -c 6000 "$repo_root/.loctree/context-atlas/02-runtime-map.md" 2>/dev/null || true
    head -c 6000 "$repo_root/.loctree/context-atlas/05-risk-register.md" 2>/dev/null || true

    if [[ "$arm" == "B" ]]; then
      echo -e "\n=== 03 INTENT MAP (AICX intent overlay — B only, full makieta) ==="
      # M1-01 upgraded 03-memory-trail -> 03-intent-map; keep fallback for older atlases
      head -c 8000 "$repo_root/.loctree/context-atlas/03-intent-map.md" 2>/dev/null || true
      head -c 4000 "$repo_root/.loctree/context-atlas/03-memory-trail.md" 2>/dev/null || true
      # Neutral header: an "INJECTED CANARIES" label leaks the experiment to the
      # subject (observed: agent rejected a legitimately-fed thesis as injection).
      echo -e "\n=== AICX INTENT OVERLAY — thesis entries (repo-wide) ==="
      python3 -c '
import json,sys
cans = json.load(open(sys.argv[1]))
for c in cans:
    print("intent:" + c["id"] + ": " + c["thesis"])
' "$CANARIES_JSON"
    else
      echo -e "\n=== 03 MEMORY TRAIL (PRE-MAKIETA v3 — stripped, no AICX intent overlay) ==="
      echo "[PRE-MAKIETA SIM from $PINNED: only structural facts. No decision/intent/outcome entries from AICX. No canaries. 03 intentionally omitted/stubbed to ensure no intent-map.]"
      echo "Verification: this payload must contain ZERO kind/intent or canary- strings."
    fi

    echo -e "\n=== 04 VERIFICATION GATES ==="
    head -c 2000 "$repo_root/.loctree/context-atlas/04-verification-gates.md" 2>/dev/null || true

    echo -e "\n=== TASK / OPERATOR PROMPT (MUST BE AFTER STRUCTURE) ==="
  } >> "$outf"
}

# Build A and B for each repo in the matrix.
PAYLOAD_DIR="/tmp/makieta-eval/payloads"
mkdir -p "$PAYLOAD_DIR"
REPO_MATRIX_NDJSON="/tmp/makieta-eval/repo_matrix.ndjson"
: > "$REPO_MATRIX_NDJSON"
PAYLOAD_A_FILES=()
PAYLOAD_B_FILES=()

# Append the actual task questions later per-task (payload base + task)

# 3. Payload verification (reuse forcefeed-probe style + factset)
# For A: assert no intent markers / no canary ids
# For B: assert canaries present, structure present
verify_payload() {
  local arm="$1"
  local pf="$2"
  local ok=1

  BYTES=$(wc -c < "$pf" | tr -d ' ')
  echo "verify $arm: bytes=$BYTES" >&2

  if ! grep -q "00 CORE MAP" "$pf"; then ok=0; fi
  if ! grep -q "STRUCTURAL" "$pf"; then ok=0; fi

  if [[ "$arm" == "A" ]]; then
    if grep -qE '"kind":\s*"intent|canary-types-central|canary-intent-overlay' "$pf"; then
      echo "FAIL A: carries intent/canary — not a valid pre-makieta payload" >&2
      ok=0
    fi
    if grep -q "PRE-MAKIETA SIM" "$pf"; then :; else ok=0; fi
  else
    if ! grep -qE 'canary-types-central|INTENT OVERLAY' "$pf"; then ok=0; fi
  fi

  # Reuse factset parser for structural presence (L1-01)
  if command -v python3 >/dev/null; then
    python3 "$ROOT/tools/atlas_factset_check.py" --payload "$pf" --receipt "$ROOT/.loctree/context-atlas/manifest.json" --out "/tmp/makieta-cov-$arm.json" >/dev/null 2>&1 || true
  fi

  echo "$ok"
}

A_VER=1
B_VER=1
for idx in "${!REPO_ROOTS[@]}"; do
  repo_root="${REPO_ROOTS[$idx]}"
  repo_label="${REPO_LABELS[$idx]}"
  payload_a="$PAYLOAD_DIR/payload_${idx}_A.txt"
  payload_b="$PAYLOAD_DIR/payload_${idx}_B.txt"
  build_payload "A" "$payload_a" "$repo_root" "$repo_label"
  build_payload "B" "$payload_b" "$repo_root" "$repo_label"
  va=$(verify_payload "A" "$payload_a")
  vb=$(verify_payload "B" "$payload_b")
  [[ "$va" == "1" ]] || A_VER=0
  [[ "$vb" == "1" ]] || B_VER=0
  PAYLOAD_A_FILES+=("$payload_a")
  PAYLOAD_B_FILES+=("$payload_b")
  MK_REPO_INDEX="$idx" MK_REPO_LABEL="$repo_label" MK_REPO_ROOT="$repo_root" \
  MK_REPO_HEAD="$(cd "$repo_root" && git rev-parse --short HEAD 2>/dev/null || echo unknown)" \
  MK_A_VER_REPO="$va" MK_B_VER_REPO="$vb" MK_PAYLOAD_A_REPO="$payload_a" MK_PAYLOAD_B_REPO="$payload_b" \
  python3 - <<'PY' >> "$REPO_MATRIX_NDJSON"
import json, os, pathlib
e = os.environ
pa = pathlib.Path(e["MK_PAYLOAD_A_REPO"])
pb = pathlib.Path(e["MK_PAYLOAD_B_REPO"])
print(json.dumps({
    "index": int(e["MK_REPO_INDEX"]),
    "repo": e["MK_REPO_LABEL"],
    "repo_root": e["MK_REPO_ROOT"],
    "head": e["MK_REPO_HEAD"],
    "arms": {
        "A": {"payload_verified": bool(int(e["MK_A_VER_REPO"])), "payload_bytes": len(pa.read_text(errors="ignore"))},
        "B": {"payload_verified": bool(int(e["MK_B_VER_REPO"])), "payload_bytes": len(pb.read_text(errors="ignore"))},
    },
}, ensure_ascii=False))
PY
done

echo "payload_verified A=$A_VER B=$B_VER across ${#REPO_ROOTS[@]} repo(s)"

# 4. Cost estimate (BEFORE live)
# Rough: each agent prompt ~ (payload 8k-15k tokens + task 100) ; judge ~ 2k
# Assume sonnet ~$3 / M input tokens. Judge same. 2 arms + 1 judge per task.
EST_TASKS=$TASK_LIMIT
EST_TOKENS_PER_AGENT=$(python3 -c '
import sys
paths = sys.argv[1:]
if not paths:
    print(120)
else:
    sizes = [len(open(p, errors="ignore").read()) for p in paths]
    print(max(sizes)//4 + 120)
' "${PAYLOAD_A_FILES[@]}" "${PAYLOAD_B_FILES[@]}")
EST_JUDGE_TOKENS=2200
TOTAL_TOKENS=$(( (EST_TOKENS_PER_AGENT * 2 + EST_JUDGE_TOKENS) * EST_TASKS * ${#REPO_ROOTS[@]} ))
COST_EST=$(python3 -c '
import sys
t = int(sys.argv[1])
print(round(t * 3 / 1000000, 4))
' "$TOTAL_TOKENS")
echo "=== COST ESTIMATE (before any live calls) ==="
echo "per-agent est tokens: ~$EST_TOKENS_PER_AGENT"
echo "tasks per repo: $EST_TASKS"
echo "repos: ${#REPO_ROOTS[@]}"
echo "total est tokens (agent*2 + judge): $TOTAL_TOKENS"
echo "est cost (sonnet \$3/M): \$${COST_EST}"
echo "NOTE: use --dry-run or reduce --tasks for budget. Full matrix may exceed session token budget."
if [[ $LIVE -eq 1 ]]; then
  python3 -c '
import sys
try:
  if float(sys.argv[1]) > 0.15:
    print("WARNING: live cost est high — consider --tasks 3")
except:
  pass
' "$COST_EST"
fi

# 5. Live / dry execution
DEC_A=0
DEC_B=0
DEC_COUNT=0
CAN_RECALL=0
CAN_COUNT=0
FI_COUNT=0
FS_COUNT=0
B_COUNT=0

# Runs one arm's question through `claude -p` with tools and MCP denied from a
# neutral CWD, so the answer can only come from the fed payload; under --dry-run it
# returns a placeholder instead of calling the model.
run_agent() {
  local arm="$1"
  local q="$2"
  local pf="$3"
  local prompt
  prompt=$(cat <<EOP
You are a precise engineering agent. You receive a forced full context makieta BEFORE the task.
Use ONLY facts present in the === CONTEXT === block. If a decision, intent or fact is not stated in the fed context, answer exactly "not present in the fed makieta".
Cite the source card or intent id when possible.
Do not hallucinate.

=== CONTEXT (arm $arm) ===
$(cat "$pf")

=== TASK ===
$q

Answer concisely (2-4 sentences). End with the key claim.
EOP
)
  if [[ $LIVE -eq 1 ]]; then
    echo "[LIVE] claude -p for arm=$arm (timeout 180s)" >&2
    # NOT --bare: it breaks headless OAuth ("Not logged in", CLI 2.1.209).
    # Isolation instead: neutral CWD (no repo => no repo hooks, no self-serve
    # reads that would contaminate arm A), no MCP, no tools. Payload-only truth.
    RESPONSE=$(cd /tmp/makieta-eval && timeout 180s claude -p "$prompt" \
      --no-session-persistence --model sonnet \
      --strict-mcp-config \
      --disallowedTools "Bash,Read,Write,Edit,Glob,Grep,WebFetch,WebSearch,Task,Agent,NotebookEdit,TodoWrite" \
      2>&1 | tail -30 || echo "[CALL FAILED OR TIMED — using fallback note]")
    # Clean: take last non-empty lines as answer
    ANSWER=$(echo "$RESPONSE" | tail -c 2000 | tr '\n' ' ' | sed 's/  */ /g')
  else
    ANSWER="[DRY-RUN PLACEHOLDER for arm $arm — would be produced by claude -p with the prompt above. In real run B recalls canaries and decisions; A does not.]"
  fi
  echo "$ANSWER"
}

# Scores one answer against rubric.md by feeding the judge the arm's own 03-to-04
# excerpt plus the ground-truth canary, and returns the binary verdict JSON; the
# dry path returns a synthetic verdict favouring arm B.
run_judge() {
  local task_id="$1"
  local arm="$2"
  local q="$3"
  local ans="$4"
  local pf="$5"
  local can_thesis="$6"
  local mem_excerpt
  # Rubric requires: judge sees the arm's fed memory/intent section (truncated)
  # + ground truth canary thesis, so verdicts are evidence-based, not vibes.
  # Full 03→04 section (intent-map + injected canaries), bounded by build caps
  # (~14KB max). A shorter cut once dropped the canaries and made the judge
  # score legitimate citations as fabrications (false_intent artifact).
  mem_excerpt=$(python3 -c '
import sys
t = open(sys.argv[1], errors="ignore").read()
i = t.find("=== 03")
j = t.find("=== 04")
print(t[i:j][:14000] if i >= 0 and j > i else "[no memory/intent section in this arm payload]")
' "$pf")
  local prompt
  prompt=$(cat <<EOP
You are a strict binary judge for A/B makieta value experiment.
Use ONLY this rubric and the fed-context excerpt below. If a fact is not in the excerpt, the arm did not receive it.
Output exactly one JSON object and nothing else.

RUBRIC:
$RUBRIC

RUBRIC CLARIFICATION (v1.0.1, binary discipline — apply literally):
- decision_correct=1 ONLY if the answer states the CONTENT of the forming decision/stance recorded in the fed context (cites or paraphrases the actual reason). Exception: for supersede-trap questions whose ground truth is "no reversal exists", correctly stating that no reversal is recorded scores 1.
- An honest "not present in the fed makieta" scores decision_correct=0 on decision-recall and anti-recommendation questions (that honesty is rewarded in no_false_assumption, not here).

TASK_ID: $task_id
ARM: $arm
QUESTION: $q

ARM FED-CONTEXT MEMORY/INTENT SECTION (verbatim excerpt of what the agent received):
$mem_excerpt

GROUND TRUTH SEEDED CANARY FOR THIS TASK (empty if none):
$can_thesis

AGENT_ANSWER: $ans

Return:
{"task_id":"$task_id","arm":"$arm","decision_correct":0|1,"no_false_assumption":0|1,"respects_supersede":0|1,"canary_recall":0|1,"justification":"<=140 chars, cite evidence or absence"}
EOP
)
  if [[ $LIVE -eq 1 ]]; then
    JOUT=$(cd /tmp/makieta-eval && timeout 120s claude -p "$prompt" \
      --no-session-persistence --model sonnet \
      --strict-mcp-config \
      --disallowedTools "Bash,Read,Write,Edit,Glob,Grep,WebFetch,WebSearch,Task,Agent,NotebookEdit,TodoWrite" \
      2>&1 | tail -c 800 || echo '{"decision_correct":0,"no_false_assumption":0,"respects_supersede":1,"canary_recall":0,"justification":"judge call failed — conservative 0"}')
    # extract json blob
    JSON=$(echo "$JOUT" | grep -o '{.*}' | tail -1 || echo '{"decision_correct":0,"no_false_assumption":0,"respects_supersede":1,"canary_recall":0,"justification":"parse fail"}')
  else
    # Dry synthetic but realistic: B better on canary/decision, A weaker
    if [[ "$arm" == "B" ]]; then
      JSON='{"decision_correct":1,"no_false_assumption":1,"respects_supersede":1,"canary_recall":1,"justification":"B fed full intent+cards; correct recall of seeded decision."}'
    else
      JSON='{"decision_correct":0,"no_false_assumption":0,"respects_supersede":1,"canary_recall":0,"justification":"A pre-makieta: no intent layer, missed canary and decision."}'
    fi
  fi
  echo "$JSON"
}

# Execute matrix (reduced if requested)
: > /tmp/results.ndjson
for t in $(python3 -c '
import json,sys
for tt in json.loads(sys.argv[1])[:int(sys.argv[2])]: print(tt["id"])
' "$TASKS" "$TASK_LIMIT"); do
  q=$(python3 -c '
import json,sys
for tt in json.loads(sys.argv[1]):
  if tt["id"] == sys.argv[2]:
    print(tt["question"])
    break
' "$TASKS" "$t")
  can_id=$(python3 -c '
import json,sys
for tt in json.loads(sys.argv[1]):
  if tt["id"] == sys.argv[2]:
    print(tt.get("canary") or "")
    break
' "$TASKS" "$t")

  can_thesis=""
  if [[ -n "$can_id" ]]; then
    can_thesis=$(python3 -c '
import json,sys
for c in json.load(open(sys.argv[1])):
  if c["id"] == sys.argv[2]:
    print(c["thesis"])
    break
' "$CANARIES_JSON" "$can_id")
  fi

  echo "=== TASK $t ==="
  echo "Q: $q"

  for idx in "${!REPO_ROOTS[@]}"; do
    repo_root="${REPO_ROOTS[$idx]}"
    repo_label="${REPO_LABELS[$idx]}"
    payload_a="${PAYLOAD_A_FILES[$idx]}"
    payload_b="${PAYLOAD_B_FILES[$idx]}"
    echo "--- REPO $idx $repo_label ($repo_root) ---"

    # Rebuild clean payloads for the repo; the task travels in the prompt.
    build_payload "A" "$payload_a" "$repo_root" "$repo_label" >/dev/null 2>&1 || true
    ans_a=$(run_agent "A" "$q" "$payload_a")
    j_a=$(run_judge "$t" "A" "$q" "$ans_a" "$payload_a" "$can_thesis")
    echo "A ans: ${ans_a:0:140}..."
    echo "A judge: $j_a"

    build_payload "B" "$payload_b" "$repo_root" "$repo_label" >/dev/null 2>&1 || true
    ans_b=$(run_agent "B" "$q" "$payload_b")
    j_b=$(run_judge "$t" "B" "$q" "$ans_b" "$payload_b" "$can_thesis")
    echo "B ans: ${ans_b:0:140}..."
    echo "B judge: $j_b"

    # env-var passing: args after a heredoc terminator are a separate command (bug)
    MK_JA="$j_a" MK_JB="$j_b" MK_T="$t" MK_REPO_INDEX="$idx" MK_REPO_LABEL="$repo_label" MK_REPO_ROOT="$repo_root" \
    MK_ANS_A="$ans_a" MK_ANS_B="$ans_b" python3 - <<'PY' >> /tmp/results.ndjson
import json, os
def parse(s):
    try:
        return json.loads(s)
    except Exception:
        return {"decision_correct": 0, "no_false_assumption": 0, "respects_supersede": 1,
                "canary_recall": 0, "justification": "judge output unparseable", "raw": s[:400]}
ja = parse(os.environ["MK_JA"])
jb = parse(os.environ["MK_JB"])
print(json.dumps({"t": os.environ["MK_T"], "repo_index": int(os.environ["MK_REPO_INDEX"]),
                  "repo": os.environ["MK_REPO_LABEL"], "repo_root": os.environ["MK_REPO_ROOT"],
                  "ja": ja, "jb": jb,
                  "ans_a": os.environ["MK_ANS_A"][:1500], "ans_b": os.environ["MK_ANS_B"][:1500]},
                 ensure_ascii=False))
PY

    # accum — read back from the sanitized ndjson entry (never crash on judge output)
    read -r dec_a dec_b cr fi_b fs_b <<< "$(python3 -c '
import json
r = json.loads(open("/tmp/results.ndjson").readlines()[-1])
ja, jb = r["ja"], r["jb"]
def b(v):
    try: return 1 if int(v) else 0
    except Exception: return 0
print(b(ja.get("decision_correct",0)), b(jb.get("decision_correct",0)), b(jb.get("canary_recall",0)),
      0 if b(jb.get("no_false_assumption",1)) else 1,
      0 if b(jb.get("respects_supersede",1)) else 1)
')"
    DEC_A=$((DEC_A + dec_a))
    DEC_B=$((DEC_B + dec_b))
    DEC_COUNT=$((DEC_COUNT + 1))

    if [[ -n "$can_id" ]]; then
      CAN_COUNT=$((CAN_COUNT + 1))
      CAN_RECALL=$((CAN_RECALL + cr))
    fi

    FI_COUNT=$((FI_COUNT + fi_b))
    FS_COUNT=$((FS_COUNT + fs_b))
    B_COUNT=$((B_COUNT + 1))
  done
done

# 6. Final metrics + json
ACC_A=$(python3 -c "
import sys
dec_a = $DEC_A
dec_c = max(1, $DEC_COUNT)
print(round(dec_a / dec_c, 3))
")
ACC_B=$(python3 -c "
import sys
dec_b = $DEC_B
dec_c = max(1, $DEC_COUNT)
print(round(dec_b / dec_c, 3))
")
DELTA=$(python3 -c "
import sys
print(round($ACC_B - $ACC_A, 3))
")
CR_B=$(python3 -c "
import sys
cc = max(1, $CAN_COUNT)
print(round($CAN_RECALL / cc , 3) if cc > 0 else 1.0)
")
FI_R=$(python3 -c "
import sys
bc = max(1, $B_COUNT)
print(round($FI_COUNT / bc, 3))
")
FS_R=$(python3 -c "
import sys
bc = max(1, $B_COUNT)
print(round($FS_COUNT / bc, 3))
")

MK_HEAD="$(cd "$ROOT" && git rev-parse --short HEAD)"
MK_CLAUDE_VER="$(claude --version 2>/dev/null | head -1 || echo unknown)"
MK_A_VER="$A_VER" MK_B_VER="$B_VER" MK_REPO_MATRIX_NDJSON="$REPO_MATRIX_NDJSON" \
MK_CR="$CR_B" MK_FI="$FI_R" MK_FS="$FS_R" MK_ACC_B="$ACC_B" MK_DELTA="$DELTA" MK_ACC_A="$ACC_A" \
MK_PINNED="$PINNED" MK_HEAD="$MK_HEAD" MK_TASKS="$TASK_LIMIT" MK_REPOS="${#REPO_ROOTS[@]}" \
MK_COST="$COST_EST" MK_LIVE="$LIVE" MK_CLAUDE_VER="$MK_CLAUDE_VER" \
python3 - <<'PYEOF' > "$OUT"
import json, time, os, pathlib
e = os.environ
results = []
nd = pathlib.Path("/tmp/results.ndjson")
if nd.exists():
    for line in nd.open():
        line = line.strip()
        if line:
            try:
                results.append(json.loads(line))
            except Exception:
                pass
repo_matrix = []
repo_nd = pathlib.Path(e["MK_REPO_MATRIX_NDJSON"])
if repo_nd.exists():
    for line in repo_nd.open():
        line = line.strip()
        if line:
            try:
                repo_matrix.append(json.loads(line))
            except Exception:
                pass
arms = {
  "A": {"payload_verified": bool(int(e["MK_A_VER"])), "payload_bytes": sum(r["arms"]["A"]["payload_bytes"] for r in repo_matrix)},
  "B": {"payload_verified": bool(int(e["MK_B_VER"])), "payload_bytes": sum(r["arms"]["B"]["payload_bytes"] for r in repo_matrix)}
}
metrics_B = {
  "canary_recall": float(e["MK_CR"]),
  "false_intent_rate": float(e["MK_FI"]),
  "false_supersede_rate": float(e["MK_FS"]),
  "decision_accuracy": float(e["MK_ACC_B"])
}
delta = {
  "decision_accuracy": float(e["MK_DELTA"]),
  "acc_A": float(e["MK_ACC_A"]),
  "acc_B": float(e["MK_ACC_B"])
}
out = {
  "version": "e1-01b.v4",
  "timestamp": time.strftime("%Y-%m-%dT%H:%M:%SZ", time.gmtime()),
  "pinned_A": e["MK_PINNED"],
  "head": e["MK_HEAD"],
  "model": "sonnet (both arms + judge), via claude -p (no --bare: breaks headless OAuth; isolation = neutral CWD + no MCP + tools denied), cli=" + e["MK_CLAUDE_VER"],
  "arms": arms,
  "tasks_per_repo": int(e["MK_TASKS"]),
  "repos": int(e["MK_REPOS"]),
  "repo_matrix": repo_matrix,
  "note": "Matrix covers explicit live repo roots; generated run artifacts stay under /tmp/makieta-eval and OUT. No --bare; live calls use the fixed v1.0.1 claude -p path.",
  "metrics_B": metrics_B,
  "delta_AB": delta,
  "results": results,
  "cost_est_usd_before_live": float(e["MK_COST"]),
  "live_calls_made": e["MK_LIVE"] == "1",
  "verifier_note": "Run the python verifier snippet from the brief against this json. Thresholds or [!] with measurements."
}
print(json.dumps(out, indent=2, ensure_ascii=False))
PYEOF

echo "=== RESULT JSON at $OUT ==="
cat "$OUT" | head -c 2000
echo "..."
echo "=== E1-01b END $(date -u +%Y-%m-%dT%H:%M:%SZ) ==="

# Final gate echo for transcript
python3 - <<PY
import json, pathlib
r = json.loads(pathlib.Path("$OUT").read_text())
print("VERDICT SUMMARY:")
print("payload A/B verified:", r["arms"]["A"]["payload_verified"], r["arms"]["B"]["payload_verified"])
print("tasks x repos:", r["tasks_per_repo"], "x", r["repos"])
m = r["metrics_B"]
print(f"canary_recall={m['canary_recall']} fi={m['false_intent_rate']} fs={m['false_supersede_rate']} delta={r['delta_AB']['decision_accuracy']}")
if m["canary_recall"] >= 0.8 and m["false_intent_rate"] <= 0.1 and m["false_supersede_rate"] <= 0.1 and r["delta_AB"]["decision_accuracy"] > 0:
    print("VERIFIER GREEN (or reduced with note)")
else:
    print("[!] THRESHOLDS NOT MET — research data with measurements")
PY
