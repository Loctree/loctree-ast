# Perception over Memory

Agent quality depends less on storing everything and more on **seeing the right thing at the right moment**.

For code work, stale or oversized memory lowers reliability. We prioritize deterministic, fresh, auditable context over broad historical recall.

## The Problem We Reject

- Prompt bloat at session start.
- Retrieval drift against fast-moving repositories.
- Low explainability of context selection.
- Expensive, non-reproducible agent behavior.

## The Architecture We Choose

1. **Perception-first** context acquisition via structural tools.
2. **Scoped, on-demand** loading instead of bulk preload.
3. **Prepared context bundles** for recurring workflows.
4. **Optional memory** as enrichment, never source of truth.

## Operational Model

Default pre-edit sequence for non-trivial changes:

```
repo-view  (global shape)
  focus    (subsystem boundary)
    slice  (target + deps + consumers)
    impact (blast radius)
    find   (reuse vs duplicate)
    follow (pursue flagged signals)
```

Use `grep`/`rg` for local detail only, not as the primary mapping layer.

## Design Constraints

- Keep context minimal but sufficient.
- Prefer fresh tool output over recalled summaries.
- Record provenance: each major decision should reference the producing command.
- Treat context as a finite attention budget.

## Relation to Memory

Memory remains valid for longitudinal preferences, user intent continuity, and historical notes. It must not override current structural state from tools.

## Non-goals

- Removing memory integrations.
- Forcing one orchestration framework.
- Treating long-context windows as a substitute for context engineering.

## Success Condition

This manifesto is successful when agents produce more first-pass-correct changes with lower token cost and clearer audit trails.

## Deep Dives

- [Architecture Decision Record](docs/perception/adr.md)
- [Agent Context KPIs](docs/perception/kpis.md)
- [Global Direction Research](docs/perception/research.md)

---

VibeCrafted with AI Agents (c)2026 Loctree Team
