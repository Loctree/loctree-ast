# Loctree Suite — Developer Documentation

Technical documentation for contributors and packagers. **End users should start at the project [README.md](../../README.md) or [docs/installation.md](../installation.md), not here.**

## Contents

| Document                           | Description                                  |
|------------------------------------|----------------------------------------------|
| [INSTALLATION.md](INSTALLATION.md) | Complete installation guide with all methods |
| [ARCHITECTURE.md](ARCHITECTURE.md) | Workspace structure and crate relationships  |
| [BINARIES.md](BINARIES.md)         | CLI reference for all binaries               |

## Quick Links

### Building from source

```bash
git clone https://github.com/Loctree/loctree-suite.git
cd loctree-suite
make install        # builds + installs loct, loctree, loctree-mcp, loctree-lsp
```

### Active crates

| Crate           | Path           | Description                                     |
|-----------------|----------------|-------------------------------------------------|
| `loctree`       | `loctree-rs/`  | Core library + CLI binaries (`loct`, `loctree`) |
| `loctree-mcp`   | `loctree-mcp/` | MCP server (`loctree-mcp` binary)               |
| `loctree-lsp`   | `loctree-lsp/` | LSP server (`loctree-lsp` binary)               |
| `loctree-ast`   | `loctree-ast/` | Tree-sitter AST extractor surface               |
| `report-leptos` | `reports/`     | Leptos-based HTML report renderer               |

Workspace versions are pinned in `Cargo.toml` under `[workspace.package]`. Current line: `0.13.0`. `loctree` and
`report-leptos` publish to crates.io; `loctree-mcp` and `loctree-lsp` are in-tree/runtime crates with `publish = false`
and ship through binary distribution channels.

### Binaries shipped from this monorepo

| Binary        | Purpose                                        |
|---------------|------------------------------------------------|
| `loct`        | Compact operator CLI (recommended)             |
| `loctree`     | Full analyzer/reporting CLI (legacy long name) |
| `loctree-mcp` | MCP server for AI agents                       |
| `loctree-lsp` | LSP server for editors                         |

`aicx` and `aicx-mcp` are bundled by `loct.io/install.sh` but sourced outside this workspace.

## Development gates

```bash
cargo build --workspace
cargo test --workspace
cargo fmt --all
cargo clippy --workspace --all-targets -- -D warnings
```

Or shortcut via Makefile:

```bash
make precheck       # fmt + clippy + check
```

## Related Documentation

- [README.md](../../README.md) — project overview + user-facing install
- [docs/installation.md](../installation.md) — end-user install paths
- [docs/01_homebrew_release.md](../01_homebrew_release.md) — release engineering
- [CHANGELOG.md](../../CHANGELOG.md) — version history
- [CONTRIBUTING.md](../CONTRIBUTING.md) — contribution guide

---

𝚅𝚒𝚋𝚎𝚌𝚛𝚊𝚏𝚝𝚎𝚍. with AI Agents by Vetcoders (c)2024-2026 LibraxisAI
