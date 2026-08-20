---
name: loctree-slice
description: Surface a single file's dependencies, consumers, exports, and structural neighborhood before any modification. Use BEFORE every Edit, Write, or refactor on a source file. Triggers on phrases "slice this file", "what depends on", "before I edit", "show file dependencies", "/loctree:slice", "what consumes this", "dependency graph for this file".
argument-hint: "<file-path>"
allowed-tools:
  - mcp__loctree-mcp__slice
---

# /loctree:slice — file structural neighborhood

Call `mcp__loctree-mcp__slice` with the file path from `$ARGUMENTS`.

## Why this matters before edits

Before mutating any source file you must know:

- **Direct dependencies** — what this file imports
- **Direct consumers** — what imports this file
- **Exported symbols** — what this file makes available
- **Internal edges** — coupling within the same module

The slice is the fastest way to establish this neighborhood. Confirm exact source
and any unsupported/dynamic wiring separately when the change is risky.

## Argument handling

Path may be relative to the repo root or absolute. If user passes a symbol or directory, route to the wrong tool:

- Symbol → `/loctree:find`
- Directory → `/loctree:focus`
- Repo-level → `/loctree:context` or `/loctree:repo-view`

## Reporting

After the slice arrives, surface to the user:

1. **Hub status** — if `consumers.count >= 10`, lead with **⚠️ HIGH-IMPACT FILE: N consumers** before any other detail.
2. **Direct consumers** — top 5 by name. If more, summarize "+N more files".
3. **Direct dependencies** — top 5 imports.
4. **Exported symbols** — names + kind (function/struct/const/etc.).
5. **Internal edges** within the same directory if the slice surfaces them.

If the file has zero represented consumers, flag it for investigation. Check
entrypoints, manifests, generated/dynamic wiring, language coverage, and tests
before calling it dead.

## Authority

Slice output is `loctree_derived` analyzer evidence from the snapshot. Trust it
for orientation; verify decisive claims with source, manifests, tests, and runtime.

## When the file is JS/TS/TSX

Live AST is available via `loctree-lsp`. The snapshot slice is still authoritative for cross-file edges, but for within-file structure the LSP's `loctree/astQuery` may have fresher unsaved-buffer awareness if the file is currently open in an editor session.

## Anti-patterns

- Calling slice on a directory — that's `/loctree:focus`.
- Calling slice on a symbol name — that's `/loctree:find`.
- Skipping slice because "I know this file" — your training-data memory of the file is months stale; the snapshot is current.
