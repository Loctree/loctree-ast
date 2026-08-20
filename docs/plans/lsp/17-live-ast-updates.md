---
name: live-ast-updates
status: done
agent_target: any
project: loctree-suite
priority: 17
created: 2026-05-05
last_updated: 2026-05-09
parent_branch: feat/context-tool-alpha
depends_on: 16-tree-sitter-foundation
---

# Plan 17 — Live AST updates from LSP `textDocument/didChange`

## Why

After Plan 16, we have incremental parsing primitives. This plan wires them
to LSP edit events: every `didChange` event passes through tree-sitter's
incremental update, producing a fresh AST in microseconds. The agent gets
a continually live view without waiting for a workspace rescan (Plan 10's
watcher handles workspace-wide scope; this plan handles single-file
edit-time precision).

## P0 Stage 2 MVP delta (2026-05-08)

Stage 2 landed the substrate-shaped half of this plan: `LiveAstStore` +
`LiveDocument`, lifecycle hooks, `loctree/documentChanged` notification,
`compute_with_live` overlay in `ast_query`, and capability flips. INCREMENTAL
sync + per-event `InputEdit` translation + the 100-edit benchmark were left
as the v2 cut; Plan 19 still owns per-language exports/imports extractors.

## P1 Stage 2 v2 delta (2026-05-09)

INCREMENTAL textDocument sync is now wired end-to-end:

- `LiveDocument` carries `content: String` so post-edit slices can be
  recomputed without re-reading disk; the field is the only correctness
  prerequisite for `Position(line, character)` translation.
- `loctree_lsp::live_ast::translate_change_event` /
  `translate_change_events` convert each `TextDocumentContentChangeEvent`
  into a tree-sitter `InputEdit { start_byte, old_end_byte, new_end_byte,
  start_position, old_end_position, new_end_position }` against the
  previous content. Range-less events fall through to a full reparse via
  `LiveAstStore::update`, matching the LSP spec for full-document
  replacement.
- `LiveAstStore::apply_change(uri, version, &events)` is the new entry
  point: composes edits, calls `Parsers::parse_incremental(prev_tree,
  new_content, &edits)`, and emits the same `DocumentChanged` payload
  shape as `update`.
- `Backend::did_change` now calls `apply_change` and keeps the legacy
  `documents` cache aligned with the post-edit content so non-AST
  handlers (goto_def, references) stay coherent.
- `server_capabilities()` advertises
  `text_document_sync.change = TextDocumentSyncKind::INCREMENTAL`; the
  capability JSON flips to `sync_mode: "incremental"`,
  `incremental_edits: true`, `position_encoding: "utf-16"` (LSP default).
- New integration test file `loctree-lsp/tests/live_ast.rs` covers
  function rename, multi-event composition, range deletion, range-less
  fall-through, the 100-edit benchmark (<100ms gate), and live-cache
  observability via `get_for_path`.

Remaining deferrals (Plan 19 scope, not 17):

- Per-language exports/imports extractors driving symbol-level diff fields
  (`exports_added`, `imports_removed`, …) on the notification payload.
- Workspace-wide consistency for cross-file edges still relies on Plan 10's
  watcher; Plan 17 only refreshes the per-document slice.

## Acceptance criteria

- [x] Each open document in the LSP backend holds a `LoctreeTree` (from
  Plan 16) keyed by URI.
- [x] On `did_open`, full parse populates the tree.
- [x] On `did_change` with `TextDocumentSyncKind::INCREMENTAL`, each
  `TextDocumentContentChangeEvent` is converted to a
  `tree_sitter::InputEdit` and fed to `parse_incremental`. Range-less
  events fall back to the full-parse path via `LiveAstStore::update`.
- [x] Per-document state: `{ tree: LoctreeTree, version, content,
      parse_duration_ms }` — extractor-driven `exports`/`imports` slices
  remain Plan 19's contract; the v2 contract here is the byte-stable
  content + tree pair.
- [ ] Re-extracted exports/imports from the new tree replace the
  per-document slice in the in-RAM snapshot **without rescanning the
  whole workspace**. **Deferred to Plan 19.**
- [x] When this happens, server emits notification
  `loctree/documentChanged { uri, lang, version, has_error,
  root_kind, parse_duration_ms }` (extractor-driven diff fields land
  additively after Plan 19).
- [x] Capability `experimental.loctree/documentChanged = { available: true,
      languages, sync_mode: "incremental", incremental_edits: true,
      position_encoding: "utf-16", extractors: false }`.
- [x] Integration test: TS function rename via incremental sync emits a
  payload with `parse_duration_ms < 5ms` and `root_kind = "program"`.
  Implemented by `function_rename_emits_incremental_payload` in
  `loctree-lsp/tests/live_ast.rs`. The full mock-client notification
  round-trip stays out of scope — `Backend` is private and the LSP
  handler delegates straight into `LiveAstStore::apply_change`, which
  is what the test exercises.
- [x] Benchmark in test: 100 small edits complete <100ms total.
  `hundred_edits_complete_under_hundred_ms` measures total wall
  time + per-edit `parse_duration_ms` and prints a histogram
  (p50/p99/max/mean) for the report. Local run: total≈3.7ms,
  p50=0.017ms, p99=0.235ms.

## Files

- `loctree-lsp/src/backend.rs` — open/change/close hooks.
- `loctree-lsp/src/live_ast.rs` (NEW) — per-document state + edit handler.
- `loctree-lsp/src/extractors/` (NEW dir) — per-language exports/imports
  extractor over the tree-sitter AST. Initially TS/JS only; others stub
  out and fall back to OXC-derived data.
- `loctree-lsp/tests/live_ast.rs` (NEW).

## Implementation sketch

```rust
// loctree-lsp/src/live_ast.rs
use tree_sitter::InputEdit;

pub struct LiveDocument {
    pub tree: LoctreeTree,
    pub exports: Vec<ExportSymbol>,
    pub imports: Vec<ImportEntry>,
    pub last_parsed_at: Instant,
}

impl LiveDocument {
    pub fn open(parsers: &TsParsers, uri: &Url, content: &str) -> Option<Self> {
        let lang = parsers.for_path(uri.to_file_path().ok()?)?;
        let tree = parsers.parse(lang, content.as_bytes())?;
        let (exports, imports) = extract(&tree);
        Some(Self { tree, exports, imports, last_parsed_at: Instant::now() })
    }

    pub fn apply_change(&mut self, parsers: &TsParsers, edits: &[InputEdit],
                        new_content: &str) -> ChangeReport {
        let prev_exports = self.exports.clone();
        let prev_imports = self.imports.clone();
        self.tree = parsers
            .parse_incremental(&self.tree, new_content.as_bytes(), edits)
            .expect("incremental parse");
        let (exports, imports) = extract(&self.tree);
        let report = ChangeReport::diff(&prev_exports, &exports, &prev_imports, &imports);
        self.exports = exports;
        self.imports = imports;
        self.last_parsed_at = Instant::now();
        report
    }
}
```

## Verification

```bash
make precheck
cargo test -p loctree-lsp live_ast
```

## Exit contract

- COMMIT: `feat(lsp): live AST updates via tree-sitter incremental parse`.
- REPORT: `.vibecrafted/reports/lsp/17-live-ast-updates.md` with
  parse_duration_ms histogram from the test bench.

## Non-goals

- Workspace-wide consistency is **not** maintained by this plan — the
  in-RAM snapshot's other-file edges may be stale until Plan 10's watcher
  triggers full incremental rescan.
- Per-language extractors are scaffolded but only TS/JS is fully wired in
  v1; other languages emit empty diffs (graceful fallback).

Vibecrafted with AI Agents (c)2024-2026 The LibraxisAI Team
