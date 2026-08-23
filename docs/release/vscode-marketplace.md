# VS Code extension — marketplace publish runbook

How the Loctree VS Code extension (`libraxis.loctree`) reaches the VS Code
Marketplace and Open VSX.

Companion to `docs/release/editors-distribution.md`, which owns the *architecture*
(who builds what, where binaries live). This document owns the *button*.

---

## The button

**Actions → "VSCode Extension Publish" → Run workflow.**

| Input | Meaning |
|---|---|
| `tag` | Release tag whose VSIX assets to publish, e.g. `v0.14.2`. |
| `run_id` | Alternative source: a `vscode-extension.yml` run ID to take artifacts from. Use when no release has been cut yet. |
| `target` | `both` (default) / `marketplace` / `openvsx`. |
| `dry_run` | **Checked by default.** Verifies everything and publishes nothing. |

Set **either** `tag` **or** `run_id` — never both, never neither. The workflow
fails immediately with a clear message otherwise.

### Normal sequence

```
1. Run with tag=v0.14.2, target=both, dry_run=CHECKED    → read the summary
2. Run with tag=v0.14.2, target=both, dry_run=UNCHECKED  → live publish
```

That is the whole procedure. Step 1 is not optional ceremony: it is the only
place where a bad PAT, a missing Open VSX namespace, an untargeted VSIX, or a
version/tag mismatch gets caught without a marketplace side effect.

---

## What the workflow does and does not do

**It does not build.** It downloads the platform-tagged VSIX artifacts that
`vscode-extension.yml` already produced, verified against `npm run check-types`
and `npm test`, and republishes those exact bytes. A rebuild at publish time
could drift from what was tested; this cannot.

Before it publishes anything it refuses to continue unless:

- at least one `.vsix` was found;
- every VSIX is `libraxis.loctree`;
- **every VSIX declares a `TargetPlatform`** — see "The clobber trap" below;
- no two VSIXs declare the *same* `TargetPlatform`;
- all VSIXs carry the same version, and (when publishing from a tag) that
  version equals the tag minus its `v`;
- every VSIX actually contains `extension/dist/extension.js` and
  `extension/bin/loctree-lsp`;
- the required secret for each requested target is present and non-empty;
- `VSCE_PAT` passes a live `vsce verify-pat libraxis`;
- the Open VSX namespace `libraxis` exists.

A missing secret is a hard failure with a pointer to this file. It is never a
silent no-op.

### Guardrails against accidental firing

`workflow_dispatch` is the only trigger in `vscode-publish.yml` — no `push`, no
`release`, no tag path. The job additionally re-checks
`github.event_name == 'workflow_dispatch'`, and a `concurrency: vscode-publish`
group prevents two operators racing the marketplace API.

---

## The clobber trap

VS Code supports *platform-specific extensions*: several VSIXs sharing one
version, each tagged with a `TargetPlatform`, each served to the matching OS/arch.

A VSIX built without `vsce package --target <platform>` has **no**
`TargetPlatform` and is treated as "universal". Publishing three universal VSIXs
under one version does not produce three platforms — each upload **replaces** the
previous one, so only the last platform survives, and macOS users can end up
being served a Linux binary.

`vscode-extension.yml` therefore packages with `npm run package -- --target
${{ matrix.vsix_suffix }}`. The suffixes (`linux-x64`, `darwin-arm64`,
`darwin-x64`) are deliberately identical to VS Code's platform identifiers, so
one matrix field drives both the artifact filename and the manifest. The publish
workflow rejects any untagged VSIX rather than trusting that this stayed true.

---

## Platform coverage (be honest with users)

| Platform | Published | Why |
|---|---|---|
| `darwin-arm64` | yes | built + `loctree-lsp` bundled |
| `darwin-x64` | yes | built + `loctree-lsp` bundled |
| `linux-x64` | yes | built + `loctree-lsp` bundled |
| `win32-x64` | **no** | no `loctree-lsp` Windows build in the release pipeline |
| `linux-arm64` | **no** | same |

Because we publish *only* platform-specific packages, Windows and Linux arm64
users are told the extension is unavailable for their platform. That is
intentional and preferable to shipping them a binary that cannot run — but it
also means there is no universal fallback riding on the runtime auto-download
path. Adding Windows/linux-arm64 to the build matrix is the fix, not adding a
universal VSIX (`scripts/prepare-bins.js` fails closed without a binary by
design).

---

## Secrets

Both are **repository secrets** on `Loctree/loctree-suite`.

### `VSCE_PAT` — VS Code Marketplace

1. The `libraxis` publisher must already exist at
   <https://marketplace.visualstudio.com/manage>, and the Azure DevOps account
   creating the token must be a member of it.
2. Azure DevOps → *User settings* → *Personal Access Tokens* → **New Token**.
3. Organization: **All accessible organizations** (required — a single-org token
   is rejected by the Marketplace API).
4. Scopes: **Marketplace → Manage**.
5. Copy the token once; store as `VSCE_PAT`.

PATs expire (1 year max). When the button starts failing at the
`verify-pat` step, the token expired — regenerate and replace the secret.

### `OVSX_PAT` — Open VSX

1. Sign the **Eclipse Foundation Publisher Agreement** at
   <https://open-vsx.org/user-settings/extensions>. Open VSX refuses publishes
   from accounts that have not signed it, and this is the single most common
   first-publish failure.
2. Log in at <https://open-vsx.org> with GitHub → *Settings* → *Access Tokens* →
   generate.
3. Store as `OVSX_PAT`.
4. The `libraxis` namespace must exist. One-time, from a machine with the token:

   ```bash
   npx ovsx create-namespace libraxis -p "$OVSX_PAT"
   ```

   Namespaces start **unverified**, which puts a warning triangle on the
   listing. Verification is a separate manual request to the Open VSX
   maintainers; it does not block publishing.

Open VSX has no token-verification endpoint, so a dry run can prove the
namespace exists but *cannot* prove `OVSX_PAT` is valid. Only a real publish does.

---

## Rollback reality

**There is no clean rollback. Plan forward-fixes, not undos.**

**VS Code Marketplace.** `vsce unpublish libraxis.loctree` removes the *entire
extension* — every version, all install counts, all ratings — not the single bad
version. Treat the identifier as unsafe to reuse afterwards. There is no
supported per-version delete. The real remedy for a bad release is to publish a
higher patch version; VS Code clients auto-update to it. If a version is actively
harmful, unpublishing to stop the bleeding is a deliberate, destructive operator
decision — not a rollback.

**Open VSX.** No `unpublish` in the CLI. Version removal is a request to the Open
VSX maintainers. Assume irreversible; forward-fix.

**Consequence for the process:** the dry run is the rollback. Use it.

---

## Prerequisites checklist before the first real publish

- [ ] `libraxis` publisher exists on the VS Code Marketplace.
- [ ] `libraxis` namespace exists on Open VSX, Publisher Agreement signed.
- [ ] `VSCE_PAT` and `OVSX_PAT` set as repository secrets.
- [ ] A tag `vX.Y.Z` exists **and** `vscode-extension.yml` ran green on it and
      attached `loct-vscode-*.vsix` assets to the release.
- [ ] `editors/vscode/CHANGELOG.md` has a real entry for `X.Y.Z`.
- [ ] `editors/vscode/package.json` version equals `X.Y.Z` equals the
      `loctree-lsp` version bundled in the VSIX (same-version hard rule,
      `docs/release/editors-distribution.md`).
- [ ] Dry run green.

---

## Known gaps

- **Neither marketplace identity exists yet.** Verified:
  `https://marketplace.visualstudio.com/publishers/libraxis` → 404, and
  `https://open-vsx.org/api/libraxis` → `{"error":"Namespace not found: libraxis"}`.
  Both are operator work that no workflow can do, and both must be done *before*
  the first dry run — `verify-pat` and the namespace probe are exactly the checks
  that will fail otherwise.
- **No `v0.14.x` tag exists yet.** The extension is at `0.14.2` but the newest
  tag is `v0.13.1`, so there is no release to publish from today. Either cut the
  tag or dispatch with `run_id`.
- **`repository` / `bugs` / `qna` point at `Loctree/loctree`**, which resolves
  (HTTP 200) but is not `Loctree/loctree-vscode`. Per `editors-distribution.md`
  the intended public home for the extension source is `loctree-vscode`, which
  does not exist yet (404). They previously pointed at the private
  `Loctree/loctree-suite`, which rendered as a dead link for every marketplace
  visitor. Re-point once the public repo exists.
- **Open VSX namespace verification** is not done, so the listing will carry an
  unverified-publisher warning.
- **Partial platform sets publish with a warning, not a refusal.** If a matrix
  leg of `vscode-extension.yml` failed, the publish workflow warns and reports
  the missing platforms in the job summary but still proceeds — a missing
  platform can be added later under the same version, so refusing would be
  worse than fixing forward.
- **A partially-succeeded `target=both` run is not idempotent.** If the
  Marketplace publish lands and Open VSX fails, re-running with `target=both`
  fails at the Marketplace step because the version already exists. Re-run with
  `target=openvsx` instead; `--skip-duplicate` is deliberately *not* used, since
  it would turn "you forgot to bump the version" into a silent green run.

---

𝚅𝚒𝚋𝚎𝚌𝚛𝚊𝚏𝚝𝚎𝚍. with AI Agents by Vetcoders (c)2024-2026 LibraxisAI
