---
name: loctree-find-request
status: queued
agent_target: any
project: loctree-suite
priority: 7
created: 2026-05-05
parent_branch: feat/context-tool-alpha
---

# Plan 7 — `loctree/find` request (semantic-aware symbol search)

## Why

`textDocument/references` returns syntactic matches. Loctree knows
**semantics** — exports vs imports vs params vs symbols, cross-language.
Agent should query "find symbol X" and get categorized hits with line
numbers, similarity scores, and dead-code status — same shape as
`loct find` CLI output (recently enhanced with line numbers in
`SimilarityCandidate`, see commit history on this branch).

## Acceptance criteria

- [ ] `loctree/find` custom request in `loctree-lsp/src/backend.rs`.
- [ ] Params: `{ query: String, mode: "single"|"split"|"and",
      lang: Option<String>, dead_only: bool, exported_only: bool,
      limit: Option<usize>, project: Option<PathBuf> }`.
- [ ] Response (mirrors existing CLI shape — see
  `loctree-rs/src/analyzer/search.rs::SearchResults`):
  `{ symbol_matches: [...], param_matches: [...],
  semantic_matches: [{symbol, file, line, score}],
  dead_status: {...}, cross_matches: [...] }`.
- [ ] Each match carries `line: Option<usize>` (just landed in
  `SimilarityCandidate` on this branch — propagate to LSP shape).
- [ ] Capability `experimental.loctree/find = { available: true }`.
- [ ] Integration test in `loctree-lsp/tests/find_request.rs`.

## Files

- `loctree-lsp/src/backend.rs` — capability + dispatch.
- `loctree-lsp/src/find.rs` (NEW) — handler delegating to
  `loctree::analyzer::search::run_search` (or equivalent).
- `loctree-lsp/tests/find_request.rs` (NEW).

## Implementation sketch

```rust
// loctree-lsp/src/find.rs
use loctree::analyzer::search::{SearchResults, run_search};
use serde::{Deserialize, Serialize};

#[derive(Debug, Deserialize)]
pub struct FindParams {
    pub query: String,
    #[serde(default = "default_mode")]
    pub mode: String,
    pub lang: Option<String>,
    #[serde(default)]
    pub dead_only: bool,
    #[serde(default)]
    pub exported_only: bool,
    pub limit: Option<usize>,
    pub project: Option<PathBuf>,
}

fn default_mode() -> String { "single".into() }

#[derive(Debug, Serialize)]
pub struct FindResponse {
    // Mirror SearchResults but flatten for JSON-RPC
    pub symbol_matches: Vec<SymbolMatch>,
    pub semantic_matches: Vec<SemanticMatch>,
    // ...
}

pub fn handle(snapshot: &Snapshot, params: FindParams) -> FindResponse {
    let mut search_args = build_search_args(&params);
    let results = run_search(snapshot, &search_args);
    // Map to FindResponse, propagating .line on every match.
}
```

## Verification

```bash
make precheck
cargo test -p loctree-lsp find_request
```

## Exit contract

- COMMIT: `feat(lsp): expose loctree/find with semantic-aware results`.
- REPORT: `.vibecrafted/reports/lsp/07-loctree-find-request.md`.

## Non-goals

- No regex compilation on the LSP side beyond what `run_search` already does.
- Results capped at `limit` (default 50) — pagination via cursor pattern
  (Plan 12) is a separate concern.

Vibecrafted with AI Agents (c)2024-2026 The LibraxisAI Team
