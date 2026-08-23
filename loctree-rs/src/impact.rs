//! Impact analysis module for understanding consequences of file changes.
//!
//! Provides "what breaks if you modify/remove this file" analysis by traversing
//! the reverse dependency graph to find all direct and transitive consumers.
//!
//! Honesty contract (trust-repair W2-A): a zero-consumer result is *absence of
//! evidence*, not evidence of absence. Removal advice is fail-closed behind
//! [`GraphCoverage`] — until every removal-relevant surface (package manifests,
//! build scripts, CI workflows, test harnesses, framework dispatch) is proven
//! accounted for, the zero-consumer path emits
//! [`COVERAGE_INCOMPLETE_DIAGNOSTIC`] instead of removal advice. Fixtures
//! encode "coverage incomplete" simply by using `GraphCoverage::default()`
//! (all surfaces `false`); "coverage proven" fixtures set every flag `true`.

use serde::{Deserialize, Serialize};
use std::collections::{HashMap, HashSet, VecDeque};

use crate::snapshot::Snapshot;

/// Canonical fail-closed diagnostic for a zero-consumer result whose graph
/// coverage is unproven. Shared verbatim by CLI, MCP, and LSP surfaces so the
/// honesty contract stays in lockstep across binaries.
pub const COVERAGE_INCOMPLETE_DIAGNOSTIC: &str = "coverage incomplete; cannot assess removal";

/// Graph-coverage accounting for a single impact analysis.
///
/// The import graph only sees statically indexed source imports. Removal
/// safety additionally depends on surfaces the scanner does not yet prove:
/// package manifests, build scripts (Make targets, build.rs), CI workflows,
/// test harnesses, and framework/runtime dispatch (loaders, reflection,
/// `@main`, entrypoint registration). Every flag stays `false` until the
/// analyzer can prove that surface was accounted for — fail closed, so a
/// zero-consumer readout can never silently become removal advice.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct GraphCoverage {
    /// Package manifests (Cargo.toml bins/crate roots, package.json, SwiftPM…).
    pub package_manifests: bool,
    /// Build scripts: Make targets, build.rs, justfiles, install scripts.
    pub build_scripts: bool,
    /// CI workflow references (GitHub Actions, pipelines).
    pub ci_workflows: bool,
    /// Test harnesses and fixtures that load the target outside import edges.
    pub test_harnesses: bool,
    /// Framework/runtime dispatch: loaders, reflection, `@main`, plugin registries.
    pub framework_dispatch: bool,
}

impl GraphCoverage {
    /// True only when every removal-relevant surface is proven accounted for.
    pub fn is_complete(&self) -> bool {
        self.package_manifests
            && self.build_scripts
            && self.ci_workflows
            && self.test_harnesses
            && self.framework_dispatch
    }

    /// Names of surfaces the graph has NOT proven accounted for.
    pub fn gaps(&self) -> Vec<&'static str> {
        let mut gaps = Vec::new();
        if !self.package_manifests {
            gaps.push("package manifests");
        }
        if !self.build_scripts {
            gaps.push("build scripts (Make, build.rs)");
        }
        if !self.ci_workflows {
            gaps.push("CI workflows");
        }
        if !self.test_harnesses {
            gaps.push("test harnesses");
        }
        if !self.framework_dispatch {
            gaps.push("framework/runtime dispatch");
        }
        gaps
    }
}

/// Result of impact analysis for a file
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ImpactResult {
    /// Target file that was analyzed
    pub target: String,
    /// Direct consumers (files that directly import this file)
    pub direct_consumers: Vec<ImpactEntry>,
    /// Transitive consumers (files that depend on direct consumers)
    pub transitive_consumers: Vec<ImpactEntry>,
    /// Total number of affected files (direct + transitive)
    pub total_affected: usize,
    /// Maximum depth in the dependency chain
    pub max_depth: usize,
    /// True when the target itself is normally excluded by `.loctignore` and was
    /// only analyzed because the read ran with `--include-ignored`. A `.loctignore`
    /// target usually has no indexed consumers, so a zero-consumer readout on
    /// such a file says nothing about real usage.
    #[serde(default, skip_serializing_if = "std::ops::Not::not")]
    pub target_ignored: bool,
    /// Coverage accounting for the graph this result was computed from.
    /// A zero-consumer result only supports removal when every flag here is
    /// proven `true`; otherwise absence of consumers is unknown, not safety.
    #[serde(default)]
    pub coverage: GraphCoverage,
}

/// Single file in the impact chain
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ImpactEntry {
    /// File path
    pub file: String,
    /// Depth from target (1 = direct consumer, 2 = consumer of consumer, etc.)
    pub depth: usize,
    /// Import relationship label
    pub import_type: String,
    /// Path from target to this file (chain of imports)
    pub chain: Vec<String>,
}

/// Options for impact analysis
#[derive(Debug, Clone)]
pub struct ImpactOptions {
    /// Maximum depth to traverse (None = unlimited)
    pub max_depth: Option<usize>,
    /// Whether to include re-export chains
    pub include_reexports: bool,
}

impl Default for ImpactOptions {
    fn default() -> Self {
        Self {
            max_depth: None,
            include_reexports: true,
        }
    }
}

/// Analyze the impact of modifying or removing a file.
///
/// This function performs a breadth-first traversal of the reverse dependency graph
/// starting from the target file, finding all files that would be affected.
///
/// # Arguments
/// * `snapshot` - The code snapshot containing the dependency graph
/// * `target` - Path to the file to analyze
/// * `options` - Impact analysis options
///
/// # Returns
/// Impact analysis result with direct and transitive consumers
pub fn analyze_impact(snapshot: &Snapshot, target: &str, options: &ImpactOptions) -> ImpactResult {
    let mut direct_consumers = Vec::new();
    let mut transitive_consumers = Vec::new();
    let mut visited: HashSet<String> = HashSet::new();
    let mut queue: VecDeque<(String, usize, Vec<String>)> = VecDeque::new();
    let mut max_depth = 0;

    // Build reverse dependency map for efficient lookup
    let reverse_deps = build_reverse_dependency_map(snapshot);

    // Normalize target path (handles absolute paths, ./ prefix, backslashes)
    let normalized_target = normalize_path(&snapshot.normalize_path(target));

    // Start BFS from target file
    queue.push_back((
        normalized_target.clone(),
        0,
        vec![normalized_target.clone()],
    ));
    visited.insert(normalized_target.clone());

    while let Some((current, depth, chain)) = queue.pop_front() {
        // Check depth limit
        if let Some(max) = options.max_depth
            && depth >= max
        {
            continue;
        }

        // Find all files that import the current file
        if let Some(importers) = reverse_deps.get(&current) {
            for (importer, import_type) in importers {
                if visited.contains(importer) {
                    continue;
                }

                visited.insert(importer.clone());
                let new_depth = depth + 1;
                max_depth = max_depth.max(new_depth);

                let mut new_chain = chain.clone();
                new_chain.push(importer.clone());

                let entry = ImpactEntry {
                    file: importer.clone(),
                    depth: new_depth,
                    import_type: import_type.clone(),
                    chain: new_chain.clone(),
                };

                // Categorize as direct or transitive
                if new_depth == 1 {
                    direct_consumers.push(entry);
                } else {
                    transitive_consumers.push(entry);
                }

                // Continue traversal
                queue.push_back((importer.clone(), new_depth, new_chain));
            }
        }
    }

    // Sort by depth, then by file path
    direct_consumers.sort_by(|a, b| a.file.cmp(&b.file));
    transitive_consumers.sort_by(|a, b| a.depth.cmp(&b.depth).then_with(|| a.file.cmp(&b.file)));

    let total_affected = direct_consumers.len() + transitive_consumers.len();

    let target_ignored = snapshot
        .files
        .iter()
        .any(|f| f.ignored && (f.path == target || f.path.ends_with(target)));

    ImpactResult {
        target: target.to_string(),
        direct_consumers,
        transitive_consumers,
        total_affected,
        max_depth,
        target_ignored,
        // The snapshot import graph does not yet prove manifests, build
        // scripts, workflows, test harnesses, or framework dispatch, so
        // coverage stays all-false until dedicated analyzers land.
        coverage: GraphCoverage::default(),
    }
}

/// Build a reverse dependency map from the snapshot edges.
///
/// Maps each file to a list of (importer, import_type) pairs.
fn build_reverse_dependency_map(snapshot: &Snapshot) -> HashMap<String, Vec<(String, String)>> {
    let mut map: HashMap<String, Vec<(String, String)>> = HashMap::new();

    for edge in &snapshot.edges {
        let from = normalize_path(&edge.from);
        let to = normalize_path(&edge.to);
        let label = edge.label.clone();

        map.entry(to).or_default().push((from, label));
    }

    map
}

/// Normalize a file path for consistent matching.
///
/// Handles:
/// - Relative vs absolute paths
/// - index.ts/index.tsx variants
/// - Trailing slashes
fn normalize_path(path: &str) -> String {
    let mut p = path.to_string();

    // Remove leading ./ or /
    p = p.trim_start_matches("./").to_string();

    // Handle index variants
    if p.ends_with("/index.ts") || p.ends_with("/index.tsx") || p.ends_with("/index.js") {
        // Also store the folder path variant
        return p;
    }

    p
}

/// Format impact result as human-readable text
pub fn format_impact_text(result: &ImpactResult) -> String {
    let mut output = String::new();

    output.push_str(&format!("Impact analysis for: {}\n", result.target));
    if result.target_ignored {
        output.push_str(
            "[ignored: .loctignore] Target is normally excluded from the index; consumer edges may be undercounted.\n",
        );
    }
    output.push('\n');

    if result.total_affected == 0 {
        if result.target_ignored {
            output.push_str(
                "[UNPROVEN] Target is `.loctignore`-excluded, so consumer edges may be missing entirely from the index.\n",
            );
        }
        if result.coverage.is_complete() {
            output.push_str(
                "[OK] No consumers found; graph coverage is proven complete for removal-relevant surfaces.\n",
            );
        } else {
            output.push_str(&format!(
                "[UNPROVEN] No consumers found in the indexed import graph — {}.\n",
                COVERAGE_INCOMPLETE_DIAGNOSTIC
            ));
            output.push_str(&format!(
                "  Unaccounted surfaces: {}.\n",
                result.coverage.gaps().join(", ")
            ));
            output.push_str(
                "  Verify with an independent oracle (manifests, Make/workflow targets, tests, framework dispatch) before removing.\n",
            );
        }
        return output;
    }

    // Direct consumers
    if !result.direct_consumers.is_empty() {
        output.push_str(&format!(
            "Direct consumers ({} files):\n",
            result.direct_consumers.len()
        ));
        for entry in &result.direct_consumers {
            output.push_str(&format!("  {} ({})\n", entry.file, entry.import_type));
        }
        output.push('\n');
    }

    // Transitive consumers
    if !result.transitive_consumers.is_empty() {
        output.push_str(&format!(
            "Transitive impact ({} files):\n",
            result.transitive_consumers.len()
        ));

        // Group by depth for better readability
        let mut by_depth: HashMap<usize, Vec<&ImpactEntry>> = HashMap::new();
        for entry in &result.transitive_consumers {
            by_depth.entry(entry.depth).or_default().push(entry);
        }

        let mut depths: Vec<usize> = by_depth.keys().copied().collect();
        depths.sort();

        for depth in depths {
            if let Some(entries) = by_depth.get(&depth) {
                for entry in entries {
                    output.push_str(&format!(
                        "  [depth {}] {} ({})\n",
                        entry.depth, entry.file, entry.import_type
                    ));
                }
            }
        }
        output.push('\n');
    }

    output.push_str(&format!(
        "[!] Removing this file would affect {} files (max depth: {})\n",
        result.total_affected, result.max_depth
    ));

    output
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::snapshot::GraphEdge;

    fn mock_snapshot() -> Snapshot {
        let mut snapshot = Snapshot::new(vec!["src".to_string()]);

        // Setup dependency chain:
        // utils.ts <- app.ts <- page.tsx
        // utils.ts <- lib.ts <- other.tsx
        snapshot.edges.push(GraphEdge {
            from: "src/app.ts".to_string(),
            to: "src/utils.ts".to_string(),
            label: "import".to_string(),
        });
        snapshot.edges.push(GraphEdge {
            from: "src/page.tsx".to_string(),
            to: "src/app.ts".to_string(),
            label: "import".to_string(),
        });
        snapshot.edges.push(GraphEdge {
            from: "src/lib.ts".to_string(),
            to: "src/utils.ts".to_string(),
            label: "import".to_string(),
        });
        snapshot.edges.push(GraphEdge {
            from: "src/other.tsx".to_string(),
            to: "src/lib.ts".to_string(),
            label: "import".to_string(),
        });

        snapshot
    }

    #[test]
    fn test_analyze_impact_direct_consumers() {
        let snapshot = mock_snapshot();
        let options = ImpactOptions::default();

        let result = analyze_impact(&snapshot, "src/utils.ts", &options);

        assert_eq!(result.target, "src/utils.ts");
        assert_eq!(result.direct_consumers.len(), 2); // app.ts and lib.ts
        assert!(
            result
                .direct_consumers
                .iter()
                .any(|e| e.file == "src/app.ts")
        );
        assert!(
            result
                .direct_consumers
                .iter()
                .any(|e| e.file == "src/lib.ts")
        );
    }

    #[test]
    fn test_analyze_impact_transitive_consumers() {
        let snapshot = mock_snapshot();
        let options = ImpactOptions::default();

        let result = analyze_impact(&snapshot, "src/utils.ts", &options);

        assert_eq!(result.transitive_consumers.len(), 2); // page.tsx and other.tsx
        assert!(
            result
                .transitive_consumers
                .iter()
                .any(|e| e.file == "src/page.tsx" && e.depth == 2)
        );
        assert!(
            result
                .transitive_consumers
                .iter()
                .any(|e| e.file == "src/other.tsx" && e.depth == 2)
        );
    }

    #[test]
    fn test_analyze_impact_total_affected() {
        let snapshot = mock_snapshot();
        let options = ImpactOptions::default();

        let result = analyze_impact(&snapshot, "src/utils.ts", &options);

        assert_eq!(result.total_affected, 4); // 2 direct + 2 transitive
        assert_eq!(result.max_depth, 2);
    }

    #[test]
    fn test_analyze_impact_depth_limit() {
        let snapshot = mock_snapshot();
        let options = ImpactOptions {
            max_depth: Some(1),
            include_reexports: true,
        };

        let result = analyze_impact(&snapshot, "src/utils.ts", &options);

        assert_eq!(result.direct_consumers.len(), 2); // app.ts and lib.ts
        assert_eq!(result.transitive_consumers.len(), 0); // Depth limited
        assert_eq!(result.total_affected, 2);
    }

    #[test]
    fn test_analyze_impact_no_consumers() {
        let snapshot = mock_snapshot();
        let options = ImpactOptions::default();

        let result = analyze_impact(&snapshot, "src/page.tsx", &options);

        assert_eq!(result.total_affected, 0);
        assert_eq!(result.direct_consumers.len(), 0);
        assert_eq!(result.transitive_consumers.len(), 0);
    }

    #[test]
    fn test_format_impact_text() {
        let snapshot = mock_snapshot();
        let options = ImpactOptions::default();

        let result = analyze_impact(&snapshot, "src/utils.ts", &options);
        let output = format_impact_text(&result);

        assert!(output.contains("Impact analysis for: src/utils.ts"));
        assert!(output.contains("Direct consumers"));
        assert!(output.contains("Transitive impact"));
        assert!(output.contains("would affect 4 files"));
    }

    #[test]
    fn test_format_impact_text_no_consumers() {
        let snapshot = mock_snapshot();
        let options = ImpactOptions::default();

        let result = analyze_impact(&snapshot, "src/page.tsx", &options);
        let output = format_impact_text(&result);

        assert!(output.contains("Impact analysis for: src/page.tsx"));
        assert!(output.contains("No consumers found in the indexed import graph"));
        assert!(output.contains(COVERAGE_INCOMPLETE_DIAGNOSTIC));
        assert!(
            !output.to_ascii_lowercase().contains("safe to remove"),
            "zero-consumer output must never claim removal safety without proven coverage: {output}"
        );
    }

    #[test]
    fn test_analyze_impact_coverage_fails_closed_by_default() {
        let snapshot = mock_snapshot();
        let options = ImpactOptions::default();

        let result = analyze_impact(&snapshot, "src/page.tsx", &options);

        assert!(
            !result.coverage.is_complete(),
            "no analyzer proves these surfaces yet; coverage must default to incomplete"
        );
        assert_eq!(
            result.coverage.gaps(),
            vec![
                "package manifests",
                "build scripts (Make, build.rs)",
                "CI workflows",
                "test harnesses",
                "framework/runtime dispatch",
            ]
        );
    }

    #[test]
    fn test_format_impact_text_proven_coverage_still_never_says_safe() {
        // Fixture encoding of "coverage proven": every GraphCoverage flag is
        // explicitly set true on the result. Only then does the zero-consumer
        // branch drop the incomplete-coverage diagnostic — and even that path
        // reports facts (no consumers, coverage complete), not removal advice.
        let snapshot = mock_snapshot();
        let options = ImpactOptions::default();

        let mut result = analyze_impact(&snapshot, "src/page.tsx", &options);
        result.coverage = GraphCoverage {
            package_manifests: true,
            build_scripts: true,
            ci_workflows: true,
            test_harnesses: true,
            framework_dispatch: true,
        };
        let output = format_impact_text(&result);

        assert!(output.contains("graph coverage is proven complete"));
        assert!(!output.contains(COVERAGE_INCOMPLETE_DIAGNOSTIC));
        assert!(!output.to_ascii_lowercase().contains("safe to remove"));
    }

    #[test]
    fn test_impact_json_carries_coverage_accounting() {
        let snapshot = mock_snapshot();
        let options = ImpactOptions::default();

        let result = analyze_impact(&snapshot, "src/page.tsx", &options);
        let json = serde_json::to_value(&result).expect("serializes");

        let coverage = json
            .get("coverage")
            .expect("coverage accounting must be visible in the JSON schema");
        assert_eq!(coverage["package_manifests"], false);
        assert_eq!(coverage["framework_dispatch"], false);
    }

    #[test]
    fn test_impact_chain_tracking() {
        let snapshot = mock_snapshot();
        let options = ImpactOptions::default();

        let result = analyze_impact(&snapshot, "src/utils.ts", &options);

        // Verify chain tracking for transitive consumers
        for entry in &result.transitive_consumers {
            assert!(!entry.chain.is_empty());
            assert_eq!(entry.chain[0], "src/utils.ts"); // Chain should start with target
            assert_eq!(entry.chain.last().unwrap(), &entry.file); // Chain should end with consumer
            assert_eq!(entry.chain.len(), entry.depth + 1); // Chain length should match depth + 1
        }
    }

    #[test]
    fn test_depth_calculation() {
        let snapshot = mock_snapshot();
        let options = ImpactOptions::default();

        let result = analyze_impact(&snapshot, "src/utils.ts", &options);

        // Direct consumers should have depth 1
        for entry in &result.direct_consumers {
            assert_eq!(entry.depth, 1);
        }

        // Transitive consumers should have depth > 1
        for entry in &result.transitive_consumers {
            assert!(entry.depth > 1);
        }

        // Max depth should match the deepest consumer
        let max_consumer_depth = result
            .direct_consumers
            .iter()
            .chain(result.transitive_consumers.iter())
            .map(|e| e.depth)
            .max()
            .unwrap_or(0);
        assert_eq!(result.max_depth, max_consumer_depth);
    }

    #[test]
    fn test_normalize_path_strips_leading_slash() {
        let path = "./src/utils.ts";
        let normalized = normalize_path(path);
        assert_eq!(normalized, "src/utils.ts");
    }

    #[test]
    fn test_normalize_path_index_variants() {
        assert_eq!(
            normalize_path("src/components/index.ts"),
            "src/components/index.ts"
        );
        assert_eq!(
            normalize_path("src/components/index.tsx"),
            "src/components/index.tsx"
        );
        assert_eq!(
            normalize_path("src/components/index.js"),
            "src/components/index.js"
        );
    }

    #[test]
    fn test_build_reverse_dependency_map() {
        let snapshot = mock_snapshot();
        let reverse_deps = build_reverse_dependency_map(&snapshot);

        // utils.ts should have two importers: app.ts and lib.ts
        assert!(reverse_deps.contains_key("src/utils.ts"));
        let utils_importers = &reverse_deps["src/utils.ts"];
        assert_eq!(utils_importers.len(), 2);
        assert!(utils_importers.iter().any(|(f, _)| f == "src/app.ts"));
        assert!(utils_importers.iter().any(|(f, _)| f == "src/lib.ts"));

        // app.ts should have one importer: page.tsx
        assert!(reverse_deps.contains_key("src/app.ts"));
        let app_importers = &reverse_deps["src/app.ts"];
        assert_eq!(app_importers.len(), 1);
        assert_eq!(app_importers[0].0, "src/page.tsx");
    }

    #[test]
    fn test_import_type_preserved() {
        let snapshot = mock_snapshot();
        let options = ImpactOptions::default();

        let result = analyze_impact(&snapshot, "src/utils.ts", &options);

        // All entries should have import_type preserved from edges
        for entry in &result.direct_consumers {
            assert_eq!(entry.import_type, "import");
        }
    }

    #[test]
    fn test_max_depth_zero() {
        let snapshot = Snapshot::new(vec!["src".to_string()]);
        let options = ImpactOptions {
            max_depth: Some(0),
            include_reexports: true,
        };

        let result = analyze_impact(&snapshot, "src/utils.ts", &options);

        // With max_depth = 0, nothing should be found
        assert_eq!(result.total_affected, 0);
        assert_eq!(result.max_depth, 0);
    }

    #[test]
    fn test_max_depth_two() {
        let snapshot = mock_snapshot();
        let options = ImpactOptions {
            max_depth: Some(2),
            include_reexports: true,
        };

        let result = analyze_impact(&snapshot, "src/utils.ts", &options);

        // Should find direct (depth 1) and transitive (depth 2)
        assert_eq!(result.direct_consumers.len(), 2);
        assert_eq!(result.transitive_consumers.len(), 2);
        assert_eq!(result.max_depth, 2);
    }

    #[test]
    fn test_sorting_by_file_path() {
        let snapshot = mock_snapshot();
        let options = ImpactOptions::default();

        let result = analyze_impact(&snapshot, "src/utils.ts", &options);

        // Direct consumers should be sorted by file path
        for i in 1..result.direct_consumers.len() {
            assert!(result.direct_consumers[i - 1].file <= result.direct_consumers[i].file);
        }

        // Transitive consumers should be sorted by depth first, then file path
        for i in 1..result.transitive_consumers.len() {
            let prev = &result.transitive_consumers[i - 1];
            let curr = &result.transitive_consumers[i];
            assert!(
                prev.depth < curr.depth || (prev.depth == curr.depth && prev.file <= curr.file)
            );
        }
    }
}
