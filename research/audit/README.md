# Sley Audit

`Sley Audit` is a standalone scanning utility focused on the highest-yield security issue in this repo lineage: cleartext logging of structured values.

## SEO Surface

- SEO title: `Sley Audit - Agent-Native Security Scanner`
- SEO description: `Sley Audit is a focused scanner for debug-log leakage in agent-native development workflows and structured output paths.`
- Keywords: `Sley Audit`, `security`, `debug logging`, `static analysis`, `structured data leakage`, `agent-native tooling`, `source audit`
- Canonical URL: `https://github.com/GreyforgeLabs/sley-audit`
- Geo metadata:
  - Region: United States (US)
  - Language: English
  - Audience: agent-native language tooling teams and operators

## What it checks

- Rust logging-like macros that use debug formatting placeholders (`{:?}`, `{:#?}`).
- Rust `{:?}` / `{:#?}` with explicit `:?#`-style placeholders that commonly leak object payloads.
- Test/source snippets that interpolate sensitive variables with explicit debug formatting.

## Usage

```bash
sley-audit --path /path/to/repo
sley-audit --json --path /path/to/repo
sley-audit --sarif audit.sarif --path /path/to/repo
sley-audit --help
```

## Exit behavior

- Returns `0` when coverage is complete and no high-severity finding exists.
- Returns `1` for high-severity findings; `--report-only` explicitly suppresses
  this finding status for non-gating diagnostics.
- Returns `2` for missing, unreadable, malformed, oversized, escaped, special,
  truncated, or empty mandatory coverage.

## CLI flags

- `--path`: workspace root to scan (default `.`)
- `--json`: emit machine-readable JSON report
- `--sarif <file>`: emit SARIF file (v2.1.0)
- `--allowlist <file>`: newline-separated regex patterns (path- or message-scoped) to ignore
- `--ci` / `--fail-on-error`: explicit aliases for the fail-closed default
- `--report-only`: report findings without exit `1`; coverage still fails closed
- `--allow-empty`: explicitly permit an empty eligible file set
- `--max-files`, `--max-file-bytes`, `--max-total-bytes`, `--max-depth`: resource ceilings

The GitHub Action is a composite Bash/Node action. It passes caller inputs only
through environment variables, scans `.` by default, and fails on high findings
or incomplete coverage. GitHub-hosted Linux and macOS runners are supported;
Windows requires the runner's Git Bash environment.
