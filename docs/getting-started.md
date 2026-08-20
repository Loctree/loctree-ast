# Getting Started with loctree

5-minute quickstart to analyzing your codebase with loctree.

## Installation

Pick the path that fits your environment. Full options live in [installation.md](./installation.md).

```bash
# Signed bundle (loct + loctree + loctree-mcp + loctree-lsp + aicx + aicx-mcp)
curl -fsSL https://loct.io/install.sh | bash

# Or via npm (runtime package with sibling MCP/LSP binaries)
npm install -g @loctree/loctree

# Or via Homebrew (CLI only)
brew install loctree/cli/loct
```

Verify the install:

```bash
loct --version
loctree --version
```

The npm runtime wrapper exposes both `loctree` and the short `loct` alias, and its platform package embeds the sibling
`loctree-mcp` / `loctree-lsp` binaries.

> **Bundle note (0.13.0):** full signed bundles ship six binaries — `loct`, `loctree`, `loctree-mcp`, `loctree-lsp`,
> `aicx`, `aicx-mcp`. The `x86_64-unknown-linux-musl-core` bundle is Loctree-only plus optional AICX runtime metadata.

> Rust users can install the published core crate with `cargo install --locked loctree` or depend on it with
> `cargo add loctree@0.13.0`; contributor source builds are covered in [dev/01_installation.md](./dev/01_installation.md).

## First Scan

Run loctree in any project directory:

```bash
cd your-project
loct
```

```
Analyzing: your-project
Stack detected: TypeScript (tsconfig.json)
Scanning 247 files...
━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━ 100% 247/247 files

Artifacts written to the user cache dir
  snapshot.json   - Full dependency graph (127 KB)
  findings.json   - Issues detected (dead code, cycles, etc.)
  agent.json      - AI-optimized context bundle
  manifest.json   - Index for tooling

Health score: 82/100
  ✓ No circular imports
  ⚠ 3 unused exports (--confidence high)
  ⚠ 1 dead parrot (0 imports)
```

## Key Artifacts

After scanning, loctree creates these artifacts in the user cache dir by default (override with `LOCT_CACHE_DIR`):

### snapshot.json

Complete import/export graph. Query it with jq-style syntax:

```bash
loct '.metadata'                    # Project info
loct '.files | length'              # Count files
loct '.edges[] | select(.from | contains("api"))'  # Filter edges
```

### findings.json

All detected issues:

```bash
loct findings | jq '.dead_exports[] | select(.confidence == "high")'
```

### agent.json

AI-optimized bundle with health score and quick wins:

```bash
loct agent | jq '.summary'
```

### manifest.json

Index for IDE integrations and tooling:

```bash
loct '.metadata'
```

## Essential Commands

### Scan and analyze

```bash
loct                    # Full scan with auto-detection
loct --fresh            # Force rescan (ignore cache)
loct --watch            # Continuous scan on file changes
```

### Get context for a file

```bash
loct slice src/components/ChatPanel.tsx
```

Output shows 3 layers:

- **Core**: The file itself
- **Deps**: Direct and transitive dependencies
- **Consumers**: Files that import this file (use `--consumers`)

```
Slice for: src/components/ChatPanel.tsx

Core (1 files, 180 LOC):
  src/components/ChatPanel.tsx (180 LOC, tsx)

Deps (4 files, 320 LOC):
  [d1] src/hooks/useChat.ts (90 LOC)
    [d2] src/contexts/ChatContext.tsx (150 LOC)
    [d2] src/utils/api.ts (80 LOC)

Consumers (3 files, 240 LOC):
  src/App.tsx (120 LOC)
  src/routes/chat.tsx (80 LOC)
  src/layouts/MainLayout.tsx (40 LOC)

Total: 8 files, 740 LOC
```

Add `--json` for AI consumption:

```bash
loct slice src/main.rs --consumers --json | claude
```

### Search for symbols

```bash
loct find useAuth                # Find symbol definitions/usage
loct find ChatPanel              # Find similar components
loct f useAuth                   # Short alias
```

### Find dead exports

```bash
loct dead                        # All unused exports
loct dead --confidence high      # High confidence only
```

Detects:

- Unused exports across all languages
- Re-export chains (barrel files)
- Registry patterns (WeakMap/WeakSet)
- Python `__all__` declarations

### Detect circular imports

```bash
loct cycles
```

```
Circular import detected:
  src/components/UserProfile.tsx
  → src/hooks/useUser.ts
  → src/contexts/UserContext.tsx
  → src/components/UserProfile.tsx

Cycle length: 3 files
Impact: 12 files in component
```

### Quick health check

```bash
loct health
```

```
Health Score: 82/100

Issues:
  ✓ Circular imports: 0
  ⚠ Dead exports: 3 (high confidence)
  ⚠ Dead parrots: 1 (0 imports)
  ⚠ Twins: 0

Recommendations:
  1. Review unused export in src/utils/helpers.ts:45
  2. Check dead parrot: calculateDistance (src/geo/distance.ts)
```

### Full codebase audit

```bash
loct audit
```

Runs comprehensive checks:

- Circular imports
- Dead exports
- Twins (duplicate exports)
- Zombie code (orphan files + shadows)
- Functional crowds (clustering)

## Query Mode (jq-style)

Query snapshot data directly:

```bash
# Extract metadata
loct '.metadata'

# Count files
loct '.files | length'

# Find all dead exports
loct '.dead_parrots[]' --artifact findings

# Find cycles
loct '.cycles[]' --artifact findings

# Filter by path
loct '.files[] | select(.path | contains("src/api"))'
```

Options:

- `-r, --raw` - Raw output (no JSON quotes)
- `-c, --compact` - One line per result
- `-e, --exit-status` - Exit 1 if result is false/null

## Next Steps

### IDE Integration

`loctree-lsp` is a Language Server Protocol server. It runs over stdio — every LSP-capable editor can use it.

```bash
npm install -g @loctree/loctree-lsp   # if not installed yet
loctree-lsp --version                  # smoke test
```

See [ide/](ide/) for editor-specific setup guides:

- [VS Code](ide/vscode.md)
- [Neovim](ide/neovim.md)
- [LSP protocol details](ide/lsp-protocol.md)

### MCP Server

`loctree-mcp` is a Model Context Protocol server for AI agents (Claude Code, Codex, Cursor, etc.).

```bash
npm install -g @loctree/loctree-mcp   # if not installed yet
loctree-mcp --version                  # smoke test
```

Wire it into your MCP-capable client:

```json
{
  "mcpServers": {
    "loctree": {
      "command": "loctree-mcp",
      "args": []
    }
  }
}
```

See [integrations/mcp-server.md](integrations/mcp-server.md) for the full tool surface.

### CI Integration

Add loctree to your CI pipeline:

```bash
# GitHub Actions example
loct lint --fail --sarif > loctree.sarif
loct health --json | jq '.summary.health_score'
```

Fail on issues:

- `--fail` - Exit non-zero if findings detected
- `--sarif` - SARIF 2.1.0 output for GitHub/GitLab

### Tauri Projects

For Tauri applications, loctree provides specialized commands:

```bash
loct commands              # Show FE↔BE handler bridges
loct trace <handler>       # Trace handler end-to-end
loct events                # Event flow analysis
loct coverage --handlers   # Handler test coverage
```

## Getting Help

```bash
loct --help              # Main help
loct --help-full         # All 28 commands
loct <command> --help    # Per-command help
loct --help-legacy       # Legacy flag migration
```

## Common Workflows

### Before creating a new component

```bash
loct find ChatSurface
# Found: ChatPanel (distance: 2), ChatWindow (distance: 3)
# → Consider reusing ChatPanel instead
```

### Before refactoring

```bash
loct impact src/utils/api.ts
# Shows all files that depend on api.ts
```

### Continuous development

```bash
loct --watch
# Auto-rescans on file changes
# Press Ctrl+C to stop
```

### AI-assisted development

```bash
# Get AI bundle with health + quick wins
loct --for-ai > context.json

# Get context for specific file
loct slice src/main.rs --json | your-ai-tool
```

---

**𝚅𝚒𝚋𝚎𝚌𝚛𝚊𝚏𝚝𝚎𝚍. with AI Agents by Vetcoders (c)2024-2026 LibraxisAI**
