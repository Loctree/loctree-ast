import { test } from 'node:test';
import assert from 'node:assert';
import { parseScopeInput, scopeKey } from '../src/contextPill/scope';

test('bare filename routes to file scope', () => {
  assert.deepStrictEqual(parseScopeInput('dispatch.py'), { kind: 'file', value: 'dispatch.py' });
});
test('path fragment routes to file scope', () => {
  assert.deepStrictEqual(parseScopeInput('editors/vscode/src/gateway.ts'), { kind: 'file', value: 'editors/vscode/src/gateway.ts' });
});
test('identifier routes to symbol scope', () => {
  assert.deepStrictEqual(parseScopeInput('parse_jsonl_to_session_record'), { kind: 'symbol', value: 'parse_jsonl_to_session_record' });
});
test('multi-word / spaced query routes to literal scope', () => {
  assert.deepStrictEqual(parseScopeInput('blast radius'), { kind: 'literal', value: 'blast radius' });
});
test('scopeKey is stable and distinct per kind', () => {
  assert.strictEqual(scopeKey({ kind: 'file', value: 'a.ts' }), 'file:a.ts');
  assert.notStrictEqual(scopeKey({ kind: 'symbol', value: 'x' }), scopeKey({ kind: 'literal', value: 'x' }));
});
