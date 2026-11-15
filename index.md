---
layout: home
hero:
  name: cypera
  text: Org docs, generated once
  tagline: Reusable crate-map, CLI-reference, and config-reference generation — the shared successor to the per-repo docs/gen-*.py scripts.
  actions:
    - theme: brand
      text: Get started
      link: /tutorials/getting-started
    - theme: alt
      text: Command reference
      link: /reference/commands
features:
  - title: One generator, every repo
    details: The same three pages every sibling docs site needs — rendered by one tool instead of drifting per-repo Python scripts.
  - title: Sources stay canonical
    details: Cargo.tomls, lib.rs module docs, gen-cli-docs output, and config-struct doc comments are the source of truth — pages are regenerated, never hand-edited.
  - title: Pure core, thin shell
    details: cypera-core is string-in/markdown-out with no I/O; cypera-cli owns argv and filesystem walks.
---

## What it is

Cypera is the org's reusable documentation generator: crate maps,
CLI references, config references, rustdoc index pages, and the
`cypera check` docs contract — rendered once for every sibling
repo instead of per-repo `gen-*.py` scripts.

## Why

Every 9aia docs site regenerates the same pages from the same
sources. Per-repo scripts drifted; cypera kills the drift — one
generator, one `cypera.toml` contract per repo, generated-not-
handwritten as an enforced rule.

## Boundary

- **Owns:** reference-doc generation + the docs contract check.
- **Doesn't own:** the docs sites themselves, `gen-cli-docs`
  producers, rustdoc internals, or any engine's ledger/coordination.

## Where to go next

| You want to… | Go to |
| --- | --- |
| Install and generate your first page | [Tutorial](/tutorials/getting-started) |
| Do a task (crate map, CLI ref, config ref, docs check) | [How-to](/how-to/) |
| Understand the design (core/shell split, markers, contract) | [Explanation](/explanation/) |
| Look up a verb, option, or config field | [Reference](/reference/commands) |
