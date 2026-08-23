//! Best-effort symbol extraction from a one-line source snippet.
//!
//! Used to turn the `context` string loctree carries alongside a match into a
//! bare symbol name. Regex-first, with a token fallback, so a snippet that is
//! not a function signature still yields something callers can display.
//!
//! Vibecrafted with AI Agents by Vetcoders (c)2024-2026 LibraxisAI

use regex::Regex;
use std::sync::OnceLock;

/// Pull the declared function name out of a one-line signature snippet.
/// Falls back to the last meaningful whitespace-separated token, and to the
/// whole input when nothing usable remains, so it never returns empty.
pub fn extract_symbol(context: &str) -> &str {
    static FN_RE: OnceLock<Regex> = OnceLock::new();
    let re = FN_RE.get_or_init(|| {
        Regex::new(r"(?:async\s+)?(?:pub(?:\([^)]*\))?\s+)?fn\s+(\w+)\s*[<(]").unwrap()
    });

    if let Some(captures) = re.captures(context)
        && let Some(m) = captures.get(1)
    {
        return m.as_str();
    }

    context
        .split_whitespace()
        .filter(|t| !["{", "}", "->", "Self", "=>", "=", ":", ";", ","].contains(t))
        .rfind(|t| t.len() > 1 || t.chars().all(char::is_alphanumeric))
        .unwrap_or(context)
}
