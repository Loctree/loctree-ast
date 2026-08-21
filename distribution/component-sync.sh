#!/usr/bin/env bash
set -euo pipefail

# Stage one public component mirror (engine, mcp or lsp) out of this private
# suite: copy the manifest-declared payload, generate the workspace/licence/readme
# metadata, scrub private markers, and optionally graft the snapshot onto the
# mirror's main branch without deleting paths the mirror owns.

SCRIPT_DIR=$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)
METADATA_ROOT=$(cd "$SCRIPT_DIR/.." && pwd)
SUITE_ROOT="$METADATA_ROOT"
MANIFEST_DIR="$SCRIPT_DIR/component-manifests"

COMPONENT=""
VERSION=""
STAGING=""
REMOTE_URL=""
PUSH=0

TARGET_REPO=""
LICENSE_ID=""
README_SRC=""
DEPENDENCY_MODE=""
DEPENDENCY_NOTE=""
INCLUDES=()
VENDORS=()
EXTRAS=()
EXCLUDES=()
REMOVALS=()

# Files this script generates into every staging tree. They are "owned" by the
# sync in exactly the same sense as the manifest include/extra targets: the
# suite is authoritative for them and the mirror copy may be replaced.
GENERATED_PATHS=(
  Cargo.toml
  Cargo.lock
  LICENSE
  README.md
  NOTICE.md
  SYNC-MANIFEST.md
)

# Print the CLI contract, including the three preconditions a push requires and
# the remove= rule that governs mirror deletions.
usage() {
  cat <<'EOF'
Usage:
  distribution/component-sync.sh --component engine|mcp|lsp --version X --staging DIR [options]

Options:
  --component <name>  Component mirror to stage: engine, mcp, or lsp.
  --version <semver>  Release version to stamp into the staging workspace.
  --staging <dir>     Local staging directory to recreate.
  --suite-root <dir>  Source suite root to copy component payloads from.
                      Defaults to the parent of this distribution directory.
  --remote <url>      Public mirror remote URL. Required only with --push.
  --push              Push staged snapshot to remote main.
  -h, --help          Show this help.

Default mode creates a local staging directory only. Push is disabled unless
--push, --remote, and LOCTREE_SYNC_CONFIRM=1 are all present.

--push grafts the staged snapshot onto remote main and PRESERVES every mirror
path the manifest does not own (governance files, mirror-only CI). Deleting a
mirror path requires an explicit `remove=<path>` line in the component
manifest; an undeclared delete aborts the push.
EOF
}

# Abort the sync with a message on stderr and a non-zero exit status.
die() {
  echo "error: $*" >&2
  exit 1
}

# Expand ~ and resolve one path to an absolute path, so the staging-safety checks
# compare canonical paths before anything is deleted.
abs_path() {
  python3 - "$1" <<'PY'
import os
import sys

print(os.path.abspath(os.path.expanduser(sys.argv[1])))
PY
}

while [[ $# -gt 0 ]]; do
  case "$1" in
    --component)
      COMPONENT="${2:-}"; shift 2 ;;
    --version)
      VERSION="${2:-}"; shift 2 ;;
    --staging)
      STAGING="${2:-}"; shift 2 ;;
    --suite-root)
      SUITE_ROOT="${2:-}"; shift 2 ;;
    --remote)
      REMOTE_URL="${2:-}"; shift 2 ;;
    --push)
      PUSH=1; shift ;;
    -h|--help)
      usage; exit 0 ;;
    *)
      die "unknown argument: $1" ;;
  esac
done

[[ -n "$COMPONENT" ]] || die "--component is required"
[[ -n "$VERSION" ]] || die "--version is required"
[[ -n "$STAGING" ]] || die "--staging is required"
[[ -n "$SUITE_ROOT" ]] || die "--suite-root must not be empty"
SUITE_ROOT=$(abs_path "$SUITE_ROOT")
[[ -d "$SUITE_ROOT" ]] || die "--suite-root does not exist: $SUITE_ROOT"

case "$COMPONENT" in
  engine|mcp|lsp) ;;
  *) die "--component must be engine, mcp, or lsp" ;;
esac

case "$VERSION" in
  v*) die "--version must not include a leading v" ;;
esac

if [[ "$PUSH" == "1" ]]; then
  [[ -n "$REMOTE_URL" ]] || die "--remote is required with --push"
  [[ "${LOCTREE_SYNC_CONFIRM:-}" == "1" ]] \
    || die "--push requires LOCTREE_SYNC_CONFIRM=1"
fi

# Parse component-manifests/<component>.manifest into the include/vendor/extra/
# exclude/remove arrays and the target-repo, licence, readme and dependency
# settings. An unrecognised line, or a missing required key, aborts the sync.
load_manifest() {
  local manifest="$1"
  local line key value

  [[ -f "$manifest" ]] || die "missing component manifest: $manifest"
  while IFS= read -r line || [[ -n "$line" ]]; do
    [[ -z "$line" || "$line" == \#* ]] && continue
    case "$line" in
      include=*)
        INCLUDES+=("${line#include=}") ;;
      vendor=*)
        VENDORS+=("${line#vendor=}") ;;
      extra=*)
        EXTRAS+=("${line#extra=}") ;;
      exclude=*)
        EXCLUDES+=("${line#exclude=}") ;;
      remove=*)
        REMOVALS+=("${line#remove=}") ;;
      *=*)
        key="${line%%=*}"
        value="${line#*=}"
        case "$key" in
          target_repo) TARGET_REPO="$value" ;;
          license) LICENSE_ID="$value" ;;
          readme_src) README_SRC="$value" ;;
          dependency_mode) DEPENDENCY_MODE="$value" ;;
          dependency_note) DEPENDENCY_NOTE="$value" ;;
        esac ;;
      *)
        die "invalid manifest line in $manifest: $line" ;;
    esac
  done < "$manifest"

  [[ -n "$TARGET_REPO" ]] || die "manifest missing target_repo"
  [[ -n "$LICENSE_ID" ]] || die "manifest missing license"
  [[ -n "$README_SRC" ]] || die "manifest missing readme_src"
  [[ -n "$DEPENDENCY_MODE" ]] || die "manifest missing dependency_mode"
}

# Recreate the staging directory from scratch, refusing to rm -rf the filesystem
# root, the suite root, the metadata root or the distribution directory itself.
safe_reset_staging() {
  STAGING=$(abs_path "$STAGING")
  [[ "$STAGING" != "/" ]] || die "refusing to use / as staging"
  [[ "$STAGING" != "$SUITE_ROOT" ]] || die "refusing to use suite root as staging"
  [[ "$STAGING" != "$METADATA_ROOT" ]] || die "refusing to use metadata root as staging"
  [[ "$STAGING" != "$SCRIPT_DIR" ]] || die "refusing to use distribution dir as staging"
  mkdir -p "$(dirname "$STAGING")"
  rm -rf "$STAGING"
  mkdir -p "$STAGING"
}

# Emit the --exclude flags every payload copy uses: the always-private paths plus
# each exclude= pattern the manifest declared.
rsync_excludes() {
  local pattern
  printf '%s\n' \
    --exclude='.git/' \
    --exclude='.loctree/' \
    --exclude='.vibecrafted/' \
    --exclude='target/' \
    --exclude='.DS_Store'
  for pattern in "${EXCLUDES[@]}"; do
    printf '%s\n' "--exclude=$pattern"
  done
}

# rsync one manifest src:dst directory mapping from the suite into staging with
# the shared exclude set applied. A missing source path aborts the sync.
copy_mapping() {
  local mapping="$1"
  local src="${mapping%%:*}"
  local dst="${mapping#*:}"
  local args=()
  local exclude_arg

  [[ -n "$src" && -n "$dst" && "$src" != "$dst" || "$mapping" == *:* ]] \
    || die "invalid include mapping: $mapping"
  [[ -e "$SUITE_ROOT/$src" ]] || die "missing include source: $src"

  mkdir -p "$STAGING/$dst"
  while IFS= read -r exclude_arg; do
    args+=("$exclude_arg")
  done < <(rsync_excludes)

  rsync -a "${args[@]}" "$SUITE_ROOT/$src/" "$STAGING/$dst/"
}

# Install every extra= single-file mapping into staging; include= is directory
# oriented, so registry/listing files travel in this lane.
copy_extra_files() {
  # Single-file mappings (registry/listing metadata like glama.json) — the
  # include= path is directory-oriented, so files get their own lane.
  [[ ${#EXTRAS[@]} -gt 0 ]] || return 0
  local mapping src dst
  for mapping in "${EXTRAS[@]}"; do
    src="${mapping%%:*}"; dst="${mapping#*:}"
    [[ -f "$METADATA_ROOT/$src" ]] || die "missing extra source file: $src"
    mkdir -p "$(dirname "$STAGING/$dst")"
    install -m 0644 "$METADATA_ROOT/$src" "$STAGING/$dst"
  done
}

# Copy the manifest's include= mappings and then its vendor= mappings into the
# staging tree.
copy_component_payload() {
  local mapping
  for mapping in "${INCLUDES[@]}"; do
    copy_mapping "$mapping"
  done
  [[ ${#VENDORS[@]} -gt 0 ]] || return 0
  for mapping in "${VENDORS[@]}"; do
    copy_mapping "$mapping"
  done
}

# Generate the mirror's root Cargo.toml: the workspace members this component
# publishes, the stamped release version and MSRV, and the engine dependencies
# resolved either as workspace-local paths (engine mirror) or crates.io versions.
write_workspace_cargo() {
  local output="$STAGING/Cargo.toml"
  local loctree_path loctree_ast_path report_path
  local members=()

  case "$DEPENDENCY_MODE" in
    "local workspace snapshot"|"crates.io registry") ;;
    *) die "unsupported dependency_mode: $DEPENDENCY_MODE" ;;
  esac

  case "$COMPONENT" in
    engine)
      # The public engine mirror ships the full bundle: engine + MCP + LSP
      # source in one workspace (operator decree — npm platform binaries,
      # Docker builds and "build from source" run against Loctree/loctree
      # alone, so all three binaries must build there).
      members=("loctree-ast" "loctree-rs" "loctree-mcp" "loctree-lsp")
      loctree_path="loctree-rs"
      loctree_ast_path="loctree-ast"
      if [[ "$DEPENDENCY_MODE" == "local workspace snapshot" ]]; then
        members+=("reports")
        report_path="reports"
      else
        report_path=""
      fi ;;
    mcp)
      members=("loctree-mcp")
      loctree_path="" loctree_ast_path="" report_path="" ;;
    lsp)
      members=("loctree-lsp")
      loctree_path="" loctree_ast_path="" report_path="" ;;
    *)
      die "internal error: unsupported component $COMPONENT" ;;
  esac

  {
    printf '[workspace]\n'
    printf 'resolver = "2"\n'
    printf 'members = [\n'
    for member in "${members[@]}"; do
      printf '    "%s",\n' "$member"
    done
    printf ']\n\n'
    printf '[workspace.package]\n'
    printf 'version = "%s"\n' "$VERSION"
    printf 'edition = "2024"\n'
    # Keep in lockstep with the suite root Cargo.toml — a lower stamped MSRV
    # makes mirror clippy (incompatible_msrv) reject code the suite accepts.
    printf 'rust-version = "1.93.0"\n'
    printf 'license = "%s"\n' "$LICENSE_ID"
    printf 'authors = ["LibraxisAI <support@loctree.com>"]\n\n'
    printf '[workspace.dependencies]\n'
    printf 'tokio = { version = "1.52", features = ["full"] }\n'
    printf 'serde = { version = "1.0", features = ["derive"] }\n'
    printf 'serde_json = "1.0"\n'
    printf 'anyhow = "1"\n'
    printf 'thiserror = "2"\n'
    printf 'clap = { version = "4.4", features = ["derive"] }\n'
    printf 'tracing = "0.1"\n'
    printf 'tracing-subscriber = "0.3"\n'
    printf 'rmcp = { version = "1.7", features = ["server"] }\n'
    printf 'futures = "0.3"\n'
    printf 'axum = { version = "0.8", features = ["json"] }\n'
    printf 'tower = { version = "0.5", features = ["util"] }\n'
    printf 'tokio-util = { version = "0.7", features = ["rt"] }\n'
    printf 'argon2 = "0.5"\n'
    printf 'password-hash = { version = "0.5", features = ["getrandom"] }\n'
    printf 'subtle = "2.6"\n'
    printf 'uuid = { version = "1", features = ["v4", "serde"] }\n'
    printf 'chrono = { version = "0.4", features = ["serde"] }\n'
    printf 'shellexpand = "3.1"\n'
    printf 'reqwest = { version = "0.12", default-features = false, features = ["json", "stream", "rustls-tls-native-roots"] }\n'
    printf 'sha2 = "0.10"\n'
    printf 'schemars = "1"\n'
    # aicx ships from crates.io in every mirror mode — loctree-rs inherits it
    # from the workspace (optional, feature aicx-inprocess is on by default).
    printf 'aicx = { version = "0.10.0", default-features = false, features = ["loctree-consumer"] }\n'
    if [[ -n "$loctree_path" ]]; then
      # engine mirror: workspace-local engine crates
      printf 'loctree = { path = "%s", version = "%s" }\n' "$loctree_path" "$VERSION"
      printf 'loctree-ast = { path = "%s", version = "%s" }\n' "$loctree_ast_path" "$VERSION"
      if [[ "$DEPENDENCY_MODE" == "local workspace snapshot" ]]; then
        printf 'report-leptos = { path = "%s", version = "%s" }\n' "$report_path" "$VERSION"
      else
        printf 'report-leptos = "%s"\n' "$VERSION"
      fi
    else
      [[ "$DEPENDENCY_MODE" == "crates.io registry" ]] \
        || die "$COMPONENT requires dependency_mode=crates.io registry"
      # component mirrors: engine crates from crates.io at the release version
      printf 'loctree = "%s"\n' "$VERSION"
      printf 'loctree-ast = "%s"\n' "$VERSION"
      printf 'report-leptos = "%s"\n' "$VERSION"
    fi
  } > "$output"
}

# Render the component README template into staging, substituting the component,
# version, target repo and dependency-mode placeholders.
render_readme() {
  local template="$METADATA_ROOT/$README_SRC"
  [[ -f "$template" ]] || die "missing README template: $README_SRC"
  python3 - "$template" "$STAGING/README.md" "$COMPONENT" "$VERSION" "$TARGET_REPO" \
    "$DEPENDENCY_MODE" "$DEPENDENCY_NOTE" <<'PY'
from pathlib import Path
import sys

template, output, component, version, target_repo, dependency_mode, dependency_note = sys.argv[1:]
text = Path(template).read_text(encoding="utf-8")
replacements = {
    "{{COMPONENT}}": component,
    "{{VERSION}}": version,
    "{{TARGET_REPO}}": target_repo,
    "{{DEPENDENCY_MODE}}": dependency_mode,
    "{{DEPENDENCY_NOTE}}": dependency_note,
}
for key, value in replacements.items():
    text = text.replace(key, value)
Path(output).write_text(text, encoding="utf-8")
PY
}

# Write the mirror's NOTICE.md: BUSL-1.1 for current releases and, for the engine
# mirror, the note that the 0.8.x line stays under its original MIT/Apache terms.
write_notice() {
  local output="$STAGING/NOTICE.md"
  {
    printf '# Notice\n\n'
    printf 'Published under BUSL-1.1. See `LICENSE` for the full text.\n\n'
    if [[ "$COMPONENT" == "engine" ]]; then
      printf 'Releases 0.13 and later are BUSL-1.1. The 0.8.x line was published under '
      printf 'MIT OR Apache-2.0; those releases remain available under their original terms.\n\n'
    fi
    printf 'Third-party attributions and dependency licenses: see `licenses/`.\n'
  } > "$output"
}

# Report the suite commit this snapshot was cut from, falling back to an archive
# marker when the suite root is not a git worktree.
source_commit() {
  if git -C "$SUITE_ROOT" rev-parse --is-inside-work-tree >/dev/null 2>&1; then
    git -C "$SUITE_ROOT" rev-parse --short=12 HEAD
  else
    printf 'archive:%s' "$(basename "$SUITE_ROOT")"
  fi
}

# Write SYNC-MANIFEST.md recording what this sync publishes: component, version,
# target repo, source commit, push mode, every payload mapping and every declared
# removal. It is the mirror's own record of where its contents came from.
write_sync_manifest() {
  local output="$STAGING/SYNC-MANIFEST.md"
  local commit generated mapping
  commit=$(source_commit)
  generated=$(date -u '+%Y-%m-%dT%H:%M:%SZ')
  {
    printf '# Component Sync Manifest\n\n'
    printf -- "- Component: \`%s\`\n" "$COMPONENT"
    printf -- "- Version: \`%s\`\n" "$VERSION"
    printf -- "- Target repo: \`%s\`\n" "$TARGET_REPO"
    printf -- "- Source commit: \`%s\`\n" "$commit"
    printf -- "- Generated at: \`%s\`\n" "$generated"
    printf -- "- Push mode: \`%s\`\n" "$(if [[ "$PUSH" == "1" ]]; then printf 'enabled'; else printf 'disabled'; fi)"
    printf -- "- Dependency mode: \`%s\`\n\n" "$DEPENDENCY_MODE"
    printf '## Included Payload\n\n'
    for mapping in "${INCLUDES[@]}"; do
      printf -- "- \`%s\` -> \`%s\`\n" "${mapping%%:*}" "${mapping#*:}"
    done
    if [[ ${#VENDORS[@]} -gt 0 ]]; then
      printf '\n## Vendored Build Payload\n\n'
      [[ ${#VENDORS[@]} -gt 0 ]] || return 0
  for mapping in "${VENDORS[@]}"; do
        printf -- "- \`%s\` -> \`%s\`\n" "${mapping%%:*}" "${mapping#*:}"
      done
    fi
    if [[ ${#REMOVALS[@]} -gt 0 ]]; then
      printf '\n## Paths Removed From The Mirror\n\n'
      for mapping in "${REMOVALS[@]}"; do
        printf -- "- \`%s\`\n" "$mapping"
      done
    fi
    printf '\n## Dependency Note\n\n%s\n' "$DEPENDENCY_NOTE"
  } > "$output"
}

# Install the distribution LICENSE into staging as the mirror's LICENSE.
copy_license() {
  install -m 0644 "$METADATA_ROOT/LICENSE" "$STAGING/LICENSE"
}

# Rewrite private absolute paths and internal artifact names out of every text
# file in staging, before anything can reach a public remote.
sanitize_private_markers() {
  python3 - "$STAGING" "$SUITE_ROOT" "$METADATA_ROOT" <<'PY'
from pathlib import Path
import sys

root = Path(sys.argv[1])
suite_root = sys.argv[2]
metadata_root = sys.argv[3]
replacements = {
    suite_root: "<private-suite-root>",
    metadata_root: "<private-suite-root>",
    "/Users/": "/home/",
    ".vibecrafted": "internal-artifacts",
    "loctree-fail": "loctree-feedback",
    "loctree-fail.md": "loctree-feedback-log",
}
skip_dirs = {".git", "target"}

for path in root.rglob("*"):
    if not path.is_file():
        continue
    if any(part in skip_dirs for part in path.parts):
        continue
    data = path.read_bytes()
    if b"\0" in data:
        continue
    try:
        text = data.decode("utf-8")
    except UnicodeDecodeError:
        continue
    updated = text
    for old, new in replacements.items():
        updated = updated.replace(old, new)
    if updated != text:
        path.write_text(updated, encoding="utf-8")
PY
}

# Fail the sync if any private marker or internal directory survived sanitisation.
# This is the last gate standing between staging and a public push.
scan_hard_excludes() {
  local hits
  hits=$(find "$STAGING" \( -path '*/.git/*' -o -path '*/target/*' \) -prune -o \
    -type f -print0 | xargs -0 grep -I -E '\.vibecrafted|loctree-fail|/Users/' || true)
  [[ -z "$hits" ]] || die "staging contains excluded markers:
$hits"
  [[ ! -d "$STAGING/.vibecrafted" ]] || die "staging contains excluded directory"
  [[ ! -d "$STAGING/.loctree" ]] || die "staging contains excluded directory"
  if find "$STAGING" -name 'loctree-fail.md' -print -quit | grep -q .; then
    die "staging contains loctree-fail.md"
  fi
}

# Resolve the generated workspace so the mirror ships a buildable Cargo.lock.
generate_lockfile() {
  cargo generate-lockfile --manifest-path "$STAGING/Cargo.toml"
}

# Paths the suite is authoritative for in the mirror: every manifest
# include/vendor/extra destination, every generated file, and every explicit
# `remove=` path. Anything NOT in this set belongs to the mirror alone and the
# sync must leave it untouched.
owned_paths() {
  local mapping
  for mapping in "${INCLUDES[@]}"; do
    printf '%s\n' "${mapping#*:}"
  done
  if [[ ${#VENDORS[@]} -gt 0 ]]; then
    for mapping in "${VENDORS[@]}"; do
      printf '%s\n' "${mapping#*:}"
    done
  fi
  if [[ ${#EXTRAS[@]} -gt 0 ]]; then
    for mapping in "${EXTRAS[@]}"; do
      printf '%s\n' "${mapping#*:}"
    done
  fi
  printf '%s\n' "${GENERATED_PATHS[@]}"
  if [[ ${#REMOVALS[@]} -gt 0 ]]; then
    printf '%s\n' "${REMOVALS[@]}"
  fi
}

# `remove=` declares a path the mirror must NOT carry. It never appears in the
# staging tree, so without this guard a stale copy on the remote would survive
# the preserving graft below forever.
assert_removals_absent_from_staging() {
  [[ ${#REMOVALS[@]} -gt 0 ]] || return 0
  local path
  for path in "${REMOVALS[@]}"; do
    [[ ! -e "$STAGING/$path" ]] \
      || die "manifest declares remove=$path but staging produced it"
  done
}

# Publish the staged snapshot to the mirror's main when push is enabled: graft it
# onto the fetched remote commit so mirror-owned paths survive, verify the deletes
# are declared, then push. Without --push this only reports the local snapshot.
maybe_push() {
  if [[ "$PUSH" != "1" ]]; then
    echo "push: disabled (default local snapshot mode)"
    return 0
  fi

  git -C "$STAGING" init -q
  git -C "$STAGING" remote add origin "$REMOTE_URL" 2>/dev/null || \
    git -C "$STAGING" remote set-url origin "$REMOTE_URL"
  # Public mirrors carry real history (the legacy line). A parentless snapshot
  # commit gets rejected as non-fast-forward and force-push would destroy that
  # history — so graft the snapshot ON TOP of the remote main when it exists.
  local parent=""
  if git -C "$STAGING" fetch -q --depth 1 origin main 2>/dev/null; then
    parent=$(git -C "$STAGING" rev-parse FETCH_HEAD)
  fi

  local tree commit
  if [[ -n "$parent" ]]; then
    # PRESERVING GRAFT (decision, 2026-08-18).
    #
    # The previous implementation wrote the tree from the staging directory
    # alone and committed it with remote main as parent. Because staging only
    # ever contains what the manifest lists, that commit DELETED every mirror
    # path the manifest does not own. Measured against Loctree/loctree
    # origin/main it would have removed 25 files: SECURITY.md, CHANGELOG.md,
    # CONTRIBUTING.md, .gitignore, the whole .github/ governance surface and
    # 10 CI workflows. A one-way source sync has no business deleting a public
    # repo's governance files, and "opt-in deletion with a confirmation prompt"
    # was rejected: the sync is meant to run unattended from a release runbook,
    # and a prompt only relocates a destructive default onto a tired operator.
    #
    # So the mirror's own paths are preserved by construction. The index starts
    # as the REMOTE tree; the suite-owned prefixes are dropped from it (so
    # stale files inside owned directories still disappear); the staging
    # content is then overlaid with --no-all, which never records a removal.
    # Deleting a mirror path is possible but must be declared: `remove=` in the
    # component manifest.
    git -C "$STAGING" read-tree "$parent"
    local owned
    while IFS= read -r owned; do
      [[ -n "$owned" ]] || continue
      # -f: the index was seeded from the remote tree and there is no HEAD, so
      # git refuses an unforced --cached removal. --cached never touches the
      # worktree, so forcing it cannot lose staged content.
      git -C "$STAGING" rm -r -q -f --cached --ignore-unmatch -- "$owned"
    done < <(owned_paths)
    # --no-all is load-bearing: plain `git add .` records worktree deletions,
    # which is exactly the bug this graft exists to prevent.
    git -C "$STAGING" add --no-all -- .
    tree=$(git -C "$STAGING" write-tree)
    commit=$(git -C "$STAGING" commit-tree "$tree" -p "$parent" \
      -m "release: Loctree $COMPONENT $VERSION source sync")
    assert_deletes_are_declared "$parent" "$commit"
  else
    git -C "$STAGING" add .
    tree=$(git -C "$STAGING" write-tree)
    commit=$(git -C "$STAGING" commit-tree "$tree" \
      -m "release: Loctree $COMPONENT $VERSION source sync")
  fi

  git -C "$STAGING" update-ref refs/heads/main "$commit"
  git -C "$STAGING" push origin main
}

# Belt and braces on the graft: print the change summary the operator is about
# to publish, and refuse the push if it would delete a path outside the
# suite-owned set. A silent governance-file wipe must be impossible even if the
# index plumbing above is later changed.
assert_deletes_are_declared() {
  local parent="$1" commit="$2"
  local deletes undeclared path owned matched

  echo "push: change summary against remote main ($parent)"
  git -C "$STAGING" diff --stat "$parent" "$commit" | tail -n 1

  deletes=$(git -C "$STAGING" diff --diff-filter=D --name-only "$parent" "$commit")
  [[ -n "$deletes" ]] || { echo "push: deletes 0"; return 0; }

  undeclared=""
  while IFS= read -r path; do
    [[ -n "$path" ]] || continue
    matched=0
    while IFS= read -r owned; do
      [[ -n "$owned" ]] || continue
      if [[ "$path" == "$owned" || "$path" == "$owned"/* ]]; then
        matched=1
        break
      fi
    done < <(owned_paths)
    if [[ "$matched" -eq 0 ]]; then
      undeclared+="$path"$'\n'
    fi
  done <<< "$deletes"

  [[ -z "$undeclared" ]] || die "refusing to push: sync would delete mirror-owned paths not covered by the manifest:
$undeclared"
  printf 'push: deletes (all inside suite-owned paths):\n%s\n' "$deletes"
}

load_manifest "$MANIFEST_DIR/$COMPONENT.manifest"
safe_reset_staging
copy_component_payload
copy_extra_files
write_workspace_cargo
copy_license
render_readme
write_notice
write_sync_manifest
sanitize_private_markers
scan_hard_excludes
assert_removals_absent_from_staging
generate_lockfile

echo "component: $COMPONENT"
echo "version:   $VERSION"
echo "target:    $TARGET_REPO"
echo "staging:   $STAGING"
echo "lockfile:  $STAGING/Cargo.lock"
maybe_push
