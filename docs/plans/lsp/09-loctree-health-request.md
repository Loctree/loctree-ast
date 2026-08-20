---
name: loctree-health-request
status: queued
agent_target: any
project: loctree-suite
priority: 9
created: 2026-05-05
parent_branch: feat/context-tool-alpha
---

# Plan 9 — `loctree/health` request (repo readiness gate)

## Why

Agent at session start asks: "is this repo healthy enough to refactor?
Are there pending cycles / dead exports / stale snapshot?" Today CLI has
`loct health` and the analyzer has `health_score` (0-100) attached to
each `ReportSection`. LSP exposes none of it.

This is also a **readiness gate** — agent can refuse to do destructive
edits if `health_score < 50` or if there are unresolved cycles. Cheap and
high-value.

## Acceptance criteria

- [ ] `loctree/health` custom request in `loctree-lsp/src/backend.rs`.
- [ ] Params: `{ project: Option<PathBuf>, include_top_risks: bool }`.
- [ ] Response: `{ health_score: u8, status: "green"|"yellow"|"red",
      cycles: usize, dead_exports: usize, twins: usize,
      hotspots: usize, snapshot_stale: bool, snapshot_age_seconds: u64,
      top_risks: [{kind, file, severity, message}], recommended_actions: [String] }`.
- [ ] Status mapping: green ≥80, yellow 50-79, red <50.
- [ ] Capability `experimental.loctree/health = { available: true }`.
- [ ] Integration test in `loctree-lsp/tests/health_request.rs`.

## Files

- `loctree-lsp/src/backend.rs` — capability + dispatch.
- `loctree-lsp/src/health.rs` (NEW) — handler delegating to
  `loctree::analyzer::output::compute_health_score` (or whatever the
  current health composer is — check `loct health` CLI flow).
- `loctree-lsp/tests/health_request.rs` (NEW).

## Implementation sketch

```rust
// loctree-lsp/src/health.rs
use serde::{Deserialize, Serialize};

#[derive(Debug, Deserialize)]
pub struct HealthParams {
    pub project: Option<PathBuf>,
    #[serde(default)]
    pub include_top_risks: bool,
}

#[derive(Debug, Serialize)]
pub struct HealthResponse {
    pub health_score: u8,
    pub status: String,
    pub cycles: usize,
    pub dead_exports: usize,
    pub twins: usize,
    pub hotspots: usize,
    pub snapshot_stale: bool,
    pub snapshot_age_seconds: u64,
    pub top_risks: Vec<RiskItem>,
    pub recommended_actions: Vec<String>,
}

pub fn handle(snapshot: &Snapshot, params: HealthParams) -> HealthResponse {
    let cycles = snapshot.compute_cycles().len();
    let dead = snapshot.compute_dead_exports().len();
    let twins = snapshot.compute_twins().len();
    let hotspots = snapshot.top_hotspots(5).len();
    let snapshot_age = snapshot_age_seconds(snapshot);
    let score = compute_score(cycles, dead, twins, hotspots, snapshot_age);
    let status = match score {
        80..=100 => "green",
        50..=79 => "yellow",
        _ => "red",
    };
    HealthResponse { /* ... */ }
}
```

## Verification

```bash
make precheck
cargo test -p loctree-lsp health_request
```

## Exit contract

- COMMIT: `feat(lsp): expose loctree/health as readiness gate`.
- REPORT: `.vibecrafted/reports/lsp/09-loctree-health-request.md`.

## Non-goals

- No automatic remediation. Response carries `recommended_actions` strings;
  caller decides whether to invoke them.
- No historical health trend (snapshot diff) — that's Plan 11's territory.

Vibecrafted with AI Agents (c)2024-2026 The LibraxisAI Team
