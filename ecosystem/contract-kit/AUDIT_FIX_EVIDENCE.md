# Audit Fix Evidence

Version: 0.1.1

Resolved findings: GF-AUD-024 and GF-AUD-037.

- Empty fixture/inventory/comparison evidence fails unless the explicit,
  command-scoped `--allow-empty` bootstrap option is present.
- Fixture comparison requires at least one comparable contract and checks
  schema, root marker, and top-level identifier drift.
- Duplicate top-level fixture IDs and schema IDs fail with file-specific errors.
- Contract files are regular files bounded by count, bytes, JSON depth, and a
  fixed node safety ceiling.
- The CLI rejects unknown/duplicate options and missing values. `--json`
  errors are stable JSON without raw stack traces.

Validation: `npm test`, `npm run fmt`, and `git diff --check`.
