---
name: loctree-impact-request
status: queued
agent_target: any
project: loctree-suite
priority: 6
created: 2026-05-05
parent_branch: feat/context-tool-alpha
---

# Plan 6 — `loctree/impact` request

## Why

Pre-refactor / pre-delete blast radius. Agent asks: "if I change/remove this
file, what breaks?" CLI has `loct impact <file>` and `loct query who-imports`;
MCP has `mcp_loctree-mcp_impact`. LSP must too — agents in pipelines
preparing destructive edits need this in one call.

## Acceptance criteria

- [ ] `loctree/impact` custom request in `loctree-lsp/src/backend.rs`.
- [ ] Params: `{ target: PathBuf, transitive: bool, project: Option<PathBuf> }`.
- [ ] Response: `{ direct: [{path, depth: 1}], transitive: [{path, depth}],
      total: usize, blast_severity: "low"|"medium"|"high",
      warnings: [String] }`.
- [ ] Severity heuristic: `low` (<5 importers), `medium` (5-20),
  `high` (>20 OR depth >3).
- [ ] Warnings: dynamic-import edges flagged separately ("X uses dynamic
  imports — runtime impact may differ from static").
- [ ] Capability `experimental.loctree/impact = { available: true }`.
- [ ] Integration test in `loctree-lsp/tests/impact_request.rs`.

## Files

- `loctree-lsp/src/backend.rs` — capability + dispatch.
- `loctree-lsp/src/impact.rs` (NEW) — handler delegating to
  `loctree::impact::ImpactAnalysis` (or equivalent — see existing
  `loct impact` flow in `loctree-rs/src/cli/dispatch/handlers/diff.rs`).
- `loctree-lsp/tests/impact_request.rs` (NEW).

## Implementation sketch

```rust
// loctree-lsp/src/impact.rs
use serde::{Deserialize, Serialize};

#[derive(Debug, Deserialize)]
pub struct ImpactParams {
    pub target: PathBuf,
    #[serde(default)]
    pub transitive: bool,
    pub project: Option<PathBuf>,
}

#[derive(Debug, Serialize)]
pub struct ImpactResponse {
    pub direct: Vec<Importer>,
    pub transitive: Vec<Importer>,
    pub total: usize,
    pub blast_severity: String,
    pub warnings: Vec<String>,
}

pub fn handle(snapshot: &Snapshot, params: ImpactParams) -> ImpactResponse {
    let direct = loctree::query::query_who_imports(snapshot, &params.target);
    let transitive = if params.transitive {
        // BFS over snapshot.edges
    } else { vec![] };
    let total = direct.len() + transitive.len();
    let blast_severity = match (total, max_depth) {
        (0..=4, _) => "low",
        (5..=20, d) if d <= 3 => "medium",
        _ => "high",
    };
    // ... warnings
    ImpactResponse { /* ... */ }
}
```

## Verification

```bash
make precheck
cargo test -p loctree-lsp impact_request
```

## Exit contract

- COMMIT: `feat(lsp): expose loctree/impact custom request`.
- REPORT: `.vibecrafted/reports/lsp/06-loctree-impact-request.md`.

## Non-goals

- No code modification suggestions — pure analysis.
- No automatic refactor preview — separate plan if/when needed.

Vibecrafted with AI Agents (c)2024-2026 The LibraxisAI Team
