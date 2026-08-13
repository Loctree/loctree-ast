# @loctree/loct

**Loctree** — structural code intelligence for AI agents. One runtime, one install.

`npm install -g @loctree/loct` installs the Loctree runtime. Source of truth
lives in [Loctree/loctree](https://github.com/Loctree/loctree).

## Install

```bash
npm install -g @loctree/loct
# or
pnpm add -g @loctree/loct
```

This gives you **one runtime**, with two CLI names and one MCP adapter:

| Command | What it is |
|---------|-----------|
| `loctree` | the Loctree runtime — scan, slice, impact, health, plus MCP / editor modes |
| `loct` | short alias for `loctree` (the same binary) |
| `loctree-mcp` | stdio MCP co-process for MCP clients and package runners |

Smoke-test:

```bash
loctree --version
loct --version
npx -y --package=@loctree/loct loctree-mcp --version
```

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
npx -y --package=@loctree/loct loctree-mcp
```

The shorter `npx -y @loctree/loct` runs the `loct` CLI. There are no separate
`@loctree/loctree-mcp` or `@loctree/loctree-lsp` npm packages.

## How delivery works

The wrapper declares one technical platform package per target as an
`optionalDependency`; npm/pnpm/yarn install only the one matching your platform.
Each platform package embeds the runtime and its co-process binaries side by side
(no postinstall download), so the runtime always finds them as siblings.
`@loctree/loct` is the only public package.

## Supported platforms

- macOS Apple Silicon: `@loctree/loct-darwin-arm64`
- macOS Intel: `@loctree/loct-darwin-x64`
- Linux x64 glibc: `@loctree/loct-linux-x64-gnu`
- Windows x64: `@loctree/loct-win32-x64-msvc`

We only claim the targets CI actually builds today.

## License

BUSL-1.1. See [LICENSE](./LICENSE).

## Links

- Source: https://github.com/Loctree/loctree
- Website: https://loct.io
- Issues: https://github.com/Loctree/loctree/issues
