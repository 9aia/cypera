//! `gen api` — the `docs/gen-api.py` + `docs/gen-api-index.py`
//! port's pure half: the styled rustdoc index page. The IO half
//! (`cargo doc`, `cargo metadata`, the `<target>/doc` →
//! `docs/public/api` copy) lives in `cypera-cli`; everything here
//! is strings in, page out.

/// Render `public/api/index.html` — byte-for-byte the python
/// `gen-api-index.py` output: one `<li>` per crate dir (the
/// caller has already prefix-filtered and sorted the names),
/// `_` → `-` in the link label.
pub fn render_api_index(title: &str, crates: &[String]) -> String {
    let links = crates
        .iter()
        .map(|c| {
            format!(
                "  <li><a href=\"{c}/index.html\">{}</a></li>",
                c.replace('_', "-")
            )
        })
        .collect::<Vec<_>>()
        .join("\n");
    format!(
        "<!doctype html><meta charset=utf-8>\
         <title>{title}</title>\
         <style>body{{background:#1b1b1f;color:#ddd;\
         font-family:ui-monospace,monospace;\
         max-width:640px;margin:4em auto;\
         padding:0 1em}}\
         a{{color:#a8b1ff;text-decoration:none}}\
         a:hover{{text-decoration:underline}}\
         li{{line-height:1.9}}\
         h1{{font-size:1.3em}}</style>\
         <h1>{title}</h1>\n<ul>\n{links}\n</ul>\n"
    )
}

#[cfg(test)]
mod tests {
    #![allow(clippy::unwrap_used)]
    use super::*;

    /// The exact bytes `gen-api-index.py` writes for a two-crate
    /// site — guards the port against drift.
    #[test]
    fn index_matches_python_output() {
        let page = render_api_index(
            "tamias — Crate API",
            &["tamias".to_string(), "tamias_core".to_string()],
        );
        let expected = concat!(
            "<!doctype html><meta charset=utf-8>",
            "<title>tamias — Crate API</title>",
            "<style>body{background:#1b1b1f;color:#ddd;font-family:",
            "ui-monospace,monospace;max-width:640px;margin:4em auto;",
            "padding:0 1em}a{color:#a8b1ff;text-decoration:none}",
            "a:hover{text-decoration:underline}li{line-height:1.9}",
            "h1{font-size:1.3em}</style>",
            "<h1>tamias — Crate API</h1>\n<ul>\n",
            "  <li><a href=\"tamias/index.html\">tamias</a></li>\n",
            "  <li><a href=\"tamias_core/index.html\">tamias-core</a></li>",
            "\n</ul>\n",
        );
        assert_eq!(page, expected);
    }

    #[test]
    fn empty_site_still_renders_list_shell() {
        let page = render_api_index("x — Crate API", &[]);
        assert!(page.contains("<h1>x — Crate API</h1>\n<ul>\n\n</ul>\n"));
    }
}
