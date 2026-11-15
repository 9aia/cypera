//! cypera-core — pure domain for cypera, the org's reusable
//! documentation/reference-generation tool.
//!
//! Everything here is string/AST-shape in, markdown out — **no
//! I/O**. `cypera-cli` owns filesystem walks, process spawning, and
//! stdout; this crate owns the models and the renderers:
//!
//! - [`manifest`] — `Cargo.toml` decoding (workspace members +
//!   per-crate manifests).
//! - [`crate_map`] — the crate-map model + markdown renderer
//!   (successor to the per-repo `docs/gen-crate-map.py` scripts).
//! - [`config`] — Rust config-struct doc-comment scanning + the
//!   config-reference page renderer (successor to
//!   `docs/gen-config-docs.py`).
//! - [`cli`] — the CLI-reference page wrapper for `gen-cli-docs`
//!   output (the `clap-markdown` markdown sibling CLIs emit).

pub mod api;
pub mod cli;
pub mod config;
pub mod crate_map;
pub mod error;
pub mod manifest;
pub mod repo_config;

pub use api::render_api_index;
pub use cli::{anchor, esc_md, parse_help, prog_name, render_cli_page, render_help_walk, HelpNode};
pub use config::{
    render_config_doc_page, render_config_page, scan_config_structs, type_label, ConfigDocSection,
    ConfigDocStyle, ConfigField, ConfigStruct,
};
pub use crate_map::{
    crate_api_page, crate_map_from_workspace, crate_map_table, guess_layer_as,
    purpose_from_entry_doc, render_crate_map, scan_entry_source, splice_first_table, splice_marked,
    CrateEntry, CrateMap, EntryScan, DEFAULT_LAYER_ORDER,
};
pub use error::CyperaError;
pub use manifest::{CrateManifest, WorkspaceManifest};
pub use repo_config::{
    ApiConfig, CheckConfig, CliConfig, ConfigDocConfig, ConfigSection, ConfigStructRef,
    CrateApiConfig, CrateMapConfig, CrateMapSplice, RepoConfig, CHECK_RULE_NAMES,
};
