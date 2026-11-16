# Architecture

Rust workspace, lean ports-and-adapters. `cypera-core` is pure
domain — models and markdown renderers, **no I/O**; `cypera-cli`
(package `cypera`) is the composition root — clap argv,
filesystem walks, stdout/file emit. The split is the org convention:
everything reconstructible from strings lives in core so it stays
unit-testable without a filesystem.

<!-- crate-map -->
| crate | layer | purpose |
|---|---|---|
| `cypera-core` | domain | Pure domain logic for cypera — doc-generation models and markdown renderers |
| `cypera-driver` | driver | tamias-driver-cypera — pavo driver exposing a cypera workspace's engine ops (no views) |
| `cypera-xtask` | tooling | Workspace automation — check / test / coverage / pack / install / fmt |
| `cypera` | binary | cypera — reusable documentation/reference generation for the 9aia org. |

Regenerate the table above (never hand-edit it):

```bash
cypera gen crate-map --repo . --out docs/explanation/architecture.md
```

## Data flow

```
repo sources                cypera-core (pure)           cypera-cli (IO)
─────────────               ─────────────────           ────────────────
Cargo.toml workspace   →    manifest models        →    workspace walk,
member Cargo.tomls          crate_map renderer          member-glob
src/lib.rs|main.rs          EntryScan                    expansion,
/// doc comments       →    config scanner         →    entry-file reads,
/// pub struct               renderers                    emit (stdout/--out)
gen-cli-docs output    →    cli page wrapper       →    stdin/file read
```

## Why a marker splice

The per-repo `gen-crate-map.py` scripts did two things: emit a
standalone crate-map page *and* splice the table into
`explanation/architecture.md` behind a `<!-- crate-map -->`
marker. cypera keeps both paths: `--out` on a marker-bearing file
replaces only the table rows; on any other file it writes the
full generated page.

## What cypera deliberately doesn't do

- **No upward repo discovery** — `--repo` is explicit so scripts
  and CI stay deterministic.
- **No rustdoc internals** — `gen api` orchestrates `cargo doc`
  and copies `<target>/doc` verbatim; it does not parse rustdoc
  JSON. `crate_api` config only emits the rustdoc *link list*.
- **No async, no daemon** — sync binary, scoped like its siblings.

See [Boundaries](boundaries.md) for what cypera owns and the
neighbors it deliberately doesn't.

## Where it sits

In the org's layering — **delivery** (tamias, scarab, tarsi,
bupha, hypha) → **memory** (myco, kuda) → **process** (sturn,
cuora, mimus, cicada, lasius, sepia, peregrin) → **interface**
(pavo).

Cypera sits *under* every layer: each repo's docs site
regenerates its reference pages through cypera, and `cypera check`
is the gate that keeps the org's docs contract honest. It is a
library-grade tool consumed by repos, not an organism the org runs.
