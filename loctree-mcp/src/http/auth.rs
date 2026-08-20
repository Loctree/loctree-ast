//! Bearer-auth posture and middleware for the loctree-mcp HTTP transport.
//!
//! The HTTP transport serves filesystem-reading MCP tools plus the
//! `/context_pack` endpoint. Until this module existed, the *only* thing
//! standing between those tools and the network was the loopback default in
//! [`crate::args`] — a single `LOCTREE_MCP_BIND=0.0.0.0:5174` turned the
//! service into an unauthenticated remote file reader.
//!
//! The posture here is bind-aware and fail-safe:
//!
//! | bind | tokens configured | `--allow-unauthenticated` | result |
//! | --- | --- | --- | --- |
//! | loopback | no | — | serve open (zero-config local UX) |
//! | loopback | yes | — | bearer auth enforced |
//! | non-loopback | yes | — | bearer auth enforced |
//! | non-loopback | no | yes | serve open, loud warning |
//! | non-loopback | no | no | **refuse to start** |
//!
//! [`resolve`] runs *before* the listener is bound, so the refusal case never
//! opens a socket. It is called from [`crate::http::serve_http`] itself rather
//! than from the caller, so there is no code path that reaches the router
//! without having gone through the policy.
//!
//! There is no TLS in this process. A non-loopback bind is only safe behind a
//! reverse proxy or on a tailnet.
//!
//! Vibecrafted with AI Agents by Vetcoders (c)2024-2026 LibraxisAI

use std::net::ToSocketAddrs;
use std::sync::Arc;

use anyhow::{Context, Result, anyhow};
use axum::extract::{Request, State};
use axum::http::{StatusCode, header};
use axum::middleware::Next;
use axum::response::{IntoResponse, Response};
use tracing::{info, warn};

use crate::args::{ALLOW_UNAUTHENTICATED_ENV, Args, LEGACY_TOKEN_ENV, TOKEN_STORE_ENV};
use crate::auth::{AuthDenial, AuthManager, Scope, TokenStoreFile};

/// Scope every HTTP route requires.
///
/// All 12 MCP tools and `/context_pack` are read-only structural queries, so
/// `context-read` is the whole surface. `tool-execute` / `cli-full` stay
/// reserved for a future write side.
const REQUIRED_SCOPE: Scope = Scope::ContextRead;

/// Whether a bind address can be reached from off-host.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum BindExposure {
    /// Every resolved address is a loopback address.
    Loopback,
    /// At least one resolved address is reachable beyond loopback — including
    /// the unspecified addresses `0.0.0.0` and `::`, which bind every
    /// interface.
    Exposed,
}

/// Classify a `--bind` string.
///
/// Fail-safe by construction: anything that does not resolve to an all-loopback
/// address set — including addresses that fail to resolve at all — is treated as
/// [`BindExposure::Exposed`]. A parsing mistake therefore tightens the policy
/// instead of loosening it.
pub(crate) fn classify_bind(bind: &str) -> BindExposure {
    let Ok(resolved) = bind.to_socket_addrs() else {
        return BindExposure::Exposed;
    };
    let resolved: Vec<_> = resolved.collect();
    if resolved.is_empty() {
        return BindExposure::Exposed;
    }
    if resolved.iter().all(|addr| addr.ip().is_loopback()) {
        BindExposure::Loopback
    } else {
        BindExposure::Exposed
    }
}

/// Auth inputs collected from CLI flags and the process environment.
#[derive(Debug, Clone, Default)]
pub(crate) struct AuthSettings {
    /// Token store path. `None` means the default store location.
    pub(crate) token_store: Option<String>,
    /// Optional single shared token (wildcard-admin legacy shape).
    pub(crate) legacy_token: Option<String>,
    /// Operator opt-in to an unauthenticated non-loopback listener.
    pub(crate) allow_unauthenticated: bool,
}

impl AuthSettings {
    /// Build settings from parsed args, falling back to the environment.
    ///
    /// The env fallbacks mirror the existing `LOCTREE_MCP_ALLOWED_ROOTS` style
    /// in `main.rs`: read explicitly with [`std::env::var`], no clap `env`
    /// feature.
    pub(crate) fn from_args(args: &Args) -> Self {
        Self {
            token_store: args
                .token_store
                .clone()
                .or_else(|| non_empty_env(TOKEN_STORE_ENV)),
            legacy_token: non_empty_env(LEGACY_TOKEN_ENV),
            allow_unauthenticated: args.allow_unauthenticated
                || env_flag(ALLOW_UNAUTHENTICATED_ENV),
        }
    }
}

fn non_empty_env(key: &str) -> Option<String> {
    std::env::var(key)
        .ok()
        .map(|raw| raw.trim().to_string())
        .filter(|raw| !raw.is_empty())
}

/// Truthy env flag: `1`, `true`, `yes`, `on` (case-insensitive).
fn env_flag(key: &str) -> bool {
    non_empty_env(key).is_some_and(|raw| {
        matches!(
            raw.to_ascii_lowercase().as_str(),
            "1" | "true" | "yes" | "on"
        )
    })
}

/// Resolved auth posture for one HTTP server run.
#[derive(Clone, Debug)]
pub(crate) enum HttpAuth {
    /// Loopback bind with no tokens configured — today's zero-config local UX.
    LoopbackOpen,
    /// Non-loopback bind the operator explicitly chose to leave open.
    ExplicitlyUnauthenticated,
    /// Bearer auth enforced against the token store.
    Enforced(Arc<AuthManager>),
}

impl HttpAuth {
    /// One-line description for the startup banner.
    pub(crate) fn describe(&self) -> &'static str {
        match self {
            Self::LoopbackOpen => "auth disabled (loopback bind, no tokens configured)",
            Self::ExplicitlyUnauthenticated => {
                "auth DISABLED on a non-loopback bind (--allow-unauthenticated)"
            }
            Self::Enforced(_) => "bearer auth enforced (Authorization: Bearer <token>)",
        }
    }
}

/// Decide the auth posture for `bind`, or refuse.
///
/// Runs before the listener is bound. The `Err` arm is the security-critical
/// one: a non-loopback bind with no configured tokens and no explicit override
/// must never become a listening socket.
pub(crate) async fn resolve(bind: &str, settings: &AuthSettings) -> Result<HttpAuth> {
    let manager = AuthManager::new(
        settings.token_store.clone().unwrap_or_default(),
        settings.legacy_token.clone(),
    );
    manager
        .init()
        .await
        .with_context(|| "failed to load the loctree-mcp bearer token store")?;

    let has_tokens = manager.has_any_tokens().await;
    let exposure = classify_bind(bind);

    if has_tokens {
        if settings.allow_unauthenticated {
            warn!(
                "--allow-unauthenticated ignored: bearer tokens are configured, so auth stays enforced on {bind}"
            );
        }
        info!("HTTP auth: bearer tokens configured; enforcing on {bind}");
        return Ok(HttpAuth::Enforced(Arc::new(manager)));
    }

    match exposure {
        BindExposure::Loopback => {
            info!(
                "HTTP auth: no tokens configured and {bind} is loopback-only; serving without authentication"
            );
            Ok(HttpAuth::LoopbackOpen)
        }
        BindExposure::Exposed if settings.allow_unauthenticated => {
            warn!(
                "SECURITY: serving UNAUTHENTICATED loctree-mcp on non-loopback bind {bind}. \
                 Every MCP tool and /context_pack can read any project directory this process \
                 can reach, from anywhere that can open this port. There is no TLS in-process. \
                 This is only acceptable behind a reverse proxy or on a private tailnet."
            );
            Ok(HttpAuth::ExplicitlyUnauthenticated)
        }
        BindExposure::Exposed => Err(anyhow!("{}", refusal_message(bind, settings))),
    }
}

/// Actionable startup-refusal text for an unauthenticated non-loopback bind.
fn refusal_message(bind: &str, settings: &AuthSettings) -> String {
    let store = settings
        .token_store
        .clone()
        .unwrap_or_else(TokenStoreFile::default_store_path);
    format!(
        "refusing to start: --bind {bind} is not loopback and no bearer tokens are configured.\n\
         \n\
         The HTTP transport exposes 12 filesystem-reading MCP tools plus /context_pack. An\n\
         unauthenticated non-loopback listener hands every project directory this process can\n\
         read to anyone who can open the port. There is no TLS termination in this process.\n\
         \n\
         Pick one:\n\
         \x20 1. Mint a token, then restart:\n\
         \x20      loctree-mcp token create --id <name> --scope context-read\n\
         \x20    (token store: {store}; override with --token-store or {TOKEN_STORE_ENV})\n\
         \x20 2. Export one shared token: {LEGACY_TOKEN_ENV}=<secret>\n\
         \x20 3. Keep it local: --bind 127.0.0.1:5174\n\
         \x20 4. Accept the risk out loud: --allow-unauthenticated \
         (or {ALLOW_UNAUTHENTICATED_ENV}=1)"
    )
}

/// Axum middleware enforcing `Authorization: Bearer <token>`.
///
/// Layered over the whole router, so `/context_pack` and the nested `/mcp`
/// service are both covered.
pub(crate) async fn bearer_guard(
    State(auth): State<Arc<AuthManager>>,
    request: Request,
    next: Next,
) -> Response {
    let presented = request
        .headers()
        .get(header::AUTHORIZATION)
        .and_then(|value| value.to_str().ok())
        .and_then(bearer_token)
        .unwrap_or_default();

    match auth.authorize(presented, &REQUIRED_SCOPE, None).await {
        Ok(_) => next.run(request).await,
        Err(denial) => denial_response(&denial),
    }
}

/// Extract the credential from an `Authorization` header value.
///
/// RFC 6750 §2.1: the `Bearer` scheme name is case-insensitive.
fn bearer_token(header_value: &str) -> Option<&str> {
    let (scheme, credential) = header_value.split_once(' ')?;
    if !scheme.eq_ignore_ascii_case("bearer") {
        return None;
    }
    let credential = credential.trim();
    (!credential.is_empty()).then_some(credential)
}

fn denial_response(denial: &AuthDenial) -> Response {
    let status = match denial {
        AuthDenial::MissingToken | AuthDenial::InvalidToken | AuthDenial::Expired { .. } => {
            StatusCode::UNAUTHORIZED
        }
        AuthDenial::InsufficientScope { .. } | AuthDenial::NamespaceDenied { .. } => {
            StatusCode::FORBIDDEN
        }
    };
    let body = axum::Json(serde_json::json!({
        "error": "unauthorized",
        "detail": denial.to_string(),
    }));

    if status == StatusCode::UNAUTHORIZED {
        (
            status,
            [(header::WWW_AUTHENTICATE, "Bearer realm=\"loctree-mcp\"")],
            body,
        )
            .into_response()
    } else {
        (status, body).into_response()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn settings(store: &std::path::Path, allow: bool) -> AuthSettings {
        AuthSettings {
            token_store: Some(store.display().to_string()),
            legacy_token: None,
            allow_unauthenticated: allow,
        }
    }

    #[test]
    fn loopback_forms_classify_as_loopback() {
        assert_eq!(classify_bind("127.0.0.1:5174"), BindExposure::Loopback);
        assert_eq!(classify_bind("127.0.0.1:0"), BindExposure::Loopback);
        assert_eq!(classify_bind("127.7.7.7:5174"), BindExposure::Loopback);
        assert_eq!(classify_bind("[::1]:5174"), BindExposure::Loopback);
    }

    #[test]
    fn wildcard_and_routable_binds_classify_as_exposed() {
        assert_eq!(classify_bind("0.0.0.0:5174"), BindExposure::Exposed);
        assert_eq!(classify_bind("[::]:5174"), BindExposure::Exposed);
        assert_eq!(classify_bind("192.168.1.10:5174"), BindExposure::Exposed);
    }

    #[test]
    fn unresolvable_bind_fails_safe_to_exposed() {
        assert_eq!(classify_bind("not-a-bind-address"), BindExposure::Exposed);
        assert_eq!(classify_bind(""), BindExposure::Exposed);
    }

    #[test]
    fn bearer_header_parsing_is_scheme_insensitive_and_rejects_junk() {
        assert_eq!(bearer_token("Bearer loct_abc"), Some("loct_abc"));
        assert_eq!(bearer_token("bearer loct_abc"), Some("loct_abc"));
        assert_eq!(bearer_token("BEARER  loct_abc "), Some("loct_abc"));
        assert_eq!(bearer_token("Basic loct_abc"), None);
        assert_eq!(bearer_token("Bearer "), None);
        assert_eq!(bearer_token("loct_abc"), None);
    }

    #[tokio::test]
    async fn loopback_without_tokens_serves_open() {
        let dir = tempfile::tempdir().unwrap();
        let store = dir.path().join("tokens.json");
        let posture = resolve("127.0.0.1:0", &settings(&store, false))
            .await
            .expect("loopback with no tokens must start");
        assert!(matches!(posture, HttpAuth::LoopbackOpen));
    }

    #[tokio::test]
    async fn non_loopback_without_tokens_refuses_to_start() {
        let dir = tempfile::tempdir().unwrap();
        let store = dir.path().join("tokens.json");
        let error = resolve("0.0.0.0:0", &settings(&store, false))
            .await
            .expect_err("non-loopback with no tokens must refuse");
        let text = format!("{error:#}");
        assert!(text.contains("refusing to start"), "{text}");
        assert!(text.contains("--allow-unauthenticated"), "{text}");
        assert!(text.contains("token create"), "{text}");
    }

    #[tokio::test]
    async fn explicit_override_is_the_only_open_non_loopback_path() {
        let dir = tempfile::tempdir().unwrap();
        let store = dir.path().join("tokens.json");
        let posture = resolve("0.0.0.0:0", &settings(&store, true))
            .await
            .expect("explicit override must start");
        assert!(matches!(posture, HttpAuth::ExplicitlyUnauthenticated));
    }

    #[tokio::test]
    async fn configured_tokens_enforce_on_both_bind_classes() {
        let dir = tempfile::tempdir().unwrap();
        let store = dir.path().join("tokens.json");

        let manager = AuthManager::new(store.display().to_string(), None);
        manager
            .create_token(
                "gate".to_string(),
                vec![Scope::ContextRead],
                vec!["*".to_string()],
                None,
                "test".to_string(),
            )
            .await
            .unwrap();

        for bind in ["127.0.0.1:0", "0.0.0.0:0"] {
            let posture = resolve(bind, &settings(&store, false)).await.unwrap();
            assert!(
                matches!(posture, HttpAuth::Enforced(_)),
                "bind {bind} must enforce when tokens exist"
            );
        }
    }

    #[tokio::test]
    async fn override_cannot_disable_auth_once_tokens_exist() {
        let dir = tempfile::tempdir().unwrap();
        let store = dir.path().join("tokens.json");

        let manager = AuthManager::new(store.display().to_string(), None);
        manager
            .create_token(
                "gate".to_string(),
                vec![Scope::ContextRead],
                vec!["*".to_string()],
                None,
                "test".to_string(),
            )
            .await
            .unwrap();

        let posture = resolve("0.0.0.0:0", &settings(&store, true)).await.unwrap();
        assert!(matches!(posture, HttpAuth::Enforced(_)));
    }

    #[tokio::test]
    async fn legacy_env_token_counts_as_configured_auth() {
        let dir = tempfile::tempdir().unwrap();
        let store = dir.path().join("tokens.json");
        let settings = AuthSettings {
            token_store: Some(store.display().to_string()),
            legacy_token: Some("shared-secret".to_string()),
            allow_unauthenticated: false,
        };
        let posture = resolve("0.0.0.0:0", &settings).await.unwrap();
        assert!(matches!(posture, HttpAuth::Enforced(_)));
    }
}
