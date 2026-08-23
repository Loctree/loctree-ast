---
name: loctree-prism
description: Score conceptual smear (drift, spread, runtime centrality, authority diversity, drift risk, closure evidence) across two or more task framings on the canonical 0..15 rubric. Use BEFORE running `vc-polarize`, BEFORE choosing a doctrine cut, or whenever an agent needs structural evidence for the abort/memo/pass/doctrine decision. Triggers on phrases "prism", "polarize gate", "smear score", "before polarize", "should I split this", "is this one truth or many", "vc-polarize evidence", "/loctree:prism", "loct prism".
argument-hint: "<task1> <task2> [<task3> ...] (two task framings minimum)"
allowed-tools:
  - mcp__loctree-mcp__prism
---

# /loctree:prism — conceptual smear evidence

Call `mcp__loctree-mcp__prism` with at least two task framings from `$ARGUMENTS`. Returns the canonical `loctree.prism.v1` JSON contract that `vc-polarize` consumes for band-action routing.

## Why this is non-negotiable before vc-polarize

`vc-polarize` chooses one axis and rejects competing truths. That choice is risky if the competing truths are imaginary (you split something that's already one truth) or if you keep what is actually three truths fused together. `prism` is the structural evidence layer that tells you which:

- **Same-truth signal** — high overlap, low spread, single authority cluster, low drift risk → `total_score` 0..4 → action `abort` (no polarize needed; one truth already)
- **Local-note signal** — moderate overlap, single runtime cluster, but evidence not yet captured → `total_score` 5..8 → action `memo` (corpus tag suffices)
- **Polarize-ready signal** — divergent overlap, multi-runtime, drift risk present → `total_score` 9..12 → action `pass` (run `vc-polarize`)
- **Doctrine-ready signal** — fully fragmented, high authority diversity, closure evidence already cached → `total_score` 13..15 → action `doctrine` (write canonical decision into context corpus)

Skipping prism = guessing which band you are in. The evidence is cheap; the polarize step is expensive.

## Five axes (each 0..3, total 0..15)

| Axis | What it measures | High score signal |
|---|---|---|
| `spread` | Surface-kind diversity across tasks | code + runtime + tests + docs all present |
| `runtime_centrality` | Hub overlap in runtime call graph | many central files, deep runtime signals |
| `authority_diversity` | Number of distinct authority labels (`repo_verified`, `loctree_derived`, `aicx_operator`, etc.) | 3+ distinct authorities |
| `drift_risk` | Average pairwise file overlap + low lexical memory + dirty cache | low overlap, missing memory, stale snapshot |
| `closure_evidence` | Verification gates + likely tests touching the surface | gates present, tests likely |

Total score is the sum. Bands: `0..=4`, `5..=8`, `9..=12`, `13..=15`.

## Reporting

After the prism JSON arrives, surface clearly:

1. **`total_score` and band** — lead the report.
2. **Band-action verdict** — translate the band to the canonical action keyword (`abort` / `memo` / `pass` / `doctrine`). The runner derives this same mapping from `total_score`; surfacing it explicitly makes the gating decision auditable.
3. **Top axis contributors** — the two highest-scoring axes drive the verdict.
4. **Overlap summary** — `union_files`, `shared_files_all_tasks`, `average_pairwise_jaccard`. Low Jaccard with high union = real divergence.
5. **`recommendation` string** — the human-readable next step (loctree's own English summary; not the action keyword).

## Authority

Prism carries `loctree_derived` snapshot-and-scoring evidence. It is useful for
comparing task surfaces, but the score does not supersede operator intent or
runtime evidence. The action vocabulary mapping is consumed by `vc-polarize`.

## Pair with these

- Before prism, run `/loctree:context` to materialize the Atlas — prism reads the same dense ContextPack per-task and needs the cache warm.
- After prism returns band 9..12 or 13..15, hand `total_score` and `axes[]` to `vc-polarize` as gating evidence. The dispatch prompt should cite the band explicitly (`band: 9..12: pass`).
- For band 5..8 (`memo`), do not dispatch `vc-polarize`. Capture a local Loctree tag or context-corpus entry instead and continue implementation.
- For band 0..4 (`abort`), do not even capture a memo. Note the score in the conversation, move on.

## Schema contract

JSON output is pinned to `loctree.prism.v1` (see `schemas/loctree.prism.v1.schema.json`). Schema bump requires a new version string and an updated golden fixture in `loctree-rs/tests/fixtures/`. Consumers (`vc-polarize` runner, `vc-operator`) should fail closed if they see an unknown `schema_version`.

## Anti-patterns

- Running `prism` with one task framing — it requires at least two to score overlap. The handler will return an error.
- Treating prism's English `recommendation` string as the gating signal. The signal is `total_score` → band → action keyword. The `recommendation` is for humans reading the markdown view.
- Skipping prism and calling `vc-polarize` directly. That bypasses the abort/memo branches and burns a polarize dispatch on cases that needed neither.
- Re-running prism repeatedly during a single session. The result is stable for the snapshot; cache the band in conversation context and only re-run if the tree actually moved.
- Mutating the `loctree.prism.v1` schema in place. Bump to `v1.1` / `v2` and refresh the golden fixture — silent shape changes break consumers.
