# rg-vs-loct Scorecard

The scorecard turns "Loctree beats grep" into a repeatable gate. It compares a
deterministic fixture and a smoke pass on this repository across query classes:

- identifier: `loct find --literal` versus `rg --fixed-strings`
- prose phrase: fixed-string prose parity
- regex: `loct find --regex` versus `rg -e`
- symbol-definition: `loct find --where-symbol`
- who-imports: `loct query who-imports`

## Hard Gate

Correctness is hard-gated on the fixture:

```bash
cargo test -p loctree --test scorecard_rg_parity -- scorecard
```

The fixture lives at `loctree-rs/tests/fixtures/scorecard_rg_parity`. Literal
and prose probes compare per-file match counts. Semantic probes compare file
coverage, because `where-symbol` and `who-imports` are not expected to return
every textual reference.

## Measurement Report

Run the full scorecard locally:

```bash
bash scripts/scorecard.sh
```

By default this writes `scorecard.json` in the repository root. Use:

```bash
bash scripts/scorecard.sh --runs 5 --output /tmp/scorecard.json
```

The JSON schema is `scorecard.rg_vs_loct.v1`. Each row records:

- correctness: whether Loctree covers the rg baseline for that class
- latency: warm median for `N` samples
- output_cost: stdout bytes/chars for the same question
- lift: Loctree-only evidence such as role summaries, scope classification, and
  file context

Latency and output size are trend signals. They warn in the report but do not
fail CI. Fixture correctness is the blocking gate.

## CI

The main CI workflow installs `ripgrep`, runs the fixture gate explicitly, then
emits a one-sample scorecard JSON with:

```bash
bash scripts/scorecard.sh --runs 1 --output scorecard.json
```

`cargo test --workspace` also picks up the Rust integration test, but the
explicit step keeps the parity gate visible in Actions logs.
