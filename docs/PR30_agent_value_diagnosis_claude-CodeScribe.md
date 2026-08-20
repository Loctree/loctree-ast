# Loctree Agent-Value Diagnosis — independent (claude, ex-CodeScribe witness)

Date: 2026-05-30 · Repo: `loctree-suite` · Branch: `div0-fix/the-truth-of-findings` · HEAD `8a524e7e`
Author: claude (Opus 4.8) — the agent who hit the pain points live in the CodeScribe audit.
Method: `loct repo-view` (dogfood) + Read + grep + first-hand session evidence. Independent of the prior fork's doc — convergences noted, divergences flagged.

## Frame

I am not analyzing loctree as a grep competitor — that is the wrong target, and the
operator was the first to say so. I am the agent who, hours earlier, (a) hit a 79 KB
unreadable `context` response, and (b) had to shell out to `git blame` for the single
most decisive fact of an audit. This is the first-hand view of where loctree loses
daily agent value — and where it already has the parts to win.

## Dogfood — honest

- `loct repo-view` on this 460-file / 184K-LOC Rust repo: **worked, bounded, useful**
  (health 90, twins, top hubs, structure in one JSON). This is exactly the cold-large-repo
  case where loctree earns its place. Credit where due.
- `mcp__loctree-mcp__context` in the CodeScribe session: **79,639 chars → "exceeds maximum
  allowed tokens", unreadable.** Both my failure and the fix live in Top Shot 1 below.

## What I confirm independently (code-grounded)

- `full: true` is **unconditional** — `loctree-mcp/src/main.rs:735`, set regardless of format.
- JSON response embeds the whole pack under `data` — `main.rs:793–802` (`structural`, `runtime`, `memory`).
- Markdown response renders the full pack too — `render_context_markdown(&pack)`, `main.rs:768`.
- `ContextParams` has **no** detail/receipt/section/card field — `main.rs:115–152`.
- Blame primitives EXIST — `GitRepo::blame_file` (`loctree-rs/src/git.rs:475`), `symbol_blame_rust` (`git.rs:530`), `BlameEntry` (430), `SymbolBlame` (447).
- CLI blame is **not wired** — `run_git_blame` returns `"not_implemented" / "git blame is planned for Phase 2"` (`loctree-rs/src/cli/entrypoint.rs:1037`). Two more `not_implemented` siblings at 1056, 1075.
- No MCP `provenance`/`blame` tool — grep over `loctree-mcp/src/` returns nothing; `get_info` advertises "9 tools".

---

## TOP SHOT 1 — `context` dumps because `full` is unconditional, not because a receipt is missing

Layer: MCP contract / agent onboarding · **Confidence: HIGH** · *sharper than prior fork*

Symbols: `LoctreeServer::context` (main.rs:713) · `ContextParams` (115) · `ContextOptions.full` (735) · JSON `data` block (793–802) · markdown `render_context_markdown` (768) · `materialize_context_atlas` (753) · `context_receipt_payload` (786) · `get_info` (2147)

Evidence (first-hand + code):
- I hit the overflow on `format='markdown'` — so this is NOT a json-only problem. `full:true` (735) is format-agnostic; both branches render the full pack.
- The response **already carries the bounded receipt**: `atlas` pointer_payload (778), `receipt` freshness (786), `advisory` with next-tools (792), `sections_loaded` (780). The problem is the full `data` (793–802) bolted alongside — the `advisory` even brags "Context is complete in this response AND also materialized as a Context Atlas". That deliberate doubling is the overflow.

Why this matters: a cold agent needs identity + freshness + top hubs + card index + next two tools — which is precisely what's ALREADY in the response minus `data`. The dump is not adding agent value; it is the thing that truncates and trains the agent to distrust loctree-first.

Recommended cut (cheaper than "add receipt_only mode" — this is gating, not construction):
- Add `detail: Detail` to `ContextParams` (`enum Detail { Receipt (default), Full }`).
- Gate the `data` block (793) and `render_context_markdown` (768) behind `detail == Full`.
- Default response = `protocol/session/status/atlas/receipt/sections_loaded/advisory` only — page into the atlas via the existing `context_section`/`context_manifest`.
- Fix `get_info` (2149) to recommend `context(detail=receipt)` or `repo-view` first; stop calling markdown "operator-readable" (it's the path with the receipt scaffolding — agents should not be steered away from it).

Regression: `context_default_response_is_bounded` — assert no `data.structural`/`data.runtime` and total bytes under a hard budget unless `detail=full`.

---

## TOP SHOT 2 — provenance primitives are orphaned (the differentiated win)

Layer: provenance / line+symbol truth · **Confidence: HIGH** · *full agreement with prior fork; I am the proof case*

Symbols: `GitRepo::blame_file` (git.rs:475) · `GitRepo::symbol_blame_rust` (git.rs:530) · `BlameEntry` (430) · `SymbolBlame` (447) · `FileSymbolBlame` · `run_git_blame` (entrypoint.rs:1037, not_implemented)

Evidence: the decisive fact of my CodeScribe audit — `612c8260` (contract surface) vs `c3ce222` (relabel only) — came from `git blame`. Loctree had the core machinery (`symbol_blame_rust` maps Rust symbols → introducing/last-modifying commit — that is BEYOND grep) but no surface to reach it, so I shelled out.

Why this matters: this is the cleanest "grep cannot supply this" case loctree owns. Line-blame any tool can do; **symbol-level introduction/last-touch is loctree's native edge** and it is already written, just unreachable.

Recommended cut:
- MCP tool `provenance(file, line?, symbol?)` on `blame_file` / `symbol_blame_rust`.
- Wire CLI `loct git blame <file>` (replace the not_implemented stub).
- Compact shape: `{file, line_range, introduced_by, last_modified_by}`.
- Optionally attach provenance to `find(where-symbol)` when cheap.

Regression: replace `git_blame_returns_not_implemented` with a fixture asserting commit hash + line; MCP `provenance_symbol_returns_introducing_commit`.

---

## TOP SHOT 3 — `get_info` is doctrine, not a routing table (and it mislabels the good path)

Layer: instructions / ergonomics · **Confidence: HIGH** · *agree + one addition*

Symbols: `get_info` (main.rs:2147–2162) · `repo_view` (813)

Evidence: the instruction string leads with full `context` as START, lists 9 tools, gives no decision tree, and labels `format='markdown'` "operator-readable" — actively steering agents away from the one branch that carries the receipt scaffolding.

Recommended cut — replace doctrine with routing:
- "Where am I?" → `repo-view`  · "Edit safely?" → `slice`  · "Who depends?" → `impact`
- "Who introduced this line/symbol?" → `provenance`  · "Runtime signal?" → `follow`
- "Exact literal in a known file?" → direct read / literal search (bless it as companion, not rebellion)
- Bless `git blame` + exact literal search explicitly for line-provenance / literal questions.

Regression: snapshot test on `get_info` containing the routing table and not asserting `context` as the sole first move.

---

## Where I DIVERGE from the prior fork (witness correction)

- **Transport reliability (their Top Shot 4): NOT my failure mode.** Mine was payload
  overflow (79 KB), not `Transport closed`. For MY failure, Top Shot 1 is the fix — gating
  `data`. A transport smoke gate is fine, but do not let it absorb the overflow root cause;
  they are different bugs with different regression tests. Confidence on transport from my
  chair: LOW (didn't observe it).
- **Literal-search lane (their Top Shot 5): deprioritize.** Chasing grep's target. The
  grep-shaped questions I had (exact string, "what does this fn do") are legitimately
  grep/Read territory — the operator himself said "to nawet nie ten sam target". The ONLY
  differentiated version is **literal + attached provenance**, which collapses into Top Shot 2.
  Building a bare literal lane reimplements grep worse. Cut or fold into #2.

## Highest-leverage first move (my pick)

1. **Gate `data` behind `detail` (Top Shot 1)** — removal, not construction; fixes the exact
   79 KB overflow I hit; the receipt scaffolding already exists.
2. **`provenance(file, line|symbol)` MCP tool (Top Shot 2)** — the one thing that makes an
   agent say "loctree did what shell-blame can't" (symbol-level), on primitives already written.
3. **`get_info` routing table (Top Shot 3)** — so agents stop being told the dump is the start.

That trio converts my CodeScribe critique into product force: the useful bounded card becomes
the default, the blame gap becomes loctree's advantage, and grep becomes a blessed companion
for literals instead of a rebellion against doctrine.

## Confidence table

| Candidate | Confidence | Basis |
| --- | ---: | --- |
| Gate `data` behind `detail` (bounded default) | HIGH | First-hand overflow + `full:true` unconditional (735), receipt scaffolding already present (778/786/792) |
| `provenance` MCP tool | HIGH | Primitives exist (git.rs:475/530), CLI not_implemented (entrypoint:1037), no MCP tool; my audit is the proof case |
| `get_info` routing rewrite | HIGH | Instruction string leads with full context, mislabels markdown as operator-only (2149) |
| Transport smoke gate | LOW (my chair) | I did not observe Transport closed; my failure was payload size |
| Literal search lane | CUT / fold into provenance | Wrong target; only defensible as literal+provenance |

---

_𝚅𝚒𝚋𝚎𝚌𝚛𝚊𝚏𝚝𝚎𝚍. with AI Agents by Vetcoders (c)2024-2026 LibraxisAI_
