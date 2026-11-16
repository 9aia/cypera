+++
id = "P-002"
title = "Org LoC coverage invariant — tamias config packs + lasius doctor check"
status = "open"
created = "2026-11-16"
owner = "devin"
priority = "med"
labels = ["distribution", "loc"]
+++

## Context

The loc pipeline moved to tamias-distributed tooling (P-001):
`cypera-toolbox` pack + per-org `*-loc-config` pack installed by
`loc-org.yml` reusable workflow (semver tags, `setup-tamias` action).
KTB Health is live; 9aia org rollout added the same thin caller +
config pack (count-only, no docs publish).

## Invariant

Every non-archived org repo must appear in `<org>/.github`'s
`packs/loc/loc.yaml` `repos:` list — the config pack is the
single source of truth for what the weekly workflow counts.

## Enforcement

`lasius doctor` gained a `loc coverage` check: when
`repos/.github/packs/loc/loc.yaml` exists it compares the `repos:`
list against a raw `gh repo list` (archived filtered, private
included — loc counts private repos via LOC_READ_TOKEN, unlike the
provider's clone-filtered list). Missing/stale entries fail doctor
in both text and `serve doctor` JSON.

## Rollout pattern (per org)

1. `<org>/.github/packs/loc/{tamias.yaml,loc.yaml}` — pack manifest
   + org knobs (repos, docs outputs if any, broker URL).
2. `<org>/.github/.github/workflows/loc.yml` — thin caller:
   `uses: 9aia/cypera/.github/workflows/loc-org.yml@<semver>` +
   `config_ref: git:<org>/.github//packs/loc#main`.
3. `lasius doctor` verifies coverage drift.

## Open questions

- bins[] for loc scripts (still P-001) — files pack + store paths
  works; a `cypera-driver` would make `cypera loc …` a first-class
  command.
- Registry publish (`tamias publish` + named refs) vs git-tag refs.
- 9aia docs publish target — no central metrics page yet; add a
  docs_repo/docs_outputs pair when one exists.
- LOC_READ_TOKEN secret for 9aia private repos — until set, private
  repos count as skipped in the weekly report.
