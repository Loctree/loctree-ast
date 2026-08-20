# Installation

End users install **prebuilt binaries**. You do not need a Rust toolchain, you do not need to clone this repo, and you
do not need `cargo build`. If you want to contribute or build from source,
see [dev/01_installation.md](./dev/01_installation.md).

## Pick a path

| Path                                            | Gets you                                                                                                                                                  | Best for                            |
|-------------------------------------------------|-----------------------------------------------------------------------------------------------------------------------------------------------------------|-------------------------------------|
| `curl -fsSL https://loct.io/install.sh \| bash` | Signed bundle: `loct`, `loctree`, `loctree-mcp`, `loctree-lsp`, `aicx`, `aicx-mcp` (GPG-verified).                                                        | Most users                          |
| `npm install -g @loctree/loctree`                  | One install: `loct`, `loctree`, `loctree-mcp`, `loctree-lsp`                                                                                               | Node-first toolchains, editor users |
| Homebrew                                        | `loct` today; MCP/LSP formulae follow the thin-repo release tracks                                                                                         | macOS / Linux Homebrew users        |
| `cargo install --locked loctree`                | crates.io 0.13.0 core analyzer + `loct`/`loctree` CLIs                                                                                                     | Rust toolchain users                |
| `cargo add loctree@0.13.0`                      | library dependency for Rust integrations                                                                                                                   | Rust library consumers              |

## curl install (recommended)

```bash
curl -fsSL https://loct.io/install.sh | bash
```

The installer downloads the prebuilt tarball for your platform, verifies the SHA256 + GPG signature, and drops binaries
into `~/.local/bin`. Override targets via `INSTALL_DIR` and `LOCTREE_VERSION` env vars.

The 0.13.0 bundle contract ships `loct`, `loctree`, `loctree-mcp`, `loctree-lsp`, `aicx`, and `aicx-mcp` in the full
target bundles. The `x86_64-unknown-linux-musl-core` bundle carries the static Loctree binaries and marks AICX as an
optional runtime dependency because AICX does not publish a static musl asset.

Smoke-test the install:

```bash
loct --version
loctree --version
loctree-mcp --version
loctree-lsp --version
aicx --version
aicx-mcp --version
```

## npm

```bash
npm install -g @loctree/loctree
```

`@loctree/loctree` installs the **Loctree runtime** — `loctree` (runtime) and `loct` (short alias) plus sibling
`loctree-mcp` and `loctree-lsp` binaries inside the selected platform package. It uses `optionalDependencies` to install
only the platform-specific delivery package. Supported platforms today: `darwin-arm64`, `darwin-x64`, `linux-x64-gnu`,
`win32-x64-msvc`.

Smoke-test:

```bash
loctree --version
loct --version
loctree-mcp --version
loctree-lsp --version
```

> AICX (`aicx` / `aicx-mcp`) is owned by the sibling [`Loctree/aicx`](https://github.com/Loctree/aicx) repo and
> publishes separately as `@loctree/aicx`. The full `curl | bash` bundle includes both AICX binaries too.

## Homebrew

Install via the official Homebrew taps:

```bash
brew install loctree/cli/loct
```

The MCP/LSP Homebrew tracks are documented but should not be treated as published channels until their thin-repo syncs
land. Use npm or the signed bundle for `loctree-mcp` / `loctree-lsp` in the meantime. See
[01_homebrew_release.md](./01_homebrew_release.md) for the active release shape.

## Which binary do I use?

| Binary        | Use when you want…                                                           |
|---------------|------------------------------------------------------------------------------|
| `loct`        | A compact operator CLI for everyday scans, slices, impact, health.           |
| `loctree`     | The full analyzer / reporting CLI (legacy long name; superset of `loct`).    |
| `loctree-mcp` | An MCP server so AI agents (Claude Code, Codex, Cursor) can query your repo. |
| `loctree-lsp` | An LSP server so editors get live structural diagnostics.                    |
| `aicx`        | Capture/retrieve agent-session intentions from past conversations.           |
| `aicx-mcp`    | MCP bridge so agents can read AICX intention store.                          |

You don't need all six — pick the surface you actually use.

## C-Family Deep-Mode (Opt-In)

Release binaries currently provide base C-family extraction (heuristic Layer 1) out of the box. To access compiler-grade deep-index features for Swift and Objective-C, you must build from source with the deep-mode feature flags:

```bash
cargo build --release --features deep-index,deep-index-macos
```
This enables SCIP (`deep-index`) and IndexStore (`deep-index-macos`) integrations.

## MCP server setup

Once `loctree-mcp` is on your `PATH`, wire it into your MCP-capable client:

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

Ten tools: `context`, `repo-view`, `focus`, `slice`, `find`, `impact`, `tree`, `follow`, `suppressions`, and `prism`.

## LSP server setup

`loctree-lsp` speaks plain LSP over stdio. Editor-specific recipes live in [docs/ide/](./ide/):

- [VS Code](./ide/vscode.md)
- [Neovim](./ide/neovim.md)
- [Generic LSP protocol notes](./ide/lsp-protocol.md)

## License Notice

Loctree is licensed under the Business Source License 1.1 (BUSL-1.1). See the root [LICENSE](../LICENSE) file for full
parameters.

**Additional Use Grant Summary:**
The license restricts hosted-service redistribution that competes with Loctree-provided services. You are free to use,
modify, and distribute the software for other purposes, subject to the conditions outlined in the LICENSE file. It will
convert to the Apache License 2.0 on 2030-04-13.
