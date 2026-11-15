//! Error type — kept narrow; most domain failures are parse errors
//! carrying the offending file path.

/// cypera-core failure.
#[derive(Debug, thiserror::Error)]
pub enum CyperaError {
    /// A `Cargo.toml` failed to decode.
    #[error("{path}: invalid TOML: {source}")]
    Toml {
        /// File the manifest came from.
        path: String,
        /// Underlying parse error.
        source: toml::de::Error,
    },
    /// A workspace declares no members.
    #[error("{path}: workspace has no members")]
    NoMembers {
        /// Workspace root manifest path.
        path: String,
    },
    /// A `cypera.toml` is structurally valid TOML but semantically
    /// wrong (e.g. a splice with both/neither mode).
    #[error("{path}: {message}")]
    Config {
        /// Config file the problem is in.
        path: String,
        /// What's wrong.
        message: String,
    },
    /// A referenced member manifest is missing a required field.
    #[error("{path}: missing `{field}`")]
    MissingField {
        /// Crate manifest path.
        path: String,
        /// Field name.
        field: &'static str,
    },
}
