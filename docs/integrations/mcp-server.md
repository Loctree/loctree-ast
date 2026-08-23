# MCP Server Integration

`loctree-mcp` exposes Loctree's structural code intelligence through Model
Context Protocol over stdio or streamable HTTP.

It is the **agent access surface** in the Loctree flow: the same snapshot-first
structural authority the CLI reads, served to agents.

```text
Repo checkout → snapshot authority → structural / context pack → CLI / MCP / LSP → editor surfaces
```

The MCP product surface is intentionally small: **12 tools**. It is not a
mirrored CLI. Report commands such as `health`, `findings`, `audit`, and
`coverage` stay in `loct`.

## Architecture

```text
Claude Desktop/Code
    |
    v
loctree-mcp
    |
    |- auto-scans project on first use
    |- keeps a bounded snapshot cache in memory
    |- materializes Context Atlas cards for context requests
    `- exposes 12 agent-facing tools
```

## Installation

End users install prebuilt binaries — see [docs/installation.md](../installation.md) for the full menu.

```bash
# Recommended — signed bundle (loct + loctree + loctree-mcp + aicx + aicx-mcp)
curl -fsSL https://loct.io/install.sh | bash

# Or via npm runtime package (includes sibling loctree-mcp and loctree-lsp binaries)
npm install -g @loctree/loctree

# Or run the bundled stdio MCP server without a global install
npx -y --package=@loctree/loctree loctree-mcp

# Or via Homebrew for the core CLI; MCP/LSP formulae follow the thin-repo release tracks
brew install loctree/cli/loct
```

> `loctree-mcp` is published on crates.io (`cargo install --locked loctree-mcp`) as of 0.13.0. `cargo install` compiles
> from source (needs a Rust toolchain and `protoc`), so most end users still prefer the signed bundle or npm package
> above for a prebuilt binary. `cargo install` is the contributor/Rust-toolchain path, not the default recommendation.

Smoke-test:

```bash
loctree-mcp --version
```

For contributors building from source, see [dev/01_installation.md](../dev/01_installation.md).

### Docker / Glama

The repository-root `Dockerfile` is also copied into the public
`Loctree/loctree-mcp` mirror. It is a non-root stdio image, which Glama can
wrap as a hosted Streamable HTTP endpoint:

```bash
docker build --build-arg VCS_REF="$(git rev-parse --short=8 HEAD)" -t loctree-mcp .
docker run --rm -i \
  --mount type=bind,src="$PWD",dst=/workspace,readonly \
  --mount type=volume,src=loctree-cache,dst=/data \
  loctree-mcp
```

The source mount is read-only; snapshots live under `/data/loctree-cache`.
Glama hosting only sees files provisioned inside the hosted container, not a
checkout on the MCP client's machine.

## Configuration

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

The equivalent no-install npm configuration is:

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

`npx -y @loctree/loctree` is intentionally the `loct` CLI, so MCP clients must use
the explicit `--package` form above.

## Tool Surface

| Tool | Use |
| --- | --- |
| `context` | Complete Agent Context Pack: structure, runtime, risk, action, authority, optional AICX memory, and Context Atlas pointer. |
| `repo-view` | Repo overview: files, LOC, languages, health summary, top hubs, quick wins. |
| `focus` | Module deep-dive: files, internal edges, external dependencies, external consumers. |
| `slice` | File-level context before edit: target file, dependencies, and consumers. |
| `body` | Bounded symbol source body/range with extent and truncation metadata. |
| `find` | Symbol and feature search; `literal` scans identifier boundaries and `regex` evaluates raw file text with coverage accounting. |
| `impact` | Blast-radius check before delete or major refactor. |
| `diff` | Compare the current live snapshot, including dirty-tree structure, with a git ref such as `HEAD~1`. |
| `tree` | Directory layout with LOC counts; unlimited depth by default, plus CLI-parity path/files/match/summary/top/hidden/ignored/artifact controls. |
| `follow` | Field-level follow-up for `dead`, `cycles`, `twins`, `hotspots`, `trace`, `commands`, `events`, and `pipelines`. |
| `suppressions` | Source-side silencer inventory: Rust `#[allow]`/`#[ignore]`/`unsafe`, `nosemgrep`, `@ts-ignore`, `eslint-disable`, `# noqa`, `# type: ignore`, shellcheck disables. |
| `prism` | Score conceptual smear across task framings; emits `loctree.prism.v1` JSON for `vc-polarize` gating. |

All tools read from the same snapshot and accept `project`, auto-scanning on
first use. Non-git scratch directories require `force_no_git=true`.

Each MCP session retains one most-recently-used project snapshot in memory by
default, preventing temporary worktrees from accumulating without bound in a
long-lived process. Operators may set `--snapshot-cache-capacity COUNT` to a
larger intentional limit, or `0` to disable the in-memory cache.

Large JSON responses are capped by the `loctree.mcp.response_budget.v1` contract:
markdown/context responses default to a 38,000-character page, and the complete
unmodified payload is written beside the project under
`.loctree/mcp-response-payloads/*.full.json` when pagination or previewing is
needed. This keeps the MCP surface inside client token budgets without losing
full-fidelity evidence.

## Recommended Flow

1. Start with `context` for the complete context pack and authority receipt.
2. Use `repo-view` for the compact map and health summary.
3. Use `focus` for the module you will touch.
4. Use `slice` before editing a file.
5. Use `find` before creating a symbol; use `mode: "regex"` for raw-text patterns.
6. Use `body` after locating a symbol instead of offset-based file reads.
7. Use `impact` before deleting or changing a high-impact file.
8. Use `diff` when the Living Tree may have moved since the last observation.
9. Use `follow` to pursue `dead`, `cycles`, `twins`, `hotspots`, or runtime traces.

## Examples

Before modifying a file:

```typescript
slice({ project: ".", file: "src/hooks/useAuth.ts", consumers: true })
```

Before creating a new component:

```typescript
find({ project: ".", name: ".*[Dd]ate.*[Pp]icker.*", limit: 10 })
```

Raw-text regex with coverage accounting:

```typescript
find({ project: ".", name: "TODO|FIXME", mode: "regex", file: "docs" })
```

Read a bounded definition or compare against the last commit:

```typescript
body({ project: ".", symbol: "build_release", file: "src/release.rs" })
diff({ project: ".", since: "HEAD~1" })
```

Before deleting or splitting a module:

```typescript
impact({ project: ".", file: "src/utils/api.ts" })
```

Before commit:

```typescript
repo-view({ project: "." })
follow({ project: ".", scope: "all" })
```

Trace a Tauri command:

```typescript
follow({ project: ".", scope: "trace", handler: "get_user" })
```

## Troubleshooting

If the server does not start, check the binary path:

```bash
which loctree-mcp
```

If a project path cannot be resolved, pass an absolute `project` path.

If a scratch directory is not a git repo, opt in explicitly:

```typescript
context({ project: "/tmp/scratch", force_no_git: true })
```

If data appears stale, call the relevant tool with `fresh=true` when supported
or run a fresh CLI scan:

```bash
loct scan
```

## See Also

- [loctree CLI Reference](../dev/03_cli_reference.md)
- [CI/CD Integration](./ci-cd.md)
