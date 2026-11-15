# Decisions

ADR-lite: the load-bearing decisions, each with the alternative it
beat and the consequence it carries. Design history lives in
`project/adrs/`.

## One generator for every repo

**Decision:** the shared successor to per-repo `docs/gen-*.py`
scripts — crate map, CLI reference, config reference, API index,
docs check, rendered once for all siblings.

- *Alternative:* keep per-repo scripts (the status quo that
drifted).
- *Consequence:* one codebase owns every generated page; a fix
lands everywhere at once.

## Pure core, IO in the shell

**Decision:** `cypera-core` is string-in/markdown-out — models and
renderers only, no `std::fs`/`std::process`; `cypera-cli` owns
argv, walks, emit.

- *Alternative:* a single crate doing everything.
- *Consequence:* renderers unit-test without a filesystem; the
org's ports-and-adapters-lite convention holds.

## `cypera.toml` is the per-repo contract

**Decision:** layer maps, markers, curated struct lists, titles,
and preambles live in config, not code — schema in
`crates/cypera-core/src/repo_config.rs`.

- *Alternative:* flags for everything, or hard-coded per-repo
  tables in source.
- *Consequence:* onboarding a repo is writing a TOML, not
patching cypera.

## Marker splice over wholesale rewrite

**Decision:** `--out` on a `<!-- crate-map -->`-bearing file
replaces only the table rows; other files get the full page.

- *Alternative:* always overwrite the whole file.
- *Consequence:* generated tables can live inside hand-written
prose (the architecture page) without destroying it.

## `--repo` is explicit, never discovered upward

**Decision:** generators take `--repo` pointing at the workspace
root; no upward `Cargo.toml` search.

- *Alternative:* discover like cargo does.
- *Consequence:* scripts and CI stay deterministic — same
invocation, same repo, everywhere.

## Walk `--help`, don't parse clap-markdown

**Decision:** `gen cli` recurses `<bin> --help` itself;
`--input` wrap remains only for repos still emitting
`gen-cli-docs` bodies.

- *Alternative:* depend on each engine's clap-markdown producer.
- *Consequence:* the reference tracks `--help` byte-for-byte;
producer crates can retire independently.

## `check` encodes the contract, substance stays editorial

**Decision:** `cypera check` enforces the mechanical docs rules
(README/AGENTS/Diátaxis/freshness/content pages per profile);
narrative quality is not machine-checked.

- *Alternative:* a linter for prose quality.
- *Consequence:* drift is a build failure, not a review comment;
the bar stays honest without false positives.
