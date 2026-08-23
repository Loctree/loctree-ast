# AI Hooks Runtime

This directory is the source of truth for the Loctree-first and AICX
continuity hooks installed by `scripts/install-ai-hooks.sh`.

The policy is deliberately asymmetric:

- Loctree is the first-choice structural instrument.
- Raw `grep` and `rg` are legitimate local lenses, not forbidden tools.
- A deliberate fallback must be visible as `command grep ...` or
  `command rg ...` when it starts a repository-mapping command.
- Pipe filters such as `git status | grep ...` are never policed.
- A fallback that exposes a Loctree miss should be appended to
  `~/.vibecrafted/loctree/loctree-fail.md`.

This is supervision for expert operators, not a security boundary.

## The supervising-surgeon contract

An experimental hospital does not ask a surgeon to use a worse instrument out
of ritual obedience. It does require the surgeon to begin with the instrument
that gives the best sight, make an exceptional choice consciously, and report
the shape that the experimental instrument could not yet handle.

In shell terms:

```bash
# First choice: structural truth with curated context
loct find --literal 'SessionModel'
loct occurrences SessionModel
loct body SessionModel
loct slice src/session.rs

# Exceptional local lens after the map, or when Loctree cannot answer cleanly
command rg -n 'exact literal' src/session.rs
command grep -n 'exact literal' src/session.rs

# A downstream filter is ordinary shell composition and remains untouched
git status --short | grep '^ M'
```

The guard fails open on malformed payloads or uncertain repository scope. Its
job is to form a better reflex and collect product feedback, not to hold an
agent's hands behind its back.

## Packages

### Loctree-first

Claude receives:

- `loctree-first-guard.py` on `PreToolUse:Bash`;
- `loct-grep-augment.sh` on `PostToolUse:Grep`;
- `loct-smart-suggest.sh` as the suggestion payload.

Codex receives the versioned `loctree-first@loctree-local` plugin from:

```text
ai-hooks/codex/loctree-marketplace/
```

The marketplace lives inside this repository, so moving the workspace does not
leave `~/.codex/config.toml` pointing at a dead legacy checkout.

Gemini receives the proven Loctree augmentation payload. A blocking lifecycle
guard is not installed there until its hook semantics are verified.

### AICX compact recall

Claude receives the protected runtime pair:

- `PreCompact` -> `aicx-precompact.sh`;
- `PostCompact` -> `aicx-postcompact.sh`;
- `aicx-recall-selftest.sh` as the verification gate.

Codex receives `aicx-compact-recall@personal` from:

```text
ai-hooks/codex/aicx-marketplace/
```

Its lifecycle is intentionally different:

- `PreCompact` extracts the exact Codex transcript file;
- `SessionStart` matched only on `compact` injects bounded recall;
- literal `PostCompact` is forbidden because its stdout is not the supported
  model-context path in Codex.

The installer runs the packaged `doctor.sh` and refuses to call the Codex AICX
installation healthy when source, installed plugin, registry, extraction, or
recall fails.

AICX continuity is not installed for Gemini yet. The installer says so loudly
instead of manufacturing a hook whose output path has not been proven.

## Legacy Memex hard cut

The following payloads are removed from this package and cleaned from hook
registrations owned by the old installer:

```text
memex-context.sh
memex-startup.sh
memory-on-compact.sh
```

`HOOKS=memex` fails with an explicit migration message. It does not silently
mean AICX, because silent compatibility would blur which continuity mechanism
is actually running.

The installer does not remove an independently configured `rust-memex` MCP
server. That is a separate product decision, not hook-package cleanup.

## Installation

Interactive:

```bash
make ai-hooks
```

All proven hooks for one runtime:

```bash
make ai-hooks-claude
make ai-hooks-codex
```

All detected runtimes:

```bash
make ai-hooks-all
```

Direct package selection:

```bash
CLI=claude HOOKS=loctree ./scripts/install-ai-hooks.sh
CLI=claude HOOKS=aicx ./scripts/install-ai-hooks.sh
CLI=codex HOOKS=all ./scripts/install-ai-hooks.sh
```

`HOOKS` accepts `loctree`, `aicx`, or `all`. Selective installation preserves
the other live package and always removes only the obsolete Memex hook files
and registrations.

Running agents must be restarted or resumed after installation. Hook registries
are not assumed to hot-reload.

## Verification

Fast isolated contract suite:

```bash
python3 -m unittest -v tests.test_ai_hooks
```

The suite proves:

- ordinary first-choice repo grep is paused;
- explicit `command grep` and `command rg` pass;
- pipe grep passes;
- a clean-HOME install preserves unrelated hooks;
- selective package installation preserves the other package;
- legacy Memex payload and registrations disappear;
- Codex installation uses versioned local marketplaces.

The AICX plugin carries its own fixtures and lifecycle tests. For the live
operator runtime, the final gate is:

```bash
~/plugins/aicx-compact-recall/scripts/doctor.sh
```

Do not bypass that doctor for a real installation. The
`AI_HOOKS_SKIP_DOCTOR=1` switch exists only for isolated installer tests whose
temporary HOME has no real session corpus or Codex registry.

## Runtime feedback

Append Loctree misses and awkward fallback cases to:

```text
~/.vibecrafted/loctree/loctree-fail.md
```

Crash and code-signing truth belongs in the evidence cone too. On macOS, inspect
`~/Library/Logs/DiagnosticReports/*.ips` before reducing a process death to a
generic startup failure. A sysdump is runtime testimony: exception class,
termination namespace, faulting thread, binary image, and timestamp are stronger
than folklore.

𝚅𝚒𝚋𝚎𝚌𝚛𝚊𝚏𝚝𝚎𝚍. with AI Agents by vetcoders.
