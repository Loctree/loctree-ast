# LOCTREE_NEXT

Strategic direction for Loctree after the April 2026 product/repo review.

This is not a feature wishlist. It is a convergence document for the next
shape of Loctree, grounded in three sources:

- the full strategic conversation in `../Niezwyke-wazna-rozmowa-o-loctree.md`
- the current `loctree-suite` codebase on `release/v0.9.1`
- the current AICX repo at `$HOME/Libraxis/vc-runtime/aicx`

## P0 Undelivered Plan Sweep - 2026-05-08

This sweep reconciles the live repo against plan/report material found under
`$HOME/.vibecrafted/inbox` and `$HOME/.vibecrafted/artifacts`. It is a catalog,
not an implementation plan dump. The important truth: the LSP request layer is
partly real, but the AST/live-symbol substrate that would make Loctree Suite an
AI-engine daemon is still mostly unbuilt.

### 0. Context scope flag

Status: Plan 22 implementation landed locally on 2026-05-09 and is in review.

Why it matters: operators need a deterministic truth primitive for "where are
we looking?" that does not drift with embedding state or keyword luck. `--scope`
now owns the file set; `--task` is intent/ranking on top when paired with scope.

Current repo evidence:

- `loct context --scope 'path:loctree-rs/src/cli/' --full` emits top-level
  `scope` metadata and `risk.cache_scope = Scoped(<fingerprint>)`.
- `.loctree/scopes.toml` defines `context-pipeline` as the first named scope.
- The markdown TL;DR renders `**Scope.**` and `**Task.**` lines before "Top 3
  things to know".

### 1. Tree-sitter foundation / tree-sitter substrate

Status: Stage 1 landed on 2026-05-08; still blocks full plans 17-19.

Why it matters: without an incremental parser layer, LSP edit-time intelligence
falls back to full snapshots and heterogeneous analyzer paths. Tree-sitter is
not the product, but it is the substrate needed for live AST, symbol identity,
cross-language extractors, and structural AST queries.

Evidence/source artifact:

- `docs/plans/lsp/TRACKER.md` and
  `$HOME/.vibecrafted/inbox/Loctree/loctree-suite/plans/lsp/TRACKER.md` keep
  plan 16 `tree-sitter foundation` queued.
- `docs/plans/lsp/16-tree-sitter-foundation.md` requires a `loctree-ast` crate
  or `loctree-rs/src/ast/`, parser registry, `LoctreeTree`, incremental parse,
  multi-language parse tests, and benchmark evidence.
- `$HOME/.vibecrafted/inbox/EPIC-SESSION-2026-05-05/STARTER-PACK.md` records
  the LSP roadmap as including "tree-sitter foundation + live AST + symbol-id +
  extractors + astQuery".

Current repo evidence after Stage 1:

- Workspace now includes `loctree-ast`, a narrow tree-sitter crate with
  `LoctreeTree`, `LangParser`, and `Parsers`.
- Supported parser paths are currently JS, TS, and TSX only.
- `loctree-rs` cold-scan analyzer still uses OXC/custom analyzers; no migration
  happened in this cut.
- Deferred: Rust/Python/Go/CSS/HTML/Vue/Svelte/Astro grammars, extractors,
  benchmarks, and live LSP document-cache integration.

Next cut: add live document state for TS/JS first, then decide whether the
additional grammars belong in `loctree-ast` before extractor parity work.

### 2. Live AST updates from LSP edits

Status: MVP landed on 2026-05-08 (P0 Stage 2). FULL sync only — INCREMENTAL
edit translation and per-language extractors still pending.

Why it matters: this is the difference between "daemon with a recent snapshot"
and "daemon that understands the file currently being edited". Agents need the
latter before relying on LSP state for surgical edits.

Evidence/source artifact:

- `docs/plans/lsp/17-live-ast-updates.md` requires `loctree-lsp/src/live_ast.rs`,
  incremental `textDocument/didChange`, per-document `LoctreeTree` state, and a
  `loctree/documentChanged` notification.

Current repo evidence after MVP:

- `loctree-lsp/src/live_ast.rs` carries `LiveAstStore`, `LiveDocument`, and the
  `LoctreeDocumentChanged` notification handle (`loctree/documentChanged`).
- `loctree-lsp/src/backend.rs` keeps a `LiveAstStore`, parses on `did_open` /
  `did_change` / `did_save`, drops on `did_close`, and emits
  `loctree/documentChanged` after every successful parse.
- Capability JSON advertises `loctree/documentChanged: { available: true,
  languages: ["javascript","typescript","tsx"], sync_mode: "full",
  incremental_edits: false, extractors: false }`.
- `loctree-lsp/src/ast_query.rs` (`compute_with_live`) reads the live tree
  before reparsing from disk, so `loctree/astQuery` already runs against
  unsaved buffers.
- `TextDocumentSyncKind::FULL` is still in place; INCREMENTAL `InputEdit`
  translation and per-language exports/imports extractors are deferred.

Next cut: convert `did_change` content events into `tree_sitter::InputEdit`s
and stand up the Plan 19 extractors so the notification can carry symbol-level
diff fields without breaking the additive serde contract.

### 3. Symbol-level granularity

Status: v1 string contract landed on 2026-05-08 (P0 Stage 2). v2 stable
byte-range hash + `loctree/symbolChanged` notification still deferred behind
Plan 19 extractors.

Why it matters: current symbol identity is still mostly file/name shaped. That
is too coarse for per-function AICX overlays, symbol diffs, edit tracking, and
future "why was this function changed?" agent workflows. Stage 2 ships the
minimum credible wire contract so `loctree/find` and `loctree/aicx` can take
`symbol_id` today; v2 lifts the contract to byte-range stability when the live
tree has extractors to ride on.

Evidence/source artifact:

- `docs/plans/lsp/18-symbol-level-granularity.md` requires a typed `SymbolId`,
  per-symbol metadata, `loctree/symbolChanged`, and `symbol_id` precision for
  `loctree/find` and `loctree/aicx`.

Current repo evidence after v1:

- `loctree-rs/src/types.rs` exposes `SymbolIdV1` — a typed newtype around the
  `<file>::<symbol>` string used by every existing Layer 3 semantic analyzer,
  with `VERSION = "v1-string"` and a documented v2 deferral that points at
  Plan 17 (live cache, MVP landed) plus Plan 19 (per-language extractors).
- `loctree-lsp/src/find.rs` accepts `symbol_id: Option<SymbolIdV1>` and echoes
  `symbol_id_version` in every `FindResponse` (serde-default, additive).
- `loctree-lsp/src/aicx.rs` is wired to the same wire contract via the
  capability advertisement (`loctree/aicx.symbol_id_version`).
- `loctree-lsp/src/backend.rs` advertises `loctree/symbolChanged` as
  `available: false` with an explicit reason: stable byte-range `SymbolId`
  (Plan 18 v2) needs Plan 19 extractors over the live tree-sitter cache from
  Plan 17 before it can ship.
- `ExportSymbol` does **not** carry a `symbol_id` field yet — that lift is
  v2 work behind extractors.

Proposed single-run cut for v2: land per-language extractors (Plan 19), then
populate `ExportSymbol.symbol_id` and emit `loctree/symbolChanged` from the
Plan 17 live cache. Until then the v1 string contract is the truthful wire.

### 4. Cross-language unified parser/extractor surface

Status: missing, large follow-on after plan 16.

Why it matters: Loctree still has real semantic value in its heterogeneous
analyzers, but the split parser model keeps cold scan, live LSP state, and
future language additions from sharing one extraction surface.

Evidence/source artifact:

- `docs/plans/lsp/19-cross-language-unified-surface.md` calls for
  `LangExtractor`, per-language tree-sitter extractors, parser feature flags,
  and parity tests against current OXC/custom outputs.

Current repo evidence:

- `loctree-rs/src/analyzer/ast_js/*` uses OXC for JS/TS.
- `loctree-rs/src/semantic/{shell,make,python,rust,tauri}.rs` and related
  analyzer code still contain runtime-specific/custom extraction logic.
- There is no `analyzer.parser = "ts"|"oxc"` config path and no
  `ts_extractors_*` test family.

Proposed single-run cut: start with feature-flagged TS/JS parity only. Treat
Python/Rust/Go/CSS/SFC extractors as subsequent cuts after the common trait is
proven.

### 5. `loctree/astQuery`

Status: MVP landed on 2026-05-08 with live-document override added in P0
Stage 2 the same day.

Why it matters: `astQuery` is the structural search engine agents actually
want when grep is too blunt and full semantic search is too fuzzy.

Evidence/source artifact:

- `docs/plans/lsp/20-loctree-astQuery-request.md` requires a custom
  `loctree/astQuery` request, typed query errors, scoped matches, and a curated
  query library under `loctree-lsp/queries/`.

Current repo evidence after MVP + Stage 2:

- `loctree-lsp/src/lib.rs` registers `loctree/astQuery`.
- `loctree-lsp/src/backend.rs` advertises `loctree/astQuery` as available with
  languages `javascript`, `typescript`, and `tsx`, and now reports
  `liveDocumentCache: true` with a note pointing at `crate::live_ast`.
- `loctree-lsp/src/ast_query.rs` runs read-only tree-sitter queries through
  `compute_with_live`, which prefers the per-URI live tree from
  `LiveAstStore` and falls back to reparsing from disk when no document is
  open.
- `loctree-lsp/queries/` contains a tiny `lexical_declarations` library for
  JS/TS/TSX.

Next cut: grow the curated query library from real agent use and add scope
modes (function-level, exported-only) once Plan 19 extractors land.

### 6. Semantic and call-site precision gaps

Status: partially delivered, with sharp boundaries still missing.

Why it matters: semantic claims are Loctree's differentiator, but they become a
trust liability when agents cannot tell whether a fact is file-level,
symbol-level, call-site-level, repo-verified, or a semantic guess.

Evidence/source artifact:

- `docs/plans/lsp/14-loctree-semantic-request.md` requires
  `scope: "file"|"symbol"|"project"` and authority-labeled runtime facts.
- `$HOME/.vibecrafted/artifacts/vetcoders/aicx/2026_0504/plans/2026_0504_234149_loctree_semantic_boundary_claude.md`
  asks for an audit of Loctree paths that treat AICX/memex semantic results as
  scoped truth; its referenced report file is absent.
- `docs/use-cases/30_agent_recon_loop.md` expects exports/re-exports and
  call-sites to be visible before removals.

Current repo evidence:

- `loctree-lsp/src/semantic.rs` explicitly returns
  `status: "symbol_scope_unimplemented"` for `scope = "symbol"`.
- Call-site data exists in scattered shapes (`command_calls`,
  `frontend_calls`, env occurrences, dispatch edges), but there is no unified
  `CallEntry`/call-site model shared by parser, semantic, LSP, MCP, and context
  pack surfaces.
- `loctree-rs/src/pack.rs` carries authority labels, but symbol/call-site
  granularity is not first-class enough for per-symbol AICX overlays.

Proposed single-run cut: publish the v1 semantic boundary in code/docs:
`file` and `project` are supported, `symbol` is not. Then add a narrow
`CallSite`/`CallEntry` data contract that can absorb Tauri invokes, env reads,
dispatch edges, and future AST-query results without pretending all of them are
repo-verified.

### 7. `loctree/follow` scope parity

Status: mostly delivered for LSP v1; `trace` remains an honest stub.

Why it matters: `follow` is supposed to be the one agent entry point for
structural smells. Stubs under the same advertised capability create false
confidence.

Evidence/source artifact:

- `docs/plans/lsp/15-loctree-follow-request.md` calls for `cycles`, `dead`,
  `twins`, `hotspots`, `trace`, `commands`, `events`, `pipelines`, and `all`.

Current repo evidence after Stage 2:

- `loctree-lsp/src/follow.rs` separates `SUPPORTED_SCOPES`,
  `IMPLEMENTED_SCOPES`, and `STUB_SCOPES`.
- `cycles`, `dead`, `twins`, `hotspots`, `commands`, `events`, `pipelines`,
  and `all` are implemented from snapshot data.
- `trace` is the only advertised stub and returns an explicit unsupported
  envelope instead of pretending to be live.

Proposed next cut: wire `trace` only after a library-backed trace path exists.
Do not shell out from LSP just to make the capability look complete.

### 8. MCP parity residuals from Track M

Status: partially delivered; M06 `context(format="markdown")` is now delivered.
Remaining items are lower than AST substrate but still product-visible.

Why it matters: MCP is one of the main agent surfaces. CLI/MCP drift makes
agents shell out or assume missing functionality.

Evidence/source artifact:

- `$HOME/.vibecrafted/inbox/Loctree/loctree-suite/plans/track-M-mcp-parity/TRACKER.md`
  marks M06-M09 still valid and M10 partial.
- M06 required `context(format="markdown")`; this is now present on the MCP
  `context` tool with JSON remaining the default.
- M07 requires MCP `manifests`, `dist`, and `insights`.
- M08 requires a first-class `query` tool, `component-of`, and guarded raw `jq`.
- M09 requires preview-first `prune_old_artifacts`.

Current repo evidence:

- `loctree-mcp/src/main.rs` now accepts `format: "json"|"markdown"` on
  `context`; markdown returns the curated Context pill in a JSON wrapper with
  receipt metadata. `context_next` and `context_section` remain JSON section
  surfaces.
- No MCP tools named `query`, `jq`, `manifests`, `dist`, `insights`, or
  `prune_old_artifacts` are registered.

Proposed next cut: land M07/M08 capability parity only where it can be backed by
existing library APIs. Keep M09 separate because deletion semantics need their
own confirmation gate.

### 9. Public `loctree-ast` / free-tier split

Status: missing, P0 in licensing/free-tier plans.

Why it matters: the paid/free product story depends on a real public analyzer
crate. If `loct` points users at `cargo install loctree-ast` and that package
does not exist, the first-user funnel breaks.

Evidence/source artifact:

- `$HOME/.vibecrafted/inbox/Loctree/loctree-suite/plans/track-L-licensing/L12-loctree-ast-extract.md`
  says `loctree-ast` is the free tier and is P0 alongside licensing scaffold.

Current repo evidence:

- Workspace has no `loctree-ast` member and no dependency on a published
  `loctree-ast` crate.
- Analyzer, semantic, context-pack, AICX, CLI, MCP, LSP, and report surfaces
  still live together in this suite repo.

Proposed single-run cut: do a boundary audit before any destructive
filter-repo work. If analyzer files still depend on `semantic`, `pack`, `aicx`,
or license code, split the dependency first; do not publish a public crate with
closed-source imports.

## Executive Thesis

Loctree should become the structural perception and context-compiler layer for
agentic software work.

It should not try to be a universal parser vendor, a generic semantic search
product, or a standalone memory system. Its strongest role is sharper:

> Loctree tells agents what the code is, how it is connected, what runtime
> surface it participates in, and what context is safe to act on now.

AICX is the complementary temporal/operator-memory layer:

> AICX tells agents why the work exists, what was decided, what failed before,
> what remains unresolved, and which retrieved facts are grounded in operator
> history rather than agent guesswork.

The product opportunity is the seam between them:

```text
Loctree = code perception, repo topology, runtime-oriented structural truth
AICX    = intent memory, failure memory, steering metadata, operator context
Repo    = final ground truth, verified by tests/runtime/builds
```

The next Loctree should make that triad operational.

## What Is Already Real

Loctree is not just an OXC wrapper, but OXC is a real sensor in the system.

Current code evidence:

- JS/TS parsing uses OXC in `loctree-rs/src/analyzer/ast_js/*`.
- Tree-sitter is now a real runtime dependency in two distinct places (this supersedes earlier
  "not a dependency / aspirational" notes): the `loctree-ast` crate parses JS/TS/TSX for the LSP
  live-AST layer, and `loctree-rs` (Wave B: `tree-sitter-swift/objc/c/cpp`, `analyzer/swift.rs`,
  `semantic/c_family.rs`) performs C-family Layer 1 symbol extraction. It remains a sensor feeding
  the snapshot, not a replacement for it.
- Rust, Python, Shell, Makefile, Go, Dart, Zig, CSS, HTML, Tauri, dist/source maps,
  events, commands, pipelines, test coverage, twins, cycles, dead exports, and reports
  are Loctree-owned analyzer/report surfaces.
- `loctree-rs/src/types.rs` is the shared cross-language fact model.
- `loctree-rs/src/snapshot.rs` owns scan-once/query-many persistence.
- `loctree-rs/src/analyzer/for_ai.rs` owns the agent bundle surface.
- `loctree-mcp/src/main.rs` exposes agent-native query operations.
- `reports/src` is already a Leptos report/cockpit surface.

Fresh local source-run evidence:

```text
cargo run -q -p loctree --bin loct -- --version
=> loct 0.9.0

cargo run -q -p loctree --bin loct -- --fresh --quiet findings --summary
=> 230 files, 106128 LOC, health_score 88, dead_parrots 2,
   duplicate_groups 65, cycles 0
```

The installed global `loct` was `0.8.16`, so release/runtime strategy must keep
distinguishing installed binary truth from source checkout truth.

## What The Conversation Changed

The strategic conversation reframed the product from "static analyzer" to
"agentic perception layer".

The most important points:

- Current retrieval tools mostly map "what exists now"; agents also need "why it
  exists", "what failed before", "what is forbidden", and "what the operator is
  actually trying to resolve".
- AICX's strongest principle is constitutional: canonical corpus first,
  derived indexes second. Corrupt indexes should be rebuilt, not mourned.
- Boring foundations beat sexy features until they are solid: deterministic
  identity, replayable writes, integrity checks, observable progress, rebuildable
  derived views.
- Runtime truth has higher authority than structural elegance. A function's
  actual role in execution matters more than whether a parser can name it.
- Agents need first-class failure retrieval and intent retrieval before they can
  reliably stop repeating old mistakes.

The Loctree implication:

Loctree must stop treating language support as "recognized names" and start
treating it as runtime semantics. Shell and Makefile are the warning shot.

## Strategic Doctrine

### 1. Parser Is A Sensor, Not The Product

OXC, regexes, future Tree-sitter grammars, source maps, coverage files, git,
and runtime traces are sensors.

The product is the normalized, weighted, queryable context Loctree builds from
those sensors.

This matters because adding Tree-sitter alone would not fix false dead exports
in Shell or Makefile. Tree-sitter can say "this is a function definition"; it
does not automatically know that:

- `case "$cmd" in deploy) deploy_impl ;; esac` is runtime dispatch
- `.PHONY` is Make metadata
- `usage`, `die`, `main`, `PATH`, and `cleanup` are CLI/runtime idioms
- sourced shell files can be library APIs

Loctree should add parsers only when they feed a stronger semantic model.

### 2. Ground Truth And Derived Views Must Be Explicit

Loctree has its own version of the AICX problem.

AICX:

- ground truth: `$HOME/.aicx/store/` canonical chunks and sidecars
- derived views: steer LanceDB, BM25, memex semantic indexes

Loctree:

- ground truth: the live repo checkout plus repo-local config/suppressions
- derived views: snapshots, findings, reports, agent bundles, graph layouts,
  cache indexes

Every Loctree artifact should answer:

- Am I source truth or derived?
- Can I be rebuilt from source truth?
- How does the user know I am stale, corrupt, or scope-mismatched?
- What command repairs me?

### 3. The Agent Pack Is The Core UX

The highest-leverage feature is not another report tab. It is an agent-ready
context pack.

Target command:

```bash
loct context --task "fix shell false dead exports"
loct context --file loctree-rs/src/analyzer/shell.rs
loct context --changed
loct context --with-aicx --project Loctree/loctree-suite
```

Target MCP tool:

```text
context(project, task?, file?, changed?, include_intents?, include_failures?)
```

The output should include:

- structural slice: files, symbols, imports, consumers, entrypoints
- runtime slice: commands, events, pipelines, framework-specific semantics
- risk slice: hotspots, high-fan-in files, stale snapshot status, cache scope
- action slice: next safest commands, verification gates, likely tests
- memory slice: AICX decisions, unresolved intents, failure history, source chunks
- authority slice: confidence, source type, recency, operator confirmation where known

This turns Loctree from "queryable analyzer" into "pre-edit operating context".

## Alignment With AICX

AICX already implements much of the memory-side philosophy Loctree should align
with:

- `src/store.rs`: canonical `$HOME/.aicx` store layout, repo buckets, sidecars,
  dedup, watermarks
- `src/doctor.rs`: integrity checks and safe rebuild of derived steer indexes
- `src/intents.rs`: extracted decisions/intents/outcomes/tasks with source chunks
- `src/steer_index.rs`: metadata-aware retrieval over sidecars
- `src/mcp.rs`: `aicx_search`, `aicx_rank`, `aicx_steer`, `aicx_intents`
- `src/dashboard_server.rs`: fuzzy, semantic, cross, steer, health, status,
  regeneration endpoints

The integration should be clear and bounded:

Loctree should not absorb AICX.

AICX should not become a code graph analyzer.

Instead:

- Loctree snapshots should include enough stable project identity to query AICX
  for matching intent/failure context.
- AICX chunks/sidecars should be able to store the Loctree snapshot ID, schema,
  repo identity, branch, commit, and run ID used during an agent run.
- `loct context --with-aicx` should ask AICX for decisions, unresolved intents,
  and failure history related to the current repo/task/files.
- AICX `intents` should be able to feed a Loctree "planned vs landed" report.
- Both products should share a repo identity contract rather than inventing
  separate heuristics for `Org/Repo`, local path, canonical root, and source tier.

Important live drift:

AICX docs currently describe `aicx memex-sync`, but the inspected `src/main.rs`
does not expose a matching command and there is no live `src/memex.rs`. Current
memex usage is present through steer-index storage and dashboard shell-out to
`rust-memex`/`rmcp-memex`. This should be resolved before Loctree treats
`memex-sync` as a stable integration point.

## Product Lanes

### Lane 0: Reliability Constitution

Goal: make Loctree artifacts trustworthy before adding more magic.

Build:

- `loct doctor --cache --snapshots --scope`
- stale snapshot explanation with exact expected/current roots
- cache list that speaks in human project identity, not only hashed buckets
- explicit derived-view language in docs/help
- rebuild commands for every derived artifact
- progress phases for scans, report builds, and expensive focus/context flows

Definition of done:

- operator never has to guess whether Loctree is reading the right repo
- stale/corrupt/scope-mismatched artifacts fail loudly or self-heal safely
- no root-level doc teaches users to trust cache state blindly

### Lane 1: Agent Context Pack

Goal: one command gives agents enough context to edit safely.

Build:

- `loct context`
- MCP `context`
- task/file/changed modes
- `--with-aicx` overlay
- JSON and Markdown output
- source_chunk backrefs for memory-derived claims
- confidence/authority labels

Definition of done:

- an agent can start from `loct context --changed --with-aicx` and produce a
  better plan than from `rg` plus reading random files
- context output says what it does not know
- AICX facts are never presented as repo truth unless verified against repo/runtime

### Lane 2: Runtime Semantics Per Language

Goal: reduce elegant false positives.

Priority runtimes:

- Shell: functions, variables, source edges, traps, top-level execution,
  dispatch by `case`, indirect handlers, CLI idioms
- Makefile: targets, dependencies, recipes, variables, `.PHONY`, default target,
  private/internal targets
- Tauri: command registration, invoke sites, events, plugin commands, FE/BE gaps
- CLI apps: subcommands, handler dispatch, config/env contracts, shell wrappers
- JS/TS frameworks: route loaders, Svelte/Vue templates, dynamic imports, build output

Definition of done:

- Shell-rich repos do not produce high-confidence deletion advice from CLI idioms
- Make targets are not treated as dead exports
- duplicate reports distinguish duplicate logic from idiom/env-contract repetition

### Lane 3: Intent And Failure Overlay

Goal: make "why" and "what failed before" visible at the code surface.

Build:

- `loct context --with-intents`
- `loct context --with-failures`
- report panel: "Related decisions and unresolved intents"
- report panel: "Prior failed attempts"
- planned-vs-landed report: AICX intents checked against Loctree snapshot
- source_chunk and Loctree evidence side-by-side

Definition of done:

- a repeated Monika/operator concern can surface before another agent repeats
  the same failed fix
- every memory claim links back to an AICX source chunk
- every landed/not-landed claim links to Loctree/repo evidence

### Lane 4: Browser Cockpit

Goal: make Loctree usable as a product surface, not only a terminal artifact.

Use the existing Leptos report as the foundation, not a parallel UI.

Build:

- context-pack view
- health/doctor view
- findings triage view
- AICX decisions/failures side panel
- changed-files mode for PR/review workflows
- "copy agent prompt" and "open file" affordances
- local-only mode first; no server dependency required for basic report use

Definition of done:

- an operator can open one report and understand what to do next
- an agent can consume the same underlying JSON through CLI/MCP
- UI is a cockpit over real artifacts, not a decorative duplicate

### Lane 5: Semantic Code Retrieval, But After The Foundation

Goal: add semantic search over code spans without weakening deterministic truth.

Build later:

- function/class/span extraction IDs
- embeddings over code spans plus signatures/callers/callees
- reranker for "where should I edit?" task descriptions
- similarity search for related implementations
- semantic duplicate suggestions with structural confirmation

Guardrail:

Embeddings may find candidates. They must not decide identity, dedupe, integrity,
or deletion confidence. Deterministic structure stays authoritative for those.

## Data Contracts To Converge

### Project Identity

Loctree and AICX should share a practical identity model:

```text
org_repo:        Loctree/loctree-suite
canonical_root:  absolute canonical checkout path
git_remote:      normalized origin/upstream URL if present
branch:          current branch
commit:          HEAD when available
source_tier:     primary | secondary | weak | non_repository
snapshot_id:     stable Loctree artifact ID
run_id:          AICX/Vibecrafted run ID when relevant
```

This is not bureaucracy. It prevents cross-repo cache lies.

### Fact Authority

Every cross-layer claim should carry authority:

```text
repo_verified        live code/build/test evidence
loctree_derived      derived from snapshot/finding/context pack
aicx_operator        explicit operator statement or decision
aicx_agent           agent observation, low authority
aicx_failure         prior failed attempt
semantic_guess       embedding/reranker suggestion
stale_or_unknown     useful but not current enough to trust
```

### Rebuildability

All derived artifacts should expose:

```text
source_truth
schema_version
created_at
project_identity
scope_roots
input_hashes_or_snapshot_refs
rebuild_command
doctor_check_name
```

## Near-Term Build Plan

### Cut 1: Document The Runtime Truth And Fix Docs Drift

Scope:

- mark Tree-sitter references as aspirational/stale unless implementation lands
- document OXC as JS/TS parser sensor
- document Loctree-owned semantic layers separately from parser dependencies
- make cache/snapshot derived-view language consistent
- add AICX integration caveat around live `memex-sync` drift

Gate:

```bash
make precheck
```

### Cut 2: `loct doctor --cache --scope`

Scope:

- cache base and project cache identity inspection
- snapshot schema and root-scope validation
- stale/scope-mismatch explanation
- `--json` output for agents
- no destructive fixes without explicit `--fix`

Gate:

```bash
cargo test -p loctree cache scope snapshot doctor
make precheck
```

### Cut 3: Shell And Make Runtime Semantics

Scope:

- harden `loctree-rs/src/analyzer/shell.rs`
- harden `loctree-rs/src/analyzer/makefile.rs`
- dead/twins classification aware of shell and Makefile symbol kinds
- fixture based on the current shell false-positive report

Gate:

```bash
cargo test -p loctree shell makefile dead_parrots twins
cargo run -q -p loctree --bin loct -- --fresh findings --summary
```

Acceptance:

- shell functions called in-file are not high-confidence dead
- sourced library APIs are not treated as ordinary dead exports
- Make targets and `.PHONY` are not dead exports
- `usage`, `die`, `main`, `cleanup`, `info`, `warn`, `PATH` classify as idiom/env
  unless there is same-file duplicate-definition evidence

### Cut 4: `loct context` MVP

Scope:

- `loct context --file`
- `loct context --changed`
- Markdown + JSON
- no AICX dependency yet
- includes verification commands and confidence labels

Gate:

```bash
cargo test -p loctree context for_ai slice impact
cargo run -q -p loctree --bin loct -- context --changed --json
```

### Cut 5: AICX Overlay

Scope:

- optional `--with-aicx`
- shell-out to `aicx intents` / `aicx steer` first, MCP later
- source_chunk backrefs
- explicit "memory-derived, not repo-verified" labels
- graceful fallback when AICX is missing or stale

Gate:

```bash
loct context --changed --with-aicx --json
aicx intents -p loctree-suite --emit json
```

## Non-Goals

- Do not rewrite Loctree around Tree-sitter by default.
- Do not merge AICX into Loctree.
- Do not let embeddings decide deletion or identity.
- Do not build a second browser UI separate from the Leptos report unless the
  current report architecture blocks product quality.
- Do not preserve docs that describe features the live binary cannot run.

## Strategic Risks

1. Analyzer sprawl without semantic contracts.

   If every language adds local exceptions but no runtime model, findings will
   keep looking precise while being operationally wrong.

2. Cache trust erosion.

   If agents ever see stale artifacts as current truth, Loctree loses its core
   promise. Doctor/scope/cache work is not optional plumbing.

3. Memory overreach.

   If Loctree starts owning AICX's corpus, it will become muddy. Loctree should
   request memory overlays and cite them, not own the memory store.

4. Semantic search glamour.

   Code embeddings are useful, but only after deterministic identity and
   rebuildability are boringly reliable.

5. Product surface fragmentation.

   CLI, MCP, JSON, and Leptos report must expose the same truth, not four
   parallel interpretations.

## North Star

The next Loctree should let an agent ask:

```text
What is the safest, most informed way to act on this repo right now?
```

And receive:

- the structural map
- the runtime semantics
- the likely blast radius
- the current artifact health
- the relevant decisions
- the unresolved intentions
- the prior failures
- the exact verification path

That is the leverage.

Loctree should become the code perception layer that keeps agents from coding
blind. AICX should remain the memory layer that keeps agents from repeating the
team's past mistakes. The product is strongest when both are boringly truthful
and sharply connected.
