#!/usr/bin/env bash
set -euo pipefail

SCRIPT_DIR=$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)
DISTRIBUTION_DIR=$(cd "$SCRIPT_DIR/.." && pwd)
SYNC_SCRIPT="$DISTRIBUTION_DIR/component-sync.sh"
VERSION="${LOCTREE_COMPONENT_SYNC_TEST_VERSION:-0.13.0}"
RUST_VERSION="${LOCTREE_COMPONENT_SYNC_TEST_RUST_VERSION:-1.93.0}"
SOURCE_REF="${LOCTREE_COMPONENT_SYNC_TEST_SOURCE_REF:-a38afb8aba01}"
TMP_ROOT=$(mktemp -d "${TMPDIR:-/tmp}/loctree-component-sync.XXXXXX")
SOURCE_ROOT="$TMP_ROOT/source"

cleanup() {
  # Cargo can leave macOS/APFS directory entries visible for a brief moment
  # after the last compiler exits. Retry once so a green contract is not
  # reported as failed solely by teardown lag.
  rm -rf "$TMP_ROOT" 2>/dev/null || {
    sleep 1
    rm -rf "$TMP_ROOT"
  }
}
trap cleanup EXIT

fail() {
  echo "error: $*" >&2
  exit 1
}

prepare_source_archive() {
  mkdir -p "$SOURCE_ROOT"
  git -C "$DISTRIBUTION_DIR/.." rev-parse --verify "$SOURCE_REF^{commit}" >/dev/null \
    || fail "missing source ref: $SOURCE_REF"
  git -C "$DISTRIBUTION_DIR/.." archive "$SOURCE_REF" | tar -x -C "$SOURCE_ROOT"
  [[ -d "$SOURCE_ROOT/loctree-rs" ]] || fail "archive source missing loctree-rs"
  [[ -d "$SOURCE_ROOT/loctree-ast" ]] || fail "archive source missing loctree-ast"
}

scan_excludes() {
  local staging="$1"
  if find "$staging" \( -path '*/target/*' -o -path '*/.git/*' \) -prune -o \
    -type f -print0 | xargs -0 grep -I -E '\.vibecrafted|loctree-fail|/Users/' >/tmp/component-sync-grep.$$ 2>/dev/null; then
    cat /tmp/component-sync-grep.$$
    rm -f /tmp/component-sync-grep.$$
    fail "excluded marker found in $staging"
  fi
  rm -f /tmp/component-sync-grep.$$
}

semgrep_secrets() {
  local staging="$1"
  command -v semgrep >/dev/null 2>&1 || fail "semgrep is required for component sync tests"
  semgrep --config p/secrets --error --quiet --exclude target "$staging"
}

check_engine_registry_shape() {
  local staging="$1"
  [[ ! -e "$staging/reports" ]] || fail "engine staging unexpectedly contains reports/"
  grep -qx "report-leptos = \"$VERSION\"" "$staging/Cargo.toml" \
    || fail "engine Cargo.toml does not use registry report-leptos $VERSION"
  if grep -q 'report-leptos = { path' "$staging/Cargo.toml"; then
    fail "engine Cargo.toml still contains a path dependency for report-leptos"
  fi
  if grep -q '"reports"' "$staging/Cargo.toml"; then
    fail "engine Cargo.toml still contains reports as a workspace member"
  fi
  if grep -q 'Vendored Build Payload' "$staging/SYNC-MANIFEST.md"; then
    fail "engine sync manifest still advertises vendored payload"
  fi
}

check_mcp_distribution_shape() {
  local staging="$1"
  [[ -f "$staging/glama.json" ]] || fail "MCP staging missing glama.json"
  [[ -f "$staging/Dockerfile" ]] || fail "MCP staging missing Dockerfile"
  [[ -f "$staging/.dockerignore" ]] || fail "MCP staging missing .dockerignore"
  grep -Fq '"https://glama.ai/mcp/schemas/server.json"' "$staging/glama.json" \
    || fail "MCP glama.json does not declare the Glama server schema"
  grep -Fq 'ENTRYPOINT ["loctree-mcp"]' "$staging/Dockerfile" \
    || fail "MCP Dockerfile does not launch loctree-mcp directly"
  grep -Fq 'ENV LOCT_CACHE_DIR=/data/loctree-cache' "$staging/Dockerfile" \
    || fail "MCP Dockerfile does not persist snapshots under /data"
  grep -Fq 'org.opencontainers.image.revision="${VCS_REF}"' "$staging/Dockerfile" \
    || fail "MCP Dockerfile does not expose source revision provenance"
  grep -Eq '^USER [1-9][0-9]*:[1-9][0-9]*$' "$staging/Dockerfile" \
    || fail "MCP Dockerfile does not drop root privileges"
}

check_component() {
  local component="$1"
  local staging="$TMP_ROOT/$component"
  local target_dir="$TMP_ROOT/target-$component"

  bash "$SYNC_SCRIPT" \
    --component "$component" \
    --version "$VERSION" \
    --staging "$staging" \
    --suite-root "$SOURCE_ROOT"
  [[ -f "$staging/Cargo.toml" ]] || fail "missing Cargo.toml for $component"
  [[ -f "$staging/Cargo.lock" ]] || fail "missing Cargo.lock for $component"
  grep -qx "rust-version = \"$RUST_VERSION\"" "$staging/Cargo.toml" \
    || fail "generated Cargo.toml does not declare Rust $RUST_VERSION for $component"
  [[ -f "$staging/LICENSE" ]] || fail "missing LICENSE for $component"
  grep -q 'SPDX-License-Identifier: BUSL-1.1' "$staging/LICENSE" \
    || fail "LICENSE is not BUSL-1.1 for $component"
  scan_excludes "$staging"
  semgrep_secrets "$staging"
  if [[ "$component" == "engine" ]]; then
    check_engine_registry_shape "$staging"
  elif [[ "$component" == "mcp" ]]; then
    check_mcp_distribution_shape "$staging"
  fi
  CARGO_TARGET_DIR="$target_dir" cargo check --locked --manifest-path "$staging/Cargo.toml"
}

prepare_source_archive

check_component engine
check_component mcp
check_component lsp

if LOCTREE_SYNC_CONFIRM=0 bash "$SYNC_SCRIPT" \
  --component engine \
  --version "$VERSION" \
  --staging "$TMP_ROOT/push-refusal" \
  --suite-root "$SOURCE_ROOT" \
  --remote https://example.invalid/Loctree/loctree.git \
  --push >/tmp/component-sync-push.$$ 2>&1; then
  cat /tmp/component-sync-push.$$
  rm -f /tmp/component-sync-push.$$
  fail "push without LOCTREE_SYNC_CONFIRM=1 unexpectedly succeeded"
fi
rm -f /tmp/component-sync-push.$$

echo "component-sync tests passed"
