# DEPRECATED — use `loctree-plugin/`

This directory is **not** a peer install path.

## Canonical agent plugin (in-suite)

```text
loctree-suite/loctree-plugin/
```

Install:

```bash
# From suite root
claude --plugin-dir ./loctree-plugin

# Or absolute
/plugin install /path/to/loctree-suite/loctree-plugin
```

The full surface lives there: skills for every MCP map tool (`context`,
`slice`, `impact`, `find`, `focus`, `follow`, `prism`, `repo-view`, `tree`),
hooks, agents, and wiring for both `loctree-mcp` and `loctree-lsp`.

## Why this stub exists

Historically `./plugin` was a thin Claude Code loader. It competed with
`./loctree-plugin` and advertised a second install story. That split is closed:
**one canonical plugin** next to the engine crates.

If a host still points at `./plugin`, point it at `./loctree-plugin` instead.

## Sibling public repo

`github.com/Loctree/loctree-plugin` (checkout often `../loctree-plugin`) is a
separate remote. Marketplace deprecation / README redirect on that remote is an
**operator publish** step — suite-side truth is already `loctree-plugin/` here.
