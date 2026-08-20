#!/bin/bash
# Sync version across release surfaces and hardcoded strings.
# Usage: ./scripts/sync-version.sh [new-version]
# If no version provided, reads from the workspace version in Cargo.toml.

set -e

SCRIPT_DIR="$(cd "$(dirname "$0")" && pwd)"
ROOT_DIR="$(dirname "$SCRIPT_DIR")"

# Get version from workspace Cargo.toml or argument
if [ -n "$1" ]; then
    VERSION="$1"
else
    VERSION=$(awk '
        /^\[workspace.package\]$/ { in_section=1; next }
        in_section && /^version = / { gsub(/"/, "", $3); print $3; exit }
    ' "$ROOT_DIR/Cargo.toml")
fi

echo "Syncing version to: $VERSION"

update_file() {
    local file="$1"
    local pattern="$2"

    if [ -f "$file" ]; then
        # BSD sed (macOS) requires an extension for -i, empty string '' works
        # GNU sed (Linux) treats '' as the filename if provided as a separate arg
        if sed --version 2>/dev/null | grep -q GNU; then
             sed -i "$pattern" "$file"
        else
             sed -i '' "$pattern" "$file"
        fi
        echo "  Updated: $file"
    else
        echo "  Skipped (not found): $file"
    fi
}

# Update lib.rs docs link
update_file "$ROOT_DIR/loctree-rs/src/lib.rs" 's|html_root_url = "https://docs.rs/loctree/[^"]*"|html_root_url = "https://docs.rs/loctree/'$VERSION'"|'

# The report footer reads CARGO_PKG_VERSION at compile time. Keeping the
# version dynamic avoids turning its format! invocation into a literal-only
# expression that fails the workspace's clippy -D warnings gate.

# Landing extracted to standalone repo at ../loct-io.
# Its own scripts/sync-version.sh handles VERSION + agent/index.json there.

# Keep the installer served by loct.io on the same release line. This used to
# be a manual release step and repeatedly left the public installer behind.
update_file "$ROOT_DIR/public_dist/install.sh" \
    's/LOCTREE_VERSION:-[0-9][0-9]*\.[0-9][0-9]*\.[0-9][0-9]*/LOCTREE_VERSION:-'$VERSION'/'

# Sync canonical npm release surface
if [ -f "$ROOT_DIR/distribution/npm/sync-version.mjs" ]; then
    if command -v node >/dev/null 2>&1; then
        node "$ROOT_DIR/distribution/npm/sync-version.mjs" "$VERSION"
        echo "  Updated: distribution/npm/package.json"
    else
        echo "Node.js is required to sync distribution/npm version" >&2
        exit 1
    fi
fi

# Sync editor distribution surfaces. VSIX and JetBrains ZIP are first-party
# packages, so they must carry the same suite version as the Rust workspace.
if [ -f "$ROOT_DIR/editors/vscode/package.json" ]; then
    python3 - "$VERSION" \
        "$ROOT_DIR/editors/vscode/package.json" \
        "$ROOT_DIR/editors/vscode/package-lock.json" <<'PY'
import json
import sys
from pathlib import Path

version = sys.argv[1]
for raw in sys.argv[2:]:
    path = Path(raw)
    if not path.exists():
        continue
    data = json.loads(path.read_text(encoding="utf-8"))
    data["version"] = version
    packages = data.get("packages")
    if isinstance(packages, dict) and isinstance(packages.get(""), dict):
        packages[""]["version"] = version
    path.write_text(json.dumps(data, indent=2) + "\n", encoding="utf-8")
    print(f"  Updated: {path}")
PY
fi

update_file "$ROOT_DIR/editors/jetbrains/gradle.properties" \
    's/^pluginVersion[[:space:]]*=.*/pluginVersion = '"$VERSION"'/'
update_file "$ROOT_DIR/editors/jetbrains/src/main/resources/messages/LoctreeBundle.properties" \
    's/v[0-9][0-9]*\.[0-9][0-9]*\.[0-9][0-9]*/v'"$VERSION"'/g'
update_file "$ROOT_DIR/editors/jetbrains/README.md" \
    's/v[0-9][0-9]*\.[0-9][0-9]*\.[0-9][0-9]*/v'"$VERSION"'/g'

echo ""
echo "Version sync complete: v$VERSION"
echo ""
echo "Verify with:"
echo "  grep -r 'v$VERSION\|$VERSION' --include='*.rs' --include='Cargo.toml' --include='package.json' $ROOT_DIR | grep -v target | grep -v '#'"
