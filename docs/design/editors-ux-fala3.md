# Fala 3 — Editor plugin UX redesign (one surface, zero dead-ends)

Status: **design-locked pending target-shape sign-off** (2026-06-06). Authoritative
brief for the VS Code (and, by mirror, JetBrains/Neovim) plugin UX rework. Written
immediately after a live smoke where every interaction that didn't match loctree's
internal mental model dead-ended. Other agents implement against this doc;
verification criteria are at the bottom.

## The problem (one root cause, not five bugs)

None of the smoke failures were code bugs in the strict sense — the code did what
it was written to do. That is exactly why they are worse than bugs: a new user
hits a wall of curt negatives in the first two minutes and concludes the plugin
is **not useful**. The single root cause:

> The plugin exposes every loctree capability as a separate **mode/command** and
> makes the user choose the right one **before** they get any value. It assumes the
> user already knows loctree's mental model (symbol vs literal vs file; active-file
> scoping; which surface paginates). No real user knows, remembers, or tolerates that.

Every symptom below is the same sin: **mode-before-value + no fallback + no recovery.**

## Evidence (live smoke + headless verification, transcript-builder repo)

Historical smoke evidence was collected against a shipped `loctree-lsp` binary before the 0.13.0 line; keep the UX
findings, but verify any binary/version claim against the current release before implementation.

1. **Find on a filename → "0 match(es)", dead end.** User typed `dispatch.py` in
   Find; Find is symbol-only, so 0. No hint that Literal would have found it.
   Headless: `find mode=literal "import"` → 355, `"def"` → 531, `"path"` → 352.
   The data is right there; Find just refuses anything that isn't an exact symbol.
2. **"no body found" for a symbol that demonstrably exists.** `Show Symbol Body`
   on `parse_jsonl_to_session_record` → "no body found", because the command passes
   `file: activeFile` (commands.ts:738) and the active editor was `config.py`.
   Headless proof:
   - `body(parse_jsonl_to_session_record, file=dispatch.py)` → 1 body
   - `body(parse_jsonl_to_session_record, file=config.py)` → **0 bodies**
   - `body(parse_jsonl_to_session_record, file=None)` → 1 body
   The active-file scope is a **hard filter**, returning nothing when the symbol
   lives elsewhere. (The leading-space copy-paste artifact was harmless — the
   engine trims.)
3. **Two different literal surfaces with different affordances.** `Loctree: Search
   Literal Occurrences` opens a **quick-pick navigator** (no Load More); the
   Context panel Literal mode is a **tree** (has Load More). The user cannot know
   which one paginates.
4. **Mode picker reads as a query box.** "Loctree Context — choose a mode": typing
   the thing you're looking for fuzzy-matches mode *descriptions* (typing
   `snapshot` highlighted "Find: Symbol search across the snapshot"). The two-step
   "pick a mode, then maybe a query" model is invisible.
5. **Low/empty results with no next step.** Every "0 results" / "no body" is a flat
   terminal message. No "try Literal", no "defined in another file — open it",
   no recovery path.

## One genuine design bug (not just UX)

**Body active-file scoping is a filter, not a preference.** The intent was
disambiguation — when a symbol has multiple definitions, prefer the one relevant
to the active file. The implementation makes `file` a hard filter that returns
**zero** when the active file has no body for the symbol. Correct behavior: `file`
should *rank* (prefer active-file definition when several exist) and **fall back**
to cross-file resolution, reporting where it found the body. This is the highest-
leverage single fix — it alone removes the "no body found" dead-end. Applies to
both `loctree.showBody` (commands.ts) and the panel body mode (contextPanel.ts:1011-1016).

## Target shape

The thesis: **the user must not need to understand loctree for loctree to be useful.**

### 1. One surface, type anything
- A single entry point ("Loctree: Ask" / the Context panel input). No upfront mode
  choice. The user types a symbol, a filename, or a free string.
- The plugin decides what to run (see routing) and renders **one unified result**.
- Modes do not disappear — they become an *advanced/override* affordance (a small
  "as: symbol | literal | body | impact | slice" switch on the result), never a
  prerequisite to getting an answer.

### 2. Smart routing with fallback (no dead-ends)
- Input looks like an identifier → try symbol (find/symbolContext) **and** literal;
  show symbol hits first, literal occurrences below. If symbol = 0, literal still
  fills the result — never an empty "0 matches".
- Input looks like a path/filename → route to file/slice/impact for that file, not
  a symbol search that returns 0.
- Body: resolve cross-file, **prefer** the active file when ambiguous; never return
  "no body found" for a symbol that has a body somewhere — show it and label the file.

### 3. Unified, consistent result
- One result model: the symbol (with its body inline-expandable), who imports/uses
  it, its occurrences — each section consistently paginated with the **same** Load
  More and the **same** click actions (Open file / Show body / Go to occurrence).
- Kill the second literal surface (the quick-pick) or make it feed the same tree.

### 4. Zero dead-ends — every empty state recovers
- "0 symbols 'X' — N literal occurrences →" (one click to literal).
- "no body in <activeFile> — defined in <file>, showing it" (auto-fallback).
- "no results — did you mean a file? search literal? widen scope?" with clickable
  next actions. Never a terminal flat message.

### 5. Lead with what worked instantly
- First impression should be loctree's structural perception served *for free*:
  the health/findings (the 92/100 panel that already impressed) + hover/click "what
  is this symbol / what depends on it" — not a search box with modes. Search is the
  power tool; perception is the welcome.

## Implementation tasks (for executing agents)

Touch the VS Code plugin first (`editors/vscode/src`), then mirror to JetBrains.
Each task ships with a regression test (`editors/vscode/test/`).

- **T1 (body filter→preference).** `gateway.body` / the LSP `loctree/body`: make
  `file` a ranking hint with cross-file fallback. Plugin: `commands.ts` showBody and
  `contextPanel.ts` body mode must, on empty-for-active-file, fall back and label
  the resolved file. May need an LSP-side change in `loctree-lsp` body resolution.
  Test: body for a symbol not in the active file still resolves + reports its file.
- **T2 (smart search + literal fallback).** Unify the search entry: on symbol-find
  empty, automatically include/offer literal results. Remove or redirect the
  `searchLiteral` quick-pick to the panel. Test: a filename / non-symbol query
  yields literal results, not "0 matches".
- **T3 (one surface / drop mode-first).** Replace the "choose a mode" prompt with a
  single type-anything input that routes; demote modes to an on-result override.
  Test: a bare query produces a useful result without a mode selection step.
- **T4 (empty-state recovery).** Every terminal message gains a next action. Test:
  each empty path renders a clickable recovery, asserted in contextPanel tests.
- **T5 (consistent pagination + actions).** All result sections use the generic
  continuation (Fala 2 #9) and the same item actions. Test: find/slice/literal/body
  all paginate and click identically.

## Non-goals
- Not rewriting the LSP protocol; reuse existing `loctree/*` requests (add params if
  T1 needs body ranking, but no new transport).
- Not removing power-user modes — demoting them, not deleting.
- Not touching Fala 1/2 (committed, green) except where T5 reuses the continuation.

## Verification criteria (how we prove Fala 3 worked)
A reviewer/agent must be able to confirm, on a fresh repo where the user knows
nothing about loctree:
1. Typing a **filename** returns useful results (not "0 matches").
2. Typing a **symbol not in the open file** shows its body (not "no body found").
3. There is **no step where the user must choose a mode** before seeing a result.
4. **Every** empty/negative state offers a clickable next action.
5. One literal surface, one Load More behavior, one set of click actions.
Headless LSP smoke (stdio) + contextPanel unit tests + a live VSIX install walk-through.
