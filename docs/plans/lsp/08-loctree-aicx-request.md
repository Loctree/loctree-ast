---
name: loctree-aicx-request
status: queued
agent_target: any
project: loctree-suite
priority: 8
created: 2026-05-05
parent_branch: feat/context-tool-alpha
note: memory continuity between agents — strategic feature
---

# Plan 8 — `loctree/aicx` request (memory continuity)

## Why

When Codex / Claude / Junie / Gemini work on the same repo, each starts
with a fresh context. AICX overlay (`aicx intents`, `aicx_search`) holds
the team's prior decisions, intentions, outcomes. Today, only `aicx-mcp`
exposes this; the LSP daemon does not. Surfacing it here means **any
LSP-connected agent inherits the team's memory** for the bucket of files
it's about to touch.

## Acceptance criteria

- [ ] `loctree/aicx` custom request in `loctree-lsp/src/backend.rs`.
- [ ] Params: `{ scope: "file"|"symbol"|"project", target: Option<String>,
      kinds: Option<Vec<String>>, hours: Option<u64>, limit: Option<usize>,
      project: Option<PathBuf> }`.
- [ ] Response: `{ entries: [{kind, text, authority, source_chunk,
      agent, date, timestamp, session_id, project, relevance}],
      source_chunks: [String], scope_keywords_used: [String] }`.
- [ ] Reuses `loctree::aicx::AicxClient` already wired into the analyzer
  (see `loctree-rs/src/cli/dispatch/handlers/context/mod.rs::compose_memory_slice`).
- [ ] When AICX CLI is unavailable, response carries
  `{ status: "aicx_unavailable", hint: "install aicx or set LOCT_AICX_BINARY" }`.
- [ ] Capability `experimental.loctree/aicx = { available: true }`.
- [ ] Integration test in `loctree-lsp/tests/aicx_request.rs` (with
  `aicx_unavailable` graceful path mocked).

## Files

- `loctree-lsp/src/backend.rs` — capability + dispatch.
- `loctree-lsp/src/aicx.rs` (NEW) — handler.
- `loctree-lsp/tests/aicx_request.rs` (NEW).

## Implementation sketch

```rust
// loctree-lsp/src/aicx.rs
use loctree::aicx::{AicxClient, ScopeKeywords, score_intent, authority_for_intent};
use serde::{Deserialize, Serialize};

#[derive(Debug, Deserialize)]
pub struct AicxParams {
    pub scope: String,
    pub target: Option<String>,
    pub kinds: Option<Vec<String>>,
    pub hours: Option<u64>,
    pub limit: Option<usize>,
    pub project: Option<PathBuf>,
}

pub fn handle(snapshot: &Snapshot, params: AicxParams, project_id: &str) -> AicxResponse {
    let client = AicxClient::new(project_id.to_string());
    if !loctree::aicx::is_aicx_available() {
        return AicxResponse::unavailable("install aicx CLI or set LOCT_AICX_BINARY");
    }
    let keywords = build_scope_keywords_from_target(&params, snapshot);
    let raw = client.intents(params.hours.unwrap_or(720), params.limit.unwrap_or(50));
    let scored = raw.into_iter()
        .filter_map(|intent| {
            let s = score_intent(&intent, &keywords);
            (s > 0).then(|| (s, intent))
        })
        .collect::<Vec<_>>();
    // ... sort, map to response shape
}
```

## Verification

```bash
make precheck
cargo test -p loctree-lsp aicx_request
# Manual smoke (with aicx CLI installed):
echo '{"jsonrpc":"2.0","id":1,"method":"loctree/aicx",
       "params":{"scope":"file","target":"loctree-rs/src/snapshot.rs"}}' \
  | loctree-lsp
```

## Exit contract

- COMMIT: `feat(lsp): expose loctree/aicx for memory continuity`.
- REPORT: `.vibecrafted/reports/lsp/08-loctree-aicx-request.md`.

## Non-goals

- No write-side AICX (recording new intents) — read-only for this plan.
  Writes happen via the agent's own AICX integration.
- No automatic AICX re-fetch on every request — caller controls `hours`/`limit`.

Vibecrafted with AI Agents (c)2024-2026 The LibraxisAI Team
