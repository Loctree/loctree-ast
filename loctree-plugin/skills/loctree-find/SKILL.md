---
name: loctree-find
description: Find exact identifier occurrences, definition/re-export sites, reverse imports, or broad discovery candidates. Use before creating symbols and when separating literal evidence from structural or fuzzy discovery.
argument-hint: "<query> [literal|where-symbol|who-imports|discover]"
allowed-tools:
  - mcp__loctree-mcp__find
  - Bash
  - Read
---

# /loctree:find — choose the evidence class

Do not mix literal proof with discovery candidates.

| Question | MCP | CLI |
|---|---|---|
| Where does this exact identifier occur? | `mode: literal` | `loct find Name` |
| Where is it defined or re-exported? | `mode: where-symbol` | `loct find Name --where-symbol` |
| Who imports this file? | `mode: who-imports` | `loct find path/file --who-imports` |
| What related symbols/params may exist? | discovery/symbol mode | `loct find --discover Terms` |

Plain CLI `find` is exact identifier-boundary literal search; `--literal` is an
explicit alias. Use `loct occurrences Name --compact` for terse evidence,
`--count-only`/`--group-by-file` for rollups, and `--limit/--offset` for paging.

### Multi-literal OR (anti-grep — one engine on every surface)

When the agent would reach for `grep -E 'A|B'`, prefer exact multi-literal OR
instead. Pipe form and multi-arg form are the same engine truth:

| Surface | Call |
|---|---|
| CLI | `loct find 'A\|B'` or `loct find A B` |
| MCP | `find(name="A\|B", mode="literal")` |
| LSP | `loctree/find` with `query: "A\|B"`, `mode: "literal"` |

Expect `match_mode: multi_literal` and non-zero hits when either identifier
exists in the indexed universe. Real regex still uses `loct find --regex` /
discovery modes — simple identifier segments only auto-split on `|`.

`--discover` opts into AST/parameter/regex/fuzzy candidates. Label these as
candidates and verify decisive ones with literal find, where-symbol, body, and
direct source reads.

## Reporting

State:

1. mode used;
2. snapshot/root and freshness;
3. total matches, emitted matches, and truncation/paging state;
4. definition/re-export sites separately from occurrences;
5. coverage caveats for ignored, generated, unsupported, or dynamic surfaces.

Useful demonstrated behavior:

- `LOCT_OPEN_BROWSER` does not collapse into `LOCT_OPEN_BROWSER_ENV`.
- `hotspot` does not collapse into `hotspots`.
- In live checks, `BruteForceAdapter` matched an independent 38/38 exact-word
  count and `ScaffoldArtifactStore` matched 22/22; `where-symbol` then returned
  the two meaningful definition/re-export sites.

Do not claim global absence from an indexed-scope result. Do not claim that
`--discover --limit` globally bounds every output section unless the installed
version's receipt demonstrates it.
