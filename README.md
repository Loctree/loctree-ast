<h1 align="center">𝙻𝚘𝚌𝚝𝚛𝚎𝚎</h1>
<p align="center">

  <img src="https://loctree.com/og-image.png" width="512" alt="loctree logo"/>
</p>

<p align="center">
  <strong>Scan once, query everything.</strong><br/>
  AI-oriented static analysis for dead exports, circular imports, dependency graphs, and holographic context slices.
</p>

<p align="center">
  <a href="https://crates.io/crates/loctree"><img src="https://img.shields.io/crates/v/loctree.svg" alt="crates.io"/></a>
  <a href="https://crates.io/crates/loctree"><img src="https://img.shields.io/crates/d/loctree.svg" alt="downloads"/></a>
  <a href="https://docs.rs/loctree"><img src="https://docs.rs/loctree/badge.svg" alt="docs.rs"/></a>
  <a href="https://github.com/Loctree/loctree-suite/actions/workflows/ci.yml"><img src="https://github.com/Loctree/loctree-suite/actions/workflows/ci.yml/badge.svg" alt="CI"/></a>
  <a href="LICENSE"><img src="https://img.shields.io/badge/License-BUSL--1.1-blue.svg" alt="License"/></a>
</p>

---

## Install

```bash
curl -fsSL https://loct.io/install.sh | bash
```

Alternative package-manager paths:

```bash
cargo install --locked loctree       # crates.io: core analyzer + loct/loctree CLIs
cargo add loctree@0.13.1             # library dependency for Rust integrations
npm install -g @loctree/loctree      # canonical runtime package; loct/loctree plus MCP/LSP siblings
```

> **npm naming.** One wrapper is published under three maintained identities:
> canonical `@loctree/loctree`, established scoped alias `@loctree/loct`, and
> short form `loctree`. They resolve the same four
> `@loctree/loctree-*` platform packages and expose the same commands. See
> [`distribution/npm/PUBLISHING.md`](distribution/npm/PUBLISHING.md) for the
> seven-package release and trusted-publishing contract.

Homebrew: the `loctree/cli` tap repository exists but currently ships **no
formula** — `brew install loctree/cli/loct` will fail with "no available
formula" today. Track [Loctree/homebrew-cli](https://github.com/Loctree/homebrew-cli)
for when the formula lands; until then, use the `curl | bash` installer or the
npm path above.

Run the stdio MCP server without a global npm install:

```bash
npx -y --package=@loctree/loctree loctree-mcp
```

Or build the Glama-compatible container from this checkout and mount the
project being analyzed at `/workspace`:

```bash
docker build -t loctree-mcp .
docker run --rm -i \
  --mount type=bind,src="$PWD",dst=/workspace,readonly \
  --mount type=volume,src=loctree-cache,dst=/data \
  loctree-mcp
```

> Version note: this tree is at **0.14.4**. See
> [docs/release/README.md](docs/release/README.md) for the release procedure.

Manual combined-bundle download with checksum and GPG verification:

```bash
version=0.13.1
case "$(uname -s)-$(uname -m)" in
  Darwin-arm64) asset="loctree-${version}-aarch64-apple-darwin.tar.gz" ;;
  Linux-x86_64) asset="loctree-${version}-x86_64-unknown-linux-gnu.tar.gz" ;;
  Linux-x86_64-musl) asset="loctree-${version}-x86_64-unknown-linux-musl-core.tar.gz" ;;
  *) echo "unsupported platform: $(uname -s)-$(uname -m)" >&2; exit 1 ;;
esac

base="https://github.com/Loctree/loctree-suite/releases/download/v${version}"

# For private releases, export GITHUB_TOKEN first. For public releases, remove
# the Authorization header.
curl -fL -H "Authorization: Bearer ${GITHUB_TOKEN:?}" -O "${base}/${asset}"
curl -fL -H "Authorization: Bearer ${GITHUB_TOKEN:?}" -O "${base}/${asset}.sha256"
curl -fL -H "Authorization: Bearer ${GITHUB_TOKEN:?}" -O "${base}/${asset}.sig"

shasum -a 256 -c "${asset}.sha256"
gpg --verify "${asset}.sig" "${asset}"

case "$asset" in
  *.zip) unzip "$asset" ;;
  *.tar.gz) tar -xzf "$asset" ;;
esac
```

Maintainer macOS signing and notarization profile:

```bash
set -a
source "$HOME/.keys/.notary.env"
set +a

xcrun notarytool store-credentials "${NOTARY_PROFILE:-vc-notary}" \
  --apple-id "$NOTARY_APPLE_ID" \
  --team-id "$NOTARY_TEAM_ID" \
  --password "$NOTARY_PASSWORD"
```

Source builds are contributor fallback only; see [docs/dev/01_installation.md](docs/dev/01_installation.md).

## Quick Start

Artifacts are stored in your OS cache dir by default (override via `LOCT_CACHE_DIR`).

```bash
loct                              # Scan project, write cached artifacts
loct --for-ai                     # AI-optimized overview (health, hubs, quick wins)
loct slice src/App.tsx --consumers # Context: file + deps + consumers
loct find useAuth                  # Find symbol definitions
loct find 'Snapshot FileAnalysis'  # Cross-match: where terms meet
loct impact src/utils/api.ts       # What breaks if you change this?
loct health                        # Quick summary: cycles + dead + twins
loct dead --confidence high        # Unused exports
loct cycles                        # Circular imports
loct twins                         # Dead parrots + duplicates + barrel chaos
loct audit                         # Full codebase review
```

## What It Does

loctree captures your project's real dependency graph in a single scan, then answers structural questions instantly from the snapshot. Designed for AI agents that need focused context without reading every file.

**Core capabilities:**

- **Holographic Slice** - extract file + dependencies + consumers in one call
- **Cross-Match Search** - find where multiple terms co-occur (not flat grep)
- **Dead Export Detection** - find unused exports across JS/TS, Python, Rust, Go, Dart, Shell, Make, Zig
- **Circular Import Detection** - Tarjan's SCC algorithm catches runtime bombs
- **Handler Tracing** - follow Tauri commands through the entire FE/BE pipeline
- **Impact Analysis** - see what breaks before you delete or refactor
- **jq Queries** - query snapshot data with jq syntax (`loct '.files | length'`)

## Why loctree

| | grep/rg | LSP | loctree |
|---|---------|-----|---------|
| Knows imports vs definitions | No | Per-file | Whole graph |
| Dead export detection | No | No | Yes (multi-lang) |
| Cross-file impact analysis | No | Limited | Full transitive |
| AI agent integration | No | No | MCP server + `--for-ai` |
| Speed on 1M LOC repo | Fast (text) | Slow (indexing) | **~3s** (structural) |
| Setup | None | Per-editor | One binary |

## MCP Server

`loctree-mcp` is an MCP server for AI agent integration:

```bash
loctree-mcp    # Start via stdio (configure in your MCP client)
```

12 tools: `context`, `repo-view`, `focus`, `slice`, `body`, `find`, `impact`, `diff`, `tree`, `follow`, `suppressions`, `prism`. `find` includes raw-text regex mode with coverage accounting; `tree` is unlimited-depth by default, matching the CLI. Each tool reads the same snapshot and accepts a `project` parameter — auto-scans on first use, caches snapshots in RAM. Project-agnostic: analyze any repo without configuration.

```json
{
  "mcpServers": {
    "loctree": {
      "command": "loctree-mcp",
      "args": []
    }
  }
}
```

Signed release tarballs for direct download (`loct`, `loctree`, `loctree-mcp`, `loctree-lsp`, `aicx`, `aicx-mcp`) are mirrored through the loct.io releases CDN.

## LSP Server

`loctree-lsp` is a Language Server Protocol server. Editors get live structural diagnostics — dead exports, cycles, twins, codelens importer counts — plus **15 snapshot-backed `loctree/*` custom requests** for agent/editor context: `contextAtlas`, paginated `contextPack`, `slice`, `impact`, `find`, `follow`, `body`, `symbolContext`, `diff`, `semantic`, `health`, `workspaces`, `refresh`, read-only `aicx`, and the one live-AST request `astQuery` (JS/TS/TSX). Snapshot truth is the authority; live AST is an editor-time freshness layer. See [docs/integrations/lsp-server.md](docs/integrations/lsp-server.md).

```bash
loctree-lsp --version    # smoke test
```

Wire-up examples for VS Code, Neovim, JetBrains, and any generic LSP client live in [docs/ide/](docs/ide/) and [editors/jetbrains](editors/jetbrains). `loctree-lsp` ships as part of the single `@loctree/loctree` npm install.

## Editor Plugins

Two first-party editor plugins are built in this repo and versioned in lockstep
with the CLI (currently 0.14.2):

| Plugin | Source | Build |
|--------|--------|-------|
| VS Code extension | [editors/vscode](editors/vscode) | `make editors-vscode-package` → `.vsix` |
| JetBrains plugin (IntelliJ platform) | [editors/jetbrains](editors/jetbrains) | `make editors-jetbrains` → plugin `.zip` |

Both drive `loctree-lsp` and surface the Context Atlas, slice/impact, and
structural diagnostics inside the IDE.

**Not on the marketplaces yet.** Neither plugin is currently listed on the VS
Code Marketplace or the JetBrains Plugin Repository. Manual-trigger publish
workflows exist (`vscode-publish.yml`, `jetbrains-publish.yml`) but no release
has been pushed through them. Install locally from a built artifact —
`make editors-jetbrains-install` for IntelliJ, or install the packaged `.vsix`
via *Extensions → Install from VSIX…* in VS Code.

Neovim users configure `loctree-lsp` directly; the shipped
[editors/nvim/loctree.lua](editors/nvim/loctree.lua) prefers the canonical user
install over stale PATH shadows and exposes the active path/build identity via
`:LoctreeRuntime`.

## Language Support

Loctree uses regex parsers for shell, makefile, zig, dart, go and OXC for JS/TS. Tree-sitter provides Layer 1 extraction for C-family languages (Swift, Objective-C, C, C++) with heuristic provenance; deep mode is available as an opt-in.

The full language-support details live in [docs/semantic-spec.md](docs/semantic-spec.md).

Auto-detects stack from `Cargo.toml`, `tsconfig.json`, `pyproject.toml`, `pubspec.yaml`, `src-tauri/`.

## Holographic Slice

Extract 3-layer context for any file:

```bash
loct slice src/App.tsx --consumers
```

```
Slice for: src/App.tsx

Core (1 files, 150 LOC):
  src/App.tsx (150 LOC, ts)

Deps (3 files, 420 LOC):
  [d1] src/hooks/useAuth.ts (80 LOC)
    [d2] src/contexts/AuthContext.tsx (200 LOC)
    [d2] src/utils/api.ts (140 LOC)

Consumers (2 files, 180 LOC):
  src/main.tsx (30 LOC)
  src/routes/index.tsx (150 LOC)

Total: 6 files, 750 LOC
```

## Cross-Match Search

Multi-term queries show where terms meet, not flat OR:

```bash
loct find 'Snapshot FileAnalysis'
```

```
=== Cross-Match Files (9) ===
  src/snapshot.rs: Snapshot(6), FileAnalysis(4)
  src/slicer.rs: Snapshot(2), FileAnalysis(3)
  ...

=== Symbol Matches (222 in cross-match files) ===
  src/snapshot.rs:20 - Snapshot [struct]
  src/types.rs:15 - FileAnalysis [struct]
  ...

=== Parameter Matches (4 cross-matched) ===
  src/slicer.rs:45 - snapshot: &Snapshot in build_slice(analyses: &[FileAnalysis])
```

## jq Queries

Query snapshot data directly:

```bash
loct '. | keys'                                # Snapshot surface
loct '.files | length'                         # Count files
loct '.edges[] | select(.from | contains("api"))' # Filter edges
loct '.metadata.languages'                     # Language surface

# Dead code, cycles and health are findings, not snapshot keys:
loct follow dead --json                        # Dead exports (with reasons)
loct follow cycles --json                      # Circular imports, classified
```

## CI Integration

```bash
loct lint --fail --sarif > results.sarif    # SARIF for GitHub/GitLab
loct --findings | jq '.dead_parrots | length'  # Check dead code count
loct doctor && echo 'Clean'                 # Health gate
```

## Crates

End users install prebuilt binaries (see [Install](#install)). The crates table is for contributors and packagers.

| Crate | Description |
|-------|-------------|
| [`loctree`](https://crates.io/crates/loctree) | Core analyzer + CLI binaries (`loct`, `loctree`) |
| [`loctree-mcp`](https://crates.io/crates/loctree-mcp) | MCP server (`loctree-mcp` binary) for AI agents; also ships via bundle/npm/Docker/Homebrew |
| `loctree-lsp` (in-tree, `publish = false`) | LSP server (`loctree-lsp` binary) for editors; ship via bundle/npm/editor packages, not crates.io |
| [`report-leptos`](https://crates.io/crates/report-leptos) | HTML report renderer (Leptos SSR) |

## Development

For contributors building from source. End users should use the [Install](#install) paths above instead.

```bash
make precheck        # fmt + clippy + check (run before push)
make preflight       # Full explicit validation before a PR or release
make semgrep         # Security gate (same rules as CI)
make git-hooks       # Explicitly install a committed lightweight-hook snapshot
make install         # Build + install loct, loctree, loctree-mcp into ~/.local/bin
make install-all     # As above, plus loctree-lsp
make test            # Run all workspace tests
make publish         # Cascade publish to crates.io (release engineers only)
```

Override the install prefix with `CARGO_INSTALL_ROOT=/usr/local make install`.
Binary installation does not change Git hook policy. `make git-hooks` stores a
source-commit-addressed hook snapshot under the common Git directory, shared by
linked worktrees, and refuses to overwrite a foreign hook configuration.
Release engineers: the full ordering lives in [docs/release/README.md](docs/release/README.md).

Full contributor setup, workspace structure, and dependencies: [docs/dev/01_installation.md](docs/dev/01_installation.md).

## Badge

```markdown
[![loctree](https://img.shields.io/badge/analyzed_with-loctree-a8a8a8?style=flat&logo=data:image/svg+xml;base64,PHN2ZyB4bWxucz0iaHR0cDovL3d3dy53My5vcmcvMjAwMC9zdmciIHdpZHRoPSIxNiIgaGVpZ2h0PSIxNiIgdmlld0JveD0iMCAwIDE2IDE2Ij48cmVjdCB3aWR0aD0iMTYiIGhlaWdodD0iMTYiIGZpbGw9IiMwMDAiLz48dGV4dCB4PSI4IiB5PSIxMiIgZm9udC1mYW1pbHk9Im1vbm9zcGFjZSIgZm9udC1zaXplPSIxMCIgZmlsbD0iI2E4YThhOCIgdGV4dC1hbmNob3I9Im1pZGRsZSI+TDwvdGV4dD48L3N2Zz4=)](https://crates.io/crates/loctree)
```

## License

Business Source License 1.1 (BUSL-1.1). See [LICENSE](LICENSE) for the
parameters (Licensor, Additional Use Grant, Change Date, Change License).
Loctree converts to Apache License 2.0 on 2030-04-13.

For commercial / hosted-service licensing outside the Additional Use Grant,
contact `support@loctree.com`.

---

[Marbles OS](https://vibecrafted.io) by [vetcoders](https://vetcoders.io).
