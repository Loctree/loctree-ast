---
name: symbol-level-granularity
status: done
agent_target: any
project: loctree-suite
priority: 18
created: 2026-05-05
last_updated: 2026-05-09
parent_branch: feat/context-tool-alpha
depends_on: 17-live-ast-updates
---

# Plan 18 — Symbol-level granularity (per-symbol tracking via ts node ids)

## Why

Plans 16-17 give us live ASTs. This plan exploits them to track **individual
symbols**, not just files. Today `ExportSymbol` is identified by `(name,
file)` — agent can't tell whether the function on line 42 is the same one
that was on line 38 yesterday (renamed? moved? rewritten?). With
tree-sitter node ids and stable byte ranges, we can produce
`{symbol_id: stable_hash, location: (start_byte, end_byte), kind, ...}`
that survives edits.

This unlocks:

- **Per-symbol AICX overlay**: AICX intent tagged at "this exact function"
  not "somewhere in this file".
- **Per-symbol diff**: agent sees "license_activate moved from line 42 to
  87, body unchanged" vs "rewritten".
- **Per-symbol references**: tracked across edits without re-resolution.

## P0 Stage 2 v1 delta (2026-05-08)

Stage 2 introduces the `SymbolIdV1` wire contract — the minimum credible
ID shape every existing Layer 3 semantic analyzer already keys symbols
on (`<file>::<symbol>`) — and stops the v2 stable byte-range work from
being a hidden prerequisite for shipping `loctree/find` and
`loctree/aicx` to agents. What landed (commit `b5f6e308`):

- `loctree::types::SymbolIdV1`: typed newtype with
  `VERSION = "v1-string"`, `new` / `for_export` / `for_symbol_pair`
  constructors, and a documented v2 deferral pointing at the live
  tree-sitter cache from Plan 17 plus per-language extractors from
  Plan 19.
- `loctree-lsp/src/find.rs`: `FindParams.symbol_id: Option<SymbolIdV1>`
  (serde-default, backwards-compatible). `FindResponse` echoes
  `symbol_id` back and always emits `symbol_id_version` so paginated
  callers can probe before relying on body-sensitive behaviour.
- `loctree-lsp/src/backend.rs`: capability JSON declares
  `loctree/{find, aicx}.symbol_id_version` and advertises
  `loctree/symbolChanged` as `available: false` with an explicit
  reason — stable byte-range `SymbolId` (Plan 18 v2) needs per-language
  extractors over the live tree (Plan 19) to ride on top of the
  Plan 17 cache.
- 7 SymbolIdV1 unit tests in `loctree::types` plus 3 find tests for
  `symbol_id` round-trip.

What is intentionally deferred to Plan 18 v2:

- Stable byte-range hash component (Plan 16 substrate is in place but
  no extractor surface yet — Plan 19).
- `ExportSymbol` / friends gaining `pub symbol_id: SymbolIdV1` directly
  (serde-default, but does not become structural truth until extractors
  populate byte ranges).
- Per-symbol metadata map and `loctree/symbolChanged` notification
  (added/removed/moved/rewritten classification).
- Integration test for `symbolChanged: rewritten` on a renamed function.

## Acceptance criteria

- [x] New type `SymbolId` derived from `(file_path, kind, name,
      byte_range_hash)` — `SymbolIdV1` (v1 string contract,
  `<file>::<symbol>`) plus `SymbolIdV2` (v2 byte-range hash,
  `<file>::<kind>::<name>::<hash16>` via `DefaultHasher`) both
  landed in `loctree::types`.
- [x] `ExportSymbol` and friends gain `pub symbol_id: SymbolId` field
  (backwards-compatible serde via `#[serde(default,
      skip_serializing_if = is_empty)]`). LSP live extractor populates
  via `ExportSymbol::with_symbol_id(file)`; cold-scan extractors
  keep an empty default until Plan 19 wires them through.
- [x] LSP backend tracks `HashMap<Url, HashMap<SymbolIdV1,
      SymbolMetadata>>` keyed by document URI; metadata includes
  `last_seen, byte_range, ast_node_id, prev_locations, body_hash`.
  Wired through `Backend::symbol_tracker`.
- [x] On live update (Plan 17), symbols whose hash changed are emitted
  as `loctree/symbolChanged { uri, version, changes: [{ id, kind:
      "added"|"removed"|"moved"|"rewritten", from?, to? }] }`. Capability
  flips `available: true` with `kinds: ["added", "removed", "moved",
      "rewritten"]`.
- [x] `loctree/find` and `loctree/aicx` (Plans 7, 8) accept
  `symbol_id: Option<SymbolId>` for precision lookups — round-trips
  `SymbolIdV1` and surfaces `symbol_id_version` in responses.
- [x] Integration test: edit a file, rename one function, assert
  `symbolChanged: rewritten` for the renamed symbol and no events
  for siblings. `loctree-lsp/tests/symbol_granularity.rs` covers
  this case (`rename_function_emits_single_rewritten_change`) plus
  added / removed / moved / pure-shift-suppression / class-rename /
  capability-flip — 7 tests total.

## Files

- `loctree-rs/src/types.rs` — add `SymbolId` newtype + helper.
- `loctree-lsp/src/live_ast.rs` (extend) — symbol id assignment in
  extractor.
- `loctree-lsp/src/backend.rs` — symbol map + change notifications.
- `loctree-lsp/tests/symbol_granularity.rs` (NEW).

## Implementation sketch

```rust
// loctree-rs/src/types.rs
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct SymbolId(pub String);

impl SymbolId {
    pub fn from_export(file: &str, exp: &ExportSymbol) -> Self {
        let body_hash = blake3::hash(&exp.body_bytes_or_empty()).to_hex();
        Self(format!("{}::{}::{}::{}", file, exp.kind, exp.name, &body_hash[..8]))
    }
}

// loctree-lsp/src/live_ast.rs (extend extract())
fn extract_with_ids(tree: &LoctreeTree, file: &str) -> (Vec<ExportSymbol>, Vec<ImportEntry>) {
    let exports = extract_exports(tree)
        .into_iter()
        .map(|mut exp| { exp.symbol_id = SymbolId::from_export(file, &exp); exp })
        .collect();
    // ...
}
```

## Verification

```bash
make precheck
cargo test -p loctree-lsp symbol_granularity
```

## Exit contract

- COMMIT: `feat(lsp): per-symbol tracking with stable SymbolId`.
- REPORT: `.vibecrafted/reports/lsp/18-symbol-level-granularity.md`.

## Non-goals

- Cross-file symbol identity (e.g. tracking a moved function from
  `src/utils.ts` to `src/helpers.ts`) is out of scope. Each file's
  symbols form their own id space.
- No semantic equality (different bodies but same behavior) — purely
  syntactic hash.

Vibecrafted with AI Agents (c)2024-2026 The LibraxisAI Team
