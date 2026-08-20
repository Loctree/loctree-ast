# Loctree Suite — Contributor Build Guide

This document is for **contributors building from source**. End users should use the prebuilt binary paths in [docs/installation.md](../installation.md) instead.

## Quick contributor setup

```bash
git clone https://github.com/Loctree/loctree-suite.git
cd loctree-suite
make install        # build + install loct, loctree, loctree-mcp, loctree-lsp into ~/.cargo/bin
```

Verify:

```bash
loct --version
loctree --version
loctree-mcp --version
loctree-lsp --version
```

## What gets built

### Binaries shipped from this monorepo

| Binary | Crate | Description |
|--------|-------|-------------|
| `loct` | `loctree-rs` | Compact operator CLI (recommended for daily use) |
| `loctree` | `loctree-rs` | Full analyzer/reporting CLI (legacy long name; superset of `loct`) |
| `loctree-mcp` | `loctree-mcp` | MCP server for AI agents |
| `loctree-lsp` | `loctree-lsp` | LSP server for editors |

### Binaries bundled by `loct.io/install.sh` but sourced externally

`aicx` and `aicx-mcp` are distributed as part of the signed `loct.io` release tarball but their source is not in this workspace. Cargo-only flows (`cargo install`, `make install`) do not produce them — use `curl -fsSL https://loct.io/install.sh | bash` to get the full bundle.

## Build methods

### 1. Makefile (recommended for contributors)

```bash
make install        # build + install all binaries from this workspace
make precheck       # fmt + clippy + check (run before push)
make preflight      # full explicit validation before a PR or release
make test           # run all workspace tests
```

### 2. Cargo direct

> **Install root matters.** `make install` writes to `~/.local/bin`
> (`CARGO_INSTALL_ROOT ?= $(HOME)/.local`), and `/usr/local/bin/loct` is a symlink
> into that directory. A bare `cargo install --path` writes to `~/.cargo/bin`
> instead, which `~/.local/bin` shadows on a default PATH — the build succeeds and
> links, but your shell keeps running the older copy. Pass `--root` to stay on the
> canonical path.

```bash
# All binaries from this workspace (same root as `make install`)
cargo install --root "$HOME/.local" --path loctree-rs --bins
cargo install --root "$HOME/.local" --path loctree-mcp
cargo install --root "$HOME/.local" --path loctree-lsp

# Or from crates.io (legacy crate-name contract; thin-release npm/Homebrew uses
# loct/loct-mcp/loct-lsp naming instead)
cargo install loctree
cargo install loctree-mcp
```

Confirm which build you actually run. The version banner carries the commit sha and
is the only thing that separates a fresh install from a stale one:

```bash
command -v loct        # expect ~/.local/bin/loct on a default PATH
loct --version         # loct <version>+g<sha> ... commit=<sha> dirty=false
```

### 3. Workspace dev cycle

```bash
cargo build --workspace                # debug build
cargo build --workspace --release      # release build
cargo test --workspace                 # all tests
cargo clippy --workspace -- -D warnings
```

## Workspace structure

```
├── loctree-ast/         # tree-sitter AST extractor surface (cross-language)
├── loctree-rs/          # core library + CLI binaries (loct, loctree)
├── loctree-mcp/         # MCP server crate (loctree-mcp binary)
├── loctree-lsp/         # LSP server crate (loctree-lsp binary)
├── rmcp-common/         # shared MCP plumbing
├── reports/             # Leptos-based HTML report renderer (lib + wasm)
├── distribution/        # npm wrappers, codesigning, Homebrew formulas
└── editors/             # VS Code extension, Neovim plugin
```

## MCP server config

Once installed, wire `loctree-mcp` into your MCP-capable client:

```json
{
  "mcpServers": {
    "loctree": {
      "command": "loctree-mcp",
      "args": []
    }
  }
}
```

See `docs/dev/.TL_DR/00_mcp_quickstart.md` for the full multi-client setup.

## Makefile targets

```bash
make install        # install loct, loctree, loctree-mcp, loctree-lsp
make build          # release build of all crates
make test           # run all tests
make check          # cargo check
make fmt            # cargo fmt --all
make clean          # clean build artifacts
make precheck       # quick explicit fmt + clippy + check
make preflight      # full validation; intentionally not an automatic hook
make git-hooks      # enable lightweight pre-commit and commit-msg hooks

# MCP infrastructure (optional, advanced)
make mcp-build      # build all MCP servers
make mcp-install    # install all MCP servers

# Multiplexer infrastructure (operator-only; not required for normal contributor flow)
make mux-setup
make mux-status
make mux-tui
```

`make git-hooks` is an explicit repository-configuration step. It copies a
source-commit-addressed snapshot of only the lightweight, offline-friendly `pre-commit` and
`commit-msg` hooks to `<git-common-dir>/loctree-hooks/<source-commit>` and sets
`core.hooksPath` to that absolute, immutable snapshot. All linked worktrees
therefore run the same installed code; switching to an older branch cannot
change it. Hook sources must match their source commit. The installed directory
never contains a `pre-push` hook, so full validation remains opt-in through
`make preflight`. Each existing worktree also pins the snapshot in
`config.worktree`, while the common config retains the same safe fallback for
newly created worktrees. This prevents an obsolete installer that rewrites the
common value from reactivating branch-controlled hooks in an existing checkout.

The installer refuses to shadow a global/system hook policy, replace a foreign
`core.hooksPath`, disable an additional tracked hook, or reuse a modified
snapshot. Resolve such a policy explicitly instead of silently dropping or
chaining third-party hooks. Recognized legacy
Loctree symlinks are removed during migration. Binary installation (`make
install` / `make install-all`) does not alter Git hooks.

The full preflight runs the hook and Git-environment regression suite before its
workspace checks. The Makefile removes Git's repository-local environment from
every recipe,
including test prerequisites and publish steps. `make test` and `make preflight`
also capture the current worktree root and clear that environment in their shell
wrappers as defense in depth, so fixture repositories cannot accidentally target
the caller's shared Git directory.

## Platform Support

| Platform | Status | Notes |
|----------|--------|-------|
| macOS Apple Silicon | Full | Primary development platform |
| macOS Intel | Full | Tested |
| Linux x86_64 | Full | Tested |
| Linux ARM64 | Full | Tested |
| Windows x64 | Partial | WSL recommended |

The npm wrappers and Homebrew taps ship the same four target triples.

## Version Management

```bash
make version-show                                   # current versions
make version TYPE=patch SCOPE=all                   # bump
make version TYPE=patch SCOPE=all TAG=1             # bump + tag
make version TYPE=patch SCOPE=all TAG=1 PUSH=1      # bump + tag + push (triggers release)
```

A tag push triggers the cascade documented in [01_homebrew_release.md](../01_homebrew_release.md): crate publish → binary builds → thin-release uploads → npm publish → Homebrew tap sync.

## Troubleshooting

### Build lock conflict

```
Another build running (PID xxxx). Aborting.
```

Resolve:

```bash
make unlock
```

### Cargo install conflicts

If you have both crates.io and a local source build:

```bash
cargo uninstall loctree
make install
```

## Updating

```bash
git pull
make install
```

## Uninstalling

```bash
rm ~/.cargo/bin/loct
rm ~/.cargo/bin/loctree
rm ~/.cargo/bin/loctree-mcp
rm ~/.cargo/bin/loctree-lsp
rm -rf ~/.rmcp_servers   # only if you ran the multiplexer
```

---

𝚅𝚒𝚋𝚎𝚌𝚛𝚊𝚏𝚝𝚎𝚍. with AI Agents by Vetcoders (c)2024-2026 LibraxisAI
