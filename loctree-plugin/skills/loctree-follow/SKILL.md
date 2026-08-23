---
name: loctree-follow
description: Pursue structural signals — dead code, cycles, duplicate exports (twins), hotspots, runtime traces, command/event/pipeline graphs. Use during cleanup, audit, refactor planning, or when investigating "is this used?" questions at module/system scope. Triggers on phrases "follow signals", "find dead code", "find cycles", "find duplicates", "show hotspots", "trace handler", "/loctree:follow", "structural audit", "what's dead in this repo".
argument-hint: "<scope: dead|cycles|twins|hotspots|trace|commands|events|pipelines|all> [extra-args]"
allowed-tools:
  - mcp__loctree-mcp__follow
---

# /loctree:follow — signal pursuit

Call `mcp__loctree-mcp__follow` with the scope from `$ARGUMENTS`. Default scope is `all` if user provides no scope.

## Scopes

| Scope | What it surfaces | When to call |
|---|---|---|
| `dead` | Symbols with zero consumers (candidates for deletion) | Pre-cleanup audit, forgotten experiments |
| `cycles` | Circular imports — strongly connected components | Build / type-check failures, slow incremental compile |
| `twins` | Duplicate exports across files — same name, multiple definition sites | Pre-merge consolidation, architecture review |
| `hotspots` | Files with high fan-in or churn | Refactor priority, hub awareness |
| `trace` | Handler-graph traversal — which dispatchers reach which handlers | Cross-layer coverage check (FE↔BE, API↔DB) |
| `commands` | Tauri command bridge / explicit dispatch tables | Tauri / IPC apps |
| `events` | Event emitters and subscribers across modules | Pub-sub maturity check |
| `pipelines` | Multi-stage pipeline graphs (build / data / CI) | DevOps / data pipeline audit |
| `all` | Compact omnibus — top 3 from each scope | Cold-start audit overview |

## Reporting

For each scope, surface:

- **`dead`** — list candidate dead symbols with file:line; flag confidence (high/medium/low) based on `pub use` re-export awareness; recommend `/loctree:impact` on each before deletion to confirm zero transitive consumers
- **`cycles`** — list each SCC with files involved; flag severity (`breaking` / `bidirectional` / `structural`); propose break-edge in the lowest-cohesion edge
- **`twins`** — list duplicate symbol names with all definition sites; flag whether it's intentional re-export vs accidental shadowing; recommend single-source-of-truth via `find <symbol> mode:where-symbol`
- **`hotspots`** — sorted hubs; for each, slot for `/loctree:slice` recommendation
- **`trace`** — handler reachability map; flag uncovered dispatchers and unreachable handlers (= dead code with type-system camouflage)
- **`commands` / `events` / `pipelines`** — domain-specific graphs; surface coverage gaps

## Authority

`follow` results are `loctree_derived` analyzer inference. Re-exports, dynamic
loading, generated wiring, reflection, manifests, and incomplete language
coverage can change the operational verdict. Run impact, then verify the real
entrypoint/runtime/test path before `git rm`.

## Pair with

- After `dead` → `/loctree:impact` per candidate, then batch `git rm` in a "remove dead code" PR
- After `cycles` → `/loctree:slice` on the proposed break-edge file, plan the import direction reversal
- After `twins` → `/loctree:find <name> mode:where-symbol` to confirm definition sites; `/loctree:impact` to know consumer split
- After `hotspots` → `/loctree:slice` + `/loctree:impact` per hub before any modification

## Anti-patterns

- `git rm` on dead-list without per-symbol impact verification. The use-graph misses `pub use` chains.
- Treating cycles as cosmetic — they slow type-check, slow incremental compile, and hide circular initialization bugs.
- Treating twins as harmless re-exports without confirming. Two definitions of the same symbol shipped from two paths is silent drift waiting to happen.
- Calling follow with no scope and dumping the full `all` output to chat — that's tens of KB. Page it: `/loctree:follow scope:dead` first, then `cycles`, then `twins`.
