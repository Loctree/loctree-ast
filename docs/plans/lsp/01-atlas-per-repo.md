---
name: atlas-per-repo
status: queued
agent_target: any
project: loctree-suite
priority: 1
created: 2026-05-05
parent_branch: feat/context-tool-alpha
---

# Plan 1 — Atlas per-repo (`.loctree/context-atlas/`)

## Why

Today `atlas_dir_for_project` returns `Snapshot::artifacts_dir(root)` which is
documented as "global cache directory" — so even when a repo has its own
`.loctree/` (config dir), the materialized Context Atlas lands in
`~/Library/Caches/loctree/projects/<hash>/master@<commit>/context-atlas/`.

Consequences (already observed):

- LSP / MCP / IDE consumers can't rely on a stable per-repo path.
- `report.html` needs a `latest/` fallback in `load_atlas_info`.
- Operators are surprised: `.loctree/context-atlas/` is empty.
- No git-friendly story: cannot decide per-repo whether to commit or ignore
  atlas cards (e.g. for shared agent workspaces).

We want the atlas to live at `<repo_root>/.loctree/context-atlas/` always,
both for clarity and for the LSP custom-request work that follows
(Plan 2 reads from this exact path).

## Acceptance criteria

- [ ] `atlas_dir_for_project(root)` returns `<root>/.loctree/context-atlas/`
  regardless of cache dir overrides.
- [ ] `materialize_context_atlas` creates `<root>/.loctree/` if it does not
  exist (dir, not config files).
- [ ] `loctree-rs/src/analyzer/html.rs::load_atlas_info` simplified — drops
  the `latest/context-atlas/manifest.json` fallback (no longer needed).
- [ ] After `loct auto`, `<root>/.loctree/context-atlas/manifest.json` exists
  and contains the reading path. Verified on the project itself
  (loctree-suite) and on a fixture monorepo.
- [ ] `report.html` continues to show the Context Atlas Panel (regression
  check via existing smoke flow — see Verification below).
- [ ] Existing tests in `loctree-rs/tests/e2e_cli.rs` still pass.
- [ ] New regression test:
  `e2e_cli::auto_scan::auto_writes_atlas_to_dotloctree` that runs
  `loct auto` in a fixture and asserts presence of
  `.loctree/context-atlas/manifest.json`.

## Files to modify

- `loctree-rs/src/cli/dispatch/handlers/context/atlas.rs:89` — change
  `atlas_dir_for_project` to always use `<root>/.loctree/context-atlas/`.
- `loctree-rs/src/cli/dispatch/handlers/context/atlas.rs:88-96` — ensure
  `materialize_context_atlas` calls `fs::create_dir_all(&atlas_dir)` (it
  already does — confirm path is reachable).
- `loctree-rs/src/analyzer/html.rs::load_atlas_info` — remove the
  `latest/context-atlas/...` fallback branch; keep direct path only.
- `loctree-rs/tests/e2e_cli.rs` — add regression test (under `auto_scan`
  module) that asserts atlas materialization location.

## Implementation sketch

```rust
// loctree-rs/src/cli/dispatch/handlers/context/atlas.rs
pub fn atlas_dir_for_project(project_root: &Path) -> PathBuf {
    project_root.join(".loctree").join(CONTEXT_ATLAS_DIR)
}
```

```rust
// loctree-rs/src/analyzer/html.rs
fn load_atlas_info(artifacts_dir: &Path) -> Option<ContextAtlasInfo> {
    // Atlas now always lives at <repo_root>/.loctree/context-atlas/.
    // `artifacts_dir` here is the dir containing report.html, which sits
    // either at <repo_root>/.loctree/ (auto flow) or at a cache bucket
    // (deprecated fallback). Walk up until we find the .loctree/ dir.
    let manifest_json = if artifacts_dir.ends_with(".loctree") {
        artifacts_dir.join("context-atlas").join("manifest.json")
    } else {
        // Walk to find <X>/.loctree/context-atlas/manifest.json
        artifacts_dir
            .ancestors()
            .find_map(|a| {
                let candidate = a.join(".loctree/context-atlas/manifest.json");
                if candidate.exists() { Some(candidate) } else { None }
            })?
    };
    // ... existing parsing
}
```

## Verification

```bash
make precheck
cargo test -p loctree --test e2e_cli auto_scan::auto_writes_atlas_to_dotloctree
cargo test -p loctree --test e2e_cli                              # full suite
# Smoke on this project itself:
loct auto && ls -la .loctree/context-atlas/
# Smoke on fixture monorepo:
cd loctree-rs/tests/fixtures/tauri_app && loct auto && ls -la .loctree/context-atlas/
```

## Exit contract

- COMMIT: one commit, `feat(atlas): always materialize to <root>/.loctree/context-atlas/`
- REPORT: `.vibecrafted/reports/lsp/01-atlas-per-repo.md` with status (completed|failed),
  findings, and links to changed file lines.
- BRANCH: stack on top of `feat/context-tool-alpha`. Either rebase or new
  feature branch — operator decides at PR time.

## Non-goals

- Do NOT change the global cache layout. Cache still receives snapshot.json,
  agent.json, etc. Only the atlas migrates.
- Do NOT add a fallback to cache-based atlas. New location is canonical.
- Do NOT touch `Snapshot::artifacts_dir` semantics.

Vibecrafted with AI Agents (c)2024-2026 The LibraxisAI Team
