---
name: loctree-diff-request
status: queued
agent_target: any
project: loctree-suite
priority: 11
created: 2026-05-05
parent_branch: feat/context-tool-alpha
depends_on: 10-background-watcher
---

# Plan 11 — `loctree/diff` request (delta since last snapshot)

## Why

After Plan 10 lands, the daemon holds two snapshots in memory: the previous
one and the freshly-rescanned one. Agents asking "what changed since I
last queried?" need a delta. CLI has `loct diff <since>` for snapshot-vs-
snapshot comparison; LSP should expose the equivalent for **session-local**
deltas — files added, removed, changed, with import-edge changes.

## Acceptance criteria

- [ ] `loctree/diff` custom request in `loctree-lsp/src/backend.rs`.
- [ ] Params: `{ since: "lastQuery"|"lastScan"|"epoch"|<git-rev>,
      project: Option<PathBuf> }`.
- [ ] Response: `{ files_added: [...], files_removed: [...],
      files_changed: [...], edges_added: [{from, to, label}],
      edges_removed: [...], symbols_added: [...], symbols_removed: [...],
      since_marker: String }`.
- [ ] Server tracks per-session "lastQuery" marker (timestamp / snapshot id)
  and refreshes it on each `loctree/diff` call (caller can pin via
  explicit param).
- [ ] When `since` is a git rev, delegates to the existing snapshot-diff
  flow (`loctree::diff` module).
- [ ] Capability `experimental.loctree/diff = { available: true }`.
- [ ] Integration test in `loctree-lsp/tests/diff_request.rs`.

## Files

- `loctree-lsp/src/backend.rs` — capability + dispatch + per-session
  marker storage.
- `loctree-lsp/src/diff.rs` (NEW) — handler.
- `loctree-lsp/tests/diff_request.rs` (NEW).

## Implementation sketch

```rust
// loctree-lsp/src/diff.rs
pub fn handle(prev: Option<&Snapshot>, current: &Snapshot, since: &str) -> DiffResponse {
    let prev = match (since, prev) {
        ("lastQuery", Some(p)) | ("lastScan", Some(p)) => p,
        ("epoch", _) | (_, None) => return DiffResponse::full_from(current),
        (rev, _) => return load_snapshot_at_rev(rev),
    };
    DiffResponse {
        files_added: diff_files(prev, current, FileDiffKind::Added),
        files_removed: diff_files(prev, current, FileDiffKind::Removed),
        files_changed: diff_files(prev, current, FileDiffKind::Changed),
        edges_added: diff_edges(prev, current, EdgeDiffKind::Added),
        edges_removed: diff_edges(prev, current, EdgeDiffKind::Removed),
        symbols_added: diff_symbols(prev, current, SymbolDiffKind::Added),
        symbols_removed: diff_symbols(prev, current, SymbolDiffKind::Removed),
        since_marker: format!("snapshot:{}", current.id()),
    }
}
```

## Verification

```bash
make precheck
cargo test -p loctree-lsp diff_request
```

## Exit contract

- COMMIT: `feat(lsp): expose loctree/diff for session-local deltas`.
- REPORT: `.vibecrafted/reports/lsp/11-loctree-diff-request.md`.

## Non-goals

- No persistent delta history beyond the current daemon process — restart
  resets to "epoch" baseline.
- No semantic-level diff (idiom tag changes etc.) — pure structural diff.

Vibecrafted with AI Agents (c)2024-2026 The LibraxisAI Team
