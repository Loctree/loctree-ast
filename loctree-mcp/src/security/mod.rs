//! Namespace security configuration for future SaaS HTTP wiring.
//!
//! NOT wired into the runtime. The HTTP transport (`crate::http::auth`)
//! enforces authentication and the `context-read` scope, but it does not map a
//! request onto a namespace: `/mcp` carries the project inside a JSON-RPC body
//! the middleware does not parse, so namespace ACLs could only be enforced on
//! `/context_pack` — and half-enforced authorization is worse than none. Per
//! token, `namespaces` is stored and honored by `AuthManager::verify`, but the
//! HTTP boundary calls `authorize(.., None)`.
//!
//! The `allow(dead_code)` below is deliberately item-scoped rather than
//! module-scoped: a blanket `#![allow(dead_code)]` here and in `auth` is
//! exactly what hid the fact that the whole auth subsystem had no call sites.

use anyhow::{Context, Result, anyhow};
use serde::Deserialize;

/// Security configuration for namespace-aware bearer auth.
///
/// Parked: parsed and unit-tested, with no runtime call site yet. See the
/// module docs for why the HTTP wave stopped short of namespace enforcement.
#[allow(
    dead_code,
    reason = "parked config type; no namespace seam in the HTTP surface yet"
)]
#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
#[serde(default)]
pub(crate) struct NamespaceSecurityConfig {
    pub enabled: bool,
    pub token_store_path: Option<String>,
    pub default_namespace: String,
}

impl Default for NamespaceSecurityConfig {
    fn default() -> Self {
        Self {
            enabled: false,
            token_store_path: None,
            default_namespace: "default".to_string(),
        }
    }
}

#[allow(
    dead_code,
    reason = "parked config parser; no namespace seam in the HTTP surface yet"
)]
impl NamespaceSecurityConfig {
    /// Load security config from a TOML file.
    ///
    /// Runtime wiring is intentionally left to the HTTP wave; this helper keeps
    /// parsing local to the security module and gives that wave a typed seam.
    pub(crate) fn from_toml_str(contents: &str) -> Result<Self> {
        let mut config = Self::default();

        for (line_no, raw_line) in contents.lines().enumerate() {
            let line = raw_line.split('#').next().unwrap_or_default().trim();
            if line.is_empty() || line.starts_with('[') {
                continue;
            }

            let (key, value) = line.split_once('=').ok_or_else(|| {
                anyhow!(
                    "failed to parse namespace security config TOML at line {}",
                    line_no + 1
                )
            })?;
            let key = key.trim();
            let value = value.trim().trim_matches('"');

            match key {
                "enabled" => {
                    config.enabled = value.parse::<bool>().with_context(|| {
                        format!(
                            "invalid boolean for security.enabled at line {}",
                            line_no + 1
                        )
                    })?;
                }
                "token_store_path" => {
                    config.token_store_path = Some(value.to_string());
                }
                "default_namespace" => {
                    config.default_namespace = value.to_string();
                }
                _ => {}
            }
        }

        Ok(config)
    }

    /// Read the config file off disk and parse it. Parked alongside
    /// [`Self::from_toml_str`] — nothing in the runtime calls this yet.
    pub(crate) async fn load_from_path(path: impl AsRef<std::path::Path>) -> Result<Self> {
        let path = path.as_ref();
        let contents = tokio::fs::read_to_string(path)
            .await
            .with_context(|| format!("failed to read security config at {}", path.display()))?;
        Self::from_toml_str(&contents)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn defaults_security_disabled() {
        let config = NamespaceSecurityConfig::from_toml_str("").unwrap();
        assert!(!config.enabled);
        assert_eq!(config.token_store_path, None);
        assert_eq!(config.default_namespace, "default");
    }

    #[test]
    fn deserializes_namespace_security_config() {
        let config = NamespaceSecurityConfig::from_toml_str(
            r#"
            enabled = true
            token_store_path = "~/.rmcp-servers/loctree-mcp/tokens.json"
            default_namespace = "loctree"
            "#,
        )
        .unwrap();

        assert!(config.enabled);
        assert_eq!(
            config.token_store_path.as_deref(),
            Some("~/.rmcp-servers/loctree-mcp/tokens.json")
        );
        assert_eq!(config.default_namespace, "loctree");
    }
}
