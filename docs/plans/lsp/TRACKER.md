---
name: lsp-roadmap-master-tracker
status: live
project: loctree-suite
parent_branch: feat/context-tool-alpha
created: 2026-05-05
last_updated: 2026-05-10T12:22:29Z

---

# LSP-as-AI-engine — Master Tracker

Living document. Every agent picking up a plan **must** update the
status table below before starting and after completing/failing.

---

## 1. Status Dashboard

Legend: `queued` | `in-progress` | `blocked` | `review` | `done` | `failed`

| #  | Plan                                | Status   | Owner   | Wave | Branch                       | Started      | Completed    | Report                                            |
|----|-------------------------------------|----------|---------|------|------------------------------|--------------|--------------|---------------------------------------------------|
| 01 | atlas-per-repo                      | done     | claude  | 1    | feat/lsp/01-atlas-per-repo   | 2026-05-06T18:30 | 2026-05-06T18:55 | reports/lsp/01-atlas-per-repo.md                  |
| 02 | loctree/contextAtlas request        | done     | claude  | 2    | feat/lsp/02-context-atlas    | 2026-05-06T23:00 | 2026-05-06T23:30 | reports/lsp/02-loctree-contextAtlas-request.md    |
| 03 | codeLens importers                  | done     | claude  | 4    | feat/lsp/codelens-live-analyzer | 2026-05-07T15:00 | 2026-05-07T17:00 | reports/lsp/03-codelens-importers.md              |
| 04 | codeAction open-atlas-card          | done     | claude  | 3    | feat/lsp/04-codeaction       | 2026-05-06T23:30 | 2026-05-07T00:00 | reports/lsp/04-codeaction-open-atlas-card.md      |
| 05 | loctree/slice request               | done     | claude  | 1    | feat/lsp/05-slice            | 2026-05-06T19:00 | 2026-05-06T19:30 | reports/lsp/05-loctree-slice-request.md           |
| 06 | loctree/impact request              | done     | claude  | 2    | feat/lsp/06-impact           | 2026-05-06T20:00 | 2026-05-06T20:30 | reports/lsp/06-loctree-impact-request.md          |
| 07 | loctree/find request                | done     | claude  | 3    | feat/lsp/07-find             | 2026-05-06T21:30 | 2026-05-06T22:00 | reports/lsp/07-loctree-find-request.md            |
| 08 | loctree/aicx request                | done     | claude  | 4    | feat/lsp/codelens-live-analyzer | 2026-05-07T15:00 | 2026-05-07T17:00 | reports/lsp/08-loctree-aicx-request.md            |
| 09 | loctree/health request              | done     | claude  | 1    | feat/lsp/09-health           | 2026-05-06T22:00 | 2026-05-06T22:30 | reports/lsp/09-loctree-health-request.md          |
| 10 | background watcher                  | done     | claude  | 1    | feat/lsp/10-watcher          | 2026-05-06T22:30 | 2026-05-06T23:00 | reports/lsp/10-background-watcher.md              |
| 11 | loctree/diff request                | done     | claude  | 2    | feat/lsp/codelens-live-analyzer | 2026-05-07T15:00 | 2026-05-07T17:00 | reports/lsp/11-loctree-diff-request.md            |
| 12 | streaming cursor pattern            | done     | claude  | 3    | feat/lsp/12-cursor           | 2026-05-07T00:30 | 2026-05-07T01:00 | reports/lsp/12-streaming-cursor-pattern.md        |
| 13 | multi-workspace context             | done     | claude  | 3    | feat/lsp/codelens-live-analyzer | 2026-05-07T15:00 | 2026-05-07T17:00 | reports/lsp/13-multi-workspace-context.md         |
| 14 | loctree/semantic request            | done     | claude  | 1    | feat/lsp/codelens-live-analyzer | 2026-05-07T15:00 | 2026-05-07T17:00 | reports/lsp/14-loctree-semantic-request.md        |
| 15 | loctree/follow request              | done     | claude  | 2    | feat/lsp/15-follow           | 2026-05-07T00:00 | 2026-05-07T00:30 | reports/lsp/15-loctree-follow-request.md          |
| 16 | tree-sitter foundation              | review   | codex   | 1    | feat/lsp/codelens-live-analyzer | 2026-05-08T07:00 | 2026-05-08T08:00 | reports/lsp/16-tree-sitter-foundation.md          |
| 17 | live AST updates                    | done     | claude  | 2    | feat/lsp/codelens-live-analyzer | 2026-05-08T09:00 | 2026-05-09T07:30 | reports/lsp/17-live-ast-updates.md                |
| 18 | symbol-level granularity            | done     | claude  | 3    | feat/lsp/codelens-live-analyzer | 2026-05-08T10:00 | 2026-05-09T11:00 | reports/lsp/18-symbol-granularity-v2.md           |
| 19 | cross-language unified surface      | review   | claude  | 3    | feat/lsp/codelens-live-analyzer | 2026-05-09T11:30 | 2026-05-09T13:00 | reports/lsp/19-cross-lang-stage-1.md              |
| 20 | loctree/astQuery request            | done     | claude  | 3    | feat/lsp/codelens-live-analyzer | 2026-05-08T07:00 | 2026-05-09T07:00 | reports/lsp/20-loctree-astQuery-request.md        |
| 21 | LSP polish pass (branding+fixtures+schemas) | done | claude  | -    | feat/lsp/codelens-live-analyzer | 2026-05-08T22:00 | 2026-05-08T22:25 | reports/lsp/21-lsp-polish-pass.md                  |
| 22 | loct context scope flag          | done     | codex   | -    | feat/lsp/codelens-live-analyzer | 2026-05-09T03:08 | 2026-05-10T12:22 | reports/lsp/22-context-scope-flag.md               |

**Status update protocol**: edit this table inline. Bump `last_updated` in
frontmatter. Append a one-line entry to §6 Activity Log with timestamp.

---

## 2. Tracks (parallel work-streams)

Tracks are designed so different agents can run concurrently without
stepping on each other. Each track has a recommended owner profile but
any capable agent can pick up.

### Track A — Atlas Foundation `[01 → 02 → 04]`
Sequential within track (each step depends on previous).
- 01 atlas-per-repo (foundation)
- 02 loctree/contextAtlas request (uses 01)
- 04 codeAction open-atlas-card (uses 01)
- **Recommended owner**: any agent comfortable with both `loctree-rs` and
  `loctree-lsp` (cross-crate plumbing).
- **Parallelism inside track**: 02 and 04 can be parallel after 01 lands.

### Track B — Agent Request APIs (parallel mini-fan-out) `[05, 06, 07, 09]`
All independent, all delegate to existing analyzer code via thin LSP
adapters. Pure RPC handler work.
- 05 loctree/slice
- 06 loctree/impact
- 07 loctree/find
- 09 loctree/health
- **Recommended owner**: 4 separate agents in parallel (one per plan).
- **Parallelism**: full fan-out from day one.

### Track C — AICX & Memory `[08]`
Single plan but specialized — touches AICX integration.
- 08 loctree/aicx request
- **Recommended owner**: agent familiar with AICX (Codex has prior
  AICX work in repo history).

### Track D — Watcher & Diff Infrastructure `[10 → 11]`
Sequential — 11 depends on 10's prev-snapshot retention.
- 10 background watcher + scanProgress
- 11 loctree/diff request
- **Recommended owner**: one agent for the whole track (continuity helps).

### Track E — Cursor Pagination `[12]`
Infrastructure adopted retroactively by 5, 7, 11, 14, 15. Lands once,
benefits everyone.
- 12 streaming cursor pattern
- **Recommended owner**: any agent — touches `loctree-lsp` only.

### Track F — Multi-workspace `[13]`
Cross-cutting — once landed, every plan that takes `project: Option<...>`
benefits. Best landed mid-roadmap.
- 13 multi-workspace context
- **Recommended owner**: any agent.

### Track G — Semantic & Follow `[14, 15]`
Mid-tier features mirroring CLI surface. Both independent.
- 14 loctree/semantic
- 15 loctree/follow
- **Recommended owner**: 2 agents in parallel.

### Track H — Tree-sitter Foundation `[16 → 17 → 18, plus 19 || 20 after 16]`
Critical path. Largest track. Wave 6 of the roadmap.
- 16 tree-sitter foundation (substrate)
- 17 live AST updates (depends 16)
- 18 symbol-level granularity (depends 17)
- 19 cross-language unified surface (depends 16, parallel with 17)
- 20 loctree/astQuery (depends 16, parallel with 17/18/19)
- **Recommended owner**: 1 agent for 16 (consolidate); after 16 lands,
  3 agents in parallel for 17/19/20; then 18 after 17.

### Track I — CodeLens Tail `[03]`
Low priority under AI-engine paradigm. Defer to Wave 4 unless an idle
agent grabs it.
- 03 codeLens importers
- **Recommended owner**: spare capacity.

---

## 3. Schedule by Wave

Each wave is "all selected plans land before next wave starts" — so the
slowest plan in a wave gates the wave. Estimated 4-8h per plan with one
agent (depends on plan size).

### Wave 1 — Foundations (5 parallel)
Goal: establish substrate that subsequent waves rely on.

| Slot | Plan                | Track | Why now                         |
|------|---------------------|-------|---------------------------------|
| 1.A  | 01 atlas-per-repo   | A     | unblocks 02, 04                 |
| 1.B  | 16 ts-foundation    | H     | unblocks 17, 19, 20             |
| 1.C  | 10 watcher          | D     | unblocks 11; enables real-time  |
| 1.D  | 05 slice            | B     | most-used agent surface         |
| 1.E  | 14 semantic         | G     | diferentiator vs other LSPs     |

**Wave 1 done** when all 5 land on `feat/context-tool-alpha` (or stack
on top via rebase merge). PR per plan, batch-merge once green.

### Wave 2 — Build on substrate (5 parallel)

| Slot | Plan                          | Track | Depends on |
|------|-------------------------------|-------|------------|
| 2.A  | 02 loctree/contextAtlas       | A     | 01         |
| 2.B  | 17 live AST updates           | H     | 16         |
| 2.C  | 11 loctree/diff               | D     | 10         |
| 2.D  | 06 loctree/impact             | B     | -          |
| 2.E  | 15 loctree/follow             | G     | -          |

### Wave 3 — Expand surface (5 parallel)

| Slot | Plan                              | Track | Depends on |
|------|-----------------------------------|-------|------------|
| 3.A  | 04 codeAction open-atlas-card     | A     | 01         |
| 3.B  | 18 symbol-level granularity       | H     | 17         |
| 3.C  | 19 cross-language extractors      | H     | 16 (parallel with 17/18) |
| 3.D  | 07 loctree/find                   | B     | -          |
| 3.E  | 13 multi-workspace                | F     | -          |

### Wave 4 — Final coverage (4 parallel)

| Slot | Plan                              | Track | Depends on |
|------|-----------------------------------|-------|------------|
| 4.A  | 12 cursor pagination              | E     | -          |
| 4.B  | 20 loctree/astQuery               | H     | 16         |
| 4.C  | 08 loctree/aicx                   | C     | -          |
| 4.D  | 09 loctree/health                 | B     | -          |

### Wave 5 — Tail (1 plan)

| Slot | Plan                              | Track | Depends on |
|------|-----------------------------------|-------|------------|
| 5.A  | 03 codeLens importers             | I     | -          |

**Total**: 4 waves of meaningful work + 1 tail wave. With 5 agents per
wave, ~20-32h of agent-time per wave (4-8h * 5 plans / 5 agents = serial
critical path of 4-8h per wave). End-to-end calendar time: depends on
agent availability and review cadence.

---

## 4. Dependency graph (critical path)

```
                             ┌──────────────────┐
                             │  16 ts-foundation│
                             └────────┬─────────┘
                                      │
           ┌──────────────────────────┼──────────────────────────┐
           │                          │                          │
           ▼                          ▼                          ▼
   ┌─────────────┐            ┌─────────────┐           ┌──────────────┐
   │  17 live AST│            │ 19 ts-ext.  │           │ 20 astQuery  │
   └──────┬──────┘            └─────────────┘           └──────────────┘
          │
          ▼
   ┌─────────────┐
   │ 18 symbol-id│
   └─────────────┘

┌─────────────────┐
│ 01 atlas-per-rep│
└─────┬───────────┘
      │
      ├──────► 02 contextAtlas
      └──────► 04 codeAction-open-card

┌─────────────────┐
│ 10 watcher      │
└─────┬───────────┘
      └──────► 11 diff

Independent (no deps):  05, 06, 07, 08, 09, 12, 13, 14, 15, 03

Critical path length:
- Tree-sitter chain:  16 → 17 → 18         (3 sequential plans)
- Atlas chain:        01 → 02|04           (2 sequential plans)
- Watcher chain:      10 → 11              (2 sequential plans)
- All others:         single hop           (1 plan)

Longest critical path = tree-sitter (Wave 1 → 2 → 3, three waves).
```

---

## 5. Coordination protocol

### Branch & commit

- Branch per plan: `feat/lsp/<plan-id>` (e.g. `feat/lsp/01-atlas-per-repo`)
- Branch base: latest `feat/context-tool-alpha` at the time of pickup
  (rebase forward if conflicts).
- Commit format: as specified in each plan's "Exit contract" section.
- Squash to one commit per plan before PR (clean linear history).

### PR convention

- One PR per plan.
- PR title: `feat(lsp): <plan-name>` (e.g. `feat(lsp): atlas per repo`).
- PR description includes:
  - Link to plan file (`plans/lsp/<id>-<name>.md`).
  - Acceptance criteria checklist (mirror from plan).
  - Summary of changes.
  - Verification log (output of `make precheck` + relevant tests).
- PR labels: `lsp`, `wave-<N>`, `track-<letter>`.

### Reports

- Each completed plan writes a report to
  `.vibecrafted/reports/lsp/<id>-<name>.md`.
- Report includes:
  - Frontmatter: `status: completed|failed`, `agent`, `branch`, `pr`.
  - Findings (links to changed files with line refs).
  - Verification output (test names + pass/fail).
  - Surprises / deviations from plan.
  - Recommended follow-ups (other plans this work surfaced).

### Tracker updates (mandatory)

Before starting:
1. Edit row in §1 Status Dashboard: status `queued` → `in-progress`,
   set `Owner`, `Started`.
2. Append to §6 Activity Log:
   `2026-MM-DDThh:mm:ssZ <plan-id> <agent> started`.
3. Bump `last_updated` in frontmatter.

After completion:
1. Edit row: status → `done` (or `failed`), set `Completed`.
2. Append to §6: `2026-MM-DDThh:mm:ssZ <plan-id> <agent> completed
   (PR #N) [report]`.
3. Bump `last_updated`.

### Conflict resolution

Two agents claiming the same row at the same time = race condition. To
avoid:
- Always pull tracker before edit.
- Atomic edit: pull → modify → push within seconds.
- If conflict, the second agent picks a different plan from the same
  wave (capacity is fungible at the wave level).

### Wave gating

- A wave is **complete** when every plan in it lands on
  `feat/context-tool-alpha` (or main if PR'd separately).
- Next wave's plans should not start until their dependencies are merged.
  Exception: a plan with NO upstream deps in the current wave can start
  early (skip the wait).

---

## 6. Activity Log (append-only)

```
2026-05-05T11:30:00Z TRACKER initialized; 20 plans queued, 1 roadmap doc
2026-05-06T18:30:00Z 01-atlas-per-repo claude started (branch feat/lsp/01-atlas-per-repo)
2026-05-06T18:55:00Z 01-atlas-per-repo claude completed (179 tests pass; report written)
2026-05-06T19:00:00Z 05-loctree-slice-request claude started (branch feat/lsp/05-slice)
2026-05-06T19:30:00Z 05-loctree-slice-request claude completed (44 tests pass; report written)
2026-05-06T20:00:00Z 06-loctree-impact-request claude started (branch feat/lsp/06-impact)
2026-05-06T20:30:00Z 06-loctree-impact-request claude completed (52 tests pass; severity heuristic refined to depth-dominant)
2026-05-06T21:30:00Z 07-loctree-find-request claude started (branch feat/lsp/07-find)
2026-05-06T22:00:00Z 07-loctree-find-request claude completed (15 new tests; mode/lang/dead_only/exported_only/limit filters)
2026-05-06T22:00:00Z 09-loctree-health-request claude started (branch feat/lsp/09-health)
2026-05-06T22:30:00Z 09-loctree-health-request claude completed (45 tests pass; readiness gate green/yellow/red + recommended actions)
2026-05-06T22:30:00Z 10-background-watcher claude started (branch feat/lsp/10-watcher)
2026-05-06T23:00:00Z 10-background-watcher claude completed (55 tests pass; live LSP-stdio smoke deferred — analyzer wiring complete)
2026-05-06T23:00:00Z 02-loctree-contextAtlas-request claude started (branch feat/lsp/02-context-atlas, stacked on 01)
2026-05-06T23:30:00Z 02-loctree-contextAtlas-request claude completed (49 tests pass; manifest pointer over JSON-RPC)
2026-05-06T23:30:00Z 04-codeaction-open-atlas-card claude started (branch feat/lsp/04-codeaction, stacked on 01)
2026-05-07T00:00:00Z 04-codeaction-open-atlas-card claude completed (Track A finisher; loctree.openAtlasCard executeCommand)
2026-05-07T00:00:00Z 15-loctree-follow-request claude started (branch feat/lsp/15-follow)
2026-05-07T00:30:00Z 12-streaming-cursor-pattern claude started (branch feat/lsp/12-cursor)
2026-05-07T00:30:00Z 15-loctree-follow-request claude completed (cycles/dead/twins/hotspots/all wired; trace/commands/events/pipelines stubbed)
2026-05-07T01:00:00Z 12-streaming-cursor-pattern claude completed (Paginated<T> + CursorState; retroactive wrap deferred to follow-up cuts)
2026-05-07T15:00:00Z 03/08/11/13/14 claude started single-run on branch feat/lsp/codelens-live-analyzer (per docs/plans/lsp/MISSING_03_08_11_13_14_SINGLE_RUN.md)
2026-05-07T17:00:00Z 03-codelens-importers claude completed (contract closure: title-carrier kept, end-to-end tests added, docs aligned)
2026-05-07T17:00:00Z 13-multi-workspace claude completed (workspaces.rs + Backend.routed_snapshot/routed_root + loctree/workspaces handler + watcher reload of subprojects; 7 integration tests)
2026-05-07T17:00:00Z 11-diff claude completed (loctree/diff with epoch/lastScan/lastQuery + DiffSession + watcher rotates last_scan; unsupported_since for git revs deferred to v2)
2026-05-07T17:00:00Z 14-semantic claude completed (loctree/semantic file/symbol/project scopes + kinds filter + Plan 12 cursor pagination for project scope)
2026-05-07T17:00:00Z 08-aicx claude completed (loctree/aicx via compose_memory_slice + graceful aicx_unavailable + kinds filter incl. failure alias)
2026-05-08T08:00:00Z 16/20 codex stage1 completed (loctree-ast JS/TS/TSX substrate + incremental parse smoke; loctree/astQuery registered with snapshot-file MVP, typed errors, capability, and tiny query library)
2026-05-08T22:00:00Z 21-lsp-polish-pass claude started (operator dogfooding day surfaced 3 RustRover-visible rough edges: serverInfo branding, sub-workspace WARN noise from empty .loctree/ markers, missing JSON Schemas on experimental.loctree/* capability advertisements)
2026-05-08T22:25:00Z 21-lsp-polish-pass claude completed (3 commits 5345645e/c26e9662/5b90f369 on feat/lsp/codelens-live-analyzer: serverInfo "Loctree Language Server" + skip empty .loctree/ in workspace discovery + JsonSchema derives on 10 typed namespaces with shared request_capability builder; cargo test --workspace + clippy -D warnings green; report at reports/lsp/21-lsp-polish-pass.md)
2026-05-09T03:08:12Z 22-context-scope-flag codex started (deterministic `loct context --scope` from Plan 22; CLI/MCP ContextPack path, named scopes, JSON/markdown truth-intent split)
2026-05-08T09:42:00Z 17 claude stage2 review (live_ast LiveAstStore + LiveDocument + loctree/documentChanged notification; ast_query consumes live tree before disk reparse; capability flips to liveDocumentCache: true; FULL sync only — INCREMENTAL InputEdit translation + Plan 19 extractors deferred; 14 live_ast unit tests + 2 ast_query live-cache tests + 289 LSP tests green)
2026-05-08T10:06:00Z 18 claude stage2 v1 review (commit b5f6e308: SymbolIdV1 newtype VERSION=v1-string + find/aicx symbol_id round-trip + capability symbol_id_version + loctree/symbolChanged advertised available:false with Plan 19 deferral reason; v2 byte-range hash + symbolChanged emit deferred behind extractors)
2026-05-08T23:50:00Z stage2 narrative truth pass claude (plan-doc frontmatter aligned with TRACKER + code: 14/15 status queued→done with Stage 2 truth-pass delta sections, 18 status queued→review with v1 delta + v2 deferral; LOCTREE_NEXT item 3 updated; tracker row 18 set to review/claude; cargo check + 295 LSP tests + 2 loctree-ast tests still green)
2026-05-09T07:00:00Z 20-loctree-astQuery-request claude completed (plan close-out: loctree-lsp/tests/ast_query.rs with 6 real-query integration tests — TS function_declaration / JS lexical_declaration glob / TSX jsx_element / curated @library/lexical_declarations / language:auto cross-grammar dispatch / typed query_compile_error + language_unsupported; fixtures under loctree-lsp/tests/fixtures/ast_query/{greet.ts,util.js,Button.tsx}; cargo test -p loctree-lsp --test ast_query green and clippy --all-targets -D warnings green; plan frontmatter status mvp-completed→done and final acceptance box checked)
2026-05-09T07:30:00Z 17 claude stage2-v2 done (review→done: TextDocumentSyncKind::INCREMENTAL wired in server_capabilities; LiveDocument carries content+tree; new translate_change_event/translate_change_events helpers convert TextDocumentContentChangeEvent → tree_sitter::InputEdit over UTF-16 LSP positions; LiveAstStore::apply_change composes per-event edits and calls Parsers::parse_incremental; Backend::did_change switches to apply_live_ast_changes; capability JSON flips sync_mode→"incremental", incremental_edits→true, position_encoding→"utf-16"; new loctree-lsp/tests/live_ast.rs with 6 tests covering rename / multi-event / delete / range-less fall-through / 100-edit benchmark / live-cache parity; bench p50=0.017ms p99=0.235ms total=3.7ms (gate <100ms); cargo test -p loctree-lsp green at 307 tests, clippy --all-targets -D warnings green; report at reports/lsp/17-live-ast-incremental.md)
2026-05-09T13:00:00Z 19 claude stage1 review (queued→review: new loctree-ast/src/extractors/{mod,ts,js}.rs + LangExtractor trait + ExportSymbol/ImportEntry/ImportBinding/CallEntry contract types; TsExtractor/TsxExtractor/JsExtractor over tree_sitter_typescript & tree_sitter_javascript with OnceLock-cached queries; analyzer.parser config knob + LOCTREE_PARSER env override + LoctreeConfig::parser_strategy() with normalize fallback; loctree-rs/src/analyzer/scan.rs gains feature-gated ts_dispatch_js dispatching JS/TS/TSX/JSX/MJS/CJS/CTS/MTS through extractors → FileAnalysis (imports/exports/symbol_usages); CallEntry pub-use surfaces in loctree::types; new loctree-ast/tests/extractors.rs (2 tests) + loctree-rs/tests/ts_extractors_ts.rs (4 integration tests on simple_ts fixture) + loctree-rs/tests/ts_extractors_parity.rs (#[ignore] hand-counted parity 100% exports/imports); other languages (py/rs/go/css/dart/SFC) explicitly Stage-2; report at reports/lsp/19-cross-lang-stage-1.md)
2026-05-09T11:00:00Z 18 claude v2 done (review→done: SymbolIdV1 gains Default + is_empty + with_symbol_id builder; new SymbolIdV2 newtype `<file>::<kind>::<name>::<hash16>` via DefaultHasher with to_v1() projection; ExportSymbol gains pub symbol_id: SymbolIdV1 with serde-default + skip_serializing_if back-compat; new live_ast surface — LiveSymbol/SymbolMetadata/SymbolChange{Kind,Location}/SymbolChanged + LoctreeSymbolChanged notification + extract_live_symbols Query-walker over function_declaration/class_declaration/abstract_class_declaration/generator_function_declaration/function_signature for TS/JS/TSX + diff_symbol_sets classifier with body_hash sibling-shift suppression; Backend.symbol_tracker per-URI map wired through apply_live_ast_changes / update_live_ast / did_close / shutdown; capability flips loctree/symbolChanged.available false→true with kinds=[added,removed,moved,rewritten]; new loctree-lsp/tests/symbol_granularity.rs with 7 tests — rename single-rewritten + add + remove + move-with-edit + pure-shift-silent + class-rename + capability-flip; +12 SymbolIdV1/V2 unit tests in loctree::types; 27 ExportSymbol struct-literal call-sites patched with symbol_id default across loctree-rs analyzer + LSP fixture surface; gates green: cargo test -p loctree --lib types ok 25, cargo test -p loctree-lsp ok 314, cargo test -p loctree ok 179 + e2e, cargo clippy --workspace --all-targets -- -D warnings 0; report at reports/lsp/18-symbol-granularity-v2.md)
2026-05-10T12:22:29Z 22-context-scope-flag codex completed (done: deterministic `loct context --scope` parser/composer/MCP surface verified; added parser + scoped ContextPack regression tests; report at reports/lsp/22-context-scope-flag.md)
```

---

## 7. Capacity & throughput notes

- **5 agents at full saturation** = ~25 plans across 4-5 waves.
- **Realistic**: 3-4 agents active, some plans take 1-2 days for review +
  fixes. Calendar estimate: 2-3 weeks for waves 1-4, plus tail.
- **Risk hotspots** (review carefully when these PRs come in):
  - 16 (ts-foundation): adds many deps + new module/crate; cargo lock
    churn likely.
  - 19 (ts-extractors): largest plan; per-language acceptance lets it
    land incrementally but watch for analyzer regressions vs OXC.
  - 17 (live AST): touches LSP edit path; race conditions possible
    between watcher (Plan 10) and incremental updater.
  - 13 (multi-workspace): cross-cuts many handlers; integration tests
    must cover both single- and multi-workspace setups.

---

## 8. Out-of-band escalations

If an agent gets stuck:
1. Mark plan as `blocked` in §1; note reason in report.
2. Append `<id> blocked: <reason>` to §6.
3. Operator (or another agent) picks up — may need to spike a sub-plan
   first (file a new plan under `plans/lsp/<id>-<sub>.md` if substantial).

Vibecrafted with AI Agents (c)2024-2026 The LibraxisAI Team
