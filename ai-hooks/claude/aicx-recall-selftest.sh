#!/usr/bin/env bash
# ============================================================================
# aicx-recall-selftest.sh — prove the post-compact recall DIGEST works
# ============================================================================
# WHY THIS EXISTS: the recall hook has failed in ways green-looking checks hid —
# (1) a path mismatch made it silently emit {} (dead recall, no signal); (2) an
# `echo "$out" | jq` harness mangled escapes and reported a WORKING hook as
# broken (4 wasted "fix" cycles on a non-bug); and (3) the hook appended a full
# 15 KB verbatim chunk that the harness then TRUNCATED to a 2 KB preamble — the
# agent woke up with "a file is over there", the exact failure recall must prevent.
#
# This self-test runs the REAL hook and asserts the CURRENT contract:
#   - plain stdout (NOT JSON — the hook left the JSON contract on 2026-06-12),
#   - content delivered DIRECTLY (the latest ask is in the output AND in the
#     first 2 KB preview window), not hidden behind a file pointer,
#   - bounded size so the harness never truncates the gold,
#   - degraded paths stay LOUD, never silent.
# It ALWAYS reads hook output from a FILE (never `echo "$var"`), so the artifact
# is never corrupted by the measurement. Run it after ANY edit to the hook.
#
# USAGE:  bash ~/.claude/hooks/aicx-recall-selftest.sh
# EXIT:   0 = all assertions pass · 1 = a failure (printed)
# ============================================================================
set -uo pipefail
export PATH="$HOME/.cargo/bin:/opt/homebrew/bin:/usr/local/bin:/usr/bin:/bin:$PATH"

HOOK="$HOME/.claude/hooks/aicx-postcompact.sh"
EXTRACT_DIR="$HOME/.aicx/extracts/claude"
# Harness inline-truncation guard: the old 15.6 KB output got cut to a 2 KB
# preview. We keep the whole digest comfortably under this and front-load the
# gold so even a 2 KB preview carries the [P0] ask.
MAX_BYTES="${AICX_RECALL_MAX_BYTES:-12000}"
PREVIEW_BYTES=2048
TMP=$(mktemp -d)
trap 'rm -rf "$TMP"' EXIT
fail=0

# pass NAME : marks one assertion satisfied; purely cosmetic, never touches $fail.
pass() { printf '  \033[32m✅\033[0m %s\n' "$1"; }
# nope NAME : marks one assertion broken AND raises $fail, which becomes the exit code.
nope() { printf '  \033[31m❌\033[0m %s\n' "$1"; fail=1; }
# check NAME : runs a predicate command; reads ONLY from files, never echo "$var".
check() { if eval "$2" >/dev/null 2>&1; then pass "$1"; else nope "$1"; fi; }

bash -n "$HOOK" || { echo "❌ hook has a syntax error"; exit 1; }
echo "syntax OK"
command -v shellcheck >/dev/null 2>&1 && { shellcheck -S warning "$HOOK" && echo "shellcheck OK" || nope "shellcheck warnings"; }

# ── [1] Happy path: a real extract (real NUL/bidi/control bytes exercise sanitize) ──
echo "[1] happy path (real extract) — content delivered DIRECTLY, not as a pointer"
sid=$(ls "$EXTRACT_DIR"/*_conversation.md 2>/dev/null | head -1 | xargs -r basename 2>/dev/null | sed 's/_conversation\.md$//')
if [ -z "$sid" ]; then
  printf '  \033[33m∅\033[0m no extract present — skipping happy path (run after a real compact)\n'
else
  printf '{"session_id":"%s"}' "$sid" | bash "$HOOK" 2>/dev/null >"$TMP/out.txt"
  head -c "$PREVIEW_BYTES" "$TMP/out.txt" >"$TMP/preview.txt"
  # split content from the appendix so we can prove real content precedes the pointers
  sed '/▓▓▓ APPENDIX/,$d' "$TMP/out.txt" >"$TMP/body.txt"

  check "output non-empty"                  "[ -s '$TMP/out.txt' ]"
  check "carries the LOUD wiped-memory header" "grep -q 'AICX RECALL' '$TMP/out.txt'"
  check "[P0] LATEST ASK section present"    "grep -q '\\[P0\\] LATEST ASK' '$TMP/out.txt'"
  check "STATE handoff section present"      "grep -q '^▓▓▓ STATE' '$TMP/out.txt'"
  check "RECENT TURNS breadcrumb present"    "grep -q 'RECENT TURNS' '$TMP/out.txt'"
  check "APPENDIX pointers present"          "grep -q 'APPENDIX' '$TMP/out.txt'"
  # THE regression test for the operator's complaint: the gold is in the preview window,
  # NOT hidden behind a 'go read the file' pointer.
  check "GOLD in first 2KB preview (truncation-safe)" "grep -q '\\[P0\\] LATEST ASK' '$TMP/preview.txt'"
  # Real recovered content (>1.2KB) must exist BEFORE the appendix pointers.
  check "substantive content precedes the pointers (>1200B)" "[ \$(wc -c <'$TMP/body.txt') -gt 1200 ]"
  # Pointers are an appendix, not the substitute: the transcript path appears AFTER real content.
  check "transcript pointer is in the APPENDIX, not the lead" "[ \$(grep -n 'Full raw transcript' '$TMP/out.txt' | head -1 | cut -d: -f1) -gt 15 ]"
  check "bounded under harness truncation (<${MAX_BYTES}B)" "[ \$(wc -c <'$TMP/out.txt') -lt $MAX_BYTES ]"
  printf '  \033[36mℹ\033[0m  digest size: %s bytes\n' "$(wc -c <"$TMP/out.txt" | tr -d ' ')"
fi

# ── [2] Degraded path: missing extract MUST be a LOUD fallback, never silent ──
echo "[2] degraded path (missing extract → loud fallback, never silent)"
printf '{"session_id":"selftest-nonexistent-%s"}' "$$" | bash "$HOOK" 2>/dev/null >"$TMP/fb.txt"
check "fallback is non-empty (NOT silent)" "[ -s '$TMP/fb.txt' ]"
check "announces DEGRADED loudly"          "grep -q 'DEGRADED' '$TMP/fb.txt'"
check "still names what to read by hand"   "grep -q 'transcript' '$TMP/fb.txt'"

# ── [3] Empty session_id stays a benign no-op (empty stdout, exit 0) ──
echo "[3] empty session_id → benign no-op"
printf '{}' | bash "$HOOK" 2>/dev/null >"$TMP/empty.txt"
check "no-op produces no stdout" "[ ! -s '$TMP/empty.txt' ]"

echo
if [ "$fail" = 0 ]; then printf '\033[32mALL PASS — recall digest delivers content directly\033[0m\n'; else printf '\033[31mFAILURES — recall mechanism NOT verified\033[0m\n'; fi
exit "$fail"
