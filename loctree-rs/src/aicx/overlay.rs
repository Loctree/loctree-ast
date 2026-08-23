//! C0-01 consumer seam: `aicx overlay` transport, full-key cache, detached refresh.
//!
//! # Architecture (I1-01, substrate-makieta v4 plan B)
//!
//! The bare `loct context` render NEVER waits for the producer. It reads the
//! last validated overlay document from `.loctree/aicx-overlay.v1.json`,
//! checks freshness against locally-provable truth (snapshot commit, anchor
//! catalog revision, repo identity, schema version), and renders either:
//!
//! - fresh theses in the spec grammar (`✓[U] 2026-07-13 · <thesis> …`), or
//! - the same theses plus an explicit `intent layer stale (<reason>) —
//!   refresh: <command>` marker.
//!
//! When the cache is missing, stale, or older than the TTL, the CLI handler
//! spawns `aicx overlay --repo <root> --format json` in a DETACHED process
//! (see [`spawn_detached_refresh`]) whose output lands in the cache for the
//! next invocation. Appending a synchronous fetch before exit was measured
//! and rejected (v4): waiting before closing stdout still breaks the <2s
//! end-to-end budget.
//!
//! # Trust boundary
//!
//! Loctree does not reproduce the producer's private revision algorithm
//! (attribution/dedup/model/threshold). It trusts the signed
//! `store_revision` / `overlay_revision` as emitted and validates the pinned
//! contract shape instead. Producer-side revisions therefore participate in
//! the FULL cache key ([`OverlayKey`], [`changed_key_components`]) but only
//! locally-derivable components participate in render-time staleness
//! ([`staleness_reason`]); producer drift is converged by the detached
//! refresh + TTL, and surfaced via [`OverlayRenderState::key_transition`]
//! when a refresh actually changed the key.

use serde::{Deserialize, Serialize};
use std::fs;
use std::path::{Path, PathBuf};
use std::time::{Duration, SystemTime};

/// The only overlay contract version this consumer renders.
pub const OVERLAY_SCHEMA_VERSION: &str = "loctree.overlay.intent.v1";

/// Seconds a fresh-looking cache may age before a background refresh is
/// recommended anyway (producer-side revisions can move without any local
/// signal). `0` disables TTL-driven refreshes.
pub const OVERLAY_TTL_ENV: &str = "LOCT_AICX_OVERLAY_TTL_SECS";
const OVERLAY_TTL_DEFAULT: Duration = Duration::from_secs(300);

/// A refresh lock younger than this suppresses further spawns; older locks
/// are treated as leftovers from a crashed refresh and are overwritten.
const REFRESH_LOCK_TTL: Duration = Duration::from_secs(120);

const THESES_CAP: usize = 12;
const SCOPE_PATHS_CAP: usize = 5;

// ---------------------------------------------------------------------------
// Wire types — loctree.overlay.intent.v1 (docs/contracts/, C0-01)
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Deserialize)]
pub struct OverlayDoc {
    /// Optional self-describing tag; when present it MUST match
    /// [`OVERLAY_SCHEMA_VERSION`] or parsing fails with an explicit version
    /// error (never a silent drop).
    #[serde(default)]
    pub schema: Option<String>,
    pub repo_id: String,
    pub snapshot_commit: String,
    pub anchor_catalog_revision: String,
    pub store_revision: String,
    pub overlay_revision: String,
    pub producer_version: String,
    #[serde(default)]
    pub entries: Vec<OverlayEntry>,
    // `unresolved_attributions` is payload-only (Doctrine 7): candidates
    // below the confidence threshold must never reach a force-fed card, so
    // this consumer does not even deserialize them.
}

#[derive(Debug, Clone, Deserialize)]
pub struct OverlayEntry {
    pub intent_id: String,
    pub target: OverlayTarget,
    pub thesis: String,
    pub status: OverlayLifecycle,
    pub authority: OverlayAuthority,
    pub verification_status: OverlayVerification,
    pub valid_from: String,
    #[serde(default)]
    pub refs: Vec<OverlayRef>,
    // content_hash / relations / attributions / valid_to belong to the atlas
    // card body (M1-01), not the mandatory-read pill line.
}

#[derive(Debug, Clone, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum OverlayTarget {
    Repo,
    Path {
        path: String,
    },
    Symbol {
        #[serde(default)]
        path: Option<String>,
        qualified_symbol: String,
    },
}

/// Lifecycle marker — rendered as ✓ / ⊘ / ✗ per the contract.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum OverlayLifecycle {
    Current,
    Superseded,
    Disputed,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum OverlayAuthority {
    OperatorConfirmed,
    AgentDerived,
    Inferred,
}

/// Evidence marker, separate from lifecycle (spec v4) — rendered as [V]/[U]/[R].
#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum OverlayVerification {
    Verified,
    Unverified,
    Refuted,
}

#[derive(Debug, Clone, Deserialize)]
pub struct OverlayRef {
    pub evidence_event_id: String,
    #[serde(rename = "ref")]
    pub store_ref: String,
}

// ---------------------------------------------------------------------------
// Full cache key
// ---------------------------------------------------------------------------

/// The FULL cache key per the brief: every component carried explicitly for
/// diagnostics. Any single component changing means the cached document and
/// a fresh emission are different worlds — a cache miss, never a silent reuse.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct OverlayKey {
    pub repo_id: String,
    pub store_revision: String,
    pub overlay_revision: String,
    pub snapshot_commit: String,
    pub anchor_catalog_revision: String,
    pub schema_version: String,
}

impl OverlayDoc {
    pub fn key(&self) -> OverlayKey {
        OverlayKey {
            repo_id: self.repo_id.clone(),
            store_revision: self.store_revision.clone(),
            overlay_revision: self.overlay_revision.clone(),
            snapshot_commit: self.snapshot_commit.clone(),
            anchor_catalog_revision: self.anchor_catalog_revision.clone(),
            schema_version: self
                .schema
                .clone()
                .unwrap_or_else(|| OVERLAY_SCHEMA_VERSION.to_string()),
        }
    }
}

/// Names of key components that differ between two full keys. Empty = hit.
pub fn changed_key_components(cached: &OverlayKey, fresh: &OverlayKey) -> Vec<&'static str> {
    let mut changed = Vec::new();
    if cached.repo_id != fresh.repo_id {
        changed.push("repo_id");
    }
    if cached.store_revision != fresh.store_revision {
        changed.push("store_revision");
    }
    if cached.overlay_revision != fresh.overlay_revision {
        changed.push("overlay_revision");
    }
    if cached.snapshot_commit != fresh.snapshot_commit {
        changed.push("snapshot_commit");
    }
    if cached.anchor_catalog_revision != fresh.anchor_catalog_revision {
        changed.push("anchor_catalog_revision");
    }
    if cached.schema_version != fresh.schema_version {
        changed.push("schema_version");
    }
    changed
}

// ---------------------------------------------------------------------------
// Validation
// ---------------------------------------------------------------------------

#[derive(Debug)]
pub enum OverlayError {
    /// The producer returned success without emitting a document.
    EmptyDocument,
    /// The on-disk producer cache exists but contains no document.
    EmptyCache {
        path: PathBuf,
        producer_diagnostic: Option<String>,
    },
    /// The on-disk producer cache is non-empty but not valid JSON.
    MalformedCache {
        path: PathBuf,
        message: String,
    },
    /// The document declares a schema tag other than the pinned contract.
    SchemaVersion {
        found: String,
    },
    /// Structurally parseable JSON that violates the pinned contract shape.
    Contract(String),
    /// Not parseable as the contract document at all.
    Parse(String),
    Io(std::io::Error),
}

impl std::fmt::Display for OverlayError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            OverlayError::EmptyDocument => write!(
                f,
                "aicx overlay producer returned an empty response; run the refresh command directly and inspect its exit status and stderr"
            ),
            OverlayError::EmptyCache {
                path,
                producer_diagnostic,
            } => {
                write!(
                    f,
                    "aicx overlay cache `{}` is empty: the producer emitted no document; run `aicx overlay --repo <repo> --format json` directly and inspect stdout/stderr before retrying",
                    path.display()
                )?;
                if let Some(diagnostic) = producer_diagnostic {
                    write!(f, "; last producer diagnostic: {diagnostic}")?;
                }
                Ok(())
            }
            OverlayError::MalformedCache { path, message } => write!(
                f,
                "aicx overlay cache `{}` contains malformed JSON: {message}; remove or refresh this cache only after checking the producer output",
                path.display()
            ),
            OverlayError::SchemaVersion { found } => write!(
                f,
                "aicx overlay schema version mismatch: expected `{OVERLAY_SCHEMA_VERSION}`, got `{found}` — refusing to render (explicit version error, not a silent drop)"
            ),
            OverlayError::Contract(msg) => {
                write!(f, "aicx overlay violates {OVERLAY_SCHEMA_VERSION}: {msg}")
            }
            OverlayError::Parse(msg) => {
                write!(
                    f,
                    "aicx overlay is not valid {OVERLAY_SCHEMA_VERSION} JSON: {msg}"
                )
            }
            OverlayError::Io(err) => write!(f, "aicx overlay cache I/O error: {err}"),
        }
    }
}

impl std::error::Error for OverlayError {}

/// Parse + validate a raw producer emission against the pinned contract.
///
/// Validation is deliberately structural (typed serde + revision-prefix and
/// commit-shape checks), not a full JSON-Schema engine: the contract JSONs in
/// `docs/contracts/` stay the normative source, and `verify_contracts.py`
/// guards fixture/schema parity producer-side.
pub fn parse_overlay(raw: &str) -> Result<OverlayDoc, OverlayError> {
    if raw.trim().is_empty() {
        return Err(OverlayError::EmptyDocument);
    }

    // Surface a wrong schema tag as a version error even when the rest of
    // the document no longer parses under this consumer's typed shape.
    if let Ok(probe) = serde_json::from_str::<serde_json::Value>(raw)
        && let Some(found) = probe.get("schema").and_then(|s| s.as_str())
        && found != OVERLAY_SCHEMA_VERSION
    {
        return Err(OverlayError::SchemaVersion {
            found: found.to_string(),
        });
    }

    let doc: OverlayDoc =
        serde_json::from_str(raw).map_err(|err| OverlayError::Parse(err.to_string()))?;

    if !doc.store_revision.starts_with("sr1:") {
        return Err(OverlayError::Contract(format!(
            "store_revision must be `sr1:<sha256>`, got `{}`",
            doc.store_revision
        )));
    }
    if !doc.overlay_revision.starts_with("ov1:") {
        return Err(OverlayError::Contract(format!(
            "overlay_revision must be `ov1:<sha256>`, got `{}`",
            doc.overlay_revision
        )));
    }
    if !doc.anchor_catalog_revision.starts_with("acr1:") {
        return Err(OverlayError::Contract(format!(
            "anchor_catalog_revision must be `acr1:<sha256>`, got `{}`",
            doc.anchor_catalog_revision
        )));
    }
    let commit_ok = (7..=40).contains(&doc.snapshot_commit.len())
        && doc
            .snapshot_commit
            .chars()
            .all(|c| c.is_ascii_hexdigit() && !c.is_ascii_uppercase());
    if !commit_ok {
        return Err(OverlayError::Contract(format!(
            "snapshot_commit must be 7-40 lowercase hex, got `{}`",
            doc.snapshot_commit
        )));
    }
    Ok(doc)
}

// ---------------------------------------------------------------------------
// Cache
// ---------------------------------------------------------------------------

pub fn overlay_cache_path(root: &Path) -> PathBuf {
    root.join(".loctree").join("aicx-overlay.v1.json")
}

fn last_key_sidecar_path(root: &Path) -> PathBuf {
    root.join(".loctree").join("aicx-overlay.last-key.v1.json")
}

fn refresh_lock_path(root: &Path) -> PathBuf {
    root.join(".loctree").join("aicx-overlay.refresh.lock")
}

fn refresh_error_path(root: &Path) -> PathBuf {
    root.join(".loctree")
        .join("aicx-overlay.refresh-error.v1.txt")
}

#[derive(Debug)]
pub struct CachedOverlay {
    pub doc: OverlayDoc,
    /// Age of the cache file; `None` when the filesystem hides mtimes.
    pub age: Option<Duration>,
}

/// Load and validate the cached overlay. `Ok(None)` = no cache yet.
/// `Err` = a cache file exists but violates the contract — callers must
/// surface the error (the detached refresh will overwrite it).
pub fn load_cached_overlay(root: &Path) -> Result<Option<CachedOverlay>, OverlayError> {
    let path = overlay_cache_path(root);
    let raw = match fs::read_to_string(&path) {
        Ok(raw) => raw,
        Err(err) if err.kind() == std::io::ErrorKind::NotFound => return Ok(None),
        Err(err) => return Err(OverlayError::Io(err)),
    };
    if raw.trim().is_empty() {
        let producer_diagnostic = fs::read_to_string(refresh_error_path(root))
            .ok()
            .map(|diagnostic| diagnostic.trim().chars().take(4096).collect())
            .filter(|diagnostic: &String| !diagnostic.is_empty());
        return Err(OverlayError::EmptyCache {
            path,
            producer_diagnostic,
        });
    }
    let doc = match parse_overlay(&raw) {
        Err(OverlayError::Parse(message)) => {
            return Err(OverlayError::MalformedCache { path, message });
        }
        other => other?,
    };
    let age = fs::metadata(&path)
        .and_then(|meta| meta.modified())
        .ok()
        .and_then(|mtime| SystemTime::now().duration_since(mtime).ok());
    Ok(Some(CachedOverlay { doc, age }))
}

fn overlay_ttl() -> Duration {
    std::env::var(OVERLAY_TTL_ENV)
        .ok()
        .and_then(|raw| raw.parse::<u64>().ok())
        .map(Duration::from_secs)
        .unwrap_or(OVERLAY_TTL_DEFAULT)
}

// ---------------------------------------------------------------------------
// Freshness — locally-provable truth only
// ---------------------------------------------------------------------------

/// What loctree can prove about the CURRENT repo state without asking the
/// producer. `None` fields skip their check (e.g. a snapshot without a git
/// commit cannot prove snapshot drift).
#[derive(Debug, Clone, Default)]
pub struct LocalTruth {
    pub repo_id: Option<String>,
    pub snapshot_commit: Option<String>,
    pub anchor_catalog_revision: Option<String>,
}

/// `None` = the cached document matches every locally-provable key
/// component. `Some(reason)` = at least one component drifted.
pub fn staleness_reason(cached: &OverlayKey, local: &LocalTruth) -> Option<String> {
    if cached.schema_version != OVERLAY_SCHEMA_VERSION {
        return Some(format!(
            "schema version drift (cache {}, consumer {})",
            cached.schema_version, OVERLAY_SCHEMA_VERSION
        ));
    }
    if let Some(repo_id) = &local.repo_id
        && &cached.repo_id != repo_id
    {
        return Some(format!(
            "repo identity changed (cache {}, local {})",
            cached.repo_id, repo_id
        ));
    }
    if let Some(commit) = &local.snapshot_commit
        && &cached.snapshot_commit != commit
    {
        return Some(format!(
            "snapshot moved (cache {}, local {})",
            cached.snapshot_commit, commit
        ));
    }
    if let Some(acr) = &local.anchor_catalog_revision
        && &cached.anchor_catalog_revision != acr
    {
        return Some(format!(
            "anchor catalog changed (cache {}, local {})",
            short_revision(&cached.anchor_catalog_revision),
            short_revision(acr)
        ));
    }
    None
}

/// Shorten `sr1:<64hex>`-style revisions for human-facing lines.
pub fn short_revision(revision: &str) -> String {
    match revision.split_once(':') {
        Some((prefix, hex)) if hex.len() > 12 => format!("{prefix}:{}…", &hex[..12]),
        _ => revision.to_string(),
    }
}

// ---------------------------------------------------------------------------
// Render state — what the pill / full JSON actually carries
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "state", rename_all = "snake_case")]
pub enum OverlayFreshness {
    /// Every locally-provable key component matches the cached document.
    Fresh,
    /// Cache exists but a locally-provable component drifted; the cached
    /// theses are still rendered (last correct data beats silence) with an
    /// explicit marker, and a detached refresh converges the cache.
    Stale { reason: String },
    /// No usable cache (never fetched, or contract-invalid file).
    Missing { reason: String },
}

impl OverlayFreshness {
    pub fn is_fresh(&self) -> bool {
        matches!(self, OverlayFreshness::Fresh)
    }
}

/// Overlay consumption outcome for one `loct context` composition. Serialized
/// into the ContextPack (additive field) so `--full`/`--json` consumers see
/// the same integration identity the pill renders.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OverlayRenderState {
    pub schema_version: String,
    pub repo_id: String,
    pub store_revision: String,
    pub overlay_revision: String,
    pub snapshot_commit: String,
    pub anchor_catalog_revision: String,
    pub producer_version: String,
    pub freshness: OverlayFreshness,
    /// Key components that changed since the previously accepted cache
    /// (i.e. the last detached refresh actually replaced the world).
    /// `None` = full-key cache hit against the previous render.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub key_transition: Option<Vec<String>>,
    /// Preformatted spec-grammar lines: `✓[U] 2026-07-13 · <thesis> …`.
    pub theses: Vec<String>,
    /// Entry target paths usable as scope seeds (replaces the legacy
    /// budgeted `aicx intents` scope probe on the bare path).
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub scope_paths: Vec<String>,
    /// Operator command that refreshes the intent layer by hand.
    pub refresh_command: String,
    /// True when the CLI handler should spawn the detached refresh.
    pub refresh_recommended: bool,
}

impl OverlayRenderState {
    fn missing(reason: String, refresh_command: String) -> Self {
        OverlayRenderState {
            schema_version: OVERLAY_SCHEMA_VERSION.to_string(),
            repo_id: String::new(),
            store_revision: String::new(),
            overlay_revision: String::new(),
            snapshot_commit: String::new(),
            anchor_catalog_revision: String::new(),
            producer_version: String::new(),
            freshness: OverlayFreshness::Missing { reason },
            key_transition: None,
            theses: Vec::new(),
            scope_paths: Vec::new(),
            refresh_command,
            refresh_recommended: true,
        }
    }

    /// The explicit degradation marker (resilience gate grammar):
    /// `intent layer stale (<reason>) — refresh: <command>`.
    pub fn stale_marker(&self) -> Option<String> {
        let reason = match &self.freshness {
            OverlayFreshness::Fresh => return None,
            OverlayFreshness::Stale { reason } => {
                format!("cache {}, {reason}", short_revision(&self.store_revision))
            }
            OverlayFreshness::Missing { reason } => reason.clone(),
        };
        Some(format!(
            "intent layer stale ({reason}) — refresh: `{}`",
            self.refresh_command
        ))
    }
}

/// Human-runnable refresh command for markers and docs.
pub fn refresh_command(root: &Path) -> String {
    format!(
        "{} overlay --repo {} --format json",
        super::shell::aicx_binary().display(),
        root.display()
    )
}

/// Compose the overlay render state for one bare `loct context` run.
///
/// Pure read: loads + validates the cache, checks locally-provable
/// freshness, notes full-key transitions against the previous accepted
/// cache, and decides whether a refresh is recommended. Never spawns and
/// never waits — the CLI handler owns the detached spawn.
pub fn compose_overlay_state(root: &Path, local: &LocalTruth) -> OverlayRenderState {
    let refresh_cmd = refresh_command(root);
    let cached = match load_cached_overlay(root) {
        Ok(Some(cached)) => cached,
        Ok(None) => {
            let reason = if super::is_aicx_available() {
                "no overlay cache yet, first refresh in progress".to_string()
            } else {
                "no overlay cache and no aicx transport reachable".to_string()
            };
            return OverlayRenderState::missing(reason, refresh_cmd);
        }
        Err(err) => {
            // Explicit version/contract error — never a silent drop.
            eprintln!("[loct][context] {err}");
            return OverlayRenderState::missing(format!("cache rejected: {err}"), refresh_cmd);
        }
    };

    let key = cached.doc.key();
    let freshness = match staleness_reason(&key, local) {
        None => OverlayFreshness::Fresh,
        Some(reason) => OverlayFreshness::Stale { reason },
    };

    let key_transition = note_key_transition(root, &key);

    let ttl = overlay_ttl();
    let ttl_expired = !ttl.is_zero() && cached.age.map(|age| age > ttl).unwrap_or(true);
    let refresh_recommended = !freshness.is_fresh() || ttl_expired;

    let mut entries: Vec<&OverlayEntry> = cached.doc.entries.iter().collect();
    entries.sort_by(|a, b| {
        lifecycle_rank(a.status)
            .cmp(&lifecycle_rank(b.status))
            .then_with(|| b.valid_from.cmp(&a.valid_from))
            .then_with(|| a.intent_id.cmp(&b.intent_id))
    });

    let theses: Vec<String> = entries
        .iter()
        .take(THESES_CAP)
        .map(|e| thesis_line(e))
        .collect();

    let mut scope_paths: Vec<String> = Vec::new();
    for entry in &entries {
        let path = match &entry.target {
            OverlayTarget::Path { path } => Some(path),
            OverlayTarget::Symbol {
                path: Some(path), ..
            } => Some(path),
            _ => None,
        };
        if let Some(path) = path
            && !scope_paths.iter().any(|seen| seen == path)
        {
            scope_paths.push(path.clone());
            if scope_paths.len() >= SCOPE_PATHS_CAP {
                break;
            }
        }
    }

    OverlayRenderState {
        schema_version: key.schema_version.clone(),
        repo_id: cached.doc.repo_id.clone(),
        store_revision: cached.doc.store_revision.clone(),
        overlay_revision: cached.doc.overlay_revision.clone(),
        snapshot_commit: cached.doc.snapshot_commit.clone(),
        anchor_catalog_revision: cached.doc.anchor_catalog_revision.clone(),
        producer_version: cached.doc.producer_version.clone(),
        freshness,
        key_transition,
        theses,
        scope_paths,
        refresh_command: refresh_cmd,
        refresh_recommended,
    }
}

fn lifecycle_rank(status: OverlayLifecycle) -> u8 {
    match status {
        OverlayLifecycle::Current => 0,
        OverlayLifecycle::Superseded => 1,
        OverlayLifecycle::Disputed => 2,
    }
}

/// Compare the currently accepted cache key against the key accepted by the
/// previous render (sidecar). Any changed component = the detached refresh
/// replaced the world = a full-key cache miss made visible. Best-effort:
/// sidecar I/O failures degrade to `None` (no claim), never to an error.
fn note_key_transition(root: &Path, key: &OverlayKey) -> Option<Vec<String>> {
    let sidecar = last_key_sidecar_path(root);
    let previous: Option<OverlayKey> = fs::read_to_string(&sidecar)
        .ok()
        .and_then(|raw| serde_json::from_str(&raw).ok());
    if let Ok(serialized) = serde_json::to_string_pretty(key) {
        if let Some(parent) = sidecar.parent() {
            let _ = fs::create_dir_all(parent);
        }
        let _ = fs::write(&sidecar, serialized);
    }
    let previous = previous?;
    let changed = changed_key_components(&previous, key);
    if changed.is_empty() {
        None
    } else {
        Some(changed.into_iter().map(str::to_string).collect())
    }
}

// ---------------------------------------------------------------------------
// Thesis grammar
// ---------------------------------------------------------------------------

/// Render one entry in the mandatory-read grammar:
/// `✓[U] 2026-07-13 · <thesis> → <target> (<store ref>)`.
///
/// Lifecycle mark (✓ current / ⊘ superseded / ✗ disputed) and evidence mark
/// ([V]/[U]/[R]) are SEPARATE dimensions per the contract.
pub fn thesis_line(entry: &OverlayEntry) -> String {
    let mark = match entry.status {
        OverlayLifecycle::Current => '✓',
        OverlayLifecycle::Superseded => '⊘',
        OverlayLifecycle::Disputed => '✗',
    };
    let evidence = match entry.verification_status {
        OverlayVerification::Verified => 'V',
        OverlayVerification::Unverified => 'U',
        OverlayVerification::Refuted => 'R',
    };
    let date = entry.valid_from.get(..10).unwrap_or(&entry.valid_from);
    let thesis = entry.thesis.replace(['\n', '\r'], " ");
    let target = match &entry.target {
        OverlayTarget::Repo => "repo".to_string(),
        OverlayTarget::Path { path } => path.clone(),
        OverlayTarget::Symbol {
            qualified_symbol, ..
        } => qualified_symbol.clone(),
    };
    let label = authority_label_name(entry.authority);
    match entry.refs.first() {
        Some(reference) => format!(
            "{mark}[{evidence}] {date} · {thesis} → `{target}` ({}) ({label})",
            reference.store_ref
        ),
        None => format!("{mark}[{evidence}] {date} · {thesis} → `{target}` ({label})"),
    }
}

/// Map the producer's authority tier onto loctree's provenance label name.
pub fn authority_label_name(authority: OverlayAuthority) -> &'static str {
    match authority {
        OverlayAuthority::OperatorConfirmed => "AicxOperator",
        OverlayAuthority::AgentDerived => "AicxAgent",
        OverlayAuthority::Inferred => "SemanticGuess",
    }
}

// ---------------------------------------------------------------------------
// Detached refresh — render never waits for the producer
// ---------------------------------------------------------------------------

/// Spawn `aicx overlay --repo <root> --format json` in a detached process
/// whose validated-later output lands in the cache for the NEXT invocation.
///
/// - Never waits: the child is not reaped; output goes to a temp file that
///   is atomically renamed over the cache only on producer success.
/// - Throttled by a lock file so render storms do not stack refreshes.
/// - Honors the test-mode spawn kill switch (unit tests never fork).
/// - Unix-only; on other platforms the stale marker + refresh command hint
///   remain the operator's path.
pub fn spawn_detached_refresh(root: &Path) -> bool {
    if super::test_mode_blocks_spawn() {
        return false;
    }
    #[cfg(unix)]
    {
        spawn_detached_refresh_unix(root)
    }
    #[cfg(not(unix))]
    {
        false
    }
}

#[cfg(unix)]
fn spawn_detached_refresh_unix(root: &Path) -> bool {
    use std::os::unix::process::CommandExt;
    use std::process::{Command, Stdio};

    let lock = refresh_lock_path(root);
    if let Ok(meta) = fs::metadata(&lock)
        && let Ok(mtime) = meta.modified()
        && SystemTime::now()
            .duration_since(mtime)
            .map(|age| age < REFRESH_LOCK_TTL)
            .unwrap_or(true)
    {
        return false;
    }
    if let Some(parent) = lock.parent()
        && fs::create_dir_all(parent).is_err()
    {
        return false;
    }
    if fs::write(&lock, format!("{}\n", std::process::id())).is_err() {
        return false;
    }

    // Paths travel via env vars, not string interpolation, so a repo path
    // containing shell metacharacters cannot break out of the script.
    let script = refresh_script();

    let mut cmd = Command::new("/bin/sh");
    cmd.arg("-c")
        .arg(script)
        .env("LOCT_OVERLAY_BIN", super::shell::aicx_binary())
        .env("LOCT_OVERLAY_REPO", root)
        .env("LOCT_OVERLAY_CACHE", overlay_cache_path(root))
        .env("LOCT_OVERLAY_ERROR", refresh_error_path(root))
        .env("LOCT_OVERLAY_LOCK", &lock)
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .process_group(0);

    match cmd.spawn() {
        Ok(_child) => true, // deliberately not reaped — render never waits
        Err(_) => {
            let _ = fs::remove_file(&lock);
            false
        }
    }
}

#[cfg(unix)]
fn refresh_script() -> &'static str {
    r#"
trap 'rm -f "$LOCT_OVERLAY_LOCK"' EXIT
tmp="${LOCT_OVERLAY_CACHE}.tmp.$$"
err="${LOCT_OVERLAY_ERROR}.tmp.$$"
if "$LOCT_OVERLAY_BIN" overlay --repo "$LOCT_OVERLAY_REPO" --format json >"$tmp" 2>"$err"; then
  if test -s "$tmp"; then
    mv -f "$tmp" "$LOCT_OVERLAY_CACHE"
    rm -f "$err" "$LOCT_OVERLAY_ERROR"
  else
    rm -f "$tmp"
    {
      printf '%s\n' 'AICX overlay producer exited 0 but emitted an empty response.'
      printf 'command: %s overlay --repo %s --format json\n' "$LOCT_OVERLAY_BIN" "$LOCT_OVERLAY_REPO"
      if test -s "$err"; then
        printf '%s\n' 'producer stderr:'
        head -c 4096 "$err"
        printf '\n'
      fi
    } >"$LOCT_OVERLAY_ERROR"
    rm -f "$err"
  fi
else
  status=$?
  rm -f "$tmp"
  {
    printf 'AICX overlay producer exited non-zero (status %s).\n' "$status"
    printf 'command: %s overlay --repo %s --format json\n' "$LOCT_OVERLAY_BIN" "$LOCT_OVERLAY_REPO"
    if test -s "$err"; then
      printf '%s\n' 'producer stderr:'
      head -c 4096 "$err"
      printf '\n'
    fi
  } >"$LOCT_OVERLAY_ERROR"
  rm -f "$err"
fi
"#
}

#[cfg(test)]
mod tests {
    use super::*;

    fn valid_overlay_json() -> String {
        format!(
            r#"{{
  "schema": "{OVERLAY_SCHEMA_VERSION}",
  "repo_id": "Loctree/loctree-suite",
  "snapshot_commit": "176fdba9",
  "anchor_catalog_revision": "acr1:{acr}",
  "store_revision": "sr1:{sr}",
  "overlay_revision": "ov1:{ov}",
  "producer_version": "0.11.0",
  "entries": [
    {{
      "intent_id": "int1:2371588e4469af0e",
      "content_hash": "ch1:{ch}",
      "target": {{ "kind": "path", "path": ".github/workflows/ci.yml", "language": "yml" }},
      "thesis": "CI consumer-check is pinned in Makefile and ci.yml",
      "status": "current",
      "authority": "agent_derived",
      "verification_status": "unverified",
      "valid_from": "2026-07-13T16:13:24.877Z",
      "refs": [
        {{
          "evidence_event_id": "ev1:claude:49a84e4c:002279:user:7dd32539a72ba6fc",
          "ref": "session:49a84e4c#turn-888"
        }}
      ]
    }},
    {{
      "intent_id": "int1:dc85f3d6bf625c4b",
      "content_hash": "ch1:{ch}",
      "target": {{ "kind": "repo" }},
      "thesis": "Overlay cache rides the mandatory read",
      "status": "superseded",
      "authority": "operator_confirmed",
      "verification_status": "verified",
      "valid_from": "2026-07-12T10:00:00Z",
      "refs": [
        {{
          "evidence_event_id": "ev1:codex:aaaa:000001:user:aaaaaaaaaaaaaaaa",
          "ref": "chunk:abc123"
        }}
      ]
    }}
  ]
}}"#,
            acr = "3".repeat(64),
            sr = "6".repeat(64),
            ov = "8".repeat(64),
            ch = "e".repeat(64),
        )
    }

    fn valid_doc() -> OverlayDoc {
        parse_overlay(&valid_overlay_json()).expect("fixture parses")
    }

    #[test]
    fn parse_accepts_contract_shaped_document() {
        let doc = valid_doc();
        assert_eq!(doc.repo_id, "Loctree/loctree-suite");
        assert_eq!(doc.entries.len(), 2);
        assert_eq!(doc.key().schema_version, OVERLAY_SCHEMA_VERSION);
    }

    #[test]
    fn parse_rejects_wrong_schema_with_explicit_version_error() {
        let raw = valid_overlay_json().replace(OVERLAY_SCHEMA_VERSION, "loctree.overlay.intent.v2");
        let err = parse_overlay(&raw).expect_err("v2 tag must be rejected");
        let msg = err.to_string();
        assert!(matches!(err, OverlayError::SchemaVersion { .. }), "{msg}");
        assert!(
            msg.contains("version mismatch"),
            "explicit version error, got: {msg}"
        );
        assert!(msg.contains("loctree.overlay.intent.v2"), "{msg}");
    }

    #[test]
    fn parse_rejects_malformed_revisions_and_garbage() {
        let raw = valid_overlay_json().replace("sr1:", "srX:");
        assert!(matches!(
            parse_overlay(&raw),
            Err(OverlayError::Contract(_))
        ));
        assert!(matches!(
            parse_overlay("not json { ["),
            Err(OverlayError::Parse(_))
        ));
        let raw = valid_overlay_json().replace("176fdba9", "NOTHEX");
        assert!(matches!(
            parse_overlay(&raw),
            Err(OverlayError::Contract(_))
        ));
    }

    #[test]
    fn parse_rejects_empty_producer_response_without_json_eof_lie() {
        let err = parse_overlay("\n\t ").expect_err("empty producer response must be rejected");
        let msg = err.to_string();
        assert!(matches!(err, OverlayError::EmptyDocument), "{msg}");
        assert!(msg.contains("empty response"), "{msg}");
        assert!(!msg.contains("EOF while parsing"), "{msg}");
    }

    #[test]
    fn cache_distinguishes_empty_file_from_malformed_nonempty_json() {
        let dir = tempfile::tempdir().expect("tempdir");
        let cache = overlay_cache_path(dir.path());
        fs::create_dir_all(cache.parent().expect("cache has parent"))
            .expect("create cache directory");

        fs::write(&cache, "").expect("write empty cache");
        fs::write(
            refresh_error_path(dir.path()),
            "AICX overlay producer exited 0 but emitted an empty response.",
        )
        .expect("write producer diagnostic");
        let empty = load_cached_overlay(dir.path()).expect_err("zero-byte cache must fail");
        let empty_msg = empty.to_string();
        assert!(
            matches!(empty, OverlayError::EmptyCache { .. }),
            "{empty_msg}"
        );
        assert!(
            empty_msg.contains(cache.to_string_lossy().as_ref()),
            "{empty_msg}"
        );
        assert!(
            empty_msg.contains("producer emitted no document"),
            "{empty_msg}"
        );
        assert!(
            empty_msg.contains("last producer diagnostic"),
            "{empty_msg}"
        );

        fs::write(&cache, "{ definitely-not-json }").expect("write malformed cache");
        let malformed =
            load_cached_overlay(dir.path()).expect_err("malformed non-empty cache must fail");
        let malformed_msg = malformed.to_string();
        assert!(
            matches!(malformed, OverlayError::MalformedCache { .. }),
            "{malformed_msg}"
        );
        assert!(
            malformed_msg.contains(cache.to_string_lossy().as_ref()),
            "{malformed_msg}"
        );
        assert!(malformed_msg.contains("malformed JSON"), "{malformed_msg}");
        assert!(
            !malformed_msg.contains("producer emitted no document"),
            "{malformed_msg}"
        );
    }

    #[cfg(unix)]
    #[test]
    fn refresh_preserves_last_good_cache_when_producer_exits_zero_without_output() {
        use std::os::unix::fs::PermissionsExt;

        let dir = tempfile::tempdir().expect("tempdir");
        let cache = overlay_cache_path(dir.path());
        let error = refresh_error_path(dir.path());
        let lock = refresh_lock_path(dir.path());
        fs::create_dir_all(cache.parent().expect("cache has parent"))
            .expect("create cache directory");
        let last_good = valid_overlay_json();
        fs::write(&cache, &last_good).expect("write last good cache");
        fs::write(&lock, "refreshing").expect("write refresh lock");

        let producer = dir.path().join("empty-aicx");
        fs::write(&producer, "#!/bin/sh\nexit 0\n").expect("write fake producer");
        let mut permissions = fs::metadata(&producer)
            .expect("read fake producer metadata")
            .permissions();
        permissions.set_mode(0o755);
        fs::set_permissions(&producer, permissions).expect("make fake producer executable");

        let status = std::process::Command::new("/bin/sh")
            .arg("-c")
            .arg(refresh_script())
            .env("LOCT_OVERLAY_BIN", &producer)
            .env("LOCT_OVERLAY_REPO", dir.path())
            .env("LOCT_OVERLAY_CACHE", &cache)
            .env("LOCT_OVERLAY_ERROR", &error)
            .env("LOCT_OVERLAY_LOCK", &lock)
            .status()
            .expect("refresh script runs");

        assert!(status.success(), "refresh script itself must complete");
        assert_eq!(
            fs::read_to_string(&cache).expect("read preserved cache"),
            last_good
        );
        assert!(
            fs::read_to_string(&error)
                .expect("read producer diagnostic")
                .contains("exited 0 but emitted an empty response")
        );
        assert!(!lock.exists(), "refresh lock must be released");
    }

    #[test]
    fn full_key_miss_on_any_single_component_change() {
        let base = valid_doc().key();
        type KeyMutation = (&'static str, fn(&mut OverlayKey));
        let cases: [KeyMutation; 6] = [
            ("repo_id", |k| k.repo_id = "Other/repo".into()),
            ("store_revision", |k| {
                k.store_revision = format!("sr1:{}", "9".repeat(64))
            }),
            ("overlay_revision", |k| {
                k.overlay_revision = format!("ov1:{}", "9".repeat(64))
            }),
            ("snapshot_commit", |k| k.snapshot_commit = "deadbee".into()),
            ("anchor_catalog_revision", |k| {
                k.anchor_catalog_revision = format!("acr1:{}", "9".repeat(64))
            }),
            ("schema_version", |k| {
                k.schema_version = "loctree.overlay.intent.v2".into()
            }),
        ];
        assert!(
            changed_key_components(&base, &base.clone()).is_empty(),
            "unchanged full key must be a hit"
        );
        for (component, mutate) in cases {
            let mut fresh = base.clone();
            mutate(&mut fresh);
            let changed = changed_key_components(&base, &fresh);
            assert_eq!(
                changed,
                vec![component],
                "changing {component} must be a miss naming exactly that component"
            );
        }
    }

    #[test]
    fn staleness_is_local_truth_driven() {
        let key = valid_doc().key();
        let matching = LocalTruth {
            repo_id: Some("Loctree/loctree-suite".into()),
            snapshot_commit: Some("176fdba9".into()),
            anchor_catalog_revision: Some(format!("acr1:{}", "3".repeat(64))),
        };
        assert_eq!(staleness_reason(&key, &matching), None);

        let moved = LocalTruth {
            snapshot_commit: Some("deadbeef".into()),
            ..matching.clone()
        };
        let reason = staleness_reason(&key, &moved).expect("snapshot drift is stale");
        assert!(reason.contains("snapshot moved"), "{reason}");

        let catalog = LocalTruth {
            anchor_catalog_revision: Some(format!("acr1:{}", "9".repeat(64))),
            ..matching.clone()
        };
        let reason = staleness_reason(&key, &catalog).expect("catalog drift is stale");
        assert!(reason.contains("anchor catalog changed"), "{reason}");

        // Unknown local facts skip their check instead of guessing.
        assert_eq!(staleness_reason(&key, &LocalTruth::default()), None);
    }

    #[test]
    fn thesis_line_speaks_the_spec_grammar() {
        let doc = valid_doc();
        let current = thesis_line(&doc.entries[0]);
        assert!(
            current.starts_with("✓[U] 2026-07-13 · "),
            "lifecycle+evidence+date grammar, got: {current}"
        );
        assert!(current.contains("(session:49a84e4c#turn-888)"), "{current}");

        let superseded = thesis_line(&doc.entries[1]);
        assert!(
            superseded.starts_with("⊘[V] 2026-07-12 · "),
            "superseded/verified grammar, got: {superseded}"
        );

        let mut disputed = doc.entries[0].clone();
        disputed.status = OverlayLifecycle::Disputed;
        disputed.verification_status = OverlayVerification::Refuted;
        assert!(thesis_line(&disputed).starts_with("✗[R] "));
    }

    #[test]
    fn compose_state_missing_cache_recommends_refresh_with_stale_marker() {
        let dir = tempfile::tempdir().expect("tempdir");
        let state = compose_overlay_state(dir.path(), &LocalTruth::default());
        assert!(matches!(state.freshness, OverlayFreshness::Missing { .. }));
        assert!(state.refresh_recommended);
        let marker = state.stale_marker().expect("missing cache must mark stale");
        assert!(marker.starts_with("intent layer stale ("), "{marker}");
        assert!(marker.contains("— refresh: `"), "{marker}");
        assert!(state.theses.is_empty());
    }

    #[test]
    fn compose_state_fresh_cache_renders_theses_without_marker() {
        let dir = tempfile::tempdir().expect("tempdir");
        let cache = overlay_cache_path(dir.path());
        fs::create_dir_all(cache.parent().unwrap()).unwrap();
        fs::write(&cache, valid_overlay_json()).unwrap();

        let local = LocalTruth {
            repo_id: Some("Loctree/loctree-suite".into()),
            snapshot_commit: Some("176fdba9".into()),
            anchor_catalog_revision: Some(format!("acr1:{}", "3".repeat(64))),
        };
        let state = compose_overlay_state(dir.path(), &local);
        assert!(state.freshness.is_fresh(), "{:?}", state.freshness);
        assert_eq!(state.stale_marker(), None);
        assert_eq!(state.theses.len(), 2);
        assert!(state.theses[0].starts_with("✓[U] "), "current ranks first");
        assert_eq!(
            state.scope_paths,
            vec![".github/workflows/ci.yml".to_string()]
        );
        // Fresh but beyond TTL (age unknown = distrust) → refresh still recommended
        // only when TTL applies; a just-written file is younger than the TTL.
        assert!(!state.refresh_recommended);
    }

    #[test]
    fn compose_state_stale_cache_keeps_last_correct_data_with_marker() {
        let dir = tempfile::tempdir().expect("tempdir");
        let cache = overlay_cache_path(dir.path());
        fs::create_dir_all(cache.parent().unwrap()).unwrap();
        fs::write(&cache, valid_overlay_json()).unwrap();

        let local = LocalTruth {
            repo_id: Some("Loctree/loctree-suite".into()),
            snapshot_commit: Some("beefbeef".into()),
            anchor_catalog_revision: None,
        };
        let state = compose_overlay_state(dir.path(), &local);
        assert!(matches!(state.freshness, OverlayFreshness::Stale { .. }));
        assert!(state.refresh_recommended);
        assert_eq!(
            state.theses.len(),
            2,
            "stale render keeps last correct data"
        );
        let marker = state.stale_marker().expect("stale must mark");
        assert!(marker.contains("snapshot moved"), "{marker}");
        assert!(
            marker.contains("sr1:666666666666…"),
            "marker carries cache revision: {marker}"
        );
    }

    #[test]
    fn compose_state_rejected_cache_is_explicit_not_silent() {
        let dir = tempfile::tempdir().expect("tempdir");
        let cache = overlay_cache_path(dir.path());
        fs::create_dir_all(cache.parent().unwrap()).unwrap();
        let raw = valid_overlay_json().replace(OVERLAY_SCHEMA_VERSION, "loctree.overlay.intent.v9");
        fs::write(&cache, raw).unwrap();

        let state = compose_overlay_state(dir.path(), &LocalTruth::default());
        let OverlayFreshness::Missing { reason } = &state.freshness else {
            panic!(
                "rejected cache must present as missing, got {:?}",
                state.freshness
            );
        };
        assert!(reason.contains("cache rejected"), "{reason}");
        assert!(reason.contains("version mismatch"), "{reason}");
        assert!(
            state.theses.is_empty(),
            "invalid document must not leak rows"
        );
    }

    #[test]
    fn key_transition_reports_full_key_miss_between_renders() {
        let dir = tempfile::tempdir().expect("tempdir");
        let cache = overlay_cache_path(dir.path());
        fs::create_dir_all(cache.parent().unwrap()).unwrap();
        fs::write(&cache, valid_overlay_json()).unwrap();

        let local = LocalTruth::default();
        let first = compose_overlay_state(dir.path(), &local);
        assert_eq!(
            first.key_transition, None,
            "first render has no previous key"
        );

        let second = compose_overlay_state(dir.path(), &local);
        assert_eq!(
            second.key_transition, None,
            "unchanged full key = cache hit"
        );

        // Simulate a detached refresh landing a new producer world.
        let raw = valid_overlay_json()
            .replace(
                &format!("sr1:{}", "6".repeat(64)),
                &format!("sr1:{}", "7".repeat(64)),
            )
            .replace(
                &format!("ov1:{}", "8".repeat(64)),
                &format!("ov1:{}", "9".repeat(64)),
            );
        fs::write(&cache, raw).unwrap();
        let third = compose_overlay_state(dir.path(), &local);
        let changed = third
            .key_transition
            .expect("refresh landing = full-key miss");
        assert!(
            changed.contains(&"store_revision".to_string()),
            "{changed:?}"
        );
        assert!(
            changed.contains(&"overlay_revision".to_string()),
            "{changed:?}"
        );
    }

    #[test]
    fn spawn_is_blocked_in_test_mode() {
        let dir = tempfile::tempdir().expect("tempdir");
        assert!(
            !spawn_detached_refresh(dir.path()),
            "unit tests must never fork the refresh"
        );
    }
}
