# Changelog

All notable changes to the Loctree VS Code extension are documented here.

The extension version tracks the Loctree suite version: extension `vX.Y.Z` always
ships and expects `loctree-lsp vX.Y.Z`. A build that mixes versions is a release
bug, not a supported configuration.

## [Unreleased]

## [0.14.2] — 2026-08-13

### Added
- Marketplace publish workflow (`.github/workflows/vscode-publish.yml`) — manual
  `workflow_dispatch` button that republishes already-built, platform-tagged VSIX
  artifacts to the VS Code Marketplace and/or Open VSX, with a dry run by default.
- Multi-literal OR search shared by the MCP server and the LSP, so the Context
  panel's literal mode can query several needles in one round trip.

### Changed
- Per-platform VSIX builds are now tagged with `vsce --target`, so
  `linux-x64` / `darwin-arm64` / `darwin-x64` packages are distinct marketplace
  artifacts instead of three same-named uploads overwriting each other.
- Context Atlas card surface is English-only (no mixed-language labels).

### Fixed
- Marketplace listing metadata: `repository` / `bugs` / `qna` pointed at the
  private `Loctree/loctree-suite`, which renders as a dead link for every
  marketplace visitor. They now point at public Loctree surfaces.
- README status bar section referenced a `$(loctree-logo)` codicon that does not
  exist; the entry point actually renders as `$(type-hierarchy)`.
- Refreshed the lockfile for `fast-uri`, `linkify-it`, `brace-expansion`,
  `undici`, and `js-yaml` advisories.

## [0.14.1] — 2026-07-24

### Added
- **Mega-root guard + bounded residency LRU** in `loctree-lsp`: opening a very
  large or accidental root no longer pins an unbounded snapshot in memory.
- **Runtime cards from executable ownership** in context packs — context surfaces
  now derive runtime ownership instead of inferring it from file layout.
- **Symbol body extent + multi-candidate qualification** — `Loctree: Show Symbol
  Body` disambiguates overloads/duplicates instead of guessing one.
- Build provenance: checkout identity is stamped into the shipped binary, so
  `loctree-lsp --version` reports the exact commit it was built from.

### Changed
- Impact analysis no longer claims a file is "safe to remove". That verdict
  required graph coverage the analyzer does not yet prove, so it was removed
  rather than left as a confident-sounding guess.

### Fixed
- Scan allocator arenas are released after a scan, cutting resident memory on
  long-lived editor sessions.

## [0.14.0] — 2026-07-15

### Added
- **Context Atlas intent map** — a structural/intentional card in the Context
  panel that pairs the dependency shape with recorded intent.
- Atlas card 00 keeps executable "safe next commands" through the LSP
  `contextPack` path, so the panel's suggestions stay runnable.
- `--include-ignored` override for files excluded by `.loctignore`.

### Fixed
- **Context Pill webview host messages are origin-fenced.** The previous guard
  was both broken and over-broad: it rejected legitimate messages and killed the
  Context webview outright. Replaced with a correct check.
- Dev-dependency refresh for `undici`, `form-data`, `js-yaml`, and `esbuild`.

## [0.13.1] — 2026-07-12

### Fixed
- **Webview and binary paths are fenced** — the extension validates the
  `loctree-lsp` path it resolves and the resources the webview may load.
- **Unified LSP lifecycle state.** Start / restart / crash-recovery previously
  tracked state in several places and could leave the status bar and the Context
  panel disagreeing about whether the server was running.
- Literal occurrence hits are enriched with AST context (enclosing symbol/kind)
  instead of bare file:line.
- `Loctree: Show File Slice` output carries focus information.
- Lockfile refreshed to clear the outstanding npm audit advisories.

## [0.13.0] — 2026-07-02

### Added
- Twins detection distinguishes exact duplicates from name collisions and
  aggregates systemic pairs, so the Findings explorer stops reporting one finding
  per file pair.
- Shared artifact fence consumed by all detectors: vendored, generated, fixture,
  and template files are classified once and excluded consistently.

## [0.12.x] — 2026-06

### Added
- **Literal occurrence search** (`Loctree: Search Literal Occurrences`) — exact
  identifier-boundary matches via `loctree/find` literal mode, at parity with
  `loct occurrences` / `find --literal`, shown as a navigable QuickPick.
- **Symbol body** (`Loctree: Show Symbol Body`) — bounded source body of a symbol
  via the `loctree/body` LSP request (parity with `loct body`).
- **Context Pill webview** registered as the primary Context surface, with
  ambient active-editor binding.
- **Hotspots** group and a **Health** header (score + green/yellow/red status) in
  the Findings explorer.
- Rich status bar: health score, status color, and a stale-snapshot indicator.

### Changed
- Findings and status stream live from the `loctree-lsp` server
  (`loctree/health`) instead of reading `.loctree/*.json` from the workspace —
  loctree 0.12 writes artifacts to the per-project cache dir, so the old disk
  reads showed empty/stale results.
- Impact / consumers / slice / cycles / dead-export commands issue real LSP
  requests and render their results, replacing fire-and-forget refresh calls.
- Code lenses are opt-in and off by default, to avoid doubling up with the
  language server's own lenses.
- Expert commands are Command Palette only.
- The extension is bundled with esbuild (`dist/extension.js`), which removed the
  "Cannot find module" class of activation failures the unbundled package had.

### Removed
- Duplicate surfaces cut: the dead-code panel, the second hover provider, the
  references provider, and the standalone literal QuickPick. Loctree does not
  provide hover, go-to-definition, or find-references — those stay with your real
  language server.

### Fixed
- Findings no longer appear empty on projects scanned with loctree 0.12+.
- English-only UI labels (dropped hardcoded Polish strings).
- Public listing links no longer point at a private repository.

## Notes

This extension is MIT-licensed; the Loctree language server it bundles and
downloads is licensed under BUSL-1.1.
