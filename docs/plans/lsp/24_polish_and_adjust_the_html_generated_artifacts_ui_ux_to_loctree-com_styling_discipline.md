---
name: polish-html-artifacts-to-loctree-com-styling-discipline
description: Bring generated Loctree HTML reports into visual and UX alignment with the loctree-com SaaS/product surface.
type: implementation_plan
project: Loctree/loctree-suite
plan_number: 24
date: 2026-05-11
status: done
agent_target: any
human_importance: critical
agent_importance: low
depends_on: 23-wire-html-layer-into-real-analyzer-flow
external_context:
  - /Users/polyversai/Libraxis/vc-runtime/loctree-com/docs/licensing/polar-sh/11-cloud-pricing-surface.md
  - /Users/polyversai/Libraxis/vc-runtime/loctree-com/docs/licensing/polar-sh/SHIPPED.md
---

# Plan 24 — Polish generated HTML artifacts to loctree-com styling discipline

## Why

Once plan 23 makes HTML reports come from real analyzer data, the next human
milestone is trust. The artifact must look like it belongs to the same product
family as `loctree-com`, especially the Polar-backed `/cloud` SaaS surface, not
like an internal debug dump or a separate weekend prototype.

This is not about making agents happier. Agents will still use structured
surfaces. This task exists because human teams, buyers, reviewers, and internal
operators will judge the commercial product through screenshots, generated
reports, PDF/browser captures, sales follow-ups, and enterprise evidence packs.

## Styling discipline

Use loctree-com as product truth, not as a pixel-copy target:

- Preserve Loctree's dark, technical, evidence-first personality.
- Reuse the same hierarchy logic visible in the `/cloud` plan: eyebrow, title,
  body, fit/metadata, tier-like cards, fineprint, and clear CTA/help affordances.
- Keep `/pricing` semantics separate from SaaS artifacts. Generated reports may
  mention license/provenance, but must not become pricing pages.
- Prefer design tokens and systematic CSS over one-off inline style patches.
- Make the generated report credible offline: it should survive being attached
  to a ticket, pasted into a sales thread, or opened from `file://`.

## Acceptance criteria

- [ ] The generated HTML report has a coherent information architecture for
      humans: executive summary first, then evidence sections, then detailed
      graphs/tables, then reproducibility/provenance.
- [ ] The visual language is adjusted to match loctree-com discipline: dark
      surface, restrained contrast, readable cards, consistent spacing, stable
      typography scale, and a deliberate accent palette rather than ad hoc
      colors per component.
- [ ] `reports/src/styles.rs` is organized around reusable tokens or clearly
      named style blocks. Avoid uncontrolled growth of component-specific CSS
      fragments when a shared report token would do.
- [ ] Header/hero treatment communicates product identity without pretending to
      be the hosted `/cloud` page. It should say “generated Loctree report” (or
      equivalent), not “buy now” or “checkout”.
- [ ] Critical human-readability paths are covered: long file paths, many hub
      files, empty sections, warning/error states, graph fallback text, and
      narrow viewport rendering.
- [ ] The artifact is accessible enough for review: semantic headings, useful
      link text, focus-visible states for interactive controls, no color-only
      severity encoding, and acceptable contrast for primary text and badges.
- [ ] Interactive graph/table affordances are explained in human language. If
      JavaScript is unavailable, the report still exposes meaningful static
      fallback evidence.
- [ ] Generated reports include a small “share/evidence” footer with version,
      timestamp, source project, and reproduction command where available.
- [ ] No loctree-com secrets, Polar product IDs, checkout URLs, customer emails,
      or entitlement data are embedded in generated artifacts.
- [ ] Visual regression evidence exists for at least one fixture report and one
      non-trivial real report. Evidence can be screenshots, deterministic HTML
      snapshots, Playwright text/screenshot checks, or a documented renderer
      smoke script.

## Expected code locations

- `reports/src/styles.rs` — primary styling/token work.
- `reports/src/components/document.rs` — document shell, header, footer,
  ordering, and global UX affordances.
- `reports/src/components/section.rs` and report section components — card,
  empty-state, severity, table, graph, and metadata treatments.
- `reports/src/components/icons.rs` — only if icon semantics need alignment.
- `reports/src/lib.rs` and tests — renderer-level assertions and snapshots.
- `reports/examples/*` — update examples only if they remain useful as visual
  smoke fixtures; production truth still comes from plan 23's analyzer flow.

## Suggested implementation sequence

1. Capture the current generated report from plan 23 as the baseline artifact.
2. Compare against loctree-com `/cloud` principles from the Polar roadmap:
   separate SaaS surface, bilingual/product-grade copy discipline, card grid
   hierarchy, and high-performance static rendering.
3. Introduce or normalize report styling tokens in `reports/src/styles.rs`.
4. Polish the document shell, cards, badges, tables, graph containers, empty
   states, and footer provenance.
5. Add fixture coverage for ugly real-world cases: long paths, empty evidence,
   many rows, warnings, graph-disabled/offline mode, and narrow viewports.
6. Record before/after evidence in the completion report.

## Verification

Minimum gate for the implementing marble round:

```bash
cargo fmt --all --check
cargo test -p report-leptos
cargo run -q -p loctree --bin loct -- <html-command-from-plan-23> --project <fixture-or-repo> --output target/loctree-smoke/polished-report.html
```

If a browser/Playwright smoke harness exists or is introduced, run it against
the generated artifact and capture at least desktop + narrow viewport evidence.
If no browser harness exists yet, the completion report must explicitly say so
and include a deterministic HTML/text assertion fallback.

## Human review checklist

- [ ] Can a non-Rust teammate understand the report's top three findings in
      under one minute?
- [ ] Does the report look like a Loctree product artifact beside `loctree-com`
      screenshots?
- [ ] Are scary findings visually clear without becoming alarmist?
- [ ] Can a human copy the reproduction command and regenerate the same class of
      artifact?
- [ ] Does the artifact remain useful offline and without access to SaaS state?

## Non-goals

- No analyzer semantics changes; visual polish must not rewrite the facts.
- No billing, checkout, account, or entitlement UI inside generated reports.
- No dependency on a live loctree-com deployment to render local reports.
- No broad replacement of the report component architecture unless plan 23
  proves the current architecture cannot render real analyzer data.
- No fake marketing claims such as “cloud synchronized” unless backed by actual
  analyzer/runtime evidence.

## Exit contract

- TASK: mark this file `done` only after plan 23 is done or an explicit staged
  exception is recorded.
- REPORT: add `reports/lsp/24-polish-html-artifacts-to-loctree-com-styling-discipline.md`
  with before/after evidence, commands, screenshots or HTML assertions, and
  known residual UX gaps.
- TRACKER: if the LSP/Loctree master tracker is still being used as the batch
  control plane, add/update row 24 with exact status and report path.

Vibecrafted with AI Agents (c)2024-2026 The LibraxisAI Team