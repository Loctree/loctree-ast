//! Shared CLI/MCP build-bundle identity contract.
//!
//! A crate version alone cannot distinguish two binaries built from different
//! commits. This schema is intentionally small, stable, and human-readable so
//! operators and agents can compare `loct --version`, `loctree --version`, and
//! `loctree-mcp --version` without knowing component-specific formatting.

use serde::Serialize;

pub const BUNDLE_SCHEMA: &str = "loctree.bundle.v1";

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize)]
pub struct BundleIdentity<'a> {
    pub schema: &'static str,
    pub component: &'a str,
    pub version: &'a str,
    pub commit: &'a str,
    pub dirty: bool,
    pub bundle_id: &'a str,
}

impl<'a> BundleIdentity<'a> {
    pub const fn new(
        component: &'a str,
        version: &'a str,
        commit: &'a str,
        dirty: bool,
        bundle_id: &'a str,
    ) -> Self {
        Self {
            schema: BUNDLE_SCHEMA,
            component,
            version,
            commit,
            dirty,
            bundle_id,
        }
    }

    pub fn version_line(&self) -> String {
        format!(
            "{} {} schema={} bundle_id={} commit={} dirty={}",
            self.component, self.version, self.schema, self.bundle_id, self.commit, self.dirty
        )
    }
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum BundleCompatibility {
    Compatible,
    Mixed,
    Unknown,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
pub struct BundleDiagnostic<'a> {
    pub status: BundleCompatibility,
    pub authority: &'static str,
    pub expected_bundle_id: &'a str,
    pub actual_bundle_id: &'a str,
    pub message: String,
}

pub fn compare_bundles<'a>(
    expected: &BundleIdentity<'a>,
    actual: &BundleIdentity<'a>,
) -> BundleDiagnostic<'a> {
    if expected.commit == "unknown" || actual.commit == "unknown" {
        return BundleDiagnostic {
            status: BundleCompatibility::Unknown,
            authority: "refused",
            expected_bundle_id: expected.bundle_id,
            actual_bundle_id: actual.bundle_id,
            message: format!(
                "bundle identity is incomplete: {}={} {}={}",
                expected.component, expected.bundle_id, actual.component, actual.bundle_id
            ),
        };
    }

    if expected.schema == actual.schema && expected.bundle_id == actual.bundle_id {
        BundleDiagnostic {
            status: BundleCompatibility::Compatible,
            authority: "available",
            expected_bundle_id: expected.bundle_id,
            actual_bundle_id: actual.bundle_id,
            message: format!("bundle {} is consistent", expected.bundle_id),
        }
    } else {
        BundleDiagnostic {
            status: BundleCompatibility::Mixed,
            authority: "refused",
            expected_bundle_id: expected.bundle_id,
            actual_bundle_id: actual.bundle_id,
            message: format!(
                "mixed Loctree bundle detected: {}={} but {}={}; refusing authority until CLI and MCP are rebuilt and deployed atomically",
                expected.component, expected.bundle_id, actual.component, actual.bundle_id
            ),
        }
    }
}

pub const fn core_bundle_identity(component: &'static str) -> BundleIdentity<'static> {
    BundleIdentity::new(
        component,
        crate::BUILD_VERSION,
        crate::GIT_COMMIT,
        crate::GIT_DIRTY,
        crate::BUILD_VERSION,
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn version_line_has_the_shared_schema_fields() {
        let identity = BundleIdentity::new(
            "loct",
            "0.14.0+g12345678",
            "12345678",
            false,
            "0.14.0+g12345678",
        );
        assert_eq!(
            identity.version_line(),
            "loct 0.14.0+g12345678 schema=loctree.bundle.v1 bundle_id=0.14.0+g12345678 commit=12345678 dirty=false"
        );
    }

    #[test]
    fn mixed_markers_refuse_authority() {
        let cli = BundleIdentity::new(
            "loct",
            "0.14.0+gaaaaaaaa",
            "aaaaaaaa",
            false,
            "0.14.0+gaaaaaaaa",
        );
        let mcp = BundleIdentity::new(
            "loctree-mcp",
            "0.14.0+gbbbbbbbb",
            "bbbbbbbb",
            false,
            "0.14.0+gbbbbbbbb",
        );
        let diagnostic = compare_bundles(&cli, &mcp);
        assert_eq!(diagnostic.status, BundleCompatibility::Mixed);
        assert_eq!(diagnostic.authority, "refused");
        assert!(diagnostic.message.contains("deployed atomically"));
    }

    #[test]
    fn matching_markers_keep_authority_available() {
        let cli = BundleIdentity::new(
            "loct",
            "0.14.0+gaaaaaaaa",
            "aaaaaaaa",
            false,
            "0.14.0+gaaaaaaaa",
        );
        let mcp = BundleIdentity::new(
            "loctree-mcp",
            "0.14.0+gaaaaaaaa",
            "aaaaaaaa",
            false,
            "0.14.0+gaaaaaaaa",
        );
        let diagnostic = compare_bundles(&cli, &mcp);
        assert_eq!(diagnostic.status, BundleCompatibility::Compatible);
        assert_eq!(diagnostic.authority, "available");
    }
}
