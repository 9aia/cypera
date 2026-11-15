# Generate a crate map

The crate map is the table every repo's `explanation/architecture.md`
(or a standalone `reference/crate-map.md`) carries: one row per
workspace crate with its architectural layer and purpose.

## Run it

```bash
cypera gen crate-map --repo /path/to/repo
```

`--repo` is the directory containing the workspace `Cargo.toml` —
upward discovery is deliberately not done, so scripts stay
explicit. A bare `[package]` root (single-crate repo) works too.

## What it reads

- `[workspace] members` — plain paths pass through; `crates/*`
  globs expand to every subdirectory containing a `Cargo.toml`.
- Each member's `[package] name`/`description`; a member's
  `description.workspace = true` inherits
  `[workspace.package] description`.
- Each crate's entry file (`src/lib.rs`, else `src/main.rs`):
  `//!` inner doc becomes the crate's summary paragraph and
  top-level `mod`/`pub mod` declarations become the module list.

## Layers

The layer column is a heuristic over the org's naming convention:

| Crate name | Layer |
| --- | --- |
| `<repo>` (bare) | `binary` |
| `<repo>-core`, `<repo>-domain` | `domain` |
| `<repo>-cli` | `binary` |
| everything else | `adapter` |

Override per crate with `--layer name=layer` (repeatable):

```bash
cypera gen crate-map --repo . \
  --layer myco-ops=ops --layer myco-store=port
```

## Write it into a docs site

Full page:

```bash
cypera gen crate-map --repo . --out docs/reference/crate-map.md
```

Embedded table — put a marker in the host page, then splice:

```markdown
<!-- crate-map -->
| crate | layer | purpose |
|---|---|---|
| …stale rows… |
```

```bash
cypera gen crate-map --repo . --out docs/explanation/architecture.md
```

Only the `|…|` rows after the marker are replaced; the surrounding
prose is untouched.
