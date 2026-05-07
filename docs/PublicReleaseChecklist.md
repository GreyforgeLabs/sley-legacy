# Sley Public Release Checklist

Status: blocked pending operator metadata decisions.
Last checked: 2026-05-07.

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

Only the public branch is `main`; remote `public` must not be pushed.

Use local `private` for private development and publish only from `main`.

The current release tag target is expected to be `v1.0.1`; remove older public
tags during cut so one public tag remains per release line.
`sley-conformance report --json` includes `release.next_actions` entries that
map each blocker to the approval owner and file paths to update after approval.
The same command can render a review packet for the release decision:

```bash
sley-conformance report --json --markdown .sley/public-release-report.md
```

## Operator Decisions

Before a public v1.0 cut, the operator must choose:

- repository license text or SPDX license identifier;
- public repository URL for `Cargo.toml`;
- whether `tree-sitter-sley` is published as public package metadata or kept
  private/unpublished;
- matching license metadata for `tree-sitter-sley/package.json`;
- matching license metadata for `tree-sitter-sley/tree-sitter.json`.
- whether `editors/vscode-sley` remains a private local shim or receives
  public marketplace/package metadata in a later release lane.

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
6. Run `sley-conformance report --json --markdown .sley/public-release-report.md`
   and review the remaining `release.next_actions`.
7. Run `make v1`.
8. Run `make public-release-check`.
9. Review `CHANGELOG.md`, `README.md`, `docs/contracts.md`, and generated
   package artifacts before any public tag, push, crate publish, npm publish,
   release upload, or announcement.
10. Remove old public release tag(s) if replacing a prior tag:

```bash
git tag -d v1.0.0
git push origin :refs/tags/v1.0.0
```

11. Tag the release from `main` and push:

```bash
git tag -a v1.0.1 -m "Release v1.0.1"
git push origin v1.0.1
```

12. Push the updated public branch:

```bash
git push origin main
```

Public posting, provider calls, live deployment, external mutation, crate/npm
publication, and release tagging remain explicit operator-approved actions.
