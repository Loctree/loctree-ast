//! CLI arguments for `loctree-mcp`.
//!
//! Two transport modes:
//!   - `--transport stdio` (default) — line-delimited JSON-RPC over stdio,
//!     for editor / CLI MCP clients.
//!   - `--transport http` — axum server hosting `rmcp::transport::
//!     streamable_http_server::StreamableHttpService` at `/mcp` on `--bind`.
//!     Used by `loct watch --http` co-process and hosted MCP gateways.
//!
//! Vibecrafted with AI Agents by Vetcoders (c)2024-2026 LibraxisAI

use clap::Parser;

pub(crate) const DEFAULT_SNAPSHOT_CACHE_CAPACITY: usize = 1;

/// Which transport the server should expose.
#[derive(clap::ValueEnum, Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum TransportKind {
    /// Default: serve MCP over stdio (line-delimited JSON-RPC).
    Stdio,
    /// Serve MCP over streamable-http (HTTP POST + SSE event stream)
    /// mounted at `/mcp` on the address from `--bind`.
    Http,
}

#[derive(Parser, Debug)]
#[command(name = "loctree-mcp")]
#[command(about = "Universal MCP server for loctree - works with any project")]
#[command(disable_version_flag = true)]
pub(crate) struct Args {
    /// Print the shared CLI/MCP bundle identity marker and exit.
    #[arg(short = 'V', long)]
    pub(crate) version: bool,

    /// Log level (trace, debug, info, warn, error)
    #[arg(long, default_value = "info")]
    pub(crate) log_level: String,

    /// Which transport the server should expose.
    ///
    /// `stdio` is the long-standing default for editor / CLI MCP clients.
    /// `http` brings up an axum server hosting the streamable-http MCP
    /// surface on `--bind`. Use this when the MCP client wants to connect
    /// over a TCP socket — for example the `loct watch --http` co-process
    /// pattern, or a hosted MCP gateway.
    #[arg(long, value_enum, default_value_t = TransportKind::Stdio)]
    pub(crate) transport: TransportKind,

    /// Bind address for `--transport http`. Defaults to a loopback-only
    /// listener so the server isn't accidentally exposed to the network.
    #[arg(long, default_value = "127.0.0.1:5174")]
    pub(crate) bind: String,

    /// Pin a default project root for this server instance.
    ///
    /// When set, tool calls that omit the per-request `project` field
    /// resolve against this root instead of the server's current working
    /// directory. The per-request `project` parameter still overrides it,
    /// so the server stays "universal" — `--root` only changes the
    /// *default* that empty `project` fields fall back to.
    ///
    /// Used by `loct watch --http`, which spawns this server as a
    /// co-process pinned to the watched repo root. `--project` is an
    /// accepted alias so the launcher and operators can use whichever
    /// name reads clearer at the call site.
    #[arg(long, alias = "project", value_name = "DIR")]
    pub(crate) root: Option<String>,

    /// Maximum number of project snapshots retained in memory per MCP session.
    ///
    /// The default keeps only the most recently used project so temporary
    /// worktrees cannot accumulate unbounded snapshot memory. Set this higher
    /// only when intentionally alternating between multiple projects. A value
    /// of 0 disables the in-memory snapshot cache.
    #[arg(
        long,
        default_value_t = DEFAULT_SNAPSHOT_CACHE_CAPACITY,
        value_name = "COUNT"
    )]
    pub(crate) snapshot_cache_capacity: usize,

    /// Stop an HTTP server when its supervising process closes stdin.
    ///
    /// `loct watch --http` uses a private pipe for this contract. It makes
    /// parent death observable even when the watcher is terminated without
    /// running Rust destructors. Standalone HTTP servers leave this disabled.
    #[arg(long)]
    pub(crate) exit_on_stdin_eof: bool,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn root_flag_parses() {
        let args = Args::parse_from(["loctree-mcp", "--root", "/tmp/x"]);
        assert_eq!(args.root.as_deref(), Some("/tmp/x"));
    }

    #[test]
    fn project_alias_parses_to_root() {
        let args = Args::parse_from(["loctree-mcp", "--project", "/tmp/y"]);
        assert_eq!(args.root.as_deref(), Some("/tmp/y"));
    }

    #[test]
    fn no_root_is_none_universal_mode() {
        let args = Args::parse_from(["loctree-mcp"]);
        assert!(args.root.is_none());
        assert_eq!(
            args.snapshot_cache_capacity,
            DEFAULT_SNAPSHOT_CACHE_CAPACITY
        );
        assert!(!args.version);
    }

    #[test]
    fn snapshot_cache_capacity_accepts_zero_and_explicit_limits() {
        let disabled = Args::parse_from(["loctree-mcp", "--snapshot-cache-capacity", "0"]);
        assert_eq!(disabled.snapshot_cache_capacity, 0);

        let bounded = Args::parse_from(["loctree-mcp", "--snapshot-cache-capacity", "3"]);
        assert_eq!(bounded.snapshot_cache_capacity, 3);
    }

    #[test]
    fn version_flag_is_explicit_and_keeps_short_alias() {
        assert!(Args::parse_from(["loctree-mcp", "--version"]).version);
        assert!(Args::parse_from(["loctree-mcp", "-V"]).version);
    }

    #[test]
    fn exit_on_stdin_eof_flag_is_explicit_and_opt_in() {
        let default_args = Args::parse_from(["loctree-mcp"]);
        assert!(!default_args.exit_on_stdin_eof);

        let supervised = Args::parse_from(["loctree-mcp", "--exit-on-stdin-eof"]);
        assert!(supervised.exit_on_stdin_eof);
    }
}
