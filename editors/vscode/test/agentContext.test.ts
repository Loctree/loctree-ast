import { test } from 'node:test';
import assert from 'node:assert';
import { buildAgentContextMarkdown, inferTask } from '../src/contextPill/agentContext';
import { applyLiteralHarvest, mergeLiteralGroups } from '../src/contextPill/viewModel';
import type { ContextPillViewModel } from '../src/contextPill/viewModel';

const vm: ContextPillViewModel = {
  scope: { kind: 'file', value: 'tb_core/dispatch.py' },
  file: 'tb_core/dispatch.py',
  state: 'ready',
  lsp: { phase: 'running', label: 'Running', message: 'Initialize handshake completed.' },
  summary: 'Detects a session file agent and routes to the adapter.',
  health: { score: 92, status: 'green' },
  repoHealth: 92,
  fileLoc: 120,
  blastRadius: { count: 7, direct: ['cli.py', 'batch.py', 'main.py'] },
  exports: [{ name: 'parse_jsonl_to_session_record', kind: 'fn' }],
  deps: ['parse_claude_jsonl'],
  bodyPreview: null,
  findings: { dead: 0, cycles: 0, twins: 0, hotspots: 1 },
  repoFindings: { dead: 0, cycles: 0, twins: 0, hotspots: 1 },
  fileRisks: [],
  agentPackStatus: 'ready',
  literalGroups: [],
  literalTotal: 0,
  literalHasMore: false,
  literalNextOffset: null,
};

test('markdown pack carries scope, blast radius and provenance', () => {
  const md = buildAgentContextMarkdown(vm, 'refactor dispatch routing');
  assert.match(md, /scope=file:tb_core\/dispatch\.py/);
  assert.match(md, /task: refactor dispatch routing/);
  assert.match(md, /blast radius/i);
  assert.match(md, /cli\.py/);
  assert.match(md, /parse_jsonl_to_session_record/);
});

test('findings emit PER-FILE risks, with repo totals on a SEPARATE line', () => {
  const fileVm: ContextPillViewModel = {
    ...vm,
    fileRisks: [
      { kind: 'hotspot', severity: 'high', message: 'high churn × fan-in' },
      { kind: 'dead_export', severity: 'low', message: 'unused export foo' },
    ],
    repoFindings: { dead: 4, cycles: 2, twins: 0, hotspots: 9 },
    repoHealth: 81,
  };
  const md = buildAgentContextMarkdown(fileVm, 'refactor');
  // Per-file findings list (humanized kind + message), not repo numbers.
  assert.match(md, /## Findings\n- Hotspot — high churn × fan-in\n- Dead export — unused export foo/);
  // Repo totals on their own line, clearly labelled.
  assert.match(md, /Repo-wide: 9 hotspots · 4 dead · 2 cycles/);
  assert.match(md, /Repo health: 81\/100/);
  // The old leak (health score inline with per-file findings) is gone.
  assert.doesNotMatch(md, /## Findings\nhealth/);
});

test('findings say "No issues in this file" when fileRisks is empty', () => {
  const md = buildAgentContextMarkdown(vm, '');
  assert.match(md, /## Findings\nNo issues in this file/);
  assert.match(md, /Repo-wide: 1 hotspots · 0 dead · 0 cycles/);
});

test('literal scope omits findings entirely (N\/A for a string match)', () => {
  const litVm: ContextPillViewModel = {
    ...vm,
    scope: { kind: 'literal', value: 'TODO' },
    literalGroups: [{ file: 'a.py', lines: [3] }],
    literalTotal: 1,
  };
  const md = buildAgentContextMarkdown(litVm, '');
  assert.doesNotMatch(md, /## Findings/);
  assert.doesNotMatch(md, /Repo-wide:/);
});

test('inferTask summarizes changed files from a WIP diff', () => {
  const task = inferTask({ changedFiles: ['tb_core/dispatch.py', 'tb_core/cli.py'], selection: '' });
  assert.match(task, /dispatch\.py/);
});

test('inferTask is empty (not fabricated) when there is no signal', () => {
  assert.strictEqual(inferTask({ changedFiles: [], selection: '' }), '');
});

test('markdown pack lists literal occurrences for a literal scope', () => {
  const litVm: ContextPillViewModel = {
    ...vm,
    scope: { kind: 'literal', value: 'TODO' },
    literalGroups: [
      { file: 'a.py', lines: [3, 10] },
      { file: 'b.py', lines: [7] },
    ],
    literalTotal: 3,
    literalHasMore: false,
    literalNextOffset: null,
  };
  const md = buildAgentContextMarkdown(litVm, 'audit TODOs');
  assert.match(md, /scope=literal:TODO/);
  assert.match(md, /Literal occurrences — "TODO"/);
  assert.match(md, /a\.py: 3, 10/);
  assert.match(md, /b\.py: 7/);
  // structural sections should NOT appear for a literal scope.
  assert.doesNotMatch(md, /Blast radius/i);
});

test('Copy harvests every loaded literal page, dropping the "first page" caveat', () => {
  // What assembleViewModel builds for a literal scope: only the first page +
  // hasMore. Copying THIS (the pre-fix behaviour) leaks a partial set.
  const firstPageVm: ContextPillViewModel = {
    ...vm,
    scope: { kind: 'literal', value: 'parseScope' },
    literalGroups: [{ file: 'a.ts', lines: [1, 2] }],
    literalTotal: 4,
    literalHasMore: true,
    literalNextOffset: 50,
  };
  const partial = buildAgentContextMarkdown(firstPageVm, '');
  assert.match(partial, /\(showing first page\)/);
  assert.doesNotMatch(partial, /b\.ts/);

  // The panel accumulates page 2 via Load-more, then Copy overlays the harvest.
  const harvest = {
    query: 'parseScope',
    groups: mergeLiteralGroups(
      [{ file: 'a.ts', lines: [1, 2] }],
      [
        { file: 'a.ts', lines: [2, 9] },
        { file: 'b.ts', lines: [4] },
      ],
    ),
    total: 4,
    hasMore: false,
  };
  const md = buildAgentContextMarkdown(applyLiteralHarvest(firstPageVm, harvest), '');
  // Every occurrence across both pages, de-duped (a.ts line 2 appears once).
  assert.match(md, /a\.ts: 1, 2, 9/);
  assert.match(md, /b\.ts: 4/);
  // Once the whole set is loaded there is no partial-page caveat.
  assert.doesNotMatch(md, /\(showing first page\)/);
});

test('applyLiteralHarvest is a no-op for a mismatched query or non-literal scope', () => {
  const litVm: ContextPillViewModel = {
    ...vm,
    scope: { kind: 'literal', value: 'X' },
    literalGroups: [{ file: 'a.ts', lines: [1] }],
    literalTotal: 1,
  };
  // Mismatched harvest query → unchanged VM (same reference).
  assert.strictEqual(
    applyLiteralHarvest(litVm, { query: 'Y', groups: [], total: 9, hasMore: true }),
    litVm,
  );
  // Non-literal (file) scope → harvest never applies.
  assert.strictEqual(
    applyLiteralHarvest(vm, { query: 'file:tb_core/dispatch.py', groups: [], total: 9, hasMore: true }),
    vm,
  );
  // Null harvest → unchanged.
  assert.strictEqual(applyLiteralHarvest(litVm, null), litVm);
});
