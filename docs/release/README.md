# Loctree Suite — Release Runbook

Canonical ordering for shipping a `loctree-suite` version. Re-verified against
**0.14.2** on `release/0.14.2-rebuild`, 2026-08-18.

Related: [editors-distribution.md](editors-distribution.md) for the VS Code /
JetBrains artifact story.

---

## 0. State of play (read this first)

| Fact | Value |
|------|-------|
| Version in tree | `0.14.2` (`[workspace.package] version`, `Cargo.toml`) |
| Newest git tag | `v0.13.1` |
| Newest crates.io release | `0.13.1` (`loctree`, `loctree-ast`, `report-leptos`); `loctree-mcp` and `loctree-lsp` at `0.13.0` |
| npm | `@loctree/loct` `0.13.1` (legacy scoped name — canonical `@loctree/loctree` first ships at 0.14.2); bare `loctree` still on the legacy `0.8.16` line, deprecated 2026-08-20 |
| License | BUSL-1.1 (converts to Apache-2.0 on 2030-04-13); editor wrappers MIT |

`0.14.0`, `0.14.1` and `0.14.2` were all version-bumped in-tree; none was ever
tagged or published. A `v0.14.2` release therefore delivers all three, and
**no further bump is needed** — `make version-assert` already reports Cargo,
VS Code (package + lock), JetBrains and the web installer synced at `0.14.2`.
Bumping to 0.14.3 would only add a fourth never-shipped version.

---

## 1. Version contract

One source of truth: `[workspace.package] version` in `Cargo.toml`.

Everything else is either derived by a script or asserted by a gate:

| Surface | File(s) | Kept in sync by |
|---------|---------|-----------------|
| Rust crates | `*/Cargo.toml` (`version.workspace = true`) | inherited |
| VS Code extension | `editors/vscode/package.json` + lock | `make version-assert` |
| JetBrains plugin | `editors/jetbrains/**` | `make version-assert` |
| npm wrapper + 4 platform pkgs | `distribution/npm/loct/**/package.json` | `node distribution/npm/sync-version.mjs <ver>` |
| Homebrew formulae | rendered at release time | `scripts/render-homebrew-formula.sh` |
| Component mirrors | `distribution/component-manifests/*.manifest` | `distribution/component-sync.sh --version` |
| Web installer default | `public_dist/install.sh` (`VERSION=`) | `scripts/sync-version.sh` + `make version-assert` |

Verify:

```bash
make version-assert                                   # Cargo == editors == web installer
make version-check                                    # above + crates.io publish dry-run
node distribution/npm/sync-version.mjs --check 0.14.2 # npm wrapper + platform packages
```

The public installer is part of the enforced version contract. A release cannot
pass `version-assert` while `loct.io/install.sh`'s source default is stale.

---

## 2. Gates (all must be green before tagging)

```bash
make precheck    # cargo fmt --check + clippy -D warnings + cargo check
make test        # cargo test --workspace
make preflight   # full explicit workspace validation and dogfooding
make semgrep     # security gate — same rules as CI
make editors     # VS Code compile + Neovim smoke + JetBrains test/build
```

`make semgrep` runs:

```
semgrep scan --config auto --error --quiet \
  --exclude-rule html.security.audit.missing-integrity.missing-integrity .
```

The gate runs the same configs as the CI workflow (`auto`, `p/rust`,
`p/typescript`) and blocks on WARNING/ERROR; INFO audit rules surface in Code
Scanning for review. No rule is excluded. Override with `SEMGREP_CONFIGS`,
`SEMGREP_SEVERITY`, `SEMGREP_TARGET`.

`make git-hooks` explicitly installs an immutable, source-commit-addressed
snapshot of only the lightweight pre-commit and commit-message checks under the
repository's common Git directory. It refuses to shadow global/system policy,
replace an unknown `core.hooksPath`, or disable an additional hook; `make
install` and `make install-all` never alter hook policy. Heavy validation is deliberately
explicit: run `make preflight` before a PR or release. The preflight includes the
hook/isolation contracts, and Make removes repository-local Git environment
inherited from hooks or wrappers before every recipe, including test
prerequisites and publish steps. The test and preflight shell wrappers repeat
that isolation after capturing the current worktree root, keeping temporary
fixture repositories away from the caller's shared Git directory.
Existing linked worktrees pin the installed snapshot in their worktree config;
the common config carries the same safe fallback for future worktrees.

---

## 3. Version bump

```bash
make version VERSION=0.14.2          # or: make version TYPE=minor
```

Then write the `CHANGELOG.md` entry from `git log --oneline v<prev>..HEAD` and
re-run `make version-check`.

`make version` accepts `TAG=1 PUSH=1 PUBLISH=1`. **Do not use `PUSH=1`** unless
you intend to trigger CI immediately — see step 4.

---

## 4. Tag → what fires

Tagging is the button. Everything downstream keys off it.

```bash
git tag -a v0.14.2 -m "Release v0.14.2"
git push origin v0.14.2
```

| Workflow | Trigger | Does |
|----------|---------|------|
| `release-bundles.yml` | **push tag `v*.*.*`** (auto) | Builds combined Loctree+AICX tarballs per target, signs them |
| `publish.yml` | `workflow_dispatch` only — **manual** | crates.io cascade → per-platform CLI/MCP binaries → thin-repo releases → npm → monorepo GitHub release |
| `homebrew-release.yml` | `release: published` (auto) or manual | Renders formulae, syncs `Loctree/homebrew-cli` + `Loctree/homebrew-mcp` |
| `semgrep.yml` | push to `main`/`develop`; PR to `main`; weekly cron | SARIF upload; now on a GitHub-hosted runner |
| `ci.yml` | push to `main`/`develop`/`feat/substrate-scaffold`; internal PR to any base | Primary fmt, clippy, hook contracts, scorecard and tests on GitHub-hosted Linux + macOS |
| `loctree-ci.yml` | push/PR to `main`/`develop` | Self-analysis dogfooding; now on GitHub-hosted runners |

> **Not one button.** Pushing the tag only fires `release-bundles.yml`.
> `publish.yml` — the job that actually publishes crates, binaries and npm —
> is `workflow_dispatch` with a required `tag` input. It must be started by
> hand:
>
> ```bash
> gh workflow run publish.yml -f tag=v0.14.2
> ```
>
> `homebrew-release.yml` then fires automatically off the GitHub release that
> `publish.yml` creates at the end.

Full ordering:

```
gates → version bump → CHANGELOG → commit → tag → push tag
  ├─ (auto)   release-bundles.yml    → signed combined tarballs
  └─ (MANUAL) gh workflow run publish.yml -f tag=vX.Y.Z
        ├─ verify-release      (Cargo version == npm version == tag)
        ├─ publish-crates      loctree-ast → report-leptos → loctree → loctree-mcp
        ├─ build-cli-*         linux / macos-arm64 / macos-x86_64 / windows
        ├─ build-mcp-*         linux / macos-arm64 / macos-x86_64 / windows
        ├─ publish-thin-releases  → Loctree/loct, Loctree/loctree-mcp
        ├─ publish-npm            → @loctree/loctree + 4 platform packages
        └─ publish-monorepo-release → GitHub release on loctree-suite
              └─ (auto) homebrew-release.yml → homebrew-cli, homebrew-mcp
  └─ (MANUAL) component mirrors: distribution/component-sync.sh (after crates land)
  └─ (MANUAL) editors: gh workflow run vscode-publish.yml / jetbrains-publish.yml
```

---

## 5. Required secrets

| Secret | Used by | For |
|--------|---------|-----|
| `CARGO_REGISTRY_TOKEN` | `publish.yml`, `make publish` | crates.io |
| `NPM_TOKEN` | `publish.yml` | `npm publish --access public` |
| `HOMEBREW_GITHUB_API_TOKEN` | `publish.yml`, `homebrew-release.yml`, `release-bundles.yml` | Cross-repo release + tap writes; also publishes assets to `Loctree/loctree-release` |
| `MACOS_CERT_P12_BASE64`, `MACOS_CERT_PASSWORD`, `MACOS_KEYCHAIN_PASSWORD` | `publish.yml` | Import Developer ID cert |
| `MACOS_DEVELOPER_ID_APPLICATION` | `publish.yml`, `make release-binaries` | codesign identity |
| `APPLE_API_KEY_BASE64`, `APPLE_API_KEY_ID`, `APPLE_API_ISSUER_ID` | `publish.yml` | notarytool |
| `SEMGREP_APP_TOKEN` | `semgrep.yml` | Semgrep registry rules |
| `LOCTREE_GPG_KEY_ID` | `publish.yml`, `release-bundles.yml`, `build-bundle.sh` | Detached `.sig` bundle signatures. **Not currently set.** Without it bundles ship unsigned and `install.sh` (`LOCTREE_REQUIRE_GPG=1` by default) hard-fails every install. |
| `VSCE_PAT`, `OVSX_PAT` | `vscode-publish.yml` | VS Code Marketplace / Open VSX. **Not currently set.** |
| `JETBRAINS_MARKETPLACE_TOKEN`, `JETBRAINS_CERTIFICATE_CHAIN`, `JETBRAINS_PRIVATE_KEY`, `JETBRAINS_PRIVATE_KEY_PASSWORD` | `jetbrains-publish.yml` | JetBrains Marketplace sign + upload. **None currently set.** |
| `GITHUB_TOKEN` | `publish.yml` | Monorepo release |

Local GPG bundle signing uses key fingerprint
`8868139E8A9A2291D067135FB979B60C7079E4D4` (see `distribution/build-bundle.sh`
and `public_dist/install.sh`).

Every lane of the primary `ci.yml` gate runs on GitHub-hosted runners —
`ubuntu-latest` and `macos-latest`. The `ops-linux` security hold is resolved by
retirement rather than by a host rebuild: no self-hosted machine is trusted with
repository code any more, and `tests/preflight_contract.sh` fails if any
`self-hosted` label reappears in `ci.yml`. Release workflows retain their
documented platform-specific runners, plus `windows-latest` for Windows targets.

---

## 6. Local bundle build

```bash
make release-bundles VERSION=0.14.2 DRY_RUN=1 NO_SYNC=1   # inspect the plan
make release-bundles VERSION=0.14.2 NO_SYNC=1             # actually build
make release-pack                                          # version gate + editors + bundles
```

Targets produced: `aarch64-apple-darwin` (full), `x86_64-apple-darwin` (core),
`x86_64-unknown-linux-gnu` (full), `x86_64-unknown-linux-musl-core` (core), and
`x86_64-pc-windows-msvc` (full). Windows keeps the combined `.tar.gz` naming
and stages all six executables with `.exe` suffixes.

`NO_SYNC=1` is required unless the sibling `loct-io` repo is checked out next to
this one — without it the script aborts on a missing
`../loct-io/scripts/sync_releases.py`. Override the location with
`LOCT_IO_ROOT=/path/to/loct-io`.

Bundles embed AICX release binaries (default `AICX_VERSION=0.12.3`) downloaded
from `Loctree/aicx` releases and checksum-verified.

Smoke the staged binaries:

```bash
make smoke-release-macos-arm64 SMOKE_BIN_DIR=<staging>/bin
make smoke-release-linux-gnu   SMOKE_BIN_DIR=<staging>/bin   # asserts GLIBC floor 2.28
```

---

## 7. Public component mirrors

`distribution/component-sync.sh` stages the `engine`, `mcp`, and `lsp` mirrors
from `distribution/component-manifests/*.manifest`.

```bash
distribution/component-sync.sh --component engine --version 0.14.2 \
  --staging /tmp/loctree-sync-engine
```

Staging-only is the default. Pushing requires **all three** of `--push`,
`--remote <url>`, and `LOCTREE_SYNC_CONFIRM=1`.

> **Ordering constraint:** the engine mirror resolves `report-leptos` from
> crates.io at the release version. Running it before `publish-crates` has
> landed fails with
> `failed to select a version for the requirement report-leptos = "^<ver>"`.
> **Sync mirrors after crates are published, not before.**

---

## 8. Install path

`public_dist/install.sh` is what `curl -fsSL https://loct.io/install.sh | bash`
executes.

Security posture — **fail-closed by default**:

- `LOCTREE_REQUIRE_GPG` defaults to `1`. Missing `gpg`, an unreachable signing
  key, or a missing `.sig` sidecar all **abort** the install. Set to `0` to
  downgrade those to warnings.
- The signing key fingerprint is pinned; a mismatch is a hard exit and is *not*
  suppressible via `LOCTREE_REQUIRE_GPG`.
- The per-archive `.sha256` check is unconditional and hard-fails on mismatch.
  `SHA256SUMS` is an additional check when present.
- `LOCTREE_ALLOW_SOURCE_FALLBACK` defaults to `0`. Without it, a platform with
  no prebuilt bundle **errors out** rather than silently building from source.

Known coverage gap: `target_triple()` has no case for `darwin:x86_64`, so Intel
macOS falls into the source-fallback path and, by default, exits with an error.

---

## 9. Editors

Built in-repo, versioned in lockstep, **not yet live on either marketplace**:

```bash
make editors-vscode-package        # → .vsix
make editors-jetbrains             # → plugin .zip
make editors-jetbrains-verify      # Plugin Verifier (release confidence)
```

| Workflow | Trigger | Notes |
|----------|---------|-------|
| `vscode-extension.yml` | push / PR | Builds and tests the platform-tagged VSIX artifacts |
| `vscode-publish.yml` | `workflow_dispatch` **only** | Republishes the exact artifacts `vscode-extension.yml` produced; no push or tag path |
| `jetbrains-plugin.yml` | push / PR | Gradle test + buildPlugin |
| `jetbrains-publish.yml` | `workflow_dispatch` **only** | build → verify → sign → upload; `dry_run` defaults to **true** |

Editor publishing is deliberately decoupled from the suite tag: no commit and no
tag can put a build on a marketplace by itself. Each publish workflow needs its
own secrets and refuses to run without them:

| Workflow | Repository secrets |
|----------|--------------------|
| `vscode-publish.yml` | `VSCE_PAT` (VS Code Marketplace), `OVSX_PAT` (Open VSX) |
| `jetbrains-publish.yml` | `JETBRAINS_MARKETPLACE_TOKEN`, `JETBRAINS_CERTIFICATE_CHAIN`, `JETBRAINS_PRIVATE_KEY`, `JETBRAINS_PRIVATE_KEY_PASSWORD` |

```bash
gh workflow run vscode-publish.yml    -f tag=v0.14.2 -f target=both -f dry_run=true
gh workflow run jetbrains-publish.yml -f dry_run=true -f channel=stable
```

Neither marketplace identity exists yet: the `libraxis` VS Code publisher and
the `libraxis` Open VSX namespace both resolve 404, and `io.loct.loctree` is not
on the JetBrains Marketplace. Creating them is operator work no workflow can do.

See [editors-distribution.md](editors-distribution.md),
[vscode-marketplace.md](vscode-marketplace.md), and
[jetbrains-marketplace.md](jetbrains-marketplace.md).

---

## 10. Known blockers for a true one-button release

Status re-verified **2026-08-18** on `release/0.14.2-rebuild`. Every line below was
checked in this pass; nothing is inherited from the previous revision unaltered.

### Closed since the last revision

1. ~~**`publish.yml` npm staging is not wired.**~~ **Wired 2026-08-18 —
   structurally, not CI-proven.** `build-cli-*` now builds all four suite
   binaries per target (`cargo build -p loctree -p loctree-mcp -p loctree-lsp`)
   and uploads one `npm-suite-<plat>` artifact each; `publish-npm` downloads
   them and stages `platform-packages/<plat>/bin/{loct,loctree,loctree-mcp,loctree-lsp}`.
   The fail-closed gate still runs *after* staging and got **stricter**
   (presence + non-empty + ≥1 MiB + unix exec bit). Existing `loct-*` /
   `loct-mcp-*` artifact names and tarball shapes are byte-identical, so
   `publish-thin-releases` sees exactly the asset set it saw before.
   Found and fixed on the way: `distribution/macos/sign-and-notarize.sh` was
   deleted at the v0.10.2 baseline (`6d398f11`) while **all four** macOS jobs
   kept calling it — and its historical version consumed Apple-ID password
   envs `publish.yml` never passed. Restored against the `APPLE_API_KEY_*`
   App Store Connect envs the workflow actually provides.
   **Not proven:** no Actions run has executed this path. YAML parses,
   `actionlint` is clean, the wrapper smoke test passes, a full local
   simulation of staging+gate passes *including a negative test* (deleted and
   truncated binaries are both caught). Artifact hand-off on real runners,
   notarization with real secrets, and npm OIDC remain CI-only proofs.

2. ~~`make semgrep` is red~~ **Clean, re-verified 2026-08-18:** exit 0,
   **0 findings**. The historical 5 `rust.actix.path-traversal.tainted-path`
   findings in `loctree-rs/src/cli/dispatch/handlers/context/atlas.rs` remain
   fixed at the validated root object (`contained_atlas_dir`, single-component
   artifact names).

3. ~~**`install.sh` version has no gate.**~~ Resolved 2026-08-03.
   `scripts/sync-version.sh` updates it; `make version-assert` rejects drift.

4. ~~**MCP thin-release repo target was split-brain.**~~ Resolved 2026-08-03.
   `Loctree/loctree-mcp` is the consistent target everywhere.

5. ~~**The publish workflows are not on the default branch.**~~ **Resolved —
   verified 2026-08-18.** `gh repo view` reports the default branch is
   `develop`, and `publish.yml`, `release-bundles.yml`, `vscode-publish.yml`
   and `jetbrains-publish.yml` are all present on `origin/develop`.
   `gh workflow run` resolves them.

6. ~~**`Loctree/loctree-release` never receives the release assets.**~~
   **Closed 2026-08-18 (`c99fa8cd`), structurally.** `release-bundles.yml`
   gained a `publish-loctree-release` job that publishes **both** asset shapes
   the plugins parse — see §11. Root cause was worse than "a missing upload":
   the workflow ended at `actions/upload-artifact`, so every tagged build died
   inside a private Actions run. Also fixed here: `x86_64-apple-darwin` was
   **never** in this workflow on any revision of any branch, yet
   `vscode-extension.yml` has a live `darwin-x64` lane — one third of the VSIX
   matrix was resolving a target the release pipeline could not produce.

7. ~~**A mirror sync push silently deleted 25 governance files.**~~
   **Closed 2026-08-18 (`c99fa8cd`).** `component-sync.sh maybe_push()` built
   its commit from the staging tree alone, so every mirror path the manifest
   did not own vanished — `SECURITY.md`, `CHANGELOG.md`, `CONTRIBUTING.md`,
   `.github/ISSUE_TEMPLATE/*`, `homebrew-release.yml` and 20 more. Now the
   index is seeded from the **remote** tree, suite-owned prefixes are dropped,
   staging is overlaid with `git add --no-all`, and any delete must be declared
   via `remove=` in the manifest. Proven against a local bare clone of the real
   mirror content: **25 deletions before, 1 declared deletion after**, with a
   guard that refuses undeclared deletes (negative-tested).

### Open — blocking, and who can close them

8. **The repository does not hold the secrets its buttons require.**
   Verified 2026-08-18 via `gh secret list` plus org scope. Present: exactly
   `CARGO_REGISTRY_TOKEN`, `HOMEBREW_GITHUB_API_TOKEN`, `NPM_TOKEN` (repo) and
   `SEMGREP_APP_TOKEN`, `GEMINI_API_KEY`, `NPM_TOKEN` (org `Loctree`).
   **Missing:** `VSCE_PAT`, `OVSX_PAT`, `JETBRAINS_MARKETPLACE_TOKEN`,
   `JETBRAINS_CERTIFICATE_CHAIN`, `JETBRAINS_PRIVATE_KEY`,
   `JETBRAINS_PRIVATE_KEY_PASSWORD`, `MACOS_CERT_P12_BASE64`,
   `MACOS_CERT_PASSWORD`, `MACOS_KEYCHAIN_PASSWORD`,
   `MACOS_DEVELOPER_ID_APPLICATION`, `APPLE_API_KEY_BASE64`,
   `APPLE_API_KEY_ID`, `APPLE_API_ISSUER_ID`, `LOCTREE_GPG_KEY_ID`.
   `LOCTREE_GPG_KEY_ID` is the quiet one: without it bundles ship with no
   `.sig` sidecar, and `public_dist/install.sh` defaults `LOCTREE_REQUIRE_GPG=1`
   with a **non-suppressible** pinned-fingerprint check — so every default
   install fails while CI stays green. Operator-only.

9. **Marketplace identities do not exist.** Re-probed live 2026-08-18:
   `marketplace.visualstudio.com/publishers/libraxis` → 404;
   `open-vsx.org/api/libraxis` → `{"error":"Namespace not found: libraxis"}`;
   `plugins.jetbrains.com/api/plugins/intellij/io.loct.loctree` → 404.
   These must exist **before** the first dry run — `vsce verify-pat` and the
   Open VSX namespace probe are exactly the steps that fail otherwise. The
   first JetBrains upload cannot be automated at all: `publishPlugin` updates
   an existing listing, it cannot create one. Operator-only.

10. **The public engine mirror still carries a dangerous publish workflow —
    right now.** `Loctree/loctree` `origin/main` `.github/workflows/publish.yml`
    triggers on `push: tags`, runs on the retired `[self-hosted, …, ops]`
    runner class, uses long-lived `NPM_TOKEN`, and has **neither** the
    NOT-WIRED stub **nor** the fail-closed platform-package gate. The manifest
    no longer ships `publish.yml` or `release-bundles.yml` to the mirror
    (`c99fa8cd` moved them to `remove=`), but that only makes the **next** sync
    safe. Nothing is fixed retroactively. Either run the sync push or delete
    that file on the mirror directly, **before** any `v0.14.2` tag could reach it.
    The justification the manifest previously gave — npm trusted publishing
    binds to repo+workflow — does not survive evidence:
    `npm view @loctree/loct@0.13.1` reports `_npmUser: mszymanska` and
    `dist.attestations: null`, i.e. published by hand with a token. No CI npm
    publish has ever happened, so there is no binding to preserve.

11. **Downstream release repos are empty, so the Homebrew leg has never run.**
    `gh release list` → `Loctree/loct`: none. `Loctree/loctree-mcp`: none.
    `homebrew-release.yml` downloads its tarballs from exactly those two repos.
    The taps confirm it: `homebrew-cli` and `homebrew-mcp` are still at
    `chore: bootstrap repo` (2026-03-20) with no `Formula/` directory, and
    `homebrew-tools` last shipped `v0.8.16`. **`brew install loctree/cli/loct`
    does not work today** — the README should not promise it until it does.

12. **`mcp` and `lsp` mirrors still cannot be pre-staged.** Both manifests use
    `dependency_mode=crates.io registry`, so staging resolves `loctree` at the
    release version and fails until `publish-crates` lands (verified: exit 101
    on missing 0.14.2). Sync those two **after** crates publish. The `engine`
    manifest moved to `local workspace snapshot` and now stages clean
    pre-publish — the previous revision's "all three fail" is stale.

13. **loct.io has no automated publish path.** `make release-bundles` reaches
    loct.io only via `$LOCT_IO_ROOT/scripts/sync_releases.py`, CI always passes
    `--no-sync`, and the `loct-io` checkout is absent on this machine. Worse,
    the script served at `https://loct.io/install.sh` is a **different, older
    generation** than `public_dist/install.sh` in this tree (different length,
    pinned to 0.13.1, references a manifest mechanism this repo does not have).
    `make version-assert` therefore gates a file users never execute.

14. **Tag push is still not one button, by design.** `release-bundles.yml`
    fires on the tag; `publish.yml` stays `workflow_dispatch`-only. Keep it
    that way — two deliberate buttons beat one accidental one.

15. **`distribution/tests/component_sync_test.sh` is red at HEAD.** Pre-existing,
    reproduced against a clean extraction of `HEAD`: `check_engine_registry_shape()`
    still asserts `dependency_mode=crates.io registry` while `engine.manifest`
    is `local workspace snapshot`. The test dies before its push-refusal
    assertion, so that coverage runs by hand today. Fix the test to the manifest's
    actual mode.

---

## 11. Published asset contract (`Loctree/loctree-release`)

A release there must carry **two shapes**, because two different consumers parse
two different names. Getting either wrong is a 404 at a user's first run.

| Shape | Name | Parsed by |
|---|---|---|
| A — suite archive | `loctree-<ver>-<triple>.tar.gz` + `.sha256` | `editors/vscode/scripts/fetch-release-lsp.js:24` (VSIX packaging), `editors/vscode/src/client.ts:198` (runtime) |
| A — inner layout | `loctree-<ver>-<triple>/bin/loctree-lsp` | `fetch-release-lsp.js:28` — the **internal directory structure is part of the contract**, not just the filename |
| B — raw LSP binary | `loctree-lsp-{darwin-arm64,darwin-x64,linux-x64}` + `.sha256` | `editors/jetbrains/.../PlatformAsset.kt` `assetName()` |

Windows and Linux arm64 are deliberately absent from shape B; `PlatformAsset`
returns `null` for them and the plugin raises a visible unsupported-platform
notification instead of failing silently.

Two historical traps worth remembering. v0.13.1's `x86_64-apple-darwin` archive
was assembled **by hand** — it lacks the `components.json` that `build-bundle.sh`
writes into every bundle and that this workflow's verify step requires, so it
could not have passed the pipeline it appears to come from. And v0.13.1's
`SHA256SUMS` contains three bare filenames with **no digests at all** and no
entry for the Intel archive: a file named like a verifier that verifies nothing.
The 0.14.2 job emits real `sha256sum -c` input.

---

𝚅𝚒𝚋𝚎𝚌𝚛𝚊𝚏𝚝𝚎𝚍. with AI Agents by Vetcoders (c)2024-2026 LibraxisAI
