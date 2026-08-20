---
name: loctree-slice-request
status: queued
agent_target: any
project: loctree-suite
priority: 5
created: 2026-05-05
parent_branch: feat/context-tool-alpha
---

# Plan 5 — `loctree/slice` request

## Why

Agent before editing a file needs holographic slice — the file plus its
deps and consumers. Today only CLI (`loct slice <file>`) and MCP
(`mcp_loctree-mcp_slice`) expose this; LSP does not. Add a custom request
so daemon-mode agents (Cursor, Codex CLI with LSP, etc.) can fetch slice
in one round-trip without spawning a CLI.

## Acceptance criteria

- [ ] `loctree/slice` custom request implemented in
  `loctree-lsp/src/backend.rs`.
- [ ] Params: `{ target: PathBuf, consumers: bool, depth: Option<usize>, project: Option<PathBuf> }`.
- [ ] Response: `{ core: [{path, loc, lang}], deps: [{path, depth, lang}],
      consumers: [{path, depth, lang}], total_files: usize, total_loc: usize }`.
- [ ] All paths are repo-relative strings; no inline content.
- [ ] Capability advertised under `experimental.loctree/slice = { available: true }`.
- [ ] Integration test: `loctree-lsp/tests/slice_request.rs`.

## Files

- `loctree-lsp/src/backend.rs` — capability + dispatch.
- `loctree-lsp/src/slice.rs` (NEW) — handler delegating to
  `loctree::slicer::HolographicSlice`.
- `loctree-lsp/tests/slice_request.rs` (NEW) — integration test.

## Implementation sketch

```rust
// loctree-lsp/src/slice.rs
use loctree::slicer::{HolographicSlice, SliceConfig};
use serde::{Deserialize, Serialize};
use std::path::PathBuf;

#[derive(Debug, Deserialize)]
pub struct SliceParams {
    pub target: PathBuf,
    #[serde(default)]
    pub consumers: bool,
    pub depth: Option<usize>,
    pub project: Option<PathBuf>,
}

#[derive(Debug, Serialize)]
pub struct SliceResponse {
    pub core: Vec<FileEntry>,
    pub deps: Vec<FileEntry>,
    pub consumers: Vec<FileEntry>,
    pub total_files: usize,
    pub total_loc: usize,
}

pub fn handle(snapshot: &Snapshot, params: SliceParams) -> SliceResponse {
    let cfg = SliceConfig {
        target: params.target.clone(),
        include_consumers: params.consumers,
        max_depth: params.depth,
        ..Default::default()
    };
    let slice = HolographicSlice::compute(snapshot, &cfg);
    // Map slice into response (paths + LOC + lang only — no content)
    SliceResponse { /* ... */ }
}
```

## Verification

```bash
make precheck
cargo test -p loctree-lsp slice_request
```

## Exit contract

- COMMIT: `feat(lsp): expose loctree/slice custom request`.
- REPORT: `.vibecrafted/reports/lsp/05-loctree-slice-request.md` with sample
  request/response payloads.

## Non-goals

- No content embedding. Paths only — agent reads files separately.
- No transitive consumer expansion beyond `depth` (default 1 if unset).

Vibecrafted with AI Agents (c)2024-2026 The LibraxisAI Team
