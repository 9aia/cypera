+++
id = "P-001"
title = "Everything-via-tamias: distribute cypera + org tooling as git-ref packs"
status = "open"
created = "2026-11-16"
owner = "devin"
priority = "med"
labels = ["distribution", "tamias"]
+++

## Context

Org tooling is scattered across distribution mechanisms: product CLIs
ship as tamias packs, cypera tools are consumed via pinned
`actions/checkout` in reusable workflows, and tamias itself upgrades
via `self-upgrade`. The direction: **everything installable goes
through tamias with git-url refs** (`git:org/repo//subdir#<tag>`),
with tamias as the single distribution plane — including tamias
self-upgrade for its own packs.

## First step (done)

`cypera/tamias.yaml` — the toolbox is now a `type: files` pack:
`tamias install git:9aia/cypera#<tag>` verified end-to-end
(cypera@0.1.0 in the local store).

## Open questions

- Should loc (and future cypera tools) expose `bins[]`? Scripts can't
  ride `shim: cli` (needs a compiled driver) or `tools/bin/<triple>`
  payloads — options: a small `cypera-driver` Rust bin, wrapper shim
  scripts under `bin/`, or keep `files` + documented store paths.
- CI stays on `actions/checkout` at pinned tags — acceptable, or
  should reusable workflows `tamias install` instead (requires tamias
  on runners)?
- Publish flow: keep tag-per-release (`loc-vN`) or move to
  `tamias publish` + named registry (`9aia:cypera@<ver>`)?
- Audit other org repos for pack-able surfaces (scripts, templates,
  configs) — inventory before standardizing.

## Non-goals

- Replacing the myco/kuda/sturn content types — they already ride
  tamias.
- Changing the pinned-tag CI contract unilaterally.
