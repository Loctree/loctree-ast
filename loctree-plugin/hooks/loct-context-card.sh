#!/usr/bin/env bash
# loct-context-card.sh — SessionStart hook
#
# Emits a `loct context` Agent Context Pack as additionalContext at session
# startup / clear / compact, so every fresh thread starts with structural
# perception (vc-init Sense 2) without manual invocation.
#
# Behavior:
#   - Skipped silently when:
#     * `loct` binary is not on PATH
#     * cwd is not inside a git repo (avoids workspace-parent footgun;
#       MCP-parity gap M01 will eventually catch this server-side too)
#     * cwd has no `.loctree/` cached snapshot AND no `--fresh` indicator
#       (we use `--no-scan` so loct refuses to auto-scan; fail = exit 0)
#   - Cached per-cwd at `/tmp/claude-loct-card-<sha256(cwd)>` with 30 min TTL
#     so consecutive sessions in same repo don't re-render
#   - Markdown output is piped straight to stdout — Claude Code SessionStart
#     hook treats stdout as additionalContext for the new session
#
# Operator preferences honored:
#   - Quiet on non-loct dirs (no spam)
#   - Fast first-render (no auto-scan; rely on operator's existing snapshot)
#   - Idempotent: cache-hit means zero loct invocation
#
# 𝚅𝚒𝚋𝚎𝚌𝚛𝚊𝚏𝚝𝚎𝚍. with AI Agents by Vetcoders (c)2024-2026 LibraxisAI

set -euo pipefail

CACHE_TTL_SECONDS="${LOCT_CARD_CACHE_TTL:-1800}"   # 30 min default
CACHE_DIR="${TMPDIR:-/tmp}"

# Resolve the cwd. Claude Code sets CLAUDE_PROJECT_DIR; fall back to PWD.
cwd="${CLAUDE_PROJECT_DIR:-$PWD}"

# 0. Skip silently on irrelevant dirs.
if [[ ! -d "$cwd" ]]; then
    exit 0
fi
if ! command -v loct >/dev/null 2>&1; then
    exit 0
fi
if ! git -C "$cwd" rev-parse --git-dir >/dev/null 2>&1; then
    exit 0
fi

# 1. Per-cwd cache key — sha256 of absolute path. Avoids cross-repo bleed.
cwd_abs="$(cd "$cwd" && pwd -P)"
cwd_hash="$(printf '%s' "$cwd_abs" | shasum -a 256 | cut -c1-16)"
cache_file="${CACHE_DIR}/claude-loct-card-${cwd_hash}"

# 2. Cache-hit fast path.
if [[ -f "$cache_file" ]]; then
    cache_age=$(( $(date +%s) - $(stat -f %m "$cache_file" 2>/dev/null || stat -c %Y "$cache_file") ))
    if (( cache_age < CACHE_TTL_SECONDS )); then
        cat "$cache_file"
        exit 0
    fi
fi

# 3. Cache-miss: render fresh. Hard 10s budget — context render is fast on
#    a warm snapshot; if it takes longer, snapshot is missing/stale and
#    we'd rather ship nothing than block session startup.
context_output=$(cd "$cwd_abs" && timeout 10 loct context --no-scan 2>/dev/null || true)

# Empty output = no snapshot, skipped scan, or error. Emit nothing.
if [[ -z "${context_output// }" ]]; then
    exit 0
fi

# 4. Write cache + emit. Wrap with a brief framing block so the agent
#    knows where this came from.
{
    printf '<!-- loct-context-card SessionStart hook (cached %ds TTL) -->\n' "$CACHE_TTL_SECONDS"
    printf '\n'
    printf '%s\n' "$context_output"
} | tee "$cache_file"
