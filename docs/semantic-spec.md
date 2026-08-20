# Loctree Semantic Spec — Cut 3A

## Doctrine

(Recap from LOCTREE_NEXT.md — 4-layer model, parser-as-sensor, semantic
analyzer as product. ~30 lines.)

## C-Family Support (Swift, Objective-C, C, C++)

Tree-sitter provides Layer 1 extraction for C-family languages.
- **What is extracted:** `symbol_graph` elements, basic imports, and declarations (e.g., `@interface`, `@implementation`, `class`, `protocol`).
- **Provenance/confidence honesty:** Basic parsing relies on heuristic provenance. Relationships like `NotificationCenter` and `@selector` bridges are inferred heuristically.
- **Deep-mode tiers:** Deep-index features are available via SCIP (`deep-index`) and IndexStore (`deep-index-macos`) for exact compiler-grade provenance, requiring manual opt-in at build time.

## Shell idiom catalog (~25 entries)

For each idiom, document:

| name | aliases | classifier | runtime_role | reasoning |
|------|---------|------------|--------------|-----------|
| usage | (none) | HelpPrinter | UserFacing | CLI help-text printer; reachable via --help or no-args. |
| die | abort, fail | ErrorExit | LibraryHelper | Bail-out helper. |
| main | (none) | PrimaryEntrypoint | PrimaryEntrypoint | Script entrypoint sourced last. |
| info | _info | LibraryHelper | LibraryHelper | Conventional info logger. |
| warn | _warn | LibraryHelper | LibraryHelper | Conventional warning logger. |
| error | _error, _err | LibraryHelper | LibraryHelper | Conventional error logger. |
| log | _log | LibraryHelper | LibraryHelper | Generic log dispatcher. |
| cleanup | (none) | LibraryHelper | LibraryHelper | Trap handler / signal cleanup. |
| trap_handler | (none) | DispatchHandler | LibraryHelper | Reachable via `trap` builtin. |
| have | _have | LibraryHelper | LibraryHelper | `command -v`-style existence check. |
| exists | _exists | LibraryHelper | LibraryHelper | Variant of `have`. |
| banner | _banner | LibraryHelper | LibraryHelper | Decorative output. |
| success | _success, _ok | LibraryHelper | LibraryHelper | Convention. |
| confirm | _confirm | LibraryHelper | LibraryHelper | Interactive prompt. |
| require | _require | LibraryHelper | LibraryHelper | Hard dependency check. |
| PATH | (none) | EnvVar | EnvInput | Standard env var. |
| HOME | (none) | EnvVar | EnvInput | Standard env var. |
| USER | (none) | EnvVar | EnvInput | Standard env var. |
| SHELL | (none) | EnvVar | EnvInput | Standard env var. |
| TERM | (none) | EnvVar | EnvInput | Standard env var. |
| PWD | (none) | EnvVar | EnvInput | Standard env var. |
| EDITOR | (none) | EnvVar | EnvInput | Standard env var. |
| PAGER | (none) | EnvVar | EnvInput | Standard env var. |
| LANG | (none) | EnvVar | EnvInput | Standard env var. |
| LC_ALL | (none) | EnvVar | EnvInput | Standard env var. |

(25 total. Aliases share classifier and runtime_role with the canonical name.)

## Make idiom catalog (~10 entries)

| name | classifier | runtime_role | reasoning |
|------|------------|--------------|-----------|
| .PHONY | Metadata | Metadata | GNU Make directive; targets after .PHONY are public. |
| all | PublicEntrypoint | PublicEntrypoint | Default target invoked by bare `make`. |
| install | PublicEntrypoint | PublicEntrypoint | Conventional install target. |
| clean | PublicEntrypoint | PublicEntrypoint | Conventional clean target. |
| test | PublicEntrypoint | PublicEntrypoint | Conventional test target. |
| precheck | PublicEntrypoint | PublicEntrypoint | Vetcoders convention: quick explicit validation gate. |
| build | PublicEntrypoint | PublicEntrypoint | Conventional build target. |
| release | PublicEntrypoint | PublicEntrypoint | Conventional release target. |
| .DEFAULT | Metadata | Metadata | GNU Make special target. |
| .SUFFIXES | Metadata | Metadata | GNU Make special target. |

## Expected behavior in findings

- A shell symbol with idiom_tag `HelpPrinter` MUST NOT appear in dead_parrots high_confidence.
- A make target listed under `.PHONY` MUST NOT appear in dead_parrots regardless of import count.
- A function reachable through case dispatch (DispatchEdge present) MUST be marked reached, even with no static `import` edge.
- An env contract symbol (PATH, HOME) MUST NOT appear in dead_parrots; classified as EnvInput.
- Twin detection MUST treat two functions sharing only an idiom body shape (e.g. two `usage` printers) as legitimate duplicates; not as a refactoring signal.

## Override semantics

User can override or extend via `<workspace>/.loctree/idioms/{shell,make}.toml`. Override matches by `name` field (case-sensitive). Adding a new entry adds; redefining replaces.
