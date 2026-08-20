#!/bin/sh
set -eu

ROOT_DIR="$(CDPATH='' cd -- "$(dirname -- "$0")/.." && pwd)"
TMP_ROOT="$(mktemp -d "${TMPDIR:-/tmp}/loctree-preflight-env.XXXXXX")"
CALLER="$TMP_ROOT/caller"
FIXTURE_REPO="$TMP_ROOT/fixture"
FAKE_BIN="$TMP_ROOT/bin"
ENV_LOG="$TMP_ROOT/env.log"

cleanup() {
  rm -R "$TMP_ROOT"
}
trap cleanup EXIT

mkdir -p "$CALLER/loctree-rs" "$CALLER/target/release" "$FIXTURE_REPO" "$FAKE_BIN"
git init -q "$CALLER"
git -C "$CALLER" config core.bare false
git -C "$CALLER" config user.name "Caller Identity"
git -C "$CALLER" config user.email "caller@example.invalid"

cat > "$FAKE_BIN/cargo" <<'EOF'
#!/bin/sh
git -C "$FIXTURE_REPO" init -q
git -C "$FIXTURE_REPO" config user.name "Fixture Identity"
git -C "$FIXTURE_REPO" config user.email "fixture@example.invalid"
for git_local_var in $(git rev-parse --local-env-vars); do
  eval "git_local_value=\${$git_local_var-__loctree_unset__}"
  if [ "$git_local_value" != "__loctree_unset__" ]; then
    printf 'leaked %s=%s\n' "$git_local_var" "$git_local_value" >&2
    exit 1
  fi
done
printf '%s|%s|%s|%s\n' \
  "${GIT_DIR-unset}" \
  "${GIT_WORK_TREE-unset}" \
  "${GIT_INDEX_FILE-unset}" \
  "$*" >> "$ENV_LOG"
EOF
chmod +x "$FAKE_BIN/cargo"

cat > "$FAKE_BIN/node" <<'EOF'
#!/bin/sh
printf '%s|%s|%s|node %s\n' \
  "${GIT_DIR-unset}" \
  "${GIT_WORK_TREE-unset}" \
  "${GIT_INDEX_FILE-unset}" \
  "$*" >> "$ENV_LOG"
EOF
chmod +x "$FAKE_BIN/node"

cat > "$CALLER/target/release/loct" <<'EOF'
#!/bin/sh
printf '%s|%s|%s|loct %s\n' \
  "${GIT_DIR-unset}" \
  "${GIT_WORK_TREE-unset}" \
  "${GIT_INDEX_FILE-unset}" \
  "$*" >> "$ENV_LOG"
EOF
chmod +x "$CALLER/target/release/loct"

export ENV_LOG
export FIXTURE_REPO
export GIT_DIR="$CALLER/.git"
export GIT_WORK_TREE="$CALLER"
export GIT_INDEX_FILE="$CALLER/.git/index"
export GIT_PREFIX="leaked-prefix"
export GIT_CONFIG_COUNT=0
export GIT_NO_REPLACE_OBJECTS=1

PATH="$FAKE_BIN:$PATH" "$ROOT_DIR/tools/preflight.sh" >/dev/null

if [ "$(wc -l < "$ENV_LOG" | tr -d ' ')" -ne 7 ]; then
  echo "expected five cargo calls, one node call, and one loct call" >&2
  cat "$ENV_LOG" >&2
  exit 1
fi

if grep -v '^unset|unset|unset|' "$ENV_LOG" >/dev/null; then
  echo "preflight leaked repository-local Git environment" >&2
  cat "$ENV_LOG" >&2
  exit 1
fi

if [ "$(git -C "$CALLER" config --bool core.bare)" != "false" ]; then
  echo "caller repository was mutated" >&2
  exit 1
fi

if [ "$(git -C "$CALLER" config user.name)" != "Caller Identity" ] ||
   [ "$(git -C "$CALLER" config user.email)" != "caller@example.invalid" ]; then
  echo "fixture Git config leaked into the caller repository" >&2
  exit 1
fi

if [ "$(git -C "$FIXTURE_REPO" rev-parse --is-bare-repository)" != "false" ]; then
  echo "fixture repository was not initialized independently" >&2
  exit 1
fi

: > "$ENV_LOG"
LOCTREE_GIT_ENV_ISOLATION_NESTED=1 \
  make -s -C "$ROOT_DIR" PATH="$FAKE_BIN:$PATH" test >/dev/null

if [ "$(wc -l < "$ENV_LOG" | tr -d ' ')" -ne 1 ]; then
  echo "expected one isolated cargo call from tools/test.sh" >&2
  cat "$ENV_LOG" >&2
  exit 1
fi

if ! grep -Fxq 'unset|unset|unset|test --workspace' "$ENV_LOG"; then
  echo "make test path did not preserve the isolated cargo contract" >&2
  cat "$ENV_LOG" >&2
  exit 1
fi

if [ "$(git -C "$CALLER" config user.name)" != "Caller Identity" ] ||
   [ "$(git -C "$CALLER" config user.email)" != "caller@example.invalid" ]; then
  echo "make test prerequisites mutated the caller repository" >&2
  exit 1
fi

printf '%s\n' 'preflight Git environment isolation: ok'
