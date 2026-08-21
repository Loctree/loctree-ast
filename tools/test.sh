#!/bin/sh
# test.sh — workspace test entrypoint behind `make test`.
# Clears repository-local Git variables first so fixture repositories created by
# the tests cannot be redirected back into the caller's real repository.
set -eu

REPO_ROOT="$(git rev-parse --show-toplevel)"
SCRIPT_DIR="$(CDPATH='' cd -- "$(dirname -- "$0")" && pwd)"
# shellcheck source=tools/lib/git-env-isolation.sh
. "$SCRIPT_DIR/lib/git-env-isolation.sh"
loctree_clear_local_git_env

cd "$REPO_ROOT"
cargo test --workspace
