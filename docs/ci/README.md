# Loctree CI/CD Integration

> Artifact-first structural analysis for your CI pipeline

## Quick Start

Add this badge to your README:

```markdown
[![loctree](https://raw.githubusercontent.com/Loctree/loctree-suite/main/assets/loctree-badge.svg)](https://github.com/Loctree/loctree-suite)
```

Result: [![loctree](https://raw.githubusercontent.com/Loctree/loctree-suite/main/assets/loctree-badge.svg)](https://github.com/Loctree/loctree-suite)

## GitHub Actions Workflow

### Minimal (Python/JS/TS projects)

```yaml
name: Loctree
on: [push, pull_request]

jobs:
  analyze:
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v4
      - uses: actions/setup-node@v4
        with:
          node-version: 20
      - run: npm install -g @loctree/loctree
      - run: loct auto
      - run: |
          HEALTH=$(loct agent | jq -r '.summary.health_score')
          echo "Health: $HEALTH/100"
          [ "$HEALTH" -lt 50 ] && exit 1 || exit 0
```

### Full (with SARIF + PR comments)

See [examples/ci/loctree-ci-v2.yml](../examples/ci/loctree-ci-v2.yml)

## Key Commands for CI

| Command | Purpose | Exit Code |
|---------|---------|-----------|
| `loct auto` | Full scan → cached artifacts | Always 0 |
| `loct lint --fail` | Structural lint | 1 if issues |
| `loct dead --confidence high` | Dead exports | Always 0 |
| `loct cycles` | Circular imports | Always 0 |
| `loct health --json` | Quick summary | Always 0 |

## Artifacts Generated

After `loct auto`, you get:

```
<cache-dir>/
├── snapshot.json    # Full dependency graph (jq-queryable)
├── findings.json    # All issues (dead, cycles, twins...)
├── agent.json       # AI-optimized bundle with health_score
└── manifest.json    # Index for tooling
```

## Health Score

The `agent.json` contains a `health_score` (0-100):

```bash
# Extract with jq
HEALTH=$(loct agent | jq -r '.summary.health_score')
```

### Score Interpretation

| Score | Status | Meaning |
|-------|--------|---------|
| 80-100 | 🟢 | Excellent structural health |
| 50-79 | 🟡 | Some issues, review recommended |
| 0-49 | 🔴 | Critical issues, fix before merge |

## SARIF Integration

Generate SARIF for GitHub Code Scanning:

```yaml
- run: loct lint --sarif > loctree.sarif
- uses: github/codeql-action/upload-sarif@v3
  with:
    sarif_file: loctree.sarif
    category: loctree
```

## Environment Variables

| Variable | Default | Description |
|----------|---------|-------------|
| `HEALTH_THRESHOLD` | 50 | Minimum health score to pass |
| `FAIL_ON_FINDINGS` | false | Exit 1 if any findings |
| `MAX_DEAD_EXPORTS` | ∞ | Max dead exports allowed |
| `MAX_CYCLES` | 0 | Max circular imports allowed |

---

𝚅𝚒𝚋𝚎𝚌𝚛𝚊𝚏𝚝𝚎𝚍. with AI Agents by Vetcoders (c)2024-2026 LibraxisAI
