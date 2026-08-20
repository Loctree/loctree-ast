# AICX Compact Recall

Protected global Codex continuity hooks.

The runtime pair is intentionally asymmetric:

- `PreCompact` extracts the exact Codex transcript before compaction.
- `SessionStart` with matcher `compact` injects the recall digest after
  compaction. Codex ignores plain stdout from `PostCompact`, so using the
  literal `PostCompact` event would silently lose model-visible recall.

The Codex-specific scripts live inside this versioned plugin. They were
recovered from the operator-owned implementations in `~/.claude/hooks/`, then
adapted with `AICX_HOOK_AGENT=codex`. Codex extraction uses the hook-provided
`transcript_path`, avoiding the slow global session scan.

The current parser contract is direct-file only:

```bash
${AICX_BIN:-aicx} extract codex --file "$transcript_path" --conversation -o "$atomic_temp"
```

`AICX_BIN` may point at an uninstalled AICX build for fixture and pre-install
validation. A missing transcript never falls back to a corpus or session scan.

### Freshness seal (append-only Codex rollouts)

`PreCompact` writes `$extract.freshness.json` with the sealed `raw_bytes` of the
rollout at extract time. `SessionStart(compact)` requires that sidecar.

Codex then **appends** a multi-MB `compacted` event (plus `world_state` /
`context_compacted`) to the same rollout **before** recall runs. Exact
`raw_bytes` equality therefore fails on every real compact and forces the loud
`POSTCOMPACT RECALL DEGRADED` path even when the extract is correct.

Recall accepts **prefix growth** (`live >= sealed`), rejects shrink/replace and
path mismatch, and **consumes** the seal after a successful digest so a later
compact without a new `PreCompact` cannot re-inject stale memory.

## Activation boundary

Codex loads plugin hooks when a process starts. An already-running Codex
process is not evidence of the newly installed generation and must never be
declared healthy from a fresh app-server registry alone. After reinstalling or
upgrading this plugin, restart Codex, resume the session in a fresh process, or
start a new session before expecting `PreCompact` and `SessionStart(compact)`
to use the new payload.

## Protection contract

Do not remove or disable this plugin during global runtime cleanup without the
operator's explicit authorization in that turn. The same rule is recorded in
`~/.codex/AGENTS.md`.

Verify after every hook, plugin, marketplace, or global config change:

```bash
AICX_BIN=/path/to/aicx ~/plugins/aicx-compact-recall/scripts/doctor.sh
```

Recovery from an intact personal marketplace source:

```bash
codex plugin add aicx-compact-recall@personal
```

Then review and trust the exact hooks using `/hooks` or verify their persisted
hashes through `hooks/list`.
