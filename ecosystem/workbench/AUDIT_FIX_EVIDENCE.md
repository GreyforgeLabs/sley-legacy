# Audit Fix Evidence

Version: 0.1.1

Resolved findings: GF-AUD-017 and GF-AUD-040.

- Strict CLI defaults bind `127.0.0.1:4173`; non-loopback binding requires
  `--allow-remote` and an explicit canonical Origin.
- The static root is derived from `import.meta.url` and package version from
  `package.json`.
- Commands use JSON `POST`, exact Host/Origin checks, a per-process CSRF token,
  and read-only defaults. Mutation additionally requires `--allow-mutation`.
- Relative request targets must remain within the configured workspace both
  lexically and after `realpath`; absolute, traversal, prefix-sibling, and
  symlink-escape targets fail before spawn.
- Child processes use a timeout, combined output ceiling, AbortSignal, process
  group termination, deterministic reap, reduced environment, and no shell.
- Static assets return correct 404/405/500 responses, including stream errors.

Validation: `npm test`, `npm run check`, `npm run fmt`, and `git diff --check`.
