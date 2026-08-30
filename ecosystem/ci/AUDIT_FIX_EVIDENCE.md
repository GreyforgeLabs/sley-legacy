# GitHub audit remediation evidence

Status: implementation evidence for `GF-AUD-001` and `GF-AUD-038`.

## Starting state

- Branch: `main`
- Commit: `120dd95f164129155117aff3dfebfc5e0d1ae5e1`
- Version: `0.0.0-private`
- Baseline: action input `target` was expanded inside the Bash source; smoke
  mode exited zero when Sley was unavailable; controlled step failure could
  exit before a final report; output escaping was incomplete.

## Remediation

- Action inputs cross the YAML/shell boundary only through environment
  variables and are quoted at Bash runtime.
- Controlled steps aggregate status without `set -e` early termination.
- The Node reporter applies real JSON escaping to captured output.
- Missing Sley or jq dependencies are `unavailable` and non-zero unless the
  caller explicitly selects `--allow-missing`.
- Version advanced to `0.0.1-private`.

## Regression coverage

`npm test` covers hostile target strings, exact single-argument delivery,
quotes, backslashes, tabs, carriage returns, line feeds, control bytes,
first-to-last failed steps, missing Sley, missing jq, the explicit skip path,
and the absence of input expressions from the action run body.

Run `actionlint`, ShellCheck, `npm run fmt`, and `npm test` for release
validation. Rollback is an action/script revert with no data migration.
