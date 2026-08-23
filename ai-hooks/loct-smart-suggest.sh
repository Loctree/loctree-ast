#!/bin/bash
export PATH="$HOME/.cargo/bin:/opt/homebrew/bin:/usr/local/bin:/usr/bin:/bin:$PATH"

# Quick exit if jq unavailable (has grep fallback but jq preferred)
command -v jq >/dev/null 2>&1 || exit 0

# ============================================================================
# loct-smart-suggest.sh - Context-aware loct suggestions for AI agents
# ============================================================================
#
# ALTERNATIVE APPROACH: PreToolUse hook that SUGGESTS loct commands
# instead of automatically augmenting. Use this if you prefer manual control.
#
# Non-blocking! Adds helpful hints when loct would be better.
#
# INSTALLATION:
#   1. Copy to ~/.claude/hooks/loct-smart-suggest.sh
#   2. chmod +x ~/.claude/hooks/loct-smart-suggest.sh
#   3. Add to ~/.claude/settings.json under PreToolUse (not PostToolUse)
#
# ============================================================================

INPUT=$(cat)
OUTPUT_MODE="${LOCT_SMART_SUGGEST_OUTPUT:-stderr}"

# Prints one suggestion as JSON or as a boxed stderr banner, then increments the
# on-disk per-day counter that caps this hook at 3 hints.
emit_hint() {
    local hint="$1"
    local cmd="$2"
    local message="$hint"

    if [[ -n "$cmd" ]]; then
        message="$message Try \`$cmd\` next time."
    fi

    if [[ "$OUTPUT_MODE" == "json" ]]; then
        jq -cn --arg msg "$message" '{"systemMessage": $msg}'
    else
        echo "" >&2
        echo "┌─────────────────────────────────────────────────────────────" >&2
        echo "│ 🌳 $hint" >&2
        [[ -n "$cmd" ]] && echo "│ → $cmd" >&2
        echo "└─────────────────────────────────────────────────────────────" >&2
        echo "" >&2
    fi

    echo $((SUGGEST_COUNT + 1)) > "$SUGGEST_COUNT_FILE"
}

# True when the Bash command runs rg/grep/find directly, through sh -c, or behind
# an env VAR=... prefix. Decides whether the command is worth suggesting against.
is_text_search_command() {
    local command="$1"
    [[ "$command" =~ (^|[[:space:]\;\|\&])(rg|grep|find)([[:space:]]|$) ]] && return 0
    [[ "$command" =~ (^|[[:space:]])(bash|zsh|sh)[[:space:]]+-(l)?c[[:space:]]+.*(rg|grep|find)([[:space:]]|$) ]] && return 0
    [[ "$command" =~ (^|[[:space:]])env([[:space:]][A-Za-z_][A-Za-z0-9_]*=.*)*[[:space:]]+(rg|grep|find)([[:space:]]|$) ]] && return 0
    return 1
}

# Single-quotes a value so the suggested command can be pasted verbatim.
shell_quote() {
    local value="$1"
    printf "'%s'" "$(printf "%s" "$value" | sed "s/'/'\\\\''/g")"
}

# Renders the `loct find --literal` line to suggest, falling back to a
# placeholder pattern when none could be parsed out of the command.
loct_literal_command() {
    local pattern="$1"
    if [[ -n "$pattern" ]]; then
        printf "loct find %s --literal --group-by-file --count-only" "$(shell_quote "$pattern")"
    else
        printf "loct find '<pattern>' --literal --group-by-file --count-only"
    fi
}

# Renders the `loct tree --files --match` line to suggest for find-style
# file discovery, with a placeholder regex when no pattern was parsed.
loct_tree_command() {
    local pattern="$1"
    if [[ -n "$pattern" ]]; then
        printf "loct tree --files --match %s" "$(shell_quote "$pattern")"
    else
        printf "loct tree --files --match '<regex>'"
    fi
}

# Delegates to an embedded python parser and prints "<tool>\t<pattern>" for the
# first rg/grep/find in the command. Returns 1 when python3 is unavailable.
extract_search_intent() {
    local command="$1"

    command -v python3 >/dev/null 2>&1 || return 1

    COMMAND_TO_PARSE="$command" python3 - <<'PY'
import os
import re
import shlex
import sys

VALUE_OPTIONS = {
    "-A", "-B", "-C", "-e", "-f", "-g", "-m", "-t", "-T",
    "--after-context", "--before-context", "--context", "--glob", "--iglob",
    "--max-count", "--regexp", "--file", "--type", "--type-not",
    "--type-add", "--type-clear",
}


def split(command):
    """Tokenize a command, degrading to whitespace split on unbalanced quotes."""
    try:
        return shlex.split(command)
    except ValueError:
        return []


def strip_env(tokens):
    """Drop a leading `env` and its VAR=VALUE assignments from the token list."""
    if tokens and tokens[0] == "env":
        tokens = tokens[1:]
        while tokens and "=" in tokens[0] and not tokens[0].startswith("-"):
            tokens = tokens[1:]
    return tokens


def unwrap_shell(tokens):
    """Re-tokenize the inner script of a `sh -c '...'` wrapper, else pass through."""
    if not tokens:
        return tokens
    head = os.path.basename(tokens[0])
    if head not in {"bash", "zsh", "sh"}:
        return tokens
    for index, token in enumerate(tokens[1:], start=1):
        if token in {"-c", "-lc"} and index + 1 < len(tokens):
            return split(tokens[index + 1])
    return tokens


def option_takes_value(token):
    """True when the flag consumes the next token, so it is not the pattern."""
    return token in VALUE_OPTIONS or token.startswith("--glob=") or token.startswith("--iglob=")


def extract_rg_or_grep(tool, args):
    """Return the search pattern, honouring -e/--regexp/-- and value-taking flags."""
    index = 0
    while index < len(args):
        token = args[index]
        if token == "--":
            return args[index + 1] if index + 1 < len(args) else ""
        if token in {"-e", "--regexp"} and index + 1 < len(args):
            return args[index + 1]
        if token.startswith("--regexp="):
            return token.split("=", 1)[1]
        if token.startswith("-") and token != "-":
            if option_takes_value(token) and "=" not in token:
                index += 2
            else:
                index += 1
            continue
        return token
    return ""


def glob_to_regex(pattern):
    """Convert a find(1) glob into the regex `loct tree --match` expects."""
    if not any(char in pattern for char in "*?[]"):
        return pattern
    converted = []
    for char in pattern:
        if char == "*":
            converted.append(".*")
        elif char == "?":
            converted.append(".")
        else:
            converted.append(re.escape(char))
    return "".join(converted)


def extract_find(args):
    """Return the -name/-path/-regex argument of a find command as a regex."""
    for option in ("-name", "-iname", "-path", "-ipath", "-regex"):
        if option in args:
            index = args.index(option)
            if index + 1 < len(args):
                pattern = args[index + 1]
                if option == "-regex":
                    return pattern
                return glob_to_regex(pattern)
    return ""


def parse(command):
    """Return (tool, pattern) for the first rg/grep/find found, else ("", "")."""
    tokens = unwrap_shell(strip_env(split(command)))
    tokens = strip_env(tokens)
    for index, token in enumerate(tokens):
        tool = os.path.basename(token)
        if tool in {"rg", "grep"}:
            return tool, extract_rg_or_grep(tool, tokens[index + 1:])
        if tool == "find":
            return tool, extract_find(tokens[index + 1:])
    return "", ""


kind, pattern = parse(os.environ.get("COMMAND_TO_PARSE", ""))
if kind:
    print(f"{kind}\t{pattern}")
PY
}

# Track suggestions to avoid spam (max 3 per session)
SUGGEST_COUNT_FILE="${LOCT_SUGGEST_COUNT_FILE:-/tmp/.loct-suggest-count-$(date +%Y%m%d)}"
SUGGEST_COUNT=$(cat "$SUGGEST_COUNT_FILE" 2>/dev/null || echo "0")
[[ "$SUGGEST_COUNT" -ge 3 ]] && exit 0

TOOL_NAME=$(echo "$INPUT" | jq -r '.tool_name // empty' 2>/dev/null)
COMMAND=$(echo "$INPUT" | jq -r '.tool_input.command // empty' 2>/dev/null)

if [[ "$TOOL_NAME" == "Grep" ]]; then
    GREP_PATTERN=$(echo "$INPUT" | jq -r '.tool_input.pattern // empty' 2>/dev/null)
    SHOWN="Grep"
    [[ -n "$GREP_PATTERN" ]] && SHOWN="$SHOWN $GREP_PATTERN"
    emit_hint "I see you used \`$SHOWN\`. Loctree literal is now a quiet indexed lookup with curated context; use raw grep for raw filesystem truth." "$(loct_literal_command "$GREP_PATTERN")"
    exit 0
fi

if [[ -n "$COMMAND" ]] && is_text_search_command "$COMMAND"; then
    SEARCH_INTENT=$(extract_search_intent "$COMMAND" 2>/dev/null || true)
    SEARCH_KIND="${SEARCH_INTENT%%$'\t'*}"
    SEARCH_PATTERN=""
    if [[ "$SEARCH_INTENT" == *$'\t'* ]]; then
        SEARCH_PATTERN="${SEARCH_INTENT#*$'\t'}"
    fi

    if [[ "$SEARCH_KIND" == "find" ]]; then
        emit_hint "I see you used \`$COMMAND\`. Loctree tree can answer indexed file-list discovery without rescanning noise; use shell find for raw filesystem truth." "$(loct_tree_command "$SEARCH_PATTERN")"
    else
        emit_hint "I see you used \`$COMMAND\`. Loctree literal is now a quiet indexed lookup with curated context; use raw text search for raw filesystem truth." "$(loct_literal_command "$SEARCH_PATTERN")"
    fi
    exit 0
fi

# Extract pattern from JSON
PATTERN=$(echo "$INPUT" | jq -r '.pattern // .tool_input.pattern // empty' 2>/dev/null)
if [[ -z "$PATTERN" ]]; then
    PATTERN=$(echo "$INPUT" | grep -oP '"pattern"\s*:\s*"\K[^"]+' 2>/dev/null || echo "")
fi

[[ -z "$PATTERN" ]] && exit 0

# Thin alias over emit_hint used by the pattern-shape rules below.
suggest() {
    local hint="$1"
    local cmd="$2"
    emit_hint "$hint" "$cmd"
}

# Case 1: React Component or Type (PascalCase)
if [[ "$PATTERN" =~ ^[A-Z][a-zA-Z0-9]{2,}$ ]]; then
    suggest "Symbol search? loct finds definition + all usages" \
            "loct find $(shell_quote "$PATTERN")"
    exit 0
fi

# Case 2: React Hook (useXxx)
if [[ "$PATTERN" =~ ^use[A-Z][a-zA-Z0-9]+$ ]]; then
    suggest "Hook search? loct shows definition + import chain" \
            "loct find $(shell_quote "$PATTERN")"
    exit 0
fi

# Case 3: Event Handler (handleXxx, onXxx)
if [[ "$PATTERN" =~ ^(handle|on)[A-Z][a-zA-Z0-9]+$ ]]; then
    suggest "Handler search? loct finds definition + prop passing" \
            "loct find $(shell_quote "$PATTERN")"
    exit 0
fi

# Case 4: Tauri command patterns
if [[ "$PATTERN" =~ invoke|safeInvoke|emit\( ]]; then
    suggest "Tauri bridge? loct trace shows FE↔BE coverage" \
            "loct trace <handler_name>"
    exit 0
fi

# Case 5: Import/export analysis
if [[ "$PATTERN" =~ ^import|^export|from.+import ]]; then
    suggest "Import analysis? loct has full dependency graph" \
            "loct q who-imports <file>"
    exit 0
fi

# Case 6: Snake_case symbol (Rust/Python)
if [[ "$PATTERN" =~ ^[a-z][a-z0-9]*_[a-z_0-9]+$ ]]; then
    suggest "Symbol search? loct finds across TS+Rust with context" \
            "loct find $(shell_quote "$PATTERN")"
    exit 0
fi

# Case 7: Checking if something exists/is used
if [[ "$PATTERN" =~ ^(is|has|can|should)[A-Z] ]]; then
    suggest "Checking usage? loct can tell if it's dead code" \
            "loct find $(shell_quote "$PATTERN")"
    exit 0
fi

# Case 8: Dead/unused patterns
if [[ "$PATTERN" =~ dead|unused|orphan|stale ]]; then
    suggest "Dead code hunt? loct has pre-indexed findings" \
            "loct health"
    exit 0
fi

# Case 9: Circular/cycle patterns
if [[ "$PATTERN" =~ circular|cycle|loop|recursive ]]; then
    suggest "Cycle detection? loct has SCC analysis ready" \
            "loct health"
    exit 0
fi

# Case 10: Duplicate/twin patterns
if [[ "$PATTERN" =~ duplicate|twin|copy|similar ]]; then
    suggest "Finding duplicates? loct detected exact twins" \
            "loct health"
    exit 0
fi

# No match - grep is fine
exit 0
