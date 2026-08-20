# Changelog — Loctree for JetBrains

All notable changes to the Loctree JetBrains plugin are documented here.
The format follows [Keep a Changelog](https://keepachangelog.com/).

## [Unreleased]

## [0.14.2] - 2026-08-18

### Changed
- Download default is now **pinned to the plugin's own version** (`v0.14.2`)
  instead of `latest`, enforcing same-version discipline between plugin and
  `loctree-lsp` runtime. `latest` remains available as an explicit opt-in in
  Settings ▸ Tools ▸ Loctree.
- Plugin description states the real requirement in plain language: a paid
  JetBrains IDE on the 2025.2.1+ line (Community editions do not ship the
  native LSP module).
- Vendor URL points at the canonical `https://loctree.com` (no redirect hop).

### Fixed
- **Honest platform matrix**: the resolver no longer advertises
  `loctree-lsp-windows-x64.exe` or `loctree-lsp-linux-arm64` — neither asset
  is published on `Loctree/loctree-release`, so first run on those platforms
  was a guaranteed 404 followed by a silent, dead plugin. Unsupported
  platforms now get a visible IDE notification naming the platform and
  pointing at the manual `loctree-lsp` install path; a failed pinned-tag
  download surfaces an actionable notification instead of a silent PATH
  dead-end.
- Marketplace plugin icons: light-theme `pluginIcon.svg` /
  `loctree-action.svg` now use dark brand ink (the previous near-white mark
  was invisible on the white Marketplace card and light-theme IDE dialogs),
  with `pluginIcon_dark.svg` / `loctree-action_dark.svg` carrying the
  original light artwork. `pluginIcon*.svg` resized to the JetBrains 40×40
  spec.
- Settings UI no longer promises a "bundled binary" (released ZIPs are
  download-only by design) and no longer shows the mangled
  `v0.14.2-dev-dev-dev` example tag.
- CI build lane (`jetbrains-plugin.yml`) aligned to JDK 21, matching the
  Gradle toolchain and the publish lane.

## [0.14.1] - 2026-07-25

### Added
- Inline LSP query surface in the Loctree tool window with routed modes for
  Health, Find, Literal, Follow, Slice, Impact, Context Atlas, Context Pack,
  Workspaces, Diff, AICX, Semantic, and AST Query. Literal search actions now
  publish rich JSON results into the tool window with Load More continuation
  instead of relying on notification-only summaries.
- Initial JetBrains IDE plugin (`editors/jetbrains/`) integrating the
  `loctree-lsp` server via the native IntelliJ LSP API
  (`LspServerSupportProvider` + `ProjectWideLspServerDescriptor`),
  launched with `--root <projectBasePath>`.
- Typed `LoctreeLspGateway` for custom `loctree/*` requests
  (`health`, `find`, `impact`, `slice`, `contextAtlas`, `follow`) and the
  `refresh` notification, with lenient `Paginated<T>` decoding that
  tolerates unknown server-added fields.
- Verified runtime resolution: configured path → IDE cache → verified
  download → `PATH`, with mandatory SHA256 verification (fail-closed)
  before a downloaded binary is executed. Asset matrix matches the
  VS Code extension.
- Settings (Tools ▸ Loctree) mirroring the VS Code configuration surface:
  `serverPath`, `autoRefresh`, `showStatusBar`, `autoDownload`,
  `downloadBaseUrl`, `downloadTag`, `diagnosticSeverity`.
- Status bar widget (disconnected / downloading / running / healthy /
  error) and a findings tool window grouping dead exports, circular
  imports, and twins with loading/empty/error states and double-click
  navigation.
- Context actions (editor + project view): Refresh, Show Health, Analyze
  Impact, Show Slice, Find Consumers/Importers, Check Dead Exports,
  Show/Analyze Cycles, Open Report — all workspace-boundary safe.
- Append-only cycle suppression writer restricted to
  `<workspace>/.loctree/suppressions.toml`.
- Unit tests for asset selection, SHA256 verification, workspace path
  guards, `Paginated<T>`/`HealthResponse` decoding, findings parsing, and
  suppression writing.
- CI workflow building, testing, and verifying the plugin without
  Marketplace secrets. Signing/publishing remain operator stop-points.

### Fixed
- Tool-window query results preserve full JSON payloads and `contextPack` card
  content instead of silently truncating deep fields, large arrays, or long
  strings.
- Released ZIPs are **download-only**: the plugin no longer bundles a
  host-built `loctree-lsp` at the unqualified resource path `bin/loctree-lsp`.
  A single-platform bundle outranked cache and verified download in
  `BinaryResolver` on *every* OS, so a Linux-built ZIP would have handed macOS
  and Windows users an unexecutable binary with no self-repair. Bundling is now
  a dev-only opt-in (`-PbundleLsp=true` / `LOCTREE_BUNDLE_LSP=1`), and the
  publish workflow fails if a runtime is found inside the artifact.

### Compatibility
- Moved the active compatibility lane to IntelliJ Platform **2025.2.1+**
  (`since-build=252.1`) and the native LSP module dependency
  (`com.intellij.modules.lsp`). The plugin no longer claims one ZIP covers
  both the older Ultimate-module LSP API and the modern LSP module API.
