# Regenerate the CLI reference

`cypera gen cli` walks a binary's `--help` output recursively —
clap is the source of truth, so the reference can never drift from
`--help`. It reproduces the per-repo `gen-cli-docs.py` scripts
byte-for-byte. A `[cli]` table in `cypera.toml` declares the
package/binary/prefix; see
[`cypera.toml`](/reference/cypera-toml#cli--cypera-gen-cli).

## Configured mode

```toml
# cypera.toml
[cli]
package = "my-engine"     # cargo build -p once, then target/debug/<bin>
bin     = "my-engine"
prefix  = ["run"]         # engine-passthrough argv prefix, or omit
title   = "Command-Line Help for `my-engine`"   # optional
out     = "docs/reference/cli.md"               # freshness target
```

```bash
cypera gen cli --repo .          # writes every declared target
cypera gen cli --repo . --out /tmp/cli.md   # flag overrides config
```

## Wrap mode (legacy)

Sibling engines can also expose a `gen-cli-docs` verb that prints
their clap tree as markdown (via `clap-markdown`). cypera wraps that
output in a reference page — it does not parse clap itself:

```bash
myco gen-cli-docs | cypera gen cli --input - \
  --title "myco — commands" \
  --out docs/reference/commands.md
```

`--input` is what selects wrap mode — pass `-` for stdin or a
file path; without it `gen cli` runs the `--help` walk instead.
`--title` defaults to `Commands`. Wrap mode is for repos not yet
migrated to `[cli]` help-walk.

## What the walked page looks like

```markdown
# Command-Line Help for `my-engine`

This document contains the help content for the `my-engine`
command-line program.

**Command Overview:**

* [`my-engine`↴](#my-engine)
* [`my-engine init`↴](#my-engine-init)
…

## `my-engine init`

**Usage:** `my-engine init [OPTIONS]`

###### **Options:**

* `--all` — Every repo, not just missing ones
```

The walker descends `Commands` depth-first, skips the synthetic
`help` command, and renders the same shape the python
`gen-cli-docs.py` scripts produced.

## Wrap-mode page chrome

In wrap mode the body is passed through untouched — command-tree
formatting is the producer's job (`clap-markdown`); cypera owns
only the page chrome and the "regenerate, don't hand-edit" marker.
