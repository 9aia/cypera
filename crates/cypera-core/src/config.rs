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

/// `true` when the prelude lines carry a `serde(default…)` attribute.
fn has_serde_default(prelude: &[String]) -> bool {
    prelude
        .iter()
        .any(|l| l.contains("serde") && l.contains("default"))
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
}
