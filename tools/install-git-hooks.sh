#!/bin/sh
# install-git-hooks.sh — install the Git hooks as an immutable per-revision snapshot.
# Behind `make git-hooks`. Refuses any foreign or higher-precedence core.hooksPath,
# copies the hook blobs out of HEAD (never the working tree) into an absolute path
# shared by every worktree, and removes only exact legacy Loctree symlinks.
set -eu

ROOT_DIR="$(CDPATH='' cd -- "$(dirname -- "$0")/.." && pwd -P)"
common_dir_raw="$(git -C "$ROOT_DIR" rev-parse --git-common-dir)"
case "$common_dir_raw" in
  /*) COMMON_DIR="$common_dir_raw" ;;
  *) COMMON_DIR="$(CDPATH='' cd -- "$ROOT_DIR/$common_dir_raw" && pwd -P)" ;;
esac

SOURCE_REVISION="$(git -C "$ROOT_DIR" rev-parse HEAD)"
INSTALL_ROOT="$COMMON_DIR/loctree-hooks"
INSTALL_DIR="$INSTALL_ROOT/$SOURCE_REVISION"
MARKER="$INSTALL_DIR/.loctree-managed-hooks"
LOCK_DIR="$INSTALL_ROOT/.install-lock"
TMP_INSTALL_DIR=""
WORKTREE_CONFIGS=""

current_local_hooks_path="$(git -C "$ROOT_DIR" config --local --get core.hooksPath 2>/dev/null || true)"
current_effective_hooks_path="$(git -C "$ROOT_DIR" config --get core.hooksPath 2>/dev/null || true)"
worktree_config_enabled="$(git -C "$ROOT_DIR" config --local --get extensions.worktreeConfig 2>/dev/null || true)"
current_worktree_hooks_path=""
if [ "$worktree_config_enabled" = "true" ]; then
  current_worktree_hooks_path="$(git -C "$ROOT_DIR" config --worktree --get core.hooksPath 2>/dev/null || true)"
fi

# Refuses installation when a configured core.hooksPath is foreign — anything other
# than empty, the legacy tools/hooks value, or a marker-bearing snapshot this
# installer produced.
validate_owned_path() {
  candidate="$1"
  label="$2"
  case "$candidate" in
    ""|tools/hooks) return 0 ;;
    "$INSTALL_ROOT"/*)
      if [ -f "$candidate/.loctree-managed-hooks" ]; then
        return 0
      fi
      ;;
  esac
  printf 'Refusing foreign %s core.hooksPath: %s\n' "$label" "$candidate" >&2
  exit 1
}

validate_owned_path "$current_local_hooks_path" local
validate_owned_path "$current_worktree_hooks_path" worktree

# A global, system, command-scoped or worktree-specific policy is not ours to
# shadow. The explicit installer only migrates an absent policy, the historical
# Loctree tools/hooks value, or one of its own immutable snapshots.
expected_effective_hooks_path="$current_local_hooks_path"
if [ -n "$current_worktree_hooks_path" ]; then
  expected_effective_hooks_path="$current_worktree_hooks_path"
fi
if [ -z "$expected_effective_hooks_path" ] && [ -n "$current_effective_hooks_path" ]; then
  printf '%s\n' \
    "Refusing to shadow effective core.hooksPath: $current_effective_hooks_path" \
    "Its policy is not owned by this repository's local config." >&2
  exit 1
fi
if [ -n "$expected_effective_hooks_path" ] &&
   [ "$current_effective_hooks_path" != "$expected_effective_hooks_path" ]; then
  printf '%s\n' \
    "Refusing higher-precedence core.hooksPath: $current_effective_hooks_path" \
    "Expected the repository-owned value: $expected_effective_hooks_path" >&2
  exit 1
fi

# The legacy path is accepted only when it contains the two known lightweight
# hooks (plus their library). Additional executable hooks are foreign policy.
if [ "$current_effective_hooks_path" = "tools/hooks" ]; then
  for existing in "$ROOT_DIR"/tools/hooks/*; do
    [ -e "$existing" ] || [ -L "$existing" ] || continue
    case "$(basename -- "$existing")" in
      pre-commit|commit-msg|lib) continue ;;
    esac
    if [ -x "$existing" ]; then
      printf 'Refusing to disable additional tracked Git hook: %s\n' "$existing" >&2
      exit 1
    fi
  done
fi

# With no configured hooksPath, non-sample executables in the default common
# hooks directory are active policy. Only exact legacy Loctree symlinks may be
# migrated; unknown hooks remain untouched and block installation.
if [ -z "$current_effective_hooks_path" ]; then
  for existing in "$COMMON_DIR"/hooks/*; do
    [ -e "$existing" ] || [ -L "$existing" ] || continue
    case "$(basename -- "$existing")" in
      *.sample) continue ;;
    esac
    hook_name="$(basename -- "$existing")"
    if [ -L "$existing" ] &&
       [ "$(readlink "$existing")" = "../../tools/hooks/$hook_name" ]; then
      case "$hook_name" in
        pre-commit|commit-msg|pre-push) continue ;;
      esac
    fi
    if [ -x "$existing" ]; then
      printf '%s\n' \
        "Refusing to disable existing Git hook: $existing" \
        "Review and migrate that hook explicitly, then run 'make git-hooks' again." >&2
      exit 1
    fi
  done
fi

for hook_source in \
  tools/hooks/pre-commit \
  tools/hooks/commit-msg \
  tools/hooks/lib/commit-msg-diff-gate.sh; do
  if ! git -C "$ROOT_DIR" ls-files --error-unmatch "$hook_source" >/dev/null 2>&1; then
    printf 'Refusing untracked hook source: %s\n' "$hook_source" >&2
    exit 1
  fi
  hook_source_path="$ROOT_DIR/$hook_source"
  if [ -L "$hook_source_path" ] || [ ! -f "$hook_source_path" ] ||
     [ ! -x "$hook_source_path" ]; then
    printf 'Refusing unsafe hook source type or mode: %s\n' "$hook_source" >&2
    exit 1
  fi
  head_source_entry="$(git -C "$ROOT_DIR" ls-tree "$SOURCE_REVISION" -- "$hook_source")"
  head_source_mode="$(printf '%s\n' "$head_source_entry" | awk '{print $1}')"
  head_source_type="$(printf '%s\n' "$head_source_entry" | awk '{print $2}')"
  head_source_oid="$(printf '%s\n' "$head_source_entry" | awk '{print $3}')"
  working_source_oid="$(git hash-object --no-filters "$hook_source_path")"
  if [ "$head_source_mode" != "100755" ] || [ "$head_source_type" != "blob" ] ||
     [ -z "$head_source_oid" ] || [ "$working_source_oid" != "$head_source_oid" ]; then
    printf 'Refusing hook source that differs from executable HEAD blob: %s\n' \
      "$hook_source" >&2
    exit 1
  fi
done
if ! git -C "$ROOT_DIR" diff --quiet "$SOURCE_REVISION" -- \
  tools/hooks/pre-commit tools/hooks/commit-msg tools/hooks/lib/commit-msg-diff-gate.sh; then
  printf '%s\n' \
    'Refusing to install modified hook sources.' \
    'Commit and review the hook snapshot before running make git-hooks.' >&2
  exit 1
fi

if [ -L "$INSTALL_ROOT" ] || { [ -e "$INSTALL_ROOT" ] && [ ! -d "$INSTALL_ROOT" ]; }; then
  printf 'Refusing non-directory hook snapshot root: %s\n' "$INSTALL_ROOT" >&2
  exit 1
fi
mkdir -p "$INSTALL_ROOT"
if ! mkdir "$LOCK_DIR" 2>/dev/null; then
  printf 'Another hook installation is active or left a lock: %s\n' "$LOCK_DIR" >&2
  exit 1
fi

# EXIT trap: removes a half-built snapshot directory, the worktree-config list, and
# the installation lock so a failed run leaves no wedged state behind.
cleanup() {
  if [ -n "$TMP_INSTALL_DIR" ] && [ -d "$TMP_INSTALL_DIR" ]; then
    rm -R "$TMP_INSTALL_DIR"
  fi
  if [ -n "$WORKTREE_CONFIGS" ]; then
    rm -f "$WORKTREE_CONFIGS"
  fi
  rmdir "$LOCK_DIR" 2>/dev/null || true
}
trap cleanup EXIT
trap 'exit 1' HUP INT TERM

# Validate every existing worktree-specific config before enabling that config
# extension. This prevents a dormant foreign config.worktree from becoming
# active during migration.
WORKTREE_CONFIGS="$LOCK_DIR/worktree-configs"
: > "$WORKTREE_CONFIGS"
git -C "$ROOT_DIR" worktree list --porcelain | while IFS= read -r line; do
  case "$line" in
    "worktree "*) worktree_path="${line#worktree }" ;;
    *) continue ;;
  esac
  [ -d "$worktree_path" ] || continue
  worktree_git_dir_raw="$(git -C "$worktree_path" rev-parse --git-dir)"
  case "$worktree_git_dir_raw" in
    /*) worktree_git_dir="$worktree_git_dir_raw" ;;
    *) worktree_git_dir="$(CDPATH='' cd -- "$worktree_path/$worktree_git_dir_raw" && pwd -P)" ;;
  esac
  worktree_config="$worktree_git_dir/config.worktree"
  worktree_value="$(git config --file "$worktree_config" --get core.hooksPath 2>/dev/null || true)"
  validate_owned_path "$worktree_value" "$worktree_config"
  printf '%s\n' "$worktree_config" >> "$WORKTREE_CONFIGS"
done

if [ -e "$INSTALL_DIR" ]; then
  snapshot_unexpected="$(find "$INSTALL_DIR" -mindepth 1 \
    ! -path "$INSTALL_DIR/.loctree-managed-hooks" \
    ! -path "$INSTALL_DIR/pre-commit" \
    ! -path "$INSTALL_DIR/commit-msg" \
    ! -path "$INSTALL_DIR/lib" \
    ! -path "$INSTALL_DIR/lib/commit-msg-diff-gate.sh" -print -quit)"
  snapshot_files_valid=true
  for installed_hook in pre-commit commit-msg lib/commit-msg-diff-gate.sh; do
    if [ ! -f "$INSTALL_DIR/$installed_hook" ] ||
       [ -L "$INSTALL_DIR/$installed_hook" ] ||
       [ ! -x "$INSTALL_DIR/$installed_hook" ]; then
      snapshot_files_valid=false
    fi
  done
  if [ -L "$INSTALL_DIR" ] || [ ! -d "$INSTALL_DIR" ] || [ ! -f "$MARKER" ] ||
     [ -L "$INSTALL_DIR/lib" ] || [ ! -d "$INSTALL_DIR/lib" ] ||
     [ -n "$snapshot_unexpected" ] || [ "$snapshot_files_valid" != "true" ] ||
     ! grep -Fqx "source-revision=$SOURCE_REVISION" "$MARKER" ||
     ! cmp -s "$ROOT_DIR/tools/hooks/pre-commit" "$INSTALL_DIR/pre-commit" ||
     ! cmp -s "$ROOT_DIR/tools/hooks/commit-msg" "$INSTALL_DIR/commit-msg" ||
     ! cmp -s "$ROOT_DIR/tools/hooks/lib/commit-msg-diff-gate.sh" \
       "$INSTALL_DIR/lib/commit-msg-diff-gate.sh"; then
    printf 'Refusing modified or incomplete hook snapshot: %s\n' "$INSTALL_DIR" >&2
    exit 1
  fi
else
  TMP_INSTALL_DIR="$INSTALL_ROOT/.snapshot.$$.tmp"
  umask 077
  mkdir -p "$TMP_INSTALL_DIR/lib"
  printf 'source-revision=%s\n' "$SOURCE_REVISION" > "$TMP_INSTALL_DIR/.loctree-managed-hooks"
  git -C "$ROOT_DIR" cat-file blob \
    "$SOURCE_REVISION:tools/hooks/lib/commit-msg-diff-gate.sh" > \
    "$TMP_INSTALL_DIR/lib/commit-msg-diff-gate.sh"
  git -C "$ROOT_DIR" cat-file blob \
    "$SOURCE_REVISION:tools/hooks/commit-msg" > \
    "$TMP_INSTALL_DIR/commit-msg"
  git -C "$ROOT_DIR" cat-file blob \
    "$SOURCE_REVISION:tools/hooks/pre-commit" > \
    "$TMP_INSTALL_DIR/pre-commit"
  chmod 0755 \
    "$TMP_INSTALL_DIR/lib/commit-msg-diff-gate.sh" \
    "$TMP_INSTALL_DIR/commit-msg" \
    "$TMP_INSTALL_DIR/pre-commit"
  mv "$TMP_INSTALL_DIR" "$INSTALL_DIR"
  TMP_INSTALL_DIR=""
fi

# Configure the shared repository only after the complete immutable snapshot
# exists. The absolute path is identical for every linked worktree and never
# resolves to branch-controlled files in the checkout where Git happens to run.
git -C "$ROOT_DIR" config --local core.hooksPath "$INSTALL_DIR"
while IFS= read -r worktree_config; do
  git config --file "$worktree_config" core.hooksPath "$INSTALL_DIR"
done < "$WORKTREE_CONFIGS"
git -C "$ROOT_DIR" config --local extensions.worktreeConfig true

# Remove only exact legacy Loctree symlinks. Unknown hooks are never deleted or
# chained silently. Obsolete installers may recreate these links, but the
# absolute core.hooksPath keeps the default directory inactive.
for hook in pre-commit commit-msg pre-push; do
  legacy="$COMMON_DIR/hooks/$hook"
  if [ -L "$legacy" ] && [ "$(readlink "$legacy")" = "../../tools/hooks/$hook" ]; then
    rm -f "$legacy"
  fi
done

printf '%s\n' \
  "Installed hook snapshot: $INSTALL_DIR" \
  "Enabled: pre-commit, commit-msg" \
  "Disabled by design: pre-push (run 'make preflight' explicitly)"
