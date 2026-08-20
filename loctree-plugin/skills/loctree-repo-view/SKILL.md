---
name: loctree-repo-view
description: One-shot repository overview — file count, LOC by language, top hubs, health summary. Use at first contact with an unfamiliar repo, when the agent needs the 30-second elevator pitch of "what is this codebase". Triggers on phrases "repo overview", "what is this repo", "/loctree:repo-view", "loct repo-view", "show me the codebase", "repository summary", "loctree-suite repo-view".
argument-hint: "[no arguments]"
allowed-tools:
  - mcp__loctree-mcp__repo-view
---

# /loctree:repo-view — repository overview

Call `mcp__loctree-mcp__repo-view` with the current project (no arguments needed).

## What you get

- **File count** + total LOC
- **Language distribution** — bytes/LOC per language (TS/JS/Rust/Python/Shell/Make/Astro/Svelte/Vue/Zig/CSS/etc.)
- **Top hubs** — files by importer count (>10 = high fan-in)
- **Module structure** — top-level directories with file counts
- **Health summary** — cycles, dead exports, twin counts (if pre-computed)
- **Languages list** as a flat array (`ts`, `rs`, `shell`, `make`, …)

## When to use this vs context

- `repo-view` is a **flat overview** — quick read, ~30 lines
- `context` is a **full Atlas** — six structured cards, ~1KB JSON or ~80 lines markdown

For first-contact orientation: `repo-view` first (cheap), then `context` if more depth needed. The `context` orchestrator already calls repo-view-equivalent under the hood, so don't double-call when you already invoked `/loctree:context`.

## Reporting

Structure as a one-paragraph elevator pitch + a small table:

> "vc-operator: Rust workspace, 11 files, 4281 LOC, edition 2024. TUI dispatcher console. Top hub `src/launch.rs` (3 importers). Worktree dirty, 1 commit ahead origin. No cycles, no twins detected."

Then a 3-column table: Language | Files | LOC.

If the repo has no `.loctree` snapshot yet, call surfaces `not_measured` everywhere — recommend `loct` first scan via the orchestrator.

## Pair with

- After repo-view → `/loctree:tree` for directory structure with LOC
- After repo-view → `/loctree:focus` on the largest top-level directory
- After repo-view → `/loctree:context` if planning meaningful structural work

## Anti-patterns

- Treating a previous `repo-view` as current after the living tree, branch, or
  snapshot moved. Refresh when the receipt or worktree says the map is stale.
- Using `repo-view` for symbol search — that's `/loctree:find`.
- Treating the language list as definitive coverage — `repo-view` shows what's *scanned*, not what the analyzer fully understands. For language-aware features (live AST, semantic tags) check the LSP capabilities instead.
