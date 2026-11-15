# Boundaries

What cypera owns — and the neighbors it deliberately doesn't.

## Owns

- **Reference-doc generation** — crate maps, CLI references,
  config references, the rustdoc index page (`gen api`), and the
  `cypera.toml` schema.
- **The docs contract** — `cypera check`'s mechanical rules for
  README/AGENTS/Diátaxis/freshness/content pages.
- **The generation pipeline** — workspace walks, member-glob
  expansion, entry-file scans, doc-comment parsing, marker
  splicing, emit.

## Doesn't own

| Neighbor | Owns instead |
| --- | --- |
| Each repo's `docs/` worktree | The VitePress sites themselves — config, theme, deploy pipeline stay per-repo. |
| `gen-cli-docs` producers | Engines still emitting clap-markdown bodies; `gen cli --input` wraps them. |
| rustdoc internals | `gen api` copies `<target>/doc` verbatim — it does not parse rustdoc JSON. |
| [tamias](https://9aia.com/tamias/) | Distribution — the `cypera` command is a `bins[]` pack shim over `tamias-driver-cypera`. |
| [myco](https://9aia.com/myco/) / [sturn](https://9aia.com/sturn/) / [kuda](https://9aia.com/kuda/) / [tarsi](https://9aia.com/tarsi/) | Ledger, coordination, code maps, releases — cypera consumes their sources as inputs only. |

The rule: if a page is *generated from source*, cypera can own it;
if a page is *written for humans*, the repo owns it.
