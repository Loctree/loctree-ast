# Publishing Guide — the Loctree runtime wrapper (`loctree` / `@loctree/loctree`)

This guide describes the npm publish flow for **one** user-facing wrapper,
published under two names, plus its technical platform packages
`@loctree/loctree-*`.

## Two names, one package

`distribution/npm/loct/` is the single source for the wrapper. Publish runs it
twice: once as `@loctree/loctree`, then — with the name field swapped in place —
as plain `loctree`. Same version, same tarball contents, one resolver.

- `@loctree/loctree` is the **canonical identity**: the scope names the org, the
  name names the repo — the same golden org/repo split as
  github.com/Loctree/loctree. This is the name to put in new docs.
- `loctree` is the free short form (`npm i -g loctree`) published from the same
  tarball.
- `@loctree/loct` (0.13.x and earlier) is the **legacy scoped name**. It stays
  published — npm versions are immutable — and gets an `npm deprecate` pointer
  to `@loctree/loctree` only AFTER a first healthy 0.14.2 publish under the new
  names. Its `@loctree/loct-*` platform packages follow the same rule.

`loctree` previously carried a 0.8.x line under MIT OR Apache-2.0. Those
published versions keep their original terms — npm versions are immutable.
Releases 0.13 and later are BUSL-1.1, and that change must stay stated in the
package README, not only here.

## Authentication — trusted publishing (OIDC), not a token

Publishing uses npm trusted publishing: the workflow authenticates with a
short-lived OIDC credential minted from the job's identity. There is no
`NPM_TOKEN`, so there is no secret to leak and no publisher IP to allowlist —
the IP allowlist is what broke publishing when CI moved off the self-hosted
runner.

Requirements:

- npm >= 11.5.1 in the job (Node 20 ships npm 10.x — the workflow upgrades it).
- `permissions: id-token: write` on the publishing job.
- A trusted publisher configured on npmjs.com for **every** package this job
  pushes — `loctree`, `@loctree/loctree`, and the four `@loctree/loctree-*` platform
  packages — each pointing at the repository that holds `publish.yml` and at
  that workflow's filename.

Provenance attestation is produced automatically, and requires the publishing
repository to be public.

> **Bootstrap caveat.** A trusted publisher can only be configured on a package
> that already exists. The first Gen3 publish introduces five new scoped names
> (`@loctree/loctree` plus four `@loctree/loctree-*` platform packages) and the
> first Gen3 version of the existing bare `loctree`, so that publish runs with a
> granular npm token by dispatching `publish.yml` with `npm_bootstrap=true`.
> The workflow verifies that credential before its npm publish job, retains
> OIDC permissions so provenance attestations are still emitted, and skips any
> exact version already published if a partial run is retried. Configure trusted
> publishers for all six package identities immediately after, then dispatch future releases with
> `npm_bootstrap=false`. This ordering is deliberate, not a workaround.

> **Registry reality (2026-08-23).** The live Gen3 package is
> `@loctree/loct@0.13.1` under the legacy scoped name; `@loctree/loctree` does
> not exist on the registry yet and first ships at 0.14.3. Bare `loctree` still
> resolves the deprecated 0.8.x line (deprecated 2026-08-20). The npm runtime
> package is a separate distribution track from crates.io. crates.io publishes
> `loctree` and the legacy-named `loctree-mcp`; `loctree-lsp` remains an
> in-tree distribution crate.

## Architecture (Gen3, embed, one runtime)

| Public package | Commands | Delivery |
| --- | --- | --- |
| `@loctree/loctree` | `loctree` (runtime), `loct` (alias), `loctree-mcp` (stdio adapter) | thin JS wrapper + `optionalDependencies` |

`loctree` and `loct` are the **same binary**. MCP and LSP are sibling binaries
resolved by the runtime package, not separate npm packages:

- `loct watch --http` → streamable-HTTP MCP at `http://127.0.0.1:5174/mcp`
- `loct watch --lsp` → editor language server

The wrapper declares one platform package per target as `optionalDependencies`;
npm/pnpm/yarn install only the one matching the user's platform:

- `@loctree/loctree-darwin-arm64`
- `@loctree/loctree-darwin-x64`
- `@loctree/loctree-linux-x64-gnu`
- `@loctree/loctree-win32-x64-msvc`

**Embed model:** each platform package ships **four** release binaries under
`bin/` — `loct`, `loctree`, `loctree-mcp`, `loctree-lsp` (`.exe` on win32) —
with no postinstall download. The runtime resolves `loctree-mcp` /
`loctree-lsp` as **siblings** of its own executable, so all of them must ship
in the same `bin/` directory. In CI the `build-cli-*` jobs of `publish.yml`
build all four per target and upload one `npm-suite-<platform>` artifact each;
the `publish-npm` job stages them into
`platform-packages/<platform>/bin/` and a fail-closed gate verifies presence,
size, and the executable bit before any `npm publish` runs.

**No install scripts, by design.** Neither the wrapper nor the platform
packages declare any lifecycle script. npm 11+ blocks unapproved
`postinstall` scripts (`allowScripts`) and warns loudly, so a validation-only
postinstall costs first-install trust while delivering nothing — the runtime
shim in `index.js` already raises precise errors (missing platform package,
missing sibling binary) at first invocation.

> There are **no separate user-facing wrappers** `@loctree/loctree-mcp` or
> `@loctree/loctree-lsp`. They are co-process binaries inside the platform
> package. `loctree-mcp` is exposed as a command by `@loctree/loctree` for stdio
> MCP clients; the whole runtime still ships through one public package.

## Legacy public packages (do NOT touch as part of this release)

These already exist publicly and are **legacy / frozen** — predecessors of the
Gen1 distribution, unrelated to the Gen3 runtime wrapper. **Leave them frozen**;
deprecate only AFTER a first healthy `@loctree/loctree` is published.

npm (legacy Gen1 platform packages):

- `@loctree/darwin-arm64@0.8.16`
- `@loctree/linux-x64-gnu@0.8.16`
- `@loctree/win32-x64-msvc@0.8.16`

npm (legacy Gen3 scoped name, superseded by `@loctree/loctree` at 0.14.2):

- `@loctree/loct@0.13.1` + `@loctree/loct-darwin-arm64` /
  `@loctree/loct-darwin-x64` / `@loctree/loct-linux-x64-gnu`
  (`@loctree/loct-win32-x64-msvc` was never published — the Windows npm path
  never worked on the legacy name)

crates.io (legacy crate line):

- `loctree@0.8.17`, `loctree-mcp@0.8.17`, `report-leptos@0.8.17`

Latest public Loctree line: `0.13.0` for crates.io `loctree` and the signed
bundle track. Treat older public-channel notes as historical migration context.

Note the legacy npm platform packages are `@loctree/<platform>` (no `loct-`
prefix); the Gen3 ones are `@loctree/loctree-<platform>`. Different names — no
collision.

> **Do not run** `npm deprecate` for unpublished MCP/LSP npm names. The current
> package shape is one runtime wrapper plus platform packages; there are no
> public `@loctree/loctree-mcp` or `@loctree/loctree-lsp` wrappers to deprecate.

## Prerequisites

1. `@loctree` npm org exists and you have publish rights.
2. Each `distribution/npm/loct/platform-packages/<platform>/bin/` has been
   populated with the four release binaries for that platform — `loct`,
   `loctree`, `loctree-mcp`, `loctree-lsp` (`.exe` on win32). In CI this is
   the `publish.yml` "Stage suite binaries into platform packages" step.
3. Node.js 14+.

## Publish flow

### Step 1 — Sync versions (5 package.json: wrapper + 4 platform)

```bash
node distribution/npm/sync-version.mjs 0.14.2
node distribution/npm/sync-version.mjs --check 0.14.2
```

### Step 2 — Publish platform packages FIRST

npm requires the platform sub-packages to exist before the wrapper resolves,
because the wrapper lists them in `optionalDependencies`. First publish, so all
four are brand-new package names:

```bash
for plat in darwin-arm64 darwin-x64 linux-x64-gnu win32-x64-msvc; do
  (cd distribution/npm/loct/platform-packages/$plat && npm publish --access public)
done
```

### Step 3 — Wait for npm registry to propagate

```bash
sleep 30
```

### Step 4 — Publish the single wrapper

```bash
(cd distribution/npm/loct && npm publish --access public)
```

### Step 5 — Verify

```bash
mkdir -p /tmp/loctree-verify && cd /tmp/loctree-verify
npm init -y >/dev/null
npm install @loctree/loctree

npx loctree --version
npx loct --version
npx --yes --package=@loctree/loctree loctree-mcp --version
# MCP runtime (Ctrl-C to stop): npx loct watch --http
```

## Troubleshooting

### "Platform package not found"

- Platform packages must be published BEFORE the wrapper.
- Wait 30–60 seconds after platform publish for npm registry propagation.
- Verify the packages are public: `npm access list packages @loctree`.

### `loct watch --http` / `--lsp` cannot find the co-process

- The runtime resolves `loctree-mcp` / `loctree-lsp` as siblings of its own
  binary. If the platform package was published without all four binaries
  staged under `bin/`, re-stage and republish that platform package.

### optionalDependencies disabled

- Some CI/package-manager configs disable optional deps; users must enable them.
- Check `.npmrc` / `.yarnrc` for `optional=false` or `--ignore-optional`.

## Rollback / deprecate (future versions only)

Deprecation applies only to versions of `@loctree/loctree` that have actually been
published. Example for a hypothetical future yanked version:

```bash
npm deprecate @loctree/loctree@<published-version> "Please upgrade to <newer>."
```

## Resources

- [npm publishing docs](https://docs.npmjs.com/cli/v10/commands/npm-publish)
- [optionalDependencies](https://docs.npmjs.com/cli/v10/configuring-npm/package-json#optionaldependencies)
- [esbuild npm package strategy](https://esbuild.github.io/getting-started/)
