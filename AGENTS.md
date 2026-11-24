# Cypera

Reusable documentation/reference-generation utilities for the 9aia
org — the shared successor to the per-repo `docs/gen-*.py` scripts.
Every sibling docs site regenerates the same pages: a crate map,
a CLI reference (from each engine's `gen-cli-docs` verb), and a
config reference (from config-struct doc comments). cypera renders
all three, once, for every repo — killing per-repo script drift.

Rust workspace, ports-and-adapters-lite: `cypera-core` is pure
domain (models + markdown renderers, **no I/O**); `cypera-cli` is
the composition root — clap argv, filesystem walks, emit.
Distribution is a tamias pack (`libraries/cypera`): the `cypera`
command ships as a generated `bins[]` shim over
`tamias-driver-cypera`, like every other 9aia tool. Named for the
*cypera*, the swift — docs that keep pace with the code.

## Layout

| Crate | Role |
| --- | --- |
| `crates/cypera-core` | Pure domain — `Cargo.toml` manifest models (`manifest`), crate-map model + renderer + entry-file scanner (`crate_map`), config-struct doc-comment scanner + renderer (`config`), CLI help-walk parser/renderer (`cli`), the rustdoc index-page renderer (`api`), the `cypera.toml` schema (`repo_config`). String in, markdown out; never touches `std::fs`/`std::process`. |
| `crates/cypera-cli` | package `cypera` — the binary + engine lib (`parse_from`/`run_from`/`command()` per org convention). `gen crate-map` (workspace walk, member-glob expansion, `src/lib.rs`/`main.rs` scan, `--layer` overrides, `--out` with marker splicing), `gen cli` (recursive `<bin> --help` walk, or `--input` wrap of `gen-cli-docs` markdown), `gen config` (`--file` scans, `--only` filter, or `cypera.toml` curated sections), `gen api` (`cargo doc --workspace --no-deps` + `cargo metadata` target-dir resolution → `<target>/doc` copied into `docs/public/api` + styled crate index — the `gen-api*.py` port; `<NAME>_ROOT` env override, `[api]` config), `check` (the docs contract — README/AGENTS/Diátaxis/freshness, `--repo`, `--json`). All gens discover `cypera.toml` upward from `--repo`/cwd. |
| `crates/cypera-driver` | `tamias-driver-cypera` — pavo driver exposing the engine ops surface: every bounded `cypera` verb as a `cli.<verb>` wire query derived from `cypera::command()` (no views, no surfaces — every verb is bounded), `run <argv>` + `cli <argv>` passthroughs re-execing the engine in-process (`cli` is the `bins[]` shim verb — strips `--dir`/`--name`, inherited stdio), feature-gated `mcp` embedded surface. The shipped `cypera` command rides this driver via `libraries/cypera`. |
| `libraries/cypera` | the tamias pack — `tamias.yaml` declares `bins: [{name: cypera, shim: cli, driver: cypera}]`; `cargo xtask pack` stages `drivers/bin/<triple>/tamias-driver-cypera` (gitignored) |
| `xtask` (`cypera-xtask`) | `check` (fmt + clippy -D warnings) / `test` (nextest or cargo test) / `coverage` (llvm-cov) / `pack` (stage the pack payload) / `install` (release driver → `~/.local/bin`) / `fmt` |

## Commands

```bash
cargo xtask check          # fmt --check + clippy -D warnings
cargo xtask test           # whole suite (nextest if present)
cargo xtask coverage       # llvm-cov summary; extra args pass through
cargo xtask pack           # stage libraries/cypera driver payload
                           # (release tamias-driver-cypera → drivers/bin/<triple>/)
cargo xtask install        # release-build tamias-driver-cypera →
                           # ~/.local/bin; the `cypera` command itself
                           # is the tamias pack's generated bins[] shim —
                           # `cargo xtask pack` + `tamias install
                           # --allow-drivers ./libraries/cypera`
cargo xtask fmt            # cargo fmt --all
```

Git hooks live in `.hooks/` (`core.hooksPath` = `.hooks`):
pre-commit runs `cargo fmt --all -- --check`, pre-push runs
`cargo xtask check`. `./scripts/setup.sh` wires them on fresh clones.

## Boundary

Cypera owns **reference-doc generation** — turning repo sources
into the markdown pages sibling VitePress sites consume. It does
not own:

- **The docs sites themselves** — each repo's `docs/` worktree,
  VitePress config, and deploy pipeline stay per-repo.
- **`gen-cli-docs` producers** — cypera's `gen cli` now walks
  `<bin> --help` itself; the wrap mode (`--input`) remains for
  repos still emitting clap-markdown bodies.
- **rustdoc internals** — `gen api` orchestrates `cargo doc` and
  copies `<target>/doc` verbatim; it does not parse rustdoc JSON.
  cypera's `crate_api` config only emits the rustdoc *link list*.
- **myco/sturn/kuda/tarsi** — ledger, coordination, code maps, and
  releases are their own repos' concerns.

## Rules

- Workspace lints are shared (`[workspace.lints]`): `unsafe_code`
  forbidden, `unwrap_used` warned — `-D warnings` in `xtask check`
  means no warnings tolerated. Tests `#![allow]` liberally.
- No `unsafe`. Edition 2021.
- `cypera-core` stays pure — no `std::fs`/`std::process`/`std::net`.
  IO lives in `cypera-cli`.
- `cypera.toml` is the per-repo contract — every `gen-*.py`
  wrapper's hard-coded layer maps, markers, curated struct lists,
  titles, and preambles belong there, not in code. Schema lives in
  `crates/cypera-core/src/repo_config.rs`; document changes in the
  README + docs site.
- `cypera check` encodes the org docs contract (README shape,
  AGENTS.md sections, no CHANGELOG, no versions in prose, Diátaxis
  layout, `docs:check` script, generated-file freshness). Extend the
  rules in `crates/cypera-cli/src/check.rs` rather than re-inventing
  per-repo checks.
- Keep the binary dependency-light and sync; no async runtime.
- English everywhere (code, comments, commit messages).
- Commits: conventional prefixes (`feat:`/`fix:`/`refactor:`/`docs:`/`chore:`).

## Docs

`docs/` worktree — VitePress + Diátaxis (tutorials/how-to/reference/
explanation) → GitHub Pages at `9aia.com/cypera` via
`bun run docs:deploy` from the `docs/` worktree. Dogfood: cypera's
own reference pages are generated by `cypera gen`.

<!-- myco:agent-awareness -->

## Agent awareness

- Session start / any work discussion: run `myco status --prompt`
  before proposing next steps — know the board first.
- User describes new work → offer to capture it (`myco todo add`,
  `issue new`, `proposal new`). Don't wait to be asked.
- Picking up a task: `myco resume <ID>` is the briefing.
- Wrapping up: `myco handoff <ID>` anything resumable; keep statuses
  honest (claimed → in-progress → blocked/done).

<!-- /myco:agent-awareness -->

<!-- sturn:agent-awareness -->

## Sturn awareness

- **No PRs.** The pushed branch/commit is the deliverable; review
  streams continuously (sturn WAL), merge is a human/sturn call.
- sturn-wired repo (`sturn.toml` present): `sturn claim "<glob>"
  --ws <ws>` *before* edits — claims surface conflicts in ms.
- Ops via `sturn write|rename|delete`; `sturn snap` folds the ws
  into a real commit; `sturn push` publishes heads + refs/sturn/*.
- `sturn leave` on exit — drops the actor's claims.

<!-- /sturn:agent-awareness -->

<!-- kuda:agent-awareness -->

## Kuda awareness

- `.kuda/` cards are committed artifacts — read them before
  re-exploring (`kuda map`, `kuda card <mod>`); `kuda status`
  reports per-card drift vs HEAD.
- Code changes that invalidate a card → regenerate it (`kuda
  sync`) in the same change; cards ride the diff they describe.
- `.kuda/index.sqlite` is gitignored + rebuildable; the `*.yaml`
  cards are the checked-in source of truth.

<!-- /kuda:agent-awareness -->

## Specs

`specs/<name>.toml` is this repo's mimus-standard conformance spec —
approval-gated. Any change requires a live org proposal
(`proposal-NNNN`, status open|done) referenced in the spec header or
commit range, and no coverage regression vs merge-base. Guard:
workspace `../../scripts/spec-guard.py`; sweep:
`../../scripts/standard.sh`; regenerate coverage only via
`../../scripts/gen-repo-specs.py`.
