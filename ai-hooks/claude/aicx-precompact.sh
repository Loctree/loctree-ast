#!/usr/bin/env bash
# ============================================================================
# aicx-precompact.sh — extract current session before compaction
# ============================================================================
# TRIGGER: PreCompact hook (manual or auto)
# PURPOSE: Persist the current conversation before compaction. Claude keeps its
#          original session-mode path. Codex uses the hook-provided transcript
#          path directly, avoiding a global scan of every Codex session.
#
# REQUIRES: aicx CLI in PATH (~/.cargo/bin/)
# BUDGET: Codex direct-file extraction is normally <1s; Claude session mode is
#         typically 1-5s. No network.
# OUTPUT: no additionalContext — silent extract; recall belongs to PostCompact.
# ============================================================================

set -uo pipefail
export PATH="$HOME/.cargo/bin:/opt/homebrew/bin:/usr/local/bin:/usr/bin:/bin:$PATH"
umask 077

# Bail if aicx not available — never block compaction
command -v aicx >/dev/null 2>&1 || exit 0

input=$(cat)
session_id=$(printf '%s' "$input" | jq -r '.session_id // empty' 2>/dev/null || true)
[ -z "$session_id" ] && exit 0

agent="${AICX_HOOK_AGENT:-claude}"
transcript_path=$(printf '%s' "$input" | jq -r '.transcript_path // .transcriptPath // empty' 2>/dev/null || true)
extract_dir="$HOME/.aicx/extracts/$agent"
extract="$extract_dir/${session_id}_conversation.md"
tmp_extract="$extract.tmp.$$"
mkdir -p "$extract_dir" "$HOME/.aicx/state"

# Codex passes the exact session transcript. File mode turns the previous
# 80-second global source scan into a deterministic single-JSONL extraction.
# Fall back to the session-id filename when older harnesses omit transcript_path.
if [ "$agent" = "codex" ]; then
  if [ -z "$transcript_path" ] || [ ! -f "$transcript_path" ]; then
    transcript_path=$(find "$HOME/.codex/sessions" -type f -name "*-${session_id}.jsonl" -print -quit 2>/dev/null || true)
  fi
  if [ -n "$transcript_path" ] && [ -f "$transcript_path" ]; then
    # aicx >=0.11: agent as subcommand; fallback via --agent (deprecated
    # alias on >=0.11.1, native grammar on <=0.10). --format is gone for good.
    if aicx extract codex "$transcript_path" --conversation -o "$tmp_extract" \
      >/dev/null 2>"$HOME/.aicx/state/precompact-${agent}-${session_id}.log" \
      || aicx extract --agent codex "$transcript_path" --conversation -o "$tmp_extract" \
      >/dev/null 2>>"$HOME/.aicx/state/precompact-${agent}-${session_id}.log"; then
      mv -f "$tmp_extract" "$extract"
      rm -f "$HOME/.aicx/state/precompact-${agent}-${session_id}.log"
    else
      rm -f "$tmp_extract"
    fi
  else
    printf 'Codex transcript not found for session %s\n' "$session_id" \
      >"$HOME/.aicx/state/precompact-${agent}-${session_id}.log"
  fi
else
  # Preserve Claude's established session-store extraction behavior.
  # aicx >=0.11: agent as subcommand; fallback to <=0.10 flag grammar.
  aicx extract "$agent" --session "$session_id" --conversation \
    >/dev/null 2>"$HOME/.aicx/state/precompact-${agent}-${session_id}.log" \
    || aicx extract --agent "$agent" --session "$session_id" --conversation \
    >/dev/null 2>>"$HOME/.aicx/state/precompact-${agent}-${session_id}.log" || true
fi

exit 0
