# `cypera.toml` — the per-repo contract

Every generator discovers `cypera.toml` by walking upward from
`--repo` (or the cwd). The file absorbs what the per-repo
`docs/gen-*.py` wrappers used to hard-code: layer maps, markers,
curated struct lists, titles, preambles, and output paths. With it
present, `cypera gen …` needs no flags; `cypera check` uses it to
verify generated files are fresh.

All keys are optional; an absent file means "legacy mode" (explicit
flags only). Paths are repo-relative unless absolute. The serde
model lives in `crates/cypera-core/src/repo_config.rs`.

## `[crate_map]` — `cypera gen crate-map`

| key | type | meaning |
| --- | --- | --- |
| `out` | string | Full-page emit target, e.g. `docs/reference/crate-map.md`. Used when no `--out` flag and no splices/crate-api apply; also the freshness target for `check`. |
| `order` | string list | Layer sort order, replacing the default (`domain`, `port`, `adapter`, `ops`, `service`, `engine`, `driver`, `testkit`, `tooling`, `binary`, `impl`). Unknown layers sort last, alphabetically. |
| `default_layer` | string | Fallback layer for unclassified crates — the kuda/mimus `"impl"` convention. Default `"adapter"`. |
| `[crate_map.layers]` | table | Per-crate layer overrides: `"my-crate-xtask" = "tooling"`. |
| `[[crate_map.splices]]` | array | In-place table splices into existing pages. Each entry: `target` (required, repo-relative file), and exactly one of `marker` (replace the pipe-table run after the first line containing this string) or `first_table = true` (replace the first crate table — the no-marker convention). |
| `[crate_map.crate_api]` | table | Rustdoc link-list page: `site` (the docs-site name used in `/{site}/api/…` URLs) and `out` (e.g. `docs/reference/crate-api.md`). |

Splice example:

```toml
[[crate_map.splices]]
file = "docs/explanation/architecture.md"
marker = "<!-- crate-map -->"
```

## `[api]` — `cypera gen api`

The rustdoc API site — successor to the per-repo `docs/gen-api.py`
+ `docs/gen-api-index.py` pair (and the inline `docs:api` shell
bupha/kuda/mimus carried). `gen api` runs `cargo doc --workspace
--no-deps` in the repo, resolves the real target dir via `cargo
metadata`, empties the api dir, copies `<target>/doc` into it, and
writes a styled `index.html` listing the crate dirs. Output is a
gitignored build artifact — `check` freshness does not apply. Every
key is optional; absent `[api]` means the python defaults.

| key | type | meaning |
| --- | --- | --- |
| `out` | string | Rustdoc site dir, repo-relative. Default `docs/public/api`. A `--out` flag overrides it (a path, resolved against the cwd — run from the docs worktree to land in a detached docs checkout). |
| `prefix` | string | Crate-dir name prefix the index lists and the title is built from — the python `startswith("<repo>")` filter. Default: the repo directory name. |
| `title` | string | Index `<title>`/`<h1>` text. Default `"{prefix} — Crate API"`. |
| `target_dir` | string | Repo-relative cargo target dir exported as `CARGO_TARGET_DIR` for the doc build **when the env var is unset** — the myco `target/docs-api` isolation convention (a shared target dir served stale rmeta). |

Repo-root resolution keeps the python `<NAME>_ROOT` env convention:
`<NAME>` is the `cypera.toml` dir's basename uppercased (`tamias` →
`TAMIAS_ROOT`), and when set it overrides `--repo` — how detached-sha
doc pipelines point cargo at the real checkout while `--out
public/api` (cwd-relative, from the docs worktree) picks the output.

```toml
[api]
out        = "docs/public/api"
target_dir = "target/docs-api"   # myco's isolated doc target
```

## `[cli]` — `cypera gen cli`

A recursive `<bin> --help` walk that reproduces the per-repo
`gen-cli-docs.py` scripts byte-for-byte.

| key | type | meaning |
| --- | --- | --- |
| `package` | string | Cargo package to `cargo build -q -p` once, then invoke `target/debug/<bin>`. When absent, `bin` is executed directly (PATH or absolute path). |
| `bin` | string | Binary name under `target/debug/` (cargo mode) or the executable itself (direct mode). Required. |
| `prefix` | string list | Argv prefix before the walked subcommand path — the `["run"]` engine-passthrough shim, or empty. |
| `title` | string | Page title override. Default: `Command-Line Help for` plus the program name derived from the root usage line. |
| `out` | string | Emit target — freshness check only; generation writes to stdout or `--out`. |

```toml
[cli]
package = "lasius-driver"
bin = "tamias-driver-lasius"
prefix = ["run"]
out = "docs/reference/cli.md"
```

The walk skips the synthetic `help` command, follows `Commands`
entries depth-first, and parses `Usage:`/about/`Arguments`/
`Commands`/`Options` sections exactly like the python scripts.

## `[config]` — `cypera gen config`

Curated config-reference pages — the successor to
`gen-config-docs.py`'s per-repo struct lists, type vocabularies,
preambles, and prose sections.

| key | type | meaning |
| --- | --- | --- |
| `out` | string | Emit target for `check` freshness. |
| `title` | string | `# ` page title. |
| `files` | string list | Rust sources to scan for `pub struct`s. |
| `[[config.structs]]` | array | Named struct lookups: `file` + `name` (+ optional `title`). A name matching no `pub struct` renders a header-only section — the python `fields() → []` behavior. |
| `[config.types]` | table | Rust type → display name (`"Vec<String>" = "list"`). |
| `[config.field_types]` | table | `Struct.field` → display name, winning over `types`. |
| `[[config.sections]]` | array | Ordered page sections. Each has `title` + `level` (heading depth, default 2) and exactly one body: `struct` (a name from `structs`), `text` (inline markdown), or `text_file` (repo-relative markdown file). `struct` may combine with `text`/`text_file` for table-plus-prose sections. |
| `[config.style]` | table | Page-chrome overrides: `preamble` (prose between title and marker), `marker` (verbatim marker block), `table_head`/`table_sep` (custom field-table header/separator, e.g. tarsi's `key/meaning`). `preamble_file` reads the preamble from a repo-relative file. |

Field rules (python parity): only `///`-documented `pub` fields
appear; `Option<T>` and a literal `serde(default` attribute mark a
field optional and unwrap the inner type; notes escape `|`, `<`,
`>`.

## Freshness — what `cypera check` verifies

`cypera check --repo <path>` regenerates every declared output
in-memory and diffs against the checked-in file — crate-map page,
splices, crate-api page, cli page (real `--help` walk), config
page. No writes. A repo without `cypera.toml` reports the rule as
`SKIP`.
