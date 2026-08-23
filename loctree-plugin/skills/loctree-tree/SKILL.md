---
name: loctree-tree
description: Render directory structure with LOC counts per file/dir. Use when the agent needs to understand the physical layout of the codebase, where files live, depth of nesting, and which dirs are heavy. Triggers on phrases "show directory tree", "/loctree:tree", "loct tree", "directory structure", "where is the code", "repo layout", "filesystem overview".
argument-hint: "[optional: subdirectory to render, default = repo root] [optional: depth=N]"
allowed-tools:
  - mcp__loctree-mcp__tree
---

# /loctree:tree — directory structure with LOC

Call `mcp__loctree-mcp__tree` with project + optional path + optional depth from `$ARGUMENTS`.

## What you get

A directory tree rendered with:

- LOC count per file
- File count per directory (cumulative)
- Language hint per file (extension)
- Optional `loc_threshold` for size summaries/highlights; it does not filter the tree

## Argument routing

- No args → full repo tree from root, default depth (usually 3-4 levels)
- Path arg → tree from that subdirectory
- `depth=N` → limit nesting to N levels
- `loc_threshold=N` → mark/summarize items at or above N lines

## When to use this vs focus

- `tree` is **physical layout** — directory hierarchy with sizes
- `focus` is **module semantics** — files + edges + dependencies + consumers

For "where is the code" questions: `tree`. For "how is the code wired" questions: `focus`.

## Reporting

Render the tree as ASCII with LOC suffix on each file. Truncate large directories with `…` and a `+N more files` hint. Do not dump 200+ lines into chat — page with `path` argument or `depth=2`.

Example shape:

```text
loctree-suite/
├── loctree-rs/             (45 files, 12_400 LOC)
│   ├── src/
│   │   ├── lib.rs           (89 LOC)
│   │   ├── types.rs         (1240 LOC)  ← hub: 65 importers
│   │   └── snapshot.rs      (892 LOC)
│   └── tests/               (8 files, 1_120 LOC)
├── loctree-mcp/            (3 files, 540 LOC)
└── loctree_lsp/            (14 files, 3_419 LOC)
```

Annotate hubs inline if known from the snapshot (`← hub: N importers`).

## Pair with

- After `tree` → `/loctree:focus` on the largest sub-directory
- After `tree` → `/loctree:repo-view` for the language distribution that tree doesn't render

## Anti-patterns

- Dumping the full tree of a 500-file repo to chat. Use `depth=2` then drill down.
- Using `tree` for symbol queries — that's `/loctree:find`.
- Calling tree on a non-repo directory — fails-fast on missing `.git`. Use `/loctree:context` first to ensure repo detection passes.
