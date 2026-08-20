# Third-Party Licenses (generated with `cargo license -j`)

> **Loctree's own source code is licensed under BUSL-1.1.** See
> [`../LICENSE`](../LICENSE) for parameters (Change Date 2030-04-13,
> Change License Apache-2.0). The licenses listed below cover only
> the upstream Rust dependencies vendored or linked into Loctree
> binaries — they are unaffected by Loctree's own license choice.

## How this was produced
- Installed tool: `cargo install cargo-license`
- Ran per crate (JSON saved under `licenses/`):
  - `cd loctree-rs   && cargo license -j > ../licenses/loctree-rs.json`
  - `cd reports      && cargo license -j > ../licenses/reports.json`
  - `cd landing      && cargo license -j > ../licenses/landing.json`
- Parsed and checked for non-permissive terms (GPL/AGPL/LGPL).

## License distribution (all crates combined)
- Total deps scanned: 681
- Dominant licenses: `Apache-2.0 OR MIT`, `MIT`, `Unicode-3.0`
- Unique licenses seen: 13 (full list in JSON files)

## Potentially sensitive findings
- One dependency appears with a tri-license `Apache-2.0 OR LGPL-2.1-or-later OR MIT`:
  - `r-efi` v5.3.0 (seen in loctree-rs, reports, landing)
  - Safe path: adopt the permissive side of the tri-license (Apache-2.0 or MIT). No hard copyleft required.
- No GPL/AGPL-only dependencies detected.

## JSON artifacts
- `licenses/loctree-rs.json`
- `licenses/reports.json`
- `licenses/landing.json`

These files include per-dependency name, version, and declared license. Use them for auditing or SBOM generation.
