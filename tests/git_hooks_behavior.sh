#!/bin/sh
set -eu

ROOT_DIR="$(cd "$(dirname "$0")/.." && pwd)"
HOOK="$ROOT_DIR/tools/hooks/pre-commit"
TMP_ROOT="$(mktemp -d "${TMPDIR:-/tmp}/loctree-hook-behavior.XXXXXX")"
REPO="$TMP_ROOT/repo"
FAKE_BIN="$TMP_ROOT/bin"
CARGO_LOG="$TMP_ROOT/cargo.log"

cleanup() {
  rm -R "$TMP_ROOT"
}
trap cleanup EXIT

mkdir -p \
  "$FAKE_BIN" \
  "$REPO/loctree-ast/src" \
  "$REPO/loctree-rs" \
  "$REPO/reports" \
  "$REPO/loctree-mcp" \
  "$REPO/loctree-lsp"
REPO="$(cd "$REPO" && pwd -P)"
# shellcheck disable=SC2016
printf '%s\n' \
  '#!/bin/sh' \
  'printf "%s|%s\n" "$PWD" "$*" >> "$CARGO_LOG"' \
  > "$FAKE_BIN/cargo"
chmod +x "$FAKE_BIN/cargo"

git init -q "$REPO"
git -C "$REPO" config user.name "Loctree Test"
git -C "$REPO" config user.email "loctree-test@example.invalid"
printf '%s\n' \
  'pub fn value() {' \
  '    let staged = 1;' \
  '    let unstaged = 2;' \
  '}' \
  > "$REPO/loctree-ast/src/lib.rs"
git -C "$REPO" add .
git -C "$REPO" commit -qm "test: seed hook fixture"

printf '%s\n' \
  'pub fn value() {' \
  '    let staged = 10;' \
  '    let unstaged = 2;' \
  '}' \
  > "$REPO/loctree-ast/src/lib.rs"
git -C "$REPO" add loctree-ast/src/lib.rs
printf '%s\n' \
  'pub fn value() {' \
  '    let staged = 10;' \
  '    let unstaged = 20;' \
  '}' \
  > "$REPO/loctree-ast/src/lib.rs"

export CARGO_LOG
(
  cd "$REPO"
  PATH="$FAKE_BIN:$PATH" "$HOOK"
)

staged_diff="$(git -C "$REPO" diff --cached -- loctree-ast/src/lib.rs)"
unstaged_diff="$(git -C "$REPO" diff -- loctree-ast/src/lib.rs)"

printf '%s\n' "$staged_diff" | grep -Fq 'let staged = 10;'
if printf '%s\n' "$staged_diff" | grep -Fq 'let unstaged = 20;'; then
  echo "pre-commit absorbed an unstaged hunk into the index" >&2
  exit 1
fi
printf '%s\n' "$unstaged_diff" | grep -Fq 'let unstaged = 20;'

expected_call="$REPO|fmt --all -- --check"
if [ "$(wc -l < "$CARGO_LOG" | tr -d ' ')" -ne 1 ]; then
  echo "expected one workspace formatting check" >&2
  cat "$CARGO_LOG" >&2
  exit 1
fi
if ! grep -Fxq "$expected_call" "$CARGO_LOG"; then
  echo "expected workspace formatting check: $expected_call" >&2
  cat "$CARGO_LOG" >&2
  exit 1
fi
