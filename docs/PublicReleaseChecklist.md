# Sley Public Release Checklist

Status: blocked pending operator metadata decisions.
Last checked: 2026-05-06.

The local executable v1 gate is:

```bash
make v1
```

The public release cut gate is:

```bash
make public-release-check
```

`make public-release-check` is expected to fail until the operator chooses the
public license and repository metadata.

## Operator Decisions

Before a public v1.0 cut, the operator must choose:

- repository license text or SPDX license identifier;
- public repository URL for `Cargo.toml`;
- whether `tree-sitter-sley` is published as public package metadata or kept
  private/unpublished;
- matching license metadata for `tree-sitter-sley/package.json`;
- matching license metadata for `tree-sitter-sley/tree-sitter.json`.

Do not guess these values in an agent session. Apply them only after explicit
operator approval.

## Current Blockers

`sley-conformance report --json --require-public-release-ready` currently
blocks on:

- missing `LICENSE`;
- missing `Cargo.toml` `package.license` or `package.license-file`;
- missing `Cargo.toml` `package.repository`;
- unresolved `tree-sitter-sley/package.json` license;
- unresolved `tree-sitter-sley/tree-sitter.json` license.

## Cut Procedure

After the operator supplies the metadata decisions:

1. Add the approved `LICENSE` file or approved license file path.
2. Update `Cargo.toml` with the approved `package.license` or
   `package.license-file`.
3. Update `Cargo.toml` with the approved `package.repository`.
4. Update `tree-sitter-sley/package.json` license metadata, and refresh
   `tree-sitter-sley/package-lock.json` if npm metadata changes require it.
5. Update `tree-sitter-sley/tree-sitter.json` license metadata.
6. Run `make v1`.
7. Run `make public-release-check`.
8. Review `CHANGELOG.md`, `README.md`, `docs/contracts.md`, and generated
   package artifacts before any public tag, push, crate publish, npm publish,
   release upload, or announcement.

Public posting, provider calls, live deployment, external mutation, crate/npm
publication, and release tagging remain explicit operator-approved actions.
