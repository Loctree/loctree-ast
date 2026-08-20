---
name: loctree-semantic-request
status: done
agent_target: any
project: loctree-suite
priority: 14
created: 2026-05-05
last_updated: 2026-05-08
parent_branch: feat/context-tool-alpha
note: top-tier diferentiator — semantic facts unique to loctree
---

# Plan 14 — `loctree/semantic` request (idiom tags + dispatch + env)

## Why

This is loctree's **unique selling point** vs rust-analyzer / pyright /
ts-server: we know the *meaning* layer — idiom tags, dispatch edges,
reachability reasons, env-var contracts, Tauri command bridges, framework
hints. They live in `SemanticFacts` (see `loctree-rs/src/semantic/mod.rs`)
but no LSP request exposes them. Agent that can read these is operating
two tiers above any other code-aware AI.

## P0 Stage 2 truth pass (2026-05-08)

Plan 14 v1 landed on 2026-05-07 (commit `5b9aeccb`); Stage 2 hardened the
boundary so capability advertisements no longer overclaim symbol scope:

- `loctree-lsp/src/semantic.rs`: `scope = "symbol"` returns
  `status: "symbol_scope_unimplemented"` with a `hint` pointing operators
  at `scope = "file"` for their containing file.
- `loctree-lsp/src/backend.rs`: capability JSON now declares
  `loctree/semantic.{ supported_scopes, deferred_scopes, deferral_reason }`
  (`supported_scopes = ["file", "project"]`,
  `deferred_scopes = ["symbol"]`) so clients can probe maturity without
  paying a request.
- The `symbol` v2 deferral is bound to Plan 16 (tree-sitter substrate —
  Stage 1 landed 2026-05-08) plus Plan 18 v2 (stable byte-range
  `SymbolId`); `loctree::types::SymbolIdV1::VERSION = "v1-string"` is
  the `<file>::<symbol>` wire contract until then.

## Acceptance criteria

- [x] `loctree/semantic` custom request in `loctree-lsp/src/backend.rs`.
- [x] Params: `{ scope: "file"|"symbol"|"project",
      target: Option<String>, kinds: Option<Vec<String>>,
      project: Option<PathBuf> }`.
  Kinds filter (default = all): `idiom_tags`, `dispatch_edges`,
  `reachability`, `env_contracts`, `tauri_commands`, `tauri_events`,
  `framework_hints`.
- [x] Response: `{ idiom_tags: [...], dispatch_edges: [...],
      reachability: [...], env_contracts: [...], tauri_commands: [...],
      tauri_events: [...], framework_hints: [...] }` —
  mirroring `loctree::pack::RuntimeSlice` shape.
- [x] Each entry carries `authority: AuthorityLabel` (RepoVerified |
  LoctreeDerived | SemanticGuess | StaleOrUnknown) so agent knows how
  much to trust each fact.
- [x] Capability `experimental.loctree/semantic = { available: true,
      supported_scopes, deferred_scopes, deferral_reason }` — Stage 2
  partition replaces the original `available: true` placeholder.
- [x] Integration test in `loctree-lsp/tests/semantic_request.rs` (7 tests
  plus 7 unit tests in `semantic.rs`).
- [x] `scope = "symbol"` returns `symbol_scope_unimplemented` with hint —
  v2 lift queued behind Plan 18 stable `SymbolId`.

## Files

- `loctree-lsp/src/backend.rs` — capability + dispatch.
- `loctree-lsp/src/semantic.rs` (NEW) — handler delegating to
  `loctree::pack::compose_runtime_slice` (or equivalent — see existing
  composition path under `handlers/context/mod.rs`).
- `loctree-lsp/tests/semantic_request.rs` (NEW).

## Implementation sketch

```rust
// loctree-lsp/src/semantic.rs
use loctree::pack::{ContextOptions, compose_runtime_slice};
use serde::{Deserialize, Serialize};

#[derive(Debug, Deserialize)]
pub struct SemanticParams {
    pub scope: String,
    pub target: Option<String>,
    pub kinds: Option<Vec<String>>,
    pub project: Option<PathBuf>,
}

pub fn handle(snapshot: &Snapshot, params: SemanticParams) -> SemanticResponse {
    let opts = ContextOptions {
        file: params.target.as_ref().map(PathBuf::from),
        project: params.project.clone(),
        ..Default::default()
    };
    let runtime = compose_runtime_slice(&opts, snapshot);

    let kinds: HashSet<String> = params.kinds
        .unwrap_or_else(|| ALL_KINDS.iter().map(|s| s.to_string()).collect())
        .into_iter().collect();

    SemanticResponse {
        idiom_tags: kinds.contains("idiom_tags").then(|| runtime.idiom_tags).unwrap_or_default(),
        dispatch_edges: kinds.contains("dispatch_edges").then(|| runtime.dispatch_edges).unwrap_or_default(),
        // ...
    }
}
```

## Verification

```bash
make precheck
cargo test -p loctree-lsp semantic_request
```

## Exit contract

- COMMIT: `feat(lsp): expose loctree/semantic for idioms + dispatch + env`.
- REPORT: `.vibecrafted/reports/lsp/14-loctree-semantic-request.md`.

## Non-goals

- No automatic invalidation of semantic facts on edit — relies on Plan 10's
  watcher to refresh.
- No reach-causes graph traversal in this plan — return reachability as
  a list of `(symbol, reason)` pairs from the cached runtime slice only.

Vibecrafted with AI Agents (c)2024-2026 The LibraxisAI Team
