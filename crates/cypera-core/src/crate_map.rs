//! Crate-map model + markdown renderer — the reusable successor to
//! the per-repo `docs/gen-crate-map.py` scripts.
//!
//! Input is already-decoded [`WorkspaceManifest`]/[`CrateManifest`]
//! values plus optional entry-file scans ([`EntryScan`]); output is
//! a markdown page. No I/O.

use crate::manifest::{CrateManifest, WorkspaceManifest};

/// A crate's entry file (`src/lib.rs` or `src/main.rs`) scan —
/// module declarations and the `//!` crate doc, extracted by the
/// IO layer.
#[derive(Debug, Clone, Default)]
pub struct EntryScan {
    /// `//!` inner doc lines, joined.
    pub doc: String,
    /// Top-level `mod`/`pub mod` declarations, in source order.
    pub modules: Vec<String>,
    /// Whether the scanned file was `src/lib.rs` (vs `main.rs`).
    pub is_lib: bool,
}

/// One row of the crate map.
#[derive(Debug, Clone)]
pub struct CrateEntry {
    /// `[package] name`.
    pub name: String,
    /// Repo-relative manifest dir, e.g. `crates/cypera-core`.
    pub path: String,
    /// `[package] description`.
    pub description: String,
    /// Architectural layer guess (`domain`/`port`/`adapter`/
    /// `binary`/`ops`) — heuristic, overridable by the caller.
    pub layer: String,
    /// Entry-file scan, when the IO layer provided one.
    pub entry: Option<EntryScan>,
}

/// The whole crate map for a repo.
#[derive(Debug, Clone)]
pub struct CrateMap {
    /// Repo display name (dir basename or `--repo` label).
    pub repo: String,
    /// Entries in render order (sorted by layer then name).
    pub crates: Vec<CrateEntry>,
}

/// Conventional layer guess from the crate name — the port-and-
/// adapter naming convention (`<repo>-core` is domain, `-cli` the
/// binary, bare `<repo>` the composition root, everything else an
/// adapter/port). Callers may override per crate.
pub fn guess_layer(name: &str, repo: &str) -> String {
    if name == repo {
        return "binary".to_string();
    }
    let suffix = name.strip_prefix(repo).unwrap_or(name);
    match suffix {
        "-core" | "-domain" => "domain",
        "-cli" => "binary",
        _ => "adapter",
    }
    .to_string()
}

/// Build a [`CrateMap`] from a parsed workspace manifest and the
/// per-member crate manifests. `members` pairs each declared member
/// path with its parsed manifest; `entries` optionally pairs member
/// path → [`EntryScan`].
pub fn crate_map_from_workspace(
    repo: &str,
    ws: &WorkspaceManifest,
    members: &[(String, CrateManifest)],
    entries: &[(String, EntryScan)],
    layer_overrides: &[(String, String)],
) -> Result<CrateMap, crate::error::CyperaError> {
    let mut crates = Vec::with_capacity(members.len());
    for (path, manifest) in members {
        let layer = layer_overrides
            .iter()
            .find(|(n, _)| n == manifest.name())
            .map(|(_, l)| l.clone())
            .unwrap_or_else(|| guess_layer(manifest.name(), repo));
        let entry = entries
            .iter()
            .find(|(p, _)| p == path)
            .map(|(_, e)| e.clone());
        crates.push(CrateEntry {
            name: manifest.name().to_string(),
            path: path.clone(),
            description: manifest.description().to_string(),
            layer,
            entry,
        });
    }
    if crates.is_empty() && !ws.members().is_empty() {
        return Err(crate::error::CyperaError::NoMembers {
            path: repo.to_string(),
        });
    }
    // Stable layer order, then name — deterministic diffs.
    fn rank(layer: &str) -> u8 {
        match layer {
            "domain" => 0,
            "port" => 1,
            "ops" => 2,
            "adapter" => 3,
            "binary" => 4,
            _ => 5,
        }
    }
    crates.sort_by(|a, b| {
        rank(&a.layer)
            .cmp(&rank(&b.layer))
            .then(a.name.cmp(&b.name))
    });
    Ok(CrateMap {
        repo: repo.to_string(),
        crates,
    })
}

/// Scan an entry-file source (`src/lib.rs` or `src/main.rs`) for the
/// crate map's detail section: leading `//!` inner doc lines and
/// top-level `mod`/`pub mod` declarations. Pure string scanning —
/// `mod foo { … }` inline blocks and `#[path]` attributes are not
/// resolved (the module name is still listed).
pub fn scan_entry_source(source: &str, is_lib: bool) -> EntryScan {
    let mut doc_lines = Vec::new();
    let mut modules = Vec::new();
    let mut in_doc_block = true;
    for line in source.lines() {
        let t = line.trim();
        if in_doc_block {
            if let Some(rest) = t.strip_prefix("//!") {
                doc_lines.push(rest.trim().to_string());
                continue;
            }
            // `//!` docs end at the first non-doc, non-attribute,
            // non-empty line that isn't a module doc continuation.
            if t.is_empty()
                || t.starts_with("//!")
                || t.starts_with("#[")
                || t.starts_with("#![")
                || t.starts_with("//")
            {
                continue;
            }
            in_doc_block = false;
        }
        // Top-level `mod` declarations only — indented lines are
        // inside items and skipped.
        if !line.starts_with(char::is_whitespace) {
            let decl = t
                .strip_prefix("pub mod ")
                .or_else(|| t.strip_prefix("mod "));
            if let Some(rest) = decl {
                if let Some(name) = rest
                    .trim_end_matches(';')
                    .split(|c: char| c.is_whitespace() || c == '{')
                    .next()
                {
                    if !name.is_empty() {
                        modules.push(name.to_string());
                    }
                }
            }
        }
    }
    EntryScan {
        // Keep line structure — `//!` lists/paragraphs are markdown.
        doc: doc_lines.join("\n"),
        modules,
        is_lib,
    }
}

/// Escape a cell for a markdown table.
fn cell(s: &str) -> String {
    s.replace('|', "\\|")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
}

/// The bare `| crate | layer | purpose |` table — used both by
/// [`render_crate_map`] and by `--out` marker splicing (the
/// `<!-- crate-map -->` convention the per-repo python scripts
/// used inside `explanation/architecture.md`).
pub fn crate_map_table(map: &CrateMap) -> String {
    let mut out = vec![
        "| crate | layer | purpose |".to_string(),
        "|---|---|---|".to_string(),
    ];
    for c in &map.crates {
        out.push(format!(
            "| `{}` | {} | {} |",
            c.name,
            c.layer,
            cell(&c.description)
        ));
    }
    out.join("\n")
}

/// Render the crate map as a markdown page: summary table, then a
/// per-crate section with the lib.rs doc and module list when an
/// [`EntryScan`] was supplied.
pub fn render_crate_map(map: &CrateMap) -> String {
    let mut out = vec![
        format!("# Crate map — `{}`", map.repo),
        String::new(),
        "<!-- generated by cypera gen crate-map — edit the sources, not this file -->".into(),
        String::new(),
    ];
    out.push(crate_map_table(map));
    let detailed = map.crates.iter().any(|c| c.entry.is_some());
    if detailed {
        out.push(String::new());
        out.push("## Modules".into());
        out.push(String::new());
        for c in &map.crates {
            let Some(e) = &c.entry else { continue };
            out.push(format!("### `{}`", c.name));
            out.push(String::new());
            if !e.doc.is_empty() {
                out.push(e.doc.clone());
                out.push(String::new());
            }
            if !e.modules.is_empty() {
                let mods = e
                    .modules
                    .iter()
                    .map(|m| format!("`{m}`"))
                    .collect::<Vec<_>>()
                    .join(", ");
                out.push(format!(
                    "Modules: {mods}. Entry: `{}`.",
                    if e.is_lib {
                        "src/lib.rs"
                    } else {
                        "src/main.rs"
                    }
                ));
            } else {
                out.push(format!(
                    "No module declarations. Entry: `{}`.",
                    if e.is_lib {
                        "src/lib.rs"
                    } else {
                        "src/main.rs"
                    }
                ));
            }
            out.push(String::new());
        }
    }
    out.join("\n") + "\n"
}

#[cfg(test)]
mod tests {
    #![allow(clippy::unwrap_used)]
    use super::*;

    fn ws() -> WorkspaceManifest {
        WorkspaceManifest::parse(
            "Cargo.toml",
            "[workspace]\nmembers = [\"crates/core\", \"crates/cli\"]\n",
        )
        .unwrap()
    }

    #[test]
    fn crate_map_sorts_domain_first() {
        let members = vec![
            (
                "crates/cli".to_string(),
                CrateManifest::parse(
                    "crates/cli/Cargo.toml",
                    "[package]\nname = \"repo-cli\"\ndescription = \"the cli\"\n",
                )
                .unwrap(),
            ),
            (
                "crates/core".to_string(),
                CrateManifest::parse(
                    "crates/core/Cargo.toml",
                    "[package]\nname = \"repo-core\"\ndescription = \"the domain\"\n",
                )
                .unwrap(),
            ),
        ];
        let map = crate_map_from_workspace("repo", &ws(), &members, &[], &[]).unwrap();
        assert_eq!(map.crates[0].name, "repo-core");
        assert_eq!(map.crates[0].layer, "domain");
        assert_eq!(map.crates[1].layer, "binary");
    }

    #[test]
    fn render_emits_table_and_modules() {
        let members = vec![(
            "crates/core".to_string(),
            CrateManifest::parse(
                "crates/core/Cargo.toml",
                "[package]\nname = \"repo-core\"\ndescription = \"d | e\"\n",
            )
            .unwrap(),
        )];
        let entries = vec![(
            "crates/core".to_string(),
            EntryScan {
                doc: "Crate docs.".into(),
                modules: vec!["a".into(), "b".into()],
                is_lib: true,
            },
        )];
        let map = crate_map_from_workspace("repo", &ws(), &members, &entries, &[]).unwrap();
        let md = render_crate_map(&map);
        assert!(md.contains("| `repo-core` | domain | d \\| e |"));
        assert!(md.contains("Modules: `a`, `b`. Entry: `src/lib.rs`."));
    }
}
