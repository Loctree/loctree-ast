#!/usr/bin/env bash
# ============================================================================
# loct-edit-warning.sh v2 - Impact warning after editing critical files
# ============================================================================
# Purpose:
#   PostToolUse hook for Claude Code that warns AFTER editing files with
#   high impact (many dependents). Shows impact analysis so Claude knows
#   what might break with subsequent edits.
#
# Key goals:
#   - WARNING for files with 10+ direct consumers
#   - Always show impact context (even for low-impact files)
#   - Does NOT block edit, just adds awareness
#
# ============================================================================

set -uo pipefail

export PATH="$HOME/.cargo/bin:/opt/homebrew/bin:/usr/local/bin:/usr/bin:/bin:$PATH"

# ---------------------------------------------------------------------------
# Dependencies
# ---------------------------------------------------------------------------
command -v loct >/dev/null 2>&1 || exit 0
command -v jq   >/dev/null 2>&1 || exit 0

# ---------------------------------------------------------------------------
# Configuration
# ---------------------------------------------------------------------------
CRITICAL_THRESHOLD=10  # Show warning if 10+ direct consumers
LOG_FILE="${LOCT_HOOK_LOG_FILE:-$HOME/.claude/logs/loct-edit.log}"
mkdir -p "$(dirname "$LOG_FILE")" 2>/dev/null || true

log_line() {
  printf '%s\n' "$*" >>"$LOG_FILE" 2>/dev/null || true
}

# ---------------------------------------------------------------------------
# Read hook input (stdin JSON)
# ---------------------------------------------------------------------------
HOOK_INPUT="$(cat)"
[[ -z "$HOOK_INPUT" ]] && exit 0

# Extract file path from Edit tool
FILE_PATH="$(printf '%s' "$HOOK_INPUT" | jq -r '.tool_input.file_path // empty' 2>/dev/null)"
[[ -z "$FILE_PATH" ]] && exit 0
[[ ! -f "$FILE_PATH" ]] && exit 0

# ---------------------------------------------------------------------------
# Filter: only source files
# ---------------------------------------------------------------------------
SOURCE_EXTENSIONS="ts|tsx|js|jsx|rs|py|vue|svelte|go|rb|java|kt|swift|c|cpp|h|hpp"

if ! printf '%s' "$FILE_PATH" | grep -qE "\.($SOURCE_EXTENSIONS)$"; then
  exit 0
fi

# Skip test files
if printf '%s' "$FILE_PATH" | grep -qiE '(\.test\.|\.spec\.|\.mock\.|__tests__|__mocks__)'; then
  exit 0
fi

# Skip generated/vendor
if printf '%s' "$FILE_PATH" | grep -qE '(node_modules|/dist/|/build/|/target/|\.next/|\.generated\.)'; then
  exit 0
fi

# ---------------------------------------------------------------------------
# Find repo root with .loctree
# ---------------------------------------------------------------------------
FILE_DIR="$(dirname "$FILE_PATH")"
REPO_ROOT="$FILE_DIR"
while [[ "$REPO_ROOT" != "/" ]] && [[ ! -d "$REPO_ROOT/.loctree" ]]; do
  REPO_ROOT="$(dirname "$REPO_ROOT")"
done

if [[ ! -d "$REPO_ROOT/.loctree" ]]; then
  exit 0
fi

# Make path relative
REL_PATH="${FILE_PATH#$REPO_ROOT/}"
[[ "$REL_PATH" == "$FILE_PATH" ]] && REL_PATH="$FILE_PATH"

# ---------------------------------------------------------------------------
# Run loct impact in subshell
# ---------------------------------------------------------------------------
run_loct_in_repo() {
  (cd "$REPO_ROOT" && "$@") 2>&1
}

IMPACT_OUTPUT="$(run_loct_in_repo loct impact "$REL_PATH")" || true

[[ -z "$IMPACT_OUTPUT" ]] && exit 0

# ---------------------------------------------------------------------------
# Parse impact counts
# ---------------------------------------------------------------------------
# Extract "Direct consumers (N files):" line
DIRECT_COUNT=$(printf '%s' "$IMPACT_OUTPUT" | grep -oE 'Direct consumers \([0-9]+ files?\)' | grep -oE '[0-9]+' | head -1)
[[ -z "$DIRECT_COUNT" ]] && DIRECT_COUNT=0

# Extract total affected from "[!] Removing this file would affect N files"
TOTAL_AFFECTED=$(printf '%s' "$IMPACT_OUTPUT" | grep -oE 'would affect [0-9]+ files' | grep -oE '[0-9]+' | head -1)
[[ -z "$TOTAL_AFFECTED" ]] && TOTAL_AFFECTED=0

# ---------------------------------------------------------------------------
# Logging
# ---------------------------------------------------------------------------
log_line ""
log_line "==== LOCT EDIT WARNING ===="
log_line "time: $(date '+%Y-%m-%d %H:%M:%S')"
log_line "file: $REL_PATH"
log_line "direct_consumers: $DIRECT_COUNT"
log_line "total_affected: $TOTAL_AFFECTED"
log_line "threshold: $CRITICAL_THRESHOLD"
log_line "==========================="

# ---------------------------------------------------------------------------
# Build output
# ---------------------------------------------------------------------------
REPO_NAME="$(basename "$REPO_ROOT")"
IS_CRITICAL="false"
WARNING_MSG=""

if [[ "$DIRECT_COUNT" -ge "$CRITICAL_THRESHOLD" ]]; then
  IS_CRITICAL="true"
  WARNING_MSG="⚠️  CRITICAL FILE: ${REL_PATH} has ${DIRECT_COUNT} direct consumers (${TOTAL_AFFECTED} total affected). Changes here have HIGH IMPACT."
fi

# Build context (always include impact)
CONTEXT="LOCTREE EDIT IMPACT
repo: ${REPO_NAME}
file: ${REL_PATH}
direct_consumers: ${DIRECT_COUNT}
total_affected: ${TOTAL_AFFECTED}
critical: ${IS_CRITICAL}

${IMPACT_OUTPUT}"

# Truncate if too large
MAX_BYTES=16384
if [[ ${#CONTEXT} -gt $MAX_BYTES ]]; then
  CONTEXT="${CONTEXT:0:$MAX_BYTES}

[...truncated]"
fi

# ---------------------------------------------------------------------------
# Emit JSON output
# ---------------------------------------------------------------------------
CTX_JSON="$(printf '%s' "$CONTEXT" | jq -Rs .)"

if [[ "$IS_CRITICAL" == "true" ]]; then
  # Critical file: show warning in systemMessage + additionalContext
  WARNING_JSON="$(printf '%s' "$WARNING_MSG" | jq -Rs .)"
  cat <<EOF
{
  "systemMessage": $WARNING_JSON,
  "hookSpecificOutput": {
    "hookEventName": "PostToolUse",
    "additionalContext": $CTX_JSON
  }
}
EOF
else
  # Non-critical: just add context silently
  cat <<EOF
{
  "hookSpecificOutput": {
    "hookEventName": "PostToolUse",
    "additionalContext": $CTX_JSON
  }
}
EOF
fi
