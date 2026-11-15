//! cypera — reusable documentation/reference generation for the 9aia
//! org. The shared successor to the per-repo `docs/gen-*.py`
//! scripts: one tool renders the crate map, CLI reference, and
//! config reference pages every sibling docs site needs, driven by
//! a per-repo `cypera.toml` discovered upward from `--repo`.
//!
//! ```text
//! cypera gen crate-map [--repo <path>] [--config <path>] [--out <file>] [--layer <crate=layer>]…
//! cypera gen cli       [--repo <path>] [--config <path>] [--input <file|->] [--out <file>]
//! cypera gen config    [--repo <path>] [--config <path>] [--file <rs>]… [--out <file>]
//! cypera check         [--repo <path>] [--config <path>] [--json]
//! ```
//!
//! All generation logic lives in `cypera-core` (pure); this crate
//! owns argv, `cypera.toml` discovery, filesystem walks, process
//! spawning, and stdout.

use std::collections::HashSet;
use std::path::{Path, PathBuf};

use anyhow::{bail, Context, Result};
use clap::{Parser, Subcommand};
use cypera_core::{
    crate_api_page, crate_map_from_workspace, crate_map_table, parse_help, prog_name,
    purpose_from_entry_doc, render_cli_page, render_config_doc_page, render_config_page,
    render_crate_map, render_help_walk, scan_config_structs, scan_entry_source, splice_first_table,
    splice_marked, CliConfig, ConfigDocConfig, ConfigDocSection, ConfigStruct, CrateManifest,
    EntryScan, HelpNode, RepoConfig, WorkspaceManifest,
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
    /// Lint a repo's docs contract — README/AGENTS shape, no
    /// CHANGELOG, no version-in-prose, Diátaxis docs site,
    /// generated-reference freshness, `docs:check` script.
    Check {
        /// Repo root. Default: current directory.
        #[arg(long, default_value = ".")]
        repo: PathBuf,
        /// Explicit `cypera.toml` path (default: discovered upward
        /// from `--repo`).
        #[arg(long)]
        config: Option<PathBuf>,
        /// Emit the report as JSON.
        #[arg(long)]
        json: bool,
    },
}

/// The `gen` generators.
#[derive(Debug, Subcommand)]
pub enum Gen {
    /// Crate map — workspace `Cargo.toml`s → crate/layer/purpose
    /// table (+ per-crate module sections when entry files exist).
    /// `cypera.toml [crate_map]` supplies layer overrides, the
    /// layer order, the default layer, splice targets, and the
    /// crate-api page.
    CrateMap {
        /// Repo root (dir containing the workspace `Cargo.toml`).
        /// Default: current directory.
        #[arg(long, default_value = ".")]
        repo: PathBuf,
        /// Explicit `cypera.toml` path (default: discovered upward
        /// from `--repo`).
        #[arg(long)]
        config: Option<PathBuf>,
        /// Write a full page to this file instead of stdout. When
        /// the file exists and contains a `<!-- crate-map -->`
        /// marker, only the table after the marker is replaced.
        /// Overrides `[crate_map]` splices/crate_api.
        #[arg(long)]
        out: Option<PathBuf>,
        /// Layer override `crate-name=layer` (repeatable) —
        /// overrides `[crate_map.layers]` and the heuristic.
        #[arg(long = "layer")]
        layers: Vec<String>,
    },
    /// CLI reference — two modes:
    ///
    /// - `--input <file|->`: wrap `gen-cli-docs` markdown
    ///   (clap-markdown) in a reference page.
    /// - otherwise: the `docs/gen-cli-docs.py` port — walk
    ///   `<bin> <prefix>… <sub>… --help` recursively and emit the
    ///   same page. The command spec comes from `--package`/`--bin`/
    ///   `--prefix` flags or `[cli]` in `cypera.toml`.
    Cli {
        /// Repo root — cargo build + `cypera.toml` discovery base.
        #[arg(long, default_value = ".")]
        repo: PathBuf,
        /// Explicit `cypera.toml` path.
        #[arg(long)]
        config: Option<PathBuf>,
        /// `gen-cli-docs` markdown file, or `-` for stdin. When
        /// given, wrap mode (no help walk).
        #[arg(long)]
        input: Option<String>,
        /// Cargo package to `cargo build -q -p` first (config
        /// `[cli].package` fallback).
        #[arg(long)]
        package: Option<String>,
        /// Binary under `target/debug/` — or a PATH executable
        /// when `--package` is unset (`[cli].bin`).
        #[arg(long)]
        bin: Option<String>,
        /// Argv prefix before the walked subcommand path, e.g.
        /// `run` for the driver `run` passthrough (repeatable;
        /// `[cli].prefix` fallback).
        #[arg(long = "prefix")]
        prefixes: Vec<String>,
        /// Page title override (`[cli].title` fallback; default is
        /// the python scripts' `Command-Line Help for …` header).
        #[arg(long)]
        title: Option<String>,
        /// Subcommand path to skip in the `--help` walk, space-
        /// joined (`driver exec`; repeatable; `[cli].skip` fallback).
        #[arg(long = "skip")]
        skips: Vec<String>,
        /// Write to this file instead of stdout
        /// (`[cli].out` fallback, then stdout).
        #[arg(long)]
        out: Option<PathBuf>,
    },
    /// Config reference — two modes:
    ///
    /// - `--file <rs>…`: scan `pub struct` doc comments and render
    ///   the generic page (`--only`/`--title` apply).
    /// - otherwise: the `docs/gen-config-docs.py` port — the
    ///   curated `[config]` spec in `cypera.toml` (struct set,
    ///   section titles, type vocabulary, prose sections).
    Config {
        /// Repo root — `cypera.toml` discovery + relative `file`
        /// resolution base.
        #[arg(long, default_value = ".")]
        repo: PathBuf,
        /// Explicit `cypera.toml` path.
        #[arg(long)]
        config: Option<PathBuf>,
        /// Rust source file to scan (repeatable) — legacy mode.
        #[arg(long = "file")]
        files: Vec<PathBuf>,
        /// Only document these struct names (repeatable). Legacy
        /// mode; in `[config]` mode filters struct sections.
        #[arg(long = "only")]
        only: Vec<String>,
        /// Page title. Legacy mode default: `Config reference`;
        /// `[config]` mode: overrides `[config].title`.
        #[arg(long)]
        title: Option<String>,
        /// Write to this file instead of stdout
        /// (`[config].out` fallback, then stdout).
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

/// The clap command tree — the engine-ops driver's manifest derives
/// `cli.<verb>` wire queries from this (the org `command()`
/// convention).
pub fn command() -> clap::Command {
    <Cli as clap::CommandFactory>::command()
}

/// Run the parsed CLI; returns the process exit code.
pub fn run(cli: Cli) -> Result<i32> {
    match cli.cmd {
        Cmd::Gen { gen } => run_gen(gen),
        Cmd::Check { repo, config, json } => check::run_check(&repo, config.as_deref(), json),
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
        Gen::CrateMap {
            repo,
            config,
            out,
            layers,
        } => gen_crate_map(&repo, config.as_deref(), out.as_deref(), &layers),
        Gen::Cli {
            repo,
            config,
            input,
            package,
            bin,
            prefixes,
            title,
            skips,
            out,
        } => gen_cli(
            &repo,
            config.as_deref(),
            CliOpts {
                input: input.as_deref(),
                package: package.as_deref(),
                bin: bin.as_deref(),
                prefixes: &prefixes,
                title: title.as_deref(),
                skips: &skips,
                out: out.as_deref(),
            },
        ),
        Gen::Config {
            repo,
            config,
            files,
            only,
            title,
            out,
        } => gen_config(
            &repo,
            config.as_deref(),
            &files,
            &only,
            title.as_deref(),
            out.as_deref(),
        ),
    }
}

// ---------- cypera.toml discovery ----------

/// Canonicalize `repo` and locate `cypera.toml`: the explicit
/// `--config` path when given, else upward discovery from the repo
/// root (the file may sit at the repo root or an ancestor — e.g. a
/// workspace-level config). Returns `(config_path, parsed)`.
fn discover_config(repo: &Path, explicit: Option<&Path>) -> Result<Option<(PathBuf, RepoConfig)>> {
    let path = match explicit {
        Some(p) => p.to_path_buf(),
        None => {
            let root = repo
                .canonicalize()
                .with_context(|| format!("--repo {} does not exist", repo.display()))?;
            let mut dir = Some(root.as_path());
            let mut found = None;
            while let Some(d) = dir {
                let cand = d.join("cypera.toml");
                if cand.is_file() {
                    found = Some(cand);
                    break;
                }
                dir = d.parent();
            }
            match found {
                Some(p) => p,
                None => return Ok(None),
            }
        }
    };
    let text = std::fs::read_to_string(&path)
        .with_context(|| format!("cannot read {}", path.display()))?;
    let cfg = RepoConfig::parse(&path.display().to_string(), &text)?;
    Ok(Some((path, cfg)))
}

/// Directory relative paths in `cypera.toml` resolve against —
/// the config file's parent (repo root in the common case).
fn config_dir(config_path: &Option<(PathBuf, RepoConfig)>) -> PathBuf {
    config_path
        .as_ref()
        .and_then(|(p, _)| p.parent().map(Path::to_path_buf))
        .unwrap_or_else(|| PathBuf::from("."))
}

/// Resolve a config-relative path: try repo-root-relative first
/// (the documented convention), then config-dir-relative.
fn resolve_path(repo: &Path, base: &Path, rel: &str) -> PathBuf {
    let p = Path::new(rel);
    if p.is_absolute() {
        return p.to_path_buf();
    }
    let under_repo = repo.join(p);
    if under_repo.exists() {
        return under_repo;
    }
    base.join(p)
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

/// Build the [`cypera_core::CrateMap`] + rendered page for a repo —
/// shared by `gen crate-map` and `check` freshness. `extra_layers`
/// are `--layer` CLI overrides (they win over `[crate_map.layers]`).
fn build_crate_map(
    repo: &Path,
    cfg: Option<&RepoConfig>,
    extra_layers: &[String],
) -> Result<(cypera_core::CrateMap, String)> {
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
        let entry = scan_entry(dir);
        // Purpose resolution — the `gen-crate-map.py` `description()`
        // order: literal `[package] description` → first `//!`
        // paragraph of the entry file (tidied into one cell) →
        // `[workspace.package] description` when the crate opts in
        // (`description.workspace = true`) → `"—"`.
        let mut purpose = manifest.description().to_string();
        if purpose.is_empty() {
            if let Some(e) = &entry {
                purpose = purpose_from_entry_doc(&e.doc);
            }
            if purpose.is_empty() {
                purpose = if manifest.description_inherited() {
                    ws.workspace_description().unwrap_or("").to_string()
                } else {
                    "—".to_string()
                };
            }
        }
        manifest.package.description = Some(toml::Value::String(purpose));
        members.push((rel.clone(), manifest));
        if let Some(scan) = entry {
            entries.push((rel, scan));
        }
    }

    // `--layer` flags first — first-match wins, so CLI beats config.
    let cli_overrides = parse_layers(extra_layers)?;
    let mut overrides = cli_overrides;
    if let Some(c) = cfg {
        overrides.extend(c.layer_overrides());
    }
    let order = cfg.map_or(&[][..], |c| c.crate_map.order.as_slice());
    let default_layer = cfg.and_then(|c| c.crate_map.default_layer.as_deref());
    let map = crate_map_from_workspace(
        &repo_name,
        &ws,
        &members,
        &entries,
        &overrides,
        order,
        default_layer,
    )?;
    let page = render_crate_map(&map);
    Ok((map, page))
}

/// Apply `[crate_map]` splices + crate_api to the repo — the
/// `gen-crate-map.py` wrapper behavior (`--out`-less mode).
fn apply_crate_map_outputs(
    root: &Path,
    base: &Path,
    cfg: &RepoConfig,
    map: &cypera_core::CrateMap,
) -> Result<usize> {
    let table = crate_map_table(map);
    let mut written = 0;
    for splice in &cfg.crate_map.splices {
        let target = resolve_path(root, base, &splice.target);
        let existing = std::fs::read_to_string(&target)
            .with_context(|| format!("cannot read splice target {}", target.display()))?;
        let spliced = if splice.first_table {
            splice_first_table(&existing, &table)
        } else {
            let marker = splice.marker.as_deref().unwrap_or("<!-- crate-map -->");
            splice_marked(&existing, marker, &table).with_context(|| {
                format!("splice marker `{marker}` not found in {}", target.display())
            })?
        };
        std::fs::write(&target, spliced)
            .with_context(|| format!("cannot write {}", target.display()))?;
        println!("crate map spliced into {}", target.display());
        written += 1;
    }
    if let Some(api) = &cfg.crate_map.crate_api {
        let target = resolve_path(root, base, &api.out);
        std::fs::write(&target, crate_api_page(&api.site, map))
            .with_context(|| format!("cannot write {}", target.display()))?;
        println!("crate api written to {}", target.display());
        written += 1;
    }
    Ok(written)
}

fn gen_crate_map(
    repo: &Path,
    config: Option<&Path>,
    out: Option<&Path>,
    layers: &[String],
) -> Result<i32> {
    let discovered = discover_config(repo, config)?;
    let cfg = discovered.as_ref().map(|(_, c)| c);
    let (map, page) = build_crate_map(repo, cfg, layers)?;

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
        None => {
            if let Some((_, rc)) = &discovered {
                let root = repo.canonicalize()?;
                let base = config_dir(&discovered);
                let n = apply_crate_map_outputs(&root, &base, rc, &map)?;
                // The `out` page is a declared output like the
                // splices — `check` freshness verifies it, so it
                // must be written even when splices ran.
                if let Some(out_rel) = &rc.crate_map.out {
                    let target = resolve_path(&root, &base, out_rel);
                    std::fs::write(&target, &page)
                        .with_context(|| format!("cannot write {}", target.display()))?;
                    println!("crate map written to {}", target.display());
                } else if n == 0 {
                    print!("{page}");
                }
            } else {
                print!("{page}");
            }
        }
    }
    Ok(0)
}

// ---------- cli ----------

/// CLI-verb flags shared between clap argv and the `[cli]` config
/// fallback — everything `gen cli` can take from either source.
struct CliOpts<'a> {
    input: Option<&'a str>,
    package: Option<&'a str>,
    bin: Option<&'a str>,
    prefixes: &'a [String],
    title: Option<&'a str>,
    skips: &'a [String],
    out: Option<&'a Path>,
}

fn gen_cli(repo: &Path, config: Option<&Path>, opts: CliOpts<'_>) -> Result<i32> {
    let CliOpts {
        input,
        package,
        bin,
        prefixes,
        title,
        skips,
        out,
    } = opts;
    // Wrap mode — `gen-cli-docs` markdown in a page.
    if let Some(input) = input {
        let body = if input == "-" {
            std::io::read_to_string(std::io::stdin())?
        } else {
            std::fs::read_to_string(input).with_context(|| format!("cannot read {input}"))?
        };
        let page = render_cli_page(title.unwrap_or("Commands"), &body);
        return emit(page, out).map(|_| 0);
    }

    let discovered = discover_config(repo, config)?;
    let file_cfg = discovered.as_ref().and_then(|(_, c)| c.cli.clone());
    // Flag > `[cli]` config > error.
    let spec = resolve_cli_spec(file_cfg.as_ref(), package, bin, prefixes, skips)?;
    let title = title
        .map(str::to_string)
        .or_else(|| file_cfg.as_ref().and_then(|c| c.title.clone()));
    let emit_to = out.map(Path::to_path_buf).or_else(|| {
        file_cfg
            .as_ref()
            .and_then(|c| c.out.clone().map(PathBuf::from))
    });

    let root = repo
        .canonicalize()
        .with_context(|| format!("--repo {} does not exist", repo.display()))?;
    let mut spec = spec;
    spec.title = title;
    let page = cli_help_page(&root, &spec)?;
    emit_to_path(page, emit_to.as_deref())?;
    Ok(0)
}

/// Merge `[cli]` config with flag overrides into a [`CliConfig`].
fn resolve_cli_spec(
    file_cfg: Option<&CliConfig>,
    package: Option<&str>,
    bin: Option<&str>,
    prefixes: &[String],
    skips: &[String],
) -> Result<CliConfig> {
    let mut spec = file_cfg.cloned().unwrap_or(CliConfig {
        package: None,
        bin: String::new(),
        prefix: Vec::new(),
        title: None,
        out: None,
        skip: Vec::new(),
    });
    if let Some(p) = package {
        spec.package = Some(p.to_string());
    }
    if let Some(b) = bin {
        spec.bin = b.to_string();
    }
    if !prefixes.is_empty() {
        spec.prefix = prefixes.to_vec();
    }
    if !skips.is_empty() {
        spec.skip = skips.to_vec();
    }
    if spec.bin.is_empty() {
        bail!("gen cli: no `bin` — pass --bin or set `[cli].bin` in cypera.toml");
    }
    Ok(spec)
}

/// `cargo build -q -p <pkg>` once, then walk `<bin> <prefix>…
/// <path>… --help` recursively — the `gen-cli-docs.py` IO half.
fn cli_help_page(root: &Path, spec: &CliConfig) -> Result<String> {
    let bin_path = if let Some(pkg) = &spec.package {
        let status = std::process::Command::new("cargo")
            .args(["build", "-q", "-p", pkg])
            .current_dir(root)
            .status()
            .with_context(|| format!("cargo build -p {pkg} failed to spawn"))?;
        if !status.success() {
            bail!("cargo build -p {pkg} failed");
        }
        root.join("target/debug").join(&spec.bin)
    } else {
        PathBuf::from(&spec.bin)
    };

    fn help_text(root: &Path, bin: &Path, spec: &CliConfig, path: &[String]) -> Result<String> {
        let mut cmd = std::process::Command::new(bin);
        cmd.args(&spec.prefix)
            .args(path)
            .arg("--help")
            .current_dir(root);
        let out = cmd
            .output()
            .with_context(|| format!("cannot spawn {}", bin.display()))?;
        if !out.status.success() {
            let label = if path.is_empty() {
                "<root>".to_string()
            } else {
                path.join(" ")
            };
            bail!(
                "help failed for {label}:\n{}",
                String::from_utf8_lossy(&out.stderr)
            );
        }
        Ok(String::from_utf8_lossy(&out.stdout).into_owned())
    }

    let mut nodes: Vec<HelpNode> = Vec::new();
    let mut seen: HashSet<Vec<String>> = HashSet::new();

    fn walk(
        root: &Path,
        bin: &Path,
        spec: &CliConfig,
        path: Vec<String>,
        nodes: &mut Vec<HelpNode>,
        seen: &mut HashSet<Vec<String>>,
    ) -> Result<()> {
        if !seen.insert(path.clone()) {
            return Ok(());
        }
        // `[cli].skip` — passthrough surfaces (`disable_help_flag`)
        // have no `--help` to walk; the parent still lists them.
        if spec
            .skip
            .iter()
            .any(|s| s.split_whitespace().eq(path.iter().map(String::as_str)))
        {
            return Ok(());
        }
        let text = help_text(root, bin, spec, &path)?;
        let (usage, about, sections) = parse_help(&text);
        // Subcommand names for the recursion — python reads them
        // from the parsed Commands section before pushing.
        let subs: Vec<String> = sections
            .iter()
            .find(|(n, _)| n == "Commands")
            .map(|(_, entries)| {
                entries
                    .iter()
                    .filter_map(|(sig, _)| sig.split_whitespace().next().map(str::to_string))
                    .filter(|s| s != "help")
                    .collect()
            })
            .unwrap_or_default();
        nodes.push(HelpNode {
            path: path.clone(),
            usage,
            about,
            sections,
        });
        for sub in subs {
            let mut next = path.clone();
            next.push(sub);
            walk(root, bin, spec, next, nodes, seen)?;
        }
        Ok(())
    }

    walk(root, &bin_path, spec, Vec::new(), &mut nodes, &mut seen)?;
    let name = nodes
        .first()
        .map(|n| prog_name(&n.usage))
        .unwrap_or_else(|| "cli".to_string());
    Ok(render_help_walk(&name, &nodes, spec.title.as_deref()))
}

// ---------- config ----------

fn gen_config(
    repo: &Path,
    config: Option<&Path>,
    files: &[PathBuf],
    only: &[String],
    title: Option<&str>,
    out: Option<&Path>,
) -> Result<i32> {
    let discovered = discover_config(repo, config)?;

    // Legacy mode — explicit --file scans, generic page.
    if !files.is_empty() {
        let mut structs = Vec::new();
        for f in files {
            let src = std::fs::read_to_string(f)
                .with_context(|| format!("cannot read {}", f.display()))?;
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
        let page = render_config_page(title.unwrap_or("Config reference"), &structs);
        return emit(page, out).map(|_| 0);
    }

    // Curated mode — `[config]` in cypera.toml.
    let Some((cfg_path, rc)) = &discovered else {
        bail!(
            "gen config: no --file given and no `[config]` section — \
             pass --file <rs>… or add [config] to cypera.toml"
        );
    };
    let Some(doc) = &rc.config else {
        bail!(
            "gen config: cypera.toml at {} has no [config] section",
            cfg_path.display()
        );
    };
    let root = repo
        .canonicalize()
        .with_context(|| format!("--repo {} does not exist", repo.display()))?;
    let base = config_dir(&discovered);
    let page = config_doc_page(&root, &base, doc, only, title)?;
    let emit_to = out
        .map(Path::to_path_buf)
        .or_else(|| doc.out.as_ref().map(PathBuf::from));
    emit_to_path(page, emit_to.as_deref())?;
    Ok(0)
}

/// Render the `[config]`-driven page — shared by `gen config` and
/// `check` freshness.
fn config_doc_page(
    root: &Path,
    base: &Path,
    doc: &ConfigDocConfig,
    only: &[String],
    title_override: Option<&str>,
) -> Result<String> {
    // Scan every declared source file once; index structs by name.
    let mut scanned: std::collections::HashMap<String, ConfigStruct> =
        std::collections::HashMap::new();
    for sref in &doc.structs {
        let path = resolve_path(root, base, &sref.file);
        let src = std::fs::read_to_string(&path)
            .with_context(|| format!("cannot read {}", path.display()))?;
        let mut found = false;
        for s in scan_config_structs(&src) {
            if s.name == sref.name {
                let mut s = s;
                if doc.all_optional.iter().any(|n| n == &sref.name) {
                    for f in &mut s.fields {
                        f.optional = true;
                    }
                }
                scanned.insert(sref.name.clone(), s);
                found = true;
            }
        }
        if !found {
            // Python parity: `fields()` returned [] for a missing
            // struct — the section renders a header-only table.
            // (tamias `ConfigFormat` is a trait, not a struct.)
            eprintln!(
                "cypera: warning — [config.structs] `{}`: no `pub struct` in {}",
                sref.name,
                path.display()
            );
            scanned.insert(
                sref.name.clone(),
                ConfigStruct {
                    name: sref.name.clone(),
                    doc: String::new(),
                    fields: Vec::new(),
                },
            );
        }
    }

    let sections: Vec<ConfigDocSection> = if doc.sections.is_empty() {
        doc.structs
            .iter()
            .filter(|s| only.is_empty() || only.iter().any(|n| n == &s.name))
            .map(|sref| ConfigDocSection {
                level: 2,
                title: Some(sref.title.clone()),
                strukt: scanned.get(&sref.name).cloned(),
                body: None,
            })
            .collect()
    } else {
        let mut sections = Vec::with_capacity(doc.sections.len());
        for sec in &doc.sections {
            // A section may combine a struct table with trailing
            // prose (`struct` + `text`/`text_file`) — the tarsi
            // sync-rule/ghreleases convention.
            let body = match (&sec.text, &sec.text_file) {
                (Some(t), _) => Some(t.clone()),
                (None, Some(f)) => {
                    let p = resolve_path(root, base, f);
                    Some(
                        std::fs::read_to_string(&p)
                            .with_context(|| format!("cannot read text_file {}", p.display()))?,
                    )
                }
                (None, None) => None,
            };
            if let Some(name) = &sec.struct_name {
                if !only.is_empty() && !only.iter().any(|n| n == name) {
                    continue;
                }
                sections.push(ConfigDocSection {
                    level: sec.level,
                    title: sec.title.clone(),
                    strukt: scanned.get(name).cloned(),
                    body,
                });
            } else {
                sections.push(ConfigDocSection {
                    level: sec.level,
                    title: sec.title.clone(),
                    strukt: None,
                    body,
                });
            }
        }
        sections
    };

    let preamble = match (&doc.preamble, &doc.preamble_file) {
        (Some(t), _) => Some(t.clone()),
        (None, Some(f)) => {
            let p = resolve_path(root, base, f);
            Some(
                std::fs::read_to_string(&p)
                    .with_context(|| format!("cannot read preamble_file {}", p.display()))?,
            )
        }
        (None, None) => None,
    };
    let style = cypera_core::ConfigDocStyle {
        preamble,
        marker: doc.marker.clone(),
        table_head: doc.table_head.clone(),
        table_sep: doc.table_sep.clone(),
    };
    Ok(render_config_doc_page(
        title_override.unwrap_or(&doc.title),
        &sections,
        &doc.types,
        &doc.field_types,
        &style,
    ))
}

// ---------- emit ----------

/// stdout or `--out` file (legacy path — explicit arg only).
fn emit(page: String, out: Option<&Path>) -> Result<()> {
    emit_to_path(page, out)
}

fn emit_to_path(page: String, out: Option<&Path>) -> Result<()> {
    match out {
        Some(path) => {
            if let Some(parent) = path.parent() {
                if !parent.as_os_str().is_empty() {
                    std::fs::create_dir_all(parent)
                        .with_context(|| format!("cannot create {}", parent.display()))?;
                }
            }
            std::fs::write(path, page)
                .with_context(|| format!("cannot write {}", path.display()))?;
            println!("written to {}", path.display());
        }
        None => print!("{page}"),
    }
    Ok(())
}

// ---------- check ----------

mod check;
