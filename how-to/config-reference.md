# Regenerate the config reference

`cypera gen config` scans Rust sources for `pub struct` definitions
and renders each struct's documented `pub` fields as a markdown
table — the successor to the per-repo `gen-config-docs.py` scripts,
generalized to every `pub struct` in the given files (no per-repo
source map to maintain).

## Run it

```bash
cypera gen config \
  --file crates/myco-core/src/config.rs \
  --file crates/myco-core/src/lib.rs \
  --title "Config — myco.toml" \
  --out docs/reference/config.md
```

`--file` is repeatable and required. `--title` defaults to
`Config reference`; `--out` defaults to stdout.

## What it understands

- `///` doc comments on the struct and on `pub` fields — joined
  into the table's notes column.
- `Option<…>` fields and `#[serde(default…)]` attributes mark a
  field **optional**; everything else is **required**.
- `Option<T>` unwraps to `T` in the type column.

Limitations (deliberate — this is a doc scanner, not a Rust parser):

- Only `pub struct Name { … }` with the brace on the same line.
- Only line-oriented scanning: no macro-generated structs, no
  fields inside nested items.
- Doc comments must be adjacent to the item they document.

## Filtering

Large crates accumulate several config structs; `--only` (repeatable)
keeps just the named ones:

```bash
cypera gen config --file crates/x/src/config.rs \
  --only Config --only ViewSpec
```

An `--only` name that matches no `pub struct` is an error, not a
silent empty page.
