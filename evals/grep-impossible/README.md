# grep-impossible — the eval that measures the engine, not the basket

## Why this exists

Operator, 2026-08-19 (relayed verbatim from the vc-layouty session):

> "loctree ma wszystkie możliwe parsery narzędzia analizatory i wszystko, ale
> nic z tego powera nie jest wykorzystane jak należy, a agenty się jarają, że
> ma 100% grep/rg parity [...] to nie jest grep — warstwa find to tylko wycinek
> pozwalający wejść do świata prawdziwego code intelligence [...] dzisiejsze
> walczenie o coraz lepszy find literal i regex to jak walka o to, żeby ferrari
> miało na masce koszyczek pięknie przystrojony."

The benchmark defines the product. As long as the KPI is grep-parity, the
roadmap converges on grep. This suite is the counter-KPI: **19 questions that
text search can never answer** — all 19 answered as of `1f2f99af` (wave w1
closed the last three: module redirect, shape honesty, one-number-one-truth)
— each bound to a live command with a machine-checked assertion. A failing
question here is a product gap, not test noise.

Run it:

```bash
python3 evals/grep-impossible/run.py          # scorecard, exit 1 on any gap
python3 evals/grep-impossible/run.py --list   # the questions + why grep can't
LOCT_BIN=/path/to/loct python3 evals/grep-impossible/run.py   # measure a specific build
```

**Which binary answers.** The questions are live commands against this
checkout, so the runner resolves `LOCT_BIN` → `target/release/loct` → PATH,
and prints the binary and its `commit=` before the scorecard. This is not
cosmetic: on 2026-08-19 a verifier inherited a foreign `loct` (built from
`a3e86fbf`) and scored question 19 as a FAIL — 85 vs 72 — against a worktree
where the split-brain was already fixed. A mismatch between the binary's
commit and `HEAD` now prints `STALE` on the provenance line; rebuild
(`cargo build --release -p loctree --bin loct`) before believing a failure.

## The sixteen answered questions

| # | Question | Surface |
|---|---|---|
| 1 | Blast radius of a file change — direct AND transitive, with depth | `impact` |
| 2 | Who imports this file (module paths, not strings) | `find --who-imports` |
| 3 | Definition site vs the dozens of textual mentions | `query where-symbol` |
| 4 | Full symbol body without knowing file or offsets | `body` |
| 5 | Dead exports — with the REASON and what was checked | `follow dead` |
| 6 | Import cycles, classified hard vs benign | `follow cycles` |
| 7 | Structural twins / barrel chaos across files | `follow twins` |
| 8 | One accounted structural-health aggregate | `health` |
| 9 | Every silencer by kind across 9 ecosystems | `suppressions` |
| 10 | Env vars declared but never read (two-world join) | `env-truth` |
| 11 | Env read-sites with file:line and access kind | `env-truth` |
| 12 | Tauri handler wired end-to-end across the language bridge | `trace` |
| 13 | Are two task framings one job or a smear — scored | `prism` |
| 14 | Proof of ABSENCE with a scanned/total receipt | `find` (receipt) |
| 15 | Scoped context with a cache fingerprint of what was covered | `context --scope` |
| 19 | One health number across every surface — or a named difference | `findings --summary` / `--for-ai` |

## Known gaps (strict-xfail)

Observed live by the vc-layouty session on 2026-08-19, verified and encoded
here the same day. Each asserts the DESIRED behavior; the runner reports
`GAP` until the engine answers, then refuses to pass until the question is
promoted (`gap=False`) — a fixed gap cannot stay silently unpromoted.

**Open gaps today: none** (`19/19 answered, 0 known gaps` at `1f2f99af`). The
four questions below were encoded as gaps on 2026-08-19 and promoted the same
day by wave w1; the mechanism stays so the next observed gap lands here first.

Question 19 (one health number, or a named difference) was promoted on
2026-08-19: `findings --summary` and `--for-ai` build their `HealthMetrics`
through `analyzer::health_inputs::structural_defects` and agree; the audit
collector scores a strictly narrower set and says so in the report itself
("audit basis") instead of emitting a silent third number.

Answered and promoted (2026-08-19):

| # | Was | Now |
|---|---|---|
| 16 | `body` on a module name dead-ended: body → hint → where-symbol → 0 results | `mod health_score;` is a definition site, so `where-symbol` answers with `analyzer/mod.rs:48` and `body` redirects to the module's own symbols (`calculate_health_score`, `HealthScore`) with the fuzzy hits alongside (W1-a) |
| 17 | `pub mod health_score;` narrated as "state flag / single-writer" | the dataflow labels (`single_writer`, `read_only`) require the sole definition to be a *value* declaration; a module falls back to `mixed` and the note names the `mod` introducer (W1-b) |
| 18 | 1665 hits for `twin\w+`, first screens are CHANGELOG | emission is ranked (scope > role > file/line/column) inside `apply_report`, before paging, so every consumer — CLI, LSP, MCP — pages the same ranked order (W1-b) |

Same disease class, closed in the same cut: `loct occurrences X --regex`
answered "Unknown option" while `loct find --regex` was the canonical
spelling — twin commands with asymmetric flag contracts. `occurrences` now
takes `--regex` on the same engine, coverage line and paging.

Closed from the same class (2026-08-19): `--help-full` and README advertised
jq queries (`.dead_parrots[]`, `.cycles[]`, `.summary.health_score`) that the
snapshot schema refuses — now guarded by
`loctree-rs/tests/help_examples_truth.rs`, which executes every advertised
JQ example against a real snapshot. Engine backlog, same vein: jq over the
findings artifacts (`dead.json`/`findings.json` exist; the query surface
reads only snapshot.json), and a missing-key answer ("no key `.summary`;
available: [...]") instead of silent nulls. The wider docs sweep
(10+ files still promising `.dead_parrots`) awaits its own cut.

## Surface truth (inventoried 2026-08-19, v0.14.2 line)

- The CLI engine exposes **~35 commands** of real intelligence: `slice`,
  `impact`, `follow` (dead/cycles/twins/hotspots/trace/commands/events/
  pipelines), `trace`, `routes`, `dist`, `layoutmap`, `env-truth`, `prism`,
  `diff`, `health`, `tagmap`, `crowd`, `body`, `anchors`, `lint`, ...
- The MCP surface exposes **12 tools**: context, repo-view, focus, slice,
  find, impact, tree, follow, suppressions, prism, body, diff.
- **Not reachable over MCP today:** `env-truth`, `trace`, `routes`, `dist`,
  `layoutmap`, `tagmap`/`crowd`, `health`/`findings`. Questions 10–12 of this
  suite can only be answered by shelling out.
- The daily agent loop lives on ~6 of these. That asymmetry — not find
  quality — is the growth surface.

## Direction (signed: claude, front owner; direction mine unless quoted)

1. **This suite is the KPI.** Extend it before extending any parity suite.
   New engine capability ships with a grep-impossible question or it does not
   count as capability.
2. **find returns answers, not hits** — role, fan-in, and the next structural
   step (`slice`/`impact`) on every hit. The CLI already annotates roles;
   the MCP `find` should reach parity with that, not with grep.
3. **Lift CLI-only intelligence into MCP** — `env-truth`, `trace`, `routes`,
   `health` first; they answer questions 10–12 that agents currently cannot
   ask through the tool surface.
