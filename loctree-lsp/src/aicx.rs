//! Custom LSP request: `loctree/aicx` (Plan 08).
//!
//! Read-only AICX memory continuity for LSP-connected agents. Reuses
//! the canonical Loctree memory composer
//! ([`loctree::pack::compose_memory_slice`]) so the wire shape stays
//! aligned with the CLI's `loct context --with-aicx` and the MCP
//! server. No write side — agents that record new intents go through
//! their own AICX integration, not this endpoint.
//!
//! ## Contract
//!
//! - Params: `scope`, `target`, `symbol_id`, `kinds`, `hours`, `limit`, `project`.
//! - Routes through Plan 13's workspace map via `params.project`.
//! - When AICX is unreachable, returns
//!   `{status: "aicx_unavailable", hint: "..."}` rather than a
//!   `ServerError`. Agents probe `loctree/aicx` for capability without
//!   risking a fatal LSP response.
//! - `kinds` filter (default = all): `decision`, `intent`, `outcome`,
//!   `task`, `failure`. The wire shape mirrors AICX's intent kinds 1:1.
//!
//! 𝚅𝚒𝚋𝚎𝚌𝚛𝚊𝚏𝚝𝚎𝚍. with AI Agents by Vetcoders ⓒ 2025-2026 Vetcoders

use std::collections::HashSet;
use std::path::PathBuf;

use loctree::aicx::{AicxClient, is_aicx_available};
use loctree::pack::{
    AuthorityLabel, ContextOptions, MemoryEntry, aicx_project_bucket, compose_memory_slice,
    compose_runtime_slice, compose_structural_slice,
};
use loctree::snapshot::Snapshot;
use loctree::types::SymbolIdV1;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

/// Default look-back window when the caller does not specify one.
/// Mirrors the CLI default — 30 days is wide enough to capture a
/// sprint, narrow enough to keep AICX retrieval cheap.
pub const DEFAULT_HOURS: u64 = 720;

/// Default per-call limit. Same constant the CLI uses.
pub const DEFAULT_LIMIT: usize = 50;

/// Cap on `limit` even when the caller asks for more — keeps the
/// response inside the host JSON-RPC payload limit.
pub const MAX_LIMIT: usize = 200;

/// Request params for `loctree/aicx`.
#[derive(Debug, Clone, Deserialize, Serialize, Default, JsonSchema)]
pub struct AicxParams {
    /// `"file"`, `"symbol"`, or `"project"`.
    pub scope: String,
    /// Target path (file scope) or symbol id (symbol scope).
    #[serde(default)]
    pub target: Option<String>,
    /// Stable typed symbol anchor for `scope = "symbol"`.
    ///
    /// `SymbolIdV1` wraps the canonical `<file_path>::<symbol_name>`
    /// form. Mirrors `loctree/find` and matches the capability
    /// metadata's `symbol_id_version` advertisement. When present, the
    /// response echoes it back so agents can correlate AICX memory
    /// retrieval with `loctree/find` and paginated follow-up calls.
    /// `target` remains accepted for back-compatible v1 callers;
    /// `symbol_id` is the structured echo path.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub symbol_id: Option<SymbolIdV1>,
    /// Filter — subset of `decision`, `intent`, `outcome`, `task`,
    /// `failure`. `None` = all kinds.
    #[serde(default)]
    pub kinds: Option<Vec<String>>,
    /// Look-back window. Defaults to [`DEFAULT_HOURS`].
    #[serde(default)]
    pub hours: Option<u64>,
    /// Result cap. Clamped to [`MAX_LIMIT`]. Defaults to [`DEFAULT_LIMIT`].
    #[serde(default)]
    pub limit: Option<usize>,
    /// Plan 13 multi-workspace routing override.
    #[serde(default)]
    pub project: Option<PathBuf>,
}

/// One AICX entry in the wire response.
#[derive(Debug, Clone, Serialize)]
pub struct AicxEntry {
    pub kind: String,
    pub text: String,
    pub authority: AuthorityLabel,
    pub source_chunk: String,
    pub agent: String,
    pub date: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub timestamp: Option<String>,
    pub session_id: String,
    pub project: String,
    pub relevance: u32,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub retrieval_score: Option<i64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub retrieval_label: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub retrieval_mode: Option<String>,
    #[serde(default, skip_serializing_if = "is_false")]
    pub low_lexical_match: bool,
}

fn is_false(b: &bool) -> bool {
    !*b
}

/// Wire shape for `loctree/aicx`.
#[derive(Debug, Clone, Serialize)]
pub struct AicxResponse {
    /// `"ok"` (data populated) or `"aicx_unavailable"` (binary missing
    /// or library facade rejected).
    pub status: String,
    /// Free-form hint — populated for non-`ok` statuses.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub hint: Option<String>,
    /// AICX project bucket the daemon queried.
    pub namespace: String,
    /// Echo of the resolved scope.
    pub scope: String,
    /// Echo of the v1 [`SymbolIdV1`] supplied in the request, if any.
    ///
    /// Keeps the request/response shape aligned with the advertised
    /// `symbol_id_version` capability so agents can pin AICX symbol
    /// lookups without re-deriving the anchor from `target`.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub symbol_id: Option<SymbolIdV1>,
    /// Memory entries — empty when `status != "ok"`.
    pub entries: Vec<AicxEntry>,
    /// Deduplicated list of source chunk paths referenced by `entries`.
    pub source_chunks: Vec<String>,
    /// Diagnostic from `compose_memory_slice` — surfaced as a free-form
    /// hint when the slice came back empty so agents can distinguish
    /// "no rows" from "AICX unreachable" without parsing strings.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub skip_reason: Option<String>,
    /// Wire version of the symbol-id contract in this response.
    pub symbol_id_version: &'static str,
}

/// Catalog of valid `kinds` filter values.
pub const ALL_KINDS: &[&str] = &["decision", "intent", "outcome", "task", "failure"];

/// Build a [`ContextOptions`] for the LSP request. Only `file`,
/// `project`, and `with_aicx` are populated — output flags
/// (json/markdown) belong to the CLI.
pub fn options_for_request(params: &AicxParams) -> ContextOptions {
    let file = params.target.as_ref().map(PathBuf::from);
    ContextOptions {
        file,
        project: params.project.clone(),
        with_aicx: true,
        ..ContextOptions::default()
    }
}

/// Filter a `MemoryEntry` by the requested `kinds`. AICX intent kinds
/// are `decision` / `intent` / `outcome` / `task`; we additionally
/// expose `failure` as the `AicxFailure` authority projection so
/// agents can ask for "rolled-back paths" specifically.
fn entry_matches_kinds(entry: &MemoryEntry, allowed: &HashSet<&str>) -> bool {
    if allowed.is_empty() {
        return true;
    }
    if allowed.contains(entry.kind.as_str()) {
        return true;
    }
    if allowed.contains("failure") && entry.authority == AuthorityLabel::AicxFailure {
        return true;
    }
    false
}

/// Project a [`MemoryEntry`] (canonical pack shape) onto the wire
/// shape ([`AicxEntry`]). Kept separate from the filter so tests can
/// assert the projection contract independent of the filter logic.
pub fn project_entry(entry: &MemoryEntry) -> AicxEntry {
    AicxEntry {
        kind: entry.kind.clone(),
        text: entry.text.clone(),
        authority: entry.authority,
        source_chunk: entry.source_chunk.clone(),
        agent: entry.agent.clone(),
        date: entry.date.clone(),
        timestamp: entry.timestamp.clone(),
        session_id: entry.session_id.clone(),
        project: entry.project.clone(),
        relevance: entry.relevance,
        retrieval_score: entry.retrieval_score,
        retrieval_label: entry.retrieval_label.clone(),
        retrieval_mode: entry.retrieval_mode.clone(),
        low_lexical_match: entry.low_lexical_match,
    }
}

/// Build the `aicx_unavailable` response. Used when AICX is missing
/// at request time — never fails the LSP response, just reports the
/// limitation so the agent can fall back to its own resolver.
pub fn unavailable_response(namespace: String, scope: String) -> AicxResponse {
    AicxResponse {
        status: "aicx_unavailable".to_string(),
        hint: Some(
            "AICX library/binary not reachable from this LSP daemon. \
             Install `aicx` on PATH or set LOCT_AICX_BINARY; the daemon \
             does not need a restart to pick it up."
                .to_string(),
        ),
        namespace,
        scope,
        symbol_id: None,
        entries: Vec::new(),
        source_chunks: Vec::new(),
        skip_reason: Some("aicx_unreachable".to_string()),
        symbol_id_version: SymbolIdV1::VERSION,
    }
}

/// Cap a caller-supplied limit at [`MAX_LIMIT`]. Default applied at
/// the call site so the wire contract stays explicit.
pub fn clamp_limit(requested: Option<usize>) -> usize {
    requested.unwrap_or(DEFAULT_LIMIT).clamp(1, MAX_LIMIT)
}

/// Apply caller's `kinds` filter to a slice of entries, building the
/// wire shape and the deduplicated `source_chunks` list along the way.
pub fn filter_and_project(
    entries: &[MemoryEntry],
    kinds: &Option<Vec<String>>,
) -> (Vec<AicxEntry>, Vec<String>) {
    let allowed: HashSet<&str> = match kinds {
        Some(list) if !list.is_empty() => list.iter().map(String::as_str).collect(),
        _ => HashSet::new(),
    };
    let mut wire: Vec<AicxEntry> = Vec::with_capacity(entries.len());
    let mut chunks: Vec<String> = Vec::new();
    let mut chunk_set: HashSet<String> = HashSet::new();
    for entry in entries {
        if !entry_matches_kinds(entry, &allowed) {
            continue;
        }
        if chunk_set.insert(entry.source_chunk.clone()) {
            chunks.push(entry.source_chunk.clone());
        }
        wire.push(project_entry(entry));
    }
    (wire, chunks)
}

/// Build the response for a fully-functional AICX environment.
///
/// Uses `compose_memory_slice` so the LSP path applies the same
/// scoring/dedup/recency-fallback logic the CLI uses. Agents see one
/// canonical retrieval contract regardless of which client they hit.
pub fn compute(
    snapshot: &Snapshot,
    params: &AicxParams,
    workspace_root: Option<&std::path::Path>,
) -> AicxResponse {
    if !is_aicx_available() {
        let opts = options_for_request(params);
        let namespace = aicx_project_bucket(&opts);
        let mut response = unavailable_response(namespace, params.scope.clone());
        response.symbol_id = params.symbol_id.clone();
        return response;
    }

    let mut opts = options_for_request(params);
    if opts.project.is_none()
        && let Some(root) = workspace_root
    {
        opts.project = Some(root.to_path_buf());
    }
    let namespace = aicx_project_bucket(&opts);

    let structural = compose_structural_slice(&opts, snapshot);
    let runtime = compose_runtime_slice(&opts, snapshot);

    let client = AicxClient::new(namespace.clone());
    // Per-request window and cap travel in the options, not through the
    // process environment. The previous env-guard approach was both a data
    // race (handlers run on arbitrary Tokio worker threads; `setenv` racing
    // `getenv` is undefined behaviour on POSIX) and a silent no-op for
    // `hours`: its guard was dropped at the end of the `if` block, before the
    // composer ever ran.
    opts.memory_hours = params.hours;
    opts.memory_limit = Some(clamp_limit(params.limit));

    let memory = compose_memory_slice(&opts, &structural, &runtime, Some(&client));

    let (entries, source_chunks) = filter_and_project(&memory.entries, &params.kinds);
    let skip_reason = memory.diagnostic.as_ref().map(|d| {
        // Project the typed enum back to a free-form string so the
        // wire shape stays JSON-friendly without leaking the internal
        // enum name verbatim. The composer already classifies the
        // skip reason; we just expose it as a label.
        format!("{:?}", d.skip_reason).to_lowercase()
    });

    AicxResponse {
        status: "ok".to_string(),
        hint: None,
        namespace,
        scope: params.scope.clone(),
        symbol_id: params.symbol_id.clone(),
        entries,
        source_chunks,
        skip_reason,
        symbol_id_version: SymbolIdV1::VERSION,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn entry(kind: &str, authority: AuthorityLabel) -> MemoryEntry {
        MemoryEntry {
            kind: kind.to_string(),
            text: "demo".to_string(),
            authority,
            source_chunk: format!("/tmp/{kind}.md"),
            agent: "claude".to_string(),
            date: "2026-05-07".to_string(),
            timestamp: None,
            session_id: "abc".to_string(),
            project: "demo".to_string(),
            relevance: 1,
            retrieval_score: None,
            retrieval_label: None,
            retrieval_mode: None,
            low_lexical_match: false,
        }
    }

    #[test]
    fn clamp_limit_applies_default_and_cap() {
        assert_eq!(clamp_limit(None), DEFAULT_LIMIT);
        assert_eq!(clamp_limit(Some(0)), 1);
        assert_eq!(clamp_limit(Some(usize::MAX)), MAX_LIMIT);
        assert_eq!(clamp_limit(Some(75)), 75);
    }

    #[test]
    fn filter_keeps_all_when_kinds_unset() {
        let entries = vec![
            entry("decision", AuthorityLabel::AicxOperator),
            entry("outcome", AuthorityLabel::AicxAgent),
        ];
        let (wire, chunks) = filter_and_project(&entries, &None);
        assert_eq!(wire.len(), 2);
        assert_eq!(chunks.len(), 2);
    }

    #[test]
    fn filter_drops_unmatched_kinds() {
        let entries = vec![
            entry("decision", AuthorityLabel::AicxOperator),
            entry("outcome", AuthorityLabel::AicxAgent),
        ];
        let (wire, _) = filter_and_project(&entries, &Some(vec!["decision".into()]));
        assert_eq!(wire.len(), 1);
        assert_eq!(wire[0].kind, "decision");
    }

    #[test]
    fn filter_failure_alias_matches_aicx_failure_authority() {
        let mut e = entry("outcome", AuthorityLabel::AicxFailure);
        e.text = "rolled back".to_string();
        let (wire, _) = filter_and_project(&[e], &Some(vec!["failure".into()]));
        assert_eq!(wire.len(), 1);
        assert_eq!(wire[0].authority, AuthorityLabel::AicxFailure);
    }

    #[test]
    fn unavailable_response_carries_hint() {
        let resp = unavailable_response("ns".to_string(), "file".to_string());
        assert_eq!(resp.status, "aicx_unavailable");
        assert!(resp.hint.is_some());
        assert_eq!(resp.namespace, "ns");
    }

    #[test]
    fn project_entry_preserves_low_lexical_match() {
        let mut e = entry("intent", AuthorityLabel::AicxOperator);
        e.low_lexical_match = true;
        let projected = project_entry(&e);
        assert!(projected.low_lexical_match);
    }
}
