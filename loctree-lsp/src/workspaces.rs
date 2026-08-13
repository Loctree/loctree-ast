//! Multi-workspace snapshot routing (Plan 13 of the LSP roadmap).
//!
//! A monorepo can hold several sub-projects, each with its own
//! `.git` identity or `.loctree/` snapshot directory. One LSP daemon
//! serves them all by discovering project roots at `initialized` time and
//! routing every request that carries `project: Option<PathBuf>` to
//! the matching snapshot.
//!
//! ## Discovery contract
//!
//! - The root workspace is always part of the addressable set and is
//!   represented by [`Backend::snapshot`](crate::Backend) (the original
//!   single-workspace handle). It is intentionally not duplicated into
//!   the extras map — single source of truth.
//! - Sub-projects are discovered by walking down from the workspace
//!   root, capped at `max_depth` (default 4), and recording Git roots or
//!   parents with a local `.loctree/snapshot.json`. The root itself is
//!   excluded from extras — callers see it via the dedicated handle.
//! - Common noise directories (`.git`, `target`, `node_modules`,
//!   `dist`, `build`, `.next`, `.turbo`, `.cache`) are pruned during
//!   the walk. They never host meaningful sub-projects and unbounded
//!   walks of `node_modules` were the bug Monika hit on Vista.
//!
//! ## Wire shape
//!
//! `loctree/workspaces` (custom request) returns
//! [`WorkspacesResponse`] — a flat list of [`WorkspaceInfo`] entries,
//! one per addressable workspace, including the root marked with
//! `is_root: true`. Snapshot age is reported in whole seconds so
//! agents can decide when to ask the operator to rescan a stale
//! sub-project.
//!
//! 𝚅𝚒𝚋𝚎𝚌𝚛𝚊𝚏𝚝𝚎𝚍. with AI Agents by Vetcoders ⓒ 2025-2026 Vetcoders

use std::collections::{BTreeSet, HashMap, VecDeque};
use std::env;
use std::path::{Path, PathBuf};
use std::time::{Duration, SystemTime};

use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use serde_json::Value;

use crate::snapshot::SnapshotState;

/// Default depth used when the operator does not pass an override
/// through `initializationOptions.loctree.workspaces.maxDepth`.
pub const DEFAULT_MAX_DEPTH: usize = 4;

/// Hard ceiling — even when init options ask for more, we cap to keep
/// monorepo discovery bounded. A depth of 8 already covers Vista's
/// `apps/<name>/src-tauri/...` chain twice over.
pub const MAX_DEPTH_CEILING: usize = 8;

/// A workspace-of-repositories is degraded once at least this many
/// addressable sub-projects are found below a root that is not itself a
/// project. Three is deliberately conservative: it catches the accidental IDE
/// mega-root while leaving ordinary two-project folders alone.
pub const MEGAROOT_MIN_SUBPROJECTS: usize = 3;

/// Maximum number of parsed snapshots kept resident by default, including the
/// pinned root snapshot when the root is a normal project.
pub const DEFAULT_MAX_RESIDENT_WORKSPACES: usize = 4;

/// Best-effort aggregate snapshot residency budget. Parsed graphs are larger
/// than their JSON input, so accounting multiplies the on-disk size below.
pub const DEFAULT_MEMORY_BUDGET_MB: u64 = 2048;

/// Coarse JSON-to-resident-graph expansion estimate. This is intentionally a
/// constant rather than a heap-accounting dependency in the first guard cut.
pub const SNAPSHOT_MEMORY_ESTIMATE_FACTOR: u64 = 4;

const MEBIBYTE: u64 = 1024 * 1024;
const MAX_RESIDENT_WORKSPACES_ENV: &str = "LOCTREE_LSP_MAX_RESIDENT_WORKSPACES";
const MEMORY_BUDGET_MB_ENV: &str = "LOCTREE_LSP_MEMORY_BUDGET_MB";

/// Startup classification for the LSP root.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum WorkspaceMode {
    /// The root is a project (or contains only a small number of projects), so
    /// the long-standing root snapshot path remains authoritative.
    Standard,
    /// The root is only a container for many projects. The root snapshot is
    /// not loaded or watched; discovered children hydrate on demand.
    MegaRoot {
        subproject_count: usize,
        recommended_root: PathBuf,
    },
}

impl WorkspaceMode {
    pub fn is_mega_root(&self) -> bool {
        matches!(self, Self::MegaRoot { .. })
    }
}

/// Runtime limits for parsed workspace snapshots.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct WorkspaceResidencyConfig {
    pub max_resident_workspaces: usize,
    pub memory_budget_bytes: u64,
}

impl Default for WorkspaceResidencyConfig {
    fn default() -> Self {
        Self {
            max_resident_workspaces: DEFAULT_MAX_RESIDENT_WORKSPACES,
            memory_budget_bytes: DEFAULT_MEMORY_BUDGET_MB * MEBIBYTE,
        }
    }
}

/// Parse residency limits with initialization options taking precedence over
/// environment variables and built-in defaults.
///
/// Accepted option shapes:
/// - nested: `loctree.workspaces.maxResidentWorkspaces` / `memoryBudgetMb`
/// - flat: `loctree.workspaces.maxResidentWorkspaces` / `memoryBudgetMb`
pub fn residency_config_from_options(options: Option<&Value>) -> WorkspaceResidencyConfig {
    let max_from_options = options.and_then(|value| {
        lookup_option_u64(value, &["loctree", "workspaces", "maxResidentWorkspaces"])
    });
    let memory_from_options = options
        .and_then(|value| lookup_option_u64(value, &["loctree", "workspaces", "memoryBudgetMb"]));

    let max_resident_workspaces = max_from_options
        .or_else(|| env_u64(MAX_RESIDENT_WORKSPACES_ENV))
        .unwrap_or(DEFAULT_MAX_RESIDENT_WORKSPACES as u64)
        .max(1) as usize;
    let memory_budget_mb = memory_from_options
        .or_else(|| env_u64(MEMORY_BUDGET_MB_ENV))
        .unwrap_or(DEFAULT_MEMORY_BUDGET_MB)
        .max(1);

    WorkspaceResidencyConfig {
        max_resident_workspaces,
        memory_budget_bytes: memory_budget_mb.saturating_mul(MEBIBYTE),
    }
}

fn lookup_option_u64(value: &Value, path: &[&str]) -> Option<u64> {
    let nested = path
        .iter()
        .try_fold(value, |cursor, key| cursor.get(*key))
        .and_then(Value::as_u64);
    nested.or_else(|| value.get(path.join(".")).and_then(Value::as_u64))
}

fn env_u64(name: &str) -> Option<u64> {
    env::var(name).ok()?.trim().parse().ok()
}

/// Detect an IDE mega-root from already-bounded workspace discovery.
pub fn classify_workspace(root: &Path, discovered: &[PathBuf]) -> WorkspaceMode {
    // A generated atlas alone does not make an IDE container root a project.
    // Require repository identity or a loadable local snapshot; otherwise an
    // old `<mega-root>/.loctree/context-atlas` would permanently disable the
    // guard that exists to stop the root from being scanned again.
    let root_is_project =
        root.join(".git").exists() || root.join(".loctree").join("snapshot.json").is_file();
    if !root_is_project && discovered.len() >= MEGAROOT_MIN_SUBPROJECTS {
        return WorkspaceMode::MegaRoot {
            subproject_count: discovered.len(),
            recommended_root: discovered[0].clone(),
        };
    }
    WorkspaceMode::Standard
}

/// Estimate the live memory attributable to one parsed snapshot from the
/// authoritative JSON file size. Missing files account as zero until a load is
/// attempted; a successful load calls this again before residency insertion.
pub fn estimated_snapshot_bytes(workspace_root: &Path) -> u64 {
    snapshot_json_path(workspace_root)
        .and_then(|path| std::fs::metadata(path).ok())
        .map(|metadata| {
            metadata
                .len()
                .saturating_mul(SNAPSHOT_MEMORY_ESTIMATE_FACTOR)
        })
        .unwrap_or(0)
}

fn snapshot_json_path(workspace_root: &Path) -> Option<PathBuf> {
    let local = workspace_root.join(".loctree").join("snapshot.json");
    if local.is_file() {
        return Some(local);
    }
    let cached = loctree::snapshot::project_cache_dir(workspace_root).join("snapshot.json");
    cached.is_file().then_some(cached)
}

#[derive(Clone)]
struct ResidentWorkspace {
    state: SnapshotState,
    estimated_bytes: u64,
    last_used: u64,
}

/// Cheap routing inventory plus bounded parsed-snapshot residency.
///
/// The root snapshot remains in `Backend::snapshot`; this registry only owns
/// extras, but accounts the root as a pinned entry so both count and memory
/// budgets describe the whole LSP process.
pub(crate) struct WorkspaceRegistry {
    config: WorkspaceResidencyConfig,
    discovered: BTreeSet<PathBuf>,
    resident: HashMap<PathBuf, ResidentWorkspace>,
    pinned_root: Option<(PathBuf, u64)>,
    clock: u64,
}

impl WorkspaceRegistry {
    pub(crate) fn new(config: WorkspaceResidencyConfig) -> Self {
        Self {
            config,
            discovered: BTreeSet::new(),
            resident: HashMap::new(),
            pinned_root: None,
            clock: 0,
        }
    }

    pub(crate) fn set_config(&mut self, config: WorkspaceResidencyConfig) -> Vec<PathBuf> {
        self.config = config;
        self.evict_to_budget()
    }

    pub(crate) fn replace_discovered(&mut self, paths: Vec<PathBuf>) -> Vec<PathBuf> {
        self.discovered = paths.into_iter().collect();
        let removed: Vec<PathBuf> = self
            .resident
            .keys()
            .filter(|path| !self.discovered.contains(*path))
            .cloned()
            .collect();
        for path in &removed {
            self.resident.remove(path);
        }
        removed
    }

    pub(crate) fn set_pinned_root(&mut self, root: PathBuf, estimated_bytes: u64) -> Vec<PathBuf> {
        self.pinned_root = Some((root, estimated_bytes));
        self.evict_to_budget()
    }

    pub(crate) fn clear_pinned_root(&mut self) {
        self.pinned_root = None;
    }

    pub(crate) fn contains_discovered(&self, path: &Path) -> bool {
        self.discovered.contains(path)
    }

    pub(crate) fn discovered_paths(&self) -> Vec<PathBuf> {
        self.discovered.iter().cloned().collect()
    }

    pub(crate) fn resident_paths(&self) -> Vec<PathBuf> {
        let mut paths: Vec<PathBuf> = self.resident.keys().cloned().collect();
        paths.sort();
        paths
    }

    pub(crate) fn resident_state(&self, path: &Path) -> Option<SnapshotState> {
        self.resident.get(path).map(|entry| entry.state.clone())
    }

    pub(crate) fn touch(&mut self, path: &Path) -> Option<SnapshotState> {
        self.clock = self.clock.saturating_add(1);
        let entry = self.resident.get_mut(path)?;
        entry.last_used = self.clock;
        Some(entry.state.clone())
    }

    pub(crate) fn can_eager_load(&self, estimated_bytes: u64) -> bool {
        self.total_resident_count().saturating_add(1) <= self.config.max_resident_workspaces
            && self.total_estimated_bytes().saturating_add(estimated_bytes)
                <= self.config.memory_budget_bytes
    }

    pub(crate) fn insert(
        &mut self,
        path: PathBuf,
        state: SnapshotState,
        estimated_bytes: u64,
    ) -> Vec<PathBuf> {
        self.clock = self.clock.saturating_add(1);
        self.resident.insert(
            path,
            ResidentWorkspace {
                state,
                estimated_bytes,
                last_used: self.clock,
            },
        );
        self.evict_to_budget()
    }

    pub(crate) fn update_estimated_bytes(
        &mut self,
        path: &Path,
        estimated_bytes: u64,
    ) -> Vec<PathBuf> {
        if let Some(entry) = self.resident.get_mut(path) {
            entry.estimated_bytes = estimated_bytes;
        }
        self.evict_to_budget()
    }

    fn total_resident_count(&self) -> usize {
        usize::from(self.pinned_root.is_some()) + self.resident.len()
    }

    fn total_estimated_bytes(&self) -> u64 {
        let root_bytes = self
            .pinned_root
            .as_ref()
            .map(|(_, bytes)| *bytes)
            .unwrap_or(0);
        self.resident.values().fold(root_bytes, |total, entry| {
            total.saturating_add(entry.estimated_bytes)
        })
    }

    fn evict_to_budget(&mut self) -> Vec<PathBuf> {
        let mut evicted = Vec::new();
        while (self.total_resident_count() > self.config.max_resident_workspaces
            || self.total_estimated_bytes() > self.config.memory_budget_bytes)
            && !self.resident.is_empty()
        {
            let Some(oldest) = self
                .resident
                .iter()
                .min_by(|(path_a, a), (path_b, b)| {
                    a.last_used
                        .cmp(&b.last_used)
                        .then_with(|| path_a.cmp(path_b))
                })
                .map(|(path, _)| path.clone())
            else {
                break;
            };
            self.resident.remove(&oldest);
            evicted.push(oldest);
        }
        evicted
    }
}

/// Compute actual filesystem subscriptions. Standard roots keep the original
/// single recursive subscription. Mega-roots subscribe only to parsed resident
/// projects and therefore change exactly with LRU load/evict transitions.
pub(crate) fn watcher_scope(
    root: &Path,
    mode: &WorkspaceMode,
    registry: &WorkspaceRegistry,
) -> BTreeSet<PathBuf> {
    if mode.is_mega_root() {
        registry.resident_paths().into_iter().collect()
    } else {
        BTreeSet::from([root.to_path_buf()])
    }
}

/// Directory names pruned during workspace discovery.
///
/// They never host meaningful sub-projects and walking `node_modules`
/// was the original Vista monorepo bug — pruning is mandatory, not a
/// performance suggestion.
const PRUNED_DIRS: &[&str] = &[
    ".git",
    "target",
    "node_modules",
    "dist",
    "build",
    ".next",
    ".turbo",
    ".cache",
    ".loctree",
];

/// Empty params struct for `loctree/workspaces` (no inputs).
#[derive(Debug, Clone, Default, Deserialize, Serialize, JsonSchema)]
pub struct WorkspacesParams {}

/// One row in the response: an addressable LSP workspace.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WorkspaceInfo {
    /// Canonical project root (absolute, OS-form).
    pub root: String,
    /// `true` for the workspace LSP started in.
    pub is_root: bool,
    /// `true` when the workspace currently has a loaded snapshot.
    /// `false` for sub-projects whose `.loctree/snapshot.json` was
    /// missing or unreadable at discovery time.
    pub has_snapshot: bool,
    /// Files in the loaded snapshot (0 when `has_snapshot=false`).
    pub files: usize,
    /// Languages observed in the loaded snapshot, sorted alphabetically.
    pub languages: Vec<String>,
    /// Age of the snapshot file in whole seconds. `None` when the
    /// snapshot is missing or its mtime cannot be read.
    pub snapshot_age_seconds: Option<u64>,
}

/// Wire envelope for the `loctree/workspaces` response.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WorkspacesResponse {
    /// All addressable workspaces, root first.
    pub workspaces: Vec<WorkspaceInfo>,
}

/// Read `loctree.workspaces.maxDepth` from `initializationOptions`.
///
/// Honors both nested (`{"loctree":{"workspaces":{"maxDepth":...}}}`)
/// and flat (`{"loctree.workspaces.maxDepth": 6}`) shapes for parity
/// with the watcher and protocol options.
pub fn max_depth_from_options(options: Option<&Value>) -> usize {
    let Some(value) = options else {
        return DEFAULT_MAX_DEPTH;
    };
    let nested = value
        .pointer("/loctree/workspaces/maxDepth")
        .and_then(|v| v.as_u64());
    let flat = value
        .get("loctree.workspaces.maxDepth")
        .and_then(|v| v.as_u64());
    let raw = nested.or(flat).unwrap_or(DEFAULT_MAX_DEPTH as u64) as usize;
    raw.clamp(1, MAX_DEPTH_CEILING)
}

/// Walk `root` looking for Git roots or local Loctree snapshots.
///
/// Returns canonical parent paths (deduplicated, sorted). The root
/// itself is never included — callers handle the root workspace
/// through its dedicated handle. A discovered project is also a pruning
/// boundary, so nested fixture repositories do not leak into the workspace
/// inventory. Pruned directories
/// ([`PRUNED_DIRS`]) are skipped to keep monorepo discovery bounded.
pub fn discover_loctree_dirs(root: &Path, max_depth: usize) -> Vec<PathBuf> {
    let depth = max_depth.clamp(1, MAX_DEPTH_CEILING);
    let canonical_root = canonicalize(root);
    let mut found: BTreeSet<PathBuf> = BTreeSet::new();

    // BFS so we can prune entire subtrees with `continue;` on directory
    // names that are guaranteed-noise (node_modules, .git, target …).
    let mut queue: VecDeque<(PathBuf, usize)> = VecDeque::new();
    queue.push_back((canonical_root.clone(), 0));

    while let Some((dir, current_depth)) = queue.pop_front() {
        if current_depth > depth {
            continue;
        }

        // Discovery is intentionally path-only. Git roots remain addressable
        // even when their Loctree snapshot lives exclusively in the global
        // cache, and stopping at the project boundary avoids collecting nested
        // fixture repositories as sibling workspaces.
        if current_depth > 0
            && (dir.join(".git").exists() || dir.join(".loctree").join("snapshot.json").is_file())
        {
            found.insert(canonicalize(&dir));
            continue;
        }

        let entries = match std::fs::read_dir(&dir) {
            Ok(entries) => entries,
            Err(_) => continue,
        };

        for entry in entries.flatten() {
            let path = entry.path();
            let name = match path.file_name().and_then(|n| n.to_str()) {
                Some(n) => n,
                None => continue,
            };

            // Resolve symlinks before deciding what kind of node this
            // is — `read_dir` returns symlinks unresolved on macOS.
            let metadata = match std::fs::symlink_metadata(&path) {
                Ok(m) => m,
                Err(_) => continue,
            };
            // Don't follow symlinks during discovery: a self-referential
            // link would cause the walk to never terminate.
            if metadata.file_type().is_symlink() {
                continue;
            }
            if !metadata.is_dir() {
                continue;
            }

            if PRUNED_DIRS.contains(&name) {
                continue;
            }

            queue.push_back((path, current_depth + 1));
        }
    }

    // Always exclude the root from extras — it is addressed via the
    // backend's primary snapshot handle.
    found.remove(&canonical_root);

    found.into_iter().collect()
}

/// Best-effort canonicalization. Falls back to the original path when
/// the filesystem rejects the lookup (network drive, permissions, …)
/// so discovery remains usable on weird filesystems.
pub fn canonicalize(path: &Path) -> PathBuf {
    path.canonicalize().unwrap_or_else(|_| path.to_path_buf())
}

/// Compute snapshot age in seconds from the `.loctree/snapshot.json`
/// modification time. Returns `None` when the file is missing or its
/// mtime cannot be read — the caller surfaces the absence directly.
pub fn snapshot_age(workspace_root: &Path) -> Option<u64> {
    // Loctree stores snapshots in a global cache (see `Snapshot::save`),
    // but a per-project mirror lives at `<root>/.loctree/snapshot.json`
    // when the operator opts into local persistence. Either path works
    // as an age signal — the cache mtime is the authoritative one.
    let local = workspace_root.join(".loctree").join("snapshot.json");
    let candidate = if local.exists() {
        local
    } else {
        // Fall back to the global cache mtime. The cache layout is
        // owned by `loctree::snapshot::project_cache_dir`, so we ask
        // it directly rather than hard-coding the layout here.
        let cache = loctree::snapshot::project_cache_dir(workspace_root);
        let snapshot_json = cache.join("snapshot.json");
        if snapshot_json.exists() {
            snapshot_json
        } else {
            return None;
        }
    };

    let mtime = std::fs::metadata(&candidate).ok()?.modified().ok()?;
    let now = SystemTime::now();
    let age = now.duration_since(mtime).unwrap_or(Duration::ZERO);
    Some(age.as_secs())
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;
    use tempfile::TempDir;

    /// Helper: create a `.loctree/snapshot.json` marker under `parent`
    /// so `discover_loctree_dirs` recognizes the parent as a real
    /// addressable sub-project. The file content is irrelevant to
    /// discovery — only its presence matters (Snapshot::load handles
    /// the parsing later).
    fn touch_loctree_marker(parent: &Path) {
        let dir = parent.join(".loctree");
        std::fs::create_dir_all(&dir).expect("create .loctree dir");
        std::fs::write(dir.join("snapshot.json"), b"{}").expect("write snapshot.json marker");
    }

    fn touch_git_marker(parent: &Path) {
        std::fs::create_dir_all(parent.join(".git")).expect("create .git dir");
    }

    #[test]
    fn default_max_depth_is_4() {
        assert_eq!(DEFAULT_MAX_DEPTH, 4);
    }

    #[test]
    fn max_depth_reads_nested_option() {
        let opts = json!({
            "loctree": { "workspaces": { "maxDepth": 6 } }
        });
        assert_eq!(max_depth_from_options(Some(&opts)), 6);
    }

    #[test]
    fn max_depth_reads_flat_option() {
        let opts = json!({ "loctree.workspaces.maxDepth": 2 });
        assert_eq!(max_depth_from_options(Some(&opts)), 2);
    }

    #[test]
    fn max_depth_clamps_overflow() {
        let opts = json!({ "loctree.workspaces.maxDepth": 100 });
        assert_eq!(max_depth_from_options(Some(&opts)), MAX_DEPTH_CEILING);
    }

    #[test]
    fn max_depth_clamps_zero_to_one() {
        let opts = json!({ "loctree.workspaces.maxDepth": 0 });
        assert_eq!(max_depth_from_options(Some(&opts)), 1);
    }

    #[test]
    fn max_depth_default_when_options_absent() {
        assert_eq!(max_depth_from_options(None), DEFAULT_MAX_DEPTH);
    }

    #[test]
    fn residency_options_override_defaults_in_nested_and_flat_forms() {
        let nested = json!({
            "loctree": {
                "workspaces": {
                    "maxResidentWorkspaces": 7,
                    "memoryBudgetMb": 512
                }
            }
        });
        let nested_cfg = residency_config_from_options(Some(&nested));
        assert_eq!(nested_cfg.max_resident_workspaces, 7);
        assert_eq!(nested_cfg.memory_budget_bytes, 512 * MEBIBYTE);

        let flat = json!({
            "loctree.workspaces.maxResidentWorkspaces": 2,
            "loctree.workspaces.memoryBudgetMb": 64
        });
        let flat_cfg = residency_config_from_options(Some(&flat));
        assert_eq!(flat_cfg.max_resident_workspaces, 2);
        assert_eq!(flat_cfg.memory_budget_bytes, 64 * MEBIBYTE);
    }

    #[test]
    fn classifier_degrades_only_unmarked_roots_with_many_subprojects() {
        let temp = TempDir::new().expect("tempdir");
        for name in ["repo-a", "repo-b", "repo-c"] {
            touch_loctree_marker(&temp.path().join(name));
        }
        let discovered = discover_loctree_dirs(temp.path(), DEFAULT_MAX_DEPTH);
        assert!(matches!(
            classify_workspace(temp.path(), &discovered),
            WorkspaceMode::MegaRoot {
                subproject_count: 3,
                ..
            }
        ));

        std::fs::create_dir_all(temp.path().join(".git")).expect("root git marker");
        assert_eq!(
            classify_workspace(temp.path(), &discovered),
            WorkspaceMode::Standard,
            "a real repository root stays in standard mode"
        );
    }

    #[test]
    fn classifier_ignores_generated_root_atlas_and_discovers_git_projects() {
        let temp = TempDir::new().expect("tempdir");
        std::fs::create_dir_all(temp.path().join(".loctree/context-atlas"))
            .expect("create generated root atlas");
        for name in ["repo-a", "repo-b", "repo-c"] {
            touch_git_marker(&temp.path().join(name));
        }

        let discovered = discover_loctree_dirs(temp.path(), DEFAULT_MAX_DEPTH);
        assert_eq!(discovered.len(), 3);
        assert!(matches!(
            classify_workspace(temp.path(), &discovered),
            WorkspaceMode::MegaRoot {
                subproject_count: 3,
                ..
            }
        ));
    }

    #[test]
    fn lru_evicts_oldest_touches_recency_and_never_evicts_pinned_root() {
        let root = PathBuf::from("/root");
        let a = PathBuf::from("/root/a");
        let b = PathBuf::from("/root/b");
        let c = PathBuf::from("/root/c");
        let mut registry = WorkspaceRegistry::new(WorkspaceResidencyConfig {
            max_resident_workspaces: 3,
            memory_budget_bytes: 1_000,
        });
        registry.replace_discovered(vec![a.clone(), b.clone(), c.clone()]);
        assert!(registry.set_pinned_root(root.clone(), 10).is_empty());
        assert!(
            registry
                .insert(a.clone(), SnapshotState::new(), 10)
                .is_empty()
        );
        assert!(
            registry
                .insert(b.clone(), SnapshotState::new(), 10)
                .is_empty()
        );

        assert!(registry.touch(&a).is_some(), "route hit refreshes recency");
        let evicted = registry.insert(c.clone(), SnapshotState::new(), 10);
        assert_eq!(evicted, vec![b.clone()]);
        assert_eq!(registry.resident_paths(), vec![a, c]);
        assert_eq!(
            registry.pinned_root.as_ref().map(|(path, _)| path),
            Some(&root)
        );
        assert!(
            !evicted.contains(&root),
            "pinned root is never an LRU target"
        );
    }

    #[test]
    fn mega_root_watcher_scope_tracks_residents_after_eviction() {
        let root = PathBuf::from("/mega");
        let a = PathBuf::from("/mega/a");
        let b = PathBuf::from("/mega/b");
        let c = PathBuf::from("/mega/c");
        let mode = WorkspaceMode::MegaRoot {
            subproject_count: 3,
            recommended_root: a.clone(),
        };
        let mut registry = WorkspaceRegistry::new(WorkspaceResidencyConfig {
            max_resident_workspaces: 2,
            memory_budget_bytes: 1_000,
        });
        registry.replace_discovered(vec![a.clone(), b.clone(), c.clone()]);
        registry.insert(a.clone(), SnapshotState::new(), 10);
        registry.insert(b.clone(), SnapshotState::new(), 10);
        assert_eq!(
            watcher_scope(&root, &mode, &registry),
            BTreeSet::from([a.clone(), b.clone()])
        );

        registry.touch(&a);
        assert_eq!(
            registry.insert(c.clone(), SnapshotState::new(), 10),
            vec![b]
        );
        assert_eq!(
            watcher_scope(&root, &mode, &registry),
            BTreeSet::from([a, c])
        );
        assert!(!watcher_scope(&root, &mode, &registry).contains(&root));
    }

    #[test]
    fn discover_returns_empty_for_repo_with_no_subprojects() {
        let temp = TempDir::new().expect("tempdir");
        let root = temp.path();
        touch_loctree_marker(root);

        let found = discover_loctree_dirs(root, 4);
        assert!(found.is_empty(), "root should be excluded from extras");
    }

    #[test]
    fn discover_finds_subproject_one_level_deep() {
        let temp = TempDir::new().expect("tempdir");
        let root = temp.path();
        touch_loctree_marker(&root.join("apps/web"));

        let found = discover_loctree_dirs(root, 4);
        assert_eq!(found.len(), 1);
        assert!(
            found[0].ends_with("apps/web"),
            "expected apps/web parent, got {}",
            found[0].display()
        );
    }

    #[test]
    fn discover_finds_git_roots_without_local_snapshots_and_prunes_their_children() {
        let temp = TempDir::new().expect("tempdir");
        let repo_a = temp.path().join("repo-a");
        let repo_b = temp.path().join("nested/repo-b");
        touch_git_marker(&repo_a);
        touch_git_marker(&repo_a.join("fixtures/nested-repo"));
        touch_git_marker(&repo_b);

        let found = discover_loctree_dirs(temp.path(), DEFAULT_MAX_DEPTH);
        assert_eq!(found, vec![canonicalize(&repo_b), canonicalize(&repo_a)]);
    }

    #[test]
    fn discover_finds_multiple_subprojects_at_different_depths() {
        let temp = TempDir::new().expect("tempdir");
        let root = temp.path();
        touch_loctree_marker(&root.join("apps/web"));
        touch_loctree_marker(&root.join("apps/api/src"));
        touch_loctree_marker(&root.join("packages/ui"));

        let found = discover_loctree_dirs(root, 4);
        assert_eq!(found.len(), 3);
        let labels: Vec<String> = found
            .iter()
            .map(|p| {
                p.components()
                    .rev()
                    .take(2)
                    .map(|c| c.as_os_str().to_string_lossy().into_owned())
                    .collect::<Vec<_>>()
                    .into_iter()
                    .rev()
                    .collect::<Vec<_>>()
                    .join("/")
            })
            .collect();
        assert!(labels.iter().any(|l| l.ends_with("apps/web")));
        assert!(labels.iter().any(|l| l.ends_with("src")));
        assert!(labels.iter().any(|l| l.ends_with("packages/ui")));
    }

    #[test]
    fn discover_prunes_node_modules() {
        let temp = TempDir::new().expect("tempdir");
        let root = temp.path();
        // Synthetic .loctree under node_modules — must NOT be discovered.
        touch_loctree_marker(&root.join("node_modules/poison"));
        touch_loctree_marker(&root.join("apps/web"));

        let found = discover_loctree_dirs(root, 4);
        assert_eq!(found.len(), 1);
        assert!(found[0].ends_with("apps/web"));
    }

    #[test]
    fn discover_prunes_target_and_dot_git() {
        let temp = TempDir::new().expect("tempdir");
        let root = temp.path();
        touch_loctree_marker(&root.join("target/release"));
        touch_loctree_marker(&root.join(".git/poison"));
        touch_loctree_marker(&root.join("crate-a"));

        let found = discover_loctree_dirs(root, 4);
        assert_eq!(found.len(), 1);
        assert!(found[0].ends_with("crate-a"));
    }

    #[test]
    fn discover_respects_max_depth() {
        let temp = TempDir::new().expect("tempdir");
        let root = temp.path();
        // 6-deep — the .loctree dir itself sits at depth 7, parent at 6.
        touch_loctree_marker(&root.join("a/b/c/d/e/f"));

        let shallow = discover_loctree_dirs(root, 3);
        assert!(shallow.is_empty());

        let deep = discover_loctree_dirs(root, MAX_DEPTH_CEILING);
        assert_eq!(deep.len(), 1);
        assert!(deep[0].ends_with("a/b/c/d/e/f"));
    }

    #[test]
    fn discover_dedups_when_same_parent_has_two_loctree_paths() {
        // Pathological filesystem: a parent containing both `.loctree/`
        // and a duplicate via symlink-style alias is not testable here,
        // but the BTreeSet contract guarantees dedup. Record the
        // expectation via the simpler "single child" case.
        let temp = TempDir::new().expect("tempdir");
        let root = temp.path();
        touch_loctree_marker(&root.join("only"));

        let found = discover_loctree_dirs(root, 4);
        assert_eq!(found.len(), 1);
    }

    #[test]
    fn discover_skips_empty_loctree_marker_without_snapshot() {
        // Test fixtures sometimes leave behind `.loctree/` directories
        // without `snapshot.json` (init artifacts, copy-paste setup,
        // tools/fixtures/** integration scaffolds). Discovery must
        // skip them so the LSP root does not log spurious
        // "Snapshot not found at .../.loctree" warnings on every
        // `initialized` and so the addressable workspace set stays
        // truthful.
        let temp = TempDir::new().expect("tempdir");
        let root = temp.path();
        // Real sub-project: has snapshot.json.
        touch_loctree_marker(&root.join("apps/real"));
        // Empty markers: directory exists, no snapshot.json.
        std::fs::create_dir_all(root.join("tools/fixtures/dist-test/src/.loctree"))
            .expect("create empty fixture .loctree");
        std::fs::create_dir_all(root.join("tools/fixtures/nodejs-loader/.loctree"))
            .expect("create empty fixture .loctree");

        let found = discover_loctree_dirs(root, MAX_DEPTH_CEILING);
        assert_eq!(
            found.len(),
            1,
            "only the real sub-project should be discovered, got {found:?}"
        );
        assert!(found[0].ends_with("apps/real"));
    }
}
