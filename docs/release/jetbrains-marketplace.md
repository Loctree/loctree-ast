# JetBrains Marketplace Release Runbook — Loctree plugin

Plugin: `io.loct.loctree` (`editors/jetbrains/`, name **Loctree**)
Workflow: `.github/workflows/jetbrains-publish.yml` (manual trigger only)

## The one button

```bash
# 1. Rehearsal — builds, verifies, and SIGNS, uploads nothing:
gh workflow run jetbrains-publish.yml -f dry_run=true -f channel=stable

# 2. The real thing:
gh workflow run jetbrains-publish.yml -f dry_run=false -f channel=stable
```

Or via GitHub UI: Actions → "JetBrains Plugin Publish" → Run workflow →
untick "Dry run" → Run. The workflow cannot fire on push or PR.

`channel=stable` maps to the Marketplace `default` channel (what every
user sees). `channel=eap` publishes to a custom `eap` channel that users
only get after adding the channel-specific plugin repository URL in their
IDE — useful for staged rollouts.

## Required secrets (GitHub repo → Settings → Secrets → Actions)

The workflow fails fast in a preflight step — even in dry-run — if any of
these are missing. It never silently skips signing or publishing.

| Secret | Content | How to obtain |
|---|---|---|
| `JETBRAINS_MARKETPLACE_TOKEN` | Permanent token string | [Marketplace profile → My Tokens](https://plugins.jetbrains.com/author/me/tokens) → New token. The vendor account that owns `io.loct.loctree` must generate it. |
| `JETBRAINS_CERTIFICATE_CHAIN` | PEM text of `chain.crt` | Self-generated signing keypair (below). Paste full PEM including `-----BEGIN/END CERTIFICATE-----`. |
| `JETBRAINS_PRIVATE_KEY` | PEM text of `private.pem` | Same keypair. Paste full PEM including the `ENCRYPTED PRIVATE KEY` block. |
| `JETBRAINS_PRIVATE_KEY_PASSWORD` | Key passphrase | Chosen when generating the key. |

Generating the signing keypair (once; keep `private.pem` + password in the
operator vault, never in the repo):

```bash
openssl genpkey -aes-256-cbc -algorithm RSA \
  -out private.pem -pkeyopt rsa_keygen_bits:4096
openssl req -key private.pem -new -x509 -days 3650 -out chain.crt
```

JetBrains accepts self-signed certificates; the signature proves ZIP
integrity, it is not a CA trust chain. Note: `build.gradle.kts` reads
`CERTIFICATE_CHAIN` / `PRIVATE_KEY` env vars as **file paths**; the
workflow materializes the secret text into `$RUNNER_TEMP` files itself.

## What the workflow does

1. **Preflight** — hard-fails with the exact list of missing secrets.
2. **Free disk** — the plugin verifier downloads an IU distribution
   (~2–3 GB); a runaway verifier once filled the 14 GB runner disk.
3. `./gradlew test` → `buildPlugin` → `verifyPlugin` (verification lane:
   oldest declared build, IU 252.\*). No cargo build and no bundled runtime —
   see **Runtime delivery** below.
4. `./gradlew signPlugin` — always runs, including in dry-run, so the
   rehearsal proves the certificate chain actually works.
5. **Download-only assertion** — unpacks the freshly built ZIP and fails the
   job if any plugin JAR contains `bin/loctree-lsp`.
6. `./gradlew publishPlugin` — **only when `dry_run=false`**. Channel from
   the `channel` input.
7. Uploads the signed ZIP as workflow artifact either way.

## Runtime delivery: download-only (product decision)

The published ZIP ships **no** `loctree-lsp` binary. The plugin resolves its
runtime at first use through `BinaryResolver`: configured path → IDE cache →
SHA256-verified download from GitHub Releases (fail-closed) → `PATH`. This is
the same chain the VS Code extension falls back to, and it is the only one that
is platform-correct, because `PlatformAsset` picks the asset per OS *and* arch.

This closed the single-platform bundling blocker. `prepareBundledLsp` used to
copy exactly one host-built binary into the plugin JAR at the unqualified path
`bin/loctree-lsp`; since `BinaryResolver` ranks BUNDLED above cache and download
and `PlatformAsset.binaryName(os)` only distinguishes `.exe` from no-`.exe`, a
Linux-CI ZIP would have handed macOS and Windows users an unexecutable ELF —
cached with a valid checksum sidecar, so the download fallback never ran and
there was no self-repair path. Bundling is now an explicit dev-only opt-in
(`./gradlew buildPlugin -PbundleLsp=true`, or `LOCTREE_BUNDLE_LSP=1`), off in
every default and CI build, and `jetbrains-publish.yml` asserts the absence of
`bin/loctree-lsp` in the artifact before it can upload anything. The cost is a
one-time download on first use; the benefit is a plugin that works on every
platform from one build host. Per-platform bundled resources
(`bin/<asset-name>` plus an arch-aware `bundledBinary()`) remain possible later
if zero-download startup ever becomes worth a multi-platform build lane.

## Blockers (must be closed before `dry_run=false`)

**1. `io.loct.loctree` does not exist on the Marketplace yet.** Verified:
`GET https://plugins.jetbrains.com/api/plugins/intellij/io.loct.loctree` → 404.
That is expected for a first release and is what the next section is about.

~~**2. No `LICENSE` sits in `editors/jetbrains/`**~~ — **closed**:
`editors/jetbrains/LICENSE` exists as a wrapper license mirroring
`editors/vscode/LICENSE` (MIT glue, BUSL-1.1 engine). What remains is an
operator step, not a repo blocker: the Marketplace vendor page needs the same
license declaration selected by hand at first upload.

## First-time publish (version 0.14.2 reality check)

- The **first** upload of a brand-new plugin cannot be done by
  `publishPlugin` against a plugin ID that does not exist yet on the
  Marketplace. Upload the signed ZIP manually once at
  <https://plugins.jetbrains.com/plugin/add> (the dry-run artifact is
  exactly the right file), fill in the vendor page, and wait for approval.
  Every subsequent release goes through the button.
- **JetBrains review**: new plugins go through manual review — typically
  **2–5 business days**. Updates to an approved plugin are auto-published
  after automated checks (usually minutes to hours). Plan announcements
  accordingly; the workflow finishing green means "accepted for review",
  not "live".

## Version discipline

Before pressing the button for a new release:

1. Bump `pluginVersion` in `editors/jetbrains/gradle.properties`.
2. Move `[Unreleased]` content in `editors/jetbrains/CHANGELOG.md` to a
   dated `[x.y.z]` section.
3. Update `<change-notes>` in
   `editors/jetbrains/src/main/resources/META-INF/plugin.xml` (CDATA HTML,
   not markdown — the Marketplace renders it per-version).
4. Dry-run first. Always.

## Rollback reality

There is no un-publish button. Options, in order of preference:

- **Fix forward**: publish a patched `x.y.z+1` — automated checks only,
  live within hours.
- **Hide a broken version**: Marketplace vendor page → plugin → Versions →
  remove the specific version. Users who already updated keep the broken
  build until the next release; the IDE does not auto-downgrade.
- **Channel containment**: ship risky changes to `channel=eap` first;
  `default`-channel users never see them.

## Local equivalents (what CI runs)

```bash
cd editors/jetbrains
./gradlew test buildPlugin verifyPlugin
# dev-only: bundle a local server into the ZIP (never publish such a build)
LOCTREE_LSP_PATH=~/.local/bin/loctree-lsp ./gradlew buildPlugin -PbundleLsp=true
# signing rehearsal (needs the key files locally):
CERTIFICATE_CHAIN=/path/chain.crt PRIVATE_KEY=/path/private.pem \
  PRIVATE_KEY_PASSWORD=... ./gradlew signPlugin
```

Verified state as of 0.14.2 (2026-08-18): `test`, `buildPlugin`, and
`verifyPlugin` all pass; verifier verdict `io.loct.loctree:0.14.2 against
IU-252.28539.97: Compatible` (plugin is compiled against IU 2026.1.3 and
bytecode-verified against the oldest declared build 252.\*).
