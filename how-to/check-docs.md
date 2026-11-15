# Check the docs contract

`cypera check` verifies that a repo's documentation meets the org
contract — the same rules every sibling repo is held to. It is the
gate `docs:check` scripts run in CI.

## Run it

```bash
cypera check --repo /path/to/repo           # text report
cypera check --repo /path/to/repo --json    # machine-readable
```

Exit status is non-zero when any rule fails; `SKIP` never fails.

## The rules

| rule | what it checks |
| --- | --- |
| `readme` | `README.md` exists, is under 200 lines, has what/why/quickstart-shaped content, and contains an install or run command. |
| `agents-md` | `AGENTS.md` exists with Layout-, Commands-, and Boundary-ish sections. |
| `no-changelog` | No `CHANGELOG*` at the repo root — releases are tarsi's job. |
| `no-version-in-prose` | No `v?\d+\.\d+\.\d+` in prose (`.md`/`.txt`); manifests and lockfiles are exempt, as are fenced code blocks, inline code spans, and raw HTML `<script>`/`<style>` blocks. |
| `docs-site` | A `docs/` worktree (or equivalent) with the four Diátaxis quadrants: `tutorials/`, `how-to/`, `reference/`, `explanation/`. |
| `docs-check-script` | `docs/package.json` has a `docs:check` script. |
| `generated-fresh` | Every output declared in `cypera.toml` regenerates byte-identically — cypera rebuilds each page in memory and diffs (no writes). Without `cypera.toml` the rule skips. |

Profile-driven rules (gated by `[check] profile` in `cypera.toml`;
all SKIP without one, and when the `docs/` worktree is absent):

| rule | what it checks |
| --- | --- |
| `content-pages` | The required docs tree per profile — `engine`: all four Diátaxis quadrant dirs, ≥1 tutorial, ≥3 how-tos, and `explanation/{architecture,concepts,boundaries,decisions}.md`; `library`: `index.md` + `explanation/{architecture,concepts}.md`; `content`: exempt. |
| `tutorial-count` | `docs/tutorials/` holds exactly one markdown file for `engine` (the single golden path), at most one for `library`; `content` exempt. |
| `tutorial-name` | The canonical tutorial filename is `docs/tutorials/getting-started.md` — `engine` FAILs when `tutorials/` has markdown but none is `getting-started.md`; `library`/`content` SKIP (engine-only contract). |
| `index-router` | `docs/index.md` links to at least 3 of the 4 quadrant dirs, routing each reader journey. |
| `no-orphan-getting-started` | No `getting-started.md` at the `docs/` root — the golden path lives in `docs/tutorials/`. |

Every finding carries `file:line` evidence where applicable.

A repo that legitimately can't satisfy a rule — a content-only
ledger mount, a theme package, the website repo itself — exempts
it with `[check] skip = ["docs-site", ...]` in `cypera.toml`
(see [the schema](/reference/cypera-toml)). Exempt rules still run
and report `SKIP`, which never fails the check; unknown names warn
on stderr.

## Wire it into a repo

1. Add `cypera.toml` (see [the schema](/reference/cypera-toml)).
2. Add the script to `docs/package.json`:

   ```json
   "docs:check": "cargo run -q --manifest-path ../cypera/Cargo.toml -p cypera -- check --repo .."
   ```

3. Run `cypera gen …` once so the declared outputs exist, then
   `cypera check --repo .` stays green as long as they stay fresh.
