# Publishing Guide — one Loctree npm runtime, seven package identities

`distribution/npm/loct/` is the single source for one runtime wrapper. The
wrapper is published under three durable install names and resolves one of four
technical platform packages. All seven identities carry one release version.

## Public package graph

| Identity | Role |
| --- | --- |
| `@loctree/loctree` | Canonical wrapper; the scope and name mirror `Loctree/loctree` |
| `@loctree/loct` | Maintained scoped alias for existing users |
| `loctree` | Maintained unscoped short form |
| `@loctree/loctree-darwin-arm64` | Four embedded binaries for macOS ARM64 |
| `@loctree/loctree-darwin-x64` | Four embedded binaries for macOS x64 |
| `@loctree/loctree-linux-x64-gnu` | Four embedded binaries for Linux x64 glibc |
| `@loctree/loctree-win32-x64-msvc` | Four embedded binaries for Windows x64 MSVC |

The three wrappers are produced from the same source and differ only in the npm
`name` field. `@loctree/loct` is not deprecated or frozen. The older
`@loctree/loct-*` platform packages remain frozen because all three current
wrappers resolve the canonical `@loctree/loctree-*` platform set.

The unscoped `loctree` name previously carried the 0.8.x line under MIT OR
Apache-2.0. Those immutable versions retain their original terms. Releases
0.13 and later are BUSL-1.1, as stated in the package README and metadata.

## Runtime shape

Every platform package contains these sibling executables under `bin/`:

- `loct` and `loctree` — the same CLI/runtime binary;
- `loctree-mcp` — the stdio MCP adapter;
- `loctree-lsp` — the editor language server.

There are no separate `@loctree/loctree-mcp` or
`@loctree/loctree-lsp` wrappers. There are no lifecycle install scripts. The
wrapper uses `optionalDependencies` to install only the package matching the
host OS and architecture, then exposes all four commands.

## Authentication contract

There are exactly two lanes:

1. **0.14.4 identity bootstrap — local operator lane.** This is the last
   token-authenticated publish. It runs on the release workstation through the
   Make target below, reads the raw token from `KEYS/.npm`, stages only signed
   release-repository assets, and publishes idempotently without claiming CI
   provenance. The token is never copied into GitHub Actions or the repository.
2. **Every later release — GitHub trusted publishing.** `publish.yml` checks out
   the exact requested tag and uses npm OIDC with provenance. It has no token
   input or `NPM_TOKEN` fallback.

The CI job requires npm 11.5.1 or newer, `id-token: write`, the public
`Loctree/loctree` repository, workflow filename `publish.yml`, and a trusted
publisher configured on npmjs.com for **each of the seven identities**.

After the 0.14.4 bootstrap succeeds, configure all seven trusted publishers
before dispatching `publish.yml`. A missing publisher is a release blocker; do
not restore a repository or organization token as a fallback.

## Local bootstrap and deterministic rehearsal

The release tag must exist locally. The script extracts `distribution/npm/loct`
directly from that immutable tag, so newer branch docs or orchestration code
cannot contaminate the package payload. Unless `ASSET_DIR` is supplied, it
downloads the public `vX.Y.Z` assets from `Loctree/loctree-release`. It verifies each
archive against its `.sha256` sidecar, stages the four binaries per platform in
a temporary directory, checks package versions and helper tests, and dry-packs
all seven identities.

Verification is non-publishing and does not read the npm token:

```bash
make npm-release-verify VERSION=0.14.4
```

For an offline rehearsal, point it at a directory containing the four selected
archives and their checksum sidecars:

```bash
make npm-release-verify VERSION=0.14.4 ASSET_DIR=/workspace/release-assets
```

The explicit publication command is operator-only and must run through the
interactive shell on the release workstation:

```bash
zsh -ic 'cd /workspace/loctree && make npm-release-publish VERSION=0.14.4 KEYS=$HOME/.keys'
```

Publication order is fail-closed and idempotent:

1. Four `@loctree/loctree-*` platform packages.
2. `@loctree/loctree`.
3. `@loctree/loct`.
4. `loctree`.

The helper queries the exact `name@version`. An existing immutable version is
skipped; an absent version is published; authentication, permission, network,
and registry errors abort instead of being mistaken for absence. That makes a
partial bootstrap safely retryable.

## Post-publish proof

Wait for registry propagation, then prove all seven exact versions and cold
installs on real client platforms. At minimum:

```bash
for package in \
  @loctree/loctree \
  @loctree/loct \
  loctree \
  @loctree/loctree-darwin-arm64 \
  @loctree/loctree-darwin-x64 \
  @loctree/loctree-linux-x64-gnu \
  @loctree/loctree-win32-x64-msvc
do
  npm view "$package@0.14.4" version
done
```

For each wrapper identity on a supported host, install into a new temporary
prefix and run:

```bash
loct --version
loctree --version
loctree-mcp --version
loctree-lsp --version
```

Also run a real `loct scan` against a committed Git fixture. The required
release evidence covers macOS, Debian/Linux and the real Windows host; package
metadata alone is not runtime proof.

## Historical package boundary

Do not modify or deprecate these frozen platform packages during the 0.14.4
bootstrap:

- Gen1: `@loctree/darwin-arm64@0.8.16`,
  `@loctree/linux-x64-gnu@0.8.16`,
  `@loctree/win32-x64-msvc@0.8.16`;
- prior scoped delivery packages:
  `@loctree/loct-darwin-arm64`, `@loctree/loct-darwin-x64`, and
  `@loctree/loct-linux-x64-gnu`.

The wrapper identity `@loctree/loct` is current; only its old platform-package
implementation is frozen.

## Rollback

npm versions are immutable. If a published version is bad, publish a corrected
version and deprecate only the affected exact version with an upgrade message.
Never overwrite or silently repoint an existing version.

## References

- [npm trusted publishing](https://docs.npmjs.com/trusted-publishers)
- [npm publish](https://docs.npmjs.com/cli/commands/npm-publish)
- [optionalDependencies](https://docs.npmjs.com/cli/configuring-npm/package-json#optionaldependencies)
