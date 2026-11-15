//! `Cargo.toml` decoding — workspace members + per-crate manifests.
//!
//! Pure: callers read the files, this module parses them.

use serde::Deserialize;

use crate::error::CyperaError;

/// A workspace root `Cargo.toml` — only the fields cypera needs.
#[derive(Debug, Clone, Deserialize)]
pub struct WorkspaceManifest {
    /// `[workspace]` table.
    #[serde(default)]
    pub workspace: Option<WorkspaceTable>,
}

/// The `[workspace]` table subset cypera reads.
#[derive(Debug, Clone, Deserialize)]
pub struct WorkspaceTable {
    /// Member paths, in declaration order. Globs (`"crates/*"`)
    /// are *not* expanded here — the IO layer resolves them against
    /// the filesystem.
    #[serde(default)]
    pub members: Vec<String>,
    /// `[workspace.package]` — the inheritance source for
    /// `*.workspace = true` fields like `description`.
    #[serde(default, rename = "package")]
    pub package: Option<WorkspacePackage>,
}

/// `[workspace.package]` subset — just the description members can
/// inherit.
#[derive(Debug, Clone, Deserialize)]
pub struct WorkspacePackage {
    /// Shared description literal.
    #[serde(default)]
    pub description: Option<String>,
}

/// A member crate's `Cargo.toml` — the `[package]` fields the crate
/// map shows.
#[derive(Debug, Clone, Deserialize)]
pub struct CrateManifest {
    /// `[package]` table.
    pub package: PackageSection,
}

/// The `[package]` table subset cypera reads.
#[derive(Debug, Clone, Deserialize)]
pub struct PackageSection {
    /// Package name.
    pub name: String,
    /// Package description — a literal string, or `{ workspace =
    /// true }` under inheritance (ignored for docs).
    #[serde(default)]
    pub description: Option<toml::Value>,
    /// Version (literal or `{ workspace = true }` — only the raw
    /// value is captured; inheritance resolves at build time and is
    /// irrelevant to docs).
    #[serde(default)]
    pub version: Option<toml::Value>,
}

impl WorkspaceManifest {
    /// Parse a workspace root manifest.
    pub fn parse(path: &str, text: &str) -> Result<Self, CyperaError> {
        toml::from_str(text).map_err(|source| CyperaError::Toml {
            path: path.to_string(),
            source,
        })
    }

    /// Declared member paths (empty when the root is a bare
    /// `[package]` manifest or `members` is unset).
    pub fn members(&self) -> &[String] {
        self.workspace
            .as_ref()
            .map_or(&[], |w| w.members.as_slice())
    }

    /// `[workspace.package] description` — the value a member's
    /// `description.workspace = true` inherits. `None` for bare
    /// `[package]` roots or when undeclared.
    pub fn workspace_description(&self) -> Option<&str> {
        self.workspace
            .as_ref()
            .and_then(|w| w.package.as_ref())
            .and_then(|p| p.description.as_deref())
    }
}

impl CrateManifest {
    /// Parse a member crate manifest.
    pub fn parse(path: &str, text: &str) -> Result<Self, CyperaError> {
        toml::from_str(text).map_err(|source| CyperaError::Toml {
            path: path.to_string(),
            source,
        })
    }

    /// `[package] name`.
    pub fn name(&self) -> &str {
        &self.package.name
    }

    /// `[package] description` literal, or `""` when undeclared or
    /// workspace-inherited.
    pub fn description(&self) -> &str {
        self.package
            .description
            .as_ref()
            .and_then(toml::Value::as_str)
            .unwrap_or("")
    }

    /// `true` when `description` is the `{ workspace = true }`
    /// inheritance form (`description.workspace = true` in TOML) —
    /// the crate opts into `[workspace.package] description`.
    pub fn description_inherited(&self) -> bool {
        self.package
            .description
            .as_ref()
            .and_then(toml::Value::as_table)
            .is_some_and(|t| t.get("workspace").and_then(toml::Value::as_bool) == Some(true))
    }
}

#[cfg(test)]
mod tests {
    #![allow(clippy::unwrap_used)]
    use super::*;

    #[test]
    fn workspace_members_parse() {
        let m = WorkspaceManifest::parse(
            "Cargo.toml",
            "[workspace]\nmembers = [\"crates/a\", \"xtask\"]\n",
        )
        .unwrap();
        assert_eq!(m.members(), &["crates/a", "xtask"]);
    }

    #[test]
    fn bare_package_has_no_members() {
        let m = WorkspaceManifest::parse("Cargo.toml", "[package]\nname = \"x\"\n").unwrap();
        assert!(m.members().is_empty());
    }

    #[test]
    fn crate_manifest_reads_name_and_description() {
        let m = CrateManifest::parse(
            "crates/a/Cargo.toml",
            "[package]\nname = \"a\"\ndescription = \"does a\"\nversion.workspace = true\n",
        )
        .unwrap();
        assert_eq!(m.name(), "a");
        assert_eq!(m.description(), "does a");
    }
}
