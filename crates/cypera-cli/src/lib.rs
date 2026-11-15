//! cypera — reusable documentation/reference generation for the 9aia
//! org. The shared successor to the per-repo `docs/gen-*.py`
//! scripts: one tool renders the crate map, CLI reference, and
//! config reference pages every sibling docs site needs.
//!
//! ```text
//! cypera gen crate-map [--repo <path>] [--out <file>] [--layer <crate=layer>]…
//! cypera gen cli       [--input <file|->] [--title <t>] [--out <file>]
//! cypera gen config    --file <rs>… [--only <Struct>]… [--title <t>] [--out <file>]
//! ```
//!
//! All generation logic lives in `cypera-core` (pure); this crate
//! owns argv, filesystem walks, and stdout.

use std::path::{Path, PathBuf};

use anyhow::{bail, Context, Result};
use clap::{Parser, Subcommand};
use cypera_core::{
    crate_map_from_workspace, crate_map_table, render_cli_page, render_config_page,
    render_crate_map, scan_config_structs, scan_entry_source, CrateManifest, EntryScan,
    WorkspaceManifest,
};

/// cypera — org docs/reference generator.
#[derive(Debug, Parser)]
#[command(name = "cypera", version, about)]
pub struct Cli {
    /// Generator subcommand.
    #[command(subcommand)]
    pub cmd: Cmd,
}

/// Top-level verbs.
#[derive(Debug, Subcommand)]
pub enum Cmd {
    /// Generate a docs/reference page.
    Gen {
        /// Which generator to run.
        #[command(subcommand)]
        gen: Gen,
    },
}

/// The `gen` generators.
#[derive(Debug, Subcommand)]
pub enum Gen {
    /// Crate map — workspace `Cargo.toml`s → crate/layer/purpose
    /// table (+ per-crate module sections when entry files exist).
    CrateMap {
        /// Repo root (dir containing the workspace `Cargo.toml`).
        /// Default: current directory. Upward discovery is
        /// intentionally not done — pass `--repo` explicitly.
        #[arg(long, default_value = ".")]
        repo: PathBuf,
        /// Write to this file instead of stdout. When the file
        /// exists and contains a `<!-- crate-map -->` marker, only
        /// the table after the marker is replaced (the
        /// `gen-crate-map.py` in-place convention).
        #[arg(long)]
        out: Option<PathBuf>,
        /// Layer override `crate-name=layer` (repeatable) —
        /// overrides the `-core`/`-cli`/bare-name heuristic.
        #[arg(long = "layer")]
        layers: Vec<String>,
    },
    /// CLI reference — wrap `gen-cli-docs` output (clap-markdown
    /// markdown) in a reference page.
    Cli {
        /// `gen-cli-docs` markdown file, or `-` for stdin.
        /// Default: stdin.
        #[arg(long, default_value = "-")]
        input: String,
        /// Page title. Default: `Commands`.
        #[arg(long, default_value = "Commands")]
        title: String,
        /// Write to this file instead of stdout.
        #[arg(long)]
        out: Option<PathBuf>,
    },
    /// Config reference — scan Rust sources for `pub struct` doc
    /// comments → field tables.
    Config {
        /// Rust source file to scan (repeatable).
        #[arg(long = "file", required = true)]
        files: Vec<PathBuf>,
        /// Only document these struct names (repeatable). Default:
        /// every `pub struct` found.
        #[arg(long = "only")]
        only: Vec<String>,
        /// Page title. Default: `Config reference`.
        #[arg(long, default_value = "Config reference")]
        title: String,
        /// Write to this file instead of stdout.
        #[arg(long)]
        out: Option<PathBuf>,
    },
}

/// `parse_from`/`run_from` split so drivers/tests can inject argv —
/// the org engine convention.
pub fn parse_from<I, T>(args: I) -> Cli
where
    I: IntoIterator<Item = T>,
    T: Into<std::ffi::OsString> + Clone,
{
    Cli::parse_from(args)
}

/// Run the parsed CLI; returns the process exit code.
pub fn run(cli: Cli) -> Result<i32> {
    match cli.cmd {
        Cmd::Gen { gen } => run_gen(gen),
    }
}

/// Convenience: parse `argv` (including argv[0]) and run.
pub fn run_from<I, T>(args: I) -> Result<i32>
where
    I: IntoIterator<Item = T>,
    T: Into<std::ffi::OsString> + Clone,
{
    run(parse_from(args))
}

fn run_gen(gen: Gen) -> Result<i32> {
    match gen {
        Gen::CrateMap { repo, out, layers } => gen_crate_map(&repo, out.as_deref(), &layers),
        Gen::Cli { input, title, out } => gen_cli(&input, &title, out.as_deref()),
        Gen::Config {
            files,
            only,
            title,
            out,
        } => gen_config(&files, &only, &title, out.as_deref()),
    }
}

// ---------- crate-map ----------

/// Parse `crate=layer` overrides into `(name, layer)` pairs.
fn parse_layers(raw: &[String]) -> Result<Vec<(String, String)>> {
    raw.iter()
        .map(|s| {
            s.split_once('=')
                .map(|(n, l)| (n.to_string(), l.to_string()))
                .with_context(|| format!("--layer expects crate=layer, got `{s}`"))
        })
        .collect()
}

/// Expand `[workspace] members` entries — plain paths pass
/// through; `dir/*` globs expand to every subdir containing a
/// `Cargo.toml`.
fn expand_members(root: &Path, members: &[String]) -> Result<Vec<PathBuf>> {
    let mut out = Vec::new();
    for m in members {
        if let Some((dir, _)) = m.rsplit_once("/*") {
            let base = root.join(dir);
            let mut subs: Vec<PathBuf> = std::fs::read_dir(&base)
                .with_context(|| format!("cannot read member glob dir {}", base.display()))?
                .filter_map(|e| e.ok().map(|e| e.path()))
                .filter(|p| p.is_dir() && p.join("Cargo.toml").is_file())
                .collect();
            subs.sort();
            if subs.is_empty() {
                bail!(
                    "member glob `{m}` matched no crates under {}",
                    base.display()
                );
            }
            out.extend(subs);
        } else {
            out.push(root.join(m));
        }
    }
    Ok(out)
}

/// Scan a crate dir's entry file (`src/lib.rs`, else `src/main.rs`)
/// for `//!` docs and `mod` declarations.
fn scan_entry(crate_dir: &Path) -> Option<EntryScan> {
    for (file, is_lib) in [("src/lib.rs", true), ("src/main.rs", false)] {
        let p = crate_dir.join(file);
        if let Ok(src) = std::fs::read_to_string(&p) {
            return Some(scan_entry_source(&src, is_lib));
        }
    }
    None
}

fn gen_crate_map(repo: &Path, out: Option<&Path>, layers: &[String]) -> Result<i32> {
    let root = repo
        .canonicalize()
        .with_context(|| format!("--repo {} does not exist", repo.display()))?;
    let ws_path = root.join("Cargo.toml");
    let ws_text = std::fs::read_to_string(&ws_path)
        .with_context(|| format!("cannot read {}", ws_path.display()))?;
    let ws = WorkspaceManifest::parse(&ws_path.display().to_string(), &ws_text)?;

    // Bare `[package]` root = single-crate repo.
    let member_dirs = if ws.members().is_empty() {
        vec![root.clone()]
    } else {
        expand_members(&root, ws.members())?
    };

    let repo_name = root
        .file_name()
        .map(|n| n.to_string_lossy().into_owned())
        .unwrap_or_else(|| "repo".to_string());

    let mut members = Vec::new();
    let mut entries = Vec::new();
    for dir in &member_dirs {
        let manifest_path = dir.join("Cargo.toml");
        let text = std::fs::read_to_string(&manifest_path)
            .with_context(|| format!("cannot read {}", manifest_path.display()))?;
        let rel = dir
            .strip_prefix(&root)
            .unwrap_or(dir)
            .to_string_lossy()
            .into_owned();
        let rel = if rel.is_empty() { ".".to_string() } else { rel };
        let mut manifest = CrateManifest::parse(&manifest_path.display().to_string(), &text)?;
        // `description.workspace = true` inherits
        // `[workspace.package] description` — resolve it here so
        // the crate map shows the real text.
        if manifest.description().is_empty() {
            if let Some(d) = ws.workspace_description() {
                manifest.package.description = Some(toml::Value::String(d.to_string()));
            }
        }
        members.push((rel.clone(), manifest));
        if let Some(scan) = scan_entry(dir) {
            entries.push((rel, scan));
        }
    }

    let overrides = parse_layers(layers)?;
    let map = crate_map_from_workspace(&repo_name, &ws, &members, &entries, &overrides)?;
    let page = render_crate_map(&map);

    match out {
        Some(path) => {
            // `<!-- crate-map -->` marker → splice only the table
            // (the gen-crate-map.py in-place convention); otherwise
            // the existing generated page is replaced wholesale.
            if let Ok(existing) = std::fs::read_to_string(path) {
                if let Some(i) = existing
                    .lines()
                    .position(|l| l.contains("<!-- crate-map -->"))
                {
                    // Replace the run of `|…|` table rows after the
                    // marker with the fresh table.
                    let table = crate_map_table(&map);
                    let mut lines: Vec<String> = existing.lines().map(str::to_string).collect();
                    let mut k = i + 1;
                    while k < lines.len() && lines[k].starts_with('|') {
                        k += 1;
                    }
                    lines.splice(i + 1..k, table.lines().map(str::to_string));
                    std::fs::write(path, lines.join("\n") + "\n")?;
                    println!("crate map spliced into {}", path.display());
                    return Ok(0);
                }
            }
            std::fs::write(path, &page)
                .with_context(|| format!("cannot write {}", path.display()))?;
            println!("crate map written to {}", path.display());
        }
        None => print!("{page}"),
    }
    Ok(0)
}

// ---------- cli ----------

fn gen_cli(input: &str, title: &str, out: Option<&Path>) -> Result<i32> {
    let body = if input == "-" {
        std::io::read_to_string(std::io::stdin())?
    } else {
        std::fs::read_to_string(input).with_context(|| format!("cannot read {input}"))?
    };
    let page = render_cli_page(title, &body);
    emit(page, out)?;
    Ok(0)
}

// ---------- config ----------

fn gen_config(files: &[PathBuf], only: &[String], title: &str, out: Option<&Path>) -> Result<i32> {
    let mut structs = Vec::new();
    for f in files {
        let src =
            std::fs::read_to_string(f).with_context(|| format!("cannot read {}", f.display()))?;
        structs.extend(scan_config_structs(&src));
    }
    if !only.is_empty() {
        structs.retain(|s| only.iter().any(|n| n == &s.name));
        let found: Vec<&str> = structs.iter().map(|s| s.name.as_str()).collect();
        for n in only {
            if !found.contains(&n.as_str()) {
                bail!("--only {n}: no `pub struct {n}` found in the given files");
            }
        }
    }
    if structs.is_empty() {
        bail!("no `pub struct` definitions found — pass --file <config.rs>");
    }
    let page = render_config_page(title, &structs);
    emit(page, out)?;
    Ok(0)
}

/// stdout or file.
fn emit(page: String, out: Option<&Path>) -> Result<()> {
    match out {
        Some(path) => {
            std::fs::write(path, page)
                .with_context(|| format!("cannot write {}", path.display()))?;
            println!("written to {}", path.display());
        }
        None => print!("{page}"),
    }
    Ok(())
}
