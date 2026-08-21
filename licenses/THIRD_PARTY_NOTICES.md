# Third-Party Notices — Loctree Suite

**Product:** Loctree Suite (`loct` CLI, `loctree-mcp`, `loctree-lsp`, `loctree`
library, `report-leptos` / reports, and the VS Code / JetBrains /
Claude Code editor integrations).
**Notices version:** 1.0 — generated 2026-06-02 against workspace version `0.11.3`
(branch `feat/jetbrains-plugin`, commit `b7dff5e6`).
**Owner:** LibraxisAI — support@loctree.com

> **Loctree's own source code is licensed under BUSL-1.1.** See [`LICENSE`](../LICENSE)
> for parameters (Change Date **2030-04-13**, Change License **Apache-2.0**). The
> notices below cover **only** the third-party components statically linked into,
> bundled with, or distributed alongside Loctree binaries and editor packages.
> Their licenses are independent of and unaffected by Loctree's own license choice.

This document satisfies the attribution and notice-reproduction obligations of the
permissive and weak-copyleft licenses used by Loctree's dependencies. It is a
**release artifact** and MUST be shipped with every binary, installer, and editor
package (see [`commercial/RELEASE_COMPLIANCE_CHECKLIST.md`](../commercial/RELEASE_COMPLIANCE_CHECKLIST.md)).

---

## 1. How these notices were produced

| Surface | Tool | Source of truth |
|---|---|---|
| Rust crates | `cargo license -j` (cargo-license) | `Cargo.lock` resolved graph @ 0.11.3 |
| Rust SBOM (per-package) | `cargo license -j` per crate | [`licenses/loctree_rs.json`](loctree_rs.json), [`licenses/reports.json`](reports.json), [`licenses/landing.json`](landing.json) |
| npm (VS Code extension) | `package-lock.json` (lockfileVersion 3) | [`editors/vscode/package-lock.json`](../editors/vscode/package-lock.json) |
| JetBrains plugin | Gradle `intellijPlatform` graph | [`editors/jetbrains/build.gradle.kts`](../editors/jetbrains/build.gradle.kts) |

The machine-readable JSON files under [`licenses/`](.) are the canonical
per-package SBOM. This Markdown file is the human-readable, license-grouped
attribution required for distribution.

**Snapshot totals (Rust):** 572 crates resolved · 7 first-party (BUSL-1.1) ·
**565 third-party** · 23 unique license expressions.

---

## 2. Rust dependencies — distribution-relevant components

Loctree binaries (`loct`, `loctree-mcp`, `loctree-lsp`) are statically linked.
Every crate below is therefore part of the distributed object and its notice
obligations apply. Crates are grouped by SPDX license expression with the count
of distinct crate versions.

### 2.1 Permissive — attribution only (copyright + license text preservation)

| License expression | Count | Obligation |
|---|---:|---|
| `Apache-2.0 OR MIT` | 327 | Reproduce license + copyright; preserve any `NOTICE`. |
| `MIT` | 137 | Reproduce copyright + permission notice. |
| `Apache-2.0 OR Apache-2.0 WITH LLVM-exception OR MIT` | 15 | Elect MIT or Apache-2.0; reproduce chosen text. |
| `Apache-2.0` | 10 | Reproduce license; preserve `NOTICE` files (Section 4 of Apache-2.0). |
| `MIT OR Unlicense` | 10 | Elect MIT; reproduce copyright. |
| `ISC` | 5 | Reproduce copyright + permission notice. |
| `BSD-3-Clause` | 3 | Reproduce copyright + 3 clauses (incl. non-endorsement). |
| `Apache-2.0 OR ISC OR MIT` | 3 | Elect MIT/Apache-2.0; reproduce chosen text. |
| `Apache-2.0 OR MIT OR Zlib` | 3 | Elect a permissive option. |
| `CC0-1.0` | 3 | Public-domain dedication; no obligation (attribution courtesy). |
| `(Apache-2.0 OR MIT) AND Unicode-3.0` | 2 | Permissive **and** Unicode-3.0 notice (see 2.3). |
| `Apache-2.0 OR BSD-2-Clause OR MIT` | 2 | Elect a permissive option. |
| `BSL-1.0` | 1 | Boost license; no binary-distribution attribution required. |
| `Apache-2.0 OR BSL-1.0` | 1 | Elect Apache-2.0 or BSL-1.0. |
| `Apache-2.0 OR BSD-1-Clause OR MIT` | 1 | Elect a permissive option. |
| `Apache-2.0 WITH LLVM-exception OR BSL-1.0` | 1 | Elect BSL-1.0 (cleanest). |
| `Zlib` | 6 | Reproduce zlib notice; mark if altered (we do not alter). |

### 2.2 BSD-3-Clause — explicit non-endorsement notice required

These crates carry the third (non-endorsement) clause and MUST be attributed:

- `curve25519-dalek` (BSD-3-Clause)
- `ed25519-dalek` (BSD-3-Clause)
- `subtle` (BSD-3-Clause)
- `matchit` (BSD-3-Clause AND MIT — both apply)

> Neither the names of the copyright holders nor the names of their contributors
> may be used to endorse or promote products derived from this software without
> specific prior written permission.

### 2.3 Unicode-3.0 — Unicode data-files notice required

18 ICU crates (`icu_collections`, `icu_locale_core`, `icu_normalizer`,
`icu_normalizer_data`, `icu_properties`, `icu_properties_data`, `icu_provider`,
`litemap`, `potential_utf`, `tinystr`, `writeable`, `yoke`, `yoke-derive`,
`zerofrom`, `zerofrom-derive`, `zerotrie`, `zerovec`, `zerovec-derive`) plus the
`Unicode-3.0` component of `unicode-id-start` and `unicode-ident` are governed by
the **Unicode License v3**. The Unicode data-and-software notice (Section 5.2)
MUST be reproduced.

### 2.4 `ring` — combined Apache-2.0 AND ISC (both apply)

`ring 0.17.14` is licensed `Apache-2.0 AND ISC`. Its `LICENSE` aggregates
OpenSSL/BoringSSL-derived code under ISC-style and Apache-2.0 terms. Reproduce
the upstream `ring` LICENSE verbatim — it is **not** a choice; both sets apply.

### 2.5 Weak copyleft — MPL-2.0 (file-level, source-availability obligation)

Two crates are MPL-2.0 licensed. MPL-2.0 is **file-scoped** copyleft: the
obligation attaches only to MPL-covered source files, and only if they are
**modified**. Loctree links both **unmodified**.

| Crate | Version | Linked into | Path |
|---|---|---|---|
| `colored` | 3.1.1 | `loct`, `loctree-lsp`, `loctree-mcp` | direct dep of `loctree` |
| `option-ext` | 0.2.0 | `loct`, `loctree-lsp`, `loctree-mcp` | transitive via `dirs` → `dirs-sys` |

**Obligation discharged by:** (a) reproducing the MPL-2.0 text (Section 8 below);
(b) stating that the files are used unmodified; (c) making the corresponding
source available — satisfied by the public upstream repositories:
- `colored` → https://github.com/colored-rs/colored
- `option-ext` → https://github.com/soc/option-ext

> If Loctree ever **forks or patches** these crates, the modified files must be
> released under MPL-2.0 and the source offer updated. Tracked in the release
> checklist.

### 2.6 Multi-licensed crates where Loctree elects the permissive option

For completeness and auditability, Loctree's distribution **elects the permissive
side** of every disjunctive (`OR`) license and does not rely on any copyleft term:

| Crate | Declared | Loctree elects |
|---|---|---|
| `self_cell 1.2.2` | `Apache-2.0 OR GPL-2.0` | **Apache-2.0** (GPL-2.0 not used) |
| `r-efi 5.3.0`, `r-efi 6.0.0` | `Apache-2.0 OR LGPL-2.1-or-later OR MIT` | **MIT / Apache-2.0** (LGPL not used) |
| `ryu 1.0.23` | `Apache-2.0 OR BSL-1.0` | **Apache-2.0** |
| `fiat-crypto` | `Apache-2.0 OR BSD-1-Clause OR MIT` | **MIT** |

This election is binding for the distributed binaries and removes all copyleft
exposure other than the two file-scoped MPL-2.0 crates above.

---

## 3. JavaScript / npm dependencies — VS Code extension

The published `.vsix` bundles **only the runtime dependency closure**. The
`devDependencies` (build/test/packaging — `eslint`, `typescript`, `@vscode/vsce`,
the Azure SDK, etc.) are **not** redistributed and impose no distribution-side
obligation.

**Runtime closure shipped in the `.vsix`:**

| Package | Version | License |
|---|---|---|
| `vscode-languageclient` | 9.0.1 | MIT |
| `vscode-jsonrpc` | 8.2.0 | MIT |
| `vscode-languageserver-protocol` | 3.17.5 | MIT |
| `vscode-languageserver-types` | 3.17.5 | MIT |
| `semver` | 7.7.3 | ISC |
| `minimatch` | 5.1.9 | ISC |
| `brace-expansion` | 2.1.0 | MIT |
| `balanced-match` | 1.0.2 | MIT |

All runtime npm dependencies are MIT or ISC — attribution only. The extension
wrapper itself is published under **MIT** (see `editors/vscode/package.json`); the
`loctree-lsp` binary it downloads/bundles remains **BUSL-1.1**.

**Build-time only (NOT redistributed), flagged for awareness:**
- `@vscode/vsce-sign*` — declared `SEE LICENSE IN LICENSE.txt` (Microsoft
  proprietary `.vsix` signing helper). Used during packaging on developer/CI
  machines only; never shipped to end users. No redistribution right exercised.
- `typescript` (Apache-2.0), `eslint` (MIT), Azure Identity SDK (MIT),
  `argparse` (Python-2.0, via `js-yaml`) — all build-time.

---

## 4. JetBrains plugin

- The IntelliJ Platform SDK is consumed **at build/test/verify time only** via the
  Gradle `intellijPlatform` plugin. The published plugin artifact does **not**
  redistribute the IntelliJ Platform.
- `junit:junit:4.13.2` (EPL-1.0) is a **test-only** dependency, not shipped.
- Distribution channel is the **JetBrains Marketplace**, governed by the JetBrains
  Marketplace Agreement (separate from this product license).
- The plugin downloads/resolves the `loctree-lsp` binary, which remains BUSL-1.1.

---

## 5. Canonical license texts

The full canonical texts below MUST accompany the distribution. For the dozens of
crates under each shared license, a single reproduction of the license text plus
the per-package copyright lines in the SBOM JSON satisfies the obligation.

### 5.1 MIT License
```
Permission is hereby granted, free of charge, to any person obtaining a copy of
this software and associated documentation files (the "Software"), to deal in the
Software without restriction, including without limitation the rights to use,
copy, modify, merge, publish, distribute, sublicense, and/or sell copies of the
Software, and to permit persons to whom the Software is furnished to do so,
subject to the following conditions:

The above copyright notice and this permission notice shall be included in all
copies or substantial portions of the Software.

THE SOFTWARE IS PROVIDED "AS IS", WITHOUT WARRANTY OF ANY KIND, EXPRESS OR
IMPLIED, INCLUDING BUT NOT LIMITED TO THE WARRANTIES OF MERCHANTABILITY, FITNESS
FOR A PARTICULAR PURPOSE AND NONINFRINGEMENT. IN NO EVENT SHALL THE AUTHORS OR
COPYRIGHT HOLDERS BE LIABLE FOR ANY CLAIM, DAMAGES OR OTHER LIABILITY, WHETHER IN
AN ACTION OF CONTRACT, TORT OR OTHERWISE, ARISING FROM, OUT OF OR IN CONNECTION
WITH THE SOFTWARE OR THE USE OR OTHER DEALINGS IN THE SOFTWARE.
```
Per-package copyright holders are listed in the SBOM JSON (`authors` field).

### 5.2 Apache License 2.0
Full text: https://www.apache.org/licenses/LICENSE-2.0 — reproduce `LICENSE-APACHE`
verbatim in the distribution. Apache-2.0 Section 4(d): preserve any `NOTICE` file
content from `anyhow`, `rmcp`, `scc`, `sdd`, and other Apache-licensed crates.

### 5.3 ISC License
```
Permission to use, copy, modify, and/or distribute this software for any purpose
with or without fee is hereby granted, provided that the above copyright notice
and this permission notice appear in all copies.

THE SOFTWARE IS PROVIDED "AS IS" AND THE AUTHOR DISCLAIMS ALL WARRANTIES WITH
REGARD TO THIS SOFTWARE INCLUDING ALL IMPLIED WARRANTIES OF MERCHANTABILITY AND
FITNESS. IN NO EVENT SHALL THE AUTHOR BE LIABLE FOR ANY SPECIAL, DIRECT, INDIRECT,
OR CONSEQUENTIAL DAMAGES OR ANY DAMAGES WHATSOEVER RESULTING FROM LOSS OF USE,
DATA OR PROFITS, WHETHER IN AN ACTION OF CONTRACT, NEGLIGENCE OR OTHER TORTIOUS
ACTION, ARISING OUT OF OR IN CONNECTION WITH THE USE OR PERFORMANCE OF THIS
SOFTWARE.
```

### 5.4 BSD-3-Clause License
```
Redistribution and use in source and binary forms, with or without modification,
are permitted provided that the following conditions are met:

1. Redistributions of source code must retain the above copyright notice, this
   list of conditions and the following disclaimer.
2. Redistributions in binary form must reproduce the above copyright notice, this
   list of conditions and the following disclaimer in the documentation and/or
   other materials provided with the distribution.
3. Neither the name of the copyright holder nor the names of its contributors may
   be used to endorse or promote products derived from this software without
   specific prior written permission.

THIS SOFTWARE IS PROVIDED BY THE COPYRIGHT HOLDERS AND CONTRIBUTORS "AS IS" AND
ANY EXPRESS OR IMPLIED WARRANTIES [...] ARE DISCLAIMED. [full disclaimer applies]
```

### 5.5 Mozilla Public License 2.0 (MPL-2.0)
Full text: https://www.mozilla.org/en-US/MPL/2.0/ — reproduce verbatim. Applies to
`colored` and `option-ext` (used unmodified; source links in Section 2.5).

### 5.6 Unicode License v3 (Unicode-3.0)
Full text: https://www.unicode.org/license.txt — reproduce the Unicode data-files
and software notice for the ICU crate family (Section 2.3).

### 5.7 Other reproduced texts
- **Zlib** — https://opensource.org/license/zlib (mark "altered" only if changed; not altered here).
- **BSL-1.0 (Boost)** — https://www.boost.org/LICENSE_1_0.txt (no binary attribution required, reproduced for completeness).
- **CC0-1.0** — https://creativecommons.org/publicdomain/zero/1.0/legalcode (no obligation).
- **ring** — reproduce upstream `ring/LICENSE` (Apache-2.0 AND ISC aggregate).

---

## 6. Maintenance

Regenerate this file on every dependency change and before every release:

```bash
cargo license -j > licenses/loctree_rs.json          # from loctree-rs/
cargo license  --avoid-build-deps                    # grouped human review
# then refresh the grouped tables above and bump the Notices version
```

The release gate ([`commercial/RELEASE_COMPLIANCE_CHECKLIST.md`](../commercial/RELEASE_COMPLIANCE_CHECKLIST.md))
blocks any release whose dependency graph introduces a license not already
cleared in this document.

---

*This file enumerates third-party obligations only. It is not legal advice; see
[`commercial/LICENSE_COMPLIANCE_REPORT.md`](../commercial/LICENSE_COMPLIANCE_REPORT.md)
for the audited risk analysis and the legal-review checklist.*
