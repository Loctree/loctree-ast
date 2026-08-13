//! Offline Search section for HTML reports.
//!
//! Mirrors the JetBrains tool-window command bar (mode + query + run + results)
//! against the static evidence already embedded in the report — no live LSP.
//! Modes map to offline buckets: files, dead, twins, duplicates, cycles,
//! commands, insights, hotspots. Default "All" scans every bucket.

use crate::components::icons::{ICON_MAGNIFYING_GLASS, Icon};
use crate::types::{ReportSection, TreeNode};
use leptos::prelude::*;
use serde::Serialize;

/// One offline search hit rendered in the results table.
#[derive(Clone, Debug, Serialize)]
struct SearchHit {
    kind: &'static str,
    name: String,
    path: String,
    detail: String,
    line: Option<usize>,
}

/// Build a compact index from a report section for client-side filtering.
fn build_index(section: &ReportSection) -> Vec<SearchHit> {
    let mut hits = Vec::new();

    // Files from tree
    if let Some(tree) = &section.tree {
        fn walk(nodes: &[TreeNode], out: &mut Vec<SearchHit>) {
            for n in nodes {
                if n.children.is_empty() {
                    out.push(SearchHit {
                        kind: "file",
                        name: n
                            .path
                            .rsplit(['/', '\\'])
                            .next()
                            .unwrap_or(&n.path)
                            .to_string(),
                        path: n.path.clone(),
                        detail: format!("{} LOC", n.loc),
                        line: None,
                    });
                } else {
                    walk(&n.children, out);
                }
            }
        }
        walk(tree, &mut hits);
    }

    // Graph nodes as files if tree empty
    if section.tree.as_ref().map(|t| t.is_empty()).unwrap_or(true) {
        if let Some(graph) = &section.graph {
            for node in &graph.nodes {
                hits.push(SearchHit {
                    kind: "file",
                    name: node
                        .id
                        .rsplit(['/', '\\'])
                        .next()
                        .unwrap_or(&node.id)
                        .to_string(),
                    path: node.id.clone(),
                    detail: "graph node".into(),
                    line: None,
                });
            }
        }
    }

    for d in &section.dead_exports {
        hits.push(SearchHit {
            kind: "dead",
            name: d.symbol.clone(),
            path: d.file.clone(),
            detail: format!("{} · {}", d.confidence, d.reason),
            line: d.line,
        });
    }

    if let Some(twins) = &section.twins {
        for p in &twins.dead_parrots {
            hits.push(SearchHit {
                kind: "twins",
                name: p.name.clone(),
                path: p.file_path.clone(),
                detail: "dead parrot (0 imports)".into(),
                line: None,
            });
        }
        for t in &twins.exact_twins {
            let paths: Vec<&str> = t.locations.iter().map(|l| l.file_path.as_str()).collect();
            hits.push(SearchHit {
                kind: "twins",
                name: t.name.clone(),
                path: paths.join(" · "),
                detail: format!("exact twin · {} locations", t.locations.len()),
                line: None,
            });
        }
    }

    for d in &section.ranked_dups {
        hits.push(SearchHit {
            kind: "duplicates",
            name: d.name.clone(),
            path: d.files.join(" · "),
            detail: format!("score {} · {} files", d.score, d.files.len()),
            line: None,
        });
    }

    for cycle in &section.circular_imports {
        hits.push(SearchHit {
            kind: "cycles",
            name: cycle.first().cloned().unwrap_or_else(|| "cycle".into()),
            path: cycle.join(" → "),
            detail: format!("strict cycle · {} nodes", cycle.len()),
            line: None,
        });
    }
    for cycle in &section.lazy_circular_imports {
        hits.push(SearchHit {
            kind: "cycles",
            name: cycle
                .first()
                .cloned()
                .unwrap_or_else(|| "lazy-cycle".into()),
            path: cycle.join(" → "),
            detail: format!("lazy cycle · {} nodes", cycle.len()),
            line: None,
        });
    }

    for b in &section.command_bridges {
        let path = b
            .be_location
            .as_ref()
            .map(|(f, _, _)| f.clone())
            .or_else(|| b.fe_locations.first().map(|(f, _)| f.clone()))
            .unwrap_or_default();
        hits.push(SearchHit {
            kind: "commands",
            name: b.name.clone(),
            path,
            detail: format!("{} · {}", b.status, b.comm_type),
            line: b.be_location.as_ref().map(|(_, l, _)| *l),
        });
    }

    for i in &section.insights {
        hits.push(SearchHit {
            kind: "insights",
            name: i.title.clone(),
            path: String::new(),
            detail: format!("{} · {}", i.severity, i.message),
            line: None,
        });
    }

    for h in &section.hotspots {
        hits.push(SearchHit {
            kind: "hotspots",
            name: h
                .file
                .rsplit(['/', '\\'])
                .next()
                .unwrap_or(&h.file)
                .to_string(),
            path: h.file.clone(),
            detail: format!("{} · {} importers", h.category, h.importers),
            line: None,
        });
    }

    for h in &section.hub_files {
        hits.push(SearchHit {
            kind: "hubs",
            name: h
                .path
                .rsplit(['/', '\\'])
                .next()
                .unwrap_or(&h.path)
                .to_string(),
            path: h.path.clone(),
            detail: format!(
                "{} LOC · {} importers · {} imports",
                h.loc, h.importers_count, h.imports_count
            ),
            line: None,
        });
    }

    hits
}

/// Search panel — offline command bar + results (editor-parity UX).
#[component]
pub fn SearchPanel(section: ReportSection) -> impl IntoView {
    let index = build_index(&section);
    let total = index.len();
    let index_json = serde_json::to_string(&index).unwrap_or_else(|_| "[]".into());
    // Escape </script> so embedded JSON cannot break out of the script tag.
    let index_json = index_json.replace("</", "<\\/");
    let root_id = section
        .root
        .chars()
        .map(|c| if c.is_ascii_alphanumeric() { c } else { '_' })
        .collect::<String>();
    let script_id = format!("search-index-{}", root_id);
    let panel_id = format!("search-panel-{}", root_id);

    view! {
        <div class="search-panel panel" id=panel_id.clone() data-search-root=root_id.clone()>
            <div class="search-header">
                <h3>
                    <Icon path=ICON_MAGNIFYING_GLASS class="icon-sm" />
                    " Search"
                    <span class="count-badge">{total}" indexed"</span>
                </h3>
                <p class="muted search-subtitle">
                    "Offline search over this report's evidence — same command-bar shape as the JetBrains / VS Code Loctree surface. No live LSP; results are from the snapshot that produced this HTML."
                </p>
            </div>

            <div class="search-command-bar" role="search">
                <label class="search-mode-label">
                    <span class="search-eyebrow">"Mode"</span>
                    <select class="search-mode" data-role="search-mode" aria-label="Search mode">
                        <option value="all" selected=true>"All"</option>
                        <option value="file">"Files"</option>
                        <option value="dead">"Dead"</option>
                        <option value="twins">"Twins"</option>
                        <option value="duplicates">"Duplicates"</option>
                        <option value="cycles">"Cycles"</option>
                        <option value="commands">"Commands"</option>
                        <option value="insights">"Insights"</option>
                        <option value="hotspots">"Hotspots"</option>
                        <option value="hubs">"Hubs"</option>
                    </select>
                </label>
                <label class="search-query-label">
                    <span class="search-eyebrow">"Query"</span>
                    <input
                        type="search"
                        class="search-query"
                        data-role="search-query"
                        placeholder="symbol, path, or /regex/"
                        title="Plain substring match, or /pattern/ for regex. Empty query lists the mode bucket."
                        autocomplete="off"
                        spellcheck="false"
                    />
                </label>
                <button type="button" class="search-run" data-role="search-run">
                    "Run"
                </button>
            </div>

            <div class="search-meta muted" data-role="search-meta">
                {format!("Ready · {} records indexed", total)}
            </div>

            <div class="search-results-wrap">
                <table class="data-table search-results">
                    <thead>
                        <tr>
                            <th>"Kind"</th>
                            <th>"Name"</th>
                            <th>"Path"</th>
                            <th>"Detail"</th>
                        </tr>
                    </thead>
                    <tbody data-role="search-results">
                        <tr class="search-empty-row">
                            <td colspan="4" class="muted">
                                "Type a query and press Run (or Enter). Empty query lists the selected mode."
                            </td>
                        </tr>
                    </tbody>
                </table>
            </div>

            <script id=script_id data-search-index="true">
                {format!(
                    "window.__LOCTREE_SEARCH_INDEX__ = window.__LOCTREE_SEARCH_INDEX__ || {{}}; window.__LOCTREE_SEARCH_INDEX__[{:?}] = {};",
                    root_id, index_json
                )}
            </script>
        </div>
    }
}
