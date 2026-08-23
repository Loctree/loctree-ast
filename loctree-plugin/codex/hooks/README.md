# Codex Hooks

Updated: 2026-06-06

Codex now supports native hook wiring through `~/.codex/hooks.json` when
`features.hooks = true` is enabled in `~/.codex/config.toml`.

The Loctree Codex hook surface is intentionally smaller than the Claude Code
auto-augmentation surface. It provides informative `PostToolUse` hints instead
of running heavy augmentation on every search.

## Native PostToolUse Hint

Install through the top-level hook installer:

```bash
make ai-hooks CLI=codex HOOKS=loctree
```

This copies:

```text
ai-hooks/loct-smart-suggest.sh -> ~/.codex/hooks/loct-smart-suggest.sh
ai-hooks/loct-mcp-transport-hint.py -> ~/.codex/hooks/loct-mcp-transport-hint.py
```

and registers:

```json
{
  "hooks": {
    "PostToolUse": [
      {
        "matcher": "*",
        "hooks": [
          {
            "type": "command",
            "command": "LOCT_SMART_SUGGEST_OUTPUT=json bash \"$HOME/.codex/hooks/loct-smart-suggest.sh\"",
            "timeout": 3
          }
        ]
      },
      {
        "matcher": "*",
        "hooks": [
          {
            "type": "command",
            "command": "python3 \"$HOME/.codex/hooks/loct-mcp-transport-hint.py\"",
            "timeout": 3
          }
        ]
      }
    ]
  }
}
```

## What It Hints

MCP transport recovery:

```text
MCP transport likely went stale/down. Refresh tool discovery with tool_search for the affected MCP, then rerun your call. Fallback to CLI <command> if still broken.
```

Raw text-search fallback:

```text
I see you used `rg <options> <args>`. Loctree literal is now a quiet indexed lookup with curated context; use raw text search for raw filesystem truth. Try `loct find <pattern> --literal --group-by-file --count-only` next time.
```

File-list fallback:

```text
I see you used `find <args>`. Loctree tree can answer indexed file-list discovery without rescanning noise; use shell find for raw filesystem truth. Try `loct tree --files --match <regex>` next time.
```

## Behavior

- Non-blocking: hooks never deny or mutate tool calls.
- Quiet by default: unrelated tool calls emit no hint or return `{"continue": true}`.
- Codex-native: no prelude shim is required for current Codex hook runtimes.
- Honest scope: Loctree hints are for indexed/context lookup, not a promise of
  byte-for-byte equivalence with raw `rg` or filesystem `find`.
- CLI fallback friendly: if MCP transport remains broken after tool discovery
  refresh, the agent should use the equivalent `loct` command.

## Troubleshooting

```bash
python3 -m json.tool ~/.codex/hooks.json
ls -la ~/.codex/hooks/loct-smart-suggest.sh ~/.codex/hooks/loct-mcp-transport-hint.py
grep -n "hooks = true" ~/.codex/config.toml
```

Restart Codex after changing hook configuration.
