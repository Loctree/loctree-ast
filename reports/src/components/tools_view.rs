//! Tools sidebar view — first-class surface for the twelve Loctree MCP tools.
//!
//! Mirrors the tool descriptions in
//! `loctree-mcp/src/main.rs::LoctreeServer::get_info` (the server instructions
//! string) so the operator-facing report tells the same story the MCP server
//! tells an agent on first connect.
//!
//! This is the second of the two MCP-first sidebar surfaces — pair it with
//! [`super::atlas_view::AtlasView`] to show *what loctree precomputes* (atlas)
//! and *what an agent calls into* (tools) side by side.

use crate::components::icons::{ICON_TOOLBOX, Icon};
use leptos::prelude::*;

/// One row in the canonical 12-tool surface.
#[derive(Clone, Copy)]
struct ToolSpec {
    /// Wire-level tool name (matches `Tool.name` in JSON-RPC).
    name: &'static str,
    /// Grouping label (e.g. "Start", "Map", "Silencer", "Polarization gate").
    section: &'static str,
    /// Human-readable signature lifted from the MCP instructions string.
    signature: &'static str,
    /// One-line description lifted from the MCP instructions string.
    description: &'static str,
    /// Example `arguments` payload for the `tools/call` request body.
    example_args: &'static str,
    /// Anchor under the canonical docs page.
    doc_anchor: &'static str,
}

/// The twelve canonical tools, in the order the MCP server lists them.
const TOOLS: &[ToolSpec] = &[
    ToolSpec {
        name: "context",
        section: "Start",
        signature: "context(project, format?)",
        description: "Complete Agent Context Pack: structural + runtime semantics + risk + action + optional AICX memory + authority labels. Pretty JSON by default; use format='markdown' for operator-readable context.",
        example_args: r#"{ "project": ".", "format": "markdown" }"#,
        doc_anchor: "context",
    },
    ToolSpec {
        name: "repo-view",
        section: "Map",
        signature: "repo-view(project)",
        description: "Overview: files, LOC, languages, health, top hubs.",
        example_args: r#"{ "project": "." }"#,
        doc_anchor: "repo-view",
    },
    ToolSpec {
        name: "slice",
        section: "Map",
        signature: "slice(file, depth?, rescan?)",
        description: "Before modifying. File + dependencies + consumers with explicit traversal and refresh controls.",
        example_args: r#"{ "file": "src/auth/session.rs", "depth": 2 }"#,
        doc_anchor: "slice",
    },
    ToolSpec {
        name: "body",
        section: "Map",
        signature: "body(symbol, file?)",
        description: "Return the defining source body for a symbol without raw file reads or offset guessing.",
        example_args: r#"{ "symbol": "authenticate", "file": "src/auth/session.rs" }"#,
        doc_anchor: "body",
    },
    ToolSpec {
        name: "find",
        section: "Map",
        signature: "find(name, mode?)",
        description: "Before creating. Symbol and raw-text search. Regex mode scans source, Markdown, comments, and config with an explicit coverage receipt.",
        example_args: r#"{ "name": "auth(en|orize)", "mode": "regex" }"#,
        doc_anchor: "find",
    },
    ToolSpec {
        name: "impact",
        section: "Map",
        signature: "impact(file)",
        description: "Before deleting. Direct + transitive consumers (blast radius).",
        example_args: r#"{ "file": "src/db/connection.rs" }"#,
        doc_anchor: "impact",
    },
    ToolSpec {
        name: "diff",
        section: "Map",
        signature: "diff(since)",
        description: "Compare a git base with the current Living Tree, including dirty structural changes.",
        example_args: r#"{ "since": "HEAD~1" }"#,
        doc_anchor: "diff",
    },
    ToolSpec {
        name: "tree",
        section: "Map",
        signature: "tree(project, depth?, files?, match?)",
        description: "Directory structure with LOC counts. Omitted depth is unlimited, matching the CLI.",
        example_args: r#"{ "project": ".", "files": true, "match": "src/.+\\.rs$" }"#,
        doc_anchor: "tree",
    },
    ToolSpec {
        name: "focus",
        section: "Map",
        signature: "focus(directory, depth?)",
        description: "Understand a module. Files, internal edges, external dependencies, and optional consumers.",
        example_args: r#"{ "directory": "src/components", "depth": 2 }"#,
        doc_anchor: "focus",
    },
    ToolSpec {
        name: "follow",
        section: "Map",
        signature: "follow(scope)",
        description: "Pursue signals: dead, cycles, twins, hotspots, trace, commands, events, pipelines.",
        example_args: r#"{ "scope": "dead" }"#,
        doc_anchor: "follow",
    },
    ToolSpec {
        name: "suppressions",
        section: "Silencer surface",
        signature: "suppressions(project, kinds?)",
        description: "Source-side silencer inventory: Rust #[allow(...)], Rust #[ignore], Rust unsafe { ... } (env-var boilerplate split out), Semgrep nosemgrep, TypeScript @ts-ignore, ESLint eslint-disable, Python # noqa, Python # type: ignore, Shell # shellcheck disable. Literal-only detection (free-tier). Semantic enrichment (suspicious/stale) is paid-tier Wave 7+.",
        example_args: r#"{ "project": ".", "kinds": ["rust-allow", "ts-ignore"] }"#,
        doc_anchor: "suppressions",
    },
    ToolSpec {
        name: "prism",
        section: "Polarization gate",
        signature: "prism(task=[a, b, ...])",
        description: "Score conceptual smear across task framings. Emits loctree.prism.v1 JSON for vc-polarize gating.",
        example_args: r#"{ "task": ["auth", "session", "login"] }"#,
        doc_anchor: "prism",
    },
];

/// CSS modifier slug for a section label. Keeps section colors theme-able
/// without leaking strings into the markup.
fn section_slug(section: &str) -> &'static str {
    match section {
        "Start" => "start",
        "Map" => "map",
        "Silencer surface" => "silencer",
        "Polarization gate" => "polarize",
        _ => "default",
    }
}

/// First-class MCP tool view — twelve tool tiles, one per tool, grouped by
/// the canonical sections from the MCP server instructions string.
#[component]
pub fn ToolsView() -> impl IntoView {
    view! {
        <div class="panel tools-view">
            <header class="tools-header">
                <h3>
                    <Icon path=ICON_TOOLBOX />
                    "MCP Tools"
                    <span class="count-badge count-badge-success">"12 tools"</span>
                </h3>
                <p class="tools-intro">
                    "Loctree MCP provides one sharp agent surface: 12 library-backed tools, not a mirrored CLI. "
                    "Reports such as health, findings, audit, and coverage stay in the "
                    <code>"loct"</code>
                    " CLI. All tools accept the "
                    <code>"project"</code>
                    " parameter (default: current dir). First use auto-scans if no snapshot exists."
                </p>
            </header>

            <div class="tools-cards-grid">
                {TOOLS.iter().map(|spec| {
                    let example_json = format!(
                        "{{\n  \"name\": \"{}\",\n  \"arguments\": {}\n}}",
                        spec.name, spec.example_args
                    );
                    let example_copy = example_json.clone();
                    let doc_href = format!(
                        "https://github.com/Loctree/Loctree/blob/main/docs/mcp.md#{}",
                        spec.doc_anchor
                    );
                    let section_class = format!("tools-card-section tools-card-section-{}", section_slug(spec.section));
                    view! {
                        <article class="tools-card">
                            <header class="tools-card-header">
                                <span class=section_class>{spec.section}</span>
                                <h4 class="tools-card-name">{spec.name}</h4>
                                <code class="tools-card-signature">{spec.signature}</code>
                            </header>
                            <p class="tools-card-desc">{spec.description}</p>
                            <details class="tools-card-example">
                                <summary>"Example MCP call"</summary>
                                <pre class="tools-card-example-pre"><code>{example_json}</code></pre>
                                <button
                                    class="copy-btn"
                                    data-copy=example_copy
                                    title="Copy example call body"
                                >
                                    "Copy"
                                </button>
                            </details>
                            <a
                                class="tools-card-doc"
                                href=doc_href
                                rel="noopener noreferrer"
                            >
                                "Docs →"
                            </a>
                        </article>
                    }
                }).collect::<Vec<_>>()}
            </div>

            <footer class="tools-footer">
                <p class="tools-footer-fineprint">
                    "These descriptions are mirrored from the MCP server "
                    <code>"instructions"</code>
                    " string ("
                    <code>"LoctreeServer::get_info"</code>
                    "). If the server adds a tool, this list is the next thing to update."
                </p>
            </footer>
        </div>
    }
}
