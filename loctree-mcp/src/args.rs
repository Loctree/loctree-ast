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

/// Snapshots retained per MCP session when `--snapshot-cache-capacity` is
/// omitted: one, so an alternating worktree cannot grow RAM without bound.
pub(crate) const DEFAULT_SNAPSHOT_CACHE_CAPACITY: usize = 1;

/// Environment fallback for `--token-store`.
pub(crate) const TOKEN_STORE_ENV: &str = "LOCTREE_MCP_TOKEN_STORE";

/// Single shared bearer token, for deployments that do not want a token file.
/// Maps onto the legacy wildcard-admin entry in [`crate::auth::AuthManager`].
pub(crate) const LEGACY_TOKEN_ENV: &str = "LOCTREE_MCP_AUTH_TOKEN";

/// Environment fallback for `--allow-unauthenticated`.
pub(crate) const ALLOW_UNAUTHENTICATED_ENV: &str = "LOCTREE_MCP_ALLOW_UNAUTHENTICATED";

/// Which transport the server should expose.
#[derive(clap::ValueEnum, Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum TransportKind {
    /// Default: serve MCP over stdio (line-delimited JSON-RPC).
    Stdio,
    /// Serve MCP over streamable-http (HTTP POST + SSE event stream)
    /// mounted at `/mcp` on the address from `--bind`.
    Http,
}

/// Parsed command line for the server process.
///
/// Field defaults encode the safe posture: stdio transport, loopback bind, no
/// `--allow-unauthenticated`, and a single-project snapshot cache.
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
    ///
    /// A non-loopback bind is refused unless bearer auth is configured or
    /// `--allow-unauthenticated` is passed. See `--token-store`.
    #[arg(long, default_value = "127.0.0.1:5174")]
    pub(crate) bind: String,

    /// Bearer token store for `--transport http`.
    ///
    /// Defaults to `~/.rmcp-servers/loctree-mcp/tokens.json`. Tokens are
    /// argon2id-hashed at rest; mint them with `loctree-mcp token create`.
    /// Falls back to `LOCTREE_MCP_TOKEN_STORE` when the flag is omitted.
    ///
    /// `global` so it reads naturally after the subcommand too:
    /// `loctree-mcp token create --id x --token-store /path/tokens.json`.
    #[arg(long, value_name = "PATH", global = true)]
    pub(crate) token_store: Option<String>,

    /// Serve a non-loopback `--bind` with NO authentication.
    ///
    /// Without this, a non-loopback bind with no configured tokens is a hard
    /// startup error rather than a silently open port. The flag exists so an
    /// operator who genuinely wants an open port has to say so out loud; it is
    /// ignored (auth stays enforced) when tokens are configured. Also settable
    /// as `LOCTREE_MCP_ALLOW_UNAUTHENTICATED=1`.
    #[arg(long)]
    pub(crate) allow_unauthenticated: bool,

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

    /// Optional maintenance subcommand. Omitted, the binary serves MCP.
    #[command(subcommand)]
    pub(crate) command: Option<Command>,
}

/// Maintenance subcommands that run instead of serving.
#[derive(clap::Subcommand, Debug)]
pub(crate) enum Command {
    /// Manage bearer tokens for the HTTP transport.
    Token {
        #[command(subcommand)]
        action: TokenAction,
    },
}

/// `loctree-mcp token …`
#[derive(clap::Subcommand, Debug)]
pub(crate) enum TokenAction {
    /// Mint a token. The plaintext is printed once and never persisted.
    Create {
        /// Human-readable token id, for example `dragon-tailnet`.
        #[arg(long)]
        id: String,
        /// Granted scope; repeatable. Defaults to `context-read`, which is the
        /// whole read-only MCP surface.
        #[arg(long = "scope", value_name = "SCOPE")]
        scopes: Vec<String>,
        /// Namespace ACL entry; repeatable. Defaults to `*`.
        #[arg(long = "namespace", value_name = "NS")]
        namespaces: Vec<String>,
        /// Expire the token after N days. Omitted, it never expires.
        #[arg(long, value_name = "DAYS")]
        expires_in_days: Option<i64>,
        /// Free-text note stored alongside the hash.
        #[arg(long, default_value = "")]
        description: String,
    },
    /// List stored tokens (metadata only — plaintext is unrecoverable).
    List,
    /// Revoke a token by id.
    Revoke {
        #[arg(long)]
        id: String,
    },
    /// Revoke a token and mint a replacement with the same metadata.
    Rotate {
        #[arg(long)]
        id: String,
    },
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

    #[test]
    fn auth_flags_default_to_the_safe_posture() {
        let args = Args::parse_from(["loctree-mcp"]);
        assert!(args.token_store.is_none());
        assert!(
            !args.allow_unauthenticated,
            "an open non-loopback port must never be the default"
        );
        assert!(args.command.is_none());
    }

    #[test]
    fn auth_flags_parse() {
        let args = Args::parse_from([
            "loctree-mcp",
            "--transport",
            "http",
            "--bind",
            "0.0.0.0:5174",
            "--token-store",
            "/tmp/tokens.json",
            "--allow-unauthenticated",
        ]);
        assert_eq!(args.bind, "0.0.0.0:5174");
        assert_eq!(args.token_store.as_deref(), Some("/tmp/tokens.json"));
        assert!(args.allow_unauthenticated);
    }

    #[test]
    fn token_create_subcommand_parses_repeatable_scopes_and_namespaces() {
        let args = Args::parse_from([
            "loctree-mcp",
            "--token-store",
            "/tmp/tokens.json",
            "token",
            "create",
            "--id",
            "dragon",
            "--scope",
            "context-read",
            "--scope",
            "admin",
            "--namespace",
            "loctree",
            "--expires-in-days",
            "30",
        ]);
        assert_eq!(args.token_store.as_deref(), Some("/tmp/tokens.json"));
        match args.command {
            Some(Command::Token {
                action:
                    TokenAction::Create {
                        id,
                        scopes,
                        namespaces,
                        expires_in_days,
                        ..
                    },
            }) => {
                assert_eq!(id, "dragon");
                assert_eq!(scopes, vec!["context-read", "admin"]);
                assert_eq!(namespaces, vec!["loctree"]);
                assert_eq!(expires_in_days, Some(30));
            }
            other => panic!("expected token create, got {other:?}"),
        }
    }

    #[test]
    fn token_lifecycle_subcommands_parse() {
        for (argv, expect_rotate) in [
            (["loctree-mcp", "token", "revoke", "--id", "x"], false),
            (["loctree-mcp", "token", "rotate", "--id", "x"], true),
        ] {
            let args = Args::parse_from(argv);
            match args.command {
                Some(Command::Token {
                    action: TokenAction::Rotate { id },
                }) => {
                    assert!(expect_rotate);
                    assert_eq!(id, "x");
                }
                Some(Command::Token {
                    action: TokenAction::Revoke { id },
                }) => {
                    assert!(!expect_rotate);
                    assert_eq!(id, "x");
                }
                other => panic!("expected token lifecycle command, got {other:?}"),
            }
        }

        assert!(matches!(
            Args::parse_from(["loctree-mcp", "token", "list"]).command,
            Some(Command::Token {
                action: TokenAction::List
            })
        ));
    }
}
