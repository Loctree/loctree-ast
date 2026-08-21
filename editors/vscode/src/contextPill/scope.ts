/** The pill always has exactly one scope. */
export type Scope =
  | { kind: 'file'; value: string }
  | { kind: 'symbol'; value: string; file?: string }
  | { kind: 'literal'; value: string }
  | { kind: 'out-of-workspace'; value: string };

/** Stable `kind:value` identity for a scope — distinct per kind, so a symbol and
 *  a literal of the same text never collide. */
export function scopeKey(scope: Scope): string {
  return `${scope.kind}:${scope.value}`;
}

/** A bare identifier: routes the input to symbol scope. */
const IDENT_RE = /^[A-Za-z_][A-Za-z0-9_]*$/;
/** A path separator or a file extension: routes the input to file scope. */
const PATH_RE = /[\\/]|\.[A-Za-z0-9]+$/;

/**
 * Route raw search input to a scope. NOT a search engine — it decides what the
 * pill should look at next. file (separator/extension) -> symbol (single ident)
 * -> literal (anything else). Zero/many symbol resolution is the caller's job.
 */
export function parseScopeInput(raw: string): Scope {
  const q = raw.trim();
  if (PATH_RE.test(q) && !q.includes(' ')) return { kind: 'file', value: q };
  if (IDENT_RE.test(q)) return { kind: 'symbol', value: q };
  return { kind: 'literal', value: q };
}
