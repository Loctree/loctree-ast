//! Bounded symbol body / source-range retrieval.
//!
//! Closes the "Loctree got me to the door, grep opened it" gap: once
//! `where-symbol` locates a symbol's defining file + line, this module
//! returns the bounded source text of that symbol's body without the agent
//! ever shelling out to `grep`/`sed`/`awk`.
//!
//! Body extraction resolves a real extent whenever the language allows it:
//! brace balancing (Rust, Swift, TS/JS, C-family), bracket balancing for
//! assignment-opened collections, indentation blocks for Python `def`/`class`,
//! and single-statement lines. Only when no extent can be proven does it fall
//! back to a fixed line window — and that fallback is reported as
//! `truncated: true` with `extent: "window"`, because an unproven boundary is
//! not a closed body. Output is always bounded by a line cap with explicit
//! truncation metadata.
//!
//! 𝚅𝚒𝚋𝚎𝚌𝚛𝚊𝚏𝚝𝚎𝚍. with AI Agents ⓒ 2025-2026 Loctree Team

use serde::{Deserialize, Serialize};

use crate::query::query_where_symbol;
use crate::snapshot::Snapshot;

/// Default maximum number of source lines returned for a single body.
pub const DEFAULT_BODY_LINE_CAP: usize = 200;

/// Fallback line window (lines after the definition line) for symbols whose
/// body extent could not be proven by any structural strategy.
const FALLBACK_WINDOW: usize = 40;

/// Extent proven by `{...}` brace balancing.
pub const EXTENT_BRACE: &str = "brace";
/// Extent proven by balancing an assignment-opened `(`/`[` collection.
pub const EXTENT_BRACKET: &str = "bracket";
/// Extent proven by Python indentation-block scanning (`def`/`class`).
pub const EXTENT_INDENT: &str = "indent";
/// Extent proven as a complete single-line statement.
pub const EXTENT_LINE: &str = "line";
/// No extent proven — fixed fallback window; always reported truncated.
pub const EXTENT_WINDOW: &str = "window";

/// A bounded source body for a single symbol definition.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SymbolBody {
    /// Symbol name that was queried.
    pub symbol: String,
    /// File the body was extracted from (repo-relative path).
    pub file: String,
    /// 1-based start line of the body.
    pub start_line: usize,
    /// 1-based end line of the body (inclusive) actually returned.
    pub end_line: usize,
    /// Detected language (file extension, lowercase) or "unknown".
    pub language: String,
    /// Bounded source text (already capped to `line_cap`).
    pub source: String,
    /// True if the returned source is not provably the complete body: either
    /// the body exceeded `line_cap`, or no closing boundary could be proven
    /// (`extent == "window"`).
    pub truncated: bool,
    /// Total lines the full body would have spanned (pre-cap). For
    /// `extent == "window"` this is the window size, not a proven body length.
    pub total_lines: usize,
    /// Line cap that was applied.
    pub line_cap: usize,
    /// How the end boundary was determined: `"brace"`, `"bracket"`,
    /// `"indent"`, `"line"`, or `"window"` (unproven fallback).
    pub extent: String,
}

/// Aggregate result of a `loct body <symbol>` lookup.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BodyResult {
    /// Symbol name queried.
    pub symbol: String,
    /// Bodies found (one per defining file/line).
    pub bodies: Vec<SymbolBody>,
}

impl BodyResult {
    /// Keep only bodies defined in `file`, matching either the exact
    /// repo-relative path or a path-component suffix (`src/a.rs` matches
    /// `crate/src/a.rs` but not `crate/miscsrc/a.rs`). `None`/empty filter
    /// keeps every candidate. This is the shared disambiguation used by the
    /// CLI `--file` flag and the LSP `loctree/body` `file` param.
    pub fn filtered_to_file(mut self, file: Option<&str>) -> Self {
        if let Some(needle) = file.filter(|f| !f.is_empty()) {
            let suffix = format!("/{needle}");
            self.bodies
                .retain(|b| b.file == needle || b.file.ends_with(&suffix));
        }
        self
    }
}

/// Derive a lowercase language tag from a file path's extension.
fn language_of(path: &str) -> String {
    path.rsplit('.')
        .next()
        .filter(|ext| *ext != path)
        .map(|ext| ext.to_lowercase())
        .unwrap_or_else(|| "unknown".to_string())
}

/// If `line` is a plain assignment whose right-hand side opens a `(` tuple or
/// `[` list, return the byte offset just after the `=`. This is the signal that
/// the body is a bracket-delimited collection const (e.g.
/// `FRAMEWORK_LAUNCHER_MARKERS = (`) rather than a brace body or a `def`.
///
/// Returns `None` for `==`/`<=`/augmented/walrus operators, for `def f(...):`
/// (its paren is not preceded by a plain `=`), and for dict/object `{`
/// assignments (those keep the existing brace-balanced path). The returned
/// offset is where bracket balancing should begin counting on the first line.
fn assignment_collection_rhs(line: &str) -> Option<usize> {
    let bytes = line.as_bytes();
    let mut i = 0;
    while i < bytes.len() {
        if bytes[i] == b'=' {
            let next = bytes.get(i + 1).copied().unwrap_or(b' ');
            if next == b'=' {
                i += 2;
                continue;
            }
            let prev = if i > 0 { bytes[i - 1] } else { b' ' };
            if matches!(
                prev,
                b'!' | b'<'
                    | b'>'
                    | b'+'
                    | b'-'
                    | b'*'
                    | b'/'
                    | b'%'
                    | b'&'
                    | b'|'
                    | b'^'
                    | b'@'
                    | b':'
                    | b'~'
                    | b'='
            ) {
                return None;
            }
            let rhs = line[i + 1..].trim_start();
            if rhs.starts_with('(') || rhs.starts_with('[') {
                return Some(i + 1);
            }
            return None;
        }
        i += 1;
    }
    None
}

/// Bracket-balanced scan over `(`/`[`/`{` (and their closers), starting at
/// `start_idx` from byte offset `first_byte_offset` on that line. Returns the
/// 0-based line index where the outermost bracket closes, or `None` if it never
/// balances. Shares the quote/escape/comment shielding of the brace scanner so
/// brackets inside strings or comments do not derail the count.
fn extract_bracket_balanced(
    lines: &[&str],
    start_idx: usize,
    first_byte_offset: usize,
    language: &str,
) -> Option<usize> {
    let rust_quotes = language == "rs";
    let mut depth: i32 = 0;
    let mut started = false;
    let mut in_string: Option<char> = None;
    for (i, line) in lines.iter().enumerate().skip(start_idx) {
        let chars: Vec<char> = line.chars().collect();
        let mut escaped = false;
        let mut idx = if i == start_idx {
            line[..first_byte_offset.min(line.len())].chars().count()
        } else {
            0
        };
        while idx < chars.len() {
            let ch = chars[idx];
            if let Some(q) = in_string {
                if escaped {
                    escaped = false;
                } else if ch == '\\' {
                    escaped = true;
                } else if ch == q {
                    in_string = None;
                }
                idx += 1;
                continue;
            }
            match ch {
                '"' | '`' => in_string = Some(ch),
                '\'' => {
                    if rust_quotes {
                        let opens_char_literal =
                            chars.get(idx + 1) == Some(&'\\') || chars.get(idx + 2) == Some(&'\'');
                        if opens_char_literal {
                            in_string = Some('\'');
                        }
                    } else {
                        in_string = Some('\'');
                    }
                }
                // Python `#` and C-family `//` comments run to end-of-line.
                '#' => break,
                '/' if chars.get(idx + 1) == Some(&'/') => break,
                '(' | '[' | '{' => {
                    depth += 1;
                    started = true;
                }
                ')' | ']' | '}' => {
                    depth -= 1;
                    if started && depth == 0 {
                        return Some(i);
                    }
                }
                _ => {}
            }
            idx += 1;
        }
    }
    None
}

/// True for languages whose blocks are indentation-scoped and comments start
/// with `#` (the Python family).
fn is_python_family(language: &str) -> bool {
    matches!(language, "py" | "pyi" | "pyw")
}

/// Indentation-block extent for a Python `def`/`class` at `start_idx`.
///
/// Returns the 0-based index of the last statement line of the block: the
/// scan first balances a multi-line signature paren (if any), then walks
/// forward until the first non-blank, non-comment line whose indentation is
/// at or below the definition line's. Blank and comment-only lines neither
/// end nor extend the block, matching Python `ast` `end_lineno` semantics.
/// Returns `None` when `start_idx` is not a `def`/`class` line or when an
/// opened signature paren never balances (unprovable extent).
fn python_block_extent(lines: &[&str], start_idx: usize) -> Option<usize> {
    let start_line = lines[start_idx];
    let stripped = start_line.trim_start();
    let is_block_opener = stripped.starts_with("def ")
        || stripped.starts_with("async def ")
        || stripped.starts_with("class ");
    if !is_block_opener {
        return None;
    }
    let def_indent = start_line.len() - stripped.len();

    // Multi-line signatures: the body cannot start before the signature's
    // paren closes, and its continuation lines may sit at any indentation.
    let sig_end = match start_line.find('(') {
        Some(offset) => extract_bracket_balanced(lines, start_idx, offset, "py")?,
        None => start_idx,
    };

    let mut last_content = sig_end;
    for (i, line) in lines.iter().enumerate().skip(sig_end + 1) {
        let trimmed = line.trim_start();
        if trimmed.is_empty() || trimmed.starts_with('#') {
            continue;
        }
        if line.len() - trimmed.len() <= def_indent {
            break;
        }
        last_content = i;
    }
    Some(last_content)
}

/// True when `line` is a complete single-line statement: all brackets balance,
/// no string is left open, and the last code character (ignoring trailing
/// comments) proves completion — `;` for C-family/Rust, or any terminator
/// except `:`/`\`/`=` for the Python family (where a bare `X = 1` has no `;`).
fn line_is_complete_statement(line: &str, language: &str) -> bool {
    let python = is_python_family(language);
    let rust_quotes = language == "rs";
    let chars: Vec<char> = line.chars().collect();
    let mut depth: i32 = 0;
    let mut in_string: Option<char> = None;
    let mut escaped = false;
    let mut last_code: Option<char> = None;
    let mut idx = 0;
    while idx < chars.len() {
        let ch = chars[idx];
        if let Some(q) = in_string {
            if escaped {
                escaped = false;
            } else if ch == '\\' {
                escaped = true;
            } else if ch == q {
                in_string = None;
            }
            idx += 1;
            continue;
        }
        match ch {
            '"' | '`' => in_string = Some(ch),
            '\'' => {
                if rust_quotes {
                    let opens_char_literal =
                        chars.get(idx + 1) == Some(&'\\') || chars.get(idx + 2) == Some(&'\'');
                    if opens_char_literal {
                        in_string = Some('\'');
                    }
                } else {
                    in_string = Some('\'');
                }
            }
            '#' if python => break,
            '/' if chars.get(idx + 1) == Some(&'/') => break,
            '(' | '[' | '{' => {
                depth += 1;
                last_code = Some(ch);
            }
            ')' | ']' | '}' => {
                depth -= 1;
                last_code = Some(ch);
            }
            c if !c.is_whitespace() => last_code = Some(c),
            _ => {}
        }
        idx += 1;
    }
    if depth != 0 || in_string.is_some() {
        return false;
    }
    match last_code {
        None => false,
        Some(last) => {
            if python {
                !matches!(last, ':' | '\\' | '=')
            } else {
                last == ';'
            }
        }
    }
}

/// Resolve the 0-based end index of the body at `start_idx`, together with the
/// extent strategy that proved it. `EXTENT_WINDOW` means no strategy could
/// prove a closing boundary — callers must report that as truncated.
fn resolve_extent(lines: &[&str], start_idx: usize, language: &str) -> (usize, &'static str) {
    // Python `def`/`class` first: an indentation block is the real extent, and
    // it must win over the assignment heuristic (a default arg like
    // `def f(x=(1, 2)):` would otherwise be mistaken for a collection const).
    if is_python_family(language)
        && let Some(end_idx) = python_block_extent(lines, start_idx)
    {
        return (end_idx, EXTENT_INDENT);
    }

    // Assignment-opened tuple/list collection (`NAME = (`/`[`): balance that
    // bracket so a multi-line const returns exactly its own body instead of a
    // fixed window that overshoots into trailing code. Dict/object `{`
    // assignments fall through to the brace path below.
    if let Some(close_idx) = assignment_collection_rhs(lines[start_idx])
        .and_then(|off| extract_bracket_balanced(lines, start_idx, off, language))
    {
        return (close_idx, EXTENT_BRACKET);
    }

    // A complete single-line statement (`pub const CAP: usize = 200;`,
    // `X = 1`) ends on its own line. This must run before the brace lookahead,
    // which would otherwise balance an unrelated `{` further down and claim
    // the const plus trailing code as one closed body.
    if line_is_complete_statement(lines[start_idx], language) {
        return (start_idx, EXTENT_LINE);
    }

    // Look for the opening brace within a small lookahead from the definition.
    let mut brace_open_idx: Option<usize> = None;
    let lookahead_end = (start_idx + 10).min(lines.len());
    'outer: for (offset, line) in lines[start_idx..lookahead_end].iter().enumerate() {
        if line.contains('{') {
            brace_open_idx = Some(start_idx + offset);
            break 'outer;
        }
    }

    if let Some(open_idx) = brace_open_idx {
        // Brace-balanced scan from the opening brace line.
        let rust_quotes = language == "rs";
        let mut depth: i32 = 0;
        let mut found_end = open_idx;
        let mut in_string: Option<char> = None;
        let mut closed = false;
        'scan: for (i, line) in lines.iter().enumerate().skip(open_idx) {
            let chars: Vec<char> = line.chars().collect();
            let mut escaped = false;
            let mut idx = 0;
            while idx < chars.len() {
                let ch = chars[idx];
                if let Some(q) = in_string {
                    // Inside a string/char literal: consume escapes so that
                    // `'\\'` and `"\""` close where they actually close.
                    if escaped {
                        escaped = false;
                    } else if ch == '\\' {
                        escaped = true;
                    } else if ch == q {
                        in_string = None;
                    }
                    idx += 1;
                    continue;
                }
                match ch {
                    '"' | '`' => in_string = Some(ch),
                    '\'' => {
                        if rust_quotes {
                            // Rust: `'` opens a char literal only as `'x'` or
                            // `'\...'`. Lifetimes (`&'a`) and loop labels
                            // (`'scan:`) never close, so treating them as
                            // string openers derails brace balancing.
                            let opens_char_literal = chars.get(idx + 1) == Some(&'\\')
                                || chars.get(idx + 2) == Some(&'\'');
                            if opens_char_literal {
                                in_string = Some('\'');
                            }
                        } else {
                            in_string = Some('\'');
                        }
                    }
                    // Line comment: braces/quotes after `//` are not code.
                    '/' if chars.get(idx + 1) == Some(&'/') => break,
                    // Python-family `#` comments: an unmatched `{` in a
                    // comment must not derail dict-const brace balancing.
                    '#' if is_python_family(language) => break,
                    '{' => depth += 1,
                    '}' => {
                        depth -= 1;
                        if depth == 0 {
                            found_end = i;
                            closed = true;
                            break 'scan;
                        }
                    }
                    _ => {}
                }
                idx += 1;
            }
        }
        if closed {
            return (found_end, EXTENT_BRACE);
        }
    }

    // No provable boundary: fixed window, reported as truncated by callers.
    (
        (start_idx + FALLBACK_WINDOW).min(lines.len().saturating_sub(1)),
        EXTENT_WINDOW,
    )
}

/// A bounded extraction result: the returned range, its source text, and the
/// extent strategy that proved (or failed to prove) the body's end.
struct ExtractedBody {
    /// 1-based inclusive end line actually returned (post-cap).
    end_line: usize,
    source: String,
    /// True when capped below the resolved extent OR the extent is an
    /// unproven `window` — either way the caller does not hold a provably
    /// complete body.
    truncated: bool,
    total_lines: usize,
    extent: &'static str,
}

/// Extract a bounded body starting at `start_idx` (0-based) from `lines`.
///
/// Extent strategies, in order: Python `def`/`class` indentation block,
/// assignment-opened `(`/`[` collection balancing, complete single-line
/// statement, brace balancing, and finally a fixed window when nothing can be
/// proven. Always returns at most `line_cap` lines.
///
/// `language` is the lowercase extension tag from [`language_of`]; it selects
/// quote semantics (Rust `'` is a lifetime/label/char-literal, not a general
/// string quote) and comment syntax.
fn extract_body(
    lines: &[&str],
    start_idx: usize,
    line_cap: usize,
    language: &str,
) -> ExtractedBody {
    let (end_idx, extent) = resolve_extent(lines, start_idx, language);
    let closed = extent != EXTENT_WINDOW;

    let total_lines = end_idx - start_idx + 1;
    let capped_end_idx = (start_idx + line_cap - 1).min(end_idx);
    let truncated = capped_end_idx < end_idx || !closed;

    let source = lines[start_idx..=capped_end_idx].join("\n");
    ExtractedBody {
        end_line: capped_end_idx + 1,
        source,
        truncated,
        total_lines,
        extent,
    }
}

/// Retrieve bounded source bodies for `symbol` using the cached snapshot to
/// locate definitions, then reading source files directly from disk.
///
/// `line_cap` of `None` uses [`DEFAULT_BODY_LINE_CAP`].
/// Read a source file referenced by a snapshot match.
///
/// Snapshot file paths are project-root-relative, so a bare `read_to_string`
/// only succeeds when the process cwd happens to be the project root — which is
/// NOT guaranteed for the LSP server (it can be spawned with any cwd). Try the
/// Absolute paths are read directly. Relative paths MUST resolve against the
/// snapshot roots before any cwd fallback: an MCP/LSP server commonly runs from
/// its own checkout, where a coincidentally named `src/lib.rs` would otherwise
/// return valid-looking source from the wrong repository.
fn read_source(snapshot: &Snapshot, file: &str) -> Option<String> {
    let path = std::path::Path::new(file);
    if path.is_absolute() {
        return std::fs::read_to_string(path).ok();
    }
    for root in &snapshot.metadata.roots {
        if let Ok(content) = std::fs::read_to_string(std::path::Path::new(root).join(path)) {
            return Some(content);
        }
    }
    // Compatibility for old snapshots whose root metadata is empty. This is
    // intentionally last so cwd can never shadow an authoritative root.
    std::fs::read_to_string(path).ok()
}

pub fn query_symbol_body(snapshot: &Snapshot, symbol: &str, line_cap: Option<usize>) -> BodyResult {
    let cap = line_cap.unwrap_or(DEFAULT_BODY_LINE_CAP).max(1);
    let where_result = query_where_symbol(snapshot, symbol);

    let mut bodies = Vec::new();
    let mut seen: std::collections::HashSet<(String, usize)> = std::collections::HashSet::new();

    for m in &where_result.results {
        // We need a concrete line to anchor body extraction.
        let Some(line) = m.line else { continue };
        if line == 0 {
            continue;
        }
        let key = (m.file.clone(), line);
        if !seen.insert(key) {
            continue;
        }

        let Some(content) = read_source(snapshot, &m.file) else {
            continue;
        };
        let lines: Vec<&str> = content.lines().collect();
        let start_idx = line - 1;
        if start_idx >= lines.len() {
            continue;
        }

        let language = language_of(&m.file);
        let extracted = extract_body(&lines, start_idx, cap, &language);

        bodies.push(SymbolBody {
            symbol: symbol.to_string(),
            file: m.file.clone(),
            start_line: line,
            end_line: extracted.end_line,
            language,
            source: extracted.source,
            truncated: extracted.truncated,
            total_lines: extracted.total_lines,
            line_cap: cap,
            extent: extracted.extent.to_string(),
        });
    }

    BodyResult {
        symbol: symbol.to_string(),
        bodies,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_language_of() {
        assert_eq!(language_of("src/foo.rs"), "rs");
        assert_eq!(language_of("a/b/Thing.TSX"), "tsx");
        assert_eq!(language_of("Makefile"), "unknown");
    }

    #[test]
    fn test_extract_brace_balanced() {
        let src =
            "fn outer() {\n    let x = 1;\n    if x > 0 {\n        return;\n    }\n}\ntrailing";
        let lines: Vec<&str> = src.lines().collect();
        let b = extract_body(&lines, 0, 200, "rs");
        assert_eq!(b.end_line, 6, "closing brace is on line 6");
        assert!(b.source.contains("fn outer()"));
        assert!(b.source.ends_with("}"));
        assert!(!b.source.contains("trailing"));
        assert!(!b.truncated);
        assert_eq!(b.total_lines, 6);
        assert_eq!(b.extent, EXTENT_BRACE);
    }

    #[test]
    fn test_extract_respects_cap() {
        let src = "fn big() {\n  a;\n  b;\n  c;\n  d;\n}";
        let lines: Vec<&str> = src.lines().collect();
        let b = extract_body(&lines, 0, 3, "rs");
        assert!(b.truncated);
        assert_eq!(b.total_lines, 6);
        assert_eq!(b.source.lines().count(), 3);
    }

    #[test]
    fn test_extract_stops_at_boundary_despite_char_escape_literal() {
        // Regression: `'\\'` used to leave the scanner permanently
        // "in string", swallowing every brace after it and overshooting
        // into sibling methods (loct body resolve_file_in_snapshot bug).
        let src = "    fn normalize(&self, raw: &str) -> String {\n        raw.replace('\\\\', \"/\").to_string()\n    }\n\n    fn sibling(&self) {\n        println!(\"sibling\");\n    }";
        let lines: Vec<&str> = src.lines().collect();
        let b = extract_body(&lines, 0, 200, "rs");
        assert_eq!(b.end_line, 3, "body must close at the method's own brace");
        assert_eq!(b.total_lines, 3);
        assert!(!b.truncated);
        assert!(
            !b.source.contains("sibling"),
            "must not overshoot into sibling"
        );
    }

    #[test]
    fn test_extract_rust_lifetime_and_label_not_string_openers() {
        let src = "fn pick<'a>(&'a self, raw: &'a str) -> &'a str {\n    'outer: loop {\n        break 'outer;\n    }\n    raw\n}\nfn after() {}";
        let lines: Vec<&str> = src.lines().collect();
        let b = extract_body(&lines, 0, 200, "rs");
        assert_eq!(b.end_line, 6, "lifetimes/labels must not derail brace scan");
        assert_eq!(b.total_lines, 6);
        assert!(!b.truncated);
        assert!(!b.source.contains("fn after"));
    }

    #[test]
    fn test_extract_ignores_braces_in_line_comments() {
        let src = "fn doc() {\n    // unmatched { in a comment\n    let x = 1;\n}\nfn next() {}";
        let lines: Vec<&str> = src.lines().collect();
        let b = extract_body(&lines, 0, 200, "rs");
        assert_eq!(b.end_line, 4);
        assert!(!b.source.contains("fn next"));
    }

    #[test]
    fn test_extract_js_single_quote_string_still_shields_braces() {
        let src = "function f() {\n  const s = '}';\n  return s;\n}\nconst after = 1;";
        let lines: Vec<&str> = src.lines().collect();
        let b = extract_body(&lines, 0, 200, "js");
        assert_eq!(
            b.end_line, 4,
            "JS '}}' string literal must not close the fn"
        );
        assert!(!b.source.contains("after"));
    }

    #[test]
    fn test_extract_balances_assignment_collection_not_fixed_window() {
        // Hak (loctree-feedback.md, 2026-06-15): a module-level tuple/list/dict const
        // has no `{` fn-body brace, so it fell into the fixed 40-line window and
        // over-captured trailing code. An assignment that opens `(`/`[`/`{` should
        // balance that bracket and stop at its close.
        let src =
            "FRAMEWORK_LAUNCHER_MARKERS = (\n    \"a\",\n    \"b\",\n)\n\nOTHER = 1\nmore = 2";
        let lines: Vec<&str> = src.lines().collect();
        let b = extract_body(&lines, 0, 200, "py");
        assert_eq!(b.end_line, 4, "tuple closes on line 4 (the `)`)");
        assert_eq!(b.total_lines, 4);
        assert!(!b.truncated);
        assert!(b.source.contains("FRAMEWORK_LAUNCHER_MARKERS"));
        assert!(b.source.trim_end().ends_with(')'));
        assert!(
            !b.source.contains("OTHER"),
            "must not overshoot past the tuple"
        );
        assert_eq!(b.extent, EXTENT_BRACKET);
    }

    #[test]
    fn test_extract_def_paren_is_not_treated_as_assignment_collection() {
        // Guard: a `def f(...):` signature paren must NOT trigger bracket
        // balancing (it is not an assignment); the indent extent owns defs.
        let src = "def thing(a, b):\n    return a + b\n    # trailing";
        let lines: Vec<&str> = src.lines().collect();
        let b = extract_body(&lines, 0, 200, "py");
        assert!(b.source.contains("def thing(a, b):"));
        assert!(
            b.source.contains("return a + b"),
            "def body must still be captured, not just the signature"
        );
        assert_eq!(b.extent, EXTENT_INDENT);
        assert!(!b.truncated, "a closed indent block is not truncated");
    }

    // ── AST-extent controls (audit class B / MATRIX LCT-B*) ──────────────

    #[test]
    fn test_extract_python_def_indent_extent_excludes_siblings() {
        let src =
            "def first():\n    x = 1\n    if x:\n        return x\n\ndef second():\n    return 2";
        let lines: Vec<&str> = src.lines().collect();
        let b = extract_body(&lines, 0, 200, "py");
        assert_eq!(b.end_line, 4, "block ends at its last statement line");
        assert_eq!(b.total_lines, 4);
        assert_eq!(b.extent, EXTENT_INDENT);
        assert!(
            !b.truncated,
            "closed indent block must not claim truncation"
        );
        assert!(b.source.contains("return x"), "closing boundary included");
        assert!(!b.source.contains("second"), "must not leak into sibling");
    }

    #[test]
    fn test_extract_python_class_block_extent() {
        let src = "class Thing:\n    A = 1\n\n    def method(self):\n        return self.A\n\nTRAILING = 1";
        let lines: Vec<&str> = src.lines().collect();
        let b = extract_body(&lines, 0, 200, "py");
        assert_eq!(b.end_line, 5, "class body ends at the method's return");
        assert_eq!(b.extent, EXTENT_INDENT);
        assert!(!b.truncated);
        assert!(b.source.contains("return self.A"));
        assert!(!b.source.contains("TRAILING"));
    }

    #[test]
    fn test_extract_python_multiline_signature_def() {
        let src = "def build(\n    a,\n    b,\n):\n    return a + b\n\ndef other():\n    pass";
        let lines: Vec<&str> = src.lines().collect();
        let b = extract_body(&lines, 0, 200, "py");
        assert_eq!(b.end_line, 5, "body ends after the return, not the `):`");
        assert_eq!(b.extent, EXTENT_INDENT);
        assert!(!b.truncated);
        assert!(b.source.contains("return a + b"));
        assert!(!b.source.contains("other"));
    }

    #[test]
    fn test_extract_python_async_def_trims_trailing_comment_and_blank() {
        let src = "async def run():\n    await task()\n    # done\n\nafter = 1";
        let lines: Vec<&str> = src.lines().collect();
        let b = extract_body(&lines, 0, 200, "py");
        assert_eq!(b.end_line, 2, "block ends at last statement, ast-style");
        assert_eq!(b.extent, EXTENT_INDENT);
        assert!(!b.truncated);
        assert!(b.source.contains("await task()"));
        assert!(!b.source.contains("after"));
    }

    #[test]
    fn test_extract_python_def_default_tuple_param_stays_indent() {
        // A default arg `x=(1, 2)` must not be mistaken for an assignment
        // collection (which would cut the body at the signature paren).
        let src = "def f(x=(1, 2)):\n    return x\n\ny = 3";
        let lines: Vec<&str> = src.lines().collect();
        let b = extract_body(&lines, 0, 200, "py");
        assert_eq!(b.end_line, 2);
        assert_eq!(b.extent, EXTENT_INDENT);
        assert!(b.source.contains("return x"));
        assert!(!b.source.contains("y = 3"));
    }

    #[test]
    fn test_extract_unclosed_brace_is_window_and_truncated() {
        // Non-closure must never be a polished `truncated: false` zero.
        let src = "fn broken() {\n    let x = 1;";
        let lines: Vec<&str> = src.lines().collect();
        let b = extract_body(&lines, 0, 200, "rs");
        assert_eq!(b.extent, EXTENT_WINDOW);
        assert!(
            b.truncated,
            "an unbalanced brace body is incomplete and must say so"
        );
    }

    #[test]
    fn test_extract_unknown_language_window_is_truncated() {
        let src = "target: dep\n\tcommand\nother: dep2";
        let lines: Vec<&str> = src.lines().collect();
        let b = extract_body(&lines, 0, 200, "unknown");
        assert_eq!(b.extent, EXTENT_WINDOW);
        assert!(b.truncated, "window fallback is an unproven extent");
    }

    #[test]
    fn test_extract_rust_single_line_const_does_not_overshoot() {
        // Regression: a `;`-terminated const used to hit the brace lookahead,
        // balance the NEXT item's brace, and claim const + neighbor as one
        // closed body.
        let src = "pub const CAP: usize = 200;\n\npub fn later() {\n    body();\n}";
        let lines: Vec<&str> = src.lines().collect();
        let b = extract_body(&lines, 0, 200, "rs");
        assert_eq!(b.end_line, 1);
        assert_eq!(b.total_lines, 1);
        assert_eq!(b.extent, EXTENT_LINE);
        assert!(!b.truncated);
        assert!(!b.source.contains("later"));
    }

    #[test]
    fn test_extract_python_single_line_assignment_does_not_overshoot() {
        let src = "X = 1\n\ndef f():\n    d = {\"a\": 1}";
        let lines: Vec<&str> = src.lines().collect();
        let b = extract_body(&lines, 0, 200, "py");
        assert_eq!(b.end_line, 1);
        assert_eq!(b.extent, EXTENT_LINE);
        assert!(!b.truncated);
        assert!(!b.source.contains("def f"));
    }

    fn body_in(file: &str) -> SymbolBody {
        SymbolBody {
            symbol: "f".into(),
            file: file.into(),
            start_line: 1,
            end_line: 1,
            language: "rs".into(),
            source: "fn f() {}".into(),
            truncated: false,
            total_lines: 1,
            line_cap: DEFAULT_BODY_LINE_CAP,
            extent: EXTENT_BRACE.into(),
        }
    }

    #[test]
    fn test_filtered_to_file_matches_path_component_suffix() {
        let result = BodyResult {
            symbol: "f".into(),
            bodies: vec![
                body_in("crate/src/a.rs"),
                body_in("crate/src/b.rs"),
                body_in("crate/miscsrc/a.rs"),
            ],
        };
        let filtered = result.filtered_to_file(Some("src/a.rs"));
        assert_eq!(filtered.bodies.len(), 1, "suffix must respect path bounds");
        assert_eq!(filtered.bodies[0].file, "crate/src/a.rs");
    }

    #[test]
    fn test_filtered_to_file_none_keeps_all_candidates() {
        let result = BodyResult {
            symbol: "f".into(),
            bodies: vec![body_in("a.rs"), body_in("b.rs")],
        };
        assert_eq!(result.filtered_to_file(None).bodies.len(), 2);
    }

    #[test]
    fn test_read_source_prefers_snapshot_root_over_server_cwd_collision() {
        let tmp = tempfile::tempdir().expect("temp dir");
        std::fs::create_dir_all(tmp.path().join("src")).expect("create src");
        std::fs::write(
            tmp.path().join("src/lib.rs"),
            "pub fn from_requested_project() {}\n",
        )
        .expect("write requested project source");
        let snapshot = Snapshot::new(vec![tmp.path().to_string_lossy().to_string()]);

        let source = read_source(&snapshot, "src/lib.rs").expect("read rooted source");
        assert!(source.contains("from_requested_project"));
        assert!(
            !source.contains("pub mod auth"),
            "server cwd must not shadow the snapshot root"
        );
    }

    #[test]
    fn test_query_symbol_body_resolves_python_module_const() {
        let tmp = tempfile::tempdir().expect("temp dir");
        let source_path = tmp.path().join("markers.py");
        std::fs::write(
            &source_path,
            "FRAMEWORK_LAUNCHER_MARKERS = (\n    \"vibecrafted\",\n    \"loctree\",\n)\n",
        )
        .expect("write source");

        let mut snapshot = Snapshot::new(vec![tmp.path().to_string_lossy().to_string()]);
        let mut file = crate::types::FileAnalysis::new(source_path.to_string_lossy().to_string());
        file.local_symbols.push(crate::types::LocalSymbol {
            name: "FRAMEWORK_LAUNCHER_MARKERS".to_string(),
            kind: "const".to_string(),
            line: Some(1),
            context: "FRAMEWORK_LAUNCHER_MARKERS = (".to_string(),
            is_exported: false,
        });
        snapshot.files.push(file);

        let result = query_symbol_body(&snapshot, "FRAMEWORK_LAUNCHER_MARKERS", None);
        assert_eq!(result.bodies.len(), 1, "module const must resolve a body");
        assert!(
            result.bodies[0]
                .source
                .contains("FRAMEWORK_LAUNCHER_MARKERS")
        );
        assert!(result.bodies[0].source.contains("loctree"));
        assert_eq!(
            result.bodies[0].end_line, 4,
            "body bounded to the tuple close"
        );
        assert_eq!(result.bodies[0].extent, EXTENT_BRACKET);
        assert!(!result.bodies[0].truncated);
    }

    #[test]
    fn test_query_symbol_body_same_name_qualifiable_by_file() {
        // Audit class B: same-named functions in different files must come
        // back as distinct candidates, each selectable via file qualification.
        let tmp = tempfile::tempdir().expect("temp dir");
        let mut snapshot = Snapshot::new(vec![tmp.path().to_string_lossy().to_string()]);
        for (name, ret) in [("alpha.py", "1"), ("beta.py", "2")] {
            let source_path = tmp.path().join(name);
            std::fs::write(&source_path, format!("def build():\n    return {ret}\n"))
                .expect("write source");
            let mut file =
                crate::types::FileAnalysis::new(source_path.to_string_lossy().to_string());
            file.local_symbols.push(crate::types::LocalSymbol {
                name: "build".to_string(),
                kind: "function".to_string(),
                line: Some(1),
                context: "def build():".to_string(),
                is_exported: false,
            });
            snapshot.files.push(file);
        }

        let result = query_symbol_body(&snapshot, "build", None);
        assert_eq!(result.bodies.len(), 2, "both candidates must be returned");
        for body in &result.bodies {
            assert_eq!(body.extent, EXTENT_INDENT);
            assert!(!body.truncated);
        }

        let qualified =
            query_symbol_body(&snapshot, "build", None).filtered_to_file(Some("beta.py"));
        assert_eq!(
            qualified.bodies.len(),
            1,
            "file qualification must select one"
        );
        assert!(qualified.bodies[0].file.ends_with("beta.py"));
        assert!(qualified.bodies[0].source.contains("return 2"));
    }

    #[test]
    fn test_query_symbol_body_resolves_rust_impl_method() {
        let tmp = tempfile::tempdir().expect("temp dir");
        let source_path = tmp.path().join("recorder.rs");
        std::fs::write(
            &source_path,
            "struct Recorder;\n\nimpl Recorder {\n    pub fn start(&self) {\n        println!(\"start\");\n    }\n}\n",
        )
        .expect("write source");

        let mut snapshot = Snapshot::new(vec![tmp.path().to_string_lossy().to_string()]);
        let mut file = crate::types::FileAnalysis::new(source_path.to_string_lossy().to_string());
        file.impl_methods.push(crate::types::ImplMethod {
            name: "start".to_string(),
            qualifier: "Recorder".to_string(),
            line: Some(4),
            visibility: crate::types::Visibility::Public,
            ..Default::default()
        });
        snapshot.files.push(file);

        let result = query_symbol_body(&snapshot, "Recorder::start", None);
        assert_eq!(result.bodies.len(), 1);
        assert!(result.bodies[0].source.contains("pub fn start(&self)"));
        assert!(result.bodies[0].source.contains("println!"));
    }
}
