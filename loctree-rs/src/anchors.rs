//! Deterministic `loctree.anchors.v1` catalog builder.
//!
//! Core domain logic extracted from the `loct anchors` CLI handler so that
//! non-CLI consumers (the ContextPack composer's overlay freshness check in
//! [`crate::pack`], the LSP) can compute the current
//! `anchor_catalog_revision` without importing CLI handler modules — the
//! `pack_does_not_import_cli_handlers` contract. The CLI handler at
//! `cli::dispatch::handlers::anchors` stays the emission surface and
//! re-exports these types.

use std::collections::{BTreeMap, BTreeSet};
use std::path::Path;

use git2::{Delta, DiffFindOptions, Repository};
use serde::Serialize;
use sha2::{Digest, Sha256};

use crate::snapshot::Snapshot;

const ANCHORS_SCHEMA: &str = "loctree.anchors.v1";
const ANCHOR_ID_PREFIX: &str = "anc1:";
const CATALOG_REVISION_PREFIX: &str = "acr1:";
const SIGNATURE_HASH_PREFIX: &str = "sig1:";

#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
pub struct AnchorAlias {
    pub kind: &'static str,
    pub value: String,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
pub struct AnchorRecord {
    pub anchor_id: String,
    pub normalized_path: String,
    pub language: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub qualified_symbol: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub signature_hash: Option<String>,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub aliases: Vec<AnchorAlias>,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
pub struct AnchorCatalog {
    pub schema: &'static str,
    pub repo_id: String,
    pub snapshot_commit: String,
    pub anchor_catalog_revision: String,
    pub producer_version: &'static str,
    pub anchors: Vec<AnchorRecord>,
}

pub fn build_anchor_catalog(snapshot: &Snapshot, root: &Path) -> Result<AnchorCatalog, String> {
    let repo_id = repository_id(snapshot, root);
    let snapshot_commit = snapshot
        .metadata
        .git_commit
        .as_deref()
        .filter(|commit| is_hex_commit(commit))
        .ok_or_else(|| {
            "snapshot has no valid git commit (expected 7-40 lowercase hex)".to_string()
        })?
        .to_string();
    let rename_aliases = path_rename_aliases(root);
    let graph_symbols = graph_symbols_by_file(snapshot);
    let mut anchors = Vec::with_capacity(snapshot.files.len() * 2);

    for file in sorted_snapshot_files(snapshot) {
        let normalized_path = normalize_path(&file.path)?;
        let language = normalize_language(&file.language);
        let aliases = aliases_for_path(&normalized_path, &rename_aliases);
        anchors.push(AnchorRecord {
            anchor_id: anchor_id(&repo_id, &normalized_path, None),
            normalized_path: normalized_path.clone(),
            language: language.clone(),
            qualified_symbol: None,
            signature_hash: None,
            aliases: aliases.clone(),
        });

        let mut emitted_symbols = BTreeSet::new();
        for export in &file.exports {
            if !valid_qualified_symbol(&export.name) || !emitted_symbols.insert(export.name.clone())
            {
                continue;
            }
            let qualified_symbol = export.name.trim().to_string();
            let normalized_signature = normalized_export_signature(
                &language,
                &qualified_symbol,
                &export.kind,
                &export.params,
            );
            anchors.push(symbol_anchor(
                &repo_id,
                &normalized_path,
                &language,
                qualified_symbol,
                normalized_signature,
                aliases.clone(),
            ));
        }

        if let Some(symbols) = graph_symbols.get(&normalized_path) {
            for symbol in symbols {
                if !emitted_symbols.insert(symbol.qualified_symbol.clone()) {
                    continue;
                }
                anchors.push(symbol_anchor(
                    &repo_id,
                    &normalized_path,
                    &language,
                    symbol.qualified_symbol.clone(),
                    symbol.signature.clone(),
                    aliases.clone(),
                ));
            }
        }
    }

    anchors.sort_by(|left, right| {
        (
            &left.normalized_path,
            &left.qualified_symbol,
            &left.anchor_id,
        )
            .cmp(&(
                &right.normalized_path,
                &right.qualified_symbol,
                &right.anchor_id,
            ))
    });
    anchors.dedup_by(|left, right| left.anchor_id == right.anchor_id);

    let producer_version = crate::BUILD_VERSION;
    let revision_payload = serde_json::to_vec(&(producer_version, &anchors))
        .map_err(|error| format!("catalog revision serialization failed: {error}"))?;
    let anchor_catalog_revision =
        format!("{CATALOG_REVISION_PREFIX}{}", hex_sha256(&revision_payload));

    Ok(AnchorCatalog {
        schema: ANCHORS_SCHEMA,
        repo_id,
        snapshot_commit,
        anchor_catalog_revision,
        producer_version,
        anchors,
    })
}

#[derive(Clone, Debug)]
struct GraphSymbol {
    qualified_symbol: String,
    signature: String,
}

fn graph_symbols_by_file(snapshot: &Snapshot) -> BTreeMap<String, Vec<GraphSymbol>> {
    let mut by_file: BTreeMap<String, Vec<GraphSymbol>> = BTreeMap::new();
    let Some(graph) = &snapshot.symbol_graph else {
        return by_file;
    };

    for symbol in &graph.symbols {
        let Some(file) = &symbol.file else {
            continue;
        };
        if matches!(
            symbol.visibility,
            Some(crate::symbols::SymbolVisibility::Private)
                | Some(crate::symbols::SymbolVisibility::FilePrivate)
        ) {
            continue;
        }
        let path = file.to_string_lossy().replace('\\', "/");
        let Ok(path) = normalize_path(&path) else {
            continue;
        };
        let qualified_symbol = symbol
            .qualified_name
            .as_deref()
            .unwrap_or(&symbol.name)
            .trim()
            .to_string();
        if !valid_qualified_symbol(&qualified_symbol) {
            continue;
        }
        let signature = symbol
            .signature
            .as_deref()
            .map(normalize_signature)
            .filter(|signature| !signature.is_empty())
            .unwrap_or_else(|| format!("{:?}:{qualified_symbol}", symbol.kind).to_lowercase());
        by_file.entry(path).or_default().push(GraphSymbol {
            qualified_symbol,
            signature,
        });
    }
    for symbols in by_file.values_mut() {
        symbols.sort_by(|left, right| left.qualified_symbol.cmp(&right.qualified_symbol));
    }
    by_file
}

fn symbol_anchor(
    repo_id: &str,
    normalized_path: &str,
    language: &str,
    qualified_symbol: String,
    signature: String,
    aliases: Vec<AnchorAlias>,
) -> AnchorRecord {
    AnchorRecord {
        anchor_id: anchor_id(repo_id, normalized_path, Some(&qualified_symbol)),
        normalized_path: normalized_path.to_string(),
        language: language.to_string(),
        signature_hash: Some(format!(
            "{SIGNATURE_HASH_PREFIX}{}",
            hex_sha256(signature.as_bytes())
        )),
        qualified_symbol: Some(qualified_symbol),
        aliases,
    }
}

fn sorted_snapshot_files(snapshot: &Snapshot) -> Vec<&crate::types::FileAnalysis> {
    let mut files: Vec<_> = snapshot.files.iter().collect();
    files.sort_by(|left, right| left.path.cmp(&right.path));
    files
}

fn repository_id(snapshot: &Snapshot, root: &Path) -> String {
    snapshot
        .metadata
        .git_owner_repo
        .as_deref()
        .or(snapshot.metadata.git_repo.as_deref())
        .map(sanitize_repo_id)
        .filter(|value| !value.is_empty())
        .unwrap_or_else(|| {
            sanitize_repo_id(
                root.file_name()
                    .and_then(|name| name.to_str())
                    .unwrap_or("repository"),
            )
        })
}

fn sanitize_repo_id(value: &str) -> String {
    value
        .trim()
        .trim_end_matches(".git")
        .chars()
        .map(|character| {
            if character.is_ascii_alphanumeric() || matches!(character, '.' | '_' | '-' | '/') {
                character
            } else {
                '-'
            }
        })
        .collect()
}

fn is_hex_commit(commit: &str) -> bool {
    (7..=40).contains(&commit.len())
        && commit
            .bytes()
            .all(|byte| byte.is_ascii_hexdigit() && !byte.is_ascii_uppercase())
}

fn normalize_path(path: &str) -> Result<String, String> {
    let normalized = path.replace('\\', "/");
    if normalized.starts_with('/') {
        return Err(format!("snapshot contains absolute path: {path}"));
    }
    let mut normalized = normalized.as_str();
    while let Some(stripped) = normalized.strip_prefix("./") {
        normalized = stripped;
    }
    let normalized = normalized.trim_end_matches('/');
    if normalized.is_empty()
        || normalized
            .split('/')
            .any(|component| component.is_empty() || component == "..")
    {
        return Err(format!("snapshot contains unsafe path: {path}"));
    }
    Ok(normalized.to_string())
}

fn valid_qualified_symbol(symbol: &str) -> bool {
    let symbol = symbol.trim();
    !symbol.is_empty() && symbol.len() <= 512 && !symbol.contains('/') && !symbol.contains('\\')
}

fn normalize_language(language: &str) -> String {
    let normalized: String = language
        .trim()
        .to_lowercase()
        .chars()
        .filter(|character| character.is_ascii_alphanumeric() || "_+#-".contains(*character))
        .take(32)
        .collect();
    if normalized
        .chars()
        .next()
        .is_some_and(|character| character.is_ascii_lowercase())
    {
        normalized
    } else {
        "unknown".to_string()
    }
}

fn normalized_export_signature(
    language: &str,
    name: &str,
    kind: &str,
    params: &[crate::types::ParamInfo],
) -> String {
    let params = params
        .iter()
        .map(|param| {
            format!(
                "{}:{}:{}",
                normalize_signature(&param.name),
                param
                    .type_annotation
                    .as_deref()
                    .map(normalize_signature)
                    .unwrap_or_default(),
                param.has_default
            )
        })
        .collect::<Vec<_>>()
        .join(",");
    format!(
        "{}|{}|{}|{}",
        normalize_signature(language),
        normalize_signature(kind),
        normalize_signature(name),
        params
    )
}

fn normalize_signature(value: &str) -> String {
    value.split_whitespace().collect::<Vec<_>>().join(" ")
}

fn anchor_id(repo_id: &str, path: &str, symbol: Option<&str>) -> String {
    let mut hasher = Sha256::new();
    hasher.update(repo_id.as_bytes());
    hasher.update([0]);
    hasher.update(path.as_bytes());
    hasher.update([0]);
    hasher.update(symbol.unwrap_or_default().as_bytes());
    format!("{ANCHOR_ID_PREFIX}{}", digest_hex(&hasher.finalize()))
}

fn hex_sha256(bytes: &[u8]) -> String {
    digest_hex(&Sha256::digest(bytes))
}

fn digest_hex(digest: &[u8]) -> String {
    use std::fmt::Write;

    digest.iter().fold(
        String::with_capacity(digest.len() * 2),
        |mut output, byte| {
            write!(&mut output, "{byte:02x}").expect("writing to String cannot fail");
            output
        },
    )
}

fn aliases_for_path(
    path: &str,
    rename_aliases: &BTreeMap<String, BTreeSet<String>>,
) -> Vec<AnchorAlias> {
    rename_aliases
        .get(path)
        .into_iter()
        .flatten()
        .map(|old_path| AnchorAlias {
            kind: "path",
            value: old_path.clone(),
        })
        .collect()
}

fn path_rename_aliases(root: &Path) -> BTreeMap<String, BTreeSet<String>> {
    let mut aliases = BTreeMap::new();
    let Ok(repo) = Repository::discover(root) else {
        return aliases;
    };
    let Ok(head) = repo.head().and_then(|head| head.peel_to_commit()) else {
        return aliases;
    };
    let Ok(head_tree) = head.tree() else {
        return aliases;
    };

    if let Ok(mut diff) = repo.diff_tree_to_workdir_with_index(Some(&head_tree), None) {
        collect_renames(&mut diff, &mut aliases);
    }
    if let Ok(parent) = head.parent(0)
        && let Ok(parent_tree) = parent.tree()
        && let Ok(mut diff) = repo.diff_tree_to_tree(Some(&parent_tree), Some(&head_tree), None)
    {
        collect_renames(&mut diff, &mut aliases);
    }
    aliases
}

fn collect_renames(diff: &mut git2::Diff<'_>, aliases: &mut BTreeMap<String, BTreeSet<String>>) {
    let mut find = DiffFindOptions::new();
    find.renames(true);
    if diff.find_similar(Some(&mut find)).is_err() {
        return;
    }
    for delta in diff
        .deltas()
        .filter(|delta| delta.status() == Delta::Renamed)
    {
        let (Some(old_path), Some(new_path)) = (delta.old_file().path(), delta.new_file().path())
        else {
            continue;
        };
        let old_path = old_path.to_string_lossy().replace('\\', "/");
        let new_path = new_path.to_string_lossy().replace('\\', "/");
        if old_path != new_path {
            aliases.entry(new_path).or_default().insert(old_path);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ids_are_stable_and_symbol_sensitive() {
        let path_id = anchor_id("owner/repo", "src/lib.rs", None);
        assert_eq!(path_id, anchor_id("owner/repo", "src/lib.rs", None));
        assert_ne!(
            path_id,
            anchor_id("owner/repo", "src/lib.rs", Some("public_api"))
        );
    }

    #[test]
    fn paths_reject_parent_traversal() {
        assert!(normalize_path("../outside.rs").is_err());
        assert!(normalize_path("src/../outside.rs").is_err());
    }
}
