---
name: wire-html-layer-into-real-analyzer-flow
description: Make the Leptos HTML report layer reachable from real loctree analyzer output, not only examples or fixture/demo paths.
type: implementation_plan
project: Loctree/loctree-suite
plan_number: 23
date: 2026-05-11
status: done
agent_target: any
human_importance: critical
agent_importance: low
depends_on: 22-context-scope-flag, reports-crate
external_context:
  - /Users/polyversai/Libraxis/vc-runtime/loctree-com/docs/licensing/polar-sh/00-roadmap-readme.md
  - /Users/polyversai/Libraxis/vc-runtime/loctree-com/docs/licensing/polar-sh/SHIPPED.md
---

# Plan 23 — Wire HTML layer into the real analyzer flow

## Why

The `reports/` crate already contains a Leptos SSR HTML renderer
(`report_leptos::render_report`) and a substantial component/style layer, but
the product milestone is not “a renderer exists”. The milestone is that a human
team can run Loctree on a real repository and receive a credible HTML artifact
that is derived from the same analyzer truth used by CLI, MCP, ContextPack, and
LSP surfaces.

This is intentionally not a high-priority agent-consumption task. Agents already
prefer structured JSON, Context Atlas cards, and MCP slices. It is critical for
human teams because the SaaS transformation in `../loctree-com` needs proof
artifacts for buyers, reviewers, enterprise champions, and non-terminal users.
The Polar `/cloud` surface is already positioned as the billing/product front
door; this plan provides the generated evidence artifact that can stand behind
that commercial story.

## Product boundary

- `loctree-suite` owns analyzer truth and generated static report artifacts.
- `loctree-com` owns marketing, billing, checkout, portal, and hosted SaaS
  surfaces (`/cloud`, `/pricing`, `/account`, `/api/*`).
- This task must not move Polar, entitlement, or marketing logic into
  `loctree-suite`.
- This task must not rewrite `/pricing` or conflate BUSL legal copy with cloud
  pricing. The loctree-com Polar roadmap explicitly keeps `/pricing` as legal
  copy and `/cloud` as SaaS tier copy.

## Acceptance criteria

- [ ] A real analyzer-driven HTML generation path exists from a documented CLI
      command or flag. The entrypoint may be a new command or an extension of an
      existing report/context command, but it must be discoverable in help text.
- [ ] The HTML artifact is built from current analyzer/snapshot data for the
      selected project root, not from `reports/examples/*`, demo graph data, or
      stale bundled fixtures.
- [ ] The adapter from analyzer data to `report_leptos::types::ReportSection`
      is explicit, small, and tested. It must reuse existing analyzer outputs
      instead of re-scanning or re-implementing analysis inside `reports/`.
- [ ] The command supports a deterministic output path, writes parent
      directories when needed, and fails with an actionable error when the
      output path is unwritable.
- [ ] The generated HTML includes enough real sections to be useful for humans:
      project identity, health summary, hub/high-fan-in files, graph or tree
      evidence where available, action plan or quick commands, and provenance
      metadata.
- [ ] Provenance is visible in the artifact: generated timestamp, loctree
      version, project root or display root, git branch/commit when available,
      and snapshot/cache scope when available.
- [ ] JavaScript/CSS assets required by interactive graph sections are embedded,
      copied, or otherwise referenced in a way that survives opening the report
      from disk. No broken local asset links in the default path.
- [ ] Existing machine-readable surfaces remain stable. JSON, MCP, ContextPack,
      LSP, and AI-oriented output formats must not change unless explicitly
      required and covered by compatibility tests.
- [ ] Round-trip e2e evidence exists: run the command against a minimal fixture
      repository, assert that the HTML file is written, and assert that it
      contains real fixture-derived facts rather than only generic shell text.
- [ ] A second smoke run against this repository or another non-trivial local
      repository is recorded as human-evidence output, even if it is not a unit
      test fixture.

## Expected code locations

- `loctree-rs/src/cli/` — CLI flag/command parsing and dispatch.
- `loctree-rs/src/analyzer/` and existing snapshot/context modules — source of
  analyzer truth; do not duplicate analyzer logic in the report crate.
- `reports/src/lib.rs` — renderer entrypoint (`render_report`).
- `reports/src/types.rs` — target DTOs for report sections.
- `reports/src/components/*` and `reports/src/styles.rs` — rendering layer,
  only touched if the real data path exposes missing fields.
- `loctree-rs/tests/` or the relevant crate-level integration test directory —
  e2e command coverage with a fixture repository.

## Implementation notes

Prefer a narrow adapter boundary:

1. analyze/load the project using the same path the CLI already trusts,
2. produce a typed intermediate report model from analyzer outputs,
3. call `report_leptos::render_report`,
4. write the artifact and report the final path to stdout.

If the existing CLI already has a partial report command, extend it rather than
adding a parallel command. If there is no obvious command, choose the smallest
stable surface and document it in help text and README/CLI docs.

## Verification

Minimum gate for the implementing marble round:

```bash
cargo fmt --all --check
cargo test -p loctree --test e2e_cli html
cargo test -p report-leptos
cargo run -q -p loctree --bin loct -- <chosen-html-command> --help
cargo run -q -p loctree --bin loct -- <chosen-html-command> --project <fixture-or-repo> --output target/loctree-smoke/report.html
```

The actual command name must replace `<chosen-html-command>` in the completion
report. If the implementation touches workspace-level CLI wiring, run the
relevant broader gate from the repository guidelines (`make precheck` or the
smallest equivalent command set) before marking this done.

## Round-trip evidence contract

The completion report must include:

- exact command used to generate the fixture HTML,
- path of the generated artifact,
- 3–5 asserted strings or structural facts proving the artifact came from the
  fixture/repository under analysis,
- one screenshot or text-render excerpt suitable for human review,
- note on whether graph assets work when opened from disk.

## Non-goals

- No SaaS hosting, authentication, billing, Polar entitlement checks, or upload
  workflow in this task.
- No loctree-com route changes in this task.
- No redesign/polish of the report visual language beyond what is necessary to
  make real analyzer data render correctly; that is plan 24.
- No fake “sample” data in the production command path.
- No weakening of existing JSON/MCP/LSP outputs to satisfy HTML needs.

## Exit contract

- TASK: mark this file `done` only after code, tests, and round-trip evidence
  are present.
- REPORT: add `reports/lsp/23-wire-html-layer-into-real-analyzer-flow.md` with
  code evidence, test evidence, generated artifact path, and any UX gaps handed
  to plan 24.
- TRACKER: if the LSP/Loctree master tracker is still being used as the batch
  control plane, add/update row 23 with exact status and report path.

Vibecrafted with AI Agents (c)2024-2026 The LibraxisAI Team