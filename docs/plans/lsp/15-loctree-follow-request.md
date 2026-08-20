---
name: loctree-follow-request
status: done
agent_target: any
project: loctree-suite
priority: 15
created: 2026-05-05
last_updated: 2026-05-08
parent_branch: feat/context-tool-alpha
---

# Plan 15 — `loctree/follow` request (consolidated structural signals)

## Why

CLI's `loct follow <scope>` and MCP's `mcp_loctree-mcp_follow` consolidate
seven structural concerns under one verb: `cycles`, `dead`, `twins`,
`hotspots`, `trace`, `commands`, `events`, `pipelines`, `all`. LSP
should mirror this so agents have one entry point for "show me the
structural smells in this scope" instead of seven separate requests.

## P0 Stage 2 truth pass (2026-05-08)

Plan 15 v1 landed on 2026-05-07 with `cycles` / `dead` / `twins` /
`hotspots` / `all` wired and `trace` / `commands` / `events` /
`pipelines` returning `unsupported` envelopes. Stage 2 closed the
remaining honest scopes and made the stub boundary discoverable from the
capability surface (commit `b5f6e308`):

- `loctree-lsp/src/follow.rs`: `commands` and `events` now project
  `Snapshot::command_bridges` / `Snapshot::event_bridges` directly;
  `pipelines` reduces both bridges to a structural pipeline view with a
  `note` that points operators at `loct pipelines` for the full
  ghost/orphan/race analysis (which depends on FE/BE command-usage maps
  the daemon does not own).
- New `IMPLEMENTED_SCOPES` and `STUB_SCOPES` constants partition the
  advertised vocabulary; `trace` is the only stub left and returns the
  honest `not implemented in loctree-lsp yet — request via `loct trace``
  envelope rather than pretending to be live.
- `loctree-lsp/src/backend.rs`: capability JSON declares
  `loctree/follow.{ scopes, implemented_scopes, stub_scopes,
  stub_reason }` so agents can skip stubs without round-tripping.
- `loctree-lsp/tests/follow_request.rs` adds 4 scope-specific integration
  tests (commands / events / pipelines / trace stub envelope) on top of
  the v1 cycles/dead/twins/hotspots/all coverage; `follow.rs` adds 4 unit
  tests for the IMPLEMENTED ∪ STUB == SUPPORTED partition.

## Acceptance criteria

- [x] `loctree/follow` custom request in `loctree-lsp/src/backend.rs`.
- [x] Params: `{ scope: "cycles"|"dead"|"twins"|"hotspots"|"trace"|
      "commands"|"events"|"pipelines"|"all",
      handler: Option<String>,    // for trace
      limit: Option<usize>,
      project: Option<PathBuf> }`.
- [x] Response shape varies by scope but keeps a stable envelope:
  `{ scope: String, items: [...], summary: { count: usize,
  severity: "low"|"medium"|"high" } }`.
- [x] When `scope = "all"`, response is keyed map of all individual
  scopes (mirroring CLI's combined flow).
- [x] Capability `experimental.loctree/follow = { available: true,
      scopes, implemented_scopes, stub_scopes, stub_reason }` — Stage 2
  truth-pass partition replaces the original `scopes`-only shape.
- [x] Integration test in `loctree-lsp/tests/follow_request.rs` covers
  cycles, dead, twins, hotspots, all, commands, events, pipelines,
  and the trace stub envelope (14 integration tests + 11 unit tests).
- [x] `trace` honestly stubbed: handler-graph walker is not yet portable
  from `handle_trace_command` (CLI shells through analyzer) — wiring
  it from LSP is queued behind a library-backed trace path.

## Files

- `loctree-lsp/src/backend.rs` — capability + dispatch.
- `loctree-lsp/src/follow.rs` (NEW) — handler.
- `loctree-lsp/tests/follow_request.rs` (NEW).

## Implementation sketch

```rust
// loctree-lsp/src/follow.rs
use serde::{Deserialize, Serialize};

#[derive(Debug, Deserialize)]
pub struct FollowParams {
    pub scope: String,
    pub handler: Option<String>,
    pub limit: Option<usize>,
    pub project: Option<PathBuf>,
}

#[derive(Debug, Serialize)]
pub struct FollowResponse {
    pub scope: String,
    pub items: serde_json::Value,        // shape varies by scope
    pub summary: FollowSummary,
}

pub fn handle(snapshot: &Snapshot, params: FollowParams) -> FollowResponse {
    match params.scope.as_str() {
        "cycles" => follow_cycles(snapshot, params.limit),
        "dead" => follow_dead(snapshot, params.limit),
        "twins" => follow_twins(snapshot, params.limit),
        "hotspots" => follow_hotspots(snapshot, params.limit),
        "trace" => follow_trace(snapshot, params.handler),
        "commands" => follow_commands(snapshot),
        "events" => follow_events(snapshot),
        "pipelines" => follow_pipelines(snapshot),
        "all" => follow_all(snapshot, params.limit),
        _ => FollowResponse::error(&format!("unknown scope: {}", params.scope)),
    }
}
```

## Verification

```bash
make precheck
cargo test -p loctree-lsp follow_request
# Manual sanity:
echo '{"jsonrpc":"2.0","id":1,"method":"loctree/follow",
       "params":{"scope":"cycles","limit":5}}' | loctree-lsp
```

## Exit contract

- COMMIT: `feat(lsp): consolidated loctree/follow request`.
- REPORT: `.vibecrafted/reports/lsp/15-loctree-follow-request.md`.

## Non-goals

- No new analyzer logic — purely a thin LSP adapter over existing
  `loct follow` flow under `loctree-rs/src/cli/dispatch/handlers/analysis.rs`.
- No "fix" actions — read-only signal report.

Vibecrafted with AI Agents (c)2024-2026 The LibraxisAI Team
