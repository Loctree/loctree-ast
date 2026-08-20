---
name: lsp-polish-pass
status: reconstructed-done
agent_target: any
project: loctree-suite
priority: 21
created: 2026-05-08
reconstructed: 2026-05-11
parent_branch: feat/context-tool-alpha
depends_on: 03-codelens-importers, 12-streaming-cursor-pattern, 13-multi-workspace-context, 20-loctree-astQuery-request
authority: reconstructed from `docs/plans/lsp/TRACKER.md` activity log and current code/test evidence; original report file was missing during the 22-task audit
---

# Plan 21 — LSP polish pass (branding, fixtures, schemas)

## Why

Operator dogfooding in RustRover surfaced three LSP rough edges after the
feature-heavy LSP batch had landed:

1. the server introduced itself with weak/technical branding instead of a
   stable product name,
2. empty `.loctree/` directories left in fixtures were discovered as broken
   sub-workspaces and produced recurring WARN noise,
3. `experimental.loctree/*` capability advertisements were mostly boolean
   availability flags, forcing editor clients to duplicate request parameter
   shapes instead of consuming server-published JSON Schemas.

This pass is intentionally a polish/stabilization task: no new request method,
no protocol semantics rewrite, and no broad editor-client implementation.

## Reconstruction note

The original standalone task/report pair was not present when the 22-task audit
ran. This file reconstructs the task from:

- `docs/plans/lsp/TRACKER.md` row 21,
- `docs/plans/lsp/TRACKER.md` activity entries for
  `2026-05-08T22:00:00Z` and `2026-05-08T22:25:00Z`,
- current code evidence in `loctree-lsp/src/backend.rs` and
  `loctree-lsp/src/workspaces.rs`,
- current regression evidence in `loctree-lsp/tests/cli.rs`,
  workspace-discovery tests, and the LSP crate test suite.

If a future agent finds the original `reports/lsp/21-lsp-polish-pass.md`, treat
that report as additional evidence, not as permission to skip code/test checks.

## Acceptance criteria

- [x] `initialize` responses advertise `serverInfo.name = "Loctree Language Server"`
  and keep the version wired to the crate version instead of a stale literal.
- [x] Workspace discovery records a nested `.loctree/` parent only when the
  marker contains a real `.loctree/snapshot.json`; empty fixture/init marker
  directories must not become routed sub-workspaces.
- [x] Discovery remains bounded and safe: pruned directories are still skipped,
  symlinks are not followed, duplicate canonical parents are deduplicated, and
  the root workspace is still handled by the primary snapshot handle rather than
  as an extra workspace.
- [x] Typed LSP request parameter structs for the shipped `loctree/*` surfaces
  derive `schemars::JsonSchema` where they are advertised through experimental
  capabilities.
- [x] `experimental.loctree/*` request capabilities use a shared helper for
  `{ available: true, requestSchema: ... }`, with per-request metadata merged
  through one shared extension path rather than ad hoc JSON shape drift.
- [x] Schema publication remains honest: non-request notifications or simple
  feature flags are not forced into fake request schemas.
- [x] Regression tests cover the RustRover-visible failures: server branding,
  empty `.loctree/` fixture discovery, and schema-bearing capability
  advertisements.
- [x] Round-trip evidence exists from at least the LSP crate gate; the tracker
  recorded `cargo test --workspace` plus `clippy -D warnings` green for the
  original completion claim, but future audits must re-run current gates instead
  of trusting that historical claim.

## Files

- `loctree-lsp/src/backend.rs` — `server_info()`, shared request capability
  builders, and experimental capability advertisement surface.
- `loctree-lsp/src/workspaces.rs` — sub-workspace discovery and empty-marker
  suppression.
- `loctree-lsp/tests/cli.rs` — initialize/serverInfo regression coverage.
- `loctree-lsp/tests/multi_workspace.rs` and workspace module tests —
  multi-workspace and empty-marker discovery coverage.
- Request modules under `loctree-lsp/src/` that derive `schemars::JsonSchema`
  for advertised params.

## Implementation sketch

```rust
pub fn server_info() -> ServerInfo {
    ServerInfo {
        name: "Loctree Language Server".to_string(),
        version: Some(env!("CARGO_PKG_VERSION").to_string()),
    }
}

fn request_capability(schema: schemars::Schema) -> serde_json::Value {
    serde_json::json!({
        "available": true,
        "requestSchema": schema,
    })
}
```

Workspace discovery keeps `.loctree/` in the prune set, but before pruning a
nested marker it records the parent only when `snapshot.json` exists. That keeps
real local snapshot mirrors routable while silencing empty fixture markers.

## Verification

Minimum verification for future audits or marble rounds touching this task:

```bash
cargo test -p loctree-lsp cli
cargo test -p loctree-lsp multi_workspace
cargo test -p loctree-lsp
cargo clippy --workspace --all-targets -- -D warnings
```

Recommended full close-out gate for the whole LSP batch:

```bash
make precheck
make test
```

## Exit contract

- COMMIT: `fix(lsp): polish server branding workspace discovery and schemas`.
- REPORT: `reports/lsp/21-lsp-polish-pass.md` with exact code/test evidence.
- TRACKER: row 21 remains `done`, but this reconstructed task file must be
  included in future audits as the task authority source.

## Non-goals

- No new `loctree/*` request method.
- No editor-specific UI implementation beyond server capability truth.
- No fake schemas for notifications or simple feature flags.
- No broad workspace discovery rewrite outside the empty-marker WARN-noise fix.
- No reliance on historical commit IDs or tracker claims without current
  code/test verification.

Vibecrafted with AI Agents (c)2024-2026 The LibraxisAI Team