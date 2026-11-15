# Getting started

## Install

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
| `cypera gen cli` | a sibling's `gen-cli-docs` output (clap-markdown) | wrapped CLI reference page |
| `cypera gen config` | `pub struct` doc comments in Rust sources | config field tables |

All three print markdown to stdout by default; `--out <file>`
writes a page (or splices a table — see below).

## First run

```bash
# crate map for any repo — dogfood on cypera itself
./target/debug/cypera gen crate-map --repo .

# CLI reference — pipe a sibling engine's gen-cli-docs verb
myco gen-cli-docs | cypera gen cli --title "myco — commands"

# config reference — point at the config struct sources
cypera gen config --file crates/myco-core/src/config.rs
```

## Where the output goes

Sibling repos keep generated pages on their `docs` branch (the
`docs/` worktree). Convention:

- Full pages → `docs/reference/*.md`, written wholesale by `--out`.
- Tables embedded in prose → leave a `<!-- crate-map -->` marker in
  the host page and pass it as `--out`; cypera replaces only the
  table rows after the marker (the `gen-crate-map.py` convention).
