use std::collections::{BTreeMap, HashSet};

use super::{
    HitShape, LiteralOccurrence, MatchConfidence, MatchRole, RoleSummary, ScopeClassification,
    ScopeClassificationCount, SuggestedNext, is_identifier,
};

fn shell_quote(value: &str) -> String {
    format!("'{}'", value.replace('\'', "'\\''"))
}

/// Git conflict markers whose *middle* form is periodic: `=======` matches at
/// every 7th column of a decorative `# ==========` banner, so one banner line
/// alone yields ~10 hits. Anchoring is the fix, not filtering.
const CONFLICT_MARKERS: [&str; 3] = ["<<<<<<<", "=======", ">>>>>>>"];

/// Suggest the anchored conflict-marker pattern when the caller ran an
/// unanchored one.
///
/// Only fires for a pattern that names a conflict marker and carries neither
/// `^` nor `$` — the moment a caller anchors anything, they own the shape and
/// this hint would be noise. Regex path only: `--literal` never evaluates
/// anchors, so recommending them there would be a lie.
pub(super) fn conflict_marker_anchor_hint(pattern: &str) -> Option<SuggestedNext> {
    if pattern.contains('^') || pattern.contains('$') {
        return None;
    }
    if !CONFLICT_MARKERS.iter().any(|m| pattern.contains(m)) {
        return None;
    }
    Some(SuggestedNext {
        command: r"loct find --regex '^(<{7} |={7}$|>{7} )'".to_string(),
        reason:
            "unanchored conflict markers match mid-line: `=======` hits every 7th column of a `# =====` banner — anchor to line starts/ends for real conflict hunks"
                .to_string(),
    })
}

pub(super) fn suggested_next(query: &str, occurrences: &[LiteralOccurrence]) -> Vec<SuggestedNext> {
    let quoted_query = shell_quote(query);
    if occurrences.is_empty() {
        return vec![
            SuggestedNext {
                command: format!("loct find --discover {quoted_query} --json"),
                reason: "broaden from literal absence to symbol and fuzzy search without treating suggestions as evidence".to_string(),
            },
            SuggestedNext {
                command: format!("loct query where-symbol {quoted_query} --json"),
                reason: "check whether the query is a known symbol definition rather than a literal occurrence".to_string(),
            },
        ];
    }

    let mut out = Vec::new();
    let shape = hit_shape(occurrences);

    // Shape-driven next moves first when the distribution is trustworthy.
    if let Some(shape) = shape.as_ref() {
        match shape.label {
            "single_writer" => {
                if let Some(def_file) = occurrences
                    .iter()
                    .find(|o| o.match_role == MatchRole::Definition)
                    .map(|o| o.file.as_str())
                {
                    out.push(SuggestedNext {
                        command: format!("loct slice {}", shell_quote(def_file)),
                        reason: "single-writer flag: inspect the defining file's consumers before changing the write site"
                            .to_string(),
                    });
                }
                if let Some(write) = occurrences
                    .iter()
                    .find(|o| o.match_role == MatchRole::Mutation)
                {
                    out.push(SuggestedNext {
                        command: format!(
                            "loct body {}",
                            shell_quote(
                                write
                                    .enclosing_symbol
                                    .as_ref()
                                    .map(|s| s.name.as_str())
                                    .unwrap_or(query)
                            )
                        ),
                        reason: "open the function that performs the sole write".to_string(),
                    });
                }
            }
            "read_only" => {
                out.push(SuggestedNext {
                    command: format!("loct query where-symbol {quoted_query} --json"),
                    reason:
                        "read-only hit set — confirm the definition, then audit callers via impact"
                            .to_string(),
                });
            }
            "reference_only" => {
                out.push(SuggestedNext {
                    command: format!("loct query where-symbol {quoted_query} --json"),
                    reason: "no definition role in hits — locate the declaration before interpreting reads"
                        .to_string(),
                });
            }
            _ => {}
        }
    }

    if is_identifier(query) {
        if !out.iter().any(|s| s.command.starts_with("loct body ")) {
            out.push(SuggestedNext {
                command: format!("loct body {quoted_query} --json"),
                reason: "open the definition/body for the matched identifier when available"
                    .to_string(),
            });
        }
        out.push(SuggestedNext {
            command: format!("loct find {quoted_query} --json"),
            reason: "confirm literal parity before narrowing to structural interpretation"
                .to_string(),
        });
        if !out.iter().any(|s| s.command.contains("where-symbol")) {
            out.push(SuggestedNext {
                command: format!("loct query where-symbol {quoted_query} --json"),
                reason: "separate definition locations from literal read/write sites".to_string(),
            });
        }
    }
    if let Some(first) = occurrences.first()
        && !out.iter().any(|s| s.command.starts_with("loct slice "))
    {
        out.push(SuggestedNext {
            command: format!("loct slice {}", shell_quote(&first.file)),
            reason:
                "inspect imports, dependencies, and consumers around the first literal-hit file"
                    .to_string(),
        });
    }
    out.push(SuggestedNext {
        command: "loct follow all".to_string(),
        reason:
            "look for repo-level dead, cycle, twin, hotspot, and trace signals after local evidence"
                .to_string(),
    });
    out
}

/// Synthesize a distribution shape from role counts.
///
/// Returns `None` for an empty set. Labels stay conservative: mixed or sparse
/// patterns become `mixed` / `reference_only` / `unknown` rather than a
/// confident wrong story. Authority is `loctree_derived` only when at least one
/// high-confidence definition is present (symbol-table proven); otherwise
/// `semantic_guess`.
pub(super) fn hit_shape(occurrences: &[LiteralOccurrence]) -> Option<HitShape> {
    if occurrences.is_empty() {
        return None;
    }

    let mut definitions = 0usize;
    let mut writers = 0usize;
    let mut readers = 0usize;
    let mut high_conf_definitions = 0usize;
    let mut non_test = 0usize;

    for occ in occurrences {
        if occ.scope_classification != ScopeClassification::Test {
            non_test += 1;
        }
        match occ.match_role {
            MatchRole::Definition => {
                definitions += 1;
                if occ.confidence == MatchConfidence::High {
                    high_conf_definitions += 1;
                }
            }
            MatchRole::Mutation | MatchRole::FieldEmission => writers += 1,
            MatchRole::Reference | MatchRole::LocalBinding => readers += 1,
            _ => {}
        }
    }

    let authority = if high_conf_definitions > 0 {
        "loctree_derived"
    } else {
        "semantic_guess"
    };

    let (label, note) = if non_test == 0 {
        (
            "test_only",
            Some("every hit sits in a test-scoped file".to_string()),
        )
    } else if definitions == 1 && writers == 1 && readers >= 1 {
        (
            "single_writer",
            Some(format!(
                "1 definition, 1 write site, {readers} read site(s) — state flag / single-writer pattern"
            )),
        )
    } else if definitions == 1 && writers == 0 && readers >= 1 {
        (
            "read_only",
            Some(format!(
                "1 definition and {readers} read site(s); no assignment sites in the hit set"
            )),
        )
    } else if definitions >= 1 && writers == 0 && readers == 0 {
        (
            "definition_only",
            Some(
                "definition site(s) only — no reads or writes in the scanned universe".to_string(),
            ),
        )
    } else if definitions == 0 && writers == 0 && readers >= 1 {
        (
            "reference_only",
            Some("no definition role in this hit set — pair with where-symbol or body".to_string()),
        )
    } else if definitions > 0 || writers > 0 || readers > 0 {
        (
            "mixed",
            Some(format!(
                "{definitions} def / {writers} write / {readers} read — inspect roles before acting"
            )),
        )
    } else {
        ("unknown", None)
    };

    Some(HitShape {
        label,
        definitions,
        writers,
        readers,
        authority,
        note,
    })
}

/// Bucket the full occurrence set into the definition-vs-callsite [`RoleSummary`].
/// Returns `None` for an empty set so a not-found result omits the rollup.
pub(super) fn role_summary(occurrences: &[LiteralOccurrence]) -> Option<RoleSummary> {
    if occurrences.is_empty() {
        return None;
    }
    let mut summary = RoleSummary {
        definitions: 0,
        callsites: 0,
        imports: 0,
        non_code: 0,
        other: 0,
        definition_files: Vec::new(),
    };
    let mut def_files_seen = HashSet::new();
    for occ in occurrences {
        match occ.match_role {
            MatchRole::Definition => {
                summary.definitions += 1;
                if def_files_seen.insert(occ.file.as_str()) {
                    summary.definition_files.push(occ.file.clone());
                }
            }
            MatchRole::Reference
            | MatchRole::Mutation
            | MatchRole::FieldEmission
            | MatchRole::LocalBinding => summary.callsites += 1,
            MatchRole::Import => summary.imports += 1,
            MatchRole::Comment | MatchRole::StringLiteral | MatchRole::DataAttribute => {
                summary.non_code += 1
            }
            MatchRole::StyleProperty
            | MatchRole::ClassToken
            | MatchRole::StyleVariable
            | MatchRole::Unknown => summary.other += 1,
        }
    }
    Some(summary)
}

pub(super) fn scope_classification_counts(
    occurrences: &[LiteralOccurrence],
) -> Vec<ScopeClassificationCount> {
    let mut counts: BTreeMap<&'static str, (ScopeClassification, usize)> = BTreeMap::new();
    for occ in occurrences {
        let entry = counts
            .entry(occ.scope_classification.as_str())
            .or_insert((occ.scope_classification, 0));
        entry.1 += 1;
    }
    counts
        .into_values()
        .map(|(scope_classification, count)| ScopeClassificationCount {
            scope_classification,
            count,
        })
        .collect()
}
