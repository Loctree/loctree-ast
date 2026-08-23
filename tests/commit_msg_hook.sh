#!/bin/sh
set -eu

ROOT_DIR="$(cd "$(dirname "$0")/.." && pwd)"
HOOK="$ROOT_DIR/tools/hooks/commit-msg"
if [ ! -x "$HOOK" ]; then
  HOOK="$ROOT_DIR/.git/hooks/commit-msg"
fi

tmpdir="$(mktemp -d)"
trap 'rm -R "$tmpdir"' EXIT

run_hook() {
  msg="$1"
  "$HOOK" "$msg"
}

cat > "$tmpdir/valid-iso-coauthor.msg" <<'MSG'
[claude/interactive] fix: restore native toolbar, Replace, Share + tab UX

Addresses six findings from a ScreenScribe review of the editor.

Prior invalid history is just body text and must not be validated here:
docs(spec): VS Code Context-King surface design

Authored-By: claude <agents@vetcoders.io>
session_id: 9c13d55e-1af1-4dc0-ae3b-382659e4f766
date: 2026-06-04T15:36:27 MDT
runtime: claude-code
Co-Authored-By: Claude Opus 4.8 <noreply@anthropic.com>
MSG

run_hook "$tmpdir/valid-iso-coauthor.msg"

cat > "$tmpdir/valid-human.msg" <<'MSG'
fix(hooks): make worktree installation safe
MSG

run_hook "$tmpdir/valid-human.msg"

# Plain Conventional Commits represent human authorship and intentionally do
# not claim the agent provenance contract.
if grep -Eq '^(Authored-By|session_id|date|runtime):' "$tmpdir/valid-human.msg"; then
  echo "human fixture unexpectedly carries agent provenance" >&2
  exit 1
fi

cat > "$tmpdir/invalid-human.msg" <<'MSG'
update hooks
MSG

if run_hook "$tmpdir/invalid-human.msg" >/dev/null 2>&1; then
  echo "expected hook to reject non-conventional human commit messages" >&2
  exit 1
fi

cat > "$tmpdir/missing-current-metadata.msg" <<'MSG'
[codex/interactive] fix: repair release commit message validation

Explains the change.

Authored-By: codex <agents@vetcoders.io>
runtime: codex
MSG

if run_hook "$tmpdir/missing-current-metadata.msg" >/dev/null 2>&1; then
  echo "expected hook to reject current commit messages without session metadata" >&2
  exit 1
fi

cat > "$tmpdir/version-flow.msg" <<'MSG'
[codex/interactive] chore(release): bump versions

loctree=0.12.2 loctree-mcp=0.12.2 loctree-lsp=0.12.2

Authored-By: codex <agents@vetcoders.io>
session_id: 019e93be-379d-7303-9ad4-ffae468db99f
date: 2026-06-04T15:36:27 MDT
runtime: make-version
MSG

run_hook "$tmpdir/version-flow.msg"
