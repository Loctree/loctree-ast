# loctree-com content audit for MCP parity

Date: 2026-08-13

## Scope and evidence

This is a read-only audit of the live sibling checkout at `../loctree-com`.
No files in that repository were changed. The checkout was on `main` at
`536c2e60811806160076fb290f78855a49327b1f` with 105 dirty or untracked paths,
so the findings below describe the live working tree, not a clean release
candidate.

The audit used Loctree raw-regex search across all 178 indexed files. The search
found four current product-facing surfaces that still describe an older MCP
tool set. Historical release manifests and the private book transcript are not
release-copy targets and should remain unchanged.

## Required content changes

| Priority | Live file | Current claim | Required truth for 0.14.2 |
| --- | --- | --- | --- |
| P0 | `src/pages/how_it_works.rs:658` | EN/PL copy says ten tools and lists the pre-parity surface | Say twelve tools and list `context`, `repo-view`, `focus`, `slice`, `body`, `find`, `impact`, `diff`, `tree`, `follow`, `suppressions`, `prism`. Mention raw-text regex in `find`, unlimited default tree depth, and explicit depth/filter controls only if the page has room. |
| P0 | `src/components/cli_picker.rs:873` | Demo banner is pinned to `v0.12.4` and lists six tools | Replace the version with the release-derived value or omit the hard-coded version. List the twelve-tool surface; a compact two-line rendering is preferable to clipping names. |
| P1 | `docs/use-cases/33_agent_context_bootstrap.md:121` | Lists six MCP tools | Replace with the canonical twelve-tool list and describe `body`, raw-text `find(regex)`, and `diff` as MCP-native paths. |
| P1 | `public/llms-full.txt:292` | Lists seven MCP tools | Regenerate or edit from its canonical source so the published LLM corpus exposes the same twelve-tool contract. Do not patch only the generated file if a generator owns it. |

## Release-copy contract

The public wording should preserve the intentional boundary: reporting commands
such as `health`, `findings`, `audit`, and `coverage` remain CLI-only. MCP parity
here means parity for the library-backed agent workflow, not a promise that every
CLI command is an MCP tool.

The same-name parameter semantics must also be stated consistently where they
are documented:

- `tree` defaults to unlimited depth and accepts path, file-only, match, top,
  summary, hidden, ignored, and artifact controls;
- `slice` accepts `depth` and `rescan`;
- `impact` accepts `depth`;
- `focus` accepts `depth` and consumer inclusion controls;
- `find(mode="regex")` scans raw repository text rather than symbol names;
- `diff` compares the current Living Tree, including dirty changes, with a git
  base such as `HEAD~1`.

## Integration boundary

Two required targets are already dirty in the sibling checkout:
`docs/use-cases/33_agent_context_bootstrap.md` and `src/pages/how_it_works.rs`.
Those edits belong to an active broader content/IDE cut. Apply the MCP wording by
merging into those live changes; do not restore either file from `HEAD`.
`src/components/cli_picker.rs` and `public/llms-full.txt` were clean at audit
time, but the whole repository was not clean, so re-read all four files before
editing and stage them narrowly.

Before publishing the website, verify the rendered EN and PL how-it-works page,
the CLI picker at mobile width, and the final `public/llms-full.txt`. A literal
search for `Ten tools`, `Dziesięć narzędzi`, and the old six/seven-tool lists
must return no current product-copy hits.
