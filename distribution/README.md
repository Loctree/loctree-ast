# Distribution Spine

This directory is the single source of truth for every release channel that is
not "cargo publish the crate and hope for the best."

## Channels

- `crates/`
  Rust crates.io release contract and publish notes.
- `homebrew/`
  Formula source, tap sync notes, and helper scripts.
- `npm/`
  Canonical npm wrapper and platform-package release flow.
- `macos/`
  Codesigning and direct-download bundle contract.
- `linux/`
  Linux release asset contract.
- `windows/`
  Windows release asset contract.

## Combined Release Bundles

The binary-first Loctree suite bundle is built by:

```bash
make release-bundles VERSION=0.14.4 AICX_VERSION=0.12.5
```

The same path is used by the GitHub Actions workflow
`Build Combined Release Bundles` (`.github/workflows/release-bundles.yml`) on
GitHub-hosted macOS ARM64, Linux x64, and Windows x64 runners.

Default tarball output:

```text
dist/release-bundles/<version>/
- loctree-<version>-aarch64-apple-darwin.tar.gz
- loctree-<version>-aarch64-apple-darwin.tar.gz.sha256
- loctree-<version>-aarch64-apple-darwin.tar.gz.sig
- loctree-<version>-x86_64-unknown-linux-gnu.tar.gz
- loctree-<version>-x86_64-unknown-linux-gnu.tar.gz.sha256
- loctree-<version>-x86_64-unknown-linux-gnu.tar.gz.sig
- loctree-<version>-x86_64-unknown-linux-musl-core.tar.gz
- loctree-<version>-x86_64-unknown-linux-musl-core.tar.gz.sha256
- loctree-<version>-x86_64-unknown-linux-musl-core.tar.gz.sig
- loctree-<version>-x86_64-pc-windows-msvc.tar.gz
- loctree-<version>-x86_64-pc-windows-msvc.tar.gz.sha256
- loctree-<version>-x86_64-pc-windows-msvc.tar.gz.sig
- loctree-lsp-windows-x64.exe
- loctree-lsp-windows-x64.exe.sha256
```

The `.sig` files are emitted when `LOCTREE_GPG_KEY_ID` or `--gpg-key` is set.
Full target tarballs carry `components.json`, `CHECKSUMS.sha256`, `README.md`,
and the six suite binaries: `loct`, `loctree`, `loctree-mcp`, `loctree-lsp`,
`aicx`, `aicx-mcp`.

Windows keeps the same combined `.tar.gz` archive naming as existing consumers,
with `.exe` on every staged binary. AICX release inputs are extracted from the
platform-native `.zip` on macOS/Windows and `.tar.gz` on Linux.

The musl tarball is deliberately named `-core`. It carries static Loctree
binaries plus `components.json`, `CHECKSUMS.sha256`, and `README.md`, but it
does not bundle `aicx` or `aicx-mcp` because AICX does not publish a static musl
release asset. Its metadata marks AICX as an optional runtime dependency.

Historical note: older public bundles shipped fewer binaries and missed
`loctree-lsp`. The 0.13.0 bundle contract above keeps `loctree-lsp` in the full
release set alongside AICX while the musl-core target stays Loctree-only plus an
optional AICX runtime dependency.

## Principle

One channel, one home.

Do not scatter release state across root-level `Formula/`, ad-hoc docs, and
half-remembered shell rituals. If a distribution path is real, it belongs in
`distribution/`.
