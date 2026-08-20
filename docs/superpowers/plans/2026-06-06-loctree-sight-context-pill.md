# Loctree Sight Context Pill — Implementation Plan (VS Code)

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Replace the VS Code plugin's tree-based, mode-first Context Panel with an ambient, webview **Context Pill** for the active file, whose primary action copies an agent-ready context pack — with a secondary scope switcher and zero dead-ends.

**Architecture:** A hardened **webview** renders one typed `ContextPillViewModel` assembled by an extension-side adapter from existing `loctree/*` LSP responses (`contextPack`, `body`, `impact`, `health`). The pill is ambient (follows the active editor) and re-scopes via an `Inspect symbol or file…` switcher across `file:` / `symbol:` / `literal:` / `match-set:` scopes. Expert modes stay in the Command Palette. No LSP protocol change except an optional `body` ranking fallback.

**Tech Stack:** TypeScript, VS Code Extension API (WebviewView), esbuild bundle, node:test (compiled via tsconfig.test.json), existing `LoctreeGateway` (src/gateway.ts).

**Spec:** `docs/design/2026-06-06-loctree-sight-context-pill.md` (read it first). **Out of scope (separate plans):** JetBrains mirror (gated on Kotlin indexing — see spec dependency note) and Neovim.

---

## File Structure

- Create `editors/vscode/src/contextPill/scope.ts` — Scope taxonomy + input routing (pure, no VS Code deps).
- Create `editors/vscode/src/contextPill/viewModel.ts` — `ContextPillViewModel` type + `assembleViewModel(gateway, scope)` adapter (the only place raw LSP responses are stitched).
- Create `editors/vscode/src/contextPill/agentContext.ts` — `buildAgentContextMarkdown(vm, task)` + `inferTask(...)` (pure).
- Create `editors/vscode/src/contextPill/panel.ts` — `ContextPillViewProvider` (WebviewViewProvider): lifecycle, CSP/nonce, message allowlist, ambient binding, calls adapter, posts view model.
- Create `editors/vscode/media/contextPill.js` — webview client: render view model, section layout, CTA states, scope-switcher input, message channel. No remote assets.
- Create `editors/vscode/media/contextPill.css` — pill styling (colors/tags/decision band/preview), no emoji.
- Modify `editors/vscode/src/gateway.ts` — add `bodyRanked()` (file as ranking hint, never empty if a body exists) and a `literalScope()` helper for the literal fallback.
- Modify `editors/vscode/src/extension.ts` — register `ContextPillViewProvider`; wire `onDidChangeActiveTextEditor` → ambient rescope; register `loctree.copyAgentContext`; stop registering the old tree context panel as the primary view.
- Modify `editors/vscode/package.json` — `contributes.views`: replace the tree `loctree.context` with a `webviewView` of the same id; add command `loctree.copyAgentContext` ("Loctree: Skopiuj kontekst dla agenta"); keep Findings view; expert mode commands stay.
- Test: `editors/vscode/test/contextPillScope.test.ts`, `editors/vscode/test/contextPillViewModel.test.ts`, `editors/vscode/test/agentContext.test.ts`.

Decomposition principle: `scope.ts`, `viewModel.ts`, `agentContext.ts` are **pure and unit-tested without VS Code**; `panel.ts` and `media/*` are the VS Code/webview shell exercised via the live VSIX smoke. Keep each file single-responsibility.

---

## Task 1: Scope taxonomy + routing (pure)

**Files:**
- Create: `editors/vscode/src/contextPill/scope.ts`
- Test: `editors/vscode/test/contextPillScope.test.ts`

- [ ] **Step 1: Write the failing test**

```typescript
// editors/vscode/test/contextPillScope.test.ts
import { test } from 'node:test';
import assert from 'node:assert';
import { parseScopeInput, scopeKey } from '../src/contextPill/scope';

test('bare filename routes to file scope', () => {
  assert.deepStrictEqual(parseScopeInput('dispatch.py'), { kind: 'file', value: 'dispatch.py' });
});

test('path fragment routes to file scope', () => {
  assert.deepStrictEqual(parseScopeInput('editors/vscode/src/gateway.ts'), {
    kind: 'file', value: 'editors/vscode/src/gateway.ts',
  });
});

test('identifier routes to symbol scope', () => {
  assert.deepStrictEqual(parseScopeInput('parse_jsonl_to_session_record'), {
    kind: 'symbol', value: 'parse_jsonl_to_session_record',
  });
});

test('multi-word / spaced query routes to literal scope', () => {
  assert.deepStrictEqual(parseScopeInput('blast radius'), { kind: 'literal', value: 'blast radius' });
});

test('scopeKey is stable and distinct per kind', () => {
  assert.strictEqual(scopeKey({ kind: 'file', value: 'a.ts' }), 'file:a.ts');
  assert.notStrictEqual(
    scopeKey({ kind: 'symbol', value: 'x' }),
    scopeKey({ kind: 'literal', value: 'x' }),
  );
});
```

- [ ] **Step 2: Run test to verify it fails**

Run: `cd editors/vscode && npx tsc -p tsconfig.test.json && node --test out-test/test/contextPillScope.test.js`
Expected: FAIL — `Cannot find module '../src/contextPill/scope'`.

- [ ] **Step 3: Write minimal implementation**

```typescript
// editors/vscode/src/contextPill/scope.ts

/** The pill always has exactly one scope. */
export type Scope =
  | { kind: 'file'; value: string }       // scope=file:<path>
  | { kind: 'symbol'; value: string }     // scope=symbol:<symbol-id>
  | { kind: 'literal'; value: string }    // scope=literal:<query>
  | { kind: 'match-set'; value: string }; // scope=match-set:<query>

export function scopeKey(scope: Scope): string {
  return `${scope.kind}:${scope.value}`;
}

const IDENT_RE = /^[A-Za-z_][A-Za-z0-9_]*$/;
const PATH_RE = /[\\/]|\.[A-Za-z0-9]+$/; // has a separator or a file extension

/**
 * Route raw search input to a scope. This drives the secondary scope switcher;
 * it is NOT a search engine — it decides what the pill should look at next.
 * - looks like a file (has a separator or extension) -> file
 * - a single identifier token -> symbol
 * - anything else (spaces, phrases) -> literal
 * Symbol resolution that turns up zero / many is handled by the caller
 * (ambiguous -> match-set; none -> literal fallback) — see viewModel.
 */
export function parseScopeInput(raw: string): Scope {
  const q = raw.trim();
  if (PATH_RE.test(q) && !q.includes(' ')) {
    return { kind: 'file', value: q };
  }
  if (IDENT_RE.test(q)) {
    return { kind: 'symbol', value: q };
  }
  return { kind: 'literal', value: q };
}
```

- [ ] **Step 4: Run test to verify it passes**

Run: `cd editors/vscode && npx tsc -p tsconfig.test.json && node --test out-test/test/contextPillScope.test.js`
Expected: PASS (5 tests).

- [ ] **Step 5: Commit**

```bash
git add editors/vscode/src/contextPill/scope.ts editors/vscode/test/contextPillScope.test.ts
git commit -m "[claude/interactive] feat(vscode): context pill scope taxonomy + input routing"
# (append the Vibecrafted footer: Authored-By/session_id/date/runtime — see repo commit-msg hook)
```

---

## Task 2: ContextPillViewModel + adapter

**Files:**
- Create: `editors/vscode/src/contextPill/viewModel.ts`
- Test: `editors/vscode/test/contextPillViewModel.test.ts`
- Reference: `editors/vscode/src/gateway.ts` (response interfaces: `ContextPackResponse`, `BodyResponse`, `HealthResponse`, `RiskItem`, `OccurrenceResults`)

The adapter is the **only** place raw LSP responses are stitched. The webview consumes `ContextPillViewModel` exclusively. Inject a minimal gateway-shaped interface so the adapter is unit-testable without VS Code.

- [ ] **Step 1: Write the failing test** (uses a fake gateway returning canned LSP shapes)

```typescript
// editors/vscode/test/contextPillViewModel.test.ts
import { test } from 'node:test';
import assert from 'node:assert';
import { assembleViewModel, type PillGateway } from '../src/contextPill/viewModel';

const fakeGateway: PillGateway = {
  async contextPack() {
    return {
      file: 'tb_core/dispatch.py',
      summary: 'Detects a session file agent and routes to the adapter.',
      exports: [{ name: 'parse_jsonl_to_session_record', kind: 'fn' }, { name: 'detect_agent', kind: 'fn' }],
      dependencies: ['parse_claude_jsonl', 'parse_codex_jsonl', 'parse_gemini_cli_jsonl'],
    } as any;
  },
  async impact() {
    return { direct: ['cli.py', 'batch.py', 'main.py'], transitive_count: 7 } as any;
  },
  async health() {
    return { score: 92, status: 'green', dead: 0, cycles: 0, twins: 0, hotspots: 1 } as any;
  },
  async bodyRanked() {
    return { found: true, file: 'tb_core/dispatch.py', preview: 'def parse_jsonl_to_session_record(path):\n    ...', truncated: true } as any;
  },
};

test('assembleViewModel maps a file scope to a complete view model', async () => {
  const vm = await assembleViewModel(fakeGateway, { kind: 'file', value: 'tb_core/dispatch.py' });
  assert.strictEqual(vm.scope.kind, 'file');
  assert.strictEqual(vm.file, 'tb_core/dispatch.py');
  assert.strictEqual(vm.health.score, 92);
  assert.strictEqual(vm.blastRadius.count, 7);
  assert.deepStrictEqual(vm.blastRadius.direct.slice(0, 3), ['cli.py', 'batch.py', 'main.py']);
  assert.strictEqual(vm.exports.length, 2);
  assert.strictEqual(vm.deps.length, 3);
  assert.ok(vm.bodyPreview && vm.bodyPreview.truncated === true);
  assert.strictEqual(vm.findings.hotspots, 1);
  assert.strictEqual(vm.agentPackStatus, 'ready');
});

test('a missing section degrades gracefully, never throws', async () => {
  const sparse: PillGateway = {
    ...fakeGateway,
    async impact() { throw new Error('no impact for this scope'); },
  };
  const vm = await assembleViewModel(sparse, { kind: 'symbol', value: 'detect_agent' });
  assert.strictEqual(vm.blastRadius.count, 0);          // degraded, not crashed
  assert.strictEqual(vm.agentPackStatus, 'ready');
});
```

- [ ] **Step 2: Run test to verify it fails**

Run: `cd editors/vscode && npx tsc -p tsconfig.test.json && node --test out-test/test/contextPillViewModel.test.js`
Expected: FAIL — `Cannot find module '../src/contextPill/viewModel'`.

- [ ] **Step 3: Write minimal implementation**

```typescript
// editors/vscode/src/contextPill/viewModel.ts
import type { Scope } from './scope';

export interface ExportSym { name: string; kind: string }
export interface BodyPreview { found: boolean; file: string; preview: string; truncated: boolean }
export interface BlastRadius { count: number; direct: string[] }
export interface PillHealth { score: number; status: string }
export interface PillFindings { dead: number; cycles: number; twins: number; hotspots: number }

export interface ContextPillViewModel {
  scope: Scope;
  file: string;
  summary: string;
  health: PillHealth;
  blastRadius: BlastRadius;
  exports: ExportSym[];
  deps: string[];
  bodyPreview: BodyPreview | null;
  findings: PillFindings;
  agentPackStatus: 'ready' | 'building' | 'stale';
}

/** Minimal gateway surface the adapter needs — keeps it unit-testable. */
export interface PillGateway {
  contextPack(opts: { file?: string; task?: string }): Promise<any>;
  impact(target: string, transitive?: boolean): Promise<any>;
  health(includeTopRisks?: boolean): Promise<any>;
  bodyRanked(symbol: string, fileHint?: string): Promise<any>;
}

async function safe<T>(p: Promise<T>, fallback: T): Promise<T> {
  try { return await p; } catch { return fallback; }
}

export async function assembleViewModel(gw: PillGateway, scope: Scope): Promise<ContextPillViewModel> {
  const target = scope.value;
  const isFile = scope.kind === 'file';

  const pack = await safe(gw.contextPack({ file: isFile ? target : undefined }), {} as any);
  const impact = await safe(gw.impact(target, true), {} as any);
  const health = await safe(gw.health(true), {} as any);
  const body = scope.kind === 'symbol'
    ? await safe(gw.bodyRanked(target), null)
    : null;

  return {
    scope,
    file: pack.file ?? (isFile ? target : ''),
    summary: pack.summary ?? '',
    health: { score: Number(health.score ?? 0), status: String(health.status ?? 'unknown') },
    blastRadius: {
      count: Number(impact.transitive_count ?? (impact.direct?.length ?? 0)),
      direct: Array.isArray(impact.direct) ? impact.direct : [],
    },
    exports: Array.isArray(pack.exports) ? pack.exports : [],
    deps: Array.isArray(pack.dependencies) ? pack.dependencies : [],
    bodyPreview: body && body.found
      ? { found: true, file: body.file, preview: body.preview, truncated: !!body.truncated }
      : null,
    findings: {
      dead: Number(health.dead ?? 0), cycles: Number(health.cycles ?? 0),
      twins: Number(health.twins ?? 0), hotspots: Number(health.hotspots ?? 0),
    },
    agentPackStatus: 'ready',
  };
}
```

> Note for the implementer: the exact field names on the real `ContextPackResponse`/`HealthResponse`/`impact` may differ from the canned shapes — read `editors/vscode/src/gateway.ts` and map accordingly inside the adapter. Keep the `ContextPillViewModel` contract stable regardless of upstream shape; the `?? fallback` coalescing guarantees the view model is always complete.

- [ ] **Step 4: Run test to verify it passes**

Run: `cd editors/vscode && npx tsc -p tsconfig.test.json && node --test out-test/test/contextPillViewModel.test.js`
Expected: PASS (2 tests).

- [ ] **Step 5: Commit**

```bash
git add editors/vscode/src/contextPill/viewModel.ts editors/vscode/test/contextPillViewModel.test.ts
git commit -m "[claude/interactive] feat(vscode): ContextPillViewModel + adapter (single data contract)"
```

---

## Task 3: Body ranking fallback in gateway (fixes "no body found")

**Files:**
- Modify: `editors/vscode/src/gateway.ts` (add `bodyRanked`)
- Test: extend `editors/vscode/test/contextPillViewModel.test.ts` (or a new `bodyRanked.test.ts` if a pure seam is extracted)

Implements the spec body algorithm: `file` is a ranking hint, never a hard filter; never "no body found" if a body exists anywhere.

- [ ] **Step 1: Write the failing test** (`editors/vscode/test/bodyRanked.test.ts`)

```typescript
import { test } from 'node:test';
import assert from 'node:assert';
import { rankBody } from '../src/gateway';

test('prefers the active-file body when present', () => {
  const r = rankBody('dispatch.py', [
    { file: 'dispatch.py', preview: 'A' },
    { file: 'other.py', preview: 'B' },
  ] as any);
  assert.deepStrictEqual(r, { found: true, file: 'dispatch.py', preview: 'A', truncated: false, candidates: 2 });
});

test('falls back to the single cross-file body, labelled', () => {
  const r = rankBody('config.py', [{ file: 'claude_jsonl.py', preview: 'X' }] as any);
  assert.strictEqual(r.found, true);
  assert.strictEqual(r.file, 'claude_jsonl.py');
});

test('never reports not-found when a body exists somewhere', () => {
  const r = rankBody('nowhere.py', [{ file: 'a.py', preview: 'Y' }, { file: 'b.py', preview: 'Z' }] as any);
  assert.strictEqual(r.found, true);
  assert.strictEqual(r.candidates, 2); // caller shows candidate list
});

test('found=false only when there are genuinely no bodies', () => {
  const r = rankBody('x', [] as any);
  assert.strictEqual(r.found, false);
});
```

- [ ] **Step 2: Run test to verify it fails**

Run: `cd editors/vscode && npx tsc -p tsconfig.test.json && node --test out-test/test/bodyRanked.test.js`
Expected: FAIL — `rankBody` not exported.

- [ ] **Step 3: Write minimal implementation** — add to `editors/vscode/src/gateway.ts`

```typescript
// add near the BodyResponse interface in gateway.ts
export interface RankedBody { found: boolean; file: string; preview: string; truncated: boolean; candidates: number }

/** Pure ranking: file is a hint, not a filter. Exported for unit tests. */
export function rankBody(fileHint: string | undefined, bodies: Array<{ file: string; preview: string; truncated?: boolean }>): RankedBody {
  if (!bodies || bodies.length === 0) {
    return { found: false, file: '', preview: '', truncated: false, candidates: 0 };
  }
  const hit = (fileHint && bodies.find((b) => b.file === fileHint)) || bodies[0];
  return { found: true, file: hit.file, preview: hit.preview, truncated: !!hit.truncated, candidates: bodies.length };
}
```

Then add the gateway method that calls `loctree/body` WITHOUT the hard file filter and ranks client-side:

```typescript
// inside class LoctreeGateway
public async bodyRanked(symbol: string, fileHint?: string): Promise<RankedBody> {
  // Resolve cross-file first (no `file` filter), then rank by hint. This is the
  // fix for the Fala 3 "no body found" bug: file must rank, not filter.
  const resp = await this.body(symbol); // no file -> cross-file resolution
  const bodies = (resp.bodies ?? []).map((b: SymbolBody) => ({
    file: b.file, preview: b.preview ?? b.body ?? '', truncated: !!b.truncated,
  }));
  return rankBody(fileHint, bodies);
}
```

> Note: confirm the real `SymbolBody` fields (`preview`/`body`/`truncated`, `file`) in gateway.ts and map them. If the LSP still hard-filters when `file` is passed, the fix here is precisely to call `body(symbol)` with NO file and rank client-side — no LSP change required. (If cross-file body resolution itself is missing server-side, escalate as a backlog item; the spec marks `loctree/body` ranking as the only sanctioned protocol-adjacent change.)

- [ ] **Step 4: Run test to verify it passes**

Run: `cd editors/vscode && npx tsc -p tsconfig.test.json && node --test out-test/test/bodyRanked.test.js`
Expected: PASS (4 tests).

- [ ] **Step 5: Commit**

```bash
git add editors/vscode/src/gateway.ts editors/vscode/test/bodyRanked.test.ts
git commit -m "[claude/interactive] fix(vscode): body file is a ranking hint, not a filter (no more 'no body found')"
```

---

## Task 4: Agent context builder + task inference (pure)

**Files:**
- Create: `editors/vscode/src/contextPill/agentContext.ts`
- Test: `editors/vscode/test/agentContext.test.ts`

- [ ] **Step 1: Write the failing test**

```typescript
import { test } from 'node:test';
import assert from 'node:assert';
import { buildAgentContextMarkdown, inferTask } from '../src/contextPill/agentContext';
import type { ContextPillViewModel } from '../src/contextPill/viewModel';

const vm: ContextPillViewModel = {
  scope: { kind: 'file', value: 'tb_core/dispatch.py' },
  file: 'tb_core/dispatch.py',
  summary: 'Detects a session file agent and routes to the adapter.',
  health: { score: 92, status: 'green' },
  blastRadius: { count: 7, direct: ['cli.py', 'batch.py', 'main.py'] },
  exports: [{ name: 'parse_jsonl_to_session_record', kind: 'fn' }],
  deps: ['parse_claude_jsonl'],
  bodyPreview: null,
  findings: { dead: 0, cycles: 0, twins: 0, hotspots: 1 },
  agentPackStatus: 'ready',
};

test('markdown pack carries scope, blast radius and provenance', () => {
  const md = buildAgentContextMarkdown(vm, 'refactor dispatch routing');
  assert.match(md, /scope=file:tb_core\/dispatch\.py/);
  assert.match(md, /task: refactor dispatch routing/);
  assert.match(md, /blast radius/i);
  assert.match(md, /cli\.py/);
  assert.match(md, /parse_jsonl_to_session_record/);
});

test('inferTask summarizes changed files from a WIP diff', () => {
  const task = inferTask({ changedFiles: ['tb_core/dispatch.py', 'tb_core/cli.py'], selection: '' });
  assert.match(task, /dispatch\.py/);
});

test('inferTask is empty (not fabricated) when there is no signal', () => {
  assert.strictEqual(inferTask({ changedFiles: [], selection: '' }), '');
});
```

- [ ] **Step 2: Run test to verify it fails**

Run: `cd editors/vscode && npx tsc -p tsconfig.test.json && node --test out-test/test/agentContext.test.js`
Expected: FAIL — module not found.

- [ ] **Step 3: Write minimal implementation**

```typescript
// editors/vscode/src/contextPill/agentContext.ts
import type { ContextPillViewModel } from './viewModel';
import { scopeKey } from './scope';

export interface TaskSignal { changedFiles: string[]; selection: string }

/** Infer the task from IDE signals (WIP diff primary, selection refinement).
 * Returns '' when there is no signal — never fabricate. The caller shows this
 * as editable provenance before copying ("not creepy" contract). */
export function inferTask(sig: TaskSignal): string {
  if (sig.selection.trim()) {
    return `working on: ${sig.selection.trim().slice(0, 120)}`;
  }
  if (sig.changedFiles.length) {
    return `editing ${sig.changedFiles.slice(0, 5).join(', ')}`;
  }
  return '';
}

export function buildAgentContextMarkdown(vm: ContextPillViewModel, task: string): string {
  const lines: string[] = [];
  lines.push(`# Loctree context — ${scopeKey(vm.scope)}`);
  lines.push(`scope=${scopeKey(vm.scope)}`);
  if (task) lines.push(`task: ${task}`);
  lines.push('');
  if (vm.summary) lines.push(`## What it does\n${vm.summary}\n`);
  lines.push(`## Blast radius (risk of change)\nA change here can touch ${vm.blastRadius.count} file(s): ${vm.blastRadius.direct.join(', ')}\n`);
  if (vm.exports.length) lines.push(`## Exports\n${vm.exports.map((e) => `- ${e.kind} ${e.name}`).join('\n')}\n`);
  if (vm.deps.length) lines.push(`## Depends on\n${vm.deps.map((d) => `- ${d}`).join('\n')}\n`);
  if (vm.bodyPreview?.found) lines.push(`## Body (${vm.bodyPreview.file})\n\`\`\`\n${vm.bodyPreview.preview}\n\`\`\`\n`);
  lines.push(`## Findings\nhealth ${vm.health.score}/100 · ${vm.findings.hotspots} hotspot(s) · ${vm.findings.dead} dead · ${vm.findings.cycles} cycles`);
  return lines.join('\n');
}
```

- [ ] **Step 4: Run test to verify it passes**

Run: `cd editors/vscode && npx tsc -p tsconfig.test.json && node --test out-test/test/agentContext.test.js`
Expected: PASS (3 tests).

- [ ] **Step 5: Commit**

```bash
git add editors/vscode/src/contextPill/agentContext.ts editors/vscode/test/agentContext.test.ts
git commit -m "[claude/interactive] feat(vscode): agent context markdown builder + task inference"
```

---

## Task 5: Hardened webview panel shell

**Files:**
- Create: `editors/vscode/src/contextPill/panel.ts`
- Reference: VS Code `WebviewViewProvider` API; `editors/vscode/src/gateway.ts`

This is the VS Code shell (not unit-tested in node:test; verified by the live VSIX smoke). It owns CSP/nonce, the message allowlist, ambient binding, and posting the view model.

- [ ] **Step 1: Implement the provider with hardened webview**

```typescript
// editors/vscode/src/contextPill/panel.ts
import * as vscode from 'vscode';
import { assembleViewModel, type PillGateway } from './viewModel';
import { parseScopeInput, type Scope } from './scope';
import { buildAgentContextMarkdown, inferTask } from './agentContext';

const ALLOWED_MESSAGES = new Set(['rescope', 'copyAgentContext', 'openFile', 'showFullBody']);

export class ContextPillViewProvider implements vscode.WebviewViewProvider {
  public static readonly viewId = 'loctree.context';
  private view?: vscode.WebviewView;
  private currentScope: Scope = { kind: 'file', value: '' };

  constructor(
    private readonly context: vscode.ExtensionContext,
    private readonly gateway: PillGateway & { /* full LoctreeGateway */ },
    private readonly getTaskSignal: () => { changedFiles: string[]; selection: string },
  ) {}

  resolveWebviewView(view: vscode.WebviewView): void {
    this.view = view;
    view.webview.options = {
      enableScripts: true,
      localResourceRoots: [vscode.Uri.joinPath(this.context.extensionUri, 'media')],
    };
    view.webview.html = this.html(view.webview);
    view.webview.onDidReceiveMessage((msg) => this.onMessage(msg));
    void this.rescopeToActiveEditor();
  }

  /** Ambient: called from extension's onDidChangeActiveTextEditor. */
  public async rescopeToActiveEditor(): Promise<void> {
    const ed = vscode.window.activeTextEditor;
    if (!ed) return;
    const rel = vscode.workspace.asRelativePath(ed.document.uri);
    await this.rescope({ kind: 'file', value: rel });
  }

  private async rescope(scope: Scope): Promise<void> {
    this.currentScope = scope;
    const vm = await assembleViewModel(this.gateway, scope);
    this.view?.webview.postMessage({ type: 'render', vm });
  }

  private async onMessage(msg: any): Promise<void> {
    if (!msg || typeof msg.type !== 'string' || !ALLOWED_MESSAGES.has(msg.type)) return; // allowlist
    switch (msg.type) {
      case 'rescope':
        await this.rescope(parseScopeInput(String(msg.query ?? '')));
        break;
      case 'copyAgentContext': {
        const vm = await assembleViewModel(this.gateway, this.currentScope);
        const task = String(msg.task ?? '') || inferTask(this.getTaskSignal());
        await vscode.env.clipboard.writeText(buildAgentContextMarkdown(vm, task));
        this.view?.webview.postMessage({ type: 'copied' });
        break;
      }
      case 'openFile':
        if (typeof msg.file === 'string') {
          const uri = vscode.Uri.joinPath(this.context.extensionUri, '..'); // resolved against workspace in real impl
          await vscode.commands.executeCommand('loctree.navigateToFile', { filePath: msg.file });
        }
        break;
      case 'showFullBody':
        await vscode.commands.executeCommand('loctree.showBody', String(msg.symbol ?? ''));
        break;
    }
  }

  private html(webview: vscode.Webview): string {
    const nonce = getNonce();
    const cssUri = webview.asWebviewUri(vscode.Uri.joinPath(this.context.extensionUri, 'media', 'contextPill.css'));
    const jsUri = webview.asWebviewUri(vscode.Uri.joinPath(this.context.extensionUri, 'media', 'contextPill.js'));
    const csp = [
      `default-src 'none'`,
      `style-src ${webview.cspSource}`,
      `script-src 'nonce-${nonce}'`,
      `img-src ${webview.cspSource}`,
    ].join('; ');
    return `<!DOCTYPE html><html><head>
      <meta http-equiv="Content-Security-Policy" content="${csp}">
      <link href="${cssUri}" rel="stylesheet">
    </head><body>
      <div id="pill" aria-live="polite"></div>
      <script nonce="${nonce}" src="${jsUri}"></script>
    </body></html>`;
  }
}

function getNonce(): string {
  const chars = 'ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789';
  let s = '';
  for (let i = 0; i < 32; i++) s += chars.charAt(Math.floor(Math.random() * chars.length));
  return s;
}
```

- [ ] **Step 2: Type-check**

Run: `cd editors/vscode && npx tsc --noEmit -p ./`
Expected: PASS (resolve any gateway-type mismatches by widening the injected gateway type to the real `LoctreeGateway`).

- [ ] **Step 3: Commit**

```bash
git add editors/vscode/src/contextPill/panel.ts
git commit -m "[claude/interactive] feat(vscode): hardened Context Pill webview provider (CSP, nonce, msg allowlist, ambient)"
```

---

## Task 6: Webview client (render + CTA states), no emoji

**Files:**
- Create: `editors/vscode/media/contextPill.js`
- Create: `editors/vscode/media/contextPill.css`

The client renders the `ContextPillViewModel` per the hero mockup
(`docs/design/loctree-sight-context-pill-hero.png`): header (file + health + blast
badges), CO ROBI, BLAST RADIUS decision band, two columns (exports ‖ deps), two
columns (body preview ‖ findings), scope-switcher input, sticky CTA. **No emoji**;
severity shown via CSS color dots. **All strings escaped** before insertion.

- [ ] **Step 1: Implement the client**

```javascript
// editors/vscode/media/contextPill.js
(function () {
  const vscode = acquireVsCodeApi();
  const root = document.getElementById('pill');

  function esc(s) {
    return String(s ?? '').replace(/[&<>"']/g, (c) => (
      { '&': '&amp;', '<': '&lt;', '>': '&gt;', '"': '&quot;', "'": '&#39;' }[c]
    ));
  }
  function tags(items, cls, label) {
    return (items || []).map((t) => `<span class="tag ${cls}">${esc(typeof t === 'string' ? t : (t.kind ? t.kind + ' ' + t.name : t.name))}</span>`).join(' ');
  }

  function render(vm) {
    root.innerHTML = `
      <header class="pill-head">
        <span class="file">${esc(vm.file || vm.scope.value)}</span>
        <span class="badges"><span class="badge health">health ${esc(vm.health.score)}</span>
        <span class="badge blast">blast radius ${esc(vm.blastRadius.count)}</span></span>
      </header>
      <input id="scope" class="scope-switcher" placeholder="Inspect symbol or file…" />
      <div class="label">CO ROBI</div><p class="summary">${esc(vm.summary)}</p>
      <section class="decision-band">
        <div class="label warn">BLAST RADIUS · RYZYKO ZMIANY</div>
        <p>A change here can touch ${esc(vm.blastRadius.count)} file(s).</p>
        <div>${tags(vm.blastRadius.direct, 'blast')}</div>
      </section>
      <div class="cols">
        <div><div class="label ok">DEFINIUJE / EKSPORTUJE · ${esc(vm.exports.length)}</div>${tags(vm.exports, 'export')}</div>
        <div><div class="label dep">TEN PLIK UŻYWA · ${esc(vm.deps.length)}</div>${tags(vm.deps, 'dep')}</div>
      </div>
      <div class="cols">
        <div class="body-preview"><div class="label">BODY PREVIEW</div>
          ${vm.bodyPreview && vm.bodyPreview.found ? `<pre>${esc(vm.bodyPreview.preview)}</pre><button id="full">show full body</button>` : '<p class="muted">—</p>'}
        </div>
        <div class="findings"><div class="label danger">FINDINGS</div>
          <p><span class="dot danger"></span>${esc(vm.findings.hotspots)} hotspot · ${esc(vm.findings.dead)} dead · ${esc(vm.findings.cycles)} cycles</p>
          <p class="muted">Snapshot: fresh · Agent pack: ${esc(vm.agentPackStatus)}</p>
        </div>
      </div>
      <div class="task"><input id="task" placeholder="task: inferred from WIP diff (editable)"/></div>
      <button id="cta" class="cta">Skopiuj kontekst dla agenta</button>
    `;
    document.getElementById('scope').addEventListener('change', (e) =>
      vscode.postMessage({ type: 'rescope', query: e.target.value }));
    document.getElementById('cta').addEventListener('click', () =>
      vscode.postMessage({ type: 'copyAgentContext', task: (document.getElementById('task')||{}).value || '' }));
    const full = document.getElementById('full');
    if (full) full.addEventListener('click', () => vscode.postMessage({ type: 'showFullBody', symbol: vm.scope.value }));
  }

  window.addEventListener('message', (ev) => {
    const m = ev.data;
    if (m.type === 'render') render(m.vm);
    if (m.type === 'copied') {
      const cta = document.getElementById('cta');
      if (cta) { cta.textContent = 'Copied Agent Context'; cta.classList.add('copied'); }
    }
  });
})();
```

- [ ] **Step 2: Implement the CSS** (`editors/vscode/media/contextPill.css`)

Write a stylesheet using `var(--vscode-*)` theme variables for base colors, plus the section accent colors from the hero (exports green, deps blue, blast amber, findings red), a boxed `.decision-band` (amber border), `.tag` pills, `.dot` severity dots, a sticky `.cta` (full-width, accent), and `.copied` state. No emoji, no remote fonts/assets. Keep it ~80 lines, theme-driven.

- [ ] **Step 3: Verify bundling includes media** — confirm `.vscodeignore` does NOT exclude `media/**` (it already ships `media/`), and `esbuild` doesn't need the media (served as webview assets).

Run: `cd editors/vscode && npx tsc --noEmit -p ./ && node esbuild.js`
Expected: PASS; `dist/extension.js` built.

- [ ] **Step 4: Commit**

```bash
git add editors/vscode/media/contextPill.js editors/vscode/media/contextPill.css
git commit -m "[claude/interactive] feat(vscode): Context Pill webview client + styling (no emoji, escaped)"
```

---

## Task 7: Wire ambient binding, command, and view registration

**Files:**
- Modify: `editors/vscode/src/extension.ts`
- Modify: `editors/vscode/package.json`
- Modify: `editors/vscode/src/gateway.ts` (ensure it satisfies `PillGateway` — `contextPack`, `impact`, `health`, `bodyRanked` are present)

- [ ] **Step 1: package.json — make `loctree.context` a webview view + add the CTA command**

In `contributes.views`, change the `loctree.context` entry to `"type": "webview"` (keep id `loctree.context`, name "Loctree Context"). Keep the `loctree.findings` tree view. Add to `contributes.commands`:

```json
{ "command": "loctree.copyAgentContext", "title": "Loctree: Skopiuj kontekst dla agenta", "category": "Loctree" }
```

Remove `loctree.contextQuery` from being the primary entry (keep expert commands: analyzeImpact, showSlice, searchLiteral, showBody, findImporters, findConsumers, showCycles — they stay as palette power tools).

- [ ] **Step 2: extension.ts — register the provider + ambient listener**

```typescript
// in activate(), after gateway is created:
import { ContextPillViewProvider } from './contextPill/panel';

const pill = new ContextPillViewProvider(context, gateway as any, () => ({
  changedFiles: [], // wired in Task 8 from git WIP
  selection: vscode.window.activeTextEditor?.document.getText(vscode.window.activeTextEditor.selection) ?? '',
}));
context.subscriptions.push(
  vscode.window.registerWebviewViewProvider(ContextPillViewProvider.viewId, pill),
  vscode.window.onDidChangeActiveTextEditor(() => void pill.rescopeToActiveEditor()),
  vscode.commands.registerCommand('loctree.copyAgentContext', () => pill.rescopeToActiveEditor()),
);
```

Remove/stop registering the old tree-based context panel as the `loctree.context` provider (the Findings tree provider stays).

- [ ] **Step 3: Type-check + build**

Run: `cd editors/vscode && npx tsc --noEmit -p ./ && node esbuild.js`
Expected: PASS.

- [ ] **Step 4: Commit**

```bash
git add editors/vscode/src/extension.ts editors/vscode/package.json
git commit -m "[claude/interactive] feat(vscode): register Context Pill webview + ambient active-editor binding"
```

---

## Task 8: Task inference from WIP diff (extension side)

**Files:**
- Modify: `editors/vscode/src/extension.ts` (or a small `src/contextPill/taskSignal.ts`)

- [ ] **Step 1: Implement `getChangedFiles()` via the built-in git extension**

```typescript
// src/contextPill/taskSignal.ts
import * as vscode from 'vscode';

export function getChangedFiles(): string[] {
  const git = vscode.extensions.getExtension('vscode.git')?.exports?.getAPI?.(1);
  const repo = git?.repositories?.[0];
  if (!repo) return [];
  const changes = [...(repo.state.workingTreeChanges ?? []), ...(repo.state.indexChanges ?? [])];
  return changes.map((c: any) => vscode.workspace.asRelativePath(c.uri)).slice(0, 20);
}
```

Wire `getChangedFiles()` into the `getTaskSignal` callback passed to `ContextPillViewProvider` in extension.ts.

- [ ] **Step 2: Type-check + build**

Run: `cd editors/vscode && npx tsc --noEmit -p ./ && node esbuild.js`
Expected: PASS.

- [ ] **Step 3: Commit**

```bash
git add editors/vscode/src/contextPill/taskSignal.ts editors/vscode/src/extension.ts
git commit -m "[claude/interactive] feat(vscode): infer agent task from git WIP diff (stable, non-creepy)"
```

---

## Task 9: Full gate + live VSIX smoke

- [ ] **Step 1: All unit tests + lint + types**

Run: `cd editors/vscode && npm test && npm run lint && npm run check-types`
Expected: all green (scope, viewModel, bodyRanked, agentContext tests + existing tests).

- [ ] **Step 2: Build + package the VSIX**

Run: `cd /Users/silver/Git/loctree-suite && cargo build --release -p loctree-lsp && cd editors/vscode && npm run package`
Expected: `loctree-<ver>.vsix` with `bin/loctree-lsp` + `dist/extension.js` + `media/contextPill.{js,css}`.

- [ ] **Step 3: Install + live smoke**

Run: `code --install-extension loctree-<ver>.vsix --force` then open a repo. Verify against spec criteria:
- Pill appears for the active file with summary/exports/deps/blast radius — no query.
- Decision band answers "what breaks if I change this".
- `Inspect symbol or file…` re-scopes the same pill (symbol/file/literal-fallback), no dead-ends.
- Body preview is bounded with `show full body`; a symbol not in the active file still resolves (no "no body found").
- CTA copies markdown, button → "Copied Agent Context"; task is visible/editable.
- Read the exthost log + "Loctree" output channel: activation, LSP start, no errors, no CSP violations.

- [ ] **Step 4: Commit any fixes from the smoke, then stop**

Leave the branch ahead, unpushed (push is the operator's decision).

---

## Self-review notes (run before execution)
- Spec coverage: ambient pill (T5/T7), scope switcher + taxonomy (T1/T5/T6), view-model adapter (T2), body algorithm (T3), agent CTA + task inference (T4/T8), webview security (T5/T6), findings via health/contextPack (T2), no-emoji (T6). JetBrains mirror + Kotlin indexing intentionally excluded (separate plans, see spec dependency).
- The `PillGateway` interface (T2) must be satisfied by the real `LoctreeGateway`; T3 adds `bodyRanked`, T2 needs `contextPack/impact/health`. Verify method names match gateway.ts during T2/T7 and adjust the adapter mapping (the spec's data-source note governs).
- No placeholders: every task has concrete code; CSS in T6 is described by exact selectors/colors rather than 80 lines verbatim — the executing agent writes it against the hero PNG and theme vars (acceptable: it is presentational, fully specified by the mockup + variable list).
