# Vista: Why Loctree Exists

> Vista was the repo where grep stopped being enough.

This route used to show a point-in-time Tauri contract transcript.
That transcript aged badly, because live products move and exact handler counts rot.
What did not age is the reason Loctree exists at all.

Vista was a living desktop product with Rust, React, TypeScript, Tauri handlers,
events, dead code, half-finished features, and AI agents editing in parallel.
The team did not need another pretty report.
The team needed runtime truth before cutting anything important.

## The actual problem

At small scale, people reach for:

- grep
- editor search
- flat dead-code reports
- intuition

That breaks once the repo is alive enough.

In Vista we kept hitting the same failure modes:

1. A symbol looked used because text search found a name collision.
2. A handler looked dead but was actually reachable through a runtime path.
3. A file looked safe to edit until a hidden consumer exploded elsewhere.
4. A report looked green while the shipped bundle told a different story.
5. AI agents could write code fast, but they could not see blast radius by default.

The cost was not theoretical elegance.
The cost was real hesitation, false deletions, noisy reviews, and wasted refactor loops.

## What Vista forced us to build

Vista pushed Loctree from an internal helper into a product with a sharp shape.

### 1. Slices before edits

Agents and humans both needed one honest unit of context:

- the file
- what it depends on
- what depends on it

That became the default mental model behind `loct slice`.

### 2. Blast radius before deletion

We needed to answer one brutal question quickly:

> What breaks if this file disappears?

That became `loct impact`.

### 3. Runtime contracts, not just imports

Tauri command names, event flows, and frontend/backend seams mattered more than
pure import graphs.

That pushed Loctree toward:

- `loct commands`
- `loct trace`
- `loct events`
- `loct pipelines`

### 4. Real dead code triage

Not every “unused” symbol deserves the same action.
Vista forced a split between:

- safe removals
- dynamic/runtime surfaces
- internal plumbing
- work in progress
- false positives worth suppressing

That pressure is what made `doctor`, suppressions, and findings categorization necessary.

### 5. Bundle truth

It was not enough to know what existed in source.
We also needed to know what actually survived shipping.

That pressure eventually produced the dist surface and tree-shaking verification path.

## What stayed true

The original numbers from Vista are less important than the lessons:

- agents need prepared structural context, not raw file floods
- humans need blast radius before destructive changes
- dead code needs categories, not one scary bucket
- runtime surfaces matter as much as static imports
- installability and report surfaces matter if the tool is meant to ship

That is still the product shape today.

## The shortest honest workflow

If you want the distilled Vista lesson, it is this:

```bash
loct
loct --for-ai
loct slice <file> --consumers --json
loct impact <file>
loct findings --summary
```

Scan once.
Give the agent a real map.
Check blast radius before edits or deletions.
Look at findings when you need machine truth.

## Why the route still exists

We kept this route because Vista is the origin story.
But it should not pretend to be a live operational transcript anymore.

Loctree was forged there because a living repo demanded:

- maps
- report artifacts
- runtime truth
- safer AI-assisted editing

That is the part worth preserving.

## Where to go next

- Want the product surface: `/agents-os`
- Want the commands: `/docs`
- Want the current artifact-first machine truth: `loct findings`
- Want the origin story: you are already here
