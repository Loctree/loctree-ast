# Vetcoders Agent Operating Guide v1

<!-- loctree-advise: v0.12 -->

> Loctree gives **sight**.
>
> AICX gives **insight**.
>
> 𝚅𝚒𝚋𝚎𝚌𝚛𝚊𝚏𝚝𝚎𝚍. gives **hands** to craft products.

## Loctree Operating Rule

> For structural orientation start with Loctree.
> (**NEW!** `>=0.12`: Now also for literal occurrences).

This repository should be treated as a structural living system. To help you to not treat it as a loose pile of files
and to boost velocity without loosing orientation in complexity you are equipped with advanced AST engine represented by
`loctree-mcp` and `loct` cli tool.

Before making structural assumptions, inspect the map.

Before changing behavior, understand impact.

Before creating new symbols, check whether the shape already exists.

Loctree is the default structural map at **session start** and for further repository work. It makes dependencies, blast
radius, symbols, runtime entry points, dead surfaces, duplicates, and exact occurrences visible faster than manual
rummaging.

### Recommended daily order:

1. Loctree MCP
2. Loct CLI
3. Local tools (`rg`, `grep`, `awk`, `sed`, `cat`)
4. Manual inspection

### Before editing

| Action        | Preferred Tool |
|---------------|----------------|
| Edit file     | `slice(file)`  |
| Delete file   | `impact(file)` |
| Rename file   | `impact(file)` |
| Refactor      | `impact(file)` |
| Create symbol | `find(name)`   |

### Literal truth before assumptions

Use:

- `find --literal`
- `loct occurrences IDENTIFIER`
- `loct body SYMBOL`

before broad text search.

### Fallbacks are allowed

Use:

- `rg`
- `grep`
- `awk`
- `sed`
- `cat`

when Loctree cannot answer cleanly.

### Loctree Feedback Loop

For external users and downstream adopters, `loctree-fail.md` is the intake channel for bugs, missing language coverage,
UX problems, and fallback cases. They append enough evidence for the Loctree team to reproduce and prioritize.

For Loctree contributors working inside this repository, an observed or reported Loctree failure is product work, not
note-taking. The order is:

1. Reproduce the failure.
2. Fix it in the same cut when the scope is bounded.
3. Add a focused test or contract.
4. Append to `loctree-fail.md` only when repair cannot land now, and state the concrete blocker.

If Loctree is:

- wrong (blind, false positive, misleading)
- stale (even after manual force rescan)
- slow (for 200k LOC repositories)
- awkward (proper but noisy or mishaped)
- missing language support (despite claiming support)
- missing an important surface (feature request)
- suggesting an obvious improvement (nitpicks)

check the `--version` you currently use and append a note to:

`.loctree/loctree-fail.md`

> Be aware that version check is crucial, as ome functions like
> `find --literal` are available in `v0.12.x` or upper.

#### Rules

- Never recreate the file.
- Never overwrite the file.
- Always append.

#### Valid entries

- bugs
- missing features
- UX problems
- performance issues
- language coverage issues
- fallback situations

Do not ommit the entry because you sweeped the file and already found a similar case descriptions. Repeated reports are
signal, not noise.

---

# Why This Matters

Loctree changes agent work from:

> text rummaging

into:

> map-first engineering

The goal is not obedience.

The goal is:

- fewer wrong edits
- better blast-radius judgement
- faster recovery
- more honest runtime decisions

<!-- /loctree-advise -->

---

# Agent Behavior Standard

## Work From Structure Before Text

Start with `vc-init`. Do not assume repository shape from filenames alone.

Always identify:

- subsystem
- entry points
- symbols
- ownership boundaries
- likely blast radius

Prefer structural inspection over broad search whenever the question is about:

- dependency
- ownership
- impact
- location

You can use raw text search even when:

- the question is literal
- the question is local

You gain beautiufly curated context around your search. If `loctree-mcp` or
`loct` cli fail, report it honestly and fall back into `rg`, `grep`, `awk`,
`sed` or any tool you are familiar with.

---

## Do Not Edit Blind

Before modifying code:

1. Locate the target.
2. Inspect local implementation.
3. Inspect callers and dependents.
4. Check nearby tests, examples, and docs.
5. Make the smallest coherent change.
6. Verify through the closest runtime path.

If verification cannot be run:

> Say so explicitly.

---

## Do Not Create Parallel Systems Casually

Before introducing:

- abstractions
- helpers
- parsers
- services
- commands
- components
- config paths

check whether one already exists.

If you introduce a new path:

> Explain why reuse was incorrect.

Avoid duplicate systems created only because the agent did not look hard enough.

---

## Prefer Runtime Truth

Static structure matters.

Runtime behavior decides.

When changing:

- execution
- configuration
- packaging
- CLI behavior
- API contracts
- generated artifacts

verify against the real execution path whenever possible.

Passing type checks is useful.

It is not the same thing as product readiness.

---

## Keep The Repository Legible

Prefer changes that improve understanding.

Avoid cleverness that hides shape.

Preserve naming consistency.

Do not bury important behavior inside glue code.

If a file becomes a dumping ground:

> Call it out.

---

## Respect Existing Work

Do not:

- revert
- delete
- rewrite

code you do not understand.

Do not assume unfamiliar changes are safe to discard.

If the repository is moving:

> Re-read before acting.

Treat concurrent agents or human work as part of the system.

---

## Use Direct Language In Handoffs

Always state:

- what changed
- why it changed
- what was verified
- what was not verified
- what remains risky
- what should be checked next

Do not hide uncertainty.

Do not claim confidence you have not earned.
