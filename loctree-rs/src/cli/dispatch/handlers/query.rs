//! Query-related command handlers
//!
//! Handles: query, jq_query

use super::super::super::command::{
    BodyOptions, FindOptions, JqQueryOptions, QueryKind, QueryOptions,
};
use super::super::{
    DispatchResult, GlobalOptions, load_or_create_query_snapshot_for_roots, load_or_create_snapshot,
};

const DEFAULT_WHERE_SYMBOL_LIMIT: usize = 25;

pub fn handle_find_where_symbol_command(
    opts: &FindOptions,
    global: &GlobalOptions,
) -> DispatchResult {
    use crate::query::query_where_symbol;

    let target = opts
        .query
        .clone()
        .or_else(|| opts.queries.first().cloned())
        .unwrap_or_default();

    if target.is_empty() {
        eprintln!("Error: Query cannot be empty");
        return DispatchResult::Exit(1);
    }

    let roots = opts.scan_roots();
    let query_global = query_global_options(global);
    let snapshot = match load_or_create_query_snapshot_for_roots(&roots, &query_global) {
        Ok(s) => s,
        Err(e) => {
            eprintln!("[loct][error] {}", e);
            return DispatchResult::Exit(1);
        }
    };

    let result = query_where_symbol(&snapshot, &target).bounded(if opts.all {
        None
    } else {
        Some(opts.limit.unwrap_or(DEFAULT_WHERE_SYMBOL_LIMIT))
    });

    // Output results
    if global.json {
        match serde_json::to_string_pretty(&result) {
            Ok(json) => println!("{}", json),
            Err(e) => {
                eprintln!("[loct][error] Failed to serialize results: {}", e);
                return DispatchResult::Exit(1);
            }
        }
    } else {
        println!("where-symbol '{}':", result.target);
        if result.results.is_empty() {
            println!("  (no results)");
        } else {
            for m in &result.results {
                if let Some(line) = m.line {
                    print!("  {}:{}", m.file, line);
                } else {
                    print!("  {}", m.file);
                }
                if let Some(ref ctx) = m.context {
                    print!(" - {}", ctx);
                }
                println!();
            }
        }
        print_query_truncation(&result);
    }

    DispatchResult::Exit(0)
}

/// Handle the `body` command - bounded symbol source retrieval.
pub fn handle_body_command(opts: &BodyOptions, global: &GlobalOptions) -> DispatchResult {
    use crate::body::query_symbol_body;

    let roots = vec![std::path::PathBuf::from(".")];
    let query_global = query_global_options(global);
    let snapshot = match load_or_create_query_snapshot_for_roots(&roots, &query_global) {
        Ok(s) => s,
        Err(e) => {
            eprintln!("[loct][error] {}", e);
            return DispatchResult::Exit(1);
        }
    };

    let unfiltered = query_symbol_body(&snapshot, &opts.symbol, opts.line_cap);
    let had_candidates = !unfiltered.bodies.is_empty();
    let result = unfiltered.filtered_to_file(opts.file.as_deref());

    if global.json {
        match serde_json::to_string_pretty(&result) {
            Ok(json) => println!("{}", json),
            Err(e) => {
                eprintln!("[loct][error] Failed to serialize results: {}", e);
                return DispatchResult::Exit(1);
            }
        }
        return DispatchResult::Exit(if result.bodies.is_empty() { 1 } else { 0 });
    }

    if result.bodies.is_empty() {
        if let Some(file) = opts.file.as_deref().filter(|_| had_candidates) {
            println!(
                "body '{}': no definition in '{}' (definitions exist elsewhere).",
                result.symbol, file
            );
            println!(
                "  hint: drop --file or run `loct body {}` to list all candidates.",
                result.symbol
            );
        } else {
            println!("body '{}': (no source body found)", result.symbol);
            println!(
                "  hint: run `loct query where-symbol {}` to locate the symbol first.",
                result.symbol
            );
        }
        return DispatchResult::Exit(1);
    }

    if result.bodies.len() > 1 {
        println!(
            "body '{}': multiple exact definitions found; choose one:",
            result.symbol
        );
        for body in &result.bodies {
            println!(
                "  {}:{}-{} [{}]",
                body.file, body.start_line, body.end_line, body.language
            );
        }
        println!(
            "  hint: qualify with --file <path>, or use a qualified symbol, e.g. Type::method."
        );
        return DispatchResult::Exit(1);
    }

    for body in &result.bodies {
        println!(
            "── {} [{}] {}:{}-{} ──",
            body.symbol, body.language, body.file, body.start_line, body.end_line
        );
        println!("{}", body.source);
        if body.extent == crate::body::EXTENT_WINDOW {
            println!(
                "  … extent unproven: showing a fixed {}-line window; the body's real end \
                 was not confirmed (truncated: true).",
                body.end_line - body.start_line + 1
            );
        } else if body.truncated {
            println!(
                "  … truncated: showing {} of {} lines (cap {}). Use --max-lines to widen.",
                body.end_line - body.start_line + 1,
                body.total_lines,
                body.line_cap
            );
        }
        println!();
    }

    DispatchResult::Exit(0)
}

/// Handle the query command directly
pub fn handle_query_command(opts: &QueryOptions, global: &GlobalOptions) -> DispatchResult {
    use crate::query::{
        SwiftTypeResolutionStatus, classify_swift_type_references, query_component_of,
        query_where_symbol, query_who_imports,
    };

    // Load snapshot (auto-scan if missing). Honor --root/--project from find/query.
    let roots = opts.scan_roots();
    let query_global = query_global_options(global);
    let snapshot = match load_or_create_query_snapshot_for_roots(&roots, &query_global) {
        Ok(s) => s,
        Err(e) => {
            eprintln!("[loct][error] {}", e);
            return DispatchResult::Exit(1);
        }
    };

    if matches!(opts.kind, QueryKind::SwiftTypes) {
        let source = match std::fs::read_to_string(&opts.target) {
            Ok(source) => source,
            Err(e) => {
                eprintln!("[loct][error] Failed to read {}: {}", opts.target, e);
                return DispatchResult::Exit(1);
            }
        };
        let result = classify_swift_type_references(&snapshot, &opts.target, &source);

        if global.json {
            match serde_json::to_string_pretty(&result) {
                Ok(json) => println!("{}", json),
                Err(e) => {
                    eprintln!("[loct][error] Failed to serialize results: {}", e);
                    return DispatchResult::Exit(1);
                }
            }
        } else {
            println!("swift-types '{}':", result.target);
            if result.references.is_empty() {
                println!("  (no type-position references)");
            } else {
                for reference in &result.references {
                    match reference.status {
                        SwiftTypeResolutionStatus::Resolved => {
                            if let Some(definition) = &reference.definition {
                                let line = definition
                                    .line
                                    .map(|line| format!(":{}", line))
                                    .unwrap_or_default();
                                print!(
                                    "  {}: RESOLVED -> {}{}",
                                    reference.name, definition.file, line
                                );
                                if let Some(ctx) = &definition.context {
                                    print!(" - {}", ctx);
                                }
                                println!(" (ref line {})", reference.line);
                            } else {
                                println!(
                                    "  {}: RESOLVED (ref line {})",
                                    reference.name, reference.line
                                );
                            }
                        }
                        SwiftTypeResolutionStatus::External => {
                            println!(
                                "  {}: EXTERNAL (Swift/Foundation/SwiftUI allowlist, ref line {})",
                                reference.name, reference.line
                            );
                        }
                        SwiftTypeResolutionStatus::Unresolved => {
                            let symbol_id = reference.symbol_id.as_deref().unwrap_or("");
                            println!(
                                "  {}: UNRESOLVED {} (ref line {})",
                                reference.name, symbol_id, reference.line
                            );
                        }
                    }
                }
            }
        }

        return DispatchResult::Exit(0);
    }

    // Execute the query
    let result = match opts.kind {
        QueryKind::WhoImports => query_who_imports(&snapshot, &opts.target),
        QueryKind::WhereSymbol => query_where_symbol(&snapshot, &opts.target),
        QueryKind::ComponentOf => query_component_of(&snapshot, &opts.target),
        QueryKind::SwiftTypes => unreachable!("swift-types handled before QueryResult path"),
    };
    let result = if matches!(opts.kind, QueryKind::WhereSymbol) {
        result.bounded(if opts.all {
            None
        } else {
            Some(opts.limit.unwrap_or(DEFAULT_WHERE_SYMBOL_LIMIT))
        })
    } else {
        result
    };

    // Output results
    if global.json {
        // JSON output
        match serde_json::to_string_pretty(&result) {
            Ok(json) => println!("{}", json),
            Err(e) => {
                eprintln!("[loct][error] Failed to serialize results: {}", e);
                return DispatchResult::Exit(1);
            }
        }
    } else {
        // Human-readable output
        println!("{} '{}':", result.kind, result.target);
        if result.results.is_empty() {
            println!("  (no results)");
        } else {
            for m in &result.results {
                if let Some(line) = m.line {
                    print!("  {}:{}", m.file, line);
                } else {
                    print!("  {}", m.file);
                }
                if let Some(ref ctx) = m.context {
                    print!(" - {}", ctx);
                }
                println!();
            }
        }
        print_query_truncation(&result);
    }

    DispatchResult::Exit(0)
}

fn print_query_truncation(result: &crate::query::QueryResult) {
    println!(
        "  accounting: total={}, emitted={}, offset={}, truncated={}, has_more={}",
        result.total, result.emitted, result.offset, result.truncated, result.has_more
    );
    println!("  {}", result.universe.summary_line());
    if result.truncated {
        println!(
            "  … showing {} of {} exact definitions; qualify the symbol, pass --limit <N>, or use --all.",
            result.results.len(),
            result.total
        );
    }
}

fn query_global_options(global: &GlobalOptions) -> GlobalOptions {
    let mut scoped = global.clone();
    if !scoped.verbose {
        scoped.quiet = true;
    }
    scoped
}

/// Handle the jq query command - execute jaq filter on snapshot
pub fn handle_jq_query_command(opts: &JqQueryOptions, global: &GlobalOptions) -> DispatchResult {
    use crate::jaq_query::{JaqExecutor, format_output};
    use std::path::Path;

    // Load snapshot (auto-scan if missing)
    // If user specified explicit snapshot_path, try that first
    let snapshot = if let Some(ref explicit_path) = opts.snapshot_path {
        // User specified explicit path - use it directly without auto-create
        use crate::snapshot::Snapshot;
        let snapshot_path = match Snapshot::find_latest_snapshot(Some(explicit_path.as_ref())) {
            Ok(p) => p,
            Err(e) => {
                eprintln!("[loct][error] {}", e);
                eprintln!("[loct][hint] Specified snapshot path not found.");
                return DispatchResult::Exit(1);
            }
        };
        match std::fs::read_to_string(&snapshot_path)
            .map_err(std::io::Error::other)
            .and_then(|content| {
                serde_json::from_str::<crate::snapshot::Snapshot>(&content)
                    .map_err(|e| std::io::Error::new(std::io::ErrorKind::InvalidData, e))
            }) {
            Ok(s) => s,
            Err(e) => {
                eprintln!("[loct][error] Failed to load snapshot: {}", e);
                return DispatchResult::Exit(1);
            }
        }
    } else {
        // No explicit path - use load_or_create_snapshot for auto-scan
        match load_or_create_snapshot(Path::new("."), global) {
            Ok(s) => s,
            Err(e) => {
                eprintln!("[loct][error] {}", e);
                return DispatchResult::Exit(1);
            }
        }
    };

    // Convert snapshot to JSON value for jaq
    let snapshot_json = match serde_json::to_value(&snapshot) {
        Ok(v) => v,
        Err(e) => {
            eprintln!("[loct][error] Failed to serialize snapshot: {}", e);
            return DispatchResult::Exit(1);
        }
    };

    // Execute the jaq filter
    let executor = JaqExecutor::new();
    let results = match executor.execute(
        &opts.filter,
        &snapshot_json,
        &opts.string_args,
        &opts.json_args,
    ) {
        Ok(r) => r,
        Err(e) => {
            eprintln!("[loct][error] Filter execution failed: {}", e);
            return DispatchResult::Exit(1);
        }
    };

    // Output results
    for result in &results {
        let output = format_output(result, opts.raw_output, opts.compact_output);
        println!("{}", output);
    }

    // Exit status mode: exit 1 if no results or all results are false/null
    if opts.exit_status {
        if results.is_empty() {
            return DispatchResult::Exit(1);
        }

        // Check if all results are false or null
        let all_false_or_null = results
            .iter()
            .all(|v| v.is_null() || (v.as_bool().is_some() && !v.as_bool().unwrap()));

        if all_false_or_null {
            return DispatchResult::Exit(1);
        }
    }

    DispatchResult::Exit(0)
}

#[cfg(test)]
mod project_root_tests {
    use super::*;
    use crate::cli::command::{FindOptions, QueryKind, QueryOptions};
    use crate::query::query_where_symbol;
    use crate::snapshot::test_env;
    use std::fs;
    use std::path::PathBuf;
    use std::process::Command as Proc;

    fn git_init_with_file(root: &std::path::Path, rel: &str, body: &str) {
        fs::create_dir_all(root.join(PathBuf::from(rel).parent().unwrap_or(root))).unwrap();
        fs::write(root.join(rel), body).unwrap();
        let run = |args: &[&str]| {
            let out = Proc::new("git")
                .args(args)
                .current_dir(root)
                .output()
                .expect("git");
            assert!(
                out.status.success(),
                "git {args:?}: {}",
                String::from_utf8_lossy(&out.stderr)
            );
        };
        run(&["init"]);
        run(&["config", "user.email", "t@t.com"]);
        run(&["config", "user.name", "t"]);
        run(&["add", "."]);
        run(&["commit", "-m", "seed"]);
    }

    /// End-to-end: --project scopes where-symbol / who-imports load path so a
    /// unique symbol in sibling-a is found under that project and not under
    /// sibling-b (skeptic cluster B gap).
    // `isolated_cache()` swaps two process-global env vars (LOCT_CACHE_DIR,
    // LOCT_ALLOW_NON_GIT_ROOT), so every holder must be serialized — a
    // concurrent test swapping LOCT_CACHE_DIR between this test's scan and its
    // load makes the snapshot land in one cache and be looked up in another.
    // Every other isolated_cache() caller in the crate is already #[serial];
    // this one was not, which is why it failed only on the Linux runner.
    #[test]
    #[serial_test::serial]
    fn where_symbol_and_who_imports_honor_explicit_project_root() {
        let (_cache_dir, _cache_env) = test_env::isolated_cache();
        let base = tempfile::tempdir().unwrap();
        let sibling_a = base.path().join("sibling-a");
        let sibling_b = base.path().join("sibling-b");
        fs::create_dir_all(sibling_a.join("src")).unwrap();
        fs::create_dir_all(sibling_b.join("src")).unwrap();

        git_init_with_file(&sibling_a, "src/lib.rs", "pub fn UNIQUE_SYM_AAA() {}\n");
        git_init_with_file(&sibling_b, "src/lib.rs", "pub fn OTHER_ONLY_SYM() {}\n");

        let global = GlobalOptions {
            quiet: true,
            force_non_git: false,
            ..Default::default()
        };

        // where-symbol --project sibling-a finds the unique symbol
        let snap_a = load_or_create_query_snapshot_for_roots(
            std::slice::from_ref(&sibling_a),
            &query_global_options(&global),
        )
        .expect("scan sibling-a");
        let hit_a = query_where_symbol(&snap_a, "UNIQUE_SYM_AAA");
        assert!(
            !hit_a.results.is_empty(),
            "where-symbol under sibling-a must find UNIQUE_SYM_AAA; got {hit_a:?}"
        );

        // Same symbol under sibling-b: not present
        let snap_b = load_or_create_query_snapshot_for_roots(
            std::slice::from_ref(&sibling_b),
            &query_global_options(&global),
        )
        .expect("scan sibling-b");
        let hit_b = query_where_symbol(&snap_b, "UNIQUE_SYM_AAA");
        assert!(
            hit_b.results.is_empty(),
            "where-symbol under sibling-b must not invent UNIQUE_SYM_AAA; got {hit_b:?}"
        );

        // Wiring: FindOptions / QueryOptions scan_roots carry project
        let find_opts = FindOptions {
            queries: vec!["UNIQUE_SYM_AAA".into()],
            where_symbol: true,
            roots: vec![sibling_a.clone()],
            ..Default::default()
        };
        assert_eq!(find_opts.scan_roots(), vec![sibling_a.clone()]);

        let query_opts = QueryOptions {
            kind: QueryKind::WhoImports,
            target: "src/lib.rs".into(),
            limit: None,
            all: false,
            roots: vec![sibling_a.clone()],
        };
        assert_eq!(query_opts.scan_roots(), vec![sibling_a]);

        // Handler entry (where-symbol) returns 0 when scoped correctly
        let result = handle_find_where_symbol_command(&find_opts, &global);
        assert!(matches!(result, DispatchResult::Exit(0)));
    }
}
