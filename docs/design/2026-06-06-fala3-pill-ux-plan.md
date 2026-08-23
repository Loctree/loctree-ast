# Fala 3 — Context Pill UX, implementation plan (VS Code)

Built on the now-CLEAN pill surface (hover/refs/def cut, Phase 1 landed). Operator-
approved designs (parallel design + adversarial critique, 2026-06-06). Sequential
subagent-driven build — every task touches `viewModel.ts` + `media/contextPill.js`,
so NO parallel editing.

Verified real LSP shapes (trust these): `slice→{core:[{path,loc}], deps:{data:[{path}]},
consumers:{data:[{path}]}}`, `impact→{total, direct:[{path}]}`, `health→{health_score,
status, dead_exports, cycles, twins, hotspots, top_risks:[{kind,file,severity,message}]}`
(REPO-level), `body→{bodies:[{symbol,file,start_line,end_line,source,truncated}]}`,
`bodyRanked→{found,file,preview,truncated,candidates}`, `find mode:literal →
{literal_matches:{occurrences:[{file,line,column,...}], total, page}}`. `isSnapshotNotLoaded(err)`
(gateway.ts) detects the transient -32001 "snapshot not loaded yet".

No per-file source for exports or a per-file summary (they degrade). All copy EN, no emoji.

## Task order (sequential)
1. VM state machine + out-of-workspace / not-in-snapshot empty-state cards
2. Header badges: file LOC + blast (scope-gated); repo health demoted
3. Per-file FINDINGS (filter top_risks by file) + repo-wide background line
4. Hide-when-empty for summary / exports + blast-band-zero copy
5. Scope-switcher symbol resolution via `body.bodies` (one / many / zero→literal)
6. Literal-scope pill (occurrences grouped + Load More)

---

## Task 1 — VM state machine + out-of-workspace / not-in-snapshot
**Files:** `scope.ts`, `viewModel.ts`, `panel.ts`, `media/contextPill.js`, `media/contextPill.css`; test `test/contextPillViewModel.test.ts`.

- `scope.ts`: add `| { kind: 'out-of-workspace'; value: string }` to `Scope`. `parseScopeInput` NEVER produces it (set only by ambient binding).
- `viewModel.ts`: add `state: 'ready' | 'out-of-workspace' | 'not-in-snapshot'` to `ContextPillViewModel` (default `'ready'`).
  - `out-of-workspace` scope → early-return minimal VM `{scope, file: value, state:'out-of-workspace', ...empty}` with **ZERO gateway calls** (no health → no leaked repo numbers).
  - `file` scope: after slice+impact, if `slice.core` empty AND `impact.total` falsy AND `impact.direct` empty → `state:'not-in-snapshot'` — UNLESS the slice/impact rejection was `isSnapshotNotLoaded` (-32001), in which case keep `state:'ready'` (snapshot is warming, not missing). `safe()` must let the adapter see a -32001 vs a clean empty (e.g. catch and check `isSnapshotNotLoaded` before falling back).
- `panel.ts` `rescopeToActiveEditor`: `const folder = vscode.workspace.getWorkspaceFolder(editor.document.uri)`. If `undefined` (or scheme !== 'file') → `rescope({kind:'out-of-workspace', value: editor.document.uri.fsPath})`. Else file scope via `asRelativePath`.
- `media/contextPill.js`: `render(vm)` switches on `vm.state` at the top:
  - `out-of-workspace`: heading **"Outside the indexed workspace"**, body **"This file is not part of any open Loctree project, so there is no structural context for it. Open a file from your project to see its blast radius, dependencies, and findings."**, a muted middle-ellipsized absolute-path line (omit if scheme≠file), button **"Open a project file"**. The `Inspect symbol or file…` switcher STAYS (escape hatch). NO badges, NO CTA, NO sections.
  - `not-in-snapshot`: heading **"Not in the current snapshot"**, body **"Loctree has not indexed this file yet. Scan the workspace to include it, then reopen this file for its context."**, button **"Scan this workspace"** (posts a message → `loctree.refresh`/`initialize`). Switcher stays. No badges/CTA/sections.
  - `ready`: the normal pill.
- Message allowlist (panel.ts): add `openProjectFile` and `scanWorkspace` (→ focus explorer / run `loctree.initialize`).
- **Test:** out-of-workspace scope → state, zero gateway calls (assert with a spy gateway); file scope with empty slice+impact → `not-in-snapshot`; populated → `ready`.

## Task 2 — Header: file LOC + blast (scope-gated), repo health demoted
**Files:** `viewModel.ts`, `media/contextPill.js`, `media/contextPill.css`; test.
- `viewModel.ts`: add `fileLoc: number | null` ← `slice.core?.[0]?.loc ?? null` (file scope only; null for symbol/literal). Keep `health.score` but expose it as `repoHealth: number` for the demoted line.
- `media/contextPill.js` header (`pill-head`): badges row = `[NNN LOC]` (only when `vm.fileLoc != null`) + `[blast radius N]` (only when `scope.kind==='file'`). REMOVE the `health NN` badge. Add a muted line lower (near Snapshot line): **"Repo health: NN/100"** from `repoHealth`.
- Target header reads e.g. `dispatch.py · 248 LOC · blast radius 7` / `Snapshot: fresh · Agent pack: ready` / `Repo health: 92/100`.
- **Test:** `fileLoc` from `slice.core[0].loc`; null for symbol scope; blast badge gated on scope.kind.

## Task 3 — Per-file FINDINGS + repo-wide background
**Files:** `viewModel.ts`, `media/contextPill.js`; test.
- `viewModel.ts`: `findings` becomes per-file: filter `health.top_risks` where `r.file === <active file>` (for file scope) or `=== bodyPreview.file` / target (symbol scope) → `fileRisks: [{kind, severity, message}]`. Keep repo totals as `repoFindings: {hotspots, dead, cycles, twins}`.
- `media/contextPill.js` FINDINGS section: if `fileRisks.length` → list up to 5 rows `"{Kind} — {message}"` (Kind humanized: cycle→Cycle, dead_export→Dead export, twin→Twin, hotspot→Hotspot) with a severity dot from the risk's `severity`; else → **"No issues in this file"**. Always a muted background line: **"Repo-wide: N hotspots · M dead · K cycles"** from `repoFindings`. Section label **"FINDINGS · THIS FILE"**.
- **Test:** top_risks filtered by file; clean file → empty fileRisks; repo totals preserved.

## Task 4 — Hide-when-empty (summary / exports) + blast-band-zero
**Files:** `media/contextPill.js`, `media/contextPill.css`.
- Hide **WHAT IT DOES** block entirely when `vm.summary === ''`. Hide **DEFINES / EXPORTS** block entirely when `vm.exports.length === 0` (keep the adapter field for a future per-file-symbols source). Keep **THIS FILE USES** (deps usually present; if empty, hide too).
- Blast band when `vm.blastRadius.count === 0`: render header + **"No files depend on this yet — safe to change."** and NO empty tag `<div>`.
- (Adapter unchanged — purely render-side honesty.)

## Task 5 — Scope-switcher symbol resolution via `body.bodies`
**Files:** `viewModel.ts` (or a small `resolveSymbol` seam), `panel.ts`, `media/contextPill.js`; test.
- Resolution data source is `gw.body(symbol)` → `bodies: [{symbol, file, start_line, ...}]` (NOT `find` — find returns context strings, no clean identity; verified).
- Flow: user types in `Inspect symbol or file…` → `parseScopeInput`. For `symbol`: resolve `body(symbol)`:
  - **1 body** → rescope to `symbol` scope (bodyPreview from it).
  - **many bodies** → post `{type:'candidates', items:[{file,line,symbol}]}`; client renders an INLINE candidate list (file:line + symbol) in the pill (not a native quick-pick); click → `rescopeCandidate` (symbol scope pinned to that file).
  - **0 bodies** → probe `find(query, mode:literal)` count; render **"No symbol found — N literal occurrences. Show as a pill?"** → click → `scope=literal` (Task 6).
- panel.ts: handle the symbol-resolution branch + the new messages (`rescopeCandidate`, `showAsLiteral`) in the allowlist.
- **Test:** resolution returns 'one'/'many'/'zero' from a fake body response; many → candidate list shape.

## Task 6 — Literal-scope pill
**Files:** `viewModel.ts`, `panel.ts`, `media/contextPill.js`; test.
- `scope=literal` assembly: `gw.find(query, {mode:'literal'})` → `literal_matches.occurrences` grouped by file → `{file, lines:[line...]}[]` + `total`; `state:'ready'`.
- Render a LITERAL pill: header `"<query>" · N occurrences`; HIDE blast band / DEFINES / THIS FILE USES / body (not applicable to a literal); show the grouped occurrence list (file → lines, click → navigateToFile at line); **Load More** when `literal_matches.page.has_more` (page via `offset`); same `Copy Agent Context` CTA (the markdown carries scope=literal + occurrences).
- panel.ts: literal scope + a `loadMoreLiteral` message (append next page).
- **Test:** literal scope assembles grouped occurrences from a fake find response; load-more appends without dup.

## Gate (each task) + final
Per task: pure-module unit tests + `tsc --noEmit` + `lint` + `node --check` (for contextPill.js). Final: full `npm test` + repackage + **live VSIX reality check** (open an in-workspace file → LOC+blast+per-file findings; open a stdlib file → out-of-workspace card; type a symbol that exists in 2 files → candidate list; type a non-symbol → "show as a pill?"). Then ultracode end-to-end review.
