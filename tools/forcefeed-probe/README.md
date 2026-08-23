# forcefeed-probe — E1-01a mechanical delivery proof

Captures what the *real* runner actually injects into the agent before first action.

## Usage
```bash
tools/forcefeed-probe/run.sh --runner grok --repo . --out /tmp/ff-grok.json
tools/forcefeed-probe/run.sh --runner claude --repo . --out /tmp/ff-claude.json
```

Builds full atlas (incl. explicit 03-intent-map + intent layer) to RAW then feeds to fake-agent for mechanical "received" capture. Avoids wrapper truncation races.

## Runners
- `claude`: emulates Claude Code SessionStart via the real `loctree-plugin/hooks/loct-context-card.sh` (or falls back to full atlas cards). This is the hook that actually feeds `loct context` as additionalContext.
- `grok` / terminal (vibecrafted): uses `loct context` + the `VIBECRAFTED_PROMPT_PATH` (the full operator brief the worker received) + atlas cards.
- `codex`, `junie`: documented unavailable for native binary/hook tap on this host; run.sh returns non-zero so verifier skips.

## Output JSON (exactly what verifier asserts)
- `captured`
- `coverage.missing_fact_ids == []`
- `order.structure_before_task`
- `truncation_detected == false`
- payload metrics (bytes/tokens)

## Parser
Reuses `tools/atlas_factset_check.py` (L1-01). Fact ids = atlas cards + structural signals (core-map, structural-map, ... hubs, aicx-memory, ...).

## Gates
- shellcheck on *.sh
- zero edits to prod code paths
- synchronous foreground only

## Living Tree
Re-read touched files. Commit packs of 5-6. Title includes `[<agent>/vc-workflow] ...`

## Recovery
If hook changes, probe follows the hook source (read, never rewrite the prod hook).
