---
name: cross-language-unified-surface
status: review
agent_target: any
project: loctree-suite
priority: 19
created: 2026-05-05
parent_branch: feat/context-tool-alpha
depends_on: 16-tree-sitter-foundation
stage_1_landed: 2026-05-09
---

# Plan 19 — Cross-language unified surface (TsParser → loctree analyzer)

## Why

After Plan 16, we have a tree-sitter registry. After Plan 17/18, LSP can
update per-document. But **the analyzer (full scan path) still uses N
heterogeneous parsers**: OXC for JS/TS, regex for Python, custom code for
Rust, and so on. This plan migrates the cold-scan path to tree-sitter so
loctree has **one** parser surface.

Why bother? Three reasons:
1. **Symmetry**: live AST (LSP) and cold AST (CLI scan) use same trees,
   same SymbolIds, same edge extraction logic.
2. **Maintenance**: per-language code shrinks dramatically — instead of
   custom Python `__all__` regex, we walk a single ts query.
3. **New languages**: adding Astro / Svelte / Vue / Solid needs only a
   grammar package + extractor module, not a fresh parser.

This plan is **larger** than 16-18 and will likely span multiple PRs.
Acceptance below is **per-language** so it can land incrementally.

## Acceptance criteria (per language; complete = all green)

- [x] **Trait** `LangExtractor` extending `LangParser`:
      ```rust
      pub trait LangExtractor: LangParser {
          fn extract_exports(&self, tree: &LoctreeTree) -> Vec<ExportSymbol>;
          fn extract_imports(&self, tree: &LoctreeTree) -> Vec<ImportEntry>;
          fn extract_calls(&self, tree: &LoctreeTree) -> Vec<CallEntry>;
          // Optional: framework-specific data (Tauri commands, FastAPI routes, ...)
      }
      ```
      Stage 1 lands the trait in `loctree-ast/src/extractors/mod.rs` plus
      mirroring shapes (`ExportSymbol`, `ImportEntry`, `ImportBinding`,
      `CallEntry`) so the cold-scan dispatcher in `loctree-rs` can adapt
      without `loctree-ast` taking a reverse dependency.
- [x] **TS/JS** extractor matches OXC output on the existing TS fixture
      tree (`loctree-rs/tests/fixtures/simple_ts/`). 100% parity on
      hand-counted exports + imports (5/5 + 2/2). Calls captured as
      best-effort with documented gaps in
      `.vibecrafted/reports/lsp/19-cross-lang-stage-1.md`.
- [ ] **Python** extractor — **Stage 2 deferred (queued)**.
- [ ] **Rust** extractor — **Stage 2 deferred (queued)**.
- [ ] **Go** extractor — **Stage 2 deferred (queued)**.
- [ ] **CSS** extractor — **Stage 2 deferred (queued)**.
- [ ] **Dart** extractor — **Stage 2 deferred (queued)**.
- [ ] **HTML/Vue/Svelte/Astro** SFC base — **Stage 2 deferred (queued)**.
- [x] Feature flag `analyzer.parser = "ts"|"oxc"` (default `"oxc"` for v1)
      in `.loctree/config.toml`. Stage 1 ships workspace-wide on/off via
      `LoctreeConfig::parser_strategy()` plus `LOCTREE_PARSER` env
      override. Per-language opt-in stays Stage 2.
- [x] Integration test for TS under
      `loctree-rs/tests/ts_extractors_ts.rs` (4 tests against
      `simple_ts` fixture: function exports, multi-export-per-file,
      named/namespace/default imports, call-site recording).

## Files (per language; partial OK)

- `loctree-ast/src/extractors/{ts,js,py,rs,go,css,dart,vue,svelte,astro,html}.rs`
  (or under `loctree-rs/src/ast/extractors/` if no separate crate).
- `loctree-rs/src/analyzer/scan.rs` — feature-gated dispatch:
  `if config.parser_strategy() == "ts" { ts_dispatch(file) } else { oxc_dispatch(file) }`.
- `loctree-rs/tests/ts_extractors_<lang>.rs` (NEW per language).

## Implementation sketch

```rust
// loctree-ast/src/extractors/ts.rs
use tree_sitter::Query;

pub struct TsExtractor;

impl LangExtractor for TsExtractor {
    fn extract_exports(&self, tree: &LoctreeTree) -> Vec<ExportSymbol> {
        static QUERY: Lazy<Query> = Lazy::new(|| Query::new(
            tree_sitter_typescript::language_typescript(),
            r#"
            (export_statement
              declaration: [
                (function_declaration name: (identifier) @name)
                (class_declaration   name: (type_identifier) @name)
                (lexical_declaration (variable_declarator name: (identifier) @name))
              ]) @export
            "#,
        ).unwrap());

        let mut cursor = QueryCursor::new();
        let mut out = Vec::new();
        for m in cursor.matches(&QUERY, tree.tree.root_node(), tree.source.as_slice()) {
            // Build ExportSymbol from captured nodes, preserving line numbers.
        }
        out
    }
}
```

## Verification

```bash
make precheck
cargo test -p loctree --test ts_extractors_ts        # one per language
# Compare ts vs oxc outputs:
LOCTREE_PARSER=ts loct dead --json > /tmp/ts.json
LOCTREE_PARSER=oxc loct dead --json > /tmp/oxc.json
diff /tmp/ts.json /tmp/oxc.json | head
```

## Exit contract

- COMMIT (per language): `feat(ast): tree-sitter <lang> extractor`.
- REPORT: `.vibecrafted/reports/lsp/19-cross-language-unified-surface.md`
  with per-language parity table (oxc-equivalent / partial / TODO).

## Non-goals

- This plan does NOT remove OXC. It adds the ts path behind a feature
  flag. OXC removal happens in a separate clean-up plan once parity is
  reached for all languages of interest.
- Framework-specific extraction (Tauri command bridges, FastAPI routes)
  remains in its current location; only the **parsing** layer changes.

## Stage 1 landed (2026-05-09)

**Status delta**: `queued` → `review`. Multi-stage; full closure (status
`done`) waits for Stage 2 (other languages) and the eventual Stage 3
OXC removal cleanup.

**What landed**

- `LangExtractor` trait + Plan 19 v1 contract types (`ExportSymbol`,
  `ImportEntry`, `ImportBinding`, `CallEntry`) in
  `loctree-ast/src/extractors/mod.rs`.
- TS/JS/TSX implementations under
  `loctree-ast/src/extractors/{ts.rs,js.rs}` driven by
  `tree_sitter_typescript` + `tree_sitter_javascript` grammars with
  `OnceLock`-cached `Query` compilation.
- `loctree-rs` adapter `analyzer::scan::ts_dispatch_js` (feature-gated
  on `LOCTREE_PARSER=ts` / `analyzer.parser = "ts"`) producing
  `FileAnalysis` with `imports`, `exports`, `symbol_usages` for the JS/TS
  cold-scan path. OXC remains the default and primary engine.
- `LoctreeConfig::parser_strategy()` resolves env override → config
  field → `oxc` default; unknown values normalize to `oxc`.
- Re-export of `loctree_ast::CallEntry` in `loctree::types` so
  downstream analyzers consume one shape.
- New tests: 2 in `loctree-ast/tests/extractors.rs`, 4 in
  `loctree-rs/tests/ts_extractors_ts.rs`, 1 ignored hand-counted parity
  harness in `loctree-rs/tests/ts_extractors_parity.rs`.
- Plan 18's `loctree-lsp/src/live_ast.rs` extractor stub is **untouched
  in this pass** — Stage 1 holds the contract surface clean. Wiring
  `extract_live_symbols` to `TsExtractor::extract_exports` is a
  Stage 1.5 follow-up cut.

**What stays queued for Stage 2**

- Python extractor (replaces regex-based path).
- Rust extractor with `#[tauri::command]` recognition.
- Go / CSS / Dart extractors.
- SFC base (HTML/Vue/Svelte/Astro) dispatching to nested script/style
  trees.
- Per-language opt-in (current knob is workspace-wide).
- Dynamic `import()` capture in `extract_imports`.
- Call-resolution parity vs OXC (member-callee, optional chains, etc.).

Vibecrafted with AI Agents (c)2024-2026 The LibraxisAI Team
