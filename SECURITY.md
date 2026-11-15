# Security Policy

Report vulnerabilities privately to the maintainers via GitHub:
https://github.com/9aia/cypera/security/advisories/new

Do not open public issues for security reports.

## Scope

- cypera reads repo sources (`Cargo.toml`, `src/*.rs`) and writes
  markdown — it never executes repo code, shells out, or fetches the
  network. The threat surface is malformed input files.
- `gen config` output is markdown built from doc comments — field
  docs are cell-escaped (`|`, `<`, `>`) but a hostile doc comment
  could still inject arbitrary markdown/HTML into a generated docs
  page. Treat generated pages as untrusted when scanning repos you
  don't control, and review diffs before committing them.
- `--out` paths are taken verbatim — cypera will overwrite (or
  marker-splice) any file the operator points it at. That is the
  intended behavior; there is no sandbox.
- No secrets are read, stored, or transmitted.

## Notes

We respond to reports as quickly as we can; expect acknowledgement
within a few days.
