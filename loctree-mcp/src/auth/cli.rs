//! `loctree-mcp token …` — operator surface for the bearer token store.
//!
//! Without a way to mint tokens the bind-aware policy in
//! [`crate::http::auth`] would have exactly one escape hatch
//! (`--allow-unauthenticated`), which defeats the point. This is the other
//! one: create / list / revoke / rotate against the argon2id-hashed store in
//! [`crate::auth`].
//!
//! Plaintext is printed once, on stdout, and never persisted.
//!
//! Vibecrafted with AI Agents by Vetcoders (c)2024-2026 LibraxisAI

use anyhow::{Context, Result};
use chrono::Utc;

use crate::args::TokenAction;
use crate::auth::{AuthManager, Scope, TokenStoreFile};

/// Execute a `token` subcommand against the configured store.
pub(crate) async fn run(action: &TokenAction, token_store: Option<&str>) -> Result<()> {
    let store_path = token_store
        .map(str::to_string)
        .unwrap_or_else(TokenStoreFile::default_store_path);
    let manager = AuthManager::new(store_path.clone(), None);
    manager
        .init()
        .await
        .with_context(|| format!("failed to load token store at {store_path}"))?;

    match action {
        TokenAction::Create {
            id,
            scopes,
            namespaces,
            expires_in_days,
            description,
        } => {
            let scopes = parse_scopes(scopes)?;
            let namespaces = if namespaces.is_empty() {
                vec!["*".to_string()]
            } else {
                namespaces.clone()
            };
            let expires_at = expires_in_days
                .map(|days| {
                    chrono::Duration::try_days(days)
                        .map(|delta| Utc::now() + delta)
                        .with_context(|| format!("--expires-in-days {days} is out of range"))
                })
                .transpose()?;
            let description = if description.trim().is_empty() {
                format!("loctree-mcp token '{id}'")
            } else {
                description.clone()
            };

            let plaintext = manager
                .create_token(
                    id.clone(),
                    scopes.clone(),
                    namespaces.clone(),
                    expires_at,
                    description,
                )
                .await?;

            println!("id:         {id}");
            println!("store:      {store_path}");
            println!("scopes:     {}", join_scopes(&scopes));
            println!("namespaces: {}", namespaces.join(", "));
            println!(
                "expires:    {}",
                expires_at
                    .map(|at| at.to_rfc3339())
                    .unwrap_or_else(|| "never".to_string())
            );
            println!("token:      {plaintext}");
            println!();
            println!("Copy this token now. It is argon2id-hashed at rest and cannot be recovered.");
            println!("Use it as: Authorization: Bearer {plaintext}");
        }
        TokenAction::List => {
            let tokens = manager.list_tokens().await;
            if tokens.is_empty() {
                println!("no tokens in {store_path}");
                return Ok(());
            }
            println!("store: {store_path}");
            for entry in tokens {
                println!(
                    "{}\tscopes={}\tnamespaces={}\texpires={}\tcreated={}\t{}",
                    entry.id,
                    join_scopes(&entry.scopes),
                    entry.namespaces.join(","),
                    entry
                        .expires_at
                        .map(|at| at.to_rfc3339())
                        .unwrap_or_else(|| "never".to_string()),
                    entry.created_at.to_rfc3339(),
                    entry.description
                );
            }
        }
        TokenAction::Revoke { id } => {
            if manager.revoke_token(id).await? {
                println!("revoked '{id}' from {store_path}");
            } else {
                anyhow::bail!("no token with id '{id}' in {store_path}");
            }
        }
        TokenAction::Rotate { id } => {
            let plaintext = manager.rotate_token(id).await?;
            println!("id:         {id}");
            println!("store:      {store_path}");
            println!("token:      {plaintext}");
            println!();
            println!("The previous value for '{id}' is now invalid.");
        }
    }

    Ok(())
}

/// Turn repeated `--scope` strings into the canonical grain, defaulting to
/// `context-read` (the whole read-only MCP surface) when none were passed.
/// An unrecognized token aborts token creation rather than silently narrowing.
fn parse_scopes(raw: &[String]) -> Result<Vec<Scope>> {
    if raw.is_empty() {
        return Ok(vec![Scope::ContextRead]);
    }
    raw.iter().map(|value| value.parse::<Scope>()).collect()
}

/// Render granted scopes as the comma-separated cell used by the `create` and
/// `list` output tables.
fn join_scopes(scopes: &[Scope]) -> String {
    scopes
        .iter()
        .map(ToString::to_string)
        .collect::<Vec<_>>()
        .join(",")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn empty_scope_list_defaults_to_context_read() {
        assert_eq!(parse_scopes(&[]).unwrap(), vec![Scope::ContextRead]);
    }

    #[test]
    fn scopes_parse_through_the_canonical_grain() {
        let parsed = parse_scopes(&["context-read".to_string(), "admin".to_string()]).unwrap();
        assert_eq!(parsed, vec![Scope::ContextRead, Scope::Admin]);
        assert!(parse_scopes(&["nonsense".to_string()]).is_err());
    }

    #[test]
    fn scope_rendering_is_stable_for_the_cli_table() {
        assert_eq!(
            join_scopes(&[Scope::ContextRead, Scope::Admin]),
            "context-read,admin"
        );
    }
}
