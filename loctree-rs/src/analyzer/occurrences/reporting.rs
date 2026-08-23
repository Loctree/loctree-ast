use std::collections::HashMap;

use super::{
    FileCount, LiteralOccurrence, MatchRole, OccurrencePage, OccurrenceResults, ReportOptions,
    ScopeClassification,
};

impl OccurrenceResults {
    /// Per-file occurrence counts, in first-seen file order.
    pub fn file_rollup(&self) -> Vec<FileCount> {
        let mut order: Vec<String> = Vec::new();
        let mut counts: HashMap<String, (usize, ScopeClassification)> = HashMap::new();
        for occ in &self.occurrences {
            let entry = counts.entry(occ.file.clone()).or_insert_with(|| {
                order.push(occ.file.clone());
                (0, occ.scope_classification)
            });
            entry.0 += 1;
        }
        order
            .into_iter()
            .map(|file| {
                let (count, scope_classification) = counts
                    .get(&file)
                    .copied()
                    .unwrap_or((0, ScopeClassification::Unknown));
                FileCount {
                    file,
                    count,
                    scope_classification,
                }
            })
            .collect()
    }

    /// Order the full hit set so the first page carries answers, not alphabet.
    ///
    /// Emission used to be file-walk order — alphabetical in practice — so
    /// `loct find --regex 'twin\w+'` spent its entire first page on
    /// CHANGELOG.md while the 20 real definitions sat past hit 1000. The key is
    /// scope first (docs and generated files are the *last* place an answer
    /// lives), then role (a definition outranks a mention), then
    /// file/line/column.
    ///
    /// The tail breaker is deliberately the physical position, never a hash or
    /// a scan-order index: `--offset`/`--limit` paging is only coherent if the
    /// same query produces the same total order on every call, including from
    /// loctree-lsp and loctree-mcp, which page through this same function.
    pub fn rank_occurrences(&mut self) {
        self.occurrences
            .sort_by(|a, b| rank_key(a).cmp(&rank_key(b)));
    }

    /// Apply [`ReportOptions`] in place. Must be called on the full result set
    /// (before any truncation) so `total`/`by_file` reflect every occurrence.
    pub fn apply_report(&mut self, report: ReportOptions) {
        // Rank before any slicing: paging a file-ordered set would hand back a
        // first page of documentation and call it the answer.
        self.rank_occurrences();
        self.offset = report.offset.min(self.total);
        if report.group_by_file {
            self.by_file = Some(self.file_rollup());
        }
        if let Some(limit) = report.limit {
            // Page metadata is computed against `self.total` (the full result
            // count), but the slice indices must be clamped to the actual
            // backing length to avoid an out-of-bounds panic when
            // `self.total > self.occurrences.len()` (e.g. apply_report called
            // twice or on an already-slimmed result). When the invariant
            // `total == len` holds, behavior is unchanged.
            let len = self.occurrences.len();
            let offset = report.offset.min(len);
            let end = offset.saturating_add(limit).min(len);
            let returned = end.saturating_sub(offset);
            let has_more = end < self.total;
            self.occurrences = self.occurrences[offset..end].to_vec();
            self.page = Some(OccurrencePage {
                offset,
                limit,
                returned,
                has_more,
                next_offset: has_more.then_some(end),
            });
        } else if report.offset > 0 {
            // Page metadata uses `self.total`; the slice index is clamped to
            // the backing length to stay panic-safe (see the limit branch).
            let offset = report.offset.min(self.total);
            let returned = self.total.saturating_sub(offset);
            let slice_offset = report.offset.min(self.occurrences.len());
            self.occurrences = self.occurrences[slice_offset..].to_vec();
            self.page = Some(OccurrencePage {
                offset,
                limit: returned,
                returned,
                has_more: false,
                next_offset: None,
            });
        }
        if report.count_only {
            self.slim = true;
            self.occurrences.clear();
        }
        self.emitted = self.occurrences.len();
        self.truncated = self.emitted < self.total;
    }
}

/// Where a hit sits decides more than what it is: a `reference` inside
/// CHANGELOG.md is a changelog entry, not a call site. Docs and generated
/// files rank last so they never crowd out a real answer.
fn scope_rank(scope: ScopeClassification) -> u8 {
    match scope {
        ScopeClassification::Production => 0,
        ScopeClassification::Config => 1,
        ScopeClassification::Unknown => 2,
        ScopeClassification::Test => 3,
        ScopeClassification::Generated => 4,
        ScopeClassification::Docs => 5,
    }
}

/// Definition first, then the sites that change state, then imports, then
/// plain reads. Prose roles (comment / string literal / data attribute) and
/// unclassified hits sink to the bottom of their scope bucket.
///
/// A local binding (`let x = …`) is the introducer of its name: it is not a
/// symbol-table definition, but it is the site every later write and read
/// answers to, so it sits directly under `Definition` — above the mutations —
/// rather than among the plain reads.
fn role_rank(role: MatchRole) -> u8 {
    match role {
        MatchRole::Definition => 0,
        MatchRole::LocalBinding => 1,
        MatchRole::Mutation | MatchRole::FieldEmission => 2,
        MatchRole::Import => 3,
        MatchRole::Reference => 4,
        MatchRole::StyleProperty | MatchRole::ClassToken | MatchRole::StyleVariable => 5,
        MatchRole::Comment | MatchRole::StringLiteral | MatchRole::DataAttribute => 6,
        MatchRole::Unknown => 7,
    }
}

/// Total order over a hit set. Every component is derived from the hit itself,
/// so the order is reproducible across processes and snapshots.
fn rank_key(occ: &LiteralOccurrence) -> (u8, u8, &str, usize, usize) {
    (
        scope_rank(occ.scope_classification),
        role_rank(occ.match_role),
        occ.file.as_str(),
        occ.line,
        occ.column,
    )
}

#[cfg(test)]
mod tests {
    use super::super::{ReportOptions, ScanOptions, scan_files_with_scope};
    use super::*;

    /// Mixed universe: one production definition, one production read, one
    /// docs mention. File-walk order puts the changelog first — the exact
    /// shape of the 1665-hit `twin\w+` sweep whose first eleven screens were
    /// CHANGELOG.md.
    fn mixed_universe() -> OccurrenceResults {
        scan_files_with_scope(
            [
                (
                    "CHANGELOG.md",
                    "- respect suppressions in loct twins command\n",
                ),
                ("docs/BACKLOG.md", "twins detection backlog\n"),
                ("src/twins.rs", "let seen = 1;\nfn twins() {}\ntwins();\n"),
            ],
            "twins",
            ScanOptions::default(),
            super::super::FileScope::default(),
        )
    }

    #[test]
    fn ranking_puts_production_definitions_ahead_of_documentation() {
        let mut res = mixed_universe();
        assert_eq!(
            res.occurrences.first().map(|o| o.file.as_str()),
            Some("CHANGELOG.md"),
            "precondition: the raw scan is file-ordered"
        );

        res.rank_occurrences();

        assert_eq!(
            res.occurrences.first().map(|o| o.file.as_str()),
            Some("src/twins.rs"),
            "production code must outrank the changelog"
        );
        assert_eq!(
            res.occurrences.first().map(|o| o.match_role),
            Some(MatchRole::Definition),
            "the definition is the answer, not the call site"
        );
        assert!(
            res.occurrences
                .iter()
                .rev()
                .take(2)
                .all(|o| o.scope_classification == ScopeClassification::Docs),
            "docs sink to the tail: {:?}",
            res.occurrences
                .iter()
                .map(|o| o.file.as_str())
                .collect::<Vec<_>>()
        );
    }

    #[test]
    fn ranking_is_a_total_order_so_paging_stays_stable() {
        // Paging is only coherent if page 1 ++ page 2 reconstructs the same
        // sequence as the unpaged read — a ranking with ties broken by scan
        // order would silently drop or duplicate hits across the boundary.
        let mut full = mixed_universe();
        full.apply_report(ReportOptions {
            group_by_file: false,
            count_only: false,
            offset: 0,
            limit: None,
        });
        let expected: Vec<(String, usize, usize)> = full
            .occurrences
            .iter()
            .map(|o| (o.file.clone(), o.line, o.column))
            .collect();
        assert!(expected.len() >= 3, "need enough hits to page");

        let mut paged: Vec<(String, usize, usize)> = Vec::new();
        for offset in (0..expected.len()).step_by(2) {
            let mut page = mixed_universe();
            page.apply_report(ReportOptions {
                group_by_file: false,
                count_only: false,
                offset,
                limit: Some(2),
            });
            paged.extend(
                page.occurrences
                    .iter()
                    .map(|o| (o.file.clone(), o.line, o.column)),
            );
        }
        assert_eq!(paged, expected, "paged reads must reconstruct the ranking");
    }

    #[test]
    fn ranking_is_idempotent() {
        // `apply_report` ranks on every call and the docs say it may run twice.
        let mut once = mixed_universe();
        once.rank_occurrences();
        let mut twice = mixed_universe();
        twice.rank_occurrences();
        twice.rank_occurrences();
        assert_eq!(
            once.occurrences
                .iter()
                .map(|o| (o.file.as_str(), o.line, o.column))
                .collect::<Vec<_>>(),
            twice
                .occurrences
                .iter()
                .map(|o| (o.file.as_str(), o.line, o.column))
                .collect::<Vec<_>>()
        );
    }
}
