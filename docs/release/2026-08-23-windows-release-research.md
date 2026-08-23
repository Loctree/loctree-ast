# Windows-inclusive release research log — 2026-08-23

Status: in progress. This document records the release evidence before the
`v0.14.3` tag is created. Runtime and registry results will be added after the
tag-driven workflows finish.

## Research question

Can the canonical public Loctree release deliver the same combined Loctree +
AICX product on macOS, Linux, and Windows, with a real Windows client and editor
path, without publishing private operator data?

## Starting evidence

- `v0.14.2` could not finish the Linux musl lane because its smoke fixture was
  not a Git repository. PR #67 fixed the fixture and a Debian 12 container
  proved the static binary without installing Git in the container.
- The canonical combined-bundle workflow covered macOS and Linux only.
  Windows binaries existed in a separate legacy publish workflow, so the
  release repository and the new npm package family had no Windows artifact.
- A real Windows host had a local Cargo-built Loctree 0.14.2 binary, while its
  global npm installation remained on the legacy 0.13.1 package. That is
  installation split-brain, not Windows release support.
- AICX source was already 0.12.3, but its latest public release was 0.12.1.
  Loctree full bundles must therefore fail closed until AICX 0.12.3 assets are
  published.

## Release shape selected

| Platform | Loctree bundle | AICX in bundle | Raw LSP asset |
|---|---|---|---|
| macOS arm64 | full | yes | `loctree-lsp-darwin-arm64` |
| macOS x64 | core | no matching AICX asset | `loctree-lsp-darwin-x64` |
| Linux x64 GNU | full | yes | `loctree-lsp-linux-x64` |
| Linux x64 musl | core | no matching AICX asset | none |
| Windows x64 MSVC | full | yes | `loctree-lsp-windows-x64.exe` |

The Windows combined archive remains `.tar.gz`, matching the existing Loctree
bundle contract. The AICX input is its canonical Windows slim `.zip`. The raw
LSP executable is extracted from the verified combined archive, not rebuilt,
so editor and CLI distribution cannot drift by bytes.

## Implemented evidence

- Public Loctree PR #68 added the Windows hosted build, all-six-binary smoke,
  committed Git fixture scan, `.exe`-aware staging, raw LSP publication, and a
  focused synthetic bundle contract test. It merged as `2589ad0`.
- AICX PR #55 bound release and npm jobs to immutable tags, restored the
  merge-queue packaging matrix, routed credentialed signing to operator-owned
  runners, used built-in `tar.exe` for Windows npm extraction, enforced the
  Linux glibc 2.28 floor, and added cold npm install smoke on all three OSes. It
  merged as `8aac34c`.
- The public Loctree overlay was missing the entire JetBrains plugin even
  though Make, release docs, and version assertions treated it as shipped.
  The plugin source is restored from `loctree-suite/develop`, and Windows x64
  now resolves the raw `.exe` release asset with the existing fail-closed
  checksum verifier.

## Privacy gate

`vc-deprivatize` is required before every public merge. The Windows bundle
change introduced no private data. The restored JetBrains directory has no
CRITICAL or HIGH findings; its two REVIEW findings are the public product
support address and a deterministic test timestamp, both explicitly retained.

A separate `loctree-com` branch was rejected from the release path even though
its CI was green: compared with `origin/main`, it added raw conversation
extracts containing operator home paths, session identifiers, private people,
and a bundled archive. The branch was preserved for archaeology, but PR #15
was closed and will not be deployed. Site advancement will be rebuilt as a
narrow clean-main change after release assets exist.

## Remaining proof before tag and publish

1. Green AICX same-SHA packaging gate and signed 0.12.3 assets on macOS arm64,
   Linux x64 GNU, and Windows x64 MSVC.
2. Clean Loctree 0.14.3 version bump through the repository's own synchronized
   version contract.
3. Green tag-driven Loctree bundles, Windows smoke, npm cold installs, and
   checksum/signature verification.
4. Public installer and release registry advancement through a narrow
   deprivatized `loctree-com` PR and the VM deployment contract.
5. Real client verification on Windows plus macOS and Debian/Linux probes.
