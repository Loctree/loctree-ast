---
name: loctree-context
description: Materialize the loctree Context Atlas — six cards (core, structural, runtime, memory-trail, verification-gates, risk-register) — for the current project. Use at session start, after `clear` / `compact`, when entering an unfamiliar repo, when planning a refactor, or whenever the agent needs full structural orientation in one call. Triggers on phrases "loctree context", "context atlas", "get full repo overview", "where am I in this codebase", "atlas pack", "/loctree:context".
argument-hint: "[optional: file path to scope, or task description for token-overlap matcher]"
allowed-tools:
  - mcp__loctree-mcp__context
---

# /loctree:context — Context Atlas

The Context Atlas is the **agent-ready output of the snapshot** — Loctree's
snapshot-first structural evidence, composed into cards. The atlas explains its
scope and freshness; it does not supersede manifests, runtime probes, or direct
source reads.

Call `mcp__loctree-mcp__context` with the user's project (default: current working
directory). Attach AICX when historical intent is relevant and available.

## Argument routing

If `$ARGUMENTS` is a file path that exists in the repo → pass as `file:` to scope the atlas to that file's neighborhood.

If `$ARGUMENTS` is a task description (free text, more than 3 words, doesn't resolve as a path) → pass as `task:` for token-overlap matcher.

If `$ARGUMENTS` is empty → repo-level atlas (default).

## What you get back

Six cards plus a receipt. Read at minimum:

- **`core`** — repo identity, current risk summary, authority labels, safe next commands
- **`structural`** — files, symbols, imports, consumers, entrypoints in scope
- **`runtime`** — runtime behavior, framework hints, env contracts, reachability
- **`memory-trail`** — prior decisions, outcomes, tasks (AICX overlay)
- **`verification-gates`** — likely tests + commands to validate changes
- **`risk-register`** — hotspots, cache health, stale assumptions, next risk-reducing actions

The `receipt` carries the snapshot fingerprint and staleness markers. If `staleness.dirty_worktree=true`, your atlas mirrors uncommitted edits, not committed state.

## Authority labels

Preserve the labels returned by the tool. `repo_verified` means repository-derived
evidence, not universal runtime proof; `loctree_derived` is analyzer inference;
`semantic_guess` and `stale_or_unknown` require direct verification.

`aicx_failure` is special: anti-recommendation. Don't repeat that path.

## Reporting style

After receiving the atlas, summarize for the user in 5-8 lines:

1. Repo identity + branch + HEAD
2. Worktree state (clean / dirty)
3. Top 1-3 hubs from the structural card (importers count + authority)
4. One sentence on most recent intent from memory-trail (if AICX attached)
5. Recommended next safe command from action card
6. Any `aicx_failure` paths to avoid

Don't dump the full atlas JSON to chat — link to atlas dir path instead. The user reads atlas cards in the on-disk format if needed.

## Failure modes

- `MCP fails-fast if <repo-root> lacks .git` — surface this as "not a repo, run `git init` first" rather than retrying.
- Stale snapshot metadata → refresh through the supported CLI/MCP rescan surface,
  then confirm the new receipt rather than inventing an unverified parameter.
- Empty `structural` / `runtime` cards → check scope, language support, snapshot
  freshness, and known entrypoints. Empty can be legitimate or a coverage defect.
