# Loctree for VS Code

[![VS Code Marketplace](https://img.shields.io/visual-studio-marketplace/v/libraxis.loctree?label=VS%20Code%20Marketplace&color=c99a3b)](https://marketplace.visualstudio.com/items?itemName=libraxis.loctree)
[![Open VSX](https://img.shields.io/open-vsx/v/libraxis/loctree?label=Open%20VSX&color=3d7a72)](https://open-vsx.org/extension/libraxis/loctree)
![License](https://img.shields.io/badge/license-MIT%20wrapper%20%C2%B7%20BUSL--1.1%20engine-b86a5c)

One-shot full repository context for AI agents, powered by
[loctree](https://loct.io). The main surface is the **Loctree Context panel**:
one query box in your sidebar that answers literal occurrences, symbol bodies,
context packs, and blast-radius navigation directly from a pre-built structural
snapshot — no grepping, no re-reading the repo, no waiting on an LSP index.
Works in VS Code, Cursor, Windsurf, and other VS Code-compatible editors.

<!-- OPERATOR TODO: this README ships with zero images for a panel-centric
     product. Capture the shots below, save them under editors/vscode/media/,
     then uncomment the matching line in place.

1. editors/vscode/media/screenshot-context-panel.png
   The Loctree Context panel open in the sidebar mid-query, showing the
   Literal/Find/Body/Impact/Slice/Context Pack tabs — this is the "main
   surface" claim above, so it should be the first thing a visitor sees.
   ![Loctree Context panel](media/screenshot-context-panel.png)

2. editors/vscode/media/screenshot-findings.png
   The Findings explorer with a populated Health header and grouped findings
   (Dead Exports / Circular Imports / Twins / Hotspots).
   ![Findings explorer](media/screenshot-findings.png)

3. editors/vscode/media/screenshot-status-bar.png
   The status bar entry ($(type-hierarchy) Loctree Context) with the hover
   tooltip showing findings telemetry and a stale-snapshot indicator.
   ![Status bar](media/screenshot-status-bar.png)
-->

## Install

**Not yet on the VS Code Marketplace or Open VSX.** The extension builds and
tests cleanly on every commit (`.github/workflows/vscode-extension.yml`), but
no release has been published through the manual publish workflow yet — see
[docs/release/vscode-marketplace.md](../../docs/release/vscode-marketplace.md).
`code --install-extension libraxis.loctree` will fail until that happens.

Until then, install from a locally built VSIX:

```bash
cd editors/vscode
npm run package
code --install-extension loctree-<version>.vsix
```

Or download a platform-tagged VSIX artifact from a `VSCode Extension` GitHub
Actions run and install it via **Extensions → Install from VSIX…** in VS Code.

Once published, the intended install path is simply:

```bash
code --install-extension libraxis.loctree
```

The extension ships platform-specific builds for **macOS (Apple Silicon and
Intel)** and **Linux x64** — the matching `loctree-lsp` binary is bundled, so
there is nothing to download on first run. Windows and Linux arm64 are not
published yet.

## Features

- **Loctree Context panel**: Literal / Find / Body / Impact / Slice / Context Pack
  in one editor-native query surface. This is the main surface.
- **Literal Occurrence Search**: exact identifier-boundary occurrences across the
  codebase (`loct occurrences` / `find --literal`) — grep-precise, no fuzzy noise,
  presented as a navigable list with file, line, and occurrence kind.
- **Symbol Body**: pull the bounded source body of a function/method/symbol
  (`loct body`) into a syntax-highlighted preview, without grepping.
- **Context Pack**: bounded repository context cards for agent work, paginated from
  the LSP instead of pasted from stale files.
- **Findings Explorer**: secondary signal with grouped findings — Dead Exports,
  Circular Imports, Twins, and Hotspots.
- **Blast-radius Navigation**: analyze change impact, find consumers/importers, and
  show a file slice (dependencies + consumers).
- **Code Actions**: loctree-specific quick fixes on diagnostics — cycle and
  dead-export fixes, plus "Open Context Atlas card". Additive to your language
  server; Loctree does not provide hover, go-to-definition, or find-references
  (those stay with your real language server, which resolves them semantically).
- **Rich Status Bar**: a Context entry point with stale-snapshot and findings signal
  telemetry. Click it to run a Context Query.

All data comes live from the `loctree-lsp` server (which loads the snapshot from
loctree's cache) — the extension does not depend on files in your workspace.

## Requirements

The extension bundles `loctree-lsp` and also downloads the right per-platform binary
on first activation (when `loctree.autoDownload` is on), so no manual install is
required to get started. To produce analysis, scan your project once with the CLI:

1. Install the `loct` CLI (prebuilt binary — no Rust toolchain required):
   ```bash
   # Recommended: signed bundle (loct + loctree + loctree-mcp + aicx + aicx-mcp)
   curl -fsSL https://loct.io/install.sh | bash

   # Or via npm (CLI only)
   npm install -g @loctree/loctree

   # Or via Homebrew
   brew install loctree/cli/loct
   ```
   Smoke-test: `loct --version`.

2. Scan your project:
   ```bash
   cd your-project
   loct
   ```
   Artifacts land in your OS cache dir by default (override via `LOCT_CACHE_DIR`).

The extension activates automatically when a `.loctree/` folder is detected, on
startup, or when you run any Loctree command. On activation it starts
`loctree-lsp` immediately with `--root <workspace>` and the workspace as the
server process working directory, so sidebar/status/custom requests have a root
before any source file is opened.

The runtime resolver uses: configured path → bundled binary → verified cache /
download → `~/.local/bin/loctree-lsp` → `PATH`. Hovering the status entry shows
the exact executable path, its full `--version` build identity, and the resolver
source. If an older Cargo/Homebrew copy appears earlier on `PATH` than the
preferred `~/.local/bin` install, Loctree uses the preferred runtime and emits a
visible PATH-shadowing warning with both paths and identities.

## Usage

### Commands

| Command | What it does |
|---------|--------------|
| **Loctree: Initialize/Scan** | Start the LSP against the current workspace and load/build its snapshot |
| **Loctree: Search Literal Occurrences** | Exact identifier-boundary search; pick a result to jump to it |
| **Loctree: Show Symbol Body** | Open the bounded source body of a symbol |
| **Loctree: Focus Context Pill** | Main Context-King router: Literal / Find / Body / Impact / Slice / Context Pack |
| **Loctree: Copy Agent Context** | Copy the current context pack to the clipboard, ready to paste into an agent |
| **Loctree: Show Findings Health** | Secondary findings report (score, top risks, recommended actions) |
| **Loctree: Analyze Change Impact** | Blast radius — what breaks if you change a file |
| **Loctree: Find Consumers / Find Importers** | Files that depend on the target |
| **Loctree: Show File Slice** | A file's dependencies + consumers |
| **Loctree: Show Circular Imports** | Project cycles |
| **Loctree: Check Dead Exports** | Unused exports |
| **Loctree: Refresh Analysis** | Ask the server to re-read the snapshot |
| **Loctree: Open HTML Report** | Open the interactive report (if generated) |

`Search Literal Occurrences` and `Show Symbol Body` seed from the symbol under the
cursor and are available from the Command Palette.

### Status Bar

`$(type-hierarchy) Loctree Context` is the Context-King entry point. A history
glyph means the snapshot is stale relative to HEAD. Hover for findings telemetry;
click to focus the Context Pill.

### Settings

| Setting | Description | Default |
|---------|-------------|---------|
| `loctree.serverPath` | Path to loctree-lsp binary | (auto-detect) |
| `loctree.autoRefresh` | Refresh on file save | `false` |
| `loctree.autoScanOnStartup` | Scan workspaces without a Loctree signal on startup | `false` |
| `loctree.showStatusBar` | Show status in status bar | `true` |
| `loctree.autoDownload` | Auto-download loctree-lsp | `true` |
| `loctree.downloadBaseUrl` | Override repo URL for downloads | (empty) |
| `loctree.downloadTag` | Release tag to download | (this extension's exact version tag) |
| `loctree.diagnosticSeverity` | Severity for dead exports | `warning` |
| `loctree.codeLens` | Show Loctree inline code lenses | `false` |

## Supported Languages

TypeScript / JavaScript / TSX / JSX, Rust, Python, Go (literal search and body work
across loctree's full language coverage).

## Development

```bash
# from the repository root
make editors-vscode
```

Press `F5` to launch the Extension Development Host. The bundled `loctree-lsp`
binary (`bin/`) is a build artifact — `scripts/prepare-bins.js` copies
`LOCTREE_LSP_PATH` when set, otherwise runs `cargo build -p loctree-lsp --release`
and bundles `../../target/release/loctree-lsp`, so it is not committed.

## Packaging (VSIX)

```bash
cd editors/vscode
npm run package
```

The packager fails closed if it cannot bundle `loctree-lsp`. Set
`LOCTREE_LSP_PATH` only when you intentionally want to package a specific server
binary.

## License

This extension is MIT-licensed; the Loctree language server it installs is
licensed under BUSL-1.1.

MIT — 𝚅𝚒𝚋𝚎𝚌𝚛𝚊𝚏𝚝𝚎𝚍. with AI Agents by Vetcoders (c)2024-2026 LibraxisAI
