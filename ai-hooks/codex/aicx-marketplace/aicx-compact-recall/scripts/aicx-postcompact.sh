#!/usr/bin/env bash
# ============================================================================
# aicx-postcompact.sh — inject a PRIORITIZED recall DIGEST after compaction
# ============================================================================
# TRIGGER: SessionStart hook, matcher "compact" (fires only after manual/auto
#          compact). Codex PostCompact ignores plain stdout and systemMessage is
#          UI-only; SessionStart(compact) is the supported model-context path.
#
# PURPOSE: After compaction the model loses its turn-by-turn memory. This hook
#          hands the just-compacted model a PRIORITIZED, SELF-CONTAINED digest
#          of where it was — DIRECTLY, as content, not as "go read a file".
#
# WHY THE REWRITE (2026-06-16): the previous version appended the whole 400-line
#          most-recent chunk VERBATIM. That blew the output to ~15 KB, which the
#          harness then TRUNCATED to a 2 KB preview + an on-disk pointer — and the
#          surviving 2 KB was the manifest PREAMBLE, not the recovered context.
#          Net effect: the agent woke up with "a file is over there" — exactly the
#          failure this hook exists to prevent. The fix is a BUDGETED digest whose
#          GOLD (the latest ask) lives in the first ~1.2 KB so it survives any
#          2 KB-preview truncation, with the whole thing kept small enough to not
#          truncate at all. File pointers are an APPENDIX, never the substitute.
#
# OUTPUT contract (priority order — the operator's spec):
#   1. LOUD header (memory wiped — read this, do not guess)
#   2. [P0] LATEST ASK        — the last user turn, verbatim (the current goal)
#   3. STATE / LAST HANDOFF   — the last assistant turn (commits, done, next)
#   4. FILES & REFS TOUCHED   — paths + commit SHAs from the recent window
#   5. VERBATIM RECENT TAIL   — the last few turns, for immediate continuity
#   6. APPENDIX               — chunk + transcript paths (deep dive, optional)
#   Chunks still live at /tmp/aicx-recall/<session_id>/chunk-NNN (chronological).
#
# BUDGET: ~250ms (awk/sed/grep + split). REQUIRES: jq, awk, sed, grep, split, wc.
# SELF-TEST: bash ~/.claude/hooks/aicx-recall-selftest.sh  (run after ANY edit).
# ============================================================================

set -uo pipefail
export PATH="$HOME/.cargo/bin:/opt/homebrew/bin:/usr/local/bin:/usr/bin:/bin:$PATH"
umask 077

CHUNK_SIZE="${AICX_RECALL_CHUNK_LINES:-400}"
# Set AICX_RECALL_STRIP_SKILLS=0 to keep full skill bodies inline.
STRIP_SKILLS="${AICX_RECALL_STRIP_SKILLS:-1}"
# Set AICX_RECALL_DEDUP=0 to skip logical-turn block dedup.
DEDUP="${AICX_RECALL_DEDUP:-1}"
# Digest section caps (lines). Tuned so the GOLD (ask) is preview-safe (<~1.2 KB)
# and the whole digest stays well under the harness inline-truncation threshold.
ASK_LINES="${AICX_RECALL_ASK_LINES:-26}"
STATE_LINES="${AICX_RECALL_STATE_LINES:-30}"
TAIL_LINES="${AICX_RECALL_TAIL_LINES:-18}"
REFS_MAX="${AICX_RECALL_REFS_MAX:-12}"

input=$(cat)
session_id=$(printf '%s' "$input" | jq -r '.session_id // empty' 2>/dev/null || true)
[ -z "$session_id" ] && exit 0
agent="${AICX_HOOK_AGENT:-codex}"

# ── Pillar 1: NEVER-silent fallback ─────────────────────────────────────────
# The worst failure is this hook emitting nothing on a build error — the agent then proceeds on the
# lossy harness summary BELIEVING recall succeeded. Silent failure is worse than a crash (a crash is
# visible). Every build-failure path below routes here instead, so the agent is ALWAYS told — loudly —
# that recall is degraded and exactly what to read by hand. Plain stdout: no JSON schema to violate.
emit_fallback() {
  printf '⚠️ POSTCOMPACT RECALL DEGRADED — %s.\nYour turn-by-turn memory was just wiped and the rich recall could NOT be built — you are on the LOSSY harness summary ALONE. MANDATORY before acting on anything from earlier in this session: Read the raw transcript at %s (Read tool) plus your memory files; do not trust summary-level recall or guess. This is a mechanism, not a suggestion.\n' "$1" "${extract:-<none>}"
}

# PreCompact runs `aicx extract --conversation`, which writes the `_conversation.md` variant — NOT a
# bare `<id>.md`. Reading the bare name silently no-op'd the entire recall (the postcompact always hit
# the missing-file branch and emitted {}). Match what PreCompact actually writes; fall back to bare.
extract="$HOME/.aicx/extracts/${agent}/${session_id}_conversation.md"
[ -f "$extract" ] || extract="$HOME/.aicx/extracts/${agent}/${session_id}.md"
if [ ! -f "$extract" ]; then
  emit_fallback "aicx extract not found at $extract (PreCompact failed or aicx missing)"
  exit 0
fi

# Fail-closed on missing seal. PreCompact must write a freshness sidecar for THIS
# compact. Exact raw_bytes equality is WRONG for Codex: rollouts are append-only,
# and after PreCompact seals the pre-compact size the harness appends a multi-MB
# `compacted` event + world_state/turn_context before SessionStart(compact).
# Production evidence: 48/48 seals had live > sealed; the operator failure
# sealed=16484387 live=21630630 lands exactly after the post-compact events.
# Accept prefix growth (live >= sealed). Reject shrink/replace or path mismatch.
# After a successful digest emit, consume the seal so a later compact without a
# fresh PreCompact cannot re-inject this extract as "just compacted" truth.
freshness="$extract.freshness.json"
if [ ! -f "$freshness" ]; then
  emit_fallback "freshness sidecar missing for $extract (PreCompact did not seal this extract)"
  exit 0
fi
sealed_path=$(jq -r '.transcript_path // empty' "$freshness" 2>/dev/null || true)
sealed_bytes=$(jq -r '.raw_bytes // empty' "$freshness" 2>/dev/null || true)
hook_path=$(printf '%s' "$input" | jq -r '.transcript_path // .transcriptPath // empty' 2>/dev/null || true)
if [ -n "$hook_path" ] && [ -n "$sealed_path" ] && [ "$hook_path" != "$sealed_path" ]; then
  emit_fallback "extract freshness path mismatch: sealed=$sealed_path live=$hook_path"
  exit 0
fi
transcript_path="${hook_path:-$sealed_path}"
if [ -n "$transcript_path" ] && [ -f "$transcript_path" ] && [ -n "$sealed_bytes" ]; then
  raw_bytes=$(wc -c <"$transcript_path" | tr -d ' ')
  case "$raw_bytes$sealed_bytes" in
    *[!0-9]*)
      emit_fallback "extract freshness unreadable sizes: sealed=$sealed_bytes live=$raw_bytes"
      exit 0
      ;;
  esac
  if [ "$raw_bytes" -lt "$sealed_bytes" ]; then
    emit_fallback "extract freshness shrink: sealed raw_bytes=$sealed_bytes live=$raw_bytes (transcript replaced/truncated)"
    exit 0
  fi
fi

raw_lines=$(wc -l <"$extract" | tr -d ' ')
[ "$raw_lines" -lt 1 ] && { emit_fallback "extract at $extract is empty"; exit 0; }

chunk_dir="/tmp/aicx-recall/${agent}/${session_id}"
mkdir -p "$chunk_dir"
# Clear stale chunks from a prior compact in the same session
find "$chunk_dir" -maxdepth 1 -name 'chunk-*' -type f -delete 2>/dev/null || true
processed="${chunk_dir}/.processed.md"
rm -f "$processed"

# ── Stage 1: strip skill/private bodies ─────────────────────────────────
# Pattern recognized in aicx extracts:
#   <blockquote>\n\n````\nBase directory for this skill: /path/to/<name>\n
#   ...skill body...\n````\n\n</blockquote>
# We replace the fenced body with [SKILL BODY STRIPPED: <name>] so the
# invocation stays in the chronology but the body (3-25 KB each) is gone.
if [ "$STRIP_SKILLS" = "1" ]; then
  awk '
    /^\*\*\[[0-9][0-9:]*\] (assistant[[:space:]_-]+)?(analysis|reasoning|thinking|internal_thought):\*\*[[:space:]]*$/ {
      in_private = 1
      next
    }
    in_private && /^\*\*\[[0-9][0-9:]*\] (user|assistant):\*\*[[:space:]]*$/ {
      in_private = 0
    }
    in_private { next }
    /^<thinking([[:space:]>]|$)/ { in_thinking = 1; next }
    in_thinking && /<\/thinking>/ { in_thinking = 0; next }
    in_thinking { next }
    /^Base directory for this skill: / {
      n = split($0, parts, "/")
      skill_name = parts[n]
      print "[SKILL BODY STRIPPED: " skill_name "]"
      in_skill = 1
      next
    }
    in_skill && /^````[[:space:]]*$/ {
      in_skill = 0
      next
    }
    !in_skill { print }
  ' "$extract" >"${processed}.s1"
else
  cp "$extract" "${processed}.s1"
fi

# ── Stage 2: logical-turn block dedup ───────────────────────────────────
# The C5X Codex projection can repeat a complete role block (same header and
# body). `uniq` only compared adjacent lines and left those turns duplicated in
# the breadcrumb. Buffer complete blocks, emit the first exact occurrence, and
# discard `<dedup-ref>` placeholder turns so the latest P0/state remains the
# newest concrete user/assistant content.
if [ "$DEDUP" = "1" ]; then
  awk '
    function flush( key) {
      if (block == "") return
      if (block ~ /<dedup-ref:[^>]+>/ || block ~ /&lt;dedup-ref:[^&]+&gt;/) {
        block = ""
        return
      }
      key = block
      if (!(key in seen)) {
        printf "%s", block
        seen[key] = 1
      }
      block = ""
    }
    /^\*\*\[[0-9][0-9:]*\] (user|assistant):\*\*[[:space:]]*$/ {
      flush()
      block = $0 ORS
      in_turn = 1
      next
    }
    {
      if (in_turn) block = block $0 ORS
      else print
    }
    END { flush() }
  ' "${processed}.s1" >"$processed"
else
  mv "${processed}.s1" "$processed"
fi
rm -f "${processed}.s1"

processed_lines=$(wc -l <"$processed" | tr -d ' ')
reduction_pct=$((100 - (processed_lines * 100 / raw_lines)))

# Split processed file into chunks (the APPENDIX — lazy deep-dive, not the delivery).
split -l "$CHUNK_SIZE" -d -a 3 "$processed" "${chunk_dir}/chunk-" 2>/dev/null || true
num_chunks=$(find "$chunk_dir" -maxdepth 1 -name 'chunk-*' -type f | wc -l | tr -d ' ')
[ "$num_chunks" -lt 1 ] && { emit_fallback "chunking produced no chunks from $extract"; exit 0; }
last_idx=$((num_chunks - 1))
last_chunk=$(printf 'chunk-%03d' "$last_idx")

# ── Digest extraction ───────────────────────────────────────────────────────
# Split the processed transcript into turns and keep the LAST of each role.
# Turn headers look like: **[HH:MM:SS] user:**  /  **[HH:MM:SS] assistant:**
# close()+truncating-redirect makes each role file hold ONLY its most-recent turn.
last_user="${chunk_dir}/.last_user"
last_asst="${chunk_dir}/.last_asst"
rm -f "$last_user" "$last_asst"
awk -v uf="$last_user" -v af="$last_asst" '
  function flush() {
    if (role=="u")      { close(uf); printf "%s", buf > uf }
    else if (role=="a") { close(af); printf "%s", buf > af }
  }
  /^\*\*\[[0-9][0-9:]*\] user:\*\*[[:space:]]*$/      { flush(); role="u"; buf=""; next }
  /^\*\*\[[0-9][0-9:]*\] assistant:\*\*[[:space:]]*$/ { flush(); role="a"; buf=""; next }
  { if (role!="") buf = buf $0 "\n" }
  END { flush() }
' "$processed" 2>/dev/null || true

# Render a captured turn: strip the markdown quote prefix, squeeze blank runs,
# cap to N lines, and footnote the remainder. Reads from a FILE (never echo).
render_turn() {
  local src="$1" max="$2" total
  [ -s "$src" ] || return 1
  total=$(sed -e 's/^>[[:space:]]\{0,1\}//' "$src" | grep -cve '^[[:space:]]*$')
  [ "${total:-0}" -lt 1 ] && return 1
  sed -e 's/^>[[:space:]]\{0,1\}//' "$src" | cat -s | grep -ve '^[[:space:]]*$' | head -n "$max"
  if [ "$total" -gt "$max" ]; then
    printf '   …[+%s more lines — full text in the transcript/chunks below]\n' "$((total - max))"
  fi
  return 0
}

# Files & commit SHAs from the recent window (the operator's "pliki i miejsca w kodzie").
refs=$(
  tail -n 200 "$processed" \
    | grep -oE '(/[A-Za-z0-9._-]+){2,}\.[A-Za-z0-9]+(:[0-9]+)?|[A-Za-z0-9_-]+(/[A-Za-z0-9_.-]+)+\.(py|rs|sh|kdl|md|swift|toml|js|ts|json|tsx|kt)(:[0-9]+)?' \
    | sed "s|$HOME|~|g" \
    | grep -vE '^~?/?(tmp|private|var|usr|opt|bin|etc)/' \
    | sort -u | head -n "$REFS_MAX"
)
# Commit SHAs get their own pass (short hex, kept distinct from the path list above).
shas=$(tail -n 200 "$processed" | grep -oiE 'commit[^0-9a-f]{0,4}[0-9a-f]{7,12}|\b[0-9a-f]{7}\b' \
  | grep -oE '[0-9a-f]{7,12}' | sort -u | head -n 6)

# Recent-turns BREADCRUMB: header + first content line of each recent turn. A
# chronological "what happened" trail — far denser and less redundant than a raw
# last-N-line dump (which, when the final turn is a long handoff, just re-prints it).
breadcrumb=$(
  awk '
    /^\*\*\[[0-9][0-9:]*\] (user|assistant):\*\*[[:space:]]*$/ {
      if (hdr != "") print hdr " — " first
      hdr = $0; sub(/^\*\*/, "", hdr); sub(/:\*\*[[:space:]]*$/, "", hdr)
      first = ""; want = 1; next
    }
    want && $0 !~ /^[[:space:]]*$/ {
      line = $0; gsub(/^>[[:space:]]*/, "", line)
      if (line != "") { first = substr(line, 1, 110); want = 0 }
    }
    END { if (hdr != "") print hdr " — " first }
  ' "$processed" | tail -n 8
)

# ── Assemble the digest (front-loaded: GOLD first, pointers last) ────────────
build_digest() {
  printf '╔══════════════════════════════════════════════════════════════════════╗\n'
  printf '║ 🧠 AICX RECALL — session %s\n' "$session_id"
  printf '║ was JUST COMPACTED. Your turn-by-turn memory is WIPED (drift #5).\n'
  printf '║ This is recovered memory delivered DIRECTLY — read it, do NOT guess.\n'
  printf '║ Do NOT rely on the lossy harness summary alone. Certainty over thrift.\n'
  printf '╚══════════════════════════════════════════════════════════════════════╝\n'

  printf '\n▓▓▓ [P0] LATEST ASK — what you were last told to do ▓▓▓\n'
  render_turn "$last_user" "$ASK_LINES" \
    || printf '   (no user turn parsed — see VERBATIM TAIL and transcript below)\n'

  printf '\n▓▓▓ STATE — your last handoff (decisions · commits · next steps) ▓▓▓\n'
  render_turn "$last_asst" "$STATE_LINES" \
    || printf '   (no assistant turn parsed — see VERBATIM TAIL below)\n'

  if [ -n "$refs" ] || [ -n "$shas" ]; then
    printf '\n▓▓▓ FILES & REFS touched recently ▓▓▓\n'
    [ -n "$shas" ] && printf '   commits: %s\n' "$(printf '%s ' "$shas" | tr '\n' ' ')"
    [ -n "$refs" ] && printf '%s\n' "$refs" | sed 's/^/   /'
  fi

  printf '\n▓▓▓ RECENT TURNS — chronological breadcrumb (newest last) ▓▓▓\n'
  if [ -n "$breadcrumb" ]; then
    printf '%s\n' "$breadcrumb" | sed 's/^/   /'
  else
    tail -n "$TAIL_LINES" "$processed"
  fi

  printf '\n▓▓▓ APPENDIX (deep dive only — these are POINTERS, not the recall) ▓▓▓\n'
  printf '   %s chunks (skill-stripped + block-deduped, %s%% smaller), chronological:\n' "$num_chunks" "$reduction_pct"
  printf '     %s/chunk-000 (oldest) … %s/%s (newest)\n' "$chunk_dir" "$chunk_dir" "$last_chunk"
  printf '   Full raw transcript: %s\n' "$extract"
  printf '   Skills show as [SKILL BODY STRIPPED: <name>] — re-open via the Skill tool.\n'
}

# Readability pass: un-escape the HTML entities the markdown extract carries (&quot; &lt; &gt; &#39;
# &amp;) and drop aicx's own per-turn footer line so it never leaks into the digest sections.
clean() {
  sed -e 's/&quot;/"/g' -e 's/&#39;/'"'"'/g' -e 's/&lt;/</g' -e 's/&gt;/>/g' -e 's/&amp;/\&/g' \
    -e '/Generated by ai-contexters/d'
}

# Emit plain stdout — SessionStart injects it as context verbatim, no output schema to drift against.
# SANITIZE: aicx extracts can carry NUL/bidi/ZWS; strip C0 (except tab/newline) and drop invalid UTF-8
# so injected context is always clean text. Self-validate: emit only if non-empty, else LOUD fallback.
recall_text=$(build_digest 2>/dev/null | clean | LC_ALL=C tr -d '\000-\010\013-\037' | iconv -f UTF-8 -t UTF-8 -c)
if [ -n "$recall_text" ]; then
  printf '%s\n' "$recall_text"
  # One-shot seal: next compact must run PreCompact again or recall degrades.
  if [ -f "$freshness" ]; then
    mv -f "$freshness" "${freshness}.consumed" 2>/dev/null || rm -f "$freshness"
  fi
else
  emit_fallback "recall digest assembly failed (sanitize error or empty processed transcript)"
fi
