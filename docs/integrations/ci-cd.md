# CI/CD Integration

Integrate loctree into your continuous integration pipeline to catch dead code, circular imports, and duplicates before they reach production.

## GitHub Actions

### Basic Workflow

```yaml
# .github/workflows/loctree.yml
name: Loctree Analysis

on:
  pull_request:
  push:
    branches: [main, develop]

jobs:
  analyze:
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v4
      - uses: actions/setup-node@v4
        with:
          node-version: 20

      - name: Install loctree
        run: npm install -g @loctree/loctree

      - name: Run analysis
        run: loct findings > findings.json

      - name: Check for issues
        run: |
          DEAD=$(jq '.dead_exports | length' findings.json)
          CYCLES=$(jq '.cycles | length' findings.json)
          if [ "$DEAD" -gt 0 ] || [ "$CYCLES" -gt 0 ]; then
            echo "Found $DEAD dead exports and $CYCLES cycles"
            exit 1
          fi
```

### With Caching

```yaml
jobs:
  analyze:
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v4

      - name: Cache npm
        uses: actions/cache@v3
        with:
          path: ~/.npm
          key: ${{ runner.os }}-npm-loctree

      - name: Install loctree
        run: |
          if ! command -v loct &> /dev/null; then
            npm install -g @loctree/loctree
          fi

      - name: Analyze
        run: loct --fail-stale
```

### PR Comment with Report

```yaml
      - name: Generate report
        run: loct findings --summary > report.json

      - name: Comment on PR
        if: github.event_name == 'pull_request'
        uses: actions/github-script@v6
        with:
          script: |
            const fs = require('fs');
            const report = fs.readFileSync('report.json', 'utf8');
            github.rest.issues.createComment({
              issue_number: context.issue.number,
              owner: context.repo.owner,
              repo: context.repo.repo,
              body: '## Loctree Analysis\n```\n' + report + '\n```'
            });
```

## Pre-commit Hook

### Using pre-commit framework

```yaml
# .pre-commit-config.yaml
repos:
  - repo: local
    hooks:
      - id: loctree-dead
        name: Check dead exports
        entry: loct dead --fail
        language: system
        pass_filenames: false

      - id: loctree-cycles
        name: Check circular imports
        entry: loct cycles --fail
        language: system
        pass_filenames: false
```

### Manual git hook

```bash
#!/bin/sh
# .git/hooks/pre-push

echo "Running loctree analysis..."
loct --quiet

DEAD=$(loct dead --json | jq '. | length')
if [ "$DEAD" -gt 0 ]; then
  echo "ERROR: $DEAD dead exports found"
  echo "Run 'loct dead' for details"
  exit 1
fi

echo "Loctree: OK"
```

## CLI Flags for CI

| Command / Flag | Description | Use Case |
|------|-------------|----------|
| `--json` | JSON output | Parsing in scripts |
| `--quiet` | No progress output | Clean logs |
| `--fail-stale` | Fail if snapshot outdated | Enforce fresh analysis |
| `--fresh` | Force full rescan | Ensure accuracy |
| `findings --summary` | Health score only | Quick status |

## Exit Codes

| Code | Meaning |
|------|---------|
| 0 | Success, no issues |
| 1 | Issues found (dead, cycles) |
| 2 | Configuration error |
| 3 | Snapshot missing/stale |

## Badge

Add to your README:

```markdown
[![loctree](https://img.shields.io/badge/analyzed_with-loctree-a8a8a8)](https://crates.io/crates/loctree)
```

Result: [![loctree](https://img.shields.io/badge/analyzed_with-loctree-a8a8a8)](https://crates.io/crates/loctree)

## GitLab CI

```yaml
# .gitlab-ci.yml
loctree:
  stage: test
  image: node:20
  script:
    - npm install -g @loctree/loctree
    - loct --fail-stale
  cache:
    paths:
      - ~/.npm
```

## CircleCI

```yaml
# .circleci/config.yml
version: 2.1
jobs:
  loctree:
    docker:
      - image: cimg/node:20.0
    steps:
      - checkout
      - run: npm install -g @loctree/loctree
      - run: loct --json > results.json
      - store_artifacts:
          path: results.json
```

---

*𝚅𝚒𝚋𝚎𝚌𝚛𝚊𝚏𝚝𝚎𝚍. with AI Agents by Vetcoders (c)2024-2026 LibraxisAI*
