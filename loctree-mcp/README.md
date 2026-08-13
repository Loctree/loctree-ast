# loctree-mcp

MCP (Model Context Protocol) server exposing Loctree's structural code
intelligence to AI agents (Claude Code, Codex, Cursor, and other MCP-capable
clients) over stdio, or over streamable HTTP with bearer auth.

It is the agent access surface for Loctree: the same snapshot-first
structural authority the `loct`/`loctree` CLI reads, served as MCP tools.

## Tool surface

Twelve tools — a sharp agent surface, not a mirrored CLI. Report-style commands
(`health`, `findings`, `audit`, `coverage`) stay in the `loct` CLI.

| Tool | Use |
| --- | --- |
| `context` | Complete Agent Context Pack: structure, runtime, risk, action, authority, optional AICX memory, and Context Atlas pointer. Start here. |
| `repo-view` | Compact repository overview: file count, LOC, languages, health summary, top hubs, quick wins. |
| `focus` | Module deep-dive: files, internal edges, external dependencies, external consumers for one directory. |
| `slice` | File-level context before an edit: target file, its dependencies, and its consumers in one call. |
| `body` | Bounded symbol source body/range with extent and truncation metadata. |
| `find` | Symbol/feature search. Modes include `literal` (exact identifier boundaries) and `regex` (raw file text plus coverage accounting). |
| `impact` | Blast-radius check before deleting or majorly refactoring a file: direct + transitive consumers. |
| `diff` | Compare the current live snapshot, including dirty-tree structure, with a git ref such as `HEAD~1`. |
| `tree` | Directory structure with LOC counts; depth is unlimited when omitted and path/files/match/summary/top controls mirror the CLI. |
| `follow` | Field-level follow-up: `dead`, `cycles`, `twins`, `hotspots`, `trace`, `commands`, `events`, `pipelines`. |
| `suppressions` | Source-side silencer inventory (Rust `#[allow]`/`#[ignore]`/`unsafe`, `nosemgrep`, `@ts-ignore`, `eslint-disable`, `# noqa`, `# type: ignore`, shellcheck disables). Literal-only detection (free-tier scope); semantic enrichment is a paid-tier delta, out of scope here. |
| `prism` | Score conceptual smear across two or more task framings; emits `loctree.prism.v1` JSON for `vc-polarize` gating. |

All tools read from the same snapshot and accept a `project` parameter,
auto-scanning on first use.

The server retains only the most recently used project snapshot in memory by
default. This bounds long-lived MCP sessions that visit temporary worktrees.
Use `--snapshot-cache-capacity COUNT` to opt into a larger cache, or set it to
`0` to disable in-memory snapshot caching.

## Installation

Most users should install a prebuilt binary rather than building this crate
from source:

```bash
# Signed bundle (loct + loctree + loctree-mcp + aicx + aicx-mcp)
curl -fsSL https://loct.io/install.sh | bash

# or the npm runtime package (includes sibling loctree-mcp / loctree-lsp binaries)
npm install -g @loctree/loct
```

This crate is also published on crates.io:

```bash
cargo install --locked loctree-mcp
```

`cargo install` compiles from source and requires a Rust toolchain plus
`protoc`; the prebuilt paths above are faster for most setups.

Smoke-test any install path:

```bash
loctree-mcp --version
```

### Docker / Glama

The repository-root `Dockerfile` is the canonical container definition used by
both this suite and the public `Loctree/loctree-mcp` release mirror. It runs the
stdio server as a non-root user, reads a project mounted at `/workspace`, and
writes snapshots to `/data/loctree-cache` instead of modifying the source tree:

```bash
docker build --build-arg VCS_REF="$(git rev-parse --short=8 HEAD)" -t loctree-mcp .
docker run --rm -i \
  --mount type=bind,src="$PWD",dst=/workspace,readonly \
  --mount type=volume,src=loctree-cache,dst=/data \
  loctree-mcp
```

The root `glama.json` claims the server listing. Glama can build the Dockerfile
and wrap its stdio transport as a hosted Streamable HTTP endpoint. A hosted
container only sees files provisioned inside that container; it cannot inspect
a repository on the client's laptop.

### One-shot npm execution

`@loctree/loct` remains the single public npm package. It now exposes the MCP
co-process for package runners without creating a second npm product:

```bash
npx -y --package=@loctree/loct loctree-mcp
```

`npx -y @loctree/loct` still launches the `loct` CLI by design.

## Client configuration

### Claude Code

```bash
claude mcp add loctree -- loctree-mcp
```

Or add it directly to `.mcp.json` / `~/.claude.json`:

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

### Codex CLI

Add a `[mcp_servers.loctree]` table to `~/.codex/config.toml` (or the
project-scoped `.codex/config.toml` for a trusted project):

```toml
[mcp_servers.loctree]
command = "loctree-mcp"
args = []
```

### Generic MCP client (Claude Desktop, Cursor, etc.)

Most stdio-based MCP clients read the same `mcpServers` JSON shape:

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

## License

BUSL-1.1 — see the [LICENSE](../LICENSE) file at the workspace root. Converts
to Apache License 2.0 on 2030-04-13.

## More

Full tool reference, recommended flow, and troubleshooting:
[docs/integrations/mcp-server.md](https://github.com/Loctree/loctree-suite/blob/main/docs/integrations/mcp-server.md)
(private monorepo; public issues/PRs for this crate belong on
[Loctree/loctree-mcp](https://github.com/Loctree/loctree-mcp)).
