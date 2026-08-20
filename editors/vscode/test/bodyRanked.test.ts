import { test } from 'node:test';
import assert from 'node:assert';
import { rankBody } from '../src/gateway';

test('prefers the active-file body when present', () => {
  const r = rankBody('dispatch.py', [{ file: 'dispatch.py', preview: 'A' }, { file: 'other.py', preview: 'B' }] as any);
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
  assert.strictEqual(r.candidates, 2);
});
test('found=false only when there are genuinely no bodies', () => {
  const r = rankBody('x', [] as any);
  assert.strictEqual(r.found, false);
});
