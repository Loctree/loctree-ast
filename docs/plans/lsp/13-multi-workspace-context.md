---
name: multi-workspace-context
status: queued
agent_target: any
project: loctree-suite
priority: 13
created: 2026-05-05
parent_branch: feat/context-tool-alpha
note: infrastructure — cross-cuts 5,6,7,8,9,15 via scope param
---

# Plan 13 — Multi-workspace context (monorepo subprojects)

## Why

Vista is a monorepo: `vista/` (frontend root) + `vista/src-tauri/` (Tauri
backend) + `vista/src/features/ai-suite/` (sub-features). Today each
sub-projekt has its own `.loctree/` (the bug Monika hit was related — see
this branch's regression test). LSP daemon should serve **all** of them
from one process, with `project` param routing each request to the right
snapshot.

## Acceptance criteria

- [ ] Daemon discovers sub-projects at `initialized` time:
  walks workspace root, finds every `.loctree/` directory (limit
  depth: 4 by default, configurable).
- [ ] Each sub-project has its own snapshot loaded into a
  `HashMap<PathBuf, Arc<RwLock<Snapshot>>>` keyed by canonicalized
  project root.
- [ ] Every request that accepts `project: Option<PathBuf>` (Plans 2, 5,
  6, 7, 8, 9, 11, 14, 15) routes to the matching snapshot. Default
  is workspace root.
- [ ] New request `loctree/workspaces` returns
  `{ workspaces: [{root, files, languages, snapshot_age_seconds}] }`
  so agents can enumerate sub-projects.
- [ ] Stale-aware: each sub-project's watcher (Plan 10) refreshes its
  own snapshot independently.
- [ ] Capability `experimental.loctree/workspaces = { available: true }`.
- [ ] Integration test in `loctree-lsp/tests/multi_workspace.rs` with a
  fixture monorepo containing two sub-projects.

## Files

- `loctree-lsp/src/backend.rs` — multi-snapshot map + dispatch routing.
- `loctree-lsp/src/workspaces.rs` (NEW) — discovery + enumerate request.
- `loctree-lsp/tests/multi_workspace.rs` (NEW).

## Implementation sketch

```rust
// loctree-lsp/src/workspaces.rs
pub fn discover_workspaces(root: &Path, max_depth: usize) -> Vec<PathBuf> {
    let mut out = vec![root.to_path_buf()];      // root always included
    walk_for_loctree_dirs(root, &mut out, max_depth);
    out.sort();
    out.dedup();
    out
}

#[derive(Debug, Serialize)]
pub struct WorkspacesResponse {
    pub workspaces: Vec<WorkspaceInfo>,
}

pub struct Backend {
    snapshots: Arc<RwLock<HashMap<PathBuf, Arc<RwLock<Snapshot>>>>>,
    workspace_root: PathBuf,
}

impl Backend {
    fn route(&self, project: Option<PathBuf>) -> Arc<RwLock<Snapshot>> {
        let key = project.unwrap_or_else(|| self.workspace_root.clone());
        let canonical = key.canonicalize().unwrap_or(key);
        self.snapshots.read().get(&canonical).cloned()
            .unwrap_or_else(|| /* fallback to workspace root */)
    }
}
```

## Verification

```bash
make precheck
cargo test -p loctree-lsp multi_workspace
# Manual smoke on vista monorepo:
echo '{"jsonrpc":"2.0","id":1,"method":"loctree/workspaces","params":{}}' \
  | loctree-lsp --workspace-root /Users/maciejgad/vc-workspace/vetcoders/vista
```

## Verification

```bash
make precheck
cargo test -p loctree-lsp multi_workspace
```

## Exit contract

- COMMIT: `feat(lsp): multi-workspace snapshot routing`.
- REPORT: `.vibecrafted/reports/lsp/13-multi-workspace-context.md`.

## Non-goals

- No cross-workspace queries (e.g. `loctree/find` spanning all sub-projects
  in one call). Each request stays scoped — caller can dispatch in parallel.
- No automatic atlas materialization for every sub-projekt — keep that
  CLI-driven (`loct auto` per directory).

Vibecrafted with AI Agents (c)2024-2026 The LibraxisAI Team
