---
name: lsp-as-ai-engine-roadmap
status: planning
project: loctree-suite
parent_branch: feat/context-tool-alpha
created: 2026-05-05
---

# loctree-lsp roadmap — from VS Code helper to AI-engine daemon

## North Star

`loctree-lsp` becomes a structural-context daemon for AI agents — like
`rust-analyzer` is for Cursor, like `semgrep-lsp` is for CI pipelines.
Humans benefit (hover, codeLens) but the **first-class consumer is an agent**:
Codex CLI, Claude Code with LSP client, Junie, Cursor, Continue, etc.

Three design constants across all plans:

1. **Pointer-as-payload**: never inline 100+ KB into LSP responses. Materialize
   on disk under `<root>/.loctree/`, return pointers. Agents open the path.
2. **Same surface as CLI**: every `loct <command>` has a `loctree/<command>`
   equivalent. Snapshot lives in daemon RAM, query is sub-millisecond.
3. **Semantic over syntax**: rust-analyzer knows tokens; we know
   *meaning* — exports vs params, idiom tags, dispatch edges, env contracts,
   AICX intents. That's the diferentiator.

## Plan inventory

Foundation (already drafted, queued separately):

| # | Plan | Status |
|---|------|--------|
| 1 | `01-atlas-per-repo` — atlas at `<root>/.loctree/context-atlas/` | queued |
| 2 | `02-loctree-contextAtlas-request` — pointer for current atlas | queued |
| 3 | `03-codelens-importers` — passive structural annotation | queued (low pri) |
| 4 | `04-codeaction-open-atlas-card` — link diagnostics to cards | queued |

Agent-engine core:

| # | Plan | Why |
|---|------|-----|
| 5 | `05-loctree-slice-request` | pre-edit holographic context |
| 6 | `06-loctree-impact-request` | pre-refactor blast radius |
| 7 | `07-loctree-find-request` | semantic-aware symbol search |
| 8 | `08-loctree-aicx-request` | memory continuity between agents |
| 9 | `09-loctree-health-request` | repo-readiness gate |
| 10 | `10-background-watcher` | zero-manual rescan, fs events |
| 11 | `11-loctree-diff-request` | delta since last snapshot |
| 12 | `12-streaming-cursor-pattern` | paginate large responses (Codex Manifest Protocol) |
| 13 | `13-multi-workspace-context` | monorepo subprojects (vista-style) |
| 14 | `14-loctree-semantic-request` | idiom tags, dispatch edges, env contracts |
| 15 | `15-loctree-follow-request` | consolidated structural signals |

## Dependency graph

```
01-atlas-per-repo ─┬─→ 02-contextAtlas
                   └─→ 04-codeAction-open-atlas-card

10-background-watcher ─→ enables 11-diff (auto-invalidates snapshot)

12-streaming-cursor-pattern ─→ used by 5,7,15 (large slices/searches)

13-multi-workspace-context ─→ cross-cuts 5,6,7,8,9,15 (scope param)
```

10 + 12 + 13 are *infrastructure* plans; the rest are *feature* plans
that consume them. Land 10/12/13 in any order, but features should follow.

## Sequencing recommendation

**Wave 1** (drafted): 1, 2, 4 (atlas foundation + agent pointer + diagnostics link).
   Plan 3 deferred — passive UI, low pri under AI-engine paradigm.

**Wave 2** (core agent-engine): 5, 6, 7, 9 (slice/impact/find/health).
   Each is a self-contained request handler, independent.

**Wave 3** (infrastructure): 10, 12. Watcher + cursor pattern.

**Wave 4** (memory + multi-workspace): 8, 13. AICX integration + monorepo.

**Wave 5** (advanced): 11, 14, 15. Diff, semantic facts, consolidated follow.

**Wave 6** (tree-sitter foundation — enables true live update):

| # | Plan | Why |
|---|------|-----|
| 16 | `16-tree-sitter-foundation` | incremental parser layer, one engine for all languages |
| 17 | `17-live-ast-updates` | per-`didChange` AST refresh in microseconds |
| 18 | `18-symbol-level-granularity` | stable SymbolId via ast node hashes |
| 19 | `19-cross-language-unified-surface` | analyzer migrates from N parsers to one (per-language opt-in) |
| 20 | `20-loctree-astQuery-request` | structural search engine via tree-sitter query DSL |

Tree-sitter is the **foundation under live update** — without incremental
parsing every `didChange` would trigger a full reparse, killing the
real-time agent experience. Plan 16 lands the substrate; 17 wires it to
LSP edit events; 18 gives us symbol identity stable across edits; 19
unifies the analyzer; 20 turns the AST into a queryable surface.

## Acceptance for the whole roadmap

Each plan has its own. The roadmap is "done" when:

- Every CLI command of value to agents has an LSP equivalent.
- Snapshot stays in daemon memory; queries are <10ms.
- Pointer-as-payload is the contract for every response > 10 KB.
- AICX overlay is queryable per-file and per-symbol.
- A new agent (e.g. fresh Codex session) can bootstrap from the daemon
  in <1s using `loctree/contextAtlas` + `loctree/health` + AICX scope.

Vibecrafted with AI Agents (c)2024-2026 The LibraxisAI Team
