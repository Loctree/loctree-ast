# crates.io

The crates.io release is still the canonical Rust package surface:

- `loctree` — published at 0.13.0; install with `cargo install --locked loctree`
- `report-leptos` — published with the renderer line

`loctree-mcp` and `loctree-lsp` are in-tree distribution/runtime crates with
`publish = false` in 0.13.0. They ship through the signed bundle, npm runtime
package, editor packages, and future thin-repo distribution tracks — not
crates.io.

## Yanked releases

- `0.10.5` was yanked on 2026-05-27. Use `0.13.0` or newer.

Registry commands for the operator-owned crates.io action:

```bash
cargo yank -p loctree --vers 0.10.5
cargo yank -p report-leptos --vers 0.10.5
cargo yank -p loctree-mcp --vers 0.10.5
```

This channel is necessary, but no longer sufficient on its own. The rest of the
`distribution/` tree exists so the product can be installed by normal humans,
not only Rust users.
