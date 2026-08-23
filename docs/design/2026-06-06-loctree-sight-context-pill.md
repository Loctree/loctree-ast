# Loctree Sight — Context Pill (editor plugin redesign)

Status: **design approved** (2026-06-06, operator). Canonical design for the
editor plugin's primary surface. Supersedes the "fix the current model" approach in
[`editors-ux-fala3.md`](./editors-ux-fala3.md) — that doc's *diagnosis and evidence*
stand; its solution (bolt fallbacks onto the search-with-modes model) is replaced by
this rethink. Hero mockup: [`loctree-sight-context-pill-hero.png`](./loctree-sight-context-pill-hero.png).

## Why we rethought it

A live VSIX smoke (transcript-builder) showed every interaction that didn't match
loctree's internal mental model dead-ended: Find on a filename → "0 matches"; body
on a symbol not in the active file → "no body found"; two literal surfaces; a
mode-picker that reads as a query box. None were code bugs — which is worse: a new
user concludes the plugin is **not useful** in two minutes. Root cause: the plugin
exposed every capability as a **mode/command** and made the user choose the right
one **before** getting any value. (Full evidence + headless reproductions in
`editors-ux-fala3.md`.)

## North star

**Understand unfamiliar code, ambiently.** A dev lands in a file and instantly
knows: what it does, what it exports, what it depends on, **what breaks if they
change it**, and can hand that whole picture to an AI agent in one click. The user
must not need to understand loctree for loctree to be useful.

## The product: Loctree Sight Context Pill

Loctree is **Sight** — it sees the active file, the snapshot, and (via the IDE)
what the user is working on. It renders one **Context Pill**: the complete
structural picture of the active file as a digestible card — "scroll twice, know
everything" — whose primary action is exporting an agent-ready context pack.

### Interaction model — three tiers

- **Primary — ambient file pill.** The pill auto-updates to whatever file the user
  is viewing/editing. No query required to get value. Default state is *always* the
  active-file Context Pill. (active file → understand → copy agent context.)
- **Secondary — search as a scope switcher (visible, not buried).** A small control
  in the pill header / above it, placeholder **`Inspect symbol or file…`** (never
  `Choose mode: Literal / Find / Body…`). It does **not** return a "search results"
  view — it **re-scopes the same pill** to a different object of interest. It is
  closer to "Go to Loctree context for X" than "Search X": we are not building a
  search engine, we are letting the pill change what it is looking at.
- **Expert — power/debug commands in the Command Palette.** Literal / Body / Slice /
  Impact / Find remain as separate commands for power users. They are not the front
  door and never surface as the primary UX.

Search must never be more prominent than the pill — putting a search box first
re-introduces the original sin (user must type before loctree gives value). The
north star is the opposite: the dev looks at code and context comes to them.

### Search behavior (the secondary scope switcher)

The user types into `Inspect symbol or file…`; loctree routes by what the input is,
and the **pill re-scopes** — same layout, same sections, same CTA:

- **Symbol match** (`parse_jsonl_to_session_record`) → pill re-scopes from
  `file:dispatch.py` to `symbol:parse_jsonl_to_session_record`: what it does, body
  preview, callers, deps, blast radius, findings. Same `Skopiuj kontekst dla agenta`.
- **Ambiguous** (`parse`) → a quick-pick / mini candidate list → pick → pill.
- **Looks like a file** (`dispatch.py`) → `scope=file:dispatch.py` → pill.
- **No symbol/file match** → literal **fallback as a suggestion**, not a separate
  mode: "No symbol found. 5 literal occurrences — show as a pill?" One click re-scopes
  the pill to those matches. Never a flat "0 matches" dead-end.

**Spec rules (verbatim):**
> - Search is a secondary scope switcher, not a primary mode picker.
> - Default state is always the active-file Context Pill.
> - Search may re-scope the pill to a symbol, file, or selected match.
> - Search must never expose loctree internal modes as the primary UX.
> - Expert commands remain available through the Command Palette.

### Pill layout (canonical — see hero PNG)
A single vertical card, color-coded sections, **no emoji** (severity shown as
colored dots/badges, not emoji):

1. **Header** — `dispatch.py` + badges `health 92` (green) · `blast radius 7` (amber).
2. **CO ROBI** — one-line summary of what the file does.
3. **BLAST RADIUS · RYZYKO ZMIANY** — a distinct, boxed **decision band** (amber
   border), elevated above body because it is the killer feature: *before* reading
   any code the user sees "a change here can touch N files" + the consumer tags
   (`cli.py`, `batch.py`, … `+N transitive`). This answers "what breaks if I touch
   this" at a glance.
4. **Two columns:** `DEFINIUJE / EKSPORTUJE · N` (green tags, fn/class symbols this
   file provides) ‖ `TEN PLIK UŻYWA · N` (blue tags, this file's dependencies).
5. **Two columns:** `BODY PREVIEW` (a few lines of the primary symbol's body + a
   `show full body` affordance — **preview only, never dominates the card; the pill
   gives understanding, it does not replace the editor**) ‖ `FINDINGS` (severity dot
   + `1 hotspot` etc., `0 dead · 0 cycles`, and a trust line `Snapshot: fresh ·
   Agent pack: ready`).
6. **Sticky CTA** (one, primary, full-width): **`Skopiuj kontekst dla agenta`**.
   On click: spinner → state changes to **`Copied Agent Context`**.

### The CTA — agent-ready context (the differentiator)
Clicking `Skopiuj kontekst dla agenta` builds and copies a Markdown **Agent Context
Pack** to the clipboard:
```
loct context --scope file:<active-file> --task <inferred-from-active-work>
```
- The analyzer already produces this (`loctree/contextPack` / `loctree/contextAtlas`
  over the LSP). The pill renders that pack; the CTA serializes it to Markdown.
- **`--task` source (stability-first):** primary = **WIP diff** (uncommitted
  changes — deterministic, always available, and naturally ties to "what your edit
  does to the whole project"); refined by the **active selection / symbol under
  cursor** when present; optional **intent field** as an override. Footer states the
  provenance: `scope=file:… · task=inferred from active work`.
- Output is universal Markdown to the clipboard — paste into any agent (Claude
  Code / Codex / anything). No agent integration required to work.

## Architecture

- **Webview, not TreeView.** The pill's colored/tagged sections, decision band,
  body preview, status line, and the build→copy→copied CTA cannot be rendered by a
  VS Code `TreeDataProvider`. The current tree-based `contextPanel` is replaced by a
  **webview** panel for the Context Pill. (Findings stays its own view — it already
  delivers instant value, the 92/100 health that impressed.)
- **No LSP protocol change.** Reuse existing requests only. **There is no
  `loctree/findings`** — the real surface (verified in `backend.rs`) is:
  `loctree/contextPack` / `loctree/contextAtlas` (pill data), `loctree/body`
  (preview), `loctree/impact` (blast radius), **`loctree/health`** (health score +
  findings/top-risks: dead/cycles/twins/hotspots), `loctree/follow` (risk drill),
  plus `find`/`slice`/`symbolContext`/`semantic` for search re-scoping. Add request
  params only if strictly needed (e.g. body ranking — see algorithm below); no new
  transport.
- **One data adapter: `ContextPillViewModel` (extension side).** The webview must
  NOT stitch five raw LSP responses itself. A single extension-side adapter assembles
  the view model — `{ scope, file, summary, health, blastRadius, exports, deps,
  bodyPreview, findings, agentPackStatus }` — from the LSP/gateway calls and hands
  the webview one stable, typed contract. The webview only renders the view model;
  swapping/adding an LSP source changes the adapter, not the webview.
- **Smart routing + zero dead-ends** (carried from the Fala 3 diagnosis): symbol
  search empty → literal fallback; body resolves cross-file (algorithm below); every
  empty state offers a next action.

### Scope taxonomy (name it explicitly so the implementation doesn't drift)

The pill always has exactly one scope. The adapter and webview speak these four:
- `scope=file:<path>` — the ambient default (active file).
- `scope=symbol:<symbol-id>` — search resolved to one symbol.
- `scope=literal:<query>` — literal-occurrence pill (the "show as a pill?" fallback).
- `scope=match-set:<query>` — a chosen set of candidate matches (from ambiguous search).

Every scope renders the same pill layout (sections degrade gracefully when a section
is N/A for that scope) and offers the same `Skopiuj kontekst dla agenta` CTA.

### Body resolution algorithm (fixes the Fala 3 "no body found" bug)

`file` is a **ranking hint, not a hard filter**. `body(fileHint, symbol)`:
1. Body in the active/hinted file → show it.
2. Else exactly one cross-file body → show it, labelled `defined in <file>`.
3. Else multiple bodies → show candidate list, user picks.
4. **Never return "no body found" if the symbol has a body anywhere in the snapshot.**
"No body" is reserved for symbols with genuinely no resolvable body (e.g. a module
name). May require an LSP-side change to `loctree/body` ranking; that is the only
sanctioned protocol-adjacent change.

### Webview security & hardening (first-class, not an afterthought)

The pill webview MUST ship hardened from day one:
- Strict **CSP** with a per-render **nonce**; scripts only via nonce.
- **No raw HTML from code/markdown** — all symbol names, paths, summaries, and body
  previews are **escaped/sanitized** before injection. Treat all LSP/snapshot strings
  as untrusted.
- **No remote assets** — styles/scripts are local only; `localResourceRoots` scoped
  to the extension.
- **Command-message allowlist** — the webview↔extension channel accepts only an
  explicit set of message types (open-file, build-context, rescope, show-full-body);
  anything else is dropped.
This prevents the usual VS Code webview security debt before any agent introduces it.

### Task inference contract ("not creepy")

`--task` is inferred from the WIP diff (+ selection), and the user must be able to
see and trust it:
- The footer shows provenance: `task: inferred from WIP diff` (+ scope).
- Before copying, the inferred task is **visible and editable** (a small field /
  override), never silently embedded. The user can edit or clear it.
- Nothing leaves the machine implicitly — the CTA only writes to the clipboard.

## Fate of the old surfaces

- The "choose a mode" prompt and the six modes-as-separate-flows are **removed as the
  main path**. The pill is the main path; the `Inspect symbol or file…` scope
  switcher replaces the mode-picker as the secondary entry.
- The ~18 commands collapse to: open/focus the pill, `Skopiuj kontekst dla agenta`,
  refresh, plus the **expert** commands (impact/slice/literal/find/body-by-name) that
  remain in the Command Palette for power users.
- The literal **quick-pick** navigator is removed; literal becomes a search fallback
  ("show as a pill?") and a palette expert command, never a competing surface.
- Power modes are palette commands, never a prerequisite to getting value.

## Scope & sequencing

- **VS Code first** (this repo's `editors/vscode`). Then mirror the pill to
  **JetBrains** and the relevant surface to **Neovim**.
- Builds on Fala 1/2 (committed, green): reuse the generic continuation (#9), the
  downloader hardening (#13), the literal-scan cache (#10) under the hood.

### Dependency / risk — Kotlin indexing gap (verified 2026-06-06)

`loct` 0.11.3 does **not** index Kotlin (`.kt`) source in its literal / occurrences /
body substrate. Verified: `LoctreeLspGateway` occurs 13× in `editors/jetbrains/**.kt`
(rg) but `loct occurrences LoctreeLspGateway` returns only README.md + gateway.ts —
zero `.kt` hits; `loct body`/`find --literal` see no `.kt`. The same data backs the
pill (`contextPack`/`body`/`impact`/occurrences).

Implication:
- The **VS Code pill is unaffected** (its real targets are Rust/TS/Python/Markdown).
- The **JetBrains pill mirror will be DEGRADED** until Kotlin indexing lands — for a
  `.kt` active file, exports/deps/blast-radius/body/occurrences would be empty.
  Therefore: **gate the JetBrains mirror on Kotlin indexing**, or ship it with an
  explicit "Kotlin source not yet indexed" empty-state (never a silent blank pill).
- The VS Code-first sequencing already absorbs this; treat Kotlin indexing as a
  prerequisite milestone for the JetBrains phase, tracked in the Loctree backlog.

## Non-goals

- Not rewriting the LSP protocol; reuse `loctree/*`.
- Not removing capabilities — re-surfacing them (expert modes move, not die).
- Not replacing the editor — body is preview, not a full viewer.
- Not building agent-integration plumbing now — clipboard Markdown is the contract;
  direct push to a connected agent is a later option.

## Verification criteria

On a fresh repo, with a user who knows nothing about loctree:
1. Opening a file shows the pill with a correct summary, exports, deps, and **blast
   radius** — without any query.
2. The blast-radius decision band answers "what breaks if I change this" at a glance.
3. `Skopiuj kontekst dla agenta` copies a valid Markdown context pack scoped to the
   file + inferred task; the button reflects build→copied state.
4. No interaction dead-ends: a non-symbol query / a symbol not in the active file
   still yields a useful result, never a flat "0 matches" / "no body found".
5. Body is a bounded preview with an explicit expand; it never dominates the card.
6. Snapshot/agent-pack status is visible (trust signal).
7. `Inspect symbol or file…` re-scopes the *same* pill to a symbol/file/match (not a
   separate results view), with the same sections and CTA; an ambiguous query offers
   candidates; a non-match offers a literal "show as a pill?" suggestion. The scope is
   one of `file:` / `symbol:` / `literal:` / `match-set:`.
8. Body resolution follows the algorithm: a symbol with a body anywhere in the
   snapshot is never reported "no body found"; cross-file bodies are labelled
   `defined in <file>`; ambiguity shows candidates.
9. The inferred `--task` is visible and editable before the CTA copies; the CTA only
   writes to the clipboard (nothing leaves the machine implicitly).
10. The webview passes a security check: strict CSP + nonce, all LSP/snapshot strings
    escaped (no raw HTML injection), no remote assets, message-type allowlist.
Verified via: a single `ContextPillViewModel` adapter unit test, contextPill webview
unit/integration tests, headless LSP smoke (stdio) for the underlying requests, and a
live VSIX install walk-through.
