---
layout: home
hero:
  name: cypera
  text: Org docs, generated once
  tagline: Reusable crate-map, CLI-reference, and config-reference generation — the shared successor to the per-repo docs/gen-*.py scripts.
  actions:
    - theme: brand
      text: Get started
      link: /getting-started
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
