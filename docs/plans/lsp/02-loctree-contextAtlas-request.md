---
name: loctree-contextAtlas-request
status: queued
agent_target: any
project: loctree-suite
priority: 2
created: 2026-05-05
parent_branch: feat/context-tool-alpha
depends_on: 01-atlas-per-repo
---

# Plan 2 — Custom LSP request `loctree/contextAtlas`

## Why

LSP today is wired for human IDE flows (hover, codeAction, diagnostics).
For AI agents in pipelines (Cursor, Codex CLI, Claude Code with LSP client,
Junie, etc.) we need a **first-class agent-context API** that:

- Returns a small JSON pointer (kilobytes), not 124 KB inline payload.
- Sidesteps the "host truncates / dumps to temp / agent doesn't follow"
  payload-loss problem the operator already observed.
- Lets the agent open the cards on disk by themselves (file open is fast,
  cached, and survives host truncation).

The atlas already exists on disk after Plan 1 (`<root>/.loctree/context-atlas/`).
We just need to expose a typed LSP request that returns the manifest pointer.

## Acceptance criteria

- [ ] LSP backend (`loctree-lsp/src/backend.rs`) handles a custom request
  method `"loctree/contextAtlas"`.
- [ ] Request params: `{ project: Option<PathBuf> }` (defaults to workspace root).
- [ ] Response shape: `{ atlas_dir, manifest, manifest_json, recommended_start,
      cards: [{id, title, path, lines, why}], message, status: "ready"|"missing" }`
- [ ] If atlas does not exist on disk yet, response carries `status: "missing"`
  plus `next_action: "loct auto"` (no panic, no error — agent decides).
- [ ] Method is exposed via `tower_lsp::LanguageServer::request` or the
  crate's custom-method mechanism (whichever the existing backend uses).
- [ ] Capability is advertised in `ServerCapabilities` (using the
  `experimental` field — `loctree/contextAtlas: { available: true }`).
- [ ] Integration test: spawn loctree-lsp, send `loctree/contextAtlas`
  request via mock client, assert structured response. Place under
  `loctree-lsp/tests/`.

## Files to modify

- `loctree-lsp/src/backend.rs:185-210` — add `experimental` capability and
  request handler dispatch.
- `loctree-lsp/src/lib.rs` or new module `loctree-lsp/src/context_atlas.rs`
  — ContextAtlasRequest/Response types + handler implementation.
- `loctree-lsp/Cargo.toml` — likely no new deps (uses
  `loctree::atlas::ContextAtlasInfo` re-export — see types.rs sibling).
- `loctree-lsp/tests/context_atlas_request.rs` (NEW) — integration test.

## Implementation sketch

```rust
// loctree-lsp/src/context_atlas.rs
use serde::{Deserialize, Serialize};
use std::path::PathBuf;

#[derive(Debug, Clone, Deserialize)]
pub struct ContextAtlasParams {
    pub project: Option<PathBuf>,
}

#[derive(Debug, Clone, Serialize)]
pub struct ContextAtlasResponse {
    pub status: String,                // "ready" | "missing"
    pub atlas_dir: Option<String>,
    pub manifest: Option<String>,
    pub manifest_json: Option<String>,
    pub recommended_start: Option<String>,
    pub cards: Vec<CardPointer>,
    pub message: String,
    pub next_action: Option<String>,   // e.g. "loct auto"
}

#[derive(Debug, Clone, Serialize)]
pub struct CardPointer {
    pub id: String,
    pub title: String,
    pub path: String,
    pub lines: usize,
    pub why: String,
}

pub fn handle(workspace_root: &Path, params: ContextAtlasParams) -> ContextAtlasResponse {
    let project = params.project.unwrap_or_else(|| workspace_root.to_path_buf());
    let manifest_json = project.join(".loctree/context-atlas/manifest.json");
    if !manifest_json.exists() {
        return ContextAtlasResponse {
            status: "missing".into(),
            atlas_dir: None,
            manifest: None,
            manifest_json: None,
            recommended_start: None,
            cards: vec![],
            message: "Run `loct auto` to materialize the atlas.".into(),
            next_action: Some("loct auto".into()),
        };
    }
    // Read + deserialize, then map to ContextAtlasResponse with status "ready".
    // ... (see loctree::analyzer::html::load_atlas_info for the parse pattern)
}
```

```rust
// loctree-lsp/src/backend.rs (in capabilities + dispatcher)
let mut experimental = serde_json::Map::new();
experimental.insert(
    "loctree/contextAtlas".to_string(),
    serde_json::json!({ "available": true }),
);
capabilities.experimental = Some(serde_json::Value::Object(experimental));

// Custom request handler — tower-lsp pattern (see existing similar code if any)
async fn handle_custom_request(&self, method: &str, params: serde_json::Value) -> ... {
    if method == "loctree/contextAtlas" {
        let ps: ContextAtlasParams = serde_json::from_value(params)?;
        return Ok(serde_json::to_value(context_atlas::handle(&self.workspace_root, ps))?);
    }
    // ...
}
```

## Verification

```bash
make precheck
cargo test -p loctree-lsp                                          # unit + integration
# Manual smoke (with a JSONRPC mock client or `lsp-tester` if available):
echo '{"jsonrpc":"2.0","id":1,"method":"loctree/contextAtlas","params":{}}' | loctree-lsp
```

## Exit contract

- COMMIT: `feat(lsp): expose loctree/contextAtlas custom request`.
- REPORT: `.vibecrafted/reports/lsp/02-loctree-contextAtlas-request.md` with
  request/response example payloads + integration test path.
- DEPENDENCY: must land after Plan 1 (atlas-per-repo) so the path is stable.

## Non-goals

- Do NOT push atlas via `workspace/notification` — agents pull on demand.
- Do NOT inline card content in the response — only paths. Cards on disk
  are the source of truth.
- Do NOT auto-trigger `loct auto` when missing — agent decides whether to
  scan based on its own policy.

Vibecrafted with AI Agents (c)2024-2026 The LibraxisAI Team
