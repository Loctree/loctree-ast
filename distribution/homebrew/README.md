# Homebrew Distribution

The monorepo does not ship one generic Homebrew formula anymore.

Instead:

- active CLI formula `loct` is rendered into `Loctree/homebrew-cli`
- active MCP formula `loct-mcp` is rendered into `Loctree/homebrew-mcp`

`loct-lsp` formula rendering is supported locally, but tap sync is not wired yet
because the thin/tap repo track is still incomplete.

The rendering source of truth lives in:

- `scripts/render-homebrew-formula.sh`
- `.github/workflows/homebrew-release.yml`

## Release Contract

1. `publish.yml` builds and uploads binary assets into the thin repos:
   - `Loctree/loct`
   - `Loctree/loctree-mcp`
2. `homebrew-release.yml` downloads those tarballs, computes SHA256 values, and
   writes the tap formulas.

## Local Test Flow

After a release exists, you can render formulas locally by exporting the same
SHA variables the workflow uses and running:

```bash
scripts/render-homebrew-formula.sh loct 0.9.0 /tmp/loct.rb
scripts/render-homebrew-formula.sh loct-mcp 0.9.0 /tmp/loct-mcp.rb
```

Then test with Homebrew:

```bash
brew install --build-from-source /tmp/loct.rb
brew install --build-from-source /tmp/loct-mcp.rb
```
