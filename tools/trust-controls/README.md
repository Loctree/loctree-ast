# Loctree product trust controls

This pack replays the product-level negative controls behind the MATRIX P0
findings. It exercises built binaries, creates isolated Git fixtures, and fails
closed on the regressions that originally made Loctree look more certain than
its evidence allowed.

## Run

Build the three release binaries from one checkout, then run the pack:

```bash
cargo build --release -p loctree --bin loct --bin loctree \
  -p loctree-mcp --bin loctree-mcp
bash tools/trust-controls/run.sh
```

The default binary directory is `target/release`. Override it only when testing
an already-staged bundle:

```bash
LOCT_TRUST_BIN_DIR=/path/to/bundle/bin bash tools/trust-controls/run.sh
```

Every run writes machine-readable evidence below `target/trust-controls/` and
prints the exact directory. `git`, `jq`, and `awk` are required. Fixtures use an
isolated cache and set `LOCT_NO_GITIGNORE=1`, so the pack does not mutate the
source checkout or a user's Loctree cache.

See [MATRIX.md](MATRIX.md) for the P0 mapping and the external probes that this
repository cannot honestly close.
