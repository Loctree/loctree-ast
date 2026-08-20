# Loctree MCP {{VERSION}}

This repository is the public home of the Loctree MCP server.

The server exposes Loctree structural tools over MCP. Issues and pull requests are welcome here.

## Build

```bash
cargo check --workspace
```

## Docker / Glama

The repository ships a Glama ownership manifest and a production multi-stage
Docker image. Glama can build the repository directly and wrap the image's
stdio MCP transport as its hosted Streamable HTTP endpoint.

For a local MCP client, mount the repository being analyzed read-only at
`/workspace` and keep Loctree's generated snapshot cache in a named volume:

```bash
docker build --build-arg VCS_REF="$(git rev-parse --short=8 HEAD)" -t loctree-mcp .
docker run --rm -i \
  --mount type=bind,src="$PWD",dst=/workspace,readonly \
  --mount type=volume,src=loctree-cache,dst=/data \
  loctree-mcp
```

The image runs as a non-root user, starts `loctree-mcp` over stdio, pins the
default project root to `/workspace`, and stores generated artifacts under
`/data/loctree-cache` via `LOCT_CACHE_DIR`.

Glama hosting can only analyze files available inside its container or
persistent volume. It cannot see a repository sitting on your laptop; use the
local Docker configuration above when local source is the target.

## npm / npx

The single public npm package also exposes the bundled MCP co-process:

```bash
npx -y --package=@loctree/loctree loctree-mcp
```

The shorter `npx -y @loctree/loctree` intentionally launches the `loct` CLI, not
the MCP server.

## License

BUSL-1.1. See `LICENSE` and `NOTICE.md`.

## Snapshot Notes

- Target repo: `{{TARGET_REPO}}`
- Dependency mode: `{{DEPENDENCY_MODE}}`
- {{DEPENDENCY_NOTE}}
