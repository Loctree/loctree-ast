---
name: tree-sitter-foundation
status: stage1-completed
agent_target: any
project: loctree-suite
priority: 16
created: 2026-05-05
parent_branch: feat/context-tool-alpha
note: foundation for plans 17-20 — live update infrastructure
---

# Plan 16 — Tree-sitter foundation (incremental parser layer)

## Why

Today loctree analyzes files via heterogeneous parsers:

- JS/TS/JSX/TSX/Vue/Svelte → OXC (full reparse on every scan)
- Python → ad-hoc regex / `__all__` heuristics
- Rust → ad-hoc syntax + Tauri macro recognition
- CSS / Dart / Go → custom per-language code

This blocks live update: every `textDocument/didChange` from LSP would
need full reparse. Tree-sitter provides incremental, byte-range-edit-aware
parsing across all languages we care about, with a single integration
surface.

This plan is the **foundation** for live AST updates (Plan 17), symbol-
level granularity (Plan 18), unified cross-language API (Plan 19), and
AST query DSL (Plan 20).

## Acceptance criteria

- [x] **Stage 1 landed 2026-05-08:** new `loctree-ast` workspace crate,
  `LangParser`, `Parsers`, `LoctreeTree`, TS/JS/TSX parser registry,
  direct parse and incremental parse APIs, plus `cargo test -p
      loctree-ast` smoke coverage.
- [ ] New crate `loctree-ast` (or module under `loctree-rs/src/ast/`)
  that wraps `tree-sitter = "0.20"` plus per-language grammars:
  `tree-sitter-typescript`, `-javascript`, `-python`, `-rust`,
  `-go`, `-css`, `-html`, `-vue`, `-svelte`, `-astro`.
- [ ] Common trait:
  ```rust
  pub trait LangParser {
      fn language() -> tree_sitter::Language;
      fn extensions() -> &'static [&'static str];
  }
  ```
- [ ] Registry: `Parsers::for_path(&Path) -> Option<Box<dyn LangParser>>`
  keyed by extension.
- [ ] Wrapper type `LoctreeTree { tree: tree_sitter::Tree, source: Vec<u8>,
      lang: &'static str }` with `parse(source) -> LoctreeTree` and
  `parse_incremental(prev: &LoctreeTree, edits: &[InputEdit])
  -> LoctreeTree`.
- [ ] Existing analyzer code stays as-is for v1 (no migration in this
  plan); the ast layer runs **alongside** OXC and is consumed only by
  Plan 17's live-update path. Migration of full scan to ts is a
  separate, larger plan.
- [ ] Snapshot benchmark: `cargo bench` (or criterion) demonstrating
  tree-sitter parse vs OXC parse on a representative TS file. ts
  should be within 2× OXC for cold parse and >10× faster on
  incremental edit.
- [ ] Integration test in `loctree-rs/tests/ast_parsers.rs`: parse one
  file per supported language, assert non-empty tree.

## Stage 1 landed - 2026-05-08

This branch intentionally landed the smallest credible substrate, not the full
multi-language plan:

- `loctree-ast/` is a workspace crate using `tree-sitter = "0.25.10"`,
  `tree-sitter-javascript`, and `tree-sitter-typescript`.
- Supported language ids are `javascript`, `typescript`, and `tsx`.
- `Parsers::for_path`, `Parsers::lookup`, `Parsers::parse_path`,
  `Parsers::parse_language`, and `Parsers::parse_incremental` are available.
- `LoctreeTree` carries `tree_sitter::Tree`, source bytes, and language id.
- Existing OXC/custom analyzer paths remain unchanged.

Still deferred: Python/Rust/Go/CSS/HTML/Vue/Svelte/Astro grammars, extractor
parity, benchmark evidence, and live LSP document-cache integration.

## Files

- `loctree-ast/` (NEW crate, workspace member) OR
  `loctree-rs/src/ast/` (NEW module). Choose based on workspace
  preference — the latter avoids a new crate but couples versions.
- `loctree-rs/Cargo.toml` — add tree-sitter + grammar deps.
- `loctree-rs/tests/ast_parsers.rs` (NEW) — multi-language parse smoke.
- `Cargo.toml` (workspace root) — register new crate if chosen.

## Implementation sketch

```rust
// loctree-ast/src/lib.rs (or loctree-rs/src/ast/mod.rs)
use tree_sitter::{InputEdit, Language, Parser, Tree};

pub struct LoctreeTree {
    pub tree: Tree,
    pub source: Vec<u8>,
    pub lang: &'static str,
}

pub trait LangParser: Send + Sync {
    fn language(&self) -> Language;
    fn lang_id(&self) -> &'static str;
    fn extensions(&self) -> &'static [&'static str];
}

pub struct TsParsers {
    parsers: Vec<Box<dyn LangParser>>,
}

impl TsParsers {
    pub fn new_default() -> Self { /* register all built-in */ }
    pub fn for_path(&self, path: &Path) -> Option<&dyn LangParser> { /* ext lookup */ }
    pub fn parse(&self, lang: &dyn LangParser, source: &[u8]) -> Option<LoctreeTree>;
    pub fn parse_incremental(
        &self,
        prev: &LoctreeTree,
        new_source: &[u8],
        edits: &[InputEdit],
    ) -> Option<LoctreeTree>;
}
```

## Verification

```bash
make precheck
cargo test -p loctree --test ast_parsers     # if module under loctree-rs
# or
cargo test -p loctree-ast                    # if separate crate
cargo bench -p loctree ts_parse_bench        # if benches added
```

## Exit contract

- COMMIT: `feat(ast): tree-sitter foundation with multi-language registry`.
- REPORT: `.vibecrafted/reports/lsp/16-tree-sitter-foundation.md` with
  benchmark numbers (cold parse vs incremental).

## Non-goals

- Do NOT migrate analyzer's full scan to tree-sitter in this plan. OXC
  stays for the cold scan path; ts is consumed only by live-update plans.
- Do NOT add language grammars beyond the listed set — Astro/Svelte/Vue
  may need framework-specific extraction logic in later plans.

Vibecrafted with AI Agents (c)2024-2026 The LibraxisAI Team
