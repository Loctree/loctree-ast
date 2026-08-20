# Loctree editor surface policy (VS Code)

Status: **Phase 1 ruled + landed (2026-06-06)** — undisputable cuts done; `code_lens` deferred; Fala 3 UX next on the clean surface.
Companion to the factual inventory (`editors-surface-inventory.md`). This is the
keep/demote/disable/delete ruling per surface, with rationale, replacement path, and
user-facing consequence.

## Verdict (operator)
**The Pill is the hero.** Cut anything that pretends to be a second hero. Anything
that is plumbing or an expert escape hatch stays — but hidden. We do NOT fight IDE
hovers; Loctree wins with the Pill.

---

## KEEP — PRIMARY

| Surface | Why | User-facing |
|---|---|---|
| **Context Pill / Loctree Sight** webview (`loctree.context`) | The hero. Ambient per-file/symbol structural picture + agent-context export. | The main Loctree surface. |
| **Findings tree** (`loctree.findings`) | Separate, clear surface: repo health / findings. Complements the Pill, does not compete with it. | Repo-level health & findings view, unchanged. |
| **Status bar item** | Entry point to the Pill + at-a-glance health signal. | Click → focus Pill; shows health/state. |
| **Output channel / logs** (`Loctree`) | Plumbing — diagnostics/trace. | Unchanged (logs only). |
| **File watcher + refresh plumbing** (`**/.loctree/**`, save/refresh) | Plumbing that keeps the snapshot fresh. | Invisible; keeps data current. |

## KEEP — SECONDARY (present but not the front door)

| Surface | Why | User-facing |
|---|---|---|
| **`Inspect symbol or file…` scope switcher** (in the Pill) | Re-scopes the Pill; not a separate search UI. | A small control in the Pill, not a primary mode picker. |
| **`Copy Agent Context`** (CTA + command) | The Pill's primary action / agent bridge. | Button in Pill + palette command. |
| **`Refresh` / `Initialize`** commands | Operational escape hatches. | Palette commands. |
| **Expert commands** (impact / slice / find / showBody / etc.) | Power-user escape hatches; not the front door. | Command Palette only. |

## CUT / DISABLE BY DEFAULT

| Surface | Why | Replacement path | User-facing consequence |
|---|---|---|---|
| **`references_provider`** (server, backend.rs:128) → native Find-All-References | **Most decisive cut.** VS Code shows native "References"; users assume SEMANTIC refs from the language LSP. Loctree serves **literal occurrences** — helpful, but must not masquerade as rust-analyzer/tsserver. | Surface the same data INSIDE the Pill as **"Literal occurrences / used by"** (we already have it via slice.consumers + the literal scope). | Native Find-All-References stops mixing in Loctree literal hits → it shows ONLY the real language server's semantic refs (cleaner, less confusing). "Used by" lives in the Pill. |
| **Client `LoctreeHoverProvider`** (hover.ts) | "We don't fight hovers." IDE hovers are already crowded/conflicting. | The Pill (ambient, follows the symbol/file). | No Loctree hover card on hover. Optionally a setting to re-enable (default off). |
| **Server `hover_provider`** (backend.rs:113) → `async fn hover` | Same reason; this is the SECOND Loctree hover (double-hover with hover.ts). | The Pill. | Same — no Loctree contribution to the native hover. |
| **`definition_provider`** (server, backend.rs) → **None** | Same false-semantic-duplicate class as references: VS Code's native "Go to Definition" implies a semantic resolution from the language server (rust-analyzer / tsserver / pyright / gopls), which do it better. Loctree's is a snapshot-graph lookup and must not compete. **Cut wholesale** (operator ruled: don't keep even where no mainstream LSP exists — the masquerade risk outweighs the edge case). | Native language-server go-to-definition. | Go-to-definition uses the real language server only; no Loctree contribution. |
| **Literal quick-pick** (`searchLiteral` as a primary flow) | Competes with the Pill's literal scope; a separate results view re-introduces "mode before value." | Literal lives in the Pill (`scope=literal`) + stays a palette expert command. | No standalone literal navigator as a main path; literal results render in the Pill. |

## DELETE

| Surface | Why | User-facing |
|---|---|---|
| **`contextPanel.ts`** (orphaned tree provider + dead `loctree.context` tree + dead `contextLoadMore` / `contextLoadMoreGeneric` / `contextPackNext` / `contextShowContent` registrations) | Dead code — verified 0 consumers (`safe_to_delete`); the webview Pill owns `loctree.context` now. | None (already unreachable). Removes ~750 LOC of dead surface. |

## KEEP — RULED (not a duplicate, additive)

| Surface | Why |
|---|---|
| **`code_action_provider`** (server, backend.rs) | Operator ruled KEEP. Unlike hover/references/definition, these are **loctree-specific, additive** actions (cycle/dead-export quickfixes + "Open Context Atlas card" from a diagnostic) — the language server does not provide them, so there is no masquerade/duplicate. May later fold into the Pill, but stays ON. |

## FOLLOW-UP (later, not now)

| Surface | Note |
|---|---|
| **`code_lens_provider`** (server, backend.rs) | Candidate for **disable-by-default** (inline-lens clutter merging with the language server's), but not objectively bad → deferred. Decide after the surface is otherwise clean. |
| Dead trait handlers `async fn hover` / `references` / `goto_definition` | Now unadvertised (caps `None`) but still present as valid trait impls. Harmless (clippy-clean); tidy-up removal optional. |

## Implementation note
- Server cuts = flip the capability to `None` in `backend.rs server_capabilities` (hover/references/definition done).
- Client hover cut = `hover.ts` deleted + `registerHoverProvider` unwired in extension.ts.
- Delete = `contextPanel.ts` removed (+ its dead test) — its command ids were never in package.json.
- The 7-language `documentSelector` STAYS — the LanguageClient still needs it for push diagnostics + the custom `loctree/*` requests; only the capabilities are trimmed.

## Status — Phase 1 LANDED (2026-06-06)
Commits `797d1fb6` (dead panel + double hover + references + literal quick-pick demote) and
`bb6197b9` (definition_provider None). Verified: cargo check + clippy clean, vscode tsc/lint/test/esbuild green.
**Next:** fresh-VSIX reality check (confirm native hover / Find-References / go-to-def now come ONLY from the
language server), THEN Fala 3 UX (fallbacks, smart scope, empty states) on the clean surface.
