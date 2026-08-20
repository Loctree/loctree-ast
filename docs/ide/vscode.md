# VSCode Extension

The Loctree VSCode extension provides real-time dead code detection, circular import warnings, and code navigation powered by the `loctree-lsp` language server.

## Installation

The extension needs the `loctree-lsp` binary on `PATH`. In 0.13.0 the signed bundle and the `@loctree/loct` runtime package (legacy scoped name; `@loctree/loctree` from 0.14.2) both ship it:

```bash
npm install -g @loctree/loctree
# or
curl -fsSL https://loct.io/install.sh | bash
```

Smoke-test: `loctree-lsp --version`. Then add the extension itself by one of the paths below.

### Marketplace

The extension is not yet published to the VS Code Marketplace; track [Loctree/loctree-suite #vscode-marketplace](https://github.com/Loctree/loctree-suite/issues) for status. Until it lands, use the source / VSIX paths below — they are the supported install paths today.

### From Source (contributors / forks)

```bash
cd editors/vscode
npm install
npm run compile
```

Then in VS Code: `F1` → "Developer: Install Extension from Location" → select `editors/vscode`.

### VSIX (recommended for forks like Cursor / Windsurf)

```bash
cd editors/vscode
LOCTREE_LSP_PATH=/path/to/loctree-lsp npm run package
```

This bundles the `loctree-lsp` binary into `editors/vscode/bin/`. If `LOCTREE_LSP_PATH` is not set, the packager looks for `../../target/release/loctree-lsp` or an existing `loctree-lsp` in PATH.

## Features

### Diagnostics

The extension shows warnings directly in your editor:

| Diagnostic | Severity | Description |
|------------|----------|-------------|
| Dead Export | Warning | Export has 0 imports across codebase |
| Circular Import | Warning | File is part of an import cycle |
| Twin Symbol | Information | Symbol exported from multiple files |

### Hover Information

Hover over any export to see:
- Import count across the codebase
- Top consumer files
- Export location details

### Go to Definition

`F12` or `Ctrl+Click` on imports to jump to:
- Original export location
- Re-export chain resolution
- Cross-language definitions (TS → Rust for Tauri)

### Code Actions

`Ctrl+.` on diagnostics to access quick fixes:
- **Remove unused export** - Delete the export keyword
- **Add to .loctignore** - Suppress this warning
- **Show in HTML report** - Open detailed analysis

## Configuration

In VSCode settings (`Ctrl+,`):

```json
{
  "loctree.serverPath": "/custom/path/to/loctree-lsp",
  "loctree.autoRefresh": false,
  "loctree.trace.server": "verbose"
}
```

| Setting | Default | Description |
|---------|---------|-------------|
| `serverPath` | auto-detect | Path to loctree-lsp binary |
| `autoRefresh` | `false` | Re-scan on file save |
| `autoDownload` | `true` | Download loctree-lsp if missing |
| `downloadBaseUrl` | (empty) | Override repo URL for downloads |
| `downloadTag` | `latest` | Release tag for downloads |
| `trace.server` | `off` | LSP message logging |

## Status Bar

The status bar shows loctree status:

- 🌳 **Loctree: healthy** - No issues detected
- 🌳 **Loctree: 5 dead** - Number of dead exports
- 🌳 **Loctree: loading** - Scanning in progress

Click to open the Output panel for details.

## Commands

Open command palette (`F1`) and search for "Loctree":

| Command | Description |
|---------|-------------|
| `Loctree: Refresh` | Re-run `loct` and update diagnostics |
| `Loctree: Open Report` | Open HTML report in browser |
| `Loctree: Show Health` | Display health score summary |

## Requirements

- `loctree-lsp` reachable from `PATH`. Easiest install: `npm install -g @loctree/loctree` or `curl -fsSL https://loct.io/install.sh | bash`; alternatively rely on the VSIX-bundled binary (see _VSIX_ above).
- Project must have a Loctree snapshot (run `loct` once in the repo root)

## Troubleshooting

### No diagnostics appearing

1. Check Output panel → "Loctree" for errors
2. Ensure `.loctree/snapshot.json` exists
3. Run `loct` in project root

### Server not starting

```bash
# Check if loctree-lsp is in PATH
which loctree-lsp

# Or set custom path in settings
"loctree.serverPath": "/path/to/loctree-lsp"
```

### Stale diagnostics

Click status bar → "Loctree: Refresh" or run `loct` in terminal.

---

*𝚅𝚒𝚋𝚎𝚌𝚛𝚊𝚏𝚝𝚎𝚍. with AI Agents by Vetcoders (c)2024-2026 LibraxisAI*
