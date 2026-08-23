# Windows-inclusive release research log — 2026-08-23

Status: corrective releases in progress. The immutable `v0.14.3` tag exposed
Windows release-contract failures after four other targets passed. The fixes
are merged to public `main`; AICX `0.12.5` and Loctree `0.14.4` preserve the
failed-tag provenance instead of rewriting it.

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

## Remaining proof before corrective publish

1. AICX 0.12.5 is complete: signed merged-main release, GitHub assets, npm
   packages, hosted cold installs on all three OSes, and a real SSH Windows
   cold install all passed.
2. The synchronized Loctree 0.14.4 branch rehearsal with AICX 0.12.5 passed all
   five bundles. Merge the release PR, sign the exact merged-main tag, and
   repeat the five-platform proof on the immutable tag before publication.
3. Advance the public installer and release registry through the merged,
   deprivatized `loctree-com` VM deployment contract.
4. Re-run real client verification on Windows plus macOS and Debian/Linux
   against the final corrective versions.

## npm first-publish bootstrap finding

The live registry check before dispatch found five absent Gen3 scoped npm
identities (`@loctree/loctree` plus four `@loctree/loctree-*` platform
packages); bare `loctree` exists only on its legacy 0.8.x line. npm trusted
publishing cannot be configured for a package that does not yet exist, while
the workflow had already removed its documented bootstrap-token path. An OIDC
dispatch was therefore guaranteed to fail even though its build artifacts
could be correct.

The repaired workflow makes bootstrap an explicit dispatch input, verifies the
granular token before npm packaging, retains OIDC/provenance, and publishes
each immutable package version idempotently so a partial first run can be
retried safely. After v0.14.3 creates the five scoped identities and advances
bare `loctree`, all six package identities need trusted publishers; future
dispatches then use OIDC only.

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
