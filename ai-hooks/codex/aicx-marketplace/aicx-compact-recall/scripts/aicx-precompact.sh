#!/usr/bin/env bash
# ============================================================================
# aicx-precompact.sh — extract current session before compaction
# ============================================================================
# TRIGGER: PreCompact hook (manual or auto)
# PURPOSE: Persist the current Codex conversation from the hook-provided
#          transcript path directly, without any catalog or corpus scan.
#
# REQUIRES: aicx CLI in PATH (~/.cargo/bin/) or AICX_BIN
# BUDGET: Codex direct-file extraction is normally <1s. No network.
# OUTPUT: no additionalContext — silent extract; recall belongs to PostCompact.
# ============================================================================

set -uo pipefail
export PATH="$HOME/.cargo/bin:/opt/homebrew/bin:/usr/local/bin:/usr/bin:/bin:$PATH"
umask 077

aicx_bin="${AICX_BIN:-aicx}"
if [[ "$aicx_bin" == */* ]]; then
  [ -x "$aicx_bin" ] || exit 0
else
  command -v "$aicx_bin" >/dev/null 2>&1 || exit 0
fi

input=$(cat)
session_id=$(printf '%s' "$input" | jq -r '.session_id // empty' 2>/dev/null || true)
[ -z "$session_id" ] && exit 0

agent="${AICX_HOOK_AGENT:-codex}"
transcript_path=$(printf '%s' "$input" | jq -r '.transcript_path // .transcriptPath // empty' 2>/dev/null || true)
extract_dir="$HOME/.aicx/extracts/$agent"
extract="$extract_dir/${session_id}_conversation.md"
tmp_extract="$extract.tmp.$$"
mkdir -p "$extract_dir" "$HOME/.aicx/state"

# Codex passes the exact session transcript. Direct-file mode is the only
# accepted path: it is deterministic and cannot fall back to a corpus/session
# scan. A missing path stays fail-open for compaction but leaves a loud log.
if [ "$agent" != "codex" ]; then
  printf 'Unsupported compact-recall agent %s; this plugin is Codex-only\n' "$agent" \
    >"$HOME/.aicx/state/precompact-${agent}-${session_id}.log"
elif [ -n "$transcript_path" ] && [ -f "$transcript_path" ]; then
  raw_bytes=$(wc -c <"$transcript_path" | tr -d ' ')
  raw_mtime=$(stat -f '%m' "$transcript_path" 2>/dev/null || stat -c '%Y' "$transcript_path" 2>/dev/null || echo 0)
  if "$aicx_bin" extract codex --file "$transcript_path" --conversation -o "$tmp_extract" \
    >/dev/null 2>"$HOME/.aicx/state/precompact-${agent}-${session_id}.log"; then
    mv -f "$tmp_extract" "$extract"
    # Atomic freshness manifest: raw → extract. Postcompact refuses stale files.
    printf '{"session_id":%s,"agent":%s,"transcript_path":%s,"raw_bytes":%s,"raw_mtime":%s,"extract_path":%s,"ok":true}\n' \
      "$(printf '%s' "$session_id" | jq -Rs .)" \
      "$(printf '%s' "$agent" | jq -Rs .)" \
      "$(printf '%s' "$transcript_path" | jq -Rs .)" \
      "$raw_bytes" \
      "$raw_mtime" \
      "$(printf '%s' "$extract" | jq -Rs .)" \
      >"$extract.freshness.json"
    rm -f "$HOME/.aicx/state/precompact-${agent}-${session_id}.log"
  else
    rm -f "$tmp_extract"
    # Fail-closed: never leave a prior extract as "JUST COMPACTED" truth.
    if [ -f "$extract" ]; then
      rm -f "$extract" "$extract.freshness.json"
      printf 'PreCompact extract FAILED for %s; removed stale extract so PostCompact cannot lie\n' "$session_id" \
        >>"$HOME/.aicx/state/precompact-${agent}-${session_id}.log"
    fi
  fi
else
  printf 'Codex transcript not found for session %s\n' "$session_id" \
    >"$HOME/.aicx/state/precompact-${agent}-${session_id}.log"
fi

exit 0
