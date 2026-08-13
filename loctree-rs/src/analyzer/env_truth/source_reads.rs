//! Named environment reads discovered directly from source files.
//!
//! This is deliberately an inventory, not a value reader: only variable names,
//! source locations, and access classes leave this module. Dynamic lookups stay
//! omitted rather than being guessed.

use once_cell::sync::Lazy;
use regex::Regex;
use std::path::{Component, Path};

use crate::snapshot::Snapshot;

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct SourceEnvRead {
    pub name: String,
    pub file: String,
    pub line: u32,
    pub access_kind: String,
}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub(crate) struct SourceEnvInventory {
    pub reads: Vec<SourceEnvRead>,
    pub classes: Vec<String>,
    pub files_scanned: usize,
}

static RUST_ENV_READ: Lazy<Regex> = Lazy::new(|| {
    Regex::new(r#"(?:std::)?env::(?P<kind>var|var_os)\s*\(\s*\"(?P<name>[A-Z][A-Z0-9_]*)\""#)
        .expect("valid Rust environment-read regex")
});

static SWIFT_ENV_READ: Lazy<Regex> = Lazy::new(|| {
    Regex::new(
        r#"(?:ProcessInfo\s*\.\s*processInfo\s*\.\s*)?environment\s*\[\s*\"(?P<name>[A-Z][A-Z0-9_]*)\"\s*\]"#,
    )
    .expect("valid Swift environment-read regex")
});

static JS_ENV_DOT_READ: Lazy<Regex> = Lazy::new(|| {
    Regex::new(r#"process\s*\.\s*env\s*\.\s*(?P<name>[A-Z][A-Z0-9_]*)"#)
        .expect("valid JS process.env dot-read regex")
});

static JS_ENV_INDEX_READ: Lazy<Regex> = Lazy::new(|| {
    Regex::new(r#"process\s*\.\s*env\s*\[\s*['\"](?P<name>[A-Z][A-Z0-9_]*)['\"]\s*\]"#)
        .expect("valid JS process.env index-read regex")
});

pub(crate) fn collect_source_env_reads(
    project_root: &Path,
    snapshot: &Snapshot,
) -> SourceEnvInventory {
    let mut inventory = SourceEnvInventory {
        classes: vec![
            "javascript_process_env".to_string(),
            "rust_std_env".to_string(),
            "swift_process_environment".to_string(),
        ],
        ..SourceEnvInventory::default()
    };

    for file in &snapshot.files {
        let class = match file.language.as_str() {
            "rs" | "rust" => "rust",
            "swift" => "swift",
            "js" | "jsx" | "ts" | "tsx" | "javascript" | "typescript" => "javascript",
            _ => continue,
        };
        let Some(path) = safe_project_path(project_root, &file.path) else {
            continue;
        };
        let Ok(source) = std::fs::read_to_string(path) else {
            continue;
        };
        inventory.files_scanned += 1;
        collect_file_reads(class, &file.path, &source, &mut inventory.reads);
    }

    inventory.reads.sort_by(|a, b| {
        a.name
            .cmp(&b.name)
            .then_with(|| a.file.cmp(&b.file))
            .then_with(|| a.line.cmp(&b.line))
            .then_with(|| a.access_kind.cmp(&b.access_kind))
    });
    inventory.reads.dedup();
    inventory
}

fn safe_project_path(project_root: &Path, relative: &str) -> Option<std::path::PathBuf> {
    let candidate = Path::new(relative);
    if candidate.is_absolute()
        || candidate.components().any(|part| {
            matches!(
                part,
                Component::ParentDir | Component::RootDir | Component::Prefix(_)
            )
        })
    {
        return None;
    }
    Some(project_root.join(candidate))
}

fn collect_file_reads(class: &str, file: &str, source: &str, reads: &mut Vec<SourceEnvRead>) {
    for (index, line) in source.lines().enumerate() {
        let line_number = (index + 1) as u32;
        match class {
            "rust" => push_captures(
                &RUST_ENV_READ,
                line,
                file,
                line_number,
                |captures| {
                    let kind = captures.name("kind")?.as_str();
                    Some(format!("std::env::{kind}"))
                },
                reads,
            ),
            "swift" => push_captures(
                &SWIFT_ENV_READ,
                line,
                file,
                line_number,
                |_| Some("ProcessInfo.environment[]".to_string()),
                reads,
            ),
            "javascript" => {
                push_captures(
                    &JS_ENV_DOT_READ,
                    line,
                    file,
                    line_number,
                    |_| Some("process.env.NAME".to_string()),
                    reads,
                );
                push_captures(
                    &JS_ENV_INDEX_READ,
                    line,
                    file,
                    line_number,
                    |_| Some("process.env[]".to_string()),
                    reads,
                );
            }
            _ => {}
        }
    }
}

fn push_captures(
    regex: &Regex,
    line: &str,
    file: &str,
    line_number: u32,
    access_kind: impl Fn(&regex::Captures<'_>) -> Option<String>,
    reads: &mut Vec<SourceEnvRead>,
) {
    for captures in regex.captures_iter(line) {
        let Some(name) = captures.name("name").map(|value| value.as_str()) else {
            continue;
        };
        let Some(access_kind) = access_kind(&captures) else {
            continue;
        };
        reads.push(SourceEnvRead {
            name: name.to_string(),
            file: file.to_string(),
            line: line_number,
            access_kind,
        });
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn extracts_names_from_supported_source_classes_without_values() {
        let mut reads = Vec::new();
        collect_file_reads(
            "rust",
            "src/main.rs",
            "let _ = std::env::var(\"RUST_TOKEN\");\nlet _ = env::var_os(\"CACHE_ROOT\");",
            &mut reads,
        );
        collect_file_reads(
            "swift",
            "Sources/App.swift",
            "let token = ProcessInfo.processInfo.environment[\"SWIFT_TOKEN\"]",
            &mut reads,
        );
        collect_file_reads(
            "javascript",
            "src/config.ts",
            "const a = process.env.API_URL; const b = process.env['BUILD_ID'];",
            &mut reads,
        );

        let names: Vec<&str> = reads.iter().map(|read| read.name.as_str()).collect();
        assert_eq!(
            names,
            vec![
                "RUST_TOKEN",
                "CACHE_ROOT",
                "SWIFT_TOKEN",
                "API_URL",
                "BUILD_ID"
            ]
        );
        assert!(reads.iter().all(|read| !read.access_kind.contains("TOKEN")));
    }

    #[test]
    fn rejects_paths_outside_the_project() {
        assert!(safe_project_path(Path::new("/tmp/project"), "../secret.rs").is_none());
        assert!(safe_project_path(Path::new("/tmp/project"), "/tmp/secret.rs").is_none());
        assert!(safe_project_path(Path::new("/tmp/project"), "src/main.rs").is_some());
    }
}
