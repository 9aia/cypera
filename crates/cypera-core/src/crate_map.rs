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
    // Stable layer order, then name — deterministic diffs. The rank
    // is the union of the vocabularies the per-repo `gen-crate-map.py`
    // scripts used (`domain`/`port`/`adapter`/`service`/`engine`/
    // `driver`/`testkit`/`tooling`), which every repo's ORDER table
    // agreed on relatively; `ops`/`binary` keep their heuristic
    // slots, unknown layers sort last.
    fn rank(layer: &str) -> u8 {
        match layer {
            "domain" => 0,
            "port" => 1,
            "adapter" => 2,
            "ops" => 3,
            "service" => 4,
            "engine" => 5,
            "driver" => 6,
            "testkit" => 7,
            "tooling" => 8,
            "binary" => 9,
            _ => 10,
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

/// Escape a cell for a markdown table — pipes, angle brackets (Vue
/// element parsing), and `{{`/`}}` (Vue interpolation — the per-repo
/// scripts emitted `&#123;&#123;` for Jinja examples).
fn cell(s: &str) -> String {
    s.replace('|', "\\|")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace("{{", "&#123;&#123;")
        .replace("}}", "&#125;&#125;")
}

/// Collapse an entry-file `//!` doc block into a one-line table-cell
/// purpose — the `tidy()` half of the per-repo `gen-crate-map.py`
/// `description()` fallback: drop code fences, unwrap `[text](url)`
/// links, `[`code`]` → `` `code` ``, collapse whitespace, cap at
/// ~200 chars on a sentence boundary.
pub fn purpose_from_entry_doc(doc: &str) -> String {
    let text = doc.split("```").next().unwrap_or("");
    let text = unwrap_links(text);
    let text = text.replace("[`", "`").replace("`]", "`");
    let text = text.split_whitespace().collect::<Vec<_>>().join(" ");
    if text.chars().count() <= 200 {
        return text;
    }
    let cut: String = text.chars().take(200).collect();
    // Python parity: `rfind` indexes are *char* positions there —
    // `str::rfind` here yields bytes, so convert before the `> 60`
    // comparison and the slice (multibyte chars like `—`/`…` shift
    // the two).
    let dot = [". ", "! ", "? "]
        .iter()
        .filter_map(|p| cut.rfind(p))
        .max()
        .map(|b| cut[..b].chars().count())
        .unwrap_or(0);
    let mut out = if dot > 60 {
        cut.chars().take(dot + 1).collect()
    } else {
        format!("{}…", cut.trim_end())
    };
    while out.ends_with([' ', ',', ';', ':']) {
        out.pop();
    }
    out
}

/// `[text](url)` → `text` — the markdown-link unwrap from the python
/// `tidy()` (`re.sub(r"\[([^\]]*)\]\([^)]*\)", r"\1", text)`).
/// Non-nested only; good enough for doc-comment prose.
fn unwrap_links(text: &str) -> String {
    let mut out = String::with_capacity(text.len());
    let mut rest = text;
    while let Some(open) = rest.find('[') {
        out.push_str(&rest[..open]);
        let after_open = &rest[open + 1..];
        match after_open.find("](") {
            Some(close) if !after_open[..close].contains(']') => {
                let label = &after_open[..close];
                let tail = &after_open[close + 2..];
                match tail.find(')') {
                    Some(end) => {
                        out.push_str(label);
                        rest = &tail[end + 1..];
                    }
                    None => {
                        out.push('[');
                        rest = after_open;
                    }
                }
            }
            _ => {
                out.push('[');
                rest = after_open;
            }
        }
    }
    out.push_str(rest);
    out
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
    fn purpose_tidy_caps_at_sentence_boundary() {
        // The cicada-core lib.rs doc — python `tidy()` parity: the
        // last ". " sits at index 60 (not >60), so the cap appends
        // "…" after the full 200-char cut.
        let doc = "`cicada-core` — pure domain, ports, and planning scaffolding.\n\n\
                   Layered per ADR-0001: `domain` holds entities and value objects,\n\
                   `application` the use cases, `ports` the seven traits implemented\n\
                   by adapters, and `planning` the pure diff/plan machinery\n\
                   (scaffolded in build step 1).\n\n\
                   Purity contract: no `std::fs`, no `tokio`, no network or HTTP\n\
                   client types anywhere in this crate. All I/O lives behind the\n\
                   port traits so the same core serves the CLI and the MCP server.";
        let out = purpose_from_entry_doc(doc);
        assert!(out.ends_with("adap…"), "got: {out}");
        assert_eq!(out.chars().count(), 201);
    }

    #[test]
    fn purpose_tidy_short_doc_passes_through() {
        assert_eq!(
            purpose_from_entry_doc("`x` — does a thing.\n\nMore detail."),
            "`x` — does a thing. More detail."
        );
    }

    #[test]
    fn purpose_tidy_drops_fences_and_unwraps_links() {
        let doc = "See [`the
                   port`](ports.rs) for details.\n```\ncode\n```\nafter";
        assert_eq!(purpose_from_entry_doc(doc), "See `the port` for details.");
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
