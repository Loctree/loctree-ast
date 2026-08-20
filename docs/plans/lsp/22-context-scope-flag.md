---
name: lsp-22-loct-context-scope-flag
description: Add deterministic --scope flag to loct context alongside semantic --task.
type: implementation_plan
project: Loctree/loctree-suite
plan_number: 22
date: 2026-05-08
status: done
---

# Plan 22 — `loct context --scope <SELECTOR>`

Implemented in the CLI/MCP ContextPack path as a deterministic structural
filter:

- repeatable `--scope` parser support
- `path:`, `tag:`, `import:`, and `reach:` selectors
- named scopes from `.loctree/scopes.toml`
- top-level JSON `scope` metadata
- top-level JSON `task` metadata when `--task` is present
- `risk.cache_scope = Scoped(<fingerprint>)`
- markdown TL;DR `**Scope.**` and `**Task.**` lines before "Top 3 things"
- `--file` wins over `--scope` with a warning

`--task` remains backward-compatible as a set-cutter when no scope is present.
With scope present, task is ranker-only inside the scoped file set.

Verification performed during implementation:

```bash
cargo check -p loctree --lib
cargo check -p loctree-mcp
cargo run -q -p loctree --bin loct -- context --scope 'path:loctree-rs/src/cli/' --no-aicx --full
cargo run -q -p loctree --bin loct -- context --scope 'context-pipeline' --task 'cache invalidation' --no-aicx
cargo run -q -p loctree --bin loct -- context --file Cargo.toml --scope 'context-pipeline' --no-aicx --full
cargo run -q -p loctree --bin loct -- context --scope 'context-pipline' --no-aicx --full
```

Finalized 2026-05-10 with regression coverage for repeatable CLI
`--scope` parsing plus scoped ContextPack metadata/task/cache semantics.
Report: `reports/lsp/22-context-scope-flag.md`.
