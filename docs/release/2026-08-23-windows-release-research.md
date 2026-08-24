# Windows-inclusive release research log — 2026-08-23

Status: corrective releases complete. The immutable `v0.14.3` tag exposed
Windows release-contract failures after four other targets passed. The fixes
landed through reviewed public-main commits; AICX `0.12.5` and Loctree
`0.14.4` preserve the failed-tag provenance instead of rewriting it. Public
GitHub, crates.io, npm, installer, and production-web surfaces are verified.

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
  PR #69 restores the complete VS Code, JetBrains, and Neovim source surface
  from `loctree-suite/develop`. JetBrains Windows x64 now resolves the raw
  `.exe` release asset with the existing fail-closed checksum verifier. Its
  76 tests and plugin build pass; the VS Code typecheck and compile pass.
- The first exact-main AICX packaging run (`32645968371`) passed Cargo, Linux,
  and macOS, but Windows stopped in release-channel validation before bundle
  construction. Native Windows Python emitted CRLF; Bash command substitution
  retained the carriage return on the first two lines of the optional-package
  stream, so two visibly identical `0.12.3` strings compared unequal. AICX PR
  #56 normalizes that transport boundary and adds a self-test which forces
  Python CRLF output. The full Windows bundle remains unproven until the
  repaired exact-main matrix reaches its build and npm dry-run steps.
- `make version VERSION=0.14.3` synchronized Cargo, npm, JetBrains, VS Code,
  and installer surfaces. Its first test pass was invalidated by an operator
  mistake: `GIT_AUTHOR_NAME=codex` was exported to the whole process and
  overrode four Git fixture identities. Re-running `cargo test --workspace`
  without that environment produced a clean result, including all 1,965 core
  tests and doctests. Authorship was scoped only to the final commit.
- Semgrep emitted the VS Code version probe as a formally suppressed SARIF
  result: the executable is reduced to a canonical file or the exact platform
  binary name, argv is fixed to `--version`, and no shell is used. GitHub still
  opened the suppressed record as a blocking code-scanning alert. PR #69 now
  filters only SARIF results carrying formal suppressions before upload; the
  local workflow-equivalent sample changed from 16 records to 15 while all 15
  unsuppressed findings remained.

## Privacy gate

`vc-deprivatize` is required before every public merge. The Windows bundle
change introduced no private data. The restored complete editor source and
the isolated 0.14.3 release diff both pass verification. Their REVIEW findings
are the public product support address, the Marketplace publisher, and a
deterministic test timestamp; all are explicitly retained in the central
decision ledger.

The full public repository does not yet pass the same gate: it contains an
inherited baseline of 560 unambiguous findings, concentrated in historical
BACKLOG and CHANGELOG material. This release does not claim that baseline is
clean, and it does not mix a broad historical rewrite into the Windows release
cut. The changed release surface is clean; the repository-wide cleanup remains
a separately tracked public hygiene obligation.

A separate `loctree-com` branch was rejected from the release path even though
its CI was green: compared with `origin/main`, it added raw conversation
extracts containing operator home paths, session identifiers, private people,
and a bundled archive. The branch was preserved for archaeology, but PR #15
was closed and will not be deployed. Site advancement will be rebuilt as a
narrow clean-main change after release assets exist.

## Corrective publish result

1. AICX 0.12.5 is complete: signed merged-main release, GitHub assets, npm
   packages, hosted cold installs on all three OSes, and a real SSH Windows
   cold install all passed.
2. Loctree 0.14.4 is complete: the immutable tag passed the five-platform
   combined-bundle matrix, the public release carries nineteen artifacts, the
   eight-job thin/npm matrix passed from repaired main, and all four crates and
   seven npm identities are public.
3. The public installer and signed release registry were advanced through the
   merged, deprivatized `loctree-com` VM deployment contract. The canonical
   live index reports both current and stable as 0.14.4.
4. Real client proofs passed on Windows, macOS, and Debian/Linux. The remaining
   Windows PATH preference for older operator Cargo binaries is documented as
   local installation ownership, not release failure.

## npm first-publish bootstrap finding

The live registry check before dispatch found five absent Gen3 scoped npm
identities (`@loctree/loctree` plus four `@loctree/loctree-*` platform
packages); bare `loctree` remained on 0.8.16 and `@loctree/loct` on 0.13.1.
npm trusted publishing cannot be configured for a package that does not yet
exist, so a first OIDC dispatch was guaranteed to fail even with correct build
artifacts.

The final contract makes 0.14.4 the last token-authenticated bootstrap and owns
it locally through `make npm-release-publish`: it consumes checksum-verified
public release bundles, reads the operator credential from `KEYS/.npm`, and
publishes each immutable version idempotently. The package graph contains seven
identities: four canonical platform packages plus canonical
`@loctree/loctree`, maintained alias `@loctree/loct`, and recovered short form
`loctree`. After bootstrap, all seven receive trusted-publisher configuration;
`publish.yml` has no token lane and uses exact-tag OIDC with provenance only.

## Tag run Windows dependency-scope finding

The first v0.14.3 tag run proved four bundle targets, including Debian 12 musl,
but Windows compilation failed before packaging. `loctree-mcp/Cargo.toml` opened
`[target.'cfg(unix)'.dependencies]` for `libc` and then declared `tracing`,
`tracing-subscriber`, and `clap` without reopening a cross-platform table.
TOML kept those three dependencies inside the Unix-only table, so Linux and
macOS were green while Windows reported unresolved imports and missing clap
derive attributes. The fix moves the Unix table below the shared dependencies
and adds a cargo-metadata regression that asserts the three dependencies remain
unscoped while `libc` stays `cfg(unix)`.

## Branch bundle Windows checksum-path finding

The repaired branch build (`32658638595`) then compiled all four native
Windows executables successfully, but stopped while importing the already
published AICX 0.12.3 Windows ZIP. The sidecar expected
`348245adc3272e2ebc0cc2bc8add9bb25bc2d9d1cb2f88ecc31d87a2ea236ec9`; the
local computation returned the same digest with a leading backslash. GNU
checksum output prefixes the digest with `\` when its filename field requires
escaping, and a native Windows path exercises that format.

`sha256_file` now feeds the archive over stdin, so the checksum tool has no
Windows filename to escape. The Windows bundle contract replaces `shasum`
with a strict fake that accepts only stdin and returns the expected digest;
the old filename-argument implementation fails that test. This preserves
cryptographic verification and fixes only the textual transport boundary.

The next branch run (`32659590071`) proved that checksum verification now
passes, then exposed the archive-output equivalent: GNU tar interpreted the
absolute `D:\...` output path as its historical remote `host:path` syntax and
attempted to connect to host `D`. Bundle creation now writes a relative archive
beside the staging directory and moves the finished file into the requested
distribution directory. The Windows contract wraps tar and rejects absolute or
drive-qualified create operands, so this cannot regress behind a Linux-only
test path.

The third branch run (`32660678820`) proved archive creation on Windows and all
four non-Windows bundles, then failed while extracting selected files for
verification. Git Bash exposed `RUNNER_TEMP` as `D:\\a\\_temp`; GNU tar does
not apply MSYS argument conversion to its `-C` operand and tried to open the
literal escaped drive path. The build job now normalizes the runner temp once
with `cygpath -u` on Windows and publishes `BASH_RUNNER_TEMP` to every later
Bash step. Bundle verification, the six-binary runtime smoke, and raw LSP
staging therefore share one POSIX path contract. The Windows bundle test
asserts both the normalization and the absence of the three known raw
`RUNNER_TEMP` tar paths.

## AICX npm Windows extractor finding

After the AICX 0.12.3 binary release succeeded, all four npm packages were
published and cold installs passed on hosted macOS and Linux. Hosted Windows
still failed while the real SSH Windows host succeeded. The optional Windows
package downloaded the correct release ZIP, but its `postinstall.js` invoked
an unqualified `tar`. Git Bash placed GNU tar first on hosted CI and it could
not extract the ZIP; PowerShell placed the native Windows bsdtar first on the
real host.

AICX PR #58 resolves and validates `%SystemRoot%\\System32\\tar.exe` (falling
back to `%WINDIR%`) and invokes it without a shell. A metadata regression
forbids the unqualified Windows extraction call. A pre-release probe copied
the changed installer into an isolated directory on the real SSH Windows host,
downloaded and checksum-verified the public 0.12.3 ZIP, and ran both installed
binaries as `0.12.3+g8243654a` with exit 0. The exact temporary directory was
then removed; global npm and Cargo installations were untouched. Because npm
versions are immutable, this repair was first cut as AICX 0.12.4 rather than
overwriting 0.12.3. The 0.12.4 tag later failed its signed Windows build and
was superseded by the immutable corrective 0.12.5 release described below.

## Merged repair proof

Loctree branch run `32662090853` at `095bd210eb0475068d6ca616963c38c02d0c3282`
completed successfully on macOS arm64/x64, Linux GNU/musl, and Windows MSVC.
The Windows job passed archive construction, content verification, all six
version probes, committed-fixture scan, raw LSP staging, and artifact upload.
PR #70 then merged through protected checks as `ca1fea39d431ae6c50d80ba49e7379c67ce273cc`.

AICX PR #58 passed the complete hosted matrix on macOS, Linux, and Windows,
including default and native-GGUF tests, then merged as
`050e8244c3c4bf2b767ce4859ba74c7476845ffb`.

## AICX corrective release proof

The signed AICX `v0.12.4` run `32665708444` passed exact-tag verification,
macOS signing/notarization, and the Linux GPG bundle, but the serialized Windows
runner exhausted memory in the final single-process full-LTO `rustc` link.
Publish remained fail-closed. PR #60 keeps `opt-level=3` and stripping, disables
cross-crate LTO only for the low-memory signed Windows target, and adds a
`make version-check` contract covering serialization, LTO, and the LLVM-free
MSVC feature set. It merged as
`ced57997dd97a2b08960f35e3a657d7b0c49a200`.

The signed `v0.12.5` run `32669780693` then completed exact-tag verification,
macOS codesign/notary, Linux GPG + Debian packaging, Windows MSVC ZIP build,
all archive smokes, checksums, detached signatures, and GitHub Release publish.
Independent download verification passed every checksum and every GPG
signature; public macOS binaries reported `0.12.5+gced57997`.

npm run `32671741799` published `@loctree/aicx@0.12.5` plus all three platform
packages and passed empty-prefix cold installs on hosted macOS, Ubuntu, and
Windows. A separate real `ssh windows` install ran under a one-shot scheduled
task because that host limits individual SSH commands to about 30 seconds. It
returned npm exit 0 and both `aicx` and `aicx-mcp` as
`0.12.5+gced57997`; its isolated temp directory, task, script, result, and logs
were all removed. The mandatory AICX deprivatize pass reported no unambiguous
PII or secret leaks. Loctree 0.14.4 therefore selects AICX 0.12.5 as its
canonical combined-bundle input.

## Final Loctree branch rehearsal

Workflow-dispatch run `32672964365` completed successfully at exact release
head `3e6e090152c0db904048b6b5d2673990a5ed5112`. Input verification and all five
bundle jobs passed: macOS arm64/x64, Linux GNU/musl, and Windows MSVC. The
publish job was correctly skipped because this was a branch rehearsal rather
than an immutable `v*` tag push.

The Debian 12 musl smoke reported the extracted `loct` executable as statically
linked, ran `loct 0.14.4+g3e6e0901`, and scanned the one-file committed Git
fixture successfully without installing Git in the container. This directly
closes the original v0.14.2 `no git repository at '/fixture'` failure.

The hosted Windows job ran all six bundled executables from the archive:
Loctree `0.14.4+g3e6e0901` for `loct`, `loctree`, `loctree-mcp`, and
`loctree-lsp`, plus `aicx` and `aicx-mcp` at `0.12.5+gced57997`. Its committed
Git fixture scan completed with `Status: OK`, and the raw
`loctree-lsp-windows-x64.exe` asset plus checksum uploaded successfully. The
immutable tag run must repeat this proof before any publish button is used.

## Immutable 0.14.4 tag run and publish-gate finding

Signed tag `v0.14.4` points at merged main
`3e9eb0a74cb3c043d740de5fe7d8c93985d0a876`. Tag run `32675065929`
repeated the complete five-target build proof successfully: macOS arm64/x64,
Linux GNU/musl, and Windows MSVC all passed their bundle, version, archive,
fixture-scan, and raw-LSP contracts.

The final `Publish to Loctree/loctree-release` job nevertheless failed before
upload. Its archive-membership check ran `tar -tzf "$asset" | grep -Fxq
"$member"` under `set -o pipefail`. `grep -q` exited immediately on the valid
match, GNU tar received SIGPIPE while writing the remaining listing, and the
pipeline falsely reported that the archive lacked `loctree-lsp`. The log's
`tar: stdout: write error` is the distinguishing evidence; the build jobs had
already extracted and executed the same member successfully.

The repair queries the exact tar member directly, with no early-closing pipe.
A preflight regression forbids the `tar | grep -q` shape. Manual recovery runs
now execute the workflow definition from main but pin both source checkouts to
`refs/tags/v<version>`, so rerunning for 0.14.4 rebuilds the immutable tag rather
than accidentally stamping post-tag main into same-version binaries. npm and
web publication remain blocked until that recovery run publishes the verified
assets.

Before using the npm credential, the four relevant artifacts from failed run
`32675065929` were downloaded into an isolated temporary directory and passed
through `make npm-release-verify VERSION=0.14.4 ASSET_DIR=<artifact-dir>`. The
first rehearsals found two orchestration-only defects without reading the
token: macOS x64 is a core payload under the intentionally unsuffixed public
archive name, and wrapper-name sanitization was incorrectly applied to the
whole absolute staging path. After both repairs, all four archive checksums,
all sixteen staged binaries, the helper tests, four platform dry-packs, and
three wrapper dry-packs passed. No npm publish occurred during rehearsal.

## Manual recovery publish contract

Manual recovery run `32678237015`, launched from repaired main
`f158370e98307829e6a4afe2db8ec1760b4b0a55`, rebuilt the exact signed
`v0.14.4` tag and passed all five platform jobs. Linux GNU and musl, macOS
arm64 and x64, and Windows MSVC each completed bundle verification, runtime
smoke, committed-fixture scan, and artifact upload. This confirms both the
Debian 12 musl repair and the real Windows release path on the recovery route.

The final cross-repository publish was still skipped because its job-level
condition admitted only tag-push events, even though manual recovery had been
changed to build the immutable tag. The workflow therefore had two conflicting
manual-run contracts: exact-tag recovery in checkout, but rehearsal-only at the
publish boundary. The follow-up repair makes the contract single-valued:
manual recovery may reach publish only after the verify job proves that HEAD is
the exact `v<version>` tag and every platform build succeeds. Preflight now
locks both the exact-tag proof and manual publish reachability.

## Corrective publication and npm bootstrap proof

Recovery run `32681440907` completed successfully from the repaired workflow
while still checking out the immutable `v0.14.4` source. The public release in
`Loctree/loctree-release` was published on 2026-08-24 with nineteen
assets: five platform bundles, five per-bundle checksums, four raw LSP binaries,
four raw-LSP checksums, and aggregate `SHA256SUMS`. The platform set is macOS
arm64/x64, Linux GNU/musl x64, and Windows MSVC x64.

The one-time npm bootstrap then published all seven identities at 0.14.4:

- `@loctree/loctree`, `@loctree/loct`, and `loctree`;
- `@loctree/loctree-darwin-arm64` and `@loctree/loctree-darwin-x64`;
- `@loctree/loctree-linux-x64-gnu`;
- `@loctree/loctree-win32-x64-msvc`.

All three wrapper names passed isolated macOS installs. The canonical package
shape is script-free: npm selects one platform package through
`optionalDependencies`, and npm 11 emits no lifecycle-script approval warning.
This is now the reference packaging shape for companion binaries.

A real Windows host supplied two independent proofs. First, an isolated npm
prefix installed 0.14.4 without touching the active toolchain; all four
executables reported `0.14.4+g3e9eb0a7`, and a committed one-file Git fixture
scanned with exit 0. Later, an operator installed the short `loctree` wrapper
into the active global prefix. Direct invocation of its four npm shims again
reported `0.14.4+g3e9eb0a7`. The ordinary PATH still selected older Cargo
installs (`loct`/`loctree` 0.14.2 and `loctree-mcp` 0.8.16) ahead of those
shims. This is an ownership/shadowing finding, not a package-content failure;
the release process did not delete the operator's Cargo installation.

The production Linux host also installed `loctree@0.14.4`, reported all four
Loctree executables as `0.14.4+g3e9eb0a7`, and completed a real committed
fixture scan. Its first install met an existing global-bin entry and failed
closed with npm `EEXIST`; the operator explicitly resolved that active-prefix
collision. Release acceptance continues to use empty prefixes so `--force`
is never required as evidence.

## Main publish workflow hardening

The first post-bootstrap `Publish Releases` dispatch (`32683897382`) failed in
the full release quality gate because the hosted publish job installed
`protoc` but not `rg`. The parity fixture
`scorecard_rg_parity_fixture_matrix` correctly requires ripgrep, so this was a
workflow-image dependency defect rather than a Rust regression. PR #74 installs
both release-gate dependencies when absent; all hosted checks, dogfood jobs,
CodeQL, and Semgrep passed before squash merge as
`f0c0217bfdfd3109b10bc9d12cbdd7e0183cd9d4`.

Dispatch `32685354153` then passed the release gate and reached crates.io. Its
first two attempts failed at the upload boundary with `403 authentication
failed`. Packaging and verification had both completed, proving that this was
credential state rather than crate content. The configured GitHub secret and
the operator's local `cio...` token were both expired. A crate-restricted token
was generated for update publication of `report-leptos`, `loctree`, and
`loctree-mcp`, saved without printing it, and installed in both Cargo and the
GitHub Actions secret. The final attempt and its downstream thin-release jobs
are recorded in the terminal evidence section below.

Attempt 3 proved that the replacement token could upload: `report-leptos
0.14.4` became public. It did not publish `loctree`. Cargo stopped during local
packaging because `loctree` requires `loctree-ast = "^0.14.4"`, while the
workflow and the local release driver both omitted `loctree-ast` from their
publish sequence. The old recovery branch then hid that failure: fuzzy
`cargo search loctree | grep 0.14.4` matched an unrelated search result and
reported `loctree 0.14.4` as already published. `loctree-mcp` consequently
failed while resolving the still-absent `loctree = "^0.14.4"` dependency.

PR #75 replaces that incomplete model with the real crates.io dependency
chain: `report-leptos -> loctree-ast -> loctree -> loctree-mcp`. Exact
availability probes run from a neutral directory with the crates.io registry
named explicitly, so a same-name local workspace package cannot satisfy the
check. Tagged release source and release-control source are also separated:
the immutable tag remains under test and publication, while the poller is
pinned to the exact commit containing the running workflow. This matters for
recovery after a tag because the tag cannot contain a repair written later.
A fresh dispatch from merged main is required; rerunning an earlier failed run
would preserve its old workflow definition.

## Terminal publish closure

PR #75 was squash-merged as
`4e742584b31053010399957ab48d37c0544ec466`. Fresh dispatch `32689652655`
then proved the corrected Rust dependency chain end to end: all four crates
became public, all eight CLI/MCP artifact jobs succeeded, and both thin-repo
releases were published. The run failed later, before npm authentication,
because its npm job combined Node 20.20.2 with the moving `npm@latest` tag.
That tag had advanced to npm 12.0.2, whose engine rejected the selected Node
runtime. The monorepo release was consequently skipped.

PR #76 removed that toolchain drift. It pins Node 24 and the tested npm
11.17.0 trusted-publishing client, and its preflight contract rejects a return
to the moving latest tag. Local preflight, actionlint, ShellCheck, Semgrep, and
diff checks passed; vc-deprivatize reported no findings in the two changed
files. Hosted Linux/macOS, Linux/macOS dogfood, CodeQL, and Semgrep were all
green. The PR was squash-merged as
`690de4c04f87f1377cdb0790ad716406be523d52`.

Fresh dispatch `32692794671` ran from that exact main commit and completed
successfully. Its release gate passed; neutral crates.io probes found
`report-leptos`, `loctree-ast`, `loctree`, and `loctree-mcp` 0.14.4 already
published and preserved the immutable versions. All eight rebuilt artifacts
passed, including the complete Windows CLI and MCP jobs. The npm job reported
npm 11.17.0, verified all four platform packages and all three wrapper
identities as already public at 0.14.4, and performed no overwrite. Both thin
releases passed idempotently, and the final job created the public monorepo
release.

Independent public checks after the workflow completed found:

- `Loctree/loctree-release` v0.14.4: nineteen assets across macOS arm64/x64,
  Linux GNU/musl x64, and Windows MSVC x64;
- `Loctree/loct` v0.14.4: twelve CLI/MCP assets;
- `Loctree/loctree-mcp` v0.14.4: six MCP assets;
- `Loctree/loctree` v0.14.4: public source/orchestration release;
- all seven npm identities at 0.14.4;
- all four crates at 0.14.4 through neutral `cargo info --registry crates-io`.

The successful idempotent npm run proves the repaired Node/npm toolchain and
the package-presence path. It does not independently exercise an OIDC upload,
because immutable 0.14.4 packages already existed before that run. Registering
the seven npm trusted publishers remains an operator-authenticated hardening
step: npm requires a fresh TOTP even to inspect that account-level trust
configuration. This does not change the publication state of 0.14.4.

The mandatory post-release vc-deprivatize scan remained exactly at the
inherited repository baseline: 1,337 findings in the full review scan and 560
unambiguous findings in fail-closed verification. This research document
produced zero findings. The historical repository-wide hygiene debt is not
waived or misreported as release-introduced exposure.

## Stable web registry and installer truth

The release registry was published to the production VM through the repository
Make contract, not by copying ad hoc files. Its signed 0.14.4 manifest contains
all five platform targets and uses GPG fingerprint
`8868139E8A9A2291D067135FB979B60C7079E4D4`. Live
`releases/index.json` reports both `current` and `channels.stable` as 0.14.4.

PR #18, merged as `f39d311fa801d81ccbefa68d2f8933f80e3931c6`,
makes that registry reproducible: the default target set is the complete
five-platform matrix, release date is explicit, target names are validated,
and manifest plus index metadata are generated deterministically from verified
artifact bytes. Determinism, tamper, coverage, and unsafe-target tests passed,
as did the full local site gate and mandatory deprivatization pass.

Repository inspection nevertheless found a P0 split brain: the live release
index was current while the public bootstrap script still selected 0.13.1.
PR #19, merged as `2cd2a5322b4fb12c3216e3cfb6f4628c2a9194fb`, removes the numeric installer
default. The script now resolves `index.current`, preserves an exact
`LOCTREE_VERSION=x.y.z` override, and fails closed for an unavailable, missing,
or malformed version. Its regression proves dynamic resolution, offline exact
override, and invalid-version rejection on macOS Bash 3.2-compatible gates.

The first production `make deploy` then exposed an unrelated build-tool
split brain: Cargo.lock selected Wasm schema 0.2.120 while the machine-global
`wasm-bindgen-cli` was 0.2.122. PR #20, merged as
`1a9d3ec2814b9f18a55bef692a464799a4d60ec9`, provisions the exact locked CLI in
a versioned repository-local build-tools directory and routes build, watch, and
deploy through one wrapper. Its test proves first install, cache reuse,
stale-tool replacement, and PATH precedence. The real Wasm build and Linux
cross-build then passed.

The first atomic deployment restarted the service successfully but returned
non-zero while deleting its old rollback: exactly two retired files carried
the Linux immutable attribute. Their paths and count were verified, the
attribute was cleared only on those two files, and only the old rollback tree
was removed. A complete repeat of `make deploy` then passed build, rsync,
atomic swap, service restart, and direct upstream HTTP smoke (`200`). Release
files remained outside the site rsync and were not overwritten.

Finally, a fresh live macOS install used the public URL with an isolated install
directory and cache. It selected 0.14.4 from the index, verified the per-asset
checksum, aggregate checksum, detached GPG signature, and all six Apple code
signatures, then installed:

- `loct`, `loctree`, `loctree-mcp`, and `loctree-lsp` at
  `0.14.4+g3e9eb0a7`;
- `aicx` and `aicx-mcp` at `0.12.5+gced57997`.

The installed `loct` scanned a committed fixture with `Status: OK`. The entire
temporary install, fixture, and isolated cache were moved to Trash afterward.

## Research intake after release

These observations are inputs to the next research sprint, not changes folded
into the corrective release:

1. The curl bundle already carries Loctree and AICX together, but the public
   npm description promises "one npm install" while exporting only the four
   Loctree binaries. If AICX overlay becomes a required Loctree capability,
   all wrapper identities need a six-binary contract plus a real overlay cold
   smoke.
2. AICX 0.12.5 installs successfully on macOS and Windows, but npm 11.17 warns
   that its wrapper and platform package run unapproved postinstall scripts.
   npm 11.5 runs the same hooks without that warning. The accepted follow-up
   direction is script-free platform packages carrying ready binaries, modeled
   on Loctree's current npm shape. AICX owns that implementation separately.
3. MCP/HTTP service configuration and launchd/systemd definitions may be useful
   distribution artifacts, but network installs must not silently create boot
   persistence. Installation and start must remain an explicit opt-in command.
4. The Codex post-compaction recall is functionally correct but renders as a
   nested box with awkward wrapping and a decorative brain pictogram. This is
   an apparatus-quality finding for the research sprint. The protected recall
   hooks were not modified during release.
