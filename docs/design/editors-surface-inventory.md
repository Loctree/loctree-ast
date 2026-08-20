# Loctree editor surfaces — factual inventory (VS Code + loctree-lsp)

> **STATUS (2026-06-06): PRE-Phase-1 SNAPSHOT — SUPERSEDED for the cut surfaces.**
> This document captures the surface state BEFORE the Phase 1 surface cuts landed
> (commits `797d1fb6` and `bb6197b9`). It is retained as a historical pre-decision
> snapshot; do NOT treat the sections below as current truth. Concretely, since this
> was written:
> - `hover_provider`, `definition_provider`, and `references_provider` are now `None`
>   in `backend.rs` `server_capabilities`. Sections **A.1**, **A.2**, and **B.6**
>   ("TWO loctree hovers") no longer hold.
> - The client `hover.ts` and the dead `contextPanel.ts` modules were DELETED.
>   Sections **B.1**, **B.2**, and **C.7** (about `contextPanel`) are historical now.
> - `code_action_provider` remains `Some(...)` — the only real navigation contribution.
>
> CURRENT keep/cut ruling: `docs/design/editors-surface-policy.md`.
> LIVE truth for capabilities: `loctree-lsp/src/backend.rs` `server_capabilities`.

State as of 2026-06-06 (branch feat/jetbrains-plugin), AFTER the Context Pill landed.
Pure inventory — what exists and where. No "keep/cut" decisions here; that is the
follow-up. Source of truth, read directly (not summaries):
`editors/vscode/src/**`, `editors/vscode/package.json`, `loctree-lsp/src/backend.rs`,
`loctree-lsp/src/lib.rs`.

File:line references are to the files above unless prefixed.

---

## A. SERVER surfaces (loctree-lsp) — standard LSP, merged by VS Code with other language servers

### A.1 Full `ServerCapabilities` (backend.rs `server_capabilities`, lines 93-200)

> SUPERSEDED (see top banner): `hover_provider` / `definition_provider` /
> `references_provider` are now `None` post-cut (`797d1fb6`, `bb6197b9`).

Every field set to `Some(...)`:

| Field | Value | backend.rs |
|---|---|---|
| `text_document_sync` | `Options{ open_close: true, change: INCREMENTAL, save: SaveOptions{ include_text: true } }` | 98-112 |
| `hover_provider` | `Simple(true)` | 113 |
| `code_action_provider` | `Simple(true)` | 123 |
| `code_lens_provider` | `CodeLensOptions{ resolve_provider: false }` | 124-126 |
| `definition_provider` | `OneOf::Left(true)` | 127 |
| `references_provider` | `OneOf::Left(true)` | 128 |
| `execute_command_provider` | `ExecuteCommandOptions{ commands: vec![] }` — **deliberately EMPTY** so vscode-languageclient does not double-register `loctree.openAtlasCard` (the wrapper owns it). Server still handles the request. | 129-141 |
| `experimental` | JSON capability map advertising the custom `loctree/*` vocabulary (see A.3) | 142-197 |

Fields explicitly **None / absent** (left at `..Default::default()`, line 198):

- `diagnostic_provider: None` — set explicitly (122). Diagnostics are PUSH-only
  (`client.publish_diagnostics` on open/change/save/refresh); pull was removed because
  tower-lsp 0.20 does not route `textDocument/diagnostic`.
- Absent (never set): `completion_provider`, `signature_help_provider`,
  `declaration_provider`, `type_definition_provider`, `implementation_provider`,
  `document_highlight_provider`, `document_symbol_provider`, `workspace_symbol_provider`,
  `document_formatting_provider`, `document_range_formatting_provider`,
  `document_on_type_formatting_provider`, `rename_provider`, `folding_range_provider`,
  `selection_range_provider`, `semantic_tokens_provider`, `inlay_hint_provider`,
  `inline_value_provider`, `moniker_provider`, `linked_editing_range_provider`,
  `call_hierarchy_provider`, `type_hierarchy_provider`, `color_provider`,
  `document_link_provider`, `workspace` (file-operations / workspace-folders).

### A.2 Standard LSP method handlers actually implemented (`impl LanguageServer for Backend`)

| Method | backend.rs | What it returns |
|---|---|---|
| `initialize` | 1592 | `initialize_result` with the capabilities above |
| `initialized` | 1632 | Discovers sub-`.loctree/` workspaces, starts watcher |
| `shutdown` | 1654 | Drops watcher task |
| `execute_command` | 1668 | Handles ONLY `loctree.openAtlasCard` (validates args, returns `{ok, card_path}`); any other command → invalid_params |
| `did_open` | 1699 | Cache doc, update live AST, push diagnostics |
| `did_change` | 1717 | INCREMENTAL live-AST edits, push diagnostics |
| `did_save` | 1743 | Update doc/live-AST (version -1), push diagnostics |
| `did_close` | 1761 | Drop doc, live-AST, symbol tracker, cached diagnostics; clear diagnostics |
| `did_change_configuration` | 1786 | No-op (debug log only) |
| `goto_definition` | 1790 | Word-at-cursor → `snapshot.find_definition` (snapshot-graph lookup, not tree-sitter) |
| `references` | 1841 | Word-at-cursor → `snapshot.find_references` = **literal occurrences from snapshot edges**, optionally prepends the export declaration site. NOT semantic refs. |
| `code_action` | 1926 | Quickfixes for cycle / dead-export diagnostics, "Open Context Atlas card" per diagnostic, file-level + symbol-level refactor actions |
| `code_lens` | 2036 | `code_lens::code_lens_for_file` over the snapshot |
| `hover` | 2049 | Server-side loctree hover card |

No handler exists for any of the absent capabilities in A.1 (no completion / document_symbol / rename / formatting / semantic_tokens / inlay_hint / etc.).

### A.3 Custom requests — registered via `LspService::build().custom_method(...)` (lib.rs 97-111)

Exactly **15** custom request methods are wired:

`loctree/refresh` (97), `loctree/contextAtlas` (98), `loctree/body` (99),
`loctree/symbolContext` (100), `loctree/contextPack` (101), `loctree/find` (102),
`loctree/follow` (103), `loctree/health` (104), `loctree/impact` (105),
`loctree/slice` (106), `loctree/workspaces` (107), `loctree/diff` (108),
`loctree/semantic` (109), `loctree/aicx` (110), `loctree/astQuery` (111).

NOT custom requests (do not confuse with the above):
- `loctree/openAtlasCard` — served via the standard `execute_command` handler (backend.rs 1672-1692), advertised in `experimental` (line 151), command list kept empty on purpose.
- `loctree/scanProgress`, `loctree/documentChanged`, `loctree/symbolChanged` — server→client **notifications** (advertised in `experimental` 143-195; emitted by the server, the client subscribes via `onNotification`).

The `experimental` map (backend.rs 142-197) advertises the vocabulary AND, for `follow` / `semantic`, splits supported vs implemented/stub scopes so clients can probe capability without round-tripping.

---

## B. CLIENT surfaces (editors/vscode/src) — what the extension registers at runtime

### B.1 All VS Code API registrations (exhaustive, by file:line)

| API call | file:line | What it is |
|---|---|---|
| `createOutputChannel('Loctree')` | extension.ts:351 | The single "Loctree" output channel (also used as the LanguageClient's outputChannel + traceOutputChannel) |
| `createStatusBarItem(Right, 100)` | statusbar.ts:25 (called from extension.ts:366, gated on `loctree.showStatusBar`) | Status bar item |
| `registerCommands(...)` | extension.ts:372 → commands.ts:466 | Registers 15 commands via the `reg()` helper (see B.2) |
| `registerOpenAtlasCardCommand(...)` | extension.ts:375 → registerCommand `loctree.openAtlasCard` extension.ts:77 | execute_command bridge |
| `registerHoverProvider(...)` | extension.ts:380 → hover.ts:291 `languages.registerHoverProvider` | **Client-side hover B** (Context-King), 7-language selector |
| `registerCommand('loctree.initialize')` | extension.ts:385 | Initialize/Scan (starts LSP on demand) |
| `createTreeView('loctree.findings')` | extension.ts:396 | Findings tree (provider `LoctreeFindingsTreeProvider`, treeview.ts) |
| `registerWebviewViewProvider('loctree.context')` | extension.ts:414 → `ContextPillViewProvider` (contextPill/panel.ts:42, viewId 43) | Context Pill webview |
| `onDidChangeActiveTextEditor(...)` | extension.ts:415 | Rescopes the pill to the active editor |
| `registerCommand('loctree.copyAgentContext')` | extension.ts:416 | Copy agent context |
| `registerCommand('loctree.contextQuery')` | extension.ts:422 | Status-bar click alias: focus pill + rescope |
| `onDidSaveTextDocument(...)` | extension.ts:454 | Debounced refresh-on-save (gated on `loctree.autoRefresh`) |
| `createFileSystemWatcher('**/.loctree/**')` | client.ts:439 | LanguageClient `synchronize.fileEvents` watcher (the only file watcher) |

Inside `ContextPillViewProvider`: `webview.onDidReceiveMessage` (panel.ts:63) with an
inbound allowlist (`rescope`, `copyAgentContext`, `openFile`, `showFullBody`), strict CSP +
per-render nonce. It does not register commands; it delegates to `loctree.navigateToFile`
and `loctree.showBody`.

No `registerCodeLensProvider`, `registerCodeActionsProvider`, `registerDefinitionProvider`,
`registerReferenceProvider`, `registerCompletionItemProvider`, or
`registerDocumentSymbolProvider` exist client-side — those LSP features come from the SERVER
(section A) through the LanguageClient. The only client-side `languages.register*` is the
hover provider.

### B.2 Commands — the TRUE runtime set (19) vs package.json (19) vs DEAD

> SUPERSEDED (see top banner): the dead `contextPanel.ts` module was DELETED post-cut
> (`797d1fb6`); its orphaned registrations below are historical.


**`reg()` helper mechanism:** `commands.ts:473-475` defines
`const reg = (name, handler) => context.subscriptions.push(vscode.commands.registerCommand(name, handler))`.
So the commands in commands.ts are registered indirectly through `reg(...)`, not via direct
`registerCommand` calls — a grep for `registerCommand` alone misses them; grep `reg('loctree`.

**15 commands registered in commands.ts via `reg()`** (line):
`loctree.refresh` (478), `openReport` (489), `showHealth` (503), `analyzeImpact` (530),
`findConsumers` (568), `findImporters` (569, shares `consumersHandler`), `showSlice` (572),
`showCycleDetails` (592), `navigateToFile` (608), `ignoreCycle` (613), `showCycles` (641),
`analyzeCycle` (642, shares `cyclesHandler`), `checkDeadExports` (645), `searchLiteral` (657),
`showBody` (711).

**4 commands registered directly in extension.ts:**
`loctree.openAtlasCard` (77), `loctree.initialize` (385), `loctree.copyAgentContext` (416),
`loctree.contextQuery` (422).

**TRUE runtime total: 19 commands.**

**package.json `contributes.commands`: 19 commands** (lines 94-189). They match the 19 live
handlers exactly — there is NO declared-without-handler command and NO live-handler-without-
declaration. (`activationEvents` lists 18 `onCommand:` entries — it omits
`loctree.copyAgentContext`, which has `"category": "Loctree"` instead of an icon and is
reachable via the pill, not a palette activation event; this is a declared command without a
matching activationEvent, not a missing handler.)

**DEAD / ORPHANED — `editors/vscode/src/contextPanel.ts`:** confirmed orphaned via
`loctree impact` → 0 direct + 0 transitive consumers, `safe_to_delete: true`. Nothing imports
it (the live `loctree.context` view is the webview from `contextPill/panel.ts`, wired in
extension.ts:414). Its `registerContextPanel` (contextPanel.ts:784) is never called, so these
registrations are **dead code, never executed at runtime**:
- `createTreeView('loctree.context')` (contextPanel.ts:791) — a TREE provider for the same
  view id that package.json now declares as a `webview` (package.json:203-206); the webview
  provider wins.
- `registerCommand('loctree.contextShowContent')` (797) — DEAD, not in package.json.
- `registerCommand('loctree.contextQuery')` (814) — DEAD duplicate; the LIVE one is
  extension.ts:422.
- `registerCommand('loctree.contextLoadMore')` (873) — DEAD, not in package.json.
- `registerCommand('loctree.contextLoadMoreGeneric')` (912) — DEAD, not in package.json.
- `registerCommand('loctree.contextPackNext')` (964) — DEAD, not in package.json.

### B.3 `package.json` `contributes` (full)

- **`activationEvents`** (41-64): `workspaceContains:.loctree`, `onStartupFinished`,
  `onView:loctree.findings`, `onView:loctree.context`, plus `onCommand:` for 18 commands.
- **`colors`** (68-93): 3 theme colors — `loctree.amber` (#c99a3b), `loctree.teal` (#3d7a72),
  `loctree.danger` (#b86a5c). Used by treeview.ts group/health icons.
- **`commands`** (94-189): 19 (enumerated in B.2).
- **`viewsContainers.activitybar`** (191-198): 1 container `loctree` (title "Loctree",
  icon `media/loctree.svg`).
- **`views.loctree`** (200-211): 2 views —
  `loctree.context` (**type: webview**, name "Loctree Context") and
  `loctree.findings` (tree, name "Findings").
- **`menus`** (213-247):
  - `editor/context`: 5 entries — `analyzeImpact` (when `resourceScheme == file`,
    group `navigation@40`), `findConsumers` (@41), `showSlice` (@42),
    `searchLiteral` (when `editorTextFocus`, @43), `showBody` (when `editorTextFocus`, @44).
  - `view/title`: 1 entry — `refresh` (when `view == loctree.findings`, group `navigation@1`).
- **`configuration.properties`** (249-298): **8 keys** —
  `loctree.serverPath` (string), `loctree.autoRefresh` (bool, default false),
  `loctree.autoScanOnStartup` (bool, default false), `loctree.showStatusBar` (bool, default true),
  `loctree.autoDownload` (bool, default true), `loctree.downloadBaseUrl` (string),
  `loctree.downloadTag` (string, default "latest"),
  `loctree.diagnosticSeverity` (enum error|warning|information|hint, default warning).
- **`languages`** (300-323): **3 contributed languages** —
  `typescript` (.ts/.tsx), `javascript` (.js/.jsx/.mjs/.cjs), `rust` (.rs).
  NOTE: these 3 are NOT the same as the 7-language documentSelector (B.4). They declare
  file-extension → language-id associations; the selector decides which languages the LSP
  providers attach to.

### B.4 LanguageClient setup (client.ts `createLanguageClient`, ~lines 400-451)

- **`documentSelector`** (428-436): **7 languages**, scheme `file` —
  `typescript`, `typescriptreact`, `javascript`, `javascriptreact`, `rust`, `python`, `go`.
  This is the scope where the SERVER's hover / references / definition / code_action /
  code_lens (section A) actually attach. The client hover provider (hover.ts:35-43) uses the
  same 7-language `HOVER_DOCUMENT_SELECTOR`, kept in sync by comment.
- **`synchronize.fileEvents`** (437-440): `createFileSystemWatcher('**/.loctree/**')`.
- **`initializationOptions`** (424-426): `{ diagnosticSeverity: <loctree.diagnosticSeverity, default 'warning'> }`.
- **No middleware** configured.
- **`outputChannel` / `traceOutputChannel`** (441-442): both the shared "Loctree" channel.
- ServerOptions (404-421): stdio transport, `run` args `['--root', <root>]`,
  `debug` args `['--debug', '--root', <root>]`, cwd = workspace root.

### B.5 Status bar item (statusbar.ts)

- Created Right-aligned priority 100 (25-35); `name` "Loctree", default command
  `loctree.contextQuery`, tooltip "Loctree Context: one-shot full repository context…".
- States via `updateStatusBar` (40-85): Initializing (`$(loading~spin)`), Ready
  (`$(type-hierarchy)`), Analyzing (`$(sync~spin)`), Healthy, Error (`$(error)` + error bg),
  Stopped. Plus inactive state set inline in extension.ts:266-275
  (`$(circle-slash) Loctree inactive`, command → `loctree.initialize`).
- `updateStatusBarFromHealth` (92-129): text `$(type-hierarchy) Loctree Context` (+ stale
  `$(history)` mark), rich markdown tooltip with health score / counts / recommended actions,
  command stays `loctree.contextQuery`; red→error bg, yellow/any-finding→warning bg.

### B.6 Hovers — TWO loctree hovers coexist

> SUPERSEDED (see top banner): `hover_provider` is now `None` and client `hover.ts`
> was DELETED post-cut (`797d1fb6`); there is no longer a loctree hover.

1. **Client hover B** (hover.ts `LoctreeHoverProvider`, registered hover.ts:291): renders the
   Context-King card from `loctree/symbolContext` (badge, bounded body, same-file usages +
   workspace count, command-link actions to `loctree.showBody` / `loctree.searchLiteral`).
   Trusted-command-scoped markdown. 7-language selector.
2. **Server hover** (backend.rs `hover` 2049, `hover_provider: true`): the server-side loctree
   hover card, attached to the same 7 languages via the LanguageClient.

Both can fire on the same symbol; VS Code merges them into one hover popup.

---

## C. Overlap / "is this too much?" candidates (for the follow-up decision — NOT decided here)

1. **Hover ×2** — client `LoctreeHoverProvider` AND server `hover_provider` both registered
   for the same 7 languages → duplicate/merged loctree hover; contradicts "don't fight IDE
   hovers."
2. **`references_provider`** — `references` (backend.rs 1841) returns **literal occurrences
   from snapshot edges**, surfaced into VS Code's native Find-All-References next to
   rust-analyzer/tsserver semantic refs.
3. **`definition_provider`** — `goto_definition` (1790) is a snapshot-graph lookup, merged
   with the real language server's go-to-definition.
4. **`code_action_provider` + `code_lens_provider`** — lightbulb + lenses from loctree merged
   with the language server's.
5. **19 commands** — many expert/legacy (showCycles, analyzeCycle, checkDeadExports,
   showCycleDetails, ignoreCycle, findImporters, openReport…). All 19 still ship despite the
   pill spec's "collapse to a few + demote experts."
6. **`contextQuery`** — kept as the status-bar click alias (focus pill + rescope).
7. **Dead `contextPanel.ts`** — 41 KB orphaned module, 6 dead registrations (incl. a dead
   tree provider for `loctree.context` and a dead duplicate `loctree.contextQuery`). Ships in
   the source tree but never runs. Candidate for deletion (`safe_to_delete: true`).

---

## D. What the Context Pill redesign INTENDED (for reference)

Primary: ambient Context Pill (webview). Secondary: `Inspect symbol or file…` scope switcher.
Expert: a few palette commands. Explicitly NOT fighting hovers. Findings stays.
→ Section A (server LSP providers), the client hover B (§B.6), and most of the 19 commands
were NOT addressed by the pill work; they are pre-existing surfaces still live. The old
query-driven `contextPanel.ts` tree was REPLACED by the webview but left in the tree as dead
code (§B.2).
