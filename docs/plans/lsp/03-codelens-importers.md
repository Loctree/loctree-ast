---
name: codelens-importers
status: queued
agent_target: any
project: loctree-suite
priority: 3
created: 2026-05-05
parent_branch: feat/context-tool-alpha
note: nice-to-have under AI-engine paradigm; passive structural data
---

# Plan 3 — `textDocument/codeLens` for exports (importers count)

## Why

Under the AI-engine paradigm, CodeLens is not strictly necessary (agents
can query importers via `textDocument/references` or a future
`loctree/find` request — see Plan 7). However it is **passive structural
data** that requires zero agent interaction:

- Inline annotations next to every `pub fn` / `export function` / etc.
- Says "N importers" with an optional click-through (humans benefit; agents
  parsing IDE state still see the annotation as structured signal).
- Establishes Loctree as a first-class IDE citizen (low cost, high
  visibility for human developers running agents in IDE mode).

This plan is **lowest priority of 1-4** and may be deferred to a follow-up
PR if effort runs short.

## Acceptance criteria

- [ ] `loctree-lsp/src/backend.rs` advertises
  `code_lens_provider: Some(CodeLensOptions { resolve_provider: false })`.
- [ ] New module `loctree-lsp/src/code_lens.rs` implements
  `code_lens(file_path) -> Vec<CodeLens>`.
- [ ] Each top-level export in the file gets a CodeLens with title
  `"N importers"` (or `"unused"` when zero), at the export's line.
- [ ] No CodeLens command (no resolve) for v1 — purely informational.
  Future iteration can add `command: "loctree.showImporters"`.
- [ ] Unit tests in `code_lens.rs` for the count formatting and dead-export
  labeling.
- [ ] Manual smoke: open a file in VS Code with the loctree-lsp client
  enabled; verify counts appear.

## Files to modify

- `loctree-lsp/src/backend.rs:185-210` — capability + dispatch for
  `code_lens` requests.
- `loctree-lsp/src/code_lens.rs` (NEW) — provider implementation.
- `loctree-lsp/src/lib.rs` — `pub mod code_lens;`.
- `editors/vscode/src/extension.ts` — verify the LSP client requests
  CodeLens (most do by default; check `clientCapabilities`).

## Implementation sketch

```rust
// loctree-lsp/src/code_lens.rs
use tower_lsp::lsp_types::{CodeLens, Range, Position};
use crate::snapshot::SnapshotHandle;

pub fn code_lens_for_file(snap: &SnapshotHandle, file_path: &str) -> Vec<CodeLens> {
    let analysis = match snap.find_analysis(file_path) {
        Some(a) => a,
        None => return vec![],
    };
    let importer_counts = snap.importer_counts_per_export(file_path);

    analysis.exports.iter().filter_map(|exp| {
        let line = exp.line? as u32;
        let count = importer_counts.get(&exp.name).copied().unwrap_or(0);
        let title = if count == 0 {
            format!("unused (0 importers)")
        } else {
            format!("{count} importer{}", if count == 1 { "" } else { "s" })
        };
        Some(CodeLens {
            range: Range {
                start: Position { line: line.saturating_sub(1), character: 0 },
                end:   Position { line: line.saturating_sub(1), character: 0 },
            },
            command: None,        // v1: passive annotation only
            data: None,
        })
    }).collect()
}
```

## Verification

```bash
make precheck
cargo test -p loctree-lsp code_lens
# Manual: open loctree-rs/src/snapshot.rs in VS Code with extension active,
# verify "N importers" annotations appear on pub fn lines.
```

## Exit contract

- COMMIT: `feat(lsp): codeLens with importer counts for exports`.
- REPORT: `.vibecrafted/reports/lsp/03-codelens-importers.md` with screenshot
  if the agent has UI access.

## Non-goals

- No click handlers / resolve flow in v1 — that's plan 4 territory.
- No diagnostic-style severity coloring — neutral annotation only.

Vibecrafted with AI Agents (c)2024-2026 The LibraxisAI Team
