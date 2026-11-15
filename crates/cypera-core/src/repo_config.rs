//! `cypera.toml` — the per-repo configuration that replaces the
//! hardcoded knobs the per-repo `docs/gen-*.py` scripts carried
//! (`PKG`/`BIN`/`PREFIX`/`OUT`, `SRCS`/`TITLES`/`SIMPLE`, `LAYER`/
//! `ORDER`/`SITE`, curated prose). Discovered upward from `--repo`
//! (or the cwd); every section is optional.
//!
//! ```toml
//! [crate_map]
//! order = ["domain", "port", "adapter", "ops", "binary"]
//! default_layer = "impl"
//!
//! [crate_map.layers]
//! "myco-core" = "domain"
//!
//! [[crate_map.splices]]
//! target = "explanation/architecture.md"
//! marker = "<!-- crate-map -->"
//!
//! [[crate_map.splices]]
//! target = "explanation/architecture.md"
//! first_table = true
//!
//! [crate_map.crate_api]
//! site = "myco"
//! out = "reference/crate-api.md"
//!
//! [cli]
//! package = "myco-driver"     # cargo -p to build (omit → `bin` on PATH)
//! bin = "tamias-driver-myco"   # binary under target/debug, or PATH name
//! prefix = ["run"]             # argv prefix before the engine argv
//! out = "reference/cli.md"     # optional emit target for `check`
//!
//! [config]
//! title = "Config — `myco.toml`"
//! types = { "String" = "string", "PathBuf" = "path" }
//! field_types = { "aliases" = "{path: cuora://ref}" }
//! all_optional = ["Config"]
//! out = "reference/config.md"
//!
//! [[config.structs]]
//! file = "crates/myco-core/src/lib.rs"
//! name = "Config"
//! title = "Top level"
//!
//! [[config.sections]]
//! level = 2
//! title = "Constraints"
//! text = """
//! - `deny_unknown_fields`: typos fail the load.
//! """
//! ```
//!
//! Pure model + TOML decoding — the IO layer (discovery, file
//! reads) lives in `cypera-cli`.

use std::collections::BTreeMap;

use serde::Deserialize;

use crate::error::CyperaError;

/// A decoded `cypera.toml`.
#[derive(Debug, Clone, Default, Deserialize)]
pub struct RepoConfig {
    /// `[crate_map]` — crate-map generation knobs.
    #[serde(default)]
    pub crate_map: CrateMapConfig,
    /// `[cli]` — the `--help` walk command spec.
    #[serde(default)]
    pub cli: Option<CliConfig>,
    /// `[config]` — the config-reference page spec.
    #[serde(default)]
    pub config: Option<ConfigDocConfig>,
}

/// `[crate_map]` — absorbs the per-repo `LAYER`/`ORDER`/`SITE`
/// dicts and the `<!-- crate-map -->`/first-table splice targets
/// the `gen-crate-map.py` wrappers hardcoded.
#[derive(Debug, Clone, Default, Deserialize)]
pub struct CrateMapConfig {
    /// Layer → sort rank overrides. When set, this *is* the order:
    /// layers listed rank by position, unlisted layers sort last
    /// (then by name). When unset, cypera's canonical rank applies.
    #[serde(default)]
    pub order: Vec<String>,
    /// Layer assigned to crates the heuristic can't classify —
    /// replaces the `adapter` default (the kuda/mimus `"impl"`
    /// convention).
    #[serde(default)]
    pub default_layer: Option<String>,
    /// `crate-name = layer` overrides (the `--layer` flag's
    /// config-file form).
    #[serde(default)]
    pub layers: BTreeMap<String, String>,
    /// In-place splice targets — table-only rewrites of existing
    /// pages (the `explanation/architecture.md` convention).
    #[serde(default)]
    pub splices: Vec<CrateMapSplice>,
    /// The rustdoc link-list page (`reference/crate-api.md`).
    #[serde(default)]
    pub crate_api: Option<CrateApiConfig>,
    /// Full-page emit target (`docs/reference/crate-map.md`) —
    /// written by `gen crate-map` alongside any splices/crate_api
    /// (no `--out` flag), and checked by `cypera check` freshness.
    #[serde(default)]
    pub out: Option<String>,
}

/// One `[[crate_map.splices]]` entry — replace a crate-map table
/// inside an existing page. Exactly one of `marker`/`first_table`
/// must be set (validated by [`CrateMapConfig::validate`]).
#[derive(Debug, Clone, Deserialize)]
pub struct CrateMapSplice {
    /// Target file, relative to the repo root.
    pub target: String,
    /// A line containing this marker keeps its place; the run of
    /// `|…|` table lines after it is replaced (the
    /// `<!-- crate-map -->` convention).
    #[serde(default)]
    pub marker: Option<String>,
    /// `true` → replace the first `| crate | …` table in the file
    /// (the bupha/kuda/mimus wrapper convention).
    #[serde(default)]
    pub first_table: bool,
}

/// `[crate_map.crate_api]` — the rustdoc link list.
#[derive(Debug, Clone, Deserialize)]
pub struct CrateApiConfig {
    /// Site path segment — links render as `/{site}/api/<crate>/index.html`.
    pub site: String,
    /// Output file, relative to the repo root.
    pub out: String,
}

/// `[cli]` — how to reach the CLI for the `--help` walk. Mirrors
/// the `PKG`/`BIN`/`PREFIX` block every `gen-cli-docs.py` carried.
#[derive(Debug, Clone, Deserialize)]
pub struct CliConfig {
    /// Cargo package to `cargo build -q -p` first. When unset the
    /// `bin` name is executed directly (assumed on `PATH` or an
    /// absolute path).
    #[serde(default)]
    pub package: Option<String>,
    /// Binary name under `target/debug/` (cargo mode) or the
    /// executable itself (direct mode).
    pub bin: String,
    /// Argv prefix before the walked subcommand path — the
    /// `run` passthrough shim (`["run"]`) or empty for a plain bin.
    #[serde(default)]
    pub prefix: Vec<String>,
    /// Page title override. Default: `Command-Line Help for
    /// \`<prog>\`` derived from the root usage line (the python
    /// scripts' fixed header).
    #[serde(default)]
    pub title: Option<String>,
    /// Emit target for `cypera check` freshness — the
    /// `docs/reference/cli.md` convention. Generation itself goes
    /// to stdout or `--out`.
    #[serde(default)]
    pub out: Option<String>,
    /// Subcommand paths to leave out of the `--help` walk — space-
    /// joined paths below the program name (`["driver exec"]`). For
    /// passthrough surfaces whose clap command disables its own
    /// `--help` flag (`disable_help_flag`) so a verb's `--help`
    /// reaches the wrapped engine instead. The parent page still
    /// lists the subcommand; only its node is skipped.
    #[serde(default)]
    pub skip: Vec<String>,
}

/// `[config]` — the config-reference page spec. Absorbs the
/// `SRCS`/`TITLES`/`SIMPLE`/`FIELD_LABEL`/curated-prose blocks the
/// `gen-config-docs.py` scripts hardcoded.
#[derive(Debug, Clone, Deserialize)]
pub struct ConfigDocConfig {
    /// Page H1 text (without the leading `# `), e.g.
    /// `"Config — \`myco.toml\`"`.
    pub title: String,
    /// Rust-type → display-type map (the `SIMPLE` dict): looked up
    /// on the `Option`-unwrapped field type.
    #[serde(default)]
    pub types: BTreeMap<String, String>,
    /// Field-name → display-type overrides, applied before `types`
    /// (the `FIELD_LABEL` dict — same Rust type, different TOML
    /// shape).
    #[serde(default)]
    pub field_types: BTreeMap<String, String>,
    /// Struct names whose every field renders `optional` regardless
    /// of `Option`/`serde(default)` (the tamias `Config`
    /// struct-level `#[serde(default)]` convention).
    #[serde(default)]
    pub all_optional: Vec<String>,
    /// The curated struct set — `file` → `pub struct name` scans,
    /// in declaration order. `title` is the section heading text.
    #[serde(default)]
    pub structs: Vec<ConfigStructRef>,
    /// Output ordering/prose. When empty, every `structs` entry
    /// renders as `## <title>` + field table in order. When set,
    /// each entry renders a heading + either a struct table or
    /// curated prose — this is how the tarsi/cuera interleaved
    /// pages are expressed.
    #[serde(default)]
    pub sections: Vec<ConfigSection>,
    /// Emit target for `cypera check` freshness.
    #[serde(default)]
    pub out: Option<String>,
    /// Prose between `# title` and the marker (tarsi's intro
    /// paragraph) — inline text.
    #[serde(default)]
    pub preamble: Option<String>,
    /// Same, read from a file relative to the repo root.
    #[serde(default)]
    pub preamble_file: Option<String>,
    /// Marker comment block verbatim (multi-line allowed) —
    /// overrides the default `generated by cypera` two-liner.
    #[serde(default)]
    pub marker: Option<String>,
    /// Field-table header row override (tarsi's
    /// `| key | type | required | meaning |`).
    #[serde(default)]
    pub table_head: Option<String>,
    /// Field-table separator row override.
    #[serde(default)]
    pub table_sep: Option<String>,
}

/// A `[[config.structs]]` entry — one `pub struct` to document.
#[derive(Debug, Clone, Deserialize)]
pub struct ConfigStructRef {
    /// Rust source file, relative to the repo root.
    pub file: String,
    /// `pub struct` name.
    pub name: String,
    /// Section heading text (rendered as `## <title>` by default;
    /// `[[config.sections]]` can override the level).
    pub title: String,
}

/// A `[[config.sections]]` entry — heading + struct table or
/// curated prose, in page order. `title` omitted renders the body
/// with no heading (the bupha preamble convention).
#[derive(Debug, Clone, Deserialize)]
pub struct ConfigSection {
    /// ATX heading level (`##` = 2). Default 2.
    #[serde(default = "default_level")]
    pub level: u8,
    /// Heading text — for struct sections this is usually the
    /// struct's `title`; for prose sections the literal heading.
    /// `None` → body renders headingless.
    #[serde(default)]
    pub title: Option<String>,
    /// Render the field table of this `[[config.structs]]` name
    /// under the heading (`struct` in TOML).
    #[serde(default, rename = "struct")]
    pub struct_name: Option<String>,
    /// Inline curated markdown body (prose sections).
    #[serde(default)]
    pub text: Option<String>,
    /// Curated markdown body read from a file, relative to the
    /// repo root — for the long curated blocks (tarsi strategy
    /// semantics) that don't belong in TOML.
    #[serde(default)]
    pub text_file: Option<String>,
}

fn default_level() -> u8 {
    2
}

impl RepoConfig {
    /// Parse `cypera.toml` text.
    pub fn parse(path: &str, text: &str) -> Result<Self, CyperaError> {
        let cfg: Self = toml::from_str(text).map_err(|source| CyperaError::Toml {
            path: path.to_string(),
            source,
        })?;
        cfg.validate()?;
        Ok(cfg)
    }

    /// Cross-field validation — splice marker exclusivity, section
    /// sanity. Returns the first problem found.
    pub fn validate(&self) -> Result<(), CyperaError> {
        for (i, s) in self.crate_map.splices.iter().enumerate() {
            let has_marker = s.marker.is_some();
            if has_marker == s.first_table {
                return Err(CyperaError::Config {
                    path: "cypera.toml".to_string(),
                    message: format!(
                        "crate_map.splices[{i}] ({}): set exactly one of `marker` or `first_table`",
                        s.target
                    ),
                });
            }
        }
        if let Some(c) = &self.config {
            for (i, s) in c.sections.iter().enumerate() {
                let bodies = s.text.is_some() as u8
                    + s.text_file.is_some() as u8
                    + s.struct_name.is_some() as u8;
                if bodies == 0 {
                    return Err(CyperaError::Config {
                        path: "cypera.toml".to_string(),
                        message: format!(
                            "config.sections[{i}]: needs `struct`, `text`, or `text_file`"
                        ),
                    });
                }
            }
        }
        Ok(())
    }

    /// `[crate_map]` layer overrides as `(name, layer)` pairs —
    /// BTreeMap order is deterministic (sorted by crate name).
    pub fn layer_overrides(&self) -> Vec<(String, String)> {
        self.crate_map
            .layers
            .iter()
            .map(|(n, l)| (n.clone(), l.clone()))
            .collect()
    }
}

#[cfg(test)]
mod tests {
    #![allow(clippy::unwrap_used)]
    use super::*;

    #[test]
    fn parses_full_config() {
        let text = r#"
[crate_map]
order = ["domain", "port", "binary"]
default_layer = "impl"

[crate_map.layers]
"myco-core" = "domain"

[[crate_map.splices]]
target = "explanation/architecture.md"
marker = "<!-- crate-map -->"

[crate_map.crate_api]
site = "myco"
out = "reference/crate-api.md"

[cli]
package = "myco-driver"
bin = "tamias-driver-myco"
prefix = ["run"]

[config]
title = "Config — `myco.toml`"
types = { "String" = "string" }
all_optional = ["Config"]

[[config.structs]]
file = "crates/myco-core/src/lib.rs"
name = "Config"
title = "Top level"

[[config.sections]]
title = "Constraints"
text = "- typos fail"
"#;
        let cfg = RepoConfig::parse("cypera.toml", text).unwrap();
        assert_eq!(cfg.crate_map.order, ["domain", "port", "binary"]);
        assert_eq!(cfg.crate_map.default_layer.as_deref(), Some("impl"));
        assert_eq!(cfg.crate_map.layers["myco-core"], "domain");
        assert_eq!(cfg.crate_map.splices.len(), 1);
        assert_eq!(cfg.crate_map.crate_api.as_ref().unwrap().site, "myco");
        let cli = cfg.cli.unwrap();
        assert_eq!(cli.package.as_deref(), Some("myco-driver"));
        assert_eq!(cli.prefix, ["run"]);
        let c = cfg.config.unwrap();
        assert_eq!(c.title, "Config — `myco.toml`");
        assert_eq!(c.structs[0].name, "Config");
        assert_eq!(c.sections[0].title.as_deref(), Some("Constraints"));
        assert_eq!(c.sections[0].level, 2);
    }

    #[test]
    fn empty_is_default() {
        let cfg = RepoConfig::parse("cypera.toml", "").unwrap();
        assert!(cfg.crate_map.splices.is_empty());
        assert!(cfg.cli.is_none());
        assert!(cfg.config.is_none());
    }

    #[test]
    fn splice_requires_exactly_one_mode() {
        let text = r#"
[[crate_map.splices]]
target = "a.md"
"#;
        assert!(RepoConfig::parse("cypera.toml", text).is_err());
        let text = r#"
[[crate_map.splices]]
target = "a.md"
marker = "x"
first_table = true
"#;
        assert!(RepoConfig::parse("cypera.toml", text).is_err());
    }

    #[test]
    fn section_needs_a_body() {
        let text = r#"
[config]
title = "t"

[[config.sections]]
title = "empty"
"#;
        assert!(RepoConfig::parse("cypera.toml", text).is_err());
    }
}
