# Loctree Command Routing — which door for which question

> The simplify-shape doctrine (W3). Loctree grew several search surfaces; an
> agent that doesn't know which to use will reach for `grep` by reflex. This is
> the routing table that replaces moralizing with a map. Routing, not doctrine.

## Keystone principle

**Literal is the trustworthy floor.** `occurrences` / `find --literal` give
*complete* literal recall: when they return nothing, the identifier is **not
literally present** — never "the index missed it." That is what makes loctree
safe to trust over `grep` on a semantic question without losing exact truth.

- **Semantic** (where is the symbol, who imports it, blast radius) → loctree.
- **Literal completeness** (every exact occurrence, including locals in a large
  function) → `occurrences` / `find --literal`.
- **Provenance** (who introduced this line/symbol) → blame *(planned)*.
- **Raw literal text** (exact string in a known file, docstring, comment,
  markdown body) → `grep` / direct read — a **blessed companion**, not a
  rebellion. Use it when the answer lives in literal text, not in the AST /
  importer / dispatch graphs.

## Routing table

| The question | The door | Why this one |
|---|---|---|
| "Every exact occurrence of `X`, complete, never silent" | **`occurrences <ident>`** | the truth floor — "not found" means *not literally present* |
| "Where is the symbol `X` / who imports it (semantic)" | **`find <name>`** | importer graph, `where-symbol`, reverse deps — data grep can't give |
| "Literal, find-shaped, with literal vs fuzzy clearly labeled" | **`find --literal <ident>`** | returns `literal_matches` first; `fuzzy_suggestions` separate, never as primary |
| "Show me the body / range of symbol `X`" | **`body <symbol>`** | bounded brace-balanced source, no grep |
| "Graph query / keyword cluster" | **`query` / `tagmap`** | structural facts (kind→target; files + crowd + dead) |
| "Who introduced this line/symbol" | **blame / provenance** | git-history truth grep can't supply *(planned; primitives exist in `git.rs`)* |
| "Exact string in a known file / docstring / comment / markdown" | **`grep` / read** | **blessed companion** — literal text is grep's job, not a doctrine violation |

## Result-class contract

`find --literal` (and the MCP `find(mode="literal")` / LSP `mode=literal`
surfaces, at byte-for-byte parity with the CLI) separate result classes
explicitly so absence stays meaningful:

- `literal_matches` — exact, complete, primary.
- `fuzzy_suggestions` — separate, advisory, **never** primary.

A fuzzy suggestion must never masquerade as an answer. `generate_id` is not an
answer to `utterance_id`.

## Why this exists (the failure it closes)

`loct find utterance_id` once returned silence for a local variable buried in a
large function — and "no result" was read as "absent in code" when it meant
"index gap." That made `find` unsafe to trust and pushed agents to `grep`.
`occurrences` / `find --literal` make literal absence honest; this table tells
an agent which door to knock on, so reaching for `grep` becomes a deliberate
companion move on a literal-text question — not a vote of no confidence.

---

_𝚅𝚒𝚋𝚎𝚌𝚛𝚊𝚏𝚝𝚎𝚍. with AI Agents by Vetcoders (c)2024-2026 The LibraxisAI Team_
