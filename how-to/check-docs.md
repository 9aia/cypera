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
| `no-version-in-prose` | No `v?\d+\.\d+\.\d+` in prose (`.md`/`.txt`, plus `//` comment lines in `.rs`); manifests and lockfiles are exempt, fenced code blocks are skipped. |
| `docs-site` | A `docs/` worktree (or equivalent) with the four Diátaxis quadrants: `tutorials/`, `how-to/`, `reference/`, `explanation/`. |
| `docs-check-script` | `docs/package.json` has a `docs:check` script. |
| `generated-fresh` | Every output declared in `cypera.toml` regenerates byte-identically — cypera rebuilds each page in memory and diffs (no writes). Without `cypera.toml` the rule skips. |

Every finding carries `file:line` evidence where applicable.

## Wire it into a repo

1. Add `cypera.toml` (see [the schema](/reference/cypera-toml)).
2. Add the script to `docs/package.json`:

   ```json
   "docs:check": "cargo run -q --manifest-path ../cypera/Cargo.toml -p cypera -- check --repo .."
   ```

3. Run `cypera gen …` once so the declared outputs exist, then
   `cypera check --repo .` stays green as long as they stay fresh.
