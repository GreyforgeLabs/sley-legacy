# Sley Public Release Checklist

Status: Sley 1.2 public release cut approved for `v1.2.1`.
Last checked: 2026-08-26.

Exact operator approval was received on 2026-08-26 to push the finished Sley
1.2 tree to the public branch, create and push tag `v1.2.1`, upload the verified
release packet to a GitHub Release, deploy the existing Sley WebForge surface,
and publish the public release artifact. This approval applies to this cut only.

The local executable v1 gate is:

```bash
make v1
```

The public release cut gate is:

```bash
make public-release-check
```

`make public-release-check` must pass before a public tag or GitHub release is
created. A passing gate establishes local technical readiness only. It does
not authorize a tag, push, upload, signing operation, publication, deployment,
or announcement.

## Current Evidence

The release packet records:

- strict self-hosting in `bin/sley self-hosting-status --json`;
- the public proof packet in `docs/SleyClaimEvidence.md` and
  `docs/SleyClaimManifest.json`;
- one supported Linux x86_64 archive with strict manifest, license,
  provenance, verification, and toolchain-doctor contracts;
- a SHA-256 checksum, unsigned provenance, SPDX 2.3 SBOM, and license
  inventory;
- explicit absence of ambient publication, tag, upload, signing, and deployment
  authority inside the artifact itself. The approval recorded above is the
  separate operator authority for this release cut.

The current conformance report records:

- `schema_count: 99`
- `contract_fixture_count: 187`
- `corpus_accepted_count: 24`
- `corpus_rejected_count: 48`
- `smoke_case_count: 5`
- `integration_test_count: 264`
- `declared_integration_test_count: 264`
- `test_count_matches_declared: true`
- `v1_gate_target_count: 38`

## Cut Procedure

1. Review `docs/SleyClaimEvidence.md`, `docs/SleyClaimManifest.json`,
   `docs/SleyPriorArtSourcePack.md`, and the public proof bundle against the
   current code, examples, fixtures, schemas, and official prior-art sources.
2. Run `make release-archive` from a clean committed tree.
3. Run `make release-security-check` and review every checksum, provenance,
   manifest, license, SBOM, scrub, unpacked-toolchain, and worker-client check.
4. Run `sley-conformance report --json` and confirm the regular gate passes.
5. Run `make v1`.
6. Run `make public-release-check`.
7. Review `CHANGELOG.md`, `README.md`, `llms.txt`, `docs/contracts.md`,
   `docs/v1.2/RELEASE_ARTIFACTS.md`, and the generated artifact set.
8. Confirm exact operator approval names the tag and publication targets before
   any irreversible public action. The approval record above satisfies this
   step for `v1.2.1`, the GitHub Release, and the existing WebForge Sley surface.

Crate/npm publication, package signing, provider calls, and unrelated runtime
deployment remain outside this approval.
