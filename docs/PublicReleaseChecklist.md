# Sley Public Release Checklist

Status: ready for the operator-approved v1.1 GitHub release.
Last checked: 2026-08-22.

The local executable v1 gate is:

```bash
make v1
```

The public release cut gate is:

```bash
make public-release-check
```

`make public-release-check` must pass before a public tag or GitHub release is
created.

## Current Evidence

The release packet records:

- strict self-hosting in `bin/sley self-hosting-status --json`;
- the public proof packet in `docs/SleyClaimEvidence.md` and
  `docs/SleyClaimManifest.json`;
- explicit operator approval for the v1.1 GitHub release on 2026-08-22.

The current conformance report records:

- `schema_count: 42`
- `contract_fixture_count: 128`
- `corpus_accepted_count: 23`
- `corpus_rejected_count: 43`
- `smoke_case_count: 5`
- `integration_test_count: 199`
- `declared_integration_test_count: 199`
- `test_count_matches_declared: true`
- `v1_gate_target_count: 23`

## Cut Procedure

1. Review `docs/SleyClaimEvidence.md`, `docs/SleyClaimManifest.json`,
   `docs/SleyPriorArtSourcePack.md`, and the public proof bundle against the
   current code, examples, fixtures, schemas, and official prior-art sources.
2. Run `sley-conformance report --json` and confirm the regular gate passes.
3. Run `make v1`.
4. Run `make public-release-check`.
5. Review `CHANGELOG.md`, `README.md`, `llms.txt`, `docs/contracts.md`, and
   generated package artifacts before any public tag, push, crate publish, npm
   publish, release upload, announcement, or external issue campaign.

Crate/npm publication, public announcements, provider calls, and live
deployment are outside this GitHub release approval.
