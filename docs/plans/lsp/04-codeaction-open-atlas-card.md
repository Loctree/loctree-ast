---
name: codeaction-open-atlas-card
status: queued
agent_target: any
project: loctree-suite
priority: 4
created: 2026-05-05
parent_branch: feat/context-tool-alpha
depends_on: 01-atlas-per-repo
---

# Plan 4 — Diagnostic `codeAction` "Open in Atlas Card"

## Why

Today loctree-lsp emits diagnostics for dead exports, cycles, and twins
(see `loctree-lsp/src/diagnostics/{dead,cycles,twins}.rs`). Each diagnostic
is a leaf — agents see the warning but have no link back to broader context.

Under the AI-engine paradigm, every diagnostic should expose a code action
that resolves to a **pointer** (atlas card path) rather than inline content.
This lets the agent jump from "warning at line 42" to the relevant card on
disk in one round-trip:

- `dead_export` → open `02-runtime-map.md` (runtime semantics for the symbol).
- `circular_import` → open `01-structural-map.md` (structural deps).
- `twin` → open `01-structural-map.md` plus a quick command to copy
  `loct find <symbol>` to clipboard (use existing `data-copy` pattern in
  the HTML report).

## Acceptance criteria

- [ ] Existing `loctree-lsp/src/actions/quickfix.rs` and
  `actions/refactor.rs` get a sibling
  `loctree-lsp/src/actions/atlas_card.rs` that produces
  `Vec<CodeAction>` keyed off the diagnostic's `code` field.
- [ ] Code action uses `command` form with method
  `"loctree.openAtlasCard"` and arguments
  `{ atlas_dir, card_path, reason }`.
- [ ] LSP server registers the executeCommand handler with method
  `"loctree.openAtlasCard"` — implementation is a no-op on the server
  side (client opens the file). Server only validates that the card
  exists on disk and returns success.
- [ ] Mapping table:
  `dead_export` → `02-runtime-map.md`,
  `circular_import` → `01-structural-map.md`,
  `lazy_circular_import` → `01-structural-map.md`,
  `dead_parrot` (twin with 0 imports) → `02-runtime-map.md`,
  `exact_twin` → `01-structural-map.md`.
- [ ] If atlas is missing (per Plan 1's response shape), code action is
  not emitted (server is silent rather than offering a broken link).
- [ ] Unit test in `atlas_card.rs` covers the mapping and missing-atlas
  case.

## Files to modify

- `loctree-lsp/src/actions/atlas_card.rs` (NEW) — provider.
- `loctree-lsp/src/actions/mod.rs` — register the new submodule.
- `loctree-lsp/src/backend.rs` — add `loctree.openAtlasCard` to
  `execute_command_provider.commands`. Wire dispatch.
- `editors/vscode/src/commands.ts` — register
  `loctree.openAtlasCard` to call `vscode.window.showTextDocument(uri)`
  on the resolved card path.

## Implementation sketch

```rust
// loctree-lsp/src/actions/atlas_card.rs
use tower_lsp::lsp_types::{CodeAction, CodeActionKind, Command};

pub fn atlas_card_action(diag: &Diagnostic, atlas_dir: &Path) -> Option<CodeAction> {
    let card_filename = match diag.code.as_ref()? {
        NumberOrString::String(c) if c == "dead_export" || c == "dead_parrot" => "02-runtime-map.md",
        NumberOrString::String(c) if c.contains("circular") => "01-structural-map.md",
        NumberOrString::String(c) if c == "exact_twin" => "01-structural-map.md",
        _ => return None,
    };
    let card_path = atlas_dir.join(card_filename);
    if !card_path.exists() { return None; }

    Some(CodeAction {
        title: format!("Open Context Atlas card: {}", card_filename),
        kind: Some(CodeActionKind::QUICKFIX),
        command: Some(Command {
            title: "Open Atlas Card".into(),
            command: "loctree.openAtlasCard".into(),
            arguments: Some(vec![serde_json::json!({
                "card_path": card_path.display().to_string(),
                "diagnostic_code": diag.code,
            })]),
        }),
        ..Default::default()
    })
}
```

## Verification

```bash
make precheck
cargo test -p loctree-lsp actions::atlas_card
# Manual: introduce a dead export in a fixture file, verify the
# "Open Context Atlas card: 02-runtime-map.md" action appears in
# VS Code's lightbulb menu.
```

## Exit contract

- COMMIT: `feat(lsp): codeAction Open Atlas Card on diagnostics`.
- REPORT: `.vibecrafted/reports/lsp/04-codeaction-open-atlas-card.md`.
- DEPENDS: Plan 1 (atlas-per-repo) must land first.

## Non-goals

- No card content embedding in the action — pure pointer.
- No automatic atlas regeneration — if atlas is missing, no action is
  offered (silent — agent will see Plan 2's missing-status).

Vibecrafted with AI Agents (c)2024-2026 The LibraxisAI Team
