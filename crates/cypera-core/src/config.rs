//! Config-reference generation — scan Rust source for `pub struct`
//! definitions and their `///` doc comments, render the field
//! table. Successor to the per-repo `docs/gen-config-docs.py`
//! scripts, generalized: every `pub struct` in the input files is
//! documented (no per-repo source map).
//!
//! Pure string scanning — no parsing crate, no I/O. It understands
//! exactly the doc-comment conventions the org's config structs
//! use: `///` line docs on the struct and on `pub` fields,
//! `Option<…>`/`serde(default)` ⇒ optional.

/// One field row of a config struct.
#[derive(Debug, Clone)]
pub struct ConfigField {
    /// Field name.
    pub name: String,
    /// Rust type, normalized for display (`Option<T>` unwrapped to
    /// `T` with `optional` set).
    pub ty: String,
    /// `true` when the field is optional (`Option<…>` or a
    /// `serde(default…)` attribute in its prelude).
    pub optional: bool,
    /// Joined `///` doc text (`"—"` never stored; empty means none).
    pub doc: String,
    /// `true` when the field carried a `///`/`#[…]` prelude — the
    /// `gen-config-docs.py` scripts only document fields with one
    /// (a bare `pub x: T` is skipped).
    pub documented: bool,
}

/// A `pub struct` and its documented fields.
#[derive(Debug, Clone)]
pub struct ConfigStruct {
    /// Struct name.
    pub name: String,
    /// Struct-level `///` doc, if any.
    pub doc: String,
    /// Documented `pub` fields, in source order.
    pub fields: Vec<ConfigField>,
}

/// Strip a leading `pub ` and trailing `,`/`;` from a field decl.
fn clean_decl(decl: &str) -> Option<(&str, &str)> {
    let decl = decl.trim().trim_end_matches([',', ';']).trim();
    let rest = decl.strip_prefix("pub ")?;
    let (name, ty) = rest.split_once(':')?;
    let name = name.trim();
    if name.is_empty() || !name.chars().all(|c| c.is_alphanumeric() || c == '_') {
        return None;
    }
    Some((name, ty.trim()))
}

/// `true` when the prelude lines carry a `serde(default…)`
/// attribute — the python scripts matched the literal
/// `serde(default` substring, so `serde(rename=…, default)` does
/// *not* count.
fn has_serde_default(prelude: &[String]) -> bool {
    prelude.iter().any(|l| l.contains("serde(default"))
}

/// Join `///` doc lines out of prelude lines (`#[…]` attributes are
/// ignored except for the serde-default check).
fn join_doc(prelude: &[String]) -> String {
    prelude
        .iter()
        .filter_map(|l| l.trim().strip_prefix("///"))
        .map(str::trim)
        .collect::<Vec<_>>()
        .join(" ")
}

/// `Option<…>` unwrap — returns the inner type text when the outer
/// type is exactly `Option<…>` (naive angle match; good enough for
/// config fields).
fn unwrap_option(ty: &str) -> Option<&str> {
    let inner = ty.strip_prefix("Option<")?.strip_suffix('>')?;
    Some(inner.trim())
}

/// Scan Rust source for `pub struct` blocks and their documented
/// `pub` fields. Structs without any doc comments or pub fields are
/// still returned (callers filter); nested/inner items are ignored —
/// scanning is line-oriented and stops at the first `}` at column 0.
pub fn scan_config_structs(source: &str) -> Vec<ConfigStruct> {
    let mut out = Vec::new();
    let mut pending_doc: Vec<String> = Vec::new();
    let mut cur: Option<ConfigStruct> = None;
    let mut field_doc: Vec<String> = Vec::new();

    for line in source.lines() {
        let t = line.trim();
        if cur.is_some() {
            if t.starts_with('}') {
                if let Some(c) = cur.take() {
                    out.push(c);
                }
                continue;
            }
            if t.starts_with("///") || t.starts_with("#[") {
                field_doc.push(t.to_string());
                continue;
            }
            if let Some(c) = cur.as_mut() {
                if let Some((name, ty)) = clean_decl(t) {
                    let optional = ty.starts_with("Option<") || has_serde_default(&field_doc);
                    let inner = unwrap_option(ty).unwrap_or(ty);
                    c.fields.push(ConfigField {
                        name: name.to_string(),
                        ty: inner.to_string(),
                        optional,
                        doc: join_doc(&field_doc),
                        documented: !field_doc.is_empty(),
                    });
                }
            }
            field_doc.clear();
            continue;
        }
        if t.starts_with("///") {
            pending_doc.push(t.to_string());
            continue;
        }
        if t.starts_with("#[") {
            // Attributes between doc and struct — keep the doc pending.
            continue;
        }
        if let Some(rest) = t.strip_prefix("pub struct ") {
            let name = rest
                .split(|c: char| c.is_whitespace() || c == '<' || c == '{')
                .next()
                .unwrap_or("")
                .trim_end_matches('{')
                .to_string();
            if !name.is_empty() && t.contains('{') {
                cur = Some(ConfigStruct {
                    name,
                    doc: join_doc(&pending_doc),
                    fields: Vec::new(),
                });
                pending_doc.clear();
                field_doc.clear();
            }
        } else if !t.is_empty() && !t.starts_with("//") {
            pending_doc.clear();
        }
    }
    if let Some(c) = cur {
        out.push(c);
    }
    out
}

/// Escape a cell for a markdown table.
fn cell(s: &str) -> String {
    s.replace('|', "\\|")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
}

/// Render config structs as a markdown reference page — one
/// `## <struct>` section per struct with a field table.
pub fn render_config_page(title: &str, structs: &[ConfigStruct]) -> String {
    let mut out = vec![
        format!("# {title}"),
        String::new(),
        "<!-- generated by cypera gen config — edit the struct doc comments,".into(),
        "     not this file -->".into(),
        String::new(),
    ];
    for s in structs {
        out.push(format!("## `{}`", s.name));
        out.push(String::new());
        if !s.doc.is_empty() {
            out.push(s.doc.clone());
            out.push(String::new());
        }
        out.push("| field | type | required | notes |".into());
        out.push("|---|---|---|---|".into());
        for f in &s.fields {
            let req = if f.optional { "optional" } else { "required" };
            let doc = if f.doc.is_empty() {
                "—".to_string()
            } else {
                f.doc.clone()
            };
            out.push(format!(
                "| `{}` | `{}` | {} | {} |",
                f.name,
                cell(&f.ty),
                req,
                cell(&doc)
            ));
        }
        out.push(String::new());
    }
    out.join("\n") + "\n"
}

// ---------- `gen-config-docs.py`-parity page ----------

/// One section of a curated config page — a heading plus either a
/// struct field table or verbatim prose. Mirrors a
/// `[[config.sections]]` entry after IO resolution.
#[derive(Debug, Clone)]
pub struct ConfigDocSection {
    /// ATX level (`##` = 2).
    pub level: u8,
    /// Heading text — `None` renders the body headingless (the
    /// preamble convention).
    pub title: Option<String>,
    /// Struct field table, when this is a struct section.
    pub strukt: Option<ConfigStruct>,
    /// Verbatim markdown body, when this is a prose section.
    pub body: Option<String>,
}

/// Escape a notes cell — python `doc.replace("|","\\|")` +
/// `<`/`>` entity escape.
fn escape_notes(s: &str) -> String {
    s.replace('|', "\\|")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
}

/// Display type for a field — `field_types[name]` →
/// `types[inner]` → raw inner type (the `FIELD_LABEL`/`SIMPLE`
/// lookup chain).
pub fn type_label(
    field_name: &str,
    inner_ty: &str,
    types: &std::collections::BTreeMap<String, String>,
    field_types: &std::collections::BTreeMap<String, String>,
) -> String {
    if let Some(t) = field_types.get(field_name) {
        return t.clone();
    }
    if let Some(t) = types.get(inner_ty) {
        return t.clone();
    }
    inner_ty.to_string()
}

/// Page-chrome knobs for [`render_config_doc_page`] — the per-repo
/// variations the python scripts hardcoded (tarsi's preamble,
/// custom marker text, `key/meaning` table header).
#[derive(Debug, Clone, Default)]
pub struct ConfigDocStyle {
    /// Prose emitted between the `# title` and the generated
    /// marker (tarsi's "The config document is …" preamble).
    pub preamble: Option<String>,
    /// Marker comment block, verbatim (may be multi-line). Default:
    /// the standard `generated by cypera gen config` two-liner.
    pub marker: Option<String>,
    /// Field-table header row override (tarsi's
    /// `| key | type | required | meaning |`).
    pub table_head: Option<String>,
    /// Field-table separator row override.
    pub table_sep: Option<String>,
}

/// Render one struct's field table exactly as the python
/// `gen-config-docs.py` scripts did: only fields with a
/// `///`/`#[…]` prelude, `required`/`optional` column, `—` for
/// empty notes.
fn struct_table(
    s: &ConfigStruct,
    types: &std::collections::BTreeMap<String, String>,
    field_types: &std::collections::BTreeMap<String, String>,
    style: &ConfigDocStyle,
) -> Vec<String> {
    let mut out = vec![
        style
            .table_head
            .clone()
            .unwrap_or_else(|| "| field | type | required | notes |".to_string()),
        style
            .table_sep
            .clone()
            .unwrap_or_else(|| "|---|---|---|---|".to_string()),
    ];
    for f in &s.fields {
        if !f.documented {
            continue;
        }
        let ty = type_label(&f.name, &f.ty, types, field_types);
        let req = if f.optional { "optional" } else { "required" };
        let doc = if f.doc.is_empty() {
            "—".to_string()
        } else {
            escape_notes(&f.doc)
        };
        out.push(format!("| `{}` | `{ty}` | {req} | {doc} |", f.name));
    }
    out
}

/// Render the curated config page — byte-parity port of the
/// `gen-config-docs.py` output shape:
///
/// ```text
/// # <title>
/// <!-- generated by cypera gen config — edit the struct doc comments,
///      not this file -->
///
/// ## <section title>
///
/// | field | type | required | notes |
/// |---|---|---|---|
/// …
/// ```
///
/// Sections render in order; a struct section emits its field
/// table, a prose section emits `body` verbatim under its heading.
pub fn render_config_doc_page(
    title: &str,
    sections: &[ConfigDocSection],
    types: &std::collections::BTreeMap<String, String>,
    field_types: &std::collections::BTreeMap<String, String>,
    style: &ConfigDocStyle,
) -> String {
    let mut out: Vec<String> = vec![format!("# {title}")];
    if let Some(pre) = &style.preamble {
        out.push(String::new());
        out.push(pre.trim_end().to_string());
    }
    match &style.marker {
        Some(m) => out.extend(m.lines().map(str::to_string)),
        None => out.extend([
            "<!-- generated by cypera gen config — edit the struct doc comments,".to_string(),
            "     not this file -->".to_string(),
        ]),
    }
    out.push(String::new());
    for s in sections {
        if let Some(title) = &s.title {
            out.push(format!("{} {}", "#".repeat(s.level.max(1) as usize), title));
            out.push(String::new());
        }
        if let Some(st) = &s.strukt {
            out.extend(struct_table(st, types, field_types, style));
        }
        if let Some(body) = &s.body {
            // `struct` + prose sections (tarsi sync-rule) need the
            // blank line between table and prose the python
            // scripts emitted.
            if s.strukt.is_some() {
                out.push(String::new());
            }
            out.push(body.trim_end().to_string());
        }
        out.push(String::new());
    }
    out.join("\n")
}

#[cfg(test)]
mod tests {
    #![allow(clippy::unwrap_used)]
    use super::*;

    const SRC: &str = r#"//! module docs

/// Top-level config.
#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Config {
    /// Where the ledger lives.
    pub dir: PathBuf,
    /// Retry count.
    #[serde(default)]
    pub retries: u32,
    /// Optional override.
    pub endpoint: Option<String>,
    secret: String, // not pub — ignored
}

pub struct NotDocumented {
    pub hidden: bool,
}
"#;

    #[test]
    fn scans_pub_struct_fields() {
        let structs = scan_config_structs(SRC);
        assert_eq!(structs.len(), 2);
        let cfg = &structs[0];
        assert_eq!(cfg.name, "Config");
        assert_eq!(cfg.doc, "Top-level config.");
        assert_eq!(cfg.fields.len(), 3);
        assert_eq!(cfg.fields[0].name, "dir");
        assert!(!cfg.fields[0].optional);
        assert_eq!(cfg.fields[0].doc, "Where the ledger lives.");
        assert!(cfg.fields[1].optional); // serde(default)
        assert_eq!(cfg.fields[1].ty, "u32");
        assert!(cfg.fields[2].optional); // Option<>
        assert_eq!(cfg.fields[2].ty, "String");
    }

    #[test]
    fn renders_field_table() {
        let md = render_config_page("Config — `x.toml`", &scan_config_structs(SRC));
        assert!(md.contains("## `Config`"));
        assert!(md.contains("| `dir` | `PathBuf` | required | Where the ledger lives. |"));
        assert!(md.contains("| `endpoint` | `String` | optional | Optional override. |"));
        assert!(md.contains("## `NotDocumented`"));
    }

    #[test]
    fn serde_default_needs_literal_substring() {
        // `serde(rename = "x", default)` does NOT make the field
        // optional in the python scripts — only `serde(default`.
        let prelude = vec![
            "#[serde(rename = \"r\", default)]".to_string(),
            "pub f: u8,".to_string(),
        ];
        assert!(!has_serde_default(&prelude));
        let prelude = vec!["#[serde(default)]".to_string()];
        assert!(has_serde_default(&prelude));
    }

    #[test]
    fn curated_page_sections_prose_and_style() {
        let structs = scan_config_structs(SRC);
        let cfg = structs.iter().find(|s| s.name == "Config").unwrap();
        let types = std::collections::BTreeMap::new();
        let field_types = std::collections::BTreeMap::new();
        let style = ConfigDocStyle {
            preamble: Some("Preamble prose.".to_string()),
            marker: Some("<!-- custom marker -->".to_string()),
            table_head: Some("| key | type | required | meaning |".to_string()),
            table_sep: None,
        };
        let sections = vec![
            ConfigDocSection {
                level: 2,
                title: Some("`Config`".to_string()),
                strukt: Some(cfg.clone()),
                body: Some("Trailing prose.".to_string()),
            },
            ConfigDocSection {
                level: 2,
                title: Some("Notes".to_string()),
                strukt: None,
                body: Some("Extra notes.".to_string()),
            },
        ];
        let md =
            render_config_doc_page("Config — `x.toml`", &sections, &types, &field_types, &style);
        assert!(md.starts_with("# Config — `x.toml`\n\nPreamble prose.\n<!-- custom marker -->"));
        assert!(md.contains("| key | type | required | meaning |"));
        assert!(md.contains("| `dir` | `PathBuf` | required | Where the ledger lives. |"));
        // Only documented fields — `hidden` never appears anywhere.
        assert!(!md.contains("hidden"));
        // Struct + prose section gets the blank separator line.
        assert!(md.contains(
            "| `endpoint` | `String` | optional | Optional override. |\n\nTrailing prose."
        ));
        assert!(md.contains("## Notes\n\nExtra notes."));
    }

    #[test]
    fn type_label_lookup_chain() {
        let mut types = std::collections::BTreeMap::new();
        types.insert("Vec<String>".to_string(), "list<string>".to_string());
        let mut field_types = std::collections::BTreeMap::new();
        field_types.insert("repos".to_string(), "list<repo>".to_string());
        assert_eq!(
            type_label("repos", "Vec<String>", &types, &field_types),
            "list<repo>"
        );
        assert_eq!(
            type_label("other", "Vec<String>", &types, &field_types),
            "list<string>"
        );
        assert_eq!(type_label("x", "u8", &types, &field_types), "u8");
    }
}
