---
name: loctree-focus
description: Deep-dive on a directory or module — files, internal edges, external dependencies, LOC. Use when entering an unfamiliar module, mapping a sub-system before refactor, or evaluating module cohesion. Triggers on phrases "focus on this module", "map this directory", "show me the auth module", "/loctree:focus", "what's in src/", "module deep-dive", "internal coupling".
argument-hint: "<directory-path>"
allowed-tools:
  - mcp__loctree-mcp__focus
---

# /loctree:focus — module deep-dive

Call `mcp__loctree-mcp__focus` with the directory path from `$ARGUMENTS`.

## What you get

- **File list** with LOC per file, language, role
- **Internal edges** — imports between files inside the directory (cohesion signal)
- **External dependencies** — which other modules this directory pulls from
- **External consumers** — which other modules pull from this directory

## When to call this vs slice/impact

- `slice` is a **single file's** neighborhood
- `impact` is a **single file's** transitive blast radius
- `focus` is a **directory's** cohesion + coupling pattern

For module-level refactors (rename a directory, split a module, extract a sub-package) `focus` is the entry point.

## Reporting

Structure the report:

1. **Module summary** — file count, total LOC, languages, dominant role
2. **Coupling evidence** — internal and external edge counts, without inventing a
   health score the tool did not return.
3. **Top 3 hubs within the module** by internal importer count — these are candidate "core" files for the module.
4. **External dependencies** — top 5 by file count of imports.
5. **External consumers** — top 5 by importer count. Zero represented consumers
   is not permission to refactor freely; verify manifests and runtime boundaries.

## Pair with

- `/loctree:follow scope:cycles` after focus, to spot circular imports within the module that focus doesn't render
- `/loctree:follow scope:twins` if the module looks like it duplicates another's responsibility
- `/loctree:slice` on each hub identified, before any internal refactor

## Anti-patterns

- Calling focus on a single file — that's `/loctree:slice`.
- Calling focus on the repo root for orientation — use `/loctree:repo-view` instead. Focus on root returns a flood of edges with no clear narrative.
- Ignoring the external consumers section because "I just want to clean up internals". External consumers constrain what's safe to rename.
