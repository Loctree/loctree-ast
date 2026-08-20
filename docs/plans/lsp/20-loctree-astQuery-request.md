---
name: loctree-astQuery-request
status: done
agent_target: any
project: loctree-suite
priority: 20
created: 2026-05-05
parent_branch: feat/context-tool-alpha
depends_on: 16-tree-sitter-foundation
---

# Plan 20 — `loctree/astQuery` request (tree-sitter query DSL exposed via LSP)

## Why

Tree-sitter ships a powerful query DSL — Lisp-like patterns matching AST
shapes, used by every modern editor for syntax highlighting, code folding,
and structural search. Exposing it through LSP gives agents a
**structural search engine** way more powerful than text grep:

> "Find all `pub async fn` that contain a `tokio::spawn` call in `src/`"
>
> ```scheme
> (function_item
>   visibility: (visibility_modifier "pub")
>   modifier:   "async"
>   body: (block
>     (call_expression
>       function: (scoped_identifier
>         path:  (identifier) @ns (#eq? @ns "tokio")
>         name:  (identifier) @name (#eq? @name "spawn")))))
> @target
> ```

Agent submits the query, LSP runs it across the in-RAM tree map, returns
matches with byte ranges + paths.

## Acceptance criteria

- [x] **MVP landed 2026-05-08:** `loctree/astQuery` is registered and backed
  by snapshot-file reparsing through `loctree-ast` for JS/TS/TSX. It does
  not yet use an in-memory live document tree map.
- [x] `loctree/astQuery` custom request in `loctree-lsp/src/backend.rs`.
- [x] Params: `{ language: String, query: String,
      scope: { paths: Option<Vec<String>>, glob: Option<String> },
      limit: Option<usize>, project: Option<PathBuf> }`.
- [x] Response: `{ matches: [{file, line, column, byte_start,
      byte_end, capture_name, snippet}], total: usize, truncated: bool }`.
- [x] Snippet capped at 200 chars (caller fetches full content from path).
- [x] When `language = "auto"`, dispatch to all loaded languages.
- [x] Errors are typed: `query_compile_error { line, col, msg }`,
  `language_unsupported`, `scope_not_found`.
- [x] Capability `experimental.loctree/astQuery = { available: true,
      languages: [...] }` plus `liveDocumentCache: false`.
- [x] Curated query library shipped under
  `loctree-lsp/queries/<lang>/<name>.scm` (e.g.
  `queries/rust/tokio_spawn_in_async_pub.scm`,
  `queries/typescript/await_in_constructor.scm`). Loaded on demand
  via `loctree/astQuery { query: "@library/<name>" }`. Stage 1 ships
  only the tiny JS/TS/TSX `lexical_declarations` library.
- [x] Integration test in `loctree-lsp/tests/ast_query.rs` with at least
  three real queries against fixtures. **Landed 2026-05-09:** six
  tests in `loctree-lsp/tests/ast_query.rs` exercise real
  tree-sitter queries against on-disk JS/TS/TSX fixtures under
  `loctree-lsp/tests/fixtures/ast_query/` — TS function
  declarations, JS lexical declarations with glob scope, TSX JSX
  elements, the curated `@library/lexical_declarations` query,
  `language: "auto"` cross-grammar dispatch, and the typed
  `query_compile_error` + `language_unsupported` error round-trip.

## MVP landed - 2026-05-08

The delivered request is honest and demoable:

- Handler: `loctree-lsp/src/ast_query.rs`, registered from `loctree-lsp/src/lib.rs`.
- Capability: `experimental["loctree/astQuery"]` reports `available: true`,
  languages `javascript`, `typescript`, `tsx`, scope `snapshot_files`, and
  `liveDocumentCache: false`.
- Query library: `lexical_declarations.scm` exists for JS, TS, and TSX.
- Tests: `cargo test -p loctree-lsp ast` covers params, query matches,
  library loading, typed unsupported-language errors, glob scope, and
  capability truth.

Still deferred: Rust/Python/etc. query libraries, an integration test file with
multi-language real-query fixtures, query execution over unsaved in-memory LSP
documents, and the full live AST tree map from plans 17-19.

## Files

- `loctree-lsp/src/backend.rs` — capability + dispatch.
- `loctree-lsp/src/ast_query.rs` (NEW) — handler.
- `loctree-lsp/queries/<lang>/*.scm` (NEW) — initial curated library.
- `loctree-lsp/tests/ast_query.rs` (NEW).

## Implementation sketch

```rust
// loctree-lsp/src/ast_query.rs
use tree_sitter::{Query, QueryCursor};

pub fn handle(parsers: &TsParsers, snapshot: &Snapshot,
              params: AstQueryParams) -> AstQueryResponse {
    let lang_id = params.language.as_str();
    let query_source = if let Some(name) = params.query.strip_prefix("@library/") {
        load_library_query(lang_id, name)?
    } else {
        params.query.clone()
    };
    let lang = parsers.lookup(lang_id).ok_or(QUERY_LANG_UNSUPPORTED)?;
    let query = Query::new(lang.language(), &query_source)
        .map_err(|e| query_compile_error(e))?;

    let mut matches = Vec::new();
    for file_entry in snapshot.files_in_scope(&params.scope) {
        let tree = parsers.tree_for(&file_entry.path)?;     // from doc cache
        let mut cursor = QueryCursor::new();
        for m in cursor.matches(&query, tree.tree.root_node(), tree.source.as_slice()) {
            for cap in m.captures {
                matches.push(materialize_match(&file_entry, &query, cap));
                if matches.len() >= params.limit.unwrap_or(100) {
                    return AstQueryResponse::truncated(matches);
                }
            }
        }
    }
    AstQueryResponse::full(matches)
}
```

## Verification

```bash
make precheck
cargo test -p loctree-lsp ast_query
# Manual smoke (rust query against this very repo):
echo '{"jsonrpc":"2.0","id":1,"method":"loctree/astQuery",
       "params":{"language":"rust",
                 "query":"@library/rust/tokio_spawn_in_async_pub"}}' \
  | loctree-lsp
```

## Exit contract

- COMMIT: `feat(lsp): expose loctree/astQuery with curated query library`.
- REPORT: `.vibecrafted/reports/lsp/20-loctree-astQuery-request.md` with
  query library inventory.

## Non-goals

- No query DSL extensions beyond stock tree-sitter. If a query is hard
  to express, document it; don't extend the DSL.
- No write-side queries (refactor by AST rewrite) — read-only matches.

Vibecrafted with AI Agents (c)2024-2026 The LibraxisAI Team
