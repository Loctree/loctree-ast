#!/bin/sh
set -eu

REPO_ROOT="$(git rev-parse --show-toplevel)"
SCRIPT_DIR="$(CDPATH='' cd -- "$(dirname -- "$0")" && pwd)"
# shellcheck source=tools/lib/git-env-isolation.sh
. "$SCRIPT_DIR/lib/git-env-isolation.sh"

# Git hooks export repository-local variables such as GIT_DIR. If preflight is
# invoked from a hook or another Git-managed wrapper, those variables otherwise
# leak into tests that create fixture repositories and can redirect their Git
# commands back into the caller's real repository.
loctree_clear_local_git_env

cd "$REPO_ROOT"

echo "=== Loctree preflight ==="

echo "[1/7] Workspace formatting..."
cargo fmt --all -- --check

echo "[2/7] Workspace clippy..."
cargo clippy --workspace --all-targets -- -D warnings

echo "[3/7] Workspace check..."
cargo check --workspace

echo "[4/7] Workspace tests..."
cargo test --workspace

echo "[5/7] npm MCP adapter smoke..."
node distribution/tests/npm_wrapper_test.js

echo "[6/7] Release dogfooding build..."
cargo build -p loctree --release --quiet

echo "[7/7] Loctree cycle analysis..."
(cd loctree-rs && "$REPO_ROOT/target/release/loct" cycles)

echo "=== Loctree preflight passed ==="
