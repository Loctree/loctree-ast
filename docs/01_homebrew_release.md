# Homebrew Release Architecture

Loctree no longer treats `homebrew-core` as the primary install path.

The shipping architecture is now:

- source + CI + versioning: `Loctree/loctree-suite`
- CLI binary releases: `Loctree/loct`
- MCP binary releases: `Loctree/loctree-mcp`
- CLI tap: `Loctree/homebrew-cli`
- MCP tap: `Loctree/homebrew-mcp`

The tap repo slugs stay as-is for now, but the active formulas inside them are
`loct` and `loct-mcp`.

## User-Facing Commands

```bash
brew install loctree/cli/loct
brew install loctree/mcp/loct-mcp
```

## Why This Shape

- The monorepo stays focused on code and CI instead of serving as a public asset bucket.
- CLI and MCP now have separate binary channels, which keeps install paths honest.
- Homebrew formulas can target exactly one product each.
- Release automation becomes deterministic: build once in the monorepo, distribute outward.

## Release Sequence

The release uses two explicit operator buttons:

```bash
make version TYPE=minor TAG=1 PUSH=1
gh workflow run publish.yml -f tag=vX.Y.Z
```

The tag builds combined bundles. The manual `publish.yml` dispatch then runs:

1. crate publishing in `Loctree/loctree-suite`
2. binary builds for CLI and MCP
3. active-line asset upload to `Loctree/loct` and `Loctree/loctree-mcp`
4. active `loct` npm publish from `distribution/npm`
5. monorepo release publication
6. Homebrew tap sync for formulas `loct` and `loct-mcp` into `Loctree/homebrew-cli` and `Loctree/homebrew-mcp`

## Homebrew Formula Source of Truth

The formulas are rendered by:

```bash
scripts/render-homebrew-formula.sh
```

The workflow computes release SHA256 checksums from the thin repos and writes the
resulting files directly into the tap repos. The tap repos should not maintain
hand-edited version drift.

## Release Repository Contract

These GitHub repositories are the existing release targets:

- `Loctree/loct`
- `Loctree/loctree-mcp`
- `Loctree/homebrew-cli`
- `Loctree/homebrew-mcp`

Also configure `HOMEBREW_GITHUB_API_TOKEN` in `Loctree/loctree-suite` with write access
to all four repositories.

## Supported Homebrew Targets

- macOS Apple Silicon
- macOS Intel
- Linux x86_64

## Naming Status

- `loct` is the active Homebrew CLI formula.
- `loct-mcp` is the active Homebrew MCP formula.
- `loct-lsp` formula rendering is supported in `scripts/render-homebrew-formula.sh`, but tap sync is not wired yet.
- `loct-report` is out of the active Homebrew release path until the reports crate is renamed out of `report-leptos`.

## Operational Notes

- The monorepo release is an orchestration/changelog release, not the main binary channel.
- npm should pull CLI assets from `Loctree/loct`, never from the monorepo release.
- If a tap sync fails, fix the thin release assets first, then re-run `homebrew-release.yml`.
