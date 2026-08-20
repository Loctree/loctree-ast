---
name: background-watcher
status: queued
agent_target: any
project: loctree-suite
priority: 10
created: 2026-05-05
parent_branch: feat/context-tool-alpha
note: infrastructure plan — enables 11-diff
---

# Plan 10 — Background fs watcher + auto-rescan + scanProgress notifications

## Why

Today loctree-lsp loads snapshot once at `initialized` and never refreshes.
Agents that hold a long LSP session see stale data after every save. CLI
mode mitigates with `--fail-stale` and explicit rescan; daemon mode needs
**rust-analyzer-style background indexing**: detect fs events, debounce,
incremental rescan, push `loctree/scanProgress` notification so agents
don't poll.

## Acceptance criteria

- [ ] LSP backend uses `notify` crate (already a transitive dep via
  analyzer's watch flow — confirm) to subscribe to workspace fs events.
- [ ] Debounced batch (200-500ms): collected events trigger an
  incremental rescan via existing
  `loctree::snapshot::run_init_with_options_for_strategy(Incremental)`.
- [ ] During scan, server emits notification
  `loctree/scanProgress { phase: "scanning"|"composing"|"done",
  files_processed: usize, total_files: usize, eta_seconds: Option<f64> }`.
- [ ] Snapshot in daemon RAM is replaced atomically when the rescan
  completes (no torn-state queries).
- [ ] Configurable via init options:
  `loctree.watcher.enabled` (default true),
  `loctree.watcher.debounceMs` (default 300),
  `loctree.watcher.includePatterns` / `excludePatterns`.
- [ ] Capability `experimental.loctree/scanProgress = { available: true }`.
- [ ] Test: `loctree-lsp/tests/watcher_smoke.rs` writes a file in tempdir,
  confirms `loctree/scanProgress` notification arrives within 1s.

## Files

- `loctree-lsp/src/backend.rs` — start watcher in `initialized`, stop in
  `shutdown`, dispatch notifications.
- `loctree-lsp/src/watcher.rs` (NEW) — fs watcher + debounce + rescan
  trigger.
- `loctree-lsp/tests/watcher_smoke.rs` (NEW).
- `loctree-lsp/Cargo.toml` — confirm `notify` is listed (add if not).

## Implementation sketch

```rust
// loctree-lsp/src/watcher.rs
use notify::{RecommendedWatcher, RecursiveMode, Watcher};
use std::sync::mpsc;
use std::time::{Duration, Instant};

pub struct WorkspaceWatcher {
    _watcher: RecommendedWatcher,
    rx: mpsc::Receiver<notify::Result<notify::Event>>,
    debounce_ms: u64,
}

impl WorkspaceWatcher {
    pub fn new(root: &Path, debounce_ms: u64) -> notify::Result<Self> {
        let (tx, rx) = mpsc::channel();
        let mut watcher = notify::recommended_watcher(tx)?;
        watcher.watch(root, RecursiveMode::Recursive)?;
        Ok(Self { _watcher: watcher, rx, debounce_ms })
    }

    /// Returns Some(events) when a debounced batch is ready.
    pub fn poll(&self) -> Option<Vec<PathBuf>> {
        // Collect events for `debounce_ms` after the first one
    }
}
```

## Verification

```bash
make precheck
cargo test -p loctree-lsp watcher_smoke
# Manual: open VS Code with extension, edit a file, see loctree/scanProgress
# in the LSP trace panel.
```

## Exit contract

- COMMIT: `feat(lsp): background fs watcher + auto-rescan with progress`.
- REPORT: `.vibecrafted/reports/lsp/10-background-watcher.md`.

## Non-goals

- No global cache invalidation on watcher events — only the in-RAM snapshot
  is refreshed. CLI users still drive their own scans.
- No per-file scan; always whole-workspace incremental rescan (matches
  CLI's `Incremental` strategy).

Vibecrafted with AI Agents (c)2024-2026 The LibraxisAI Team
