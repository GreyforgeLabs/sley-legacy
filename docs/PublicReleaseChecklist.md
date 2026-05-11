# Sley Public Release Checklist

Status: blocked pending strict parity, proof bundle, and operator approval.
Last checked: 2026-05-11.

The local executable v1 gate is:

```bash
make v1
```

The public release cut gate is:

```bash
make public-release-check
```

`make public-release-check` is expected to fail until the operator chooses the
strict Sley-written semantic parity target, approves the public proof bundle,
and authorizes the release. Do not downgrade that blocker in an agent session.

## Current Blockers

`sley-conformance report --json --require-public-release-ready` currently
blocks on:

- `PUBLIC_RELEASE_BLOCKED`: public release requires operator approval, strict
  Sley-written parity, and a published proof bundle.

The regular conformance report currently passes and records:

- `schema_count: 39`
- `contract_fixture_count: 125`
- `corpus_accepted_count: 23`
- `corpus_rejected_count: 43`
- `integration_test_count: 195`
- `declared_integration_test_count: 195`
- `test_count_matches_declared: true`

## Cut Procedure

After strict parity, proof bundle, and operator approval:

1. Review `docs/SleyClaimEvidence.md` and the public proof bundle against the
   current code, examples, fixtures, and schemas.
2. Run `sley-conformance report --json` and confirm the regular gate passes.
3. Run `make v1`.
4. Run `make public-release-check`.
5. Review `CHANGELOG.md`, `README.md`, `llms.txt`, `docs/contracts.md`, and
   generated package artifacts before any public tag, push, crate publish, npm
   publish, release upload, announcement, or external issue campaign.

Public posting, provider calls, live deployment, external mutation, crate/npm
publication, and release tagging remain explicit operator-approved actions.
