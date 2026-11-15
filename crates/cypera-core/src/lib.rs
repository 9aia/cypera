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

pub mod cli;
pub mod config;
pub mod crate_map;
pub mod error;
pub mod manifest;

pub use cli::render_cli_page;
pub use config::{render_config_page, scan_config_structs, ConfigField, ConfigStruct};
pub use crate_map::{
    crate_map_from_workspace, crate_map_table, render_crate_map, scan_entry_source, CrateEntry,
    CrateMap, EntryScan,
};
pub use error::CyperaError;
pub use manifest::{CrateManifest, WorkspaceManifest};
