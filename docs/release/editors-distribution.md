# Editor Distribution Architecture

Status: **design-locked** (2026-06-06). Authoritative rules for how the Loctree
editor plugins (VS Code, JetBrains, Neovim) are sourced, versioned, built, and
shipped, and how the commercial `loctree-lsp` binary reaches them.

## Principles

- **Canonical source: private `loctree-suite`.** It stays PRIVATE and moves from
  BUSL/BSL to a commercial license. It builds `loctree-lsp` and holds the
  canonical plugin source under `editors/{vscode,jetbrains,nvim}`.
- **Public editor repos are publish targets, not forks.** Carved out via
  `cp + sync`, never `move`. The source of truth is never split — this bounds drift.
- **Separate public repo per ecosystem** (not one `loctree-editors`):
  - `Loctree/loctree-vscode`    — MIT shell, syncs from `editors/vscode`
  - `Loctree/loctree-jetbrains` — MIT shell, syncs from `editors/jetbrains`
  - `Loctree/loctree-nvim`      — MIT shell, syncs from `editors/nvim`

  Each ecosystem has its own distribution, CI, marketplace, changelog, packaging,
  and user expectations.
- **Binary home: public `Loctree/loctree-release`.** Single place for per-platform
  binaries (`loctree-lsp`, `loct`, `loctree`, `loctree-mcp`, ...), versioned by tag.
- **License split:** MIT on the plugin shells (source may be published), commercial
  on the binary (we publish the **binary**, never the LSP **source**).

## Hard rules

1. **Same-version, always.** `loctree-lsp vX.Y.Z` == VSIX `vX.Y.Z` == JetBrains ZIP
   `vX.Y.Z`. A tagged release build that mixes versions is a release bug.
2. **No `latest` in tagged release builds.** Plugin CI pulls the binary by exact
   tag (`vX.Y.Z`) from `loctree-release`, never `latest`. A CI assert refuses the
   build when the plugin version has no matching `loctree-release` version.
3. **Binaries are never committed to git history.** They live only as GitHub
   Release assets. (Committing the 23 MB Mach-O once already bit us — see the
   `public-dist-committed-binary` finding.)
4. **Bundle-at-build is primary; runtime download is fallback.**

## Primary mechanism — bundle-at-build

```
tag vX.Y.Z  →  CI downloads loctree-lsp vX.Y.Z from loctree-release
            →  bundles the binary into the per-platform VSIX / ZIP
            →  publishes the artifact to the marketplace
```

This is the primary path because it gives the best UX: the user installs the
plugin and it works — no runtime 404, no tokens, no private-repo access, no
"first run must download something."

## Fallback mechanism — runtime download

The runtime download tier (`editors/vscode/src/client.ts`, JetBrains equivalent)
is retained but **explicitly secondary**. Its roles:

- **Self-heal** if the bundled binary goes missing or is corrupted.
- **Dev / unpacked** path (running from source without a packaged artifact).
- **Foundation** for the future gated / licensed flow (license-key gating is a
  later PR cycle; today the binary is free to download).

When implemented, it points at `Loctree/loctree-release`, resolves the binary by
**exact tag** (not `latest`), verifies the `.sha256` sidecar, and fails
gracefully (fall through to PATH) when no matching asset exists for the platform.

## Release flow

1. Work in `loctree-suite` (canonical). Open a PR.
2. PR CI builds the plugins as a **check** — green is visible, artifacts are
   **not** auto-attached to the merge.
3. Merge when green.
4. Cut a tag `vX.Y.Z` (manual, after eyeballing the green plugin builds). The tag
   triggers the Release workflow.
5. Release workflow:
   a. Build `loctree-lsp` per-platform → publish to `loctree-release vX.Y.Z`.
   b. `cp`-sync `editors/*` → the public plugin repos.
   c. Build per-platform VSIX / ZIP bundling the same-version binary.
   d. Publish to Open VSX / VS Code Marketplace / JetBrains Marketplace.

Release trigger: manual dispatch **or** PR-trigger for the build check; the **tag**
is the publish trigger.

## Current-state gaps (2026-06-06)

| # | Gap | Owner |
|---|-----|-------|
| 1 | ~~`loctree-release` must publish `loctree-lsp` per-platform, same-version~~ **Closed 2026-08-18** — `release-bundles.yml` now publishes the release on a `v*` tag push. See *Published asset contract* below. | release-side |
| 2 | Create the 3 public plugin repos + initial cp-sync | operator + agent |
| 3 | `sync-public-editors.sh` (one-way suite → public) | agent |
| 4 | Per-platform VSIX/ZIP build + same-version assert in CI | agent + release-side |
| 5 | Repoint `client.ts` download tier at `loctree-release` (exact tag, as fallback) | agent |
| 6 | `loctree-suite` license change BUSL → commercial | operator (legal) |
| 7 | JetBrains `verifyPlugin` is compatible but noisy: current IU verifier runs emit layout/classPath warnings and one bundled `DatabaseTools` read warning. Treat as release-readiness drift until the IDE matrix or verifier input is made clean. | agent + JetBrains-side |
| 8 | Parser-engine dependency drift: `tree-sitter` and `oxc_*` are core engine dependencies, not optional tooling. Update them under a dedicated parser regression pass, not as an incidental package bump. | agent |

## Published asset contract

`Loctree/loctree-release` carries **two asset shapes with different consumers**.
Both are published by `.github/workflows/release-bundles.yml` job
`publish-loctree-release`, which fires on a `v*.*.*` tag push. The names below
are a contract: the plugin code builds them by string concatenation, so a
mismatch is a 404 at install time, not a warning.

### Shape A — combined suite archive

| | |
|---|---|
| Name | `loctree-<version>-<triple>.tar.gz` (+ `.sha256`, `.sig` when signed) |
| Internal path | `loctree-<version>-<triple>/bin/loctree-lsp` — **part of the contract** |
| Producer | `distribution/build-bundle.sh` via the `build` matrix |
| Consumers | `editors/vscode/scripts/fetch-release-lsp.js` (`archiveName`, `extractedBinary`) at VSIX packaging time; `editors/vscode/src/client.ts` (`archiveAssetName`, `archiveRootDir`) at runtime |

Published triples: `aarch64-apple-darwin`, `x86_64-apple-darwin`,
`x86_64-unknown-linux-gnu`, `x86_64-unknown-linux-musl-core`.

### Shape B — raw per-platform `loctree-lsp`

| | |
|---|---|
| Names | `loctree-lsp-darwin-arm64`, `loctree-lsp-darwin-x64`, `loctree-lsp-linux-x64` (each + `.sha256`) |
| Sidecar format | `<sha256>  <name>` — every consumer reads whitespace field 1 of line 1 |
| Producer | **extracted from the shape-A archive**, not rebuilt (see below) |
| Consumers | JetBrains `PlatformAsset.assetName()` + `ReleaseDownloader` (fail-closed on the sidecar); `editors/vscode/src/client.ts` `lspAssetNameForTarget()` for its cache-version marker |

Shape B is *extracted* from the bundle that was just built, verified and signed
— never built a second time. Two reasons: the macOS binaries inside the bundle
are already codesigned, and a second `cargo build` would make "same version"
a claim rather than a property. `publish-loctree-release` asserts the raw asset
hashes byte-identical to `<root>/bin/loctree-lsp` in the matching archive, so
the runtime JetBrains downloads and the runtime the VSIX packs cannot drift.

### Platform coverage, stated honestly

`linux-arm64` and `windows-x64` have **no published `loctree-lsp` asset** and
none is produced. The JetBrains resolver returns `null` for them and surfaces an
unsupported-platform notice; `vscode-extension.yml` omits those lanes. A name
that 404s is worse than an honest absence — do not add either name to a plugin
before the corresponding asset exists here.

`x86_64-apple-darwin` (Intel macOS) is a special case. AICX publishes no release
asset for that triple (verified against `Loctree/aicx` v0.7.3), so its bundle can
only ever be `core` — Loctree binaries without a bundled AICX. But
`fetch-release-lsp.js` hard-codes the plain archive name and v0.13.1 shipped it
under that name, so `release-bundles.yml` passes `--bundle-suffix ""` to keep the
published contract. The bundle's own `README.md` and `components.json` still
declare AICX unbundled, and the workflow's verify step asserts they do.

`public_dist/install.sh` is **not** a consumer of the Intel archive: its
`target_triple()` returns empty for `darwin:x86_64` and
`unsupported_platform_reason()` documents that gap explicitly. Now that the
triple is published, enabling it in the installer is a separate, deliberate
operator decision.

### Cross-repo credential

`publish-loctree-release` writes to a different repository, so it uses
`HOMEBREW_GITHUB_API_TOKEN` — the cross-repo token this repo already holds for
release and tap writes (`docs/release/README.md` §5, also used by `publish.yml`
and `homebrew-release.yml`). No new secret was introduced. The job's first step
fails loudly with an actionable message when the token is absent; it never skips
silently, because a silently missing release is exactly the failure that left
v0.14.x unshippable.

Re-running a tag build is safe and non-destructive: an asset already on the
release is compared by sha256. Identical bytes are skipped; **different** bytes
are a hard failure with the two digests and the explicit `gh release
delete-asset` command needed to override. A published, verified asset is never
overwritten implicitly.

## Implementation notes

- Release asset is a tarball `loctree-<ver>-<triple>.tar.gz`; `loctree-lsp` lives
  at `<dir>/bin/loctree-lsp`. Extraction shells out to `tar -xzf`. The bare
  `loctree-lsp-<os>-<arch>` asset that used to be listed here as an optional
  improvement now ships (shape B above).
- Per-asset `.sha256` sidecars exist and match the existing client-side
  verification (`downloadFile` + `isCachedBinaryVerified`).
- `SHA256SUMS` is generated by the publish job as real `sha256sum -c` input.
  The v0.13.1 `SHA256SUMS` carried bare file names and no digests, and omitted
  the Intel archive entirely — it was unverifiable.
- Commit `00780608` set the VS Code download default to `Loctree/loctree-suite`
  (private) — wrong for end users; it must become `Loctree/loctree-release`.
