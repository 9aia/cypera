# Security Policy

Report vulnerabilities privately by emailing
luisfuturist+9aia@gmail.com with "SECURITY" in the subject — do
not open public issues, do not use GitHub's advisory or
private-reporting features, and do not disclose details publicly;
contact us first and we will coordinate disclosure privately.

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
