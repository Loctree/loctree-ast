---
name: missing-lsp-03-08-11-13-14-single-run
status: planning
project: loctree-suite
created: 2026-05-07
scope: docs-only handoff for queued LSP plans
---

# Missing LSP Plans 03, 08, 11, 13, 14 — Single-Run Handoff

This handoff covers the five tracker rows that remain queued in
`docs/plans/lsp/TRACKER.md`: 03, 08, 11, 13, and 14. It is intentionally
implementation-shaped: the next worker should be able to open this file,
touch the listed modules, and run one coherent pass instead of re-planning
the roadmap.

## Current State

Plans 08, 11, 13, and 14 are not wired in `loctree-lsp` yet: there are no
`aicx.rs`, `diff.rs`, `workspaces.rs`, or `semantic.rs` handler modules, no
custom-method registrations, and no integration tests for those requests.

Plan 03 is different: `loctree-lsp/src/code_lens.rs`,
`loctree-lsp/tests/code_lens_request.rs`, backend capability advertising,
and the `LanguageServer::code_lens` handler already exist. The tracker still
marks the plan as queued, and the implementation is not fully contract-clean:
the original plan asked for no CodeLens command in v1, while the current code
emits `command: Some(Command { title, command: "" })`. Treat 03 as a contract
closure/reporting cut, not as a greenfield feature.

## What Each Plan Does

### 03 — CodeLens Importers

Adds passive inline IDE annotations on top-level exports: `"unused (0 importers)"`,
`"1 importer"`, or `"N importers"`. The useful runtime value is human-visible
structural signal inside editors; agent value is secondary.

Current implementation exists, but the single-run should decide and document
the v1 contract: either keep the current empty-command title carrier if the LSP
client needs it to render, or change it to the original plan's `command: None`
and prove rendering still works. Then update the tracker/report in the actual
implementation branch.

### 08 — `loctree/aicx`

Adds a read-only custom LSP request for AICX memory continuity. A fresh
LSP-connected agent can ask for prior decisions/intents scoped to a file,
symbol, or project and receive ranked entries plus source chunks.

This should reuse the existing AICX machinery from `loctree-rs`: `AicxClient`,
`is_aicx_available`, `ScopeKeywords`, `score_intent`, `authority_for_intent`,
and the context composer patterns around `compose_memory_slice`.

### 11 — `loctree/diff`

Adds a custom LSP request for session-local structural deltas: files added,
removed, changed; import edges added/removed; symbols added/removed; and a
marker describing the baseline. It depends on Plan 10's watcher/snapshot
refresh work because the daemon must remember a previous snapshot.

This should reuse `loctree-rs/src/diff.rs::SnapshotDiff` for snapshot-vs-
snapshot comparisons, while adding LSP-side session markers such as
`lastQuery`, `lastScan`, and `epoch`.

### 13 — Multi-Workspace Context

Makes one LSP daemon serve a monorepo with multiple `.loctree/` roots. The
daemon discovers subprojects at initialization, keeps a per-project
`SnapshotState`, routes every request with `project: Option<PathBuf>` to the
right snapshot, and exposes `loctree/workspaces` for enumeration.

This is the cross-cutting cut. It should preserve single-workspace behavior
while changing the backend's snapshot ownership model from one `SnapshotState`
to a routed workspace map.

### 14 — `loctree/semantic`

Adds a custom LSP request for Loctree's meaning layer: idiom tags, dispatch
edges, reachability, env contracts, Tauri commands/events, and framework
hints. This is the strongest AI-engine differentiator in the missing set.

This should reuse the context runtime composer around
`compose_runtime_slice`, `RuntimeSlice`, and authority labels rather than
recreating semantic extraction in the LSP crate.

## Dependencies And Order

1. Close 03 first because it is already mostly implemented and should not
   collide with the broader request-router work.
2. Implement 13 next. Multi-workspace routing changes the shape of `Backend`;
   landing it before new request modules avoids writing four handlers against
   the old single-snapshot path and then adapting them.
3. Implement 11 after 13 and on top of Plan 10's watcher. It needs current and
   previous snapshots per routed workspace, so it should follow the workspace
   map design.
4. Implement 14 after 13. It benefits directly from routed snapshots and can
   reuse `project` resolution immediately.
5. Implement 08 after 13 and preferably after 14. AICX scope keywords become
   better when structural context and runtime semantic facts are both available.

Single-run dependency shape:

```text
03 closure
  -> 13 workspace router
      -> 11 diff
      -> 14 semantic
          -> 08 aicx
```

## Single-Run Implementation Plan

### 0. Baseline Guardrails

- Re-read `docs/plans/lsp/TRACKER.md` and the five plan files immediately
  before edits.
- Check `git status --short` and do not overwrite concurrent LSP changes.
- Do not update unrelated roadmap files until the implementation branch is
  ready to claim tracker rows.

### 1. Plan 03 Contract Closure

Touch:

- `loctree-lsp/src/code_lens.rs`
- `loctree-lsp/tests/code_lens_request.rs`
- `loctree-lsp/src/backend.rs`

Actions:

- Decide whether v1 CodeLens is truly passive (`command: None`) or whether
  the current empty command is required for title rendering in target clients.
- Make the code and tests match that decision.
- Keep `code_lens_provider.resolve_provider = Some(false)`.
- Add or tighten an integration-style test around emitted lenses if feasible
  without needing a live VS Code smoke.

### 2. Plan 13 Workspace Router Foundation

Touch:

- `loctree-lsp/src/workspaces.rs` (new)
- `loctree-lsp/src/backend.rs`
- `loctree-lsp/src/lib.rs`
- `loctree-lsp/tests/multi_workspace.rs` (new)
- Existing request modules with `project: Option<PathBuf>`:
  `context_atlas.rs`, `slice.rs`, `impact.rs`, `find.rs`, `health.rs`,
  `follow.rs`

Actions:

- Add discovery for `.loctree/` directories under the initialized workspace
  root, depth-limited by initialization options with default depth 4.
- Add a `WorkspaceSnapshots` or equivalent owner around
  `HashMap<PathBuf, SnapshotState>` keyed by canonical project root.
- Keep the current `snapshot: SnapshotState` path only if it becomes a
  compatibility facade over the root workspace; avoid two divergent sources of
  snapshot truth.
- Add `Backend::route_snapshot(project: Option<PathBuf>)` and
  `Backend::route_root(project: Option<PathBuf>)`.
- Wire `loctree/workspaces` custom request in `loctree-lsp/src/lib.rs`.
- Advertise `experimental.loctree/workspaces`.
- Update existing handlers to use routed snapshot/root instead of the single
  `self.snapshot` and `self.workspace_root` where their params carry
  `project`.

### 3. Plan 11 Diff Request

Touch:

- `loctree-lsp/src/diff.rs` (new)
- `loctree-lsp/src/backend.rs`
- `loctree-lsp/src/lib.rs`
- `loctree-lsp/src/snapshot.rs` or the new workspace snapshot owner
- `loctree-lsp/src/watcher.rs` only if the watcher must expose previous-scan
  markers cleanly
- `loctree-lsp/tests/diff_request.rs` (new)
- `loctree-rs/src/diff.rs` only if a reusable public helper is missing

Actions:

- Add request params: `since`, `project`, and optional cursor/chunk fields if
  the response can be large.
- Store per-workspace previous snapshot after each successful watcher reload.
- Store per-workspace/per-session `lastQuery` marker and advance it only after
  a successful `loctree/diff` response.
- For `epoch`, return a full-from-current response.
- For `lastScan`, diff previous successful watcher snapshot against current.
- For `lastQuery`, diff the stored query baseline against current.
- For git revs, delegate to the existing snapshot diff flow where possible;
  if that cannot be made clean in the first pass, return a typed
  `unsupported_since` error rather than silently faking it.
- Reuse Plan 12 cursor pagination for large edge/symbol arrays if needed.
- Advertise `experimental.loctree/diff`.

### 4. Plan 14 Semantic Request

Touch:

- `loctree-lsp/src/semantic.rs` (new)
- `loctree-lsp/src/backend.rs`
- `loctree-lsp/src/lib.rs`
- `loctree-lsp/tests/semantic_request.rs` (new)
- `loctree-rs/src/cli/dispatch/handlers/context/mod.rs` only if public exports
  are missing for `ContextOptions`, `RuntimeSlice`, or `compose_runtime_slice`

Actions:

- Add params: `scope`, `target`, `kinds`, `project`, plus optional cursor
  fields if response arrays can be large.
- Route to the correct workspace snapshot through Plan 13.
- For `scope=file`, call the existing runtime-slice composer with
  `ContextOptions { file: Some(target), project, .. }`.
- For `scope=symbol`, derive file/symbol targeting from snapshot facts where
  possible; otherwise return a typed `symbol_scope_unimplemented` with a hint
  to use file scope in v1.
- For `scope=project`, either aggregate bounded facts or return a paginated
  response. Do not emit huge inline JSON without Plan 12 protection.
- Filter by `kinds`: `idiom_tags`, `dispatch_edges`, `reachability`,
  `env_contracts`, `tauri_commands`, `tauri_events`, `framework_hints`.
- Preserve `AuthorityLabel` on every fact.
- Advertise `experimental.loctree/semantic`.

### 5. Plan 08 AICX Request

Touch:

- `loctree-lsp/src/aicx.rs` (new)
- `loctree-lsp/src/backend.rs`
- `loctree-lsp/src/lib.rs`
- `loctree-lsp/tests/aicx_request.rs` (new)
- `loctree-rs/src/aicx/mod.rs` and
  `loctree-rs/src/cli/dispatch/handlers/context/mod.rs` only if the LSP crate
  needs public helper exports instead of duplicating logic

Actions:

- Add params: `scope`, `target`, `kinds`, `hours`, `limit`, `project`.
- Route to the correct workspace root/snapshot through Plan 13.
- Build scope keywords from the target path/symbol plus available structural
  and semantic facts.
- Reuse `AicxClient::new(...)`, `client.intents(...)`, `score_intent(...)`,
  `authority_for_intent(...)`, and source-chunk collection semantics from
  `compose_memory_slice`.
- If AICX is unavailable, return a normal typed response with
  `status: "aicx_unavailable"` and a hint mentioning `aicx` or
  `LOCT_AICX_BINARY`; do not fail the whole request as an LSP server error.
- Keep this read-only. No AICX write path in this run.
- Advertise `experimental.loctree/aicx`.

### 6. Tracker And Reports After Implementation

Only after the implementation branch is real:

- Update the five tracker rows from `queued` to `done` or `failed`.
- Write reports to the tracker-listed report paths:
  `reports/lsp/03-codelens-importers.md`,
  `reports/lsp/08-loctree-aicx-request.md`,
  `reports/lsp/11-loctree-diff-request.md`,
  `reports/lsp/13-multi-workspace-context.md`,
  `reports/lsp/14-loctree-semantic-request.md`.
- Include deviations from the original plans, especially the 03 CodeLens
  command/title contract and any staged limitation in `symbol` or `project`
  semantic scope.

## Final Gates To Run After Implementation

Run these after code implementation, not during this docs-only handoff:

```bash
cargo fmt --all --check
cargo clippy -p loctree-lsp --all-targets -- -D warnings
cargo test -p loctree-lsp code_lens
cargo test -p loctree-lsp multi_workspace
cargo test -p loctree-lsp diff_request
cargo test -p loctree-lsp semantic_request
cargo test -p loctree-lsp aicx_request
make precheck
```

Manual/LSP smoke after automated gates:

```bash
printf '%s\n' '{"jsonrpc":"2.0","id":1,"method":"loctree/workspaces","params":{}}' | loctree-lsp
printf '%s\n' '{"jsonrpc":"2.0","id":2,"method":"loctree/semantic","params":{"scope":"file","target":"loctree-lsp/src/backend.rs"}}' | loctree-lsp
printf '%s\n' '{"jsonrpc":"2.0","id":3,"method":"loctree/diff","params":{"since":"epoch"}}' | loctree-lsp
printf '%s\n' '{"jsonrpc":"2.0","id":4,"method":"loctree/aicx","params":{"scope":"project","limit":5}}' | loctree-lsp
```

If the implementation changes large response shapes, add a cursor/pagination
smoke using `loctree.protocol.defaultChunkSize` from initialization options.

## Sources Used

- `docs/plans/lsp/TRACKER.md`
- `docs/plans/lsp/00-roadmap-readme.md`
- `docs/plans/lsp/03-codelens-importers.md`
- `docs/plans/lsp/08-loctree-aicx-request.md`
- `docs/plans/lsp/11-loctree-diff-request.md`
- `docs/plans/lsp/13-multi-workspace-context.md`
- `docs/plans/lsp/14-loctree-semantic-request.md`
- `loctree-lsp/src/backend.rs`
- `loctree-lsp/src/lib.rs`
- `loctree-lsp/src/code_lens.rs`
- `loctree-lsp/tests/code_lens_request.rs`
- `loctree-lsp/src/snapshot.rs`
- `loctree-lsp/src/protocol.rs`
- `loctree-lsp/src/context_atlas.rs`
- `loctree-lsp/src/slice.rs`
- `loctree-lsp/src/find.rs`
- `loctree-rs/src/diff.rs`
- `loctree-rs/src/cli/dispatch/handlers/context/mod.rs`
