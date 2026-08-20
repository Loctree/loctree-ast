#!/usr/bin/env bash
# ============================================================================
# loct-read-context.sh v2 - File context augmentation for Read tool (no impact, slice only)
# ============================================================================
# Purpose:
#   PostToolUse hook for Claude Code that adds loctree context when reading
#   source files. Shows file structure (exports, imports) and impact analysis.
#
# Key goals:
#   - Claude receives file structure via hookSpecificOutput.additionalContext
#   - Does NOT suppress output (user should see file contents)
#   - Only triggers for source files (not configs, READMEs, etc.)
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
# Logging (optional, same location as grep hook)
# ---------------------------------------------------------------------------
LOG_FILE="${LOCT_HOOK_LOG_FILE:-$HOME/.claude/logs/loct-read.log}"
mkdir -p "$(dirname "$LOG_FILE")" 2>/dev/null || true

log_line() {
  printf '%s\n' "$*" >>"$LOG_FILE" 2>/dev/null || true
}

# ---------------------------------------------------------------------------
# Read hook input (stdin JSON)
# ---------------------------------------------------------------------------
HOOK_INPUT="$(cat)"
[[ -z "$HOOK_INPUT" ]] && exit 0

# Extract file path
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

# Skip test files, mocks, and generated files
if printf '%s' "$FILE_PATH" | grep -qiE '(\.test\.|\.spec\.|\.mock\.|__tests__|__mocks__|\.generated\.|\.d\.ts$)'; then
  exit 0
fi

# Skip node_modules, dist, build directories
if printf '%s' "$FILE_PATH" | grep -qE '(node_modules|/dist/|/build/|/target/|\.next/)'; then
  exit 0
fi

# ---------------------------------------------------------------------------
# Working directory & repo root discovery
# ---------------------------------------------------------------------------
SESSION_CWD="$(printf '%s' "$HOOK_INPUT" | jq -r '.session_cwd // .cwd // empty' 2>/dev/null)"
FILE_DIR="$(dirname "$FILE_PATH")"

# Try to find repo root (with .loctree)
REPO_ROOT="$FILE_DIR"
while [[ "$REPO_ROOT" != "/" ]] && [[ ! -d "$REPO_ROOT/.loctree" ]]; do
  REPO_ROOT="$(dirname "$REPO_ROOT")"
done

if [[ ! -d "$REPO_ROOT/.loctree" ]]; then
  # No .loctree found, skip augmentation
  exit 0
fi

# Make path relative for loct
REL_PATH="${FILE_PATH#$REPO_ROOT/}"
[[ "$REL_PATH" == "$FILE_PATH" ]] && REL_PATH="$FILE_PATH"

# Run loct commands in subshell to avoid prompt pollution
run_loct_in_repo() {
  (cd "$REPO_ROOT" && "$@") 2>&1
}

# ---------------------------------------------------------------------------
# Logging
# ---------------------------------------------------------------------------
log_line ""
log_line "==== LOCT READ CONTEXT ===="
log_line "time: $(date '+%Y-%m-%d %H:%M:%S')"
log_line "file: $REL_PATH"
log_line "repo: $(basename "$REPO_ROOT")"
log_line "==========================="

# ---------------------------------------------------------------------------
# Run loct commands
# ---------------------------------------------------------------------------
SLICE_OUTPUT=""

# Get file structure (slice) - deps + consumers summary
# NOTE: impact analysis removed to reduce context bloat.
# The loct-edit-warning hook handles impact checks before edits.
SLICE_OUTPUT="$(run_loct_in_repo loct slice "$REL_PATH")" || true

# Skip if empty
if [[ -z "$SLICE_OUTPUT" ]]; then
  exit 0
fi

# ---------------------------------------------------------------------------
# Format output
# ---------------------------------------------------------------------------
REPO_NAME="$(basename "$REPO_ROOT")"

CONTEXT="LOCTREE FILE CONTEXT
repo: ${REPO_NAME}
file: ${REL_PATH}
"

if [[ -n "$SLICE_OUTPUT" ]]; then
  CONTEXT+="
--- FILE STRUCTURE (slice) ---
${SLICE_OUTPUT}
"
fi

# ---------------------------------------------------------------------------
# Emit JSON output
# ---------------------------------------------------------------------------
CTX_JSON="$(printf '%s' "$CONTEXT" | jq -Rs .)"

cat <<EOF
{
  "hookSpecificOutput": {
    "hookEventName": "PostToolUse",
    "additionalContext": $CTX_JSON
  }
}
EOF
