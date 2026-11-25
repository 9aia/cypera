# Command-Line Help for `cypera`

This document contains the help content for the `cypera` command-line program.

Reusable documentation/reference-generation utilities for the 9aia org

**Command Overview:**

* [`cypera`↴](#cypera)
* [`cypera gen`↴](#cypera-gen)
* [`cypera gen crate-map`↴](#cypera-gen-crate-map)
* [`cypera gen cli`↴](#cypera-gen-cli)
* [`cypera gen config`↴](#cypera-gen-config)
* [`cypera gen api`↴](#cypera-gen-api)
* [`cypera check`↴](#cypera-check)

## `cypera`

Reusable documentation/reference-generation utilities for the 9aia org

**Usage:** `cypera <COMMAND>`

###### **Commands:**

* `gen` — Generate a docs/reference page
* `check` — Lint a repo's docs contract — README/AGENTS shape, no CHANGELOG, no version-in-prose, Diátaxis docs site, generated-reference freshness, `docs:check` script

###### **Options:**

* `-h, --help` — Print help
* `-V, --version` — Print version

## `cypera gen`

Generate a docs/reference page

**Usage:** `cypera gen <COMMAND>`

###### **Commands:**

* `crate-map` — Crate map — workspace `Cargo.toml`s → crate/layer/purpose table (+ per-crate module sections when entry files exist). `cypera.toml [crate_map]` supplies layer overrides, the layer order, the default layer, splice targets, and the crate-api page
* `cli` — CLI reference — two modes:
* `config` — Config reference — two modes:
* `api` — Rustdoc API site — the `docs/gen-api*.py` port: `cargo doc --workspace --no-deps` in the repo, the real target dir resolved via `cargo metadata` (respects a configured `target-dir`), `<target>/doc` copied into the docs site's `public/api/`, and a styled crate index written to `index.html`. The output is a gitignored build artifact — `check` has nothing to freshness-gate. `[api]` in cypera.toml supplies `out`/`prefix`/`title`/ `target_dir`; every field is optional

###### **Options:**

* `-h, --help` — Print help

## `cypera gen crate-map`

Crate map — workspace `Cargo.toml`s → crate/layer/purpose table (+ per-crate module sections when entry files exist). `cypera.toml [crate_map]` supplies layer overrides, the layer order, the default layer, splice targets, and the crate-api page

**Usage:** `cypera gen crate-map [OPTIONS]`

###### **Options:**

* `--repo <REPO>` — Repo root (dir containing the workspace `Cargo.toml`). Default: current directory [default: .]
* `--config <CONFIG>` — Explicit `cypera.toml` path (default: discovered upward from `--repo`)
* `--out <OUT>` — Write a full page to this file instead of stdout. When the file exists and contains a `<!-- crate-map -->` marker, only the table after the marker is replaced. Overrides `[crate_map]` splices/crate_api
* `--layer <LAYERS>` — Layer override `crate-name=layer` (repeatable) — overrides `[crate_map.layers]` and the heuristic
* `-h, --help` — Print help

## `cypera gen cli`

CLI reference — two modes: - `--input <file|->`: wrap `gen-cli-docs` markdown (clap-markdown) in a reference page. - otherwise: the `docs/gen-cli-docs.py` port — walk `<bin> <prefix>… <sub>… --help` recursively and emit the same page. The command spec comes from `--package`/`--bin`/ `--prefix` flags or `[cli]` in `cypera.toml`.

**Usage:** `cypera gen cli [OPTIONS]`

###### **Options:**

* `--repo <REPO>` — Repo root — cargo build + `cypera.toml` discovery base
* `--config <CONFIG>` — Explicit `cypera.toml` path
* `--input <INPUT>` — `gen-cli-docs` markdown file, or `-` for stdin. When given, wrap mode (no help walk)
* `--package <PACKAGE>` — Cargo package to `cargo build -q -p` first (config `[cli].package` fallback)
* `--bin <BIN>` — Binary under `target/debug/` — or a PATH executable when `--package` is unset (`[cli].bin`)
* `--prefix <PREFIXES>` — Argv prefix before the walked subcommand path, e.g. `run` for the driver `run` passthrough (repeatable; `[cli].prefix` fallback)
* `--title <TITLE>` — Page title override (`[cli].title` fallback; default is the python scripts' `Command-Line Help for …` header)
* `--skip <SKIPS>` — Subcommand path to skip in the `--help` walk, space- joined (`driver exec`; repeatable; `[cli].skip` fallback)
* `--out <OUT>` — Write to this file instead of stdout (`[cli].out` fallback, then stdout)
* `-h, --help` — Print help (see a summary with '-h')

## `cypera gen config`

Config reference — two modes: - `--file <rs>…`: scan `pub struct` doc comments and render the generic page (`--only`/`--title` apply). - otherwise: the `docs/gen-config-docs.py` port — the curated `[config]` spec in `cypera.toml` (struct set, section titles, type vocabulary, prose sections).

**Usage:** `cypera gen config [OPTIONS]`

###### **Options:**

* `--repo <REPO>` — Repo root — `cypera.toml` discovery + relative `file` resolution base
* `--config <CONFIG>` — Explicit `cypera.toml` path
* `--file <FILES>` — Rust source file to scan (repeatable) — legacy mode
* `--only <ONLY>` — Only document these struct names (repeatable). Legacy mode; in `[config]` mode filters struct sections
* `--title <TITLE>` — Page title. Legacy mode default: `Config reference`; `[config]` mode: overrides `[config].title`
* `--out <OUT>` — Write to this file instead of stdout (`[config].out` fallback, then stdout)
* `-h, --help` — Print help (see a summary with '-h')

## `cypera gen api`

Rustdoc API site — the `docs/gen-api*.py` port: `cargo doc --workspace --no-deps` in the repo, the real target dir resolved via `cargo metadata` (respects a configured `target-dir`), `<target>/doc` copied into the docs site's `public/api/`, and a styled crate index written to `index.html`. The output is a gitignored build artifact — `check` has nothing to freshness-gate. `[api]` in cypera.toml supplies `out`/`prefix`/`title`/ `target_dir`; every field is optional

**Usage:** `cypera gen api [OPTIONS]`

###### **Options:**

* `--repo <REPO>` — Repo root — the `cargo doc`/`cargo metadata` cwd. A `<NAME>_ROOT` env var (NAME = the cypera.toml dir's basename, uppercased — the python `TAMIAS_ROOT`/ `MYCO_ROOT`/… convention) overrides it, which is how detached-sha doc pipelines point at the real checkout. Default: current directory [default: .]
* `--config <CONFIG>` — Explicit `cypera.toml` path (default: discovered upward from `--repo`)
* `--out <OUT>` — API output *dir* override — replaces `[api].out`/`docs/public/api`; a relative path resolves against the cwd (run from the docs worktree to land in a detached docs checkout, the python `DOCS` anchor)
* `--target-dir <TARGET_DIR>` — Cargo target dir for the doc build — exported as `CARGO_TARGET_DIR` when unset (`[api].target_dir` fallback; the myco `target/docs-api` isolation)
* `-h, --help` — Print help

## `cypera check`

Lint a repo's docs contract — README/AGENTS shape, no CHANGELOG, no version-in-prose, Diátaxis docs site, generated-reference freshness, `docs:check` script

**Usage:** `cypera check [OPTIONS]`

###### **Options:**

* `--repo <REPO>` — Repo root. Default: current directory [default: .]
* `--config <CONFIG>` — Explicit `cypera.toml` path (default: discovered upward from `--repo`)
* `--json` — Emit the report as JSON
* `-h, --help` — Print help
