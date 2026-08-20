# Use Case: Agent Recon Loop — Snapshot-first Structural Authority

> How AI agents use Loctree to build a verifiable structural map of the repo
> before touching a single line — and why the snapshot, not any single query,
> is the thing that makes it safe.

**Context:** Real agent workflow observed in production (CodeScribe, macOS Rust+AppKit)
**Canon:** Snapshot-first Structural Authority Loop

## The Problem

An AI agent receives a task: "refactor module X, remove tab Y, redirect routing to overlay Z." The agent has two choices:

1. **Grep and pray** — search for strings, open 40 files, start editing, discover breakage mid-flight.
2. **Recon first** — build a persistent structural map, understand dependencies, then operate with precision.

Without structural awareness, agents waste tokens on exploration, create duplicate code, miss consumers, and break downstream modules they didn't know existed.

## What actually makes recon safe: the snapshot

The Loctree differentiator is **not** any one clever query (and it is **not**
`astQuery`). It is the **snapshot**: a durable, rebuildable, checkable map of the
whole repo — file graph, edges, symbols, git identity, schema fingerprint,
staleness markers — that Loctree builds once and then serves to every surface as
the single source of structural truth.

```text
Repo checkout → snapshot authority → structural / context pack → CLI / MCP / LSP → editor surfaces
```

- **`loctree-rs`** owns the snapshot, the structural model, the analyzer
  surfaces, the **Context Pack**, and CLI truth.
- **`loctree-mcp`** exposes the same snapshot / context / slice / body / impact /
  diff / find / follow truth to agents (12 tools).
- **`loctree-lsp`** exposes snapshot-backed and live-editor-aware requests to
  editor clients (15 `loctree/*` custom requests).
- **`loctree-ast`** provides the tree-sitter parser substrate for live AST and
  structural query support (JS/TS/TSX today).
- **`editors/`** surface this value where the user actually works: VS Code and
  JetBrains context pill, findings, commands, status, and the LSP bridge.

The **Context Pack** is the agent-ready output of that snapshot: structural +
runtime + risk + action + memory + authority slices in one composition.

## The Recon Loop (snapshot-first)

### Phase 1: Refresh the authority, read the pack

```bash
loct auto                  # Refresh the derived snapshot artifacts from the live checkout
loct --for-ai              # Compact repo overview: health, hubs, risks, quick wins
loct context               # Agent context pack: structural + runtime + risk + action + memory + authority
loct context --file <path> # Scope the pack to one file's neighborhood
```

After this, the agent knows project structure, entry points, hotspots, risk, and
the recommended next safe move — from a snapshot, not from guesses.

### Phase 2: Zoom into the task

```bash
loct slice <file>          # Read one file with its dependencies and consumers before modifying
loct impact <file>         # Blast radius before deletion, rename, or major refactor
loct find <symbol>         # Symbols, definitions, imports, exact literals, occurrences
loct find --literal <id>   # Exact identifier-boundary truth scan (no fuzzy guessing)
loct follow all            # Pursue structural signals: cycles, twins, dead code, hotspots, pipelines
```

The agent now has every export and re-export of the target symbol, every file
that consumes the module being changed, and the exact blast radius — all from the
same snapshot the CLI, MCP, and LSP read.

### Phase 3: Validate after changes

```bash
loct follow all            # Catch dead code, cycles, twins left behind
loct context               # Re-read the pack; the snapshot diff is structural proof
cargo clippy -- -D warnings
cargo test
```

## Roles — read this before reaching for grep

```text
loct auto      Refreshes the derived snapshot artifacts from the live repo checkout.
loct --for-ai  Gives a compact repo overview: health, hubs, risks, quick wins.
loct context   Builds the agent context pack: structural, runtime, risk, action, memory, authority.
loct slice     Reads one file with dependencies and consumers before modification.
loct impact    Computes blast radius before deletion, rename, or major refactor.
loct find      Finds symbols, definitions, imports, exact literals, occurrences.
loct follow    Pursues structural signals: cycles, twins, dead code, hotspots, commands, events, pipelines.
```

**Doctrine for agents:**

- Do not start from grep. Start from snapshot/context.
- Before editing, use `slice`.
- Before deleting or refactoring, use `impact`.
- For symbols and literals, use `find`.
- For structural signals, use `follow`.
- LSP / live AST / editor surfaces are an **exposure and freshness layer**, not a
  replacement for the snapshot.

## Why This Beats grep

| Step | grep approach | Loctree approach |
|------|--------------|-----------------|
| Find symbol | `rg "Transcription"` → 200 string matches | `loct find Transcription` → exports, imports, call-sites with roles |
| Exact literal | `rg -w name` → text hits, no boundary truth | `loct find --literal name` → identifier-boundary occurrences from the snapshot |
| Understand scope | Open files one by one, trace manually | `loct slice <file>` → full dependency + consumer graph |
| Check for dead code | Hope someone notices | `loct follow all` → dead/cycles/twins in one pass |
| Verify completeness | "I think I got everything" | Snapshot diff → structural proof |

## Snapshot truth vs. live AST — the one distinction to keep straight

```text
Snapshot truth is the default authority.
Live AST is an editor-time freshness layer.
tree-sitter is a parser substrate.
astQuery is structural query support.
None of these replaces the snapshot-first model.
```

When an unsaved buffer is in play and `loctree-lsp` is running for a JS/TS/TSX
file, the LSP's live AST gives the agent freshness ahead of the next snapshot
rebuild. The moment the cold-start signal is past, the snapshot via `loct` /
`loctree-mcp` is canonical truth again.

## Key Insight

The recon phase typically takes seconds. The alternative — an agent exploring
blindly, opening wrong files, creating duplicates, breaking consumers — takes
minutes to hours and often needs human intervention to fix.

**Seconds of snapshot-first recon prevent hours of cleanup.**

---

*Extracted from production agent sessions. 𝚅𝚒𝚋𝚎𝚌𝚛𝚊𝚏𝚝𝚎𝚍. with AI Agents by Vetcoders (c)2024-2026 LibraxisAI*
