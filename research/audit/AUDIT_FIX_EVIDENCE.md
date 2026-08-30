# GitHub audit remediation evidence

Status: implementation evidence for `GF-AUD2-005`, `GF-AUD2-016`,
`GF-AUD2-017`, and `GF-AUD2-035`.

## Starting state

- Branch: `master`
- Commit: `10de414350ada6be833e10518e76d4751b03161b`
- Version: `0.1.0`
- Baseline reproduction: no arguments printed help and exited zero; action metadata
  used invalid `named` and paired a Node action runtime with a Bash entry point;
  read failures were silently skipped; reports emitted absolute paths and SHA-1.

## Remediation

- The action is a valid composite action with environment-only input transport,
  an explicit target, and fail-closed default behavior.
- No arguments scans `.`, high findings exit `1`, and incomplete coverage exits
  `2`; only explicit `--report-only` or `--allow-empty` relaxes those cases.
- Traversal rejects symlinks/special files and caps depth, count, per-file bytes,
  and aggregate bytes. Every omission has a stable reason and reconciled count.
- Reports default to root-relative paths and use domain-separated SHA-256 IDs in
  versioned `sley.audit.report.v1` output.
- Version advanced to `0.2.0`.

## Regression coverage

`npm test` covers default scans, clean/error/empty/missing targets, report-only,
unreadable files, symlink escapes, special files, oversized files, file-count
truncation, invalid UTF-8, cross-root stable fingerprints, SARIF versioning,
paths with spaces, and action metadata/input transport.

Release validation also runs syntax checks and, when installed, `actionlint`.
Rollback is code-only; retaining report schema v1 compatibility is preferred.
