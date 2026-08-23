# LSP Server Integration

`loctree-lsp` exposes Loctree's structural code intelligence to editor clients
through the Language Server Protocol over stdio.

The LSP is the **editor-aware surface** in the Loctree flow. It is not a separate
analyzer: it serves the same snapshot-backed structural truth as the CLI and the
MCP server, and adds a **live AST freshness layer** for unsaved buffers where the
language is supported.

```text
Repo checkout → snapshot authority → structural / context pack → CLI / MCP / LSP → editor surfaces
```

## Where the LSP sits

- **Snapshot truth is the default authority.** Most `loctree/*` requests read
  straight from the snapshot — the same durable, rebuildable map the CLI and MCP
  read.
- **Live AST is an editor-time freshness layer.** When a JS/TS/TSX buffer is
  open and unsaved, `loctree-lsp` parses it incrementally via `loctree-ast`
  (tree-sitter substrate) so the editor sees structure ahead of the next
  snapshot rebuild.
- **`astQuery` is structural query support**, not the product. It is one of the
  custom requests below — a tree-sitter query surface, not the Loctree
  differentiator.

The differentiator remains snapshot-first structural authority. Live AST,
tree-sitter, and `astQuery` are exposure and freshness mechanisms over that
authority — none of them replaces the snapshot-first model.

## Architecture

```text
VS Code / JetBrains / Neovim / generic LSP client
    |
    v
loctree-lsp
    |
    |- standard LSP diagnostics (dead exports, cycles, twins, codelens importer counts)
    |- snapshot-backed custom requests (read the same snapshot as loct / loctree-mcp)
    |- live-AST custom request (loctree-ast: JS/TS/TSX unsaved-buffer freshness)
    |- read-only AICX overlay
    `- LSP bridge for the editor surfaces in ./editors
```

## Installation

`loctree-lsp` ships as part of the single `@loctree/loctree` npm install and in the
signed combined bundle. It is an in-tree crate with `publish = false`, so do not
install it from crates.io. See [docs/installation.md](../installation.md) for the
full menu.

Smoke-test:

```bash
loctree-lsp --version
```

## Standard LSP features

Editors get live structural diagnostics with zero per-file ceremony:

- dead exports
- circular imports (cycles)
- duplicate exports / route twins
- CodeLens importer counts on definitions

## Custom request surface

These are **LSP custom requests** (JSON-RPC `loctree/*` methods registered in
`loctree-lsp/src/lib.rs`), not CLI commands. A host runtime may re-expose some of
them as tools; named here they are LSP methods.

| Custom request | Backing | Use |
| --- | --- | --- |
| `loctree/refresh` | control | Trigger a rescan / snapshot refresh. |
| `loctree/contextAtlas` | snapshot | Materialize Context Atlas card pointers for the workspace. |
| `loctree/contextPack` | snapshot | Paginated agent context pack (structural + runtime + risk + action + memory + authority). |
| `loctree/slice` | snapshot | File + dependencies + consumers before editing. |
| `loctree/impact` | snapshot | Direct + transitive consumers (blast radius) before delete/refactor. |
| `loctree/find` | snapshot | Symbol / import / literal search. |
| `loctree/follow` | snapshot | Structural signals: dead, cycles, twins, hotspots, pipelines. |
| `loctree/body` | snapshot | Definition body of a symbol. |
| `loctree/symbolContext` | snapshot | Symbol identity, parent, occurrences, position. |
| `loctree/diff` | snapshot | Structural diff between snapshot states. |
| `loctree/semantic` | snapshot | Runtime semantic facts / idiom tags. |
| `loctree/health` | snapshot | Risk + health summary. |
| `loctree/workspaces` | snapshot | Discover Loctree workspaces under the root. |
| `loctree/aicx` | AICX overlay | Read-only AICX memory entries. |
| `loctree/astQuery` | live AST | tree-sitter structural query over the live buffer (JS/TS/TSX). |

Snapshot-backed requests read from the same snapshot as the CLI and MCP, so a
`loctree/slice` over the wire and a `loct slice` on the terminal answer from one
source of truth. `loctree/astQuery` is the one live-AST request: it reflects the
unsaved buffer for supported languages and falls back to snapshot truth
otherwise.

## Parser substrate: `loctree-ast`

`loctree-ast` is a **narrow tree-sitter substrate** for the live-AST and
structural-query paths. It runs alongside the existing analyzer stack; it does
**not** replace OXC or the cold-scan extractors, and it does not generate the
snapshot. It currently parses JavaScript, TypeScript, and TSX
(`Parsers::new_default()`), with incremental re-parse for editor edits. Extractor
parity for more languages is tracked separately and does not change the
snapshot-first model.

## Editor surfaces (`./editors`)

The LSP is the bridge that lets Loctree show up where the user actually works:

- **VS Code** (`editors/vscode/`) — context pill, findings, commands, status bar,
  tree view, and the LSP client gateway.
- **JetBrains** (`editors/jetbrains/`) — IntelliJ-platform plugin: tool window,
  contributed LSP commands, findings reader, binary resolver, settings.
- **Neovim** (`editors/nvim/`) — generic LSP client wiring with the same active
  runtime path, build identity, resolver source, and PATH-shadow warning contract.

These are a first-class product surface, not an add-on: they expose the same
snapshot-backed structural truth in-editor.

## Wire-up

Editor wire-up examples live in [docs/ide/](../ide/) and
[editors/jetbrains](../../editors/jetbrains). Pin the workspace root at startup
with `--root`; when omitted, the root is discovered from the LSP `initialize`
handshake.

```bash
loctree-lsp --root /path/to/repo
```

## See Also

- [MCP Server Integration](./mcp-server.md)
- [loctree CLI Reference](../dev/03_cli_reference.md)
- [Architecture](../architecture.md)
