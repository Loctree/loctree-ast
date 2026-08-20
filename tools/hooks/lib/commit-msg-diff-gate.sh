#!/usr/bin/env bash
# commit-msg-diff-gate — message claims vs staged diff
#
# Catches the localized fleet failure mode: commit prose written from
# *intent* (or a prior plan) that falsifies the staged tree.
#
# Checks:
#   [1] type-vs-content  — test|docs|chore (+ style?) with production source staged
#   [2] file-count claim — "N-file pack" / "only N files" vs staged count
#   [3] no-touch claim   — "No touches to X" / "without touching X" vs staged paths
#
# Mode:
#   default            advisory (print findings, exit 0)
#   LOCTREE_COMMIT_GATE_STRICT=1  blocking (exit 1 when findings)
#
# Fleet subject forms supported:
#   [agent/workflow] type(scope): subject
#   type(scope): subject
#
# Known blind spot (documented, not claimed closed):
#   Purely semantic lies ("No dupe") with no path/count claim are NOT
#   caught — that needs structural tests (see pack Hotspots regression),
#   not a message heuristic.
#
# Usage:
#   commit-msg-diff-gate.sh <msg-file> [staged-list-file]
#   staged-list-file (optional): one path per line; if omitted, uses
#   `git diff --cached --name-only --diff-filter=ACMRD`.
#
# 𝚅𝚒𝚋𝚎𝚌𝚛𝚊𝚏𝚝𝚎𝚍. with AI Agents by Vetcoders (c)2024-2026 LibraxisAI

set -euo pipefail

msg_file="${1:-}"
staged_list_file="${2:-}"

if [ -z "$msg_file" ] || [ ! -f "$msg_file" ]; then
    printf 'commit-msg-diff-gate: expected path to commit message file\n' >&2
    exit 2
fi

strict="${LOCTREE_COMMIT_GATE_STRICT:-0}"
findings=()

first_line="$(head -n1 "$msg_file" | tr -d '\r')"

# Skip git-generated subjects entirely.
case "$first_line" in
    "Merge "*|"Revert "*|"fixup!"*|"squash!"*|"amend!"*) exit 0 ;;
esac

# ---------------------------------------------------------------------------
# Staged paths
# ---------------------------------------------------------------------------
staged_paths=()
if [ -n "$staged_list_file" ]; then
    if [ ! -f "$staged_list_file" ]; then
        printf 'commit-msg-diff-gate: staged list not found: %s\n' "$staged_list_file" >&2
        exit 2
    fi
    while IFS= read -r p || [ -n "$p" ]; do
        p="${p%$'\r'}"
        [ -z "$p" ] && continue
        staged_paths+=("$p")
    done <"$staged_list_file"
else
    # During commit-msg, the index still holds the staged set.
    while IFS= read -r p || [ -n "$p" ]; do
        [ -z "$p" ] && continue
        staged_paths+=("$p")
    done < <(git diff --cached --name-only --diff-filter=ACMRD 2>/dev/null || true)
fi

staged_count="${#staged_paths[@]}"

# ---------------------------------------------------------------------------
# Type extraction (fleet + plain conventional)
# ---------------------------------------------------------------------------
# Strip optional [agent/runtime] prefix, then take type before ( or :
subject_rest="$first_line"
if [[ "$subject_rest" =~ ^\[[^][]+\][[:space:]]+(.+)$ ]]; then
    subject_rest="${BASH_REMATCH[1]}"
fi

commit_type=""
if [[ "$subject_rest" =~ ^(feat|fix|docs|style|refactor|perf|test|build|ci|chore|revert|release)(\(|:|[[:space:]]|$) ]]; then
    commit_type="${BASH_REMATCH[1]}"
fi

# ---------------------------------------------------------------------------
# [1] type-vs-content
# ---------------------------------------------------------------------------
is_prod_source() {
    local p="$1"
    # Explicit non-product / non-source surfaces
    case "$p" in
        *.md|*.txt|*.rst|*.lock|*.svg|*.png|*.jpg|*.jpeg|*.gif|*.webp|*.ico|*.pdf)
            return 1
            ;;
        LICENSE|LICENSE.*|COPYING|NOTICE|CHANGELOG*|CONTRIBUTING*|README*|Makefile|Cargo.toml|package.json|*.toml|*.yml|*.yaml|*.json|*.jsonc)
            return 1
            ;;
        .github/*|docs/*|commercial/*|legal/*|public_dist/*|licenses/*|assets/*|*.html|*.css|*.wasm)
            return 1
            ;;
    esac
    # Test / fixture trees
    case "$p" in
        tests/*|*/tests/*|*/test/*|*_test.*|*.test.*|*.spec.*|*/fixtures/*|*/__tests__/*|*/e2e/*)
            return 1
            ;;
    esac
    # Production-ish source extensions
    case "$p" in
        *.rs|*.kt|*.kts|*.java|*.ts|*.tsx|*.js|*.jsx|*.mjs|*.cjs|*.py|*.go|*.swift|*.c|*.cc|*.cpp|*.h|*.hpp|*.m|*.mm|*.sh|*.bash|*.zsh)
            return 0
            ;;
    esac
    # src/ trees even with uncommon extensions
    case "$p" in
        */src/*|tools/hooks/*)
            return 0
            ;;
    esac
    return 1
}

prod_staged=()
for p in "${staged_paths[@]+"${staged_paths[@]}"}"; do
    if is_prod_source "$p"; then
        prod_staged+=("$p")
    fi
done

case "$commit_type" in
    test|docs|chore)
        if [ "${#prod_staged[@]}" -gt 0 ]; then
            sample="$(printf '%s, ' "${prod_staged[@]:0:5}" | sed 's/, $//')"
            findings+=("[1] type-vs-content: subject type '${commit_type}' but production source is staged (${#prod_staged[@]}): ${sample}")
        fi
        ;;
esac

# ---------------------------------------------------------------------------
# [2] file-count claims
# ---------------------------------------------------------------------------
body="$(tr -d '\r' <"$msg_file")"
# Collect claimed counts from common phrasings.
claimed_counts=()
while IFS= read -r claim; do
    [ -z "$claim" ] && continue
    claimed_counts+=("$claim")
done < <(
    printf '%s\n' "$body" | grep -Eio \
        '([0-9]+)[- ]file(s)?[[:space:]]+pack|only[[:space:]]+([0-9]+)[[:space:]]+files?|([0-9]+)[[:space:]]+file(s)?[[:space:]]+only|\(([0-9]+)[[:space:]]+files?\)|files:[[:space:]]*([0-9]+)|([0-9]+)[[:space:]]+file(s)?[[:space:]]+changed' \
        || true
)

# Extract the first integer from each matched phrase and compare.
for phrase in "${claimed_counts[@]+"${claimed_counts[@]}"}"; do
    n="$(printf '%s' "$phrase" | grep -Eo '[0-9]+' | head -n1 || true)"
    [ -z "$n" ] && continue
    if [ "$n" -ne "$staged_count" ]; then
        findings+=("[2] file-count claim: message says '${phrase}' but staged count is ${staged_count}")
    fi
done

# ---------------------------------------------------------------------------
# [3] no-touch claims
# ---------------------------------------------------------------------------
# Lines like:
#   No touches to core (...overlay...)
#   without touching pack.rs
#   does not touch loctree-rs/src/overlay.rs
no_touch_lines=()
while IFS= read -r line || [ -n "$line" ]; do
    line="${line%$'\r'}"
    if printf '%s\n' "$line" | grep -Eiq \
        'no touches? to |without touching |does not touch |didn.?t touch |no touch of '; then
        no_touch_lines+=("$line")
    fi
done <"$msg_file"

path_mentioned_in_line() {
    local line="$1"
    local staged="$2"
    local base
    base="$(basename "$staged")"
    # Match full path, basename, or a path segment that looks intentional.
    if printf '%s\n' "$line" | grep -Fiq -- "$staged"; then
        return 0
    fi
    if [ -n "$base" ] && printf '%s\n' "$line" | grep -Fiq -- "$base"; then
        return 0
    fi
    # Parent dir name when distinctive (e.g. "overlay" for .../overlay.rs)
    local parent
    parent="$(basename "$(dirname "$staged")")"
    if [ -n "$parent" ] && [ "$parent" != "." ] && [ "$parent" != "/" ] \
        && printf '%s\n' "$line" | grep -Eiq "\\b${parent}\\b"; then
        # Avoid ultra-generic parents
        case "$parent" in
            src|lib|bin|main|test|tests|kotlin|java|rs) return 1 ;;
        esac
        return 0
    fi
    return 1
}

for line in "${no_touch_lines[@]+"${no_touch_lines[@]}"}"; do
    for p in "${staged_paths[@]+"${staged_paths[@]}"}"; do
        if path_mentioned_in_line "$line" "$p"; then
            findings+=("[3] no-touch claim: message says «${line}» but staged path matches: ${p}")
        fi
    done
done

# ---------------------------------------------------------------------------
# Report
# ---------------------------------------------------------------------------
if [ "${#findings[@]}" -eq 0 ]; then
    exit 0
fi

printf 'commit-msg-diff-gate: message claims disagree with staged diff\n' >&2
for f in "${findings[@]}"; do
    printf '  - %s\n' "$f" >&2
done
printf '\n' >&2
printf '  Staged (%s):\n' "$staged_count" >&2
for p in "${staged_paths[@]+"${staged_paths[@]}"}"; do
    printf '    %s\n' "$p" >&2
done
printf '\n' >&2
printf '%s\n' '  Reconcile the subject/body with git diff --cached --stat, or set' >&2
printf '  LOCTREE_COMMIT_GATE_STRICT=0 (default) is advisory; STRICT=1 blocks.\n' >&2
printf '  Blind spot: pure semantic claims ("No dupe") need code tests, not this gate.\n' >&2

if [ "$strict" = "1" ]; then
    exit 1
fi
exit 0
