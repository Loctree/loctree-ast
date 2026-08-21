//! Narrow tree-sitter substrate for Loctree live-AST and structural query paths.
//!
//! This crate intentionally runs alongside the existing analyzer stack. It does
//! not replace OXC or the cold-scan extractors; it gives LSP/runtime code a
//! typed parser boundary that can grow without disturbing snapshot generation.

use std::path::Path;

pub mod extractors;

pub use extractors::{
    CallEntry, ExportSymbol as ExtractedExport, ImportBinding, ImportEntry as ExtractedImport,
    JsExtractor, LangExtractor, PyExtractor, TsExtractor,
};
pub use tree_sitter::{
    InputEdit, Language, Parser, Point, Query, QueryCursor, StreamingIterator, Tree,
};

/// Parsed source plus the tree-sitter tree and Loctree language id.
pub struct LoctreeTree {
    pub tree: Tree,
    pub source: Vec<u8>,
    pub lang: &'static str,
}

/// Cheap accessors over the parsed tree, so callers can judge a parse before
/// running queries against it.
impl LoctreeTree {
    /// Returns the grammar's node kind for the tree root, identifying the
    /// document shape the parse actually produced.
    pub fn root_kind(&self) -> &str {
        self.tree.root_node().kind()
    }

    /// Reports whether tree-sitter had to error-recover anywhere in the file,
    /// letting callers keep a stale tree instead of publishing a broken one.
    pub fn has_error(&self) -> bool {
        self.tree.root_node().has_error()
    }
}

/// Object-safe parser metadata used by the registry.
pub trait LangParser: Send + Sync {
    /// Hands back the compiled tree-sitter grammar to load into a `Parser`.
    fn language(&self) -> Language;
    /// Loctree's canonical language id (`"typescript"`, `"python"`, ...). Used as
    /// the registry lookup key and stamped onto every `LoctreeTree` this parser produces.
    fn lang_id(&self) -> &'static str;
    /// File extensions this parser claims, which is what drives
    /// `Parsers::for_path` dispatch.
    fn extensions(&self) -> &'static [&'static str];
}

/// Everything that can go wrong on the parse path: an unclaimed language, a
/// grammar tree-sitter refuses to load, or a parse that returns no tree.
#[derive(Debug, thiserror::Error)]
pub enum AstError {
    #[error("unsupported AST language: {0}")]
    UnsupportedLanguage(String),
    #[error("tree-sitter rejected {lang} grammar: {source}")]
    Language {
        lang: &'static str,
        source: tree_sitter::LanguageError,
    },
    #[error("tree-sitter could not parse {lang} source")]
    ParseFailed { lang: &'static str },
}

/// Registry of the grammars compiled into this crate. Owns the boxed
/// `LangParser`s and is the only supported way to turn bytes into a `LoctreeTree`.
pub struct Parsers {
    parsers: Vec<Box<dyn LangParser>>,
}

impl Default for Parsers {
    fn default() -> Self {
        Self::new_default()
    }
}

/// Lookup and parse surface consumed by `loctree-lsp` (live AST + AST queries)
/// and by the tree-sitter branch of the `loctree-rs` cold scan.
impl Parsers {
    /// Builds the registry with the four grammars this crate links:
    /// JavaScript, Python, TypeScript and TSX.
    pub fn new_default() -> Self {
        Self {
            parsers: vec![
                Box::new(JavaScriptParser),
                Box::new(PythonParser),
                Box::new(TypeScriptParser),
                Box::new(TsxParser),
            ],
        }
    }

    /// Lists the registered language ids in registration order.
    pub fn language_ids(&self) -> Vec<&'static str> {
        self.parsers.iter().map(|parser| parser.lang_id()).collect()
    }

    /// Resolves a language id to its parser, normalizing short aliases first so
    /// callers may pass `js` / `py` / `ts` instead of the canonical names.
    pub fn lookup(&self, lang_id: &str) -> Option<&dyn LangParser> {
        let normalized = normalize_lang_id(lang_id);
        self.parsers
            .iter()
            .find(|parser| parser.lang_id() == normalized)
            .map(|parser| parser.as_ref())
    }

    /// Picks a parser from the path's lowercased file extension; `None` when no
    /// registered grammar claims that extension.
    pub fn for_path(&self, path: &Path) -> Option<&dyn LangParser> {
        let ext = path.extension()?.to_str()?.to_ascii_lowercase();
        self.parsers
            .iter()
            .find(|parser| parser.extensions().contains(&ext.as_str()))
            .map(|parser| parser.as_ref())
    }

    /// Parses `source` from scratch under the given grammar, returning the tree
    /// together with an owned copy of the bytes its ranges point into.
    pub fn parse(&self, lang: &dyn LangParser, source: &[u8]) -> Result<LoctreeTree, AstError> {
        let mut parser = Parser::new();
        let language = lang.language();
        parser
            .set_language(&language)
            .map_err(|source| AstError::Language {
                lang: lang.lang_id(),
                source,
            })?;
        let tree = parser.parse(source, None).ok_or(AstError::ParseFailed {
            lang: lang.lang_id(),
        })?;
        Ok(LoctreeTree {
            tree,
            source: source.to_vec(),
            lang: lang.lang_id(),
        })
    }

    /// Parses a file by resolving the grammar from its path extension, reporting
    /// the offending extension when nothing claims it.
    pub fn parse_path(&self, path: &Path, source: &[u8]) -> Result<LoctreeTree, AstError> {
        let lang = self.for_path(path).ok_or_else(|| {
            AstError::UnsupportedLanguage(path.extension_id().unwrap_or("unknown").to_string())
        })?;
        self.parse(lang, source)
    }

    /// Parses `source` under an explicitly named language id rather than a path.
    pub fn parse_language(&self, lang_id: &str, source: &[u8]) -> Result<LoctreeTree, AstError> {
        let lang = self
            .lookup(lang_id)
            .ok_or_else(|| AstError::UnsupportedLanguage(lang_id.to_string()))?;
        self.parse(lang, source)
    }

    /// Re-parses an edited buffer by replaying `edits` onto the previous tree and
    /// feeding it back to tree-sitter, so only touched ranges are re-scanned.
    /// This is the keystroke path behind the LSP's live AST.
    pub fn parse_incremental(
        &self,
        prev: &LoctreeTree,
        new_source: &[u8],
        edits: &[InputEdit],
    ) -> Result<LoctreeTree, AstError> {
        let lang = self
            .lookup(prev.lang)
            .ok_or_else(|| AstError::UnsupportedLanguage(prev.lang.to_string()))?;
        let mut old_tree = prev.tree.clone();
        for edit in edits {
            old_tree.edit(edit);
        }

        let mut parser = Parser::new();
        let language = lang.language();
        parser
            .set_language(&language)
            .map_err(|source| AstError::Language {
                lang: lang.lang_id(),
                source,
            })?;
        let tree = parser
            .parse(new_source, Some(&old_tree))
            .ok_or(AstError::ParseFailed {
                lang: lang.lang_id(),
            })?;

        Ok(LoctreeTree {
            tree,
            source: new_source.to_vec(),
            lang: lang.lang_id(),
        })
    }
}

/// Registry entry binding the `tree-sitter-javascript` grammar.
struct JavaScriptParser;
/// Registry entry binding the `tree-sitter-python` grammar.
struct PythonParser;
/// Registry entry binding the TypeScript dialect of `tree-sitter-typescript`.
struct TypeScriptParser;
/// Registry entry binding the TSX dialect of `tree-sitter-typescript`.
struct TsxParser;

impl LangParser for JavaScriptParser {
    fn language(&self) -> Language {
        tree_sitter_javascript::LANGUAGE.into()
    }

    fn lang_id(&self) -> &'static str {
        "javascript"
    }

    fn extensions(&self) -> &'static [&'static str] {
        &["js", "cjs", "mjs", "jsx"]
    }
}

impl LangParser for PythonParser {
    fn language(&self) -> Language {
        tree_sitter_python::LANGUAGE.into()
    }

    fn lang_id(&self) -> &'static str {
        "python"
    }

    fn extensions(&self) -> &'static [&'static str] {
        &["py", "pyi"]
    }
}

impl LangParser for TypeScriptParser {
    fn language(&self) -> Language {
        tree_sitter_typescript::LANGUAGE_TYPESCRIPT.into()
    }

    fn lang_id(&self) -> &'static str {
        "typescript"
    }

    fn extensions(&self) -> &'static [&'static str] {
        &["ts", "cts", "mts"]
    }
}

impl LangParser for TsxParser {
    fn language(&self) -> Language {
        tree_sitter_typescript::LANGUAGE_TSX.into()
    }

    fn lang_id(&self) -> &'static str {
        "tsx"
    }

    fn extensions(&self) -> &'static [&'static str] {
        &["tsx"]
    }
}

/// Folds short aliases onto canonical language ids so `lookup` accepts `js`,
/// `jsx`, `node`, `py` and `ts`; anything else passes through untouched.
fn normalize_lang_id(lang_id: &str) -> &str {
    match lang_id {
        "js" | "jsx" | "node" => "javascript",
        "py" => "python",
        "ts" => "typescript",
        other => other,
    }
}

/// Internal helper: borrow a path's extension as `&str` without allocating.
/// Exists only so `parse_path` can name the offending extension in `AstError`.
trait PathExtensionId {
    /// Returns the extension as UTF-8, or `None` when absent or not valid UTF-8.
    fn extension_id(&self) -> Option<&str>;
}

impl PathExtensionId for Path {
    fn extension_id(&self) -> Option<&str> {
        self.extension().and_then(|ext| ext.to_str())
    }
}
