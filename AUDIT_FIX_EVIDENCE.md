# GitHub audit remediation evidence

Status: implementation evidence for `GF-AUD-003` and `GF-AUD-041`.

## Starting state

- Branch: `main`
- Commit: `30e37251481d3b54b15ba4d1b4cbb11b46245ce0`
- Version: `0.0.0-private`
- Baseline reproduction: a valid fixture plus one malformed mandatory contract
  exited zero under `guard`; `coverage --minimum 0` also exited zero.

## Remediation

- Every mandatory evidence class tracks discovered, parsed, malformed,
  duplicate, and validated totals.
- Missing, malformed, duplicate, unreferenced, and empty evidence fails health.
- Schema IDs and explicit contract IDs must be unique.
- CLI options are command-scoped; thresholds must be safe integers >= 1; usage
  errors are stable and do not print stack traces.
- Report schema advanced to `sley.conformance.report.v1`; package version
  advanced to `0.0.1-private`.

## Regression coverage

`npm test` covers a valid fixture, malformed JSON among valid files, duplicate
schema and contract identifiers, empty evidence, zero/negative/fractional/NaN/
missing thresholds, missing option values, unknown options, help, Markdown,
and JSON output.

Rollback is a code-only revert with no persisted state migration.
