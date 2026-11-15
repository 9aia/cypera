# Concepts

The domain vocabulary, each term defined once. Field-level
contracts live in [Reference](../reference/).

## Generator

A `cypera gen <kind>` verb: `crate-map`, `cli`, `config`, `api`.
Each reads repo sources and emits markdown — stdout by default,
`--out` to a file.

## `cypera.toml`

The per-repo contract discovered upward from `--repo`/cwd: layer
overrides, curated struct lists, titles, markers, splices, the
`[check]` profile. Every `gen-*.py` wrapper's hard-coded values
belong here — the schema is
[`cypera.toml`](../reference/cypera-toml).

## Crate map

The crate/layer/purpose table every repo's explanation carries:
workspace members + globs → per-crate rows, layer guessed by
naming convention (`-core`→domain, `-cli`→binary, bare→binary,
else adapter) and overridable via `--layer` or `[crate_map.layers]`.

## Marker splice

`<!-- crate-map -->` in a host page (typically
`explanation/architecture.md`): `--out` on a marker-bearing file
replaces only the table rows after it — prose untouched. This is
the `gen-crate-map.py` convention, kept.

## Entry scan

The per-crate read of `src/lib.rs`/`main.rs`: `//!` inner doc →
summary paragraph; top-level `mod`/`pub mod` → module list.

## Config scan

`pub struct` doc comments → field tables. `[config.structs]`
curates which structs render; `[config.types]` maps Rust types to
prose vocabulary.

## CLI walk

`gen cli` recurses `<bin> --help` (or wraps `gen-cli-docs`
clap-markdown via `--input`) into a reference page. clap is the
source of truth — the page can never drift from `--help`.

## API index

`gen api` runs `cargo doc --workspace --no-deps`, resolves the
target dir via `cargo metadata` (`<NAME>_ROOT` env override),
copies `<target>/doc` into `docs/public/api`, and writes a styled
crate index — the `gen-api*.py` port.

## Docs contract (`cypera check`)

The org docs rules — README shape, AGENTS.md sections, no
CHANGELOG, no versions in prose, Diátaxis layout, `docs:check`
script, generated-file freshness, content pages per `[check]
profile` — enforced mechanically; `--json` for CI.
