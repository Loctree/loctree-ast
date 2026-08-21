#!/usr/bin/env bash
# doctor.sh — health gate for the aicx-compact-recall Codex plugin: validates the
# source and installed payload contracts, source/cache byte identity, the live hook
# registry, and a real direct-file precompact/recall smoke inside a disposable HOME.
set -euo pipefail

export PATH="$HOME/.cargo/bin:/opt/homebrew/bin:/usr/local/bin:/usr/bin:/bin:$PATH"
umask 077

plugin=$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)
aicx_bin="${AICX_BIN:-aicx}"

# fail MSG : reports a broken gate on stderr and aborts the whole doctor run.
fail() { printf 'FAIL: %s\n' "$*" >&2; exit 1; }
# pass MSG : reports a satisfied gate on stdout; informational, never exits.
pass() { printf 'PASS: %s\n' "$*"; }
# note MSG : reports a non-blocking observation on stdout; never fails the run.
note() { printf 'NOTE: %s\n' "$*"; }

if [[ "$aicx_bin" == */* ]]; then
  [ -x "$aicx_bin" ] || fail "AICX_BIN is not executable: $aicx_bin"
else
  command -v "$aicx_bin" >/dev/null 2>&1 || fail "AICX binary not found: $aicx_bin"
  aicx_bin=$(command -v "$aicx_bin")
fi

# validate_payload ROOT LABEL : asserts one plugin payload (source tree or installed
# cache) carries the manifests, executable hooks, the exact PreCompact +
# SessionStart(compact) pair and no literal PostCompact, then bash -n/shellchecks it.
validate_payload() {
  local root="$1" label="$2" pre post
  pre="$root/scripts/aicx-precompact.sh"
  post="$root/scripts/aicx-postcompact.sh"
  [ -f "$root/.codex-plugin/plugin.json" ] || fail "$label plugin.json missing"
  [ -f "$root/hooks/hooks.json" ] || fail "$label hooks.json missing"
  [ -x "$pre" ] || fail "$label precompact hook missing or not executable"
  [ -x "$post" ] || fail "$label postcompact hook missing or not executable"
  jq -e '.hooks.PreCompact[0].hooks[0].command | contains("$PLUGIN_ROOT/scripts/aicx-precompact.sh")' \
    "$root/hooks/hooks.json" >/dev/null || fail "$label Codex precompact command missing"
  jq -e '.hooks.SessionStart[0].matcher == "compact"' \
    "$root/hooks/hooks.json" >/dev/null || fail "$label compact-only recall matcher missing"
  jq -e '.hooks.SessionStart[0].hooks[0].command | contains("$PLUGIN_ROOT/scripts/aicx-postcompact.sh")' \
    "$root/hooks/hooks.json" >/dev/null || fail "$label plugin-owned recall command missing"
  jq -e 'has("PostCompact") | not' "$root/hooks/hooks.json" >/dev/null \
    || fail "$label literal PostCompact would ignore recall stdout"
  bash -n "$pre" "$post"
  if command -v shellcheck >/dev/null 2>&1; then shellcheck -S warning "$pre" "$post"; fi
  pass "$label payload contract"
}

validate_payload "$plugin" "source"
python3 "$plugin/tests/test_hooks.py"
pass "isolated argv/dedup/failure/sanitization fixtures"

codex --strict-config --version >/dev/null
plugin_json=$(codex plugin list --json)
installed_version=$(printf '%s' "$plugin_json" | jq -r \
  '.installed[] | select(.pluginId == "aicx-compact-recall@personal" and .installed and .enabled) | .version' \
  | head -n 1)
[ -n "$installed_version" ] || fail "plugin is not installed and enabled"
installed="$HOME/.codex/plugins/cache/personal/aicx-compact-recall/$installed_version"
[ -d "$installed" ] || fail "resolved installed cache missing: $installed"
validate_payload "$installed" "installed $installed_version"

source_version=$(jq -r '.version' "$plugin/.codex-plugin/plugin.json")
if [ "$source_version" = "$installed_version" ]; then
  while IFS= read -r rel; do
    [ -f "$installed/$rel" ] || fail "installed payload missing $rel"
    cmp -s "$plugin/$rel" "$installed/$rel" || fail "source/cache mismatch: $rel"
  done < <(cd "$plugin" && find . -type f \
    ! -path './.git/*' ! -name '.DS_Store' ! -path './tests/__pycache__/*' \
    -print | sed 's|^./||' | sort)
  pass "source/cache payload byte identity ($source_version)"
  python3 "$plugin/scripts/check_registry.py"
else
  note "activation pending: source=$source_version installed=$installed_version"
  note "installed payload was tested independently; reinstall before final identity/registry gate"
fi

# Real C5X direct-file smoke in an isolated HOME. Copy one operator transcript
# into the disposable home so path validation and discovery cannot touch live
# Claude/Codex stores during the hook subprocess.
operator_home="$HOME"
transcript=$(find "$operator_home/.codex/sessions" -type f -name '*.jsonl' -print0 2>/dev/null \
  | xargs -0 ls -1t 2>/dev/null | head -n 1 || true)
[ -f "$transcript" ] || fail "no Codex transcript available for direct-file smoke"
sid=$(basename "$transcript" | sed -E 's/^.*-([0-9a-f]{8}-[0-9a-f-]{27})\.jsonl$/\1/')
[[ "$sid" =~ ^[0-9a-f]{8}-[0-9a-f-]{27}$ ]] || fail "cannot parse session id from transcript"
home_tmp=$(mktemp -d "${TMPDIR:-/tmp}/aicx-doctor-home.XXXXXX")
recall_tmp=$(mktemp "${TMPDIR:-/tmp}/aicx-doctor-recall.XXXXXX")
trap 'rm -rf "$home_tmp" "$recall_tmp"' EXIT
copied="$home_tmp/.codex/sessions/$(basename "$transcript")"
mkdir -p "$(dirname "$copied")"
cp "$transcript" "$copied"
payload=$(jq -nc --arg sid "$sid" --arg path "$copied" \
  '{session_id:$sid,transcript_path:$path,hook_event_name:"PreCompact",trigger:"manual"}')
start=$SECONDS
printf '%s' "$payload" | HOME="$home_tmp" USERPROFILE="$home_tmp" \
  AICX_BIN="$aicx_bin" AICX_HOOK_AGENT=codex bash "$plugin/scripts/aicx-precompact.sh"
elapsed=$((SECONDS - start))
extract="$home_tmp/.aicx/extracts/codex/${sid}_conversation.md"
[ -s "$extract" ] || fail "real C5X direct-file conversation extract missing"
[ "$elapsed" -lt 10 ] || fail "precompact extraction took ${elapsed}s (budget <10s)"
pass "real C5X direct-file extract (${elapsed}s)"

jq -nc --arg sid "$sid" '{session_id:$sid,hook_event_name:"SessionStart",source:"compact"}' \
  | HOME="$home_tmp" USERPROFILE="$home_tmp" AICX_HOOK_AGENT=codex \
    bash "$plugin/scripts/aicx-postcompact.sh" >"$recall_tmp"
grep -q 'AICX RECALL' "$recall_tmp" || fail "recall header missing"
grep -q '\[P0\] LATEST ASK' "$recall_tmp" || fail "latest ask missing"
grep -q '^▓▓▓ STATE' "$recall_tmp" || fail "state section missing"
[ "$(wc -c <"$recall_tmp")" -lt 12000 ] || fail "recall exceeds 12KB budget"
pass "post-compact recall digest ($(wc -c <"$recall_tmp" | tr -d ' ') bytes)"

printf 'ACTIVATION BOUNDARY: already-running Codex processes require restart/resume/new session; fresh registry proof is not hot-reload proof.\n'
printf 'AICX compact recall source is healthy; source=%s installed=%s.\n' "$source_version" "$installed_version"
