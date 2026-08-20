---
name: streaming-cursor-pattern
status: queued
agent_target: any
project: loctree-suite
priority: 12
created: 2026-05-05
parent_branch: feat/context-tool-alpha
note: infrastructure plan — adopted by 5,7,15 for large responses
---

# Plan 12 — Streaming + cursor pattern (Codex Manifest Protocol in LSP)

## Why

Some loctree responses can be large: a slice on a hub file (50+ deps + 100+
consumers), a workspace-wide find with 200 hits, full semantic facts for
a 3000-file workspace. JSON-RPC responses get truncated by the host (the
operator already saw this with MCP atlas — payload too long, host dumps to
temp, agent never reads temp). Solution: paginate via `cursor` token, the
same way `loctree-mcp::context` already does (Codex Manifest Protocol —
see commits `c5d474e3` and `7fb5077f`).

## Acceptance criteria

- [ ] Add a generic `Paginated<T>` wrapper in
  `loctree-lsp/src/protocol.rs` (NEW) with shape:
  `{ chunk: u32, total_chunks: u32, next_cursor: Option<String>,
  data: T, advisory: Option<String> }`.
- [ ] Each large-response request (Plans 5, 7, 11, 14, 15) accepts an
  optional `cursor: Option<String>` param and an optional
  `chunk_size: Option<usize>` param.
- [ ] Default chunk size: 50 items (configurable via init option
  `loctree.protocol.defaultChunkSize`).
- [ ] When response is small enough, single response with `next_cursor: null`.
- [ ] When chunked, each subsequent request with the cursor returns the
  next chunk; final chunk has `next_cursor: null`.
- [ ] Cursor format: opaque base64 string encoding `{snapshot_id, offset, kind}`
  (server validates snapshot_id matches current; otherwise returns
  `error: snapshot_drifted, retry: true`).
- [ ] Integration test in `loctree-lsp/tests/cursor_smoke.rs` covering:
  first chunk, mid chunk, final chunk, snapshot drift mid-pagination.

## Files

- `loctree-lsp/src/protocol.rs` (NEW) — `Paginated<T>` + cursor encode/decode.
- `loctree-lsp/src/cursor.rs` (NEW) — cursor token impl.
- Plans 5, 7, 11, 14, 15 handlers — wrap responses in `Paginated<T>`.
- `loctree-lsp/tests/cursor_smoke.rs` (NEW).

## Implementation sketch

```rust
// loctree-lsp/src/protocol.rs
#[derive(Debug, Serialize)]
pub struct Paginated<T> {
    pub chunk: u32,
    pub total_chunks: u32,
    pub next_cursor: Option<String>,
    pub data: T,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub advisory: Option<String>,
}

pub struct CursorState {
    pub snapshot_id: String,
    pub offset: usize,
    pub kind: String,         // request method, for type-checking
}
```

## Verification

```bash
make precheck
cargo test -p loctree-lsp cursor_smoke
```

## Exit contract

- COMMIT: `feat(lsp): paginated responses with cursor pattern`.
- REPORT: `.vibecrafted/reports/lsp/12-streaming-cursor-pattern.md`.

## Non-goals

- No true streaming via JSON-RPC notifications (LSP doesn't support it
  for client→server requests). Pagination via cursor is the analog.
- Cursor is process-local — restart invalidates outstanding cursors
  (clients see `snapshot_drifted` and retry).

Vibecrafted with AI Agents (c)2024-2026 The LibraxisAI Team
