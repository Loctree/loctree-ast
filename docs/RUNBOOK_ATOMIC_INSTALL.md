# Atomic Loctree CLI + MCP install runbook

This is the operator button for installing `loct`, `loctree`, and
`loctree-mcp` as one versioned bundle. It deliberately does not run as part of
CI or the trust-control pack.

The atomic unit is one `current` symlink. Three independent copies or renames
are not atomic as a set and can expose a mixed CLI/MCP generation.

## 1. Build and prove one checkout

Start from a clean, reviewed commit on the intended branch:

```bash
git status --short --branch
git rev-parse HEAD
cargo build --release -p loctree --bin loct --bin loctree \
  -p loctree-mcp --bin loctree-mcp
bash tools/trust-controls/run.sh
```

Record all three markers. They must share `schema=loctree.bundle.v1`,
`bundle_id`, and `commit`:

```bash
for bin in loct loctree loctree-mcp; do
  "target/release/$bin" --version
done
```

Stop if any marker differs or the control pack fails.

## 2. Stage an immutable bundle

Choose an operator-owned prefix. The example uses the existing local install
root without modifying it until the explicit copy commands:

```bash
PREFIX="$HOME/.local/share/loctree"
BUNDLE_ID=$(target/release/loct --version | awk '{for (i=1;i<=NF;i++) if ($i ~ /^bundle_id=/) {sub(/^bundle_id=/,"",$i); print $i}}')
STAGE="$PREFIX/bundles/$BUNDLE_ID"
install -d -m 0755 "$STAGE/bin"
for bin in loct loctree loctree-mcp; do
  install -m 0755 "target/release/$bin" "$STAGE/bin/$bin"
done
```

Verify the staged paths, not `PATH`:

```bash
for bin in loct loctree loctree-mcp; do
  "$STAGE/bin/$bin" --version
done
LOCT_TRUST_BIN_DIR="$STAGE/bin" bash tools/trust-controls/run.sh
```

Do not alter a previously staged bundle. A changed build gets a new bundle
directory.

## 3. Bootstrap stable entrypoints once

This migration is a maintenance-window operation because creating the first
three stable links is not a multi-file atomic action. Stop MCP clients first.

```bash
install -d -m 0755 "$HOME/.local/bin" "$PREFIX"
ln -sfn "$STAGE" "$PREFIX/current.next"
mv -fh "$PREFIX/current.next" "$PREFIX/current"
for bin in loct loctree loctree-mcp; do
  ln -sfn "$PREFIX/current/bin/$bin" "$HOME/.local/bin/$bin"
done
```

Confirm that the three stable links resolve through `current`, then restart MCP
clients. Never replace only one of these links during an upgrade.

## 4. Atomic upgrade

After staging and proving the new immutable bundle, switch one directory entry:

```bash
ln -sfn "$STAGE" "$PREFIX/current.next"
mv -fh "$PREFIX/current.next" "$PREFIX/current"
```

On macOS, `mv -h` is essential: it replaces the `current` symlink itself
instead of following that symlink and moving `current.next` into the old bundle.

Existing processes may continue running the old executable they already
opened. Restart Codex, Claude, IDE, and standalone MCP processes so every new
process resolves the same bundle.

Verify runtime truth:

```bash
for bin in loct loctree loctree-mcp; do
  command -v "$bin"
  "$bin" --version
done
LOCT_TRUST_BIN_DIR="$PREFIX/current/bin" bash tools/trust-controls/run.sh
```

## 5. Roll back

Point `current` at the last known-good immutable directory using the same
single-symlink switch, restart MCP clients, and repeat the runtime verification.
Keep the failed bundle and its trust-control evidence until the incident is
understood; do not silently overwrite it.
