---
name: loctree-impact
description: Compute direct AND transitive consumers (full blast radius) of a file before deletion, rename, or breaking signature change. Use BEFORE any `git rm`, `git mv`, public API rename, or large signature change. Triggers on phrases "impact of this file", "blast radius", "what breaks if I remove", "before I delete", "transitive consumers", "/loctree:impact", "is this safe to delete", "rename impact".
argument-hint: "<file-path>"
allowed-tools:
  - mcp__loctree-mcp__impact
---

# /loctree:impact — blast radius

Call `mcp__loctree-mcp__impact` with the file path from `$ARGUMENTS`.

## Why this is non-negotiable before deletion

`slice` shows the immediate neighborhood. `impact` traverses the transitive
closure represented in the current snapshot dependency graph. The numbers can
diverge sharply for hubs:

- A types module may have 10 direct consumers but **300+ transitive** dependents.
- A utility module may have 30 direct consumers but **only 30 transitive** if those consumers are leaves.

Deletion or breaking-rename without `impact` is structural Russian roulette. Don't.

## Reporting

After the impact arrives, surface clearly:

1. **Total affected** (transitive count) — lead the report.
2. **Direct consumers** — count + top 5 by name.
3. **Transitive multiplier** — direct/transitive ratio. >5x means deeply propagating.
4. **Severity verdict**:
   - `0` represented consumers → investigate manifests, entrypoints, generated
     wiring, reflection/dynamic loading, unsupported languages, and tests.
   - `1–9` → low risk; surgically migrate consumers.
   - `10–49` → medium risk; plan migration in a single PR with feature flag if behavior changes.
   - `50+` → high risk; this is a hub, propose alternative (deprecate + dual-publish, type alias, etc.) before deletion.
   - `200+` → **critical hub**. Removal is a multi-PR release engineering operation, not an edit.

## Authority

Impact is repository-derived graph evidence. Use it to bound structural scope,
then require independent manifest/runtime/test evidence before destructive work.

## Pair with these

- After impact, run `/loctree:follow scope:twins` to find duplicate-export sites that may absorb the deleted symbol.
- For high-risk deletions, run `/loctree:slice` on the top 3-5 transitive consumers to know what they need from this file before you cut.

## Anti-patterns

- Trusting your memory of how often a file is imported. The graph changes; impact reflects truth.
- Confusing impact with usage frequency. Impact is "files that would break", not "calls per second". A file imported once but in a hub propagates broadly.
- Running impact on a directory — that's blast-radius for a module, use `/loctree:focus` then sum or use loctree-lsp's `loctree/impact` with module scope when available.
