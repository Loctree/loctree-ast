# @loctree/loctree

**Loctree** — structural code intelligence for AI agents. One runtime, one install.

`npm install -g @loctree/loctree` installs the Loctree runtime. The same wrapper
also publishes under the maintained scoped alias `@loctree/loct` and short name
`loctree`. All three identities contain the same runtime payload. Source of truth lives in
[Loctree/loctree](https://github.com/Loctree/loctree): the npm scope names the
org, the package names the repo.

## Install

```bash
npm install -g @loctree/loctree     # canonical name
# or
npm install -g @loctree/loct        # maintained scoped alias
# or
npm install -g loctree              # same package, short name
# or
pnpm add -g @loctree/loctree
```

This gives you **one runtime** with four commands on PATH:

| Command | What it is |
|---------|-----------|
| `loctree` | the Loctree runtime — scan, slice, impact, health, plus MCP / editor modes |
| `loct` | short alias for `loctree` (the same binary) |
| `loctree-mcp` | stdio MCP co-process for MCP clients and package runners |
| `loctree-lsp` | language server for editors (VS Code, JetBrains, Neovim) |

Smoke-test:

```bash
loctree --version
loct --version
npx -y --package=@loctree/loctree loctree-mcp --version
```

## First scan and MCP wiring

```bash
cd your-project
loct scan            # build the snapshot
loct --for-ai        # orientation card for agents
```

MCP clients (Claude Code, Claude Desktop, Cursor, ...) that speak stdio:

```json
{
  "mcpServers": {
    "loctree": {
      "command": "npx",
      "args": ["-y", "--package=@loctree/loctree", "loctree-mcp"]
    }
  }
}
```

With the global install on PATH, `"command": "loctree-mcp"` with no args works
too. Prefer streamable HTTP? Run `loct watch --http` and point the client at
`http://127.0.0.1:5174/mcp`.

## MCP and editors are modes of the runtime

MCP and LSP remain modes the runtime brings up on demand:

```bash
loct watch --http     # streamable-HTTP MCP server at http://127.0.0.1:5174/mcp
loct watch --lsp      # editor language server (co-process for IDEs)
```

The runtime spawns its `loctree-mcp` / `loctree-lsp` co-process binaries from
inside the installed package. MCP clients that require a stdio command can run
the same bundled MCP binary without a global install:

```bash
npx -y --package=@loctree/loctree loctree-mcp
```

The shorter `npx -y @loctree/loctree` runs the `loct` CLI. There are no separate
`@loctree/loctree-mcp` or `@loctree/loctree-lsp` npm packages.

## How delivery works

The wrapper declares one technical platform package per target as an
`optionalDependency`; npm/pnpm/yarn install only the one matching your platform.
Each platform package embeds the runtime and its co-process binaries side by side
(no postinstall download), so the runtime always finds them as siblings.
`@loctree/loctree`, `@loctree/loct`, and `loctree` are maintained install
identities over this one wrapper source.

## Supported platforms

- macOS Apple Silicon: `@loctree/loctree-darwin-arm64`
- macOS Intel: `@loctree/loctree-darwin-x64`
- Linux x64 glibc: `@loctree/loctree-linux-x64-gnu`
- Windows x64: `@loctree/loctree-win32-x64-msvc`

We only claim the targets CI actually builds today.

## License

BUSL-1.1. See [LICENSE](./LICENSE).

## Links

- Source: https://github.com/Loctree/loctree
- Website: https://loct.io
- Issues: https://github.com/Loctree/loctree/issues
