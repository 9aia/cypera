# cypera

Reusable documentation/reference generation for the 9aia org — the
shared successor to the per-repo `docs/gen-*.py` scripts.

Named for *Cypera*, the swift — docs that keep pace with the code.

Every sibling repo's VitePress site regenerates the same three pages
from the same sources: a **crate map** (workspace `Cargo.toml`s +
`lib.rs` module docs), a **CLI reference** (each engine's
`gen-cli-docs` verb — clap-markdown output), and a **config
reference** (config-struct doc comments). cypera renders all three,
once, for every repo — killing the per-repo script drift.

## Install

```bash
cargo install --path crates/cypera-cli   # or: cargo build --release -p cypera
```

Requires a Rust toolchain (stable); no other runtime deps.

## Usage

```bash
# crate map — markdown page to stdout, or --out <file>
cypera gen crate-map --repo ../myco
cypera gen crate-map --repo ../myco --out docs/reference/crate-map.md
# …or splice just the table into a page with a <!-- crate-map --> marker
cypera gen crate-map --repo ../myco --out docs/explanation/architecture.md

# CLI reference — feed a sibling's `gen-cli-docs` output
myco gen-cli-docs | cypera gen cli --title "myco — commands" \
    --out docs/reference/commands.md

# config reference — scan config struct sources
cypera gen config --file crates/myco-core/src/config.rs \
    --title "Config — myco.toml" --out docs/reference/config.md
```

All generators accept `--out`; without it, markdown goes to stdout.
`gen crate-map` also takes `--layer crate-name=layer` overrides for
the domain/port/adapter/binary heuristic.

## Layout

| Crate | Role |
| --- | --- |
| `cypera-core` | Pure domain — manifest models, doc-comment scanners, markdown renderers. No I/O. |
| `cypera` (`crates/cypera-cli`) | The `cypera` binary — clap front-end, filesystem walks, stdout/file emit. |
| `cypera-xtask` | `cargo xtask check/test/coverage/fmt` workspace automation. |

## Docs

The `docs/` worktree (orphan `docs` branch) holds the VitePress +
Diátaxis site → GitHub Pages at `9aia.com/cypera`. Build with
`bun run docs:build` from `docs/`.

## Releasing

`scripts/release.sh` is an operator-run shim over `tarsi-serve
release` (tag-only; see `tarsi.toml`).
