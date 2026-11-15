//! `cypera check` — the mechanical docs-contract linter. Each rule
//! reports pass/fail(/skip) with `file:line` evidence; any fail
//! exits non-zero. `--json` emits the report as JSON.

use std::path::Path;

use anyhow::{Context, Result};
use serde_json::{json, Value};

use crate::{
    build_crate_map, cli_help_page, config_dir, config_doc_page, discover_config, repo_name,
    resolve_cli_spec, resolve_path, RepoConfig,
};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Status {
    Pass,
    Fail,
    Skip,
}

impl Status {
    fn as_str(self) -> &'static str {
        match self {
            Status::Pass => "pass",
            Status::Fail => "fail",
            Status::Skip => "skip",
        }
    }
}

struct Rule {
    name: &'static str,
    status: Status,
    evidence: Vec<String>,
}

impl Rule {
    fn new(name: &'static str, status: Status, evidence: Vec<String>) -> Self {
        Self {
            name,
            status,
            evidence,
        }
    }
    fn pass(name: &'static str, evidence: Vec<String>) -> Self {
        Self::new(name, Status::Pass, evidence)
    }
    fn fail(name: &'static str, evidence: Vec<String>) -> Self {
        Self::new(name, Status::Fail, evidence)
    }
    fn skip(name: &'static str, evidence: Vec<String>) -> Self {
        Self::new(name, Status::Skip, evidence)
    }
}

/// Entry point from `run()`.
pub fn run_check(repo: &Path, config: Option<&Path>, json_out: bool) -> Result<i32> {
    let root = repo
        .canonicalize()
        .with_context(|| format!("--repo {} does not exist", repo.display()))?;
    let discovered = discover_config(&root, config)?;
    let cfg = discovered.as_ref().map(|(_, c)| c);
    let base = config_dir(&discovered);

    let mut rules = vec![
        readme_rule(&root),
        agents_rule(&root),
        changelog_rule(&root),
        version_prose_rule(&root)?,
        docs_site_rule(&root),
        freshness_rule(&root, &base, cfg)?,
        docs_check_script_rule(&root),
    ];
    // Deterministic order regardless of construction.
    rules.sort_by_key(|r| r.name);

    // `[check] skip` exemptions — rules still run (evidence is
    // honest) but report SKIP. Unknown names warn and are ignored:
    // a typo must not break `cypera check` for the whole repo.
    if let Some(rc) = cfg {
        for name in &rc.check.skip {
            if !cypera_core::CHECK_RULE_NAMES.contains(&name.as_str()) {
                eprintln!("cypera check: warning: unknown [check] skip rule `{name}`");
            }
        }
        for r in &mut rules {
            if rc.check.skip.iter().any(|s| s == r.name) {
                r.status = Status::Skip;
                r.evidence
                    .push("exempt via cypera.toml [check] skip".into());
            }
        }
    }

    let failed = rules.iter().any(|r| r.status == Status::Fail);
    if json_out {
        let report = json!({
            "repo": root.display().to_string(),
            "config": discovered.as_ref().map(|(p, _)| p.display().to_string()),
            "ok": !failed,
            "rules": rules.iter().map(|r| json!({
                "name": r.name,
                "status": r.status.as_str(),
                "evidence": r.evidence,
            })).collect::<Vec<_>>(),
        });
        println!("{}", serde_json::to_string_pretty(&report)?);
    } else {
        println!("cypera check — {}", root.display());
        for r in &rules {
            let mark = match r.status {
                Status::Pass => "PASS",
                Status::Fail => "FAIL",
                Status::Skip => "SKIP",
            };
            println!("{mark} {}", r.name);
            for e in &r.evidence {
                println!("     {e}");
            }
        }
        println!(
            "{}",
            if failed {
                "cypera check: FAIL"
            } else {
                "cypera check: OK"
            }
        );
    }
    Ok(if failed { 1 } else { 0 })
}

// ---------- rule: README shape ----------

/// README exists, stays a quickstart (<200 lines), and shows an
/// install/run command — a fenced code line starting with a known
/// tool verb or the repo's own binary name.
fn readme_rule(root: &Path) -> Rule {
    const NAME: &str = "readme";
    let path = root.join("README.md");
    let Ok(text) = std::fs::read_to_string(&path) else {
        return Rule::fail(NAME, vec!["README.md: missing".into()]);
    };
    let mut evidence = Vec::new();
    let lines: Vec<&str> = text.lines().collect();
    evidence.push(format!("README.md: {} lines", lines.len()));
    if lines.len() >= 200 {
        evidence.push("README.md: over 200 lines — quickstart, not docs".into());
    }
    // Candidate command names — the remote-aware repo name plus the
    // dir basename (gate worktrees live under hashed dirs where only
    // the remote still says `myco`).
    let mut names = vec![repo_name(root)];
    if let Some(d) = root.file_name().map(|n| n.to_string_lossy().to_lowercase()) {
        if !names.iter().any(|n| n == &d) {
            names.push(d);
        }
    }
    names.retain(|n| !n.is_empty());
    let mut in_fence = false;
    let mut found: Option<usize> = None;
    for (i, line) in lines.iter().enumerate() {
        let t = line.trim();
        if t.starts_with("```") {
            in_fence = !in_fence;
            continue;
        }
        if !in_fence {
            continue;
        }
        let cmd = t.trim_start_matches(['$', '>', ' ']).trim_start();
        let first = cmd.split_whitespace().next().unwrap_or("");
        let known = matches!(
            first,
            "cargo"
                | "bun"
                | "npm"
                | "npx"
                | "pnpm"
                | "yarn"
                | "pip"
                | "pip3"
                | "brew"
                | "go"
                | "apt"
                | "apt-get"
                | "make"
                | "just"
                | "bash"
                | "sh"
                | "sudo"
                | "tamias"
                | "myco"
                | "install"
                | "curl"
        );
        if known || names.iter().any(|n| first.eq_ignore_ascii_case(n)) {
            found = Some(i + 1);
            break;
        }
    }
    match found {
        Some(n) => {
            evidence.push(format!("README.md:{n}: install/run command"));
            if lines.len() < 200 {
                Rule::pass(NAME, evidence)
            } else {
                Rule::fail(NAME, evidence)
            }
        }
        None => {
            evidence.push("README.md: no install/run command in a fenced block".into());
            Rule::fail(NAME, evidence)
        }
    }
}

// ---------- rule: AGENTS.md shape ----------

/// AGENTS.md exists with the conventional sections — at least two
/// of {layout-ish, commands-ish, boundary-ish} headings.
fn agents_rule(root: &Path) -> Rule {
    const NAME: &str = "agents-md";
    let path = root.join("AGENTS.md");
    let Ok(text) = std::fs::read_to_string(&path) else {
        return Rule::fail(NAME, vec!["AGENTS.md: missing".into()]);
    };
    let mut layout = false;
    let mut commands = false;
    let mut boundary = false;
    let mut headings = Vec::new();
    for line in text.lines() {
        let t = line.trim_start();
        if !t.starts_with('#') {
            continue;
        }
        let h = t.trim_start_matches('#').trim().to_lowercase();
        headings.push(h.clone());
        if h.contains("layout") || h.contains("structure") || h.contains("crate") {
            layout = true;
        }
        if h.contains("command") || h.contains("usage") || h.contains("quickstart") {
            commands = true;
        }
        if h.contains("boundary") || h.contains("scope") || h.contains("rules") {
            boundary = true;
        }
    }
    let found = [layout, commands, boundary].iter().filter(|b| **b).count();
    let mut evidence: Vec<String> = headings
        .iter()
        .map(|h| format!("AGENTS.md: ## {h}"))
        .collect();
    evidence.push(format!("section categories present: {found}/3"));
    if found >= 2 {
        Rule::pass(NAME, evidence)
    } else {
        evidence.push("AGENTS.md: needs Layout/Commands/Boundary-ish sections".into());
        Rule::fail(NAME, evidence)
    }
}

// ---------- rule: no CHANGELOG ----------

/// No `CHANGELOG*` at the repo root — the org keeps release notes
/// in myco/tarsi, not per-repo changelog files.
fn changelog_rule(root: &Path) -> Rule {
    const NAME: &str = "no-changelog";
    let mut hits = Vec::new();
    if let Ok(rd) = std::fs::read_dir(root) {
        for e in rd.flatten() {
            let n = e.file_name().to_string_lossy().to_lowercase();
            if n.starts_with("changelog") {
                hits.push(format!("{}: found", e.path().display()));
            }
        }
    }
    if hits.is_empty() {
        Rule::pass(NAME, vec!["no CHANGELOG* at repo root".into()])
    } else {
        Rule::fail(NAME, hits)
    }
}

// ---------- rule: no version numbers in prose ----------

/// Scan prose sources (`.md`/`.txt`) for `v?\d+\.\d+\.\d+`
/// outside fenced code blocks, inline code spans, raw HTML
/// `<script>`/`<style>` blocks, and skipped dirs — versions belong
/// in manifests/lockfiles, not prose. Source files are out of
/// scope: semver literals in code and doc comments are legitimate.
fn version_prose_rule(root: &Path) -> Result<Rule> {
    const NAME: &str = "no-version-in-prose";
    let mut hits = Vec::new();
    let mut scanned = 0usize;
    walk_prose(root, &mut |path, rel| {
        if is_generated_reference(&rel) {
            return;
        }
        scanned += 1;
        let Ok(text) = std::fs::read_to_string(path) else {
            return;
        };
        // Changelogs are version-native by definition.
        let fname = path
            .file_name()
            .map(|f| f.to_string_lossy().to_lowercase())
            .unwrap_or_default();
        if fname.starts_with("changelog") {
            return;
        }
        // `<script>`/`<style>` blocks are code, not prose — VitePress
        // markdown allows raw HTML/JS (SVG path floats look like
        // semver to the scanner).
        let text = mask_html_blocks(&text);
        let mut in_fence = false;
        for (i, line) in text.lines().enumerate() {
            let t = line.trim_start();
            if t.starts_with("```") || t.starts_with("~~~") {
                in_fence = !in_fence;
                continue;
            }
            if in_fence {
                continue;
            }
            // Inline code spans and URLs are not prose claims.
            let masked = mask_urls(&mask_inline_code(line));
            for m in find_versions(&masked) {
                if hits.len() >= 20 {
                    return;
                }
                hits.push(format!("{}:{}: `{m}`", rel, i + 1));
            }
        }
    })?;
    if hits.is_empty() {
        Ok(Rule::pass(
            NAME,
            vec![format!("{scanned} prose files clean")],
        ))
    } else {
        hits.push("...".into());
        Ok(Rule::fail(NAME, hits))
    }
}

/// Directories never scanned for prose.
const SKIP_DIRS: &[&str] = &[
    ".git",
    "target",
    "node_modules",
    "project",
    ".kuda",
    "dist",
    ".vitepress",
];

/// `docs/**/reference/` is generated-only per the docs contract —
/// its content is generator output, gated by the freshness rule.
fn is_generated_reference(rel: &str) -> bool {
    let parts: Vec<&str> = rel.split('/').collect();
    parts.contains(&"reference") && parts.first() == Some(&"docs")
}

/// Recursive prose-file walk — `.md`/`.txt`, skipping
/// [`SKIP_DIRS`] and dot-dirs other than `docs`-style trees.
fn walk_prose(root: &Path, f: &mut impl FnMut(&Path, String)) -> Result<()> {
    let mut stack = vec![root.to_path_buf()];
    while let Some(dir) = stack.pop() {
        for e in std::fs::read_dir(&dir)?.flatten() {
            let p = e.path();
            if p.is_dir() {
                let name = e.file_name().to_string_lossy().to_lowercase();
                if SKIP_DIRS.contains(&name.as_str()) || name.starts_with('.') {
                    continue;
                }
                stack.push(p);
            } else if let Some(ext) = p.extension().and_then(|e| e.to_str()) {
                if matches!(ext, "md" | "txt") {
                    let rel = p
                        .strip_prefix(root)
                        .unwrap_or(&p)
                        .to_string_lossy()
                        .into_owned();
                    f(&p, rel);
                }
            }
        }
    }
    Ok(())
}

/// Blank out `http(s)://…` spans so versions inside URLs don't
/// count as prose claims.
fn mask_urls(line: &str) -> String {
    let chars: Vec<(usize, char)> = line.char_indices().collect();
    let bytes = line.as_bytes();
    let mut masked = vec![false; line.len()];
    let mut i = 0;
    while i + 7 < bytes.len() {
        let is_url = (bytes[i..].starts_with(b"http://") || bytes[i..].starts_with(b"https://"))
            && (i == 0 || !(bytes[i - 1].is_ascii_alphanumeric() || bytes[i - 1] == b'/'));
        if !is_url {
            i += 1;
            continue;
        }
        let mut j = i;
        while j < bytes.len()
            && !bytes[j].is_ascii_whitespace()
            && !matches!(bytes[j], b')' | b']' | b'"' | b'\'')
        {
            j += 1;
        }
        for m in masked.iter_mut().take(j).skip(i) {
            *m = true;
        }
        i = j;
    }
    let mut out = String::with_capacity(line.len());
    for (pos, ch) in chars {
        out.push(if masked[pos] { ' ' } else { ch });
    }
    out
}

/// Blank out the *contents* of raw HTML `<script>`/`<style>`
/// blocks (the tags themselves stay). Markdown allows inline HTML,
/// and VitePress pages carry JS/CSS whose numeric literals are not
/// prose claims. Newlines are preserved so line numbers hold; an
/// unclosed block masks to EOF.
fn mask_html_blocks(text: &str) -> String {
    let lower = text.to_lowercase();
    let bytes = text.as_bytes();
    let mut masked = vec![false; text.len()];
    let mut pos = 0usize;
    while pos < bytes.len() {
        // Next `<script`/`<style` open tag at or after `pos`.
        let mut found: Option<(usize, &'static str)> = None;
        for (tag, close) in [("<script", "</script"), ("<style", "</style")] {
            if let Some(off) = lower[pos..].find(tag) {
                let start = pos + off;
                // Tag-name boundary: next char must end the name.
                let next = bytes.get(start + tag.len()).copied();
                if next.is_some_and(|c| c.is_ascii_whitespace() || c == b'>' || c == b'/')
                    && found.is_none_or(|(s, _)| start < s)
                {
                    found = Some((start, close));
                }
            }
        }
        let Some((open, close)) = found else { break };
        let open_end = open
            + if lower[open..].starts_with("<script") {
                "<script".len()
            } else {
                "<style".len()
            };
        // Find the matching close tag; mask up to (not incl.) it.
        let close_at = lower[open_end..]
            .find(close)
            .map(|o| open_end + o)
            .filter(|&c| {
                bytes
                    .get(c + close.len())
                    .is_none_or(|&n| n == b'>' || n.is_ascii_whitespace())
            });
        let mask_end = close_at.unwrap_or(text.len());
        for (i, m) in masked.iter_mut().enumerate().take(mask_end).skip(open_end) {
            if bytes[i] != b'\n' && bytes[i] != b'\r' {
                *m = true;
            }
        }
        // Continue scanning after the close tag (or stop at EOF).
        pos = match close_at {
            Some(c) => c + close.len(),
            None => text.len(),
        };
    }
    let mut out = String::with_capacity(text.len());
    for (pos, ch) in text.char_indices() {
        out.push(if masked[pos] { ' ' } else { ch });
    }
    out
}

/// Blank out inline code spans — text between matching backtick
/// runs (same run length, per CommonMark). An unmatched run leaves
/// the rest of the line literal. Newlines never appear here (the
/// caller works line-by-line).
fn mask_inline_code(line: &str) -> String {
    let bytes = line.as_bytes();
    let mut masked = vec![false; line.len()];
    let mut i = 0usize;
    while i < bytes.len() {
        if bytes[i] != b'`' {
            i += 1;
            continue;
        }
        let run_start = i;
        while i < bytes.len() && bytes[i] == b'`' {
            i += 1;
        }
        let n = i - run_start;
        // Find the next run of exactly `n` backticks.
        let mut j = i;
        let mut close_end = None;
        while j < bytes.len() {
            if bytes[j] == b'`' {
                let cs = j;
                while j < bytes.len() && bytes[j] == b'`' {
                    j += 1;
                }
                if j - cs == n {
                    close_end = Some(j);
                    break;
                }
            } else {
                j += 1;
            }
        }
        match close_end {
            Some(ce) => {
                for m in masked.iter_mut().take(ce).skip(run_start) {
                    *m = true;
                }
                i = ce;
            }
            // Unmatched opener — the backticks are literal text.
            None => break,
        }
    }
    let mut out = String::with_capacity(line.len());
    for (pos, ch) in line.char_indices() {
        out.push(if masked[pos] { ' ' } else { ch });
    }
    out
}

/// Find `v?\d+\.\d+\.\d+` matches in a line — not embedded in a
/// longer version (`1.2.3.4`) or identifier.
fn find_versions(line: &str) -> Vec<String> {
    let b = line.as_bytes();
    let mut out = Vec::new();
    let mut i = 0;
    while i < b.len() {
        let c = b[i];
        let is_v = (c == b'v' || c == b'V') && i + 1 < b.len() && b[i + 1].is_ascii_digit();
        let is_d = c.is_ascii_digit();
        if !(is_v || is_d) {
            i += 1;
            continue;
        }
        // Preceding char must not be part of a longer token.
        if i > 0 {
            let p = b[i - 1];
            if p.is_ascii_alphanumeric() || p == b'.' || p == b'_' || p == b'-' {
                i += 1;
                continue;
            }
        }
        let start = if is_v { i + 1 } else { i };
        // Parse \d+\.\d+\.\d+
        let mut j = start;
        let mut dots = 0;
        let mut digits = 0;
        while j < b.len() {
            if b[j].is_ascii_digit() {
                digits += 1;
            } else if b[j] == b'.' && digits > 0 {
                dots += 1;
                digits = 0;
            } else {
                break;
            }
            j += 1;
        }
        // Need exactly digit+ . digit+ . digit+ and a following
        // digit run; dots==2 and trailing digits>0.
        if dots == 2 && digits > 0 {
            // Trailing must not continue into .digit (1.2.3.4).
            if j < b.len() && (b[j].is_ascii_digit() || b[j] == b'.') {
                i += 1;
                continue;
            }
            out.push(line[i..j].to_string());
            i = j;
            continue;
        }
        i += 1;
    }
    out
}

// ---------- rule: docs site (Diátaxis) ----------

/// `docs/` exists (the VitePress worktree) with at least two
/// Diátaxis quadrant dirs (tutorials/how-to/reference/explanation
/// or close synonyms).
fn docs_site_rule(root: &Path) -> Rule {
    const NAME: &str = "docs-site";
    let docs = root.join("docs");
    if !docs.is_dir() {
        return Rule::fail(NAME, vec!["docs/: missing — no VitePress worktree".into()]);
    }
    const CANON: &[&str] = &["tutorials", "how-to", "reference", "explanation"];
    const SYNONYMS: &[(&str, &str)] = &[
        ("tutorial", "tutorials"),
        ("howto", "how-to"),
        ("how-tos", "how-to"),
        ("references", "reference"),
        ("explanations", "explanation"),
        ("guides", "how-to"),
        ("guide", "how-to"),
    ];
    let mut quadrants = std::collections::BTreeSet::new();
    let mut names = Vec::new();
    if let Ok(rd) = std::fs::read_dir(&docs) {
        for e in rd.flatten() {
            if !e.path().is_dir() {
                continue;
            }
            let n = e.file_name().to_string_lossy().to_lowercase();
            names.push(n.clone());
            if CANON.contains(&n.as_str()) {
                quadrants.insert(n);
            } else if let Some((_, canon)) = SYNONYMS.iter().find(|(s, _)| *s == n) {
                quadrants.insert(canon.to_string());
            }
        }
    }
    let mut evidence = vec![format!("docs/: dirs {names:?}")];
    evidence.push(format!("diataxis quadrants: {}", quadrants.len()));
    if quadrants.len() >= 2 {
        Rule::pass(NAME, evidence)
    } else {
        evidence.push("docs/: needs >=2 of tutorials/how-to/reference/explanation".into());
        Rule::fail(NAME, evidence)
    }
}

// ---------- rule: generated references are fresh ----------

/// Regenerate every output `cypera.toml` declares (crate-map
/// splices + crate-api, `[cli]` page, `[config]` page) and diff
/// against the checked-in files. Skips when no `cypera.toml`
/// generators are declared.
fn freshness_rule(root: &Path, base: &Path, cfg: Option<&RepoConfig>) -> Result<Rule> {
    const NAME: &str = "generated-fresh";
    let Some(rc) = cfg else {
        return Ok(Rule::skip(
            NAME,
            vec!["no cypera.toml — freshness not verifiable".into()],
        ));
    };
    let mut stale = Vec::new();
    let mut checked = 0usize;

    // crate-map outputs.
    if !rc.crate_map.splices.is_empty()
        || rc.crate_map.crate_api.is_some()
        || rc.crate_map.out.is_some()
    {
        match build_crate_map(root, Some(rc), &[]) {
            Ok((map, page)) => {
                let table = cypera_core::crate_map_table(&map);
                for splice in &rc.crate_map.splices {
                    let target = resolve_path(root, base, &splice.target);
                    checked += 1;
                    match std::fs::read_to_string(&target) {
                        Ok(existing) => {
                            let fresh = if splice.first_table {
                                cypera_core::splice_first_table(&existing, &table) == existing
                            } else {
                                let marker =
                                    splice.marker.as_deref().unwrap_or("<!-- crate-map -->");
                                match cypera_core::splice_marked(&existing, marker, &table) {
                                    Some(s) => s == existing,
                                    None => false,
                                }
                            };
                            if !fresh {
                                stale.push(format!("{}: crate-map table stale", target.display()));
                            }
                        }
                        Err(_) => stale.push(format!("{}: missing", target.display())),
                    }
                }
                if let Some(api) = &rc.crate_map.crate_api {
                    let target = resolve_path(root, base, &api.out);
                    checked += 1;
                    let want = cypera_core::crate_api_page(&api.site, &map);
                    match std::fs::read_to_string(&target) {
                        Ok(have) if have == want => {}
                        Ok(_) => stale.push(format!("{}: crate-api stale", target.display())),
                        Err(_) => stale.push(format!("{}: missing", target.display())),
                    }
                }
                if let Some(out_rel) = &rc.crate_map.out {
                    let target = resolve_path(root, base, out_rel);
                    checked += 1;
                    match std::fs::read_to_string(&target) {
                        Ok(have) if have == page => {}
                        Ok(_) => stale.push(format!("{}: crate-map page stale", target.display())),
                        Err(_) => stale.push(format!("{}: missing", target.display())),
                    }
                }
            }
            Err(e) => {
                return Ok(Rule::fail(
                    NAME,
                    vec![format!("crate-map regen failed: {e:#}")],
                ));
            }
        }
    }

    // cli page — real regen (cargo build + help walk).
    if let Some(cli) = &rc.cli {
        if let Some(out_rel) = &cli.out {
            checked += 1;
            match resolve_cli_spec(Some(cli), None, None, &[], &[]) {
                Ok(spec) => match cli_help_page(root, &spec) {
                    Ok(page) => {
                        let target = resolve_path(root, base, out_rel);
                        match std::fs::read_to_string(&target) {
                            Ok(have) if have == page => {}
                            Ok(_) => stale.push(format!("{}: cli page stale", target.display())),
                            Err(_) => stale.push(format!("{}: missing", target.display())),
                        }
                    }
                    Err(e) => {
                        return Ok(Rule::skip(NAME, vec![format!("cli regen skipped: {e:#}")]));
                    }
                },
                Err(e) => {
                    return Ok(Rule::fail(NAME, vec![format!("[cli] spec invalid: {e:#}")]));
                }
            }
        }
    }

    // config page.
    if let (Some(doc), Some(out_rel)) = (&rc.config, rc.config.as_ref().and_then(|c| c.out.clone()))
    {
        checked += 1;
        match config_doc_page(root, base, doc, &[], None) {
            Ok(page) => {
                let target = resolve_path(root, base, &out_rel);
                match std::fs::read_to_string(&target) {
                    Ok(have) if have == page => {}
                    Ok(_) => stale.push(format!("{}: config page stale", target.display())),
                    Err(_) => stale.push(format!("{}: missing", target.display())),
                }
            }
            Err(e) => {
                return Ok(Rule::fail(
                    NAME,
                    vec![format!("config regen failed: {e:#}")],
                ));
            }
        }
    }

    if checked == 0 {
        return Ok(Rule::skip(
            NAME,
            vec!["cypera.toml declares no generated outputs".into()],
        ));
    }
    if stale.is_empty() {
        Ok(Rule::pass(
            NAME,
            vec![format!("{checked} generated outputs fresh")],
        ))
    } else {
        stale.push("regenerate with `cypera gen …`".to_string());
        Ok(Rule::fail(NAME, stale))
    }
}

// ---------- rule: docs:check script ----------

/// `docs/package.json` exists and its `scripts` carry
/// `docs:check` (the regen-and-diff gate).
fn docs_check_script_rule(root: &Path) -> Rule {
    const NAME: &str = "docs-check-script";
    let path = root.join("docs/package.json");
    let Ok(text) = std::fs::read_to_string(&path) else {
        return Rule::fail(NAME, vec!["docs/package.json: missing".into()]);
    };
    let Ok(json) = serde_json::from_str::<Value>(&text) else {
        return Rule::fail(NAME, vec!["docs/package.json: invalid JSON".into()]);
    };
    let has = json
        .get("scripts")
        .and_then(|s| s.get("docs:check"))
        .is_some();
    if has {
        Rule::pass(NAME, vec!["docs/package.json: docs:check present".into()])
    } else {
        Rule::fail(
            NAME,
            vec!["docs/package.json: no `docs:check` script".into()],
        )
    }
}

#[cfg(test)]
mod tests {
    #![allow(clippy::unwrap_used)]
    use super::*;

    #[test]
    fn inline_code_spans_are_masked() {
        let line = "pin `v1.2.3` and ``a `1.4.5` b`` but release 2.0.0";
        let masked = mask_inline_code(line);
        assert!(find_versions(&masked).is_empty() || find_versions(&masked) == ["2.0.0"]);
        assert_eq!(find_versions(&masked), vec!["2.0.0".to_string()]);
    }

    #[test]
    fn unmatched_backtick_stays_literal() {
        let line = "a ` stray 1.2.3 mention";
        assert_eq!(find_versions(&mask_inline_code(line)), vec!["1.2.3"]);
    }

    #[test]
    fn script_block_is_masked() {
        let text = "# Title\n\n<script setup>\nconst d = \"M1.733.656 2.5 1.623.312\";\n</script>\n\nReleased in 3.1.4.\n";
        let masked = mask_html_blocks(text);
        let lines: Vec<&str> = masked.lines().collect();
        // The JS line (idx 3) is blanked; the prose line (idx 6)
        // still carries its version.
        assert!(!lines[3].contains("1.733"));
        assert!(lines[6].contains("3.1.4"));
    }

    #[test]
    fn style_block_and_unclosed_script() {
        let text =
            "<style>\n.a { width: 1.2.3px }\n</style>\nprose 4.5.6\n<script>\nnever closed 7.8.9";
        let masked = mask_html_blocks(text);
        let lines: Vec<&str> = masked.lines().collect();
        assert!(!lines[1].contains("1.2.3"));
        assert!(lines[3].contains("4.5.6"));
        assert!(!lines[5].contains("7.8.9"));
    }

    #[test]
    fn prose_version_still_flags() {
        let line = "Ships with 9.9.9 next week";
        assert_eq!(find_versions(&mask_inline_code(line)), vec!["9.9.9"]);
    }
}
