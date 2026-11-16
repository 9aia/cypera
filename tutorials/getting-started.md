# Getting started

## Install

The `cypera` command ships as a [tamias](https://github.com/9aia/tamias)
pack (`libraries/cypera`) — a generated `bins[]` shim over
`tamias-driver-cypera`, like every other 9aia tool. With `tamias`
installed:

```bash
tamias install --allow-drivers ./libraries/cypera   # from a checkout (after cargo xtask pack)
```

From a checkout without tamias:

```bash
cargo install --path crates/cypera-cli   # from a checkout
# or
cargo build --release -p cypera          # then use target/release/cypera
```

Requires a stable Rust toolchain; no other runtime dependencies.

## What cypera generates

| Generator | Source | Output |
| --- | --- | --- |
| `cypera gen crate-map` | workspace + member `Cargo.toml`s, `src/lib.rs`/`main.rs` scans | crate/layer/purpose table + per-crate module sections |
| `cypera gen cli` | a binary's recursive `--help` walk (or wrapped `gen-cli-docs` markdown via `--input`) | CLI reference page |
| `cypera gen config` | `pub struct` doc comments in Rust sources, or the curated `[config]` spec in `cypera.toml` | config field tables |

All three print markdown to stdout by default; `--out <file>`
writes a page (or splices a table — see below).

## First run

```bash
# crate map for any repo — dogfood on cypera itself
./target/debug/cypera gen crate-map --repo .

# CLI reference — the --help walk; the command spec comes from
# [cli] in the target repo's cypera.toml (or --package/--bin/--prefix)
cypera gen cli --repo /path/to/myco

# …or wrap a gen-cli-docs markdown body instead
myco gen-cli-docs | cypera gen cli --input - --title "myco — commands"

# config reference — curated [config] spec via --repo, or scan
# struct sources directly (legacy mode)
cypera gen config --repo /path/to/myco
cypera gen config --file crates/myco-core/src/lib.rs
```

## Where the output goes

Sibling repos keep generated pages on their `docs` branch (the
`docs/` worktree). Convention:

- Full pages → `docs/reference/*.md`, written wholesale by `--out`.
- Tables embedded in prose → leave a `<!-- crate-map -->` marker in
  the host page and pass it as `--out`; cypera replaces only the
  table rows after the marker (the `gen-crate-map.py` convention).

## What you now know

- The three generators (`crate-map`, `cli`, `config`) plus `api`
  and `check`, and where each reads its truth from.
- `--out` writes pages wholesale; a `<!-- crate-map -->` marker
  splices tables into prose.
- `cypera.toml` is where per-repo hard-coding belongs.

## Go deeper

- [Generate a crate map](../how-to/generate-a-crate-map.md) — the
  full walk: globs, layers, splices.
- [Check the docs contract](../how-to/check-docs.md) — the rules
  `cypera check` enforces.
- [Architecture](../explanation/architecture.md) — why core is
  pure and the shell owns I/O.
