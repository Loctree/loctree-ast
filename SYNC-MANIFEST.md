# Component Sync Manifest

- Component: `engine`
- Version: `0.14.2`
- Target repo: `Loctree/loctree`
- Source commit: `16199c23d39c`
- Generated at: `2026-08-13T15:04:04Z`
- Push mode: `disabled`
- Dependency mode: `local workspace snapshot`

## Included Payload

- `loctree-rs` -> `loctree-rs`
- `loctree-ast` -> `loctree-ast`
- `loctree-mcp` -> `loctree-mcp`
- `loctree-lsp` -> `loctree-lsp`
- `reports` -> `reports`
- `distribution/npm` -> `distribution/npm`
- `distribution/macos` -> `distribution/macos`

## Dependency Note

The engine repo carries the full source workspace - loctree, loctree-ast, loctree-mcp, loctree-lsp and the report-leptos renderer - as path dependencies. It builds standalone, with no wait on crates.io publication.
