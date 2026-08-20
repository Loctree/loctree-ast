#!/usr/bin/env bash
# ============================================================================
# loct-grep-augment.sh v18 - v17 + REDIRECTION SKIP from v15
# ============================================================================
# Purpose:
#   PostToolUse hook for Claude Code that augments rg/grep searches with loctree
#   context (slice/find/impact/focus/etc.).
#
# Key goals:
#   - Claude receives HUMAN-READABLE loct output via hookSpecificOutput.additionalContext
#   - User sees a small TEXT TABLE (systemMessage)
#   - stdout stays VALID JSON (no mixed stderr/stdout issues)
#   - All loct calls are logged to ~/.claude/logs/loct-grep.log (or LOCT_HOOK_LOG_FILE)
#
# NOTE:
#   Claude Code only processes JSON output on stdout when exit code is 0.
#   Avoid printing anything else to stdout.
# ============================================================================

set -uo pipefail

export PATH="$HOME/.cargo/bin:/opt/homebrew/bin:/usr/local/bin:/usr/bin:/bin:$PATH"

# ---------------------------------------------------------------------------
# Dependencies
# ---------------------------------------------------------------------------
command -v loct >/dev/null 2>&1 || exit 0
command -v jq   >/dev/null 2>&1 || exit 0

# ---------------------------------------------------------------------------
# Logging
# ---------------------------------------------------------------------------
LOG_FILE="${LOCT_HOOK_LOG_FILE:-$HOME/.claude/logs/loct-grep.log}"
mkdir -p "$(dirname "$LOG_FILE")" 2>/dev/null || true

log_line() {
  # Logs to file only. Never to stdout.
  printf '%s\n' "$*" >>"$LOG_FILE" 2>/dev/null || true
}

now_ms() {
  # Best-effort millisecond timestamp (macOS-friendly)
  if command -v gdate >/dev/null 2>&1; then
    gdate +%s%3N
    return
  fi

  # python3 is usually available; if not, fall back to seconds.
  python3 - <<'PY' 2>/dev/null || echo $(( $(date +%s) * 1000 ))
import time
print(int(time.time()*1000))
PY
}

# Global: last loct duration
LOCT_LAST_DURATION_MS=0

run_loct() {
  # Usage: run_loct <subcmd> [args...]
  local subcmd="$1"; shift

  local start_ms end_ms exit_code out
  start_ms="$(now_ms)"

  # Capture both stdout/stderr from loct (for logging + additionalContext)
  out="$(loct "$subcmd" "$@" 2>&1)"
  exit_code=$?

  end_ms="$(now_ms)"
  LOCT_LAST_DURATION_MS=$(( end_ms - start_ms ))

  # Log
  log_line ""
  log_line "---- LOCT CALL ----"
  log_line "time: $(date '+%Y-%m-%d %H:%M:%S')"
  log_line "cwd:  $(pwd)"
  log_line "ms:   ${LOCT_LAST_DURATION_MS}"
  log_line "exit: ${exit_code}"
  log_line "cmd:  loct ${subcmd} $*"
  log_line "output:"
  # indent output
  if [[ -n "$out" ]]; then
    while IFS= read -r line; do
      log_line "  $line"
    done <<<"$out"
  else
    log_line "  (empty)"
  fi
  log_line "-------------------"

  printf '%s' "$out"
  return $exit_code
}

# ---------------------------------------------------------------------------
# Read hook input (stdin JSON)
# ---------------------------------------------------------------------------
HOOK_INPUT="$(cat)"
[[ -z "$HOOK_INPUT" ]] && exit 0

HOOK_EVENT_NAME="$(printf '%s' "$HOOK_INPUT" | jq -r '.hook_event_name // empty' 2>/dev/null)"
# This script is intended for PostToolUse, but we fail open.

# ---------------------------------------------------------------------------
# Log the original Claude search (best effort) - CLEAN ASCII format
# ---------------------------------------------------------------------------
log_claude_search() {
  local tool_name tool_pattern tool_path
  tool_name="$(printf '%s' "$HOOK_INPUT" | jq -r '.tool_name // "(unknown)"' 2>/dev/null)"
  tool_pattern="$(printf '%s' "$HOOK_INPUT" | jq -r '.tool_input.pattern // .tool_input.command // "(unknown)"' 2>/dev/null)"
  tool_path="$(printf '%s' "$HOOK_INPUT" | jq -r '.tool_input.path // "."' 2>/dev/null)"

  # Shorten pattern if too long
  [[ ${#tool_pattern} -gt 60 ]] && tool_pattern="${tool_pattern:0:57}..."

  log_line ""
  log_line "==== CLAUDE: $tool_name ===="
  log_line "time:    $(date '+%Y-%m-%d %H:%M:%S')"
  log_line "pattern: $tool_pattern"
  log_line "path:    $tool_path"

  # Parse tool_response based on tool type
  local tool_resp
  tool_resp="$(printf '%s' "$HOOK_INPUT" | jq '.tool_response // empty' 2>/dev/null)"

  if [[ -n "$tool_resp" && "$tool_resp" != "null" ]]; then
    if [[ "$tool_name" == "Grep" ]]; then
      local mode num_files
      mode="$(printf '%s' "$tool_resp" | jq -r '.mode // "?"' 2>/dev/null)"
      num_files="$(printf '%s' "$tool_resp" | jq -r '.numFiles // 0' 2>/dev/null)"
      log_line "result:  $num_files files ($mode)"

      # Show filenames
      printf '%s' "$tool_resp" | jq -r '.filenames[]? // empty' 2>/dev/null | head -10 | while IFS= read -r filepath; do
        # Pretty-print: try repo-relative first, fall back to basename.
        local short="${filepath#$REPO_ROOT/}"
        [[ "$short" == "$filepath" ]] && short="${filepath##*/}"
        [[ ${#short} -gt 60 ]] && short="...${short: -57}"
        log_line "  -> $short"
      done

    elif [[ "$tool_name" == "Bash" ]]; then
      local stdout
      stdout="$(printf '%s' "$tool_resp" | jq -r '.stdout // ""' 2>/dev/null)"
      if [[ -n "$stdout" ]]; then
        local lc
        lc="$(printf '%s' "$stdout" | wc -l | tr -d ' ')"
        log_line "result:  $lc lines"
        printf '%s' "$stdout" | head -8 | while IFS= read -r line; do
          # Smart shorten for grep output (path:line:content)
          if [[ "$line" == *:*:* ]]; then
            local fp="${line%%:*}" rest="${line#*:}"
            local fn="${fp##*/}" par="${fp%/*}"; par="${par##*/}"
            line="$par/$fn:$rest"
          fi
          [[ ${#line} -gt 70 ]] && line="${line:0:67}..."
          log_line "  $line"
        done
        [[ "$lc" -gt 8 ]] && log_line "  ... (+$((lc - 8)) more)"
      else
        log_line "result:  (empty)"
      fi
    fi
  fi
  log_line "============================"
}

log_claude_search

# ---------------------------------------------------------------------------
# Helpers (tables + escaping)
# ---------------------------------------------------------------------------
repeat_char() {
  local char="$1" n="$2"
  printf '%*s' "$n" '' | tr ' ' "$char"
}

ellipsize() {
  local s="$1" w="$2"
  if [[ "${#s}" -le "$w" ]]; then
    printf '%s' "$s"
  else
    printf '%s…' "${s:0:$((w-1))}"
  fi
}

kv_table() {
  # Text-only key/value table (safe for systemMessage)
  local key_w=8
  local val_w=68
  local border="+-$(repeat_char '-' $key_w)-+-$(repeat_char '-' $val_w)-+"

  echo "LOCTREE"
  echo "$border"
  while [[ "$#" -gt 1 ]]; do
    local k="$1" v="$2"; shift 2
    v="$(ellipsize "$v" "$val_w")"
    printf "| %-*s | %-*s |\n" "$key_w" "$k" "$val_w" "$v"
  done
  echo "$border"
}

extract_count() {
  # Extracts integer from lines like: === Symbol Matches (18) ===
  local label="$1" text="$2"
  printf '%s\n' "$text" | sed -nE "s/^=== ${label} \(([0-9]+)\) ===$/\1/p" | head -1
}

# Max payload size (32KB) to avoid bloating additionalContext
MAX_PAYLOAD_BYTES=32768

truncate_payload() {
  local text="$1"
  local max="$2"
  if [[ ${#text} -gt $max ]]; then
    printf '%s\n\n[...truncated, showing first %d bytes of %d total]' \
      "${text:0:$max}" "$max" "${#text}"
  else
    printf '%s' "$text"
  fi
}

emit_hook_output() {
  # emit_hook_output <action> <payload>
  local action="$1"
  local payload="$2"

  # Truncate if too large
  payload="$(truncate_payload "$payload" "$MAX_PAYLOAD_BYTES")"

  # Repo label (best effort)
  local repo
  repo="$(basename "${REPO_ROOT:-$(pwd)}" 2>/dev/null)"
  [[ -z "$repo" ]] && repo="."

  # Counts (best effort; works for loct find text output)
  local c_sym c_sem c_param
  c_sym="$(extract_count "Symbol Matches" "$payload")"
  c_sem="$(extract_count "Semantic Matches" "$payload")"
  c_param="$(extract_count "Parameter Matches" "$payload")"
  [[ -z "$c_sym" ]] && c_sym="-"
  [[ -z "$c_sem" ]] && c_sem="-"
  [[ -z "$c_param" ]] && c_param="-"

  local sys_msg ctx
  sys_msg="$(kv_table \
    action "$action" \
    repo   "$repo" \
    query  "$PATTERN" \
    path   "$PATH_ARG" \
    ms     "${LOCT_LAST_DURATION_MS}ms" \
    match  "sym:$c_sym sem:$c_sem param:$c_param" \
  )"

  ctx="LOCTREE CONTEXT (${action})
repo: ${repo}
query: ${PATTERN}
path: ${PATH_ARG}

${payload}"

  local sys_json ctx_json
  sys_json="$(printf '%s' "$sys_msg" | jq -Rs .)"
  ctx_json="$(printf '%s' "$ctx" | jq -Rs .)"

  # Claude Code hook output schema (PostToolUse)
  cat <<EOF
{
  "suppressOutput": true,
  "systemMessage": $sys_json,
  "hookSpecificOutput": {
    "hookEventName": "PostToolUse",
    "additionalContext": $ctx_json
  }
}
EOF
}

# ---------------------------------------------------------------------------
# Pattern extraction
# ---------------------------------------------------------------------------
PATTERN=""
PATH_ARG="."

parse_rg_grep_command() {
  # Parses a shell command string and extracts:
  #   - pattern (first non-flag after rg/grep)
  #   - path (last non-flag token after pattern, WITHOUT checking FS)
  # Uses python shlex for robust quoting.
  #
  # NOTE: We do NOT check if path exists here — that's done AFTER cd to SESSION_CWD.
  #       This fixes the cwd mismatch bug where relative paths weren't found.
  local cmd="$1"

  # Ignore everything after first pipe for token parsing
  cmd="${cmd%%|*}"

  local tool_index=-1

  # shlex split via python - bash 3.2 compatible (no mapfile)
  local toks_output
  toks_output="$(printf '%s' "$cmd" | python3 -c '
import shlex, sys
cmd = sys.stdin.read()
try:
  parts = shlex.split(cmd)
except Exception:
  parts = cmd.split()
for p in parts:
  print(p)
' 2>/dev/null)"

  # Build array line by line (bash 3.2 compatible)
  local -a toks=()
  local line
  while IFS= read -r line; do
    toks[${#toks[@]}]="$line"
  done <<< "$toks_output"

  # Find the first rg/grep token
  local i
  for i in "${!toks[@]}"; do
    case "${toks[$i]}" in
      rg|ripgrep|grep)
        tool_index="$i"
        break
        ;;
    esac
  done

  [[ "$tool_index" -lt 0 ]] && return 1

  # Parse pattern: first non-flag after tool
  local pattern=""
  i=$((tool_index + 1))
  for ((; i<${#toks[@]}; i++)); do
    local t="${toks[$i]}"
    if [[ "$t" == "--" ]]; then
      i=$((i+1))
      break
    fi
    [[ "$t" == -* ]] && continue
    pattern="$t"
    i=$((i+1))
    break
  done

  [[ -z "$pattern" ]] && return 1

  # Parse path: last non-flag token after pattern (NO FS check here!)
  local path="."
  local j
  for ((j=${#toks[@]}-1; j>=i; j--)); do
    local candidate="${toks[$j]}"
    # Skip flags and empty tokens
    [[ "$candidate" == -* ]] && continue
    [[ -z "$candidate" ]] && continue
    # Skip shell redirections (2>/dev/null, >/tmp/out, 2>&1, etc.)
    [[ "$candidate" =~ ^[0-9]*\>[^\&] ]] && continue
    [[ "$candidate" =~ ^[0-9]*\>\& ]] && continue
    [[ "$candidate" =~ ^\<\( ]] && continue
    # Accept as path candidate (will be validated after cd)
    path="$candidate"
    break
  done

  printf '%s\n%s\n' "$pattern" "$path"
}

if [[ "${1:-}" == "--bash-filter" ]]; then
  COMMAND="$(printf '%s' "$HOOK_INPUT" | jq -r '.tool_input.command // empty' 2>/dev/null)"
  [[ -z "$COMMAND" ]] && exit 0

  # Only react to rg/grep
  printf '%s' "$COMMAND" | grep -qE '(^|[[:space:]])(rg|ripgrep|grep)[[:space:]]' || exit 0

  if parsed="$(parse_rg_grep_command "$COMMAND")"; then
    PATTERN="$(printf '%s' "$parsed" | sed -n '1p')"
    PATH_ARG="$(printf '%s' "$parsed" | sed -n '2p')"
  else
    # Fallback: best-effort (old heuristic)
    PATTERN="$(printf '%s' "$COMMAND" | grep -oE '"[^"]+"' | head -1 | tr -d '"')"
    [[ -z "$PATTERN" ]] && PATTERN="$(printf '%s' "$COMMAND" | grep -oE "'[^']+'" | head -1 | tr -d "'")"
    [[ -z "$PATTERN" ]] && PATTERN="$(printf '%s' "$COMMAND" | sed -nE 's/.*\b(rg|grep)\b[[:space:]]+([^[:space:]-][^[:space:]]*).*/\2/p')"

    # Fallback path: last token that's not a flag or redirection
    GREP_CMD="${COMMAND%%|*}"
    # Strip shell redirections before extracting path
    GREP_CMD="$(printf '%s' "$GREP_CMD" | sed -E 's/ *[0-9]*>[^[:space:]]* *//g; s/ *[0-9]*>&[0-9]* *//g')"
    last_token="$(printf '%s' "$GREP_CMD" | awk '{print $NF}')"
    # Only use if it doesn't look like a flag
    if [[ "$last_token" != -* ]] && [[ -n "$last_token" ]]; then
      PATH_ARG="$last_token"
    else
      PATH_ARG="."
    fi
  fi
else
  PATTERN="$(printf '%s' "$HOOK_INPUT" | jq -r '.tool_input.pattern // empty' 2>/dev/null)"
  PATH_ARG="$(printf '%s' "$HOOK_INPUT" | jq -r '.tool_input.path // "."' 2>/dev/null)"
fi

# ---------------------------------------------------------------------------
# Working directory & repo root discovery
# ---------------------------------------------------------------------------
SESSION_CWD="$(printf '%s' "$HOOK_INPUT" | jq -r '.session_cwd // .cwd // empty' 2>/dev/null)"

# Expand tilde to $HOME (bash doesn't expand ~ in variables)
# Use string prefix check, not glob pattern (~/*)
if [[ "${PATH_ARG:0:2}" == "~/" ]]; then
  PATH_ARG="$HOME/${PATH_ARG:2}"
elif [[ "$PATH_ARG" == "~" ]]; then
  PATH_ARG="$HOME"
fi

# Prefer path if absolute
if [[ "$PATH_ARG" == /* && -d "$PATH_ARG" ]]; then
  cd "$PATH_ARG" 2>/dev/null || true
elif [[ "$PATH_ARG" == /* && -f "$PATH_ARG" ]]; then
  cd "$(dirname "$PATH_ARG")" 2>/dev/null || true
elif [[ -n "$SESSION_CWD" && -d "$SESSION_CWD" ]]; then
  cd "$SESSION_CWD" 2>/dev/null || true
fi

REPO_ROOT="$(pwd)"
while [[ "$REPO_ROOT" != "/" ]] && [[ ! -d "$REPO_ROOT/.loctree" ]]; do
  REPO_ROOT="$(dirname "$REPO_ROOT")"
done

if [[ -d "$REPO_ROOT/.loctree" ]]; then
  cd "$REPO_ROOT" 2>/dev/null || true
else
  REPO_ROOT="$(pwd)"
fi

# ---------------------------------------------------------------------------
# Validate PATH_ARG now that we're in the correct cwd
# ---------------------------------------------------------------------------
if [[ "$PATH_ARG" != "." ]] && [[ ! -e "$PATH_ARG" ]] && [[ ! -d "$PATH_ARG" ]]; then
  # Path doesn't exist in repo — reset to "."
  PATH_ARG="."
fi

# ---------------------------------------------------------------------------
# Validation / filtering
# ---------------------------------------------------------------------------
[[ -z "$PATTERN" ]] && exit 0
[[ ${#PATTERN} -lt 3 ]] && exit 0

# Skip truly heavy regex patterns (nested groups, excessive wildcards)
# But allow simple alternations like foo|bar|baz
if printf '%s' "$PATTERN" | grep -qE '(\([^)]*\)){3,}'; then exit 0; fi  # 3+ nested groups
if printf '%s' "$PATTERN" | grep -qE '(\.\*.*){4,}'; then exit 0; fi     # 4+ .* wildcards
if printf '%s' "$PATTERN" | grep -qE '\(\?[!=<]'; then exit 0; fi        # lookaheads/lookbehinds

# Strip outer quotes
PATTERN="${PATTERN%\"}"; PATTERN="${PATTERN#\"}"
PATTERN="${PATTERN%\'}"; PATTERN="${PATTERN#\'}"

# ---------------------------------------------------------------------------
# Augmentation functions
# ---------------------------------------------------------------------------
FILE_CONTEXT=""

augment_symbol() {
  local symbol="$1"
  local result

  result="$(run_loct find "$symbol")"
  [[ -z "$result" ]] && return 1

  # If nothing found: param=0, sym=0, sem=0
  local c_sym c_sem c_param
  c_sym="$(extract_count "Symbol Matches" "$result")"; [[ -z "$c_sym" ]] && c_sym=0
  c_sem="$(extract_count "Semantic Matches" "$result")"; [[ -z "$c_sem" ]] && c_sem=0
  c_param="$(extract_count "Parameter Matches" "$result")"; [[ -z "$c_param" ]] && c_param=0

  if [[ "$c_sym" == 0 && "$c_sem" == 0 && "$c_param" == 0 ]]; then
    return 1
  fi

  local combined="$result"
  if [[ -n "$FILE_CONTEXT" ]]; then
    combined="$result

---- FILE CONTEXT (impact) ----
$FILE_CONTEXT"
  fi

  emit_hook_output "find $symbol" "$combined"
  exit 0
}

augment_file() {
  local file="$1"
  [[ ! -f "$file" ]] && return 1

  local result
  result="$(run_loct slice "$file")"
  [[ -z "$result" ]] && return 1

  emit_hook_output "slice $file" "$result"
  exit 0
}

augment_impact_context() {
  local file="$1"
  [[ ! -f "$file" ]] && return 1
  run_loct impact "$file"
}

augment_who_imports() {
  local file="$1"
  [[ ! -f "$file" ]] && return 1

  local result
  result="$(run_loct query who-imports "$file")"
  [[ -z "$result" ]] && return 1

  emit_hook_output "who-imports $file" "$result"
  exit 0
}

augment_directory() {
  local dir="$1"
  [[ ! -d "$dir" ]] && return 1

  local result
  result="$(run_loct focus "$dir")"
  [[ -z "$result" ]] && return 1

  emit_hook_output "focus $dir" "$result"
  exit 0
}

augment_tauri_command() {
  local cmd="$1"

  local all
  all="$(run_loct commands)"
  [[ -z "$all" ]] && return 1

  local result
  result="$(printf '%s\n' "$all" | grep -i "$cmd" || true)"
  [[ -z "$result" ]] && return 1

  emit_hook_output "commands matching $cmd" "$result"
  exit 0
}

augment_health() {
  local result
  result="$(run_loct health)"
  [[ -z "$result" ]] && return 1

  emit_hook_output "health" "$result"
  exit 0
}

# ---------------------------------------------------------------------------
# Optional: if search targeted a specific file, add impact as context
# ---------------------------------------------------------------------------
if [[ "$PATH_ARG" != "." && -f "$PATH_ARG" ]]; then
  FILE_CONTEXT="$(augment_impact_context "$PATH_ARG" 2>/dev/null || true)"
fi

# ---------------------------------------------------------------------------
# Smart routing
# ---------------------------------------------------------------------------

# 1) Exact file path as PATTERN -> slice
if [[ -f "$PATTERN" ]]; then
  augment_file "$PATTERN"
fi

# 2) Directory path as PATTERN -> focus
if [[ -d "$PATTERN" || "$PATTERN" == */ ]]; then
  augment_directory "${PATTERN%/}"
fi

# 3) Tauri snake_case commands - DISABLED in v14
# Snake_case patterns now go through loct find (step 6) for better results.
# This was causing previous_response_id to hit loct commands instead of loct find.
# Use `loct commands` directly if you need Tauri command bridge analysis.
# if printf '%s' "$PATTERN" | grep -qE '^[a-z][a-z0-9]*(_[a-z0-9]+)+$'; then
#   augment_tauri_command "$PATTERN"
# fi

# 4) File-like pattern (extension) -> try to resolve and slice
if printf '%s' "$PATTERN" | grep -qE '\.(ts|tsx|rs|js|jsx|py|vue|svelte|css|scss)$'; then
  FOUND="$(find . -path "./.git" -prune -o -name "$PATTERN" -type f -print 2>/dev/null | head -1)"
  [[ -n "$FOUND" ]] && augment_file "$FOUND"
fi

# 5) Multi-term symbol search with | alternation or .* wildcards
# E.g., "ApiClient|send_request", "user_model.*fetch|fetch.*endpoint"
# Transform: split on | and .* to extract individual terms
if printf '%s' "$PATTERN" | grep -qE '\||\.\*|\.\+' && printf '%s' "$PATTERN" | grep -qE '[A-Z_]'; then
  # Transform: replace .* and .+ with |, then clean up regex noise
  # canonical_model.*vision|vision.*endpoint → canonical_model|vision|vision|endpoint
  CLEAN_PATTERN="$(printf '%s' "$PATTERN" | sed 's/\.\*/|/g; s/\.\+/|/g; s/[\^$]//g; s/\\//g; s/||*/|/g; s/^|//; s/|$//')"
  # Extract unique valid symbol terms (alphanumeric with _ or camelCase)
  UNIQUE_TERMS="$(printf '%s' "$CLEAN_PATTERN" | tr '|' '\n' | grep -E '^[a-zA-Z_][a-zA-Z0-9_]*$' | sort -u | tr '\n' '|' | sed 's/|$//')"
  TERM_COUNT="$(printf '%s' "$UNIQUE_TERMS" | tr '|' '\n' | grep -c '.')"
  if [[ "$TERM_COUNT" -ge 2 ]] && [[ -n "$UNIQUE_TERMS" ]]; then
    augment_symbol "$UNIQUE_TERMS"
  fi
fi

# 6) Symbol-ish patterns -> loct find
if printf '%s' "$PATTERN" | grep -qE '^[A-Z][a-zA-Z0-9]{2,}$'; then augment_symbol "$PATTERN"; fi
if printf '%s' "$PATTERN" | grep -qE '^[a-z]+[A-Z][a-zA-Z0-9]*$'; then augment_symbol "$PATTERN"; fi
if printf '%s' "$PATTERN" | grep -qE '^use[A-Z][a-zA-Z0-9]+$'; then augment_symbol "$PATTERN"; fi
if printf '%s' "$PATTERN" | grep -qE '^(handle|on)[A-Z][a-zA-Z0-9]+$'; then augment_symbol "$PATTERN"; fi
if printf '%s' "$PATTERN" | grep -qE '^[a-z][a-z0-9]*_[a-z_0-9]+$'; then augment_symbol "$PATTERN"; fi
if printf '%s' "$PATTERN" | grep -qE '^(is|has|can|should|will)[A-Z][a-zA-Z0-9]*$'; then augment_symbol "$PATTERN"; fi
if printf '%s' "$PATTERN" | grep -qE '^[A-Z][A-Z0-9_]+$'; then augment_symbol "$PATTERN"; fi

# 7) Path-like patterns -> try to resolve
if [[ "$PATTERN" == *"/"* ]]; then
  FOUND="$(find . -path "./.git" -prune -o -path "*$PATTERN*" -type f -print 2>/dev/null | head -1)"
  [[ -n "$FOUND" ]] && augment_file "$FOUND"

  FOUND_DIR="$(find . -path "./.git" -prune -o -path "*$PATTERN*" -type d -print 2>/dev/null | head -1)"
  [[ -n "$FOUND_DIR" ]] && augment_directory "$FOUND_DIR"
fi

# 8) Health-related keywords -> health
if printf '%s' "$PATTERN" | grep -qiE 'dead|unused|orphan|stale|deprecated|circular|cycle|duplicate|twin'; then
  augment_health
fi

# 9) FALLBACK: No pattern matched above - try loct find anyway
# This catches cases like:
#   - "assistive" (plain lowercase word)
#   - "Config::load" (:: not handled by multi-term)
#   - "assert result[0].isupper()" (complex expression → extract words)
#
# Strategy: extract alphanumeric words (3+ chars), join with |
CLEAN_SYM="$(printf '%s' "$PATTERN" | \
  sed 's/\\//g' | \
  tr -cs 'a-zA-Z0-9_' ' ' | \
  tr ' ' '\n' | \
  grep -E '^[a-zA-Z_][a-zA-Z0-9_]{2,}$' | \
  head -5 | \
  tr '\n' '|' | \
  sed 's/|$//')"
# Try find with cleaned pattern (ignore failure)
[[ -n "$CLEAN_SYM" ]] && augment_symbol "$CLEAN_SYM" 2>/dev/null || true

# Otherwise: no augmentation
exit 0
