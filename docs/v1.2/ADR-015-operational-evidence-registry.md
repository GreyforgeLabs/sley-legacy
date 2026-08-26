# ADR-015: Fail-closed operational evidence registry

Status: accepted for S12-603 infrastructure

Owner: Sley maintainers

Date: 2026-08-26

## Context

W6 has two distinct evidence lanes. S12-601 retains a real independent-oracle
reference replay, while S12-602 defines an approval-gated controlled agent
comparison. The release wave cannot close from either artifact alone. A final
decision must keep both workloads visible, preserve losses and limitations,
and grant no product or publication authority.

A hand-maintained ledger is insufficient for that boundary. It can drift from
the evidence digests, accept the synthetic comparison fixture, omit negative
results, or turn a one-trial observation into a general claim.

## Decision

`loom.operational` owns the registry schema identity, release identity,
authority mode, exact two entry identities, and fixed decision rationale.
`sley operational-evidence assemble` executes those descriptors through a
read-only host boundary.

The assembler accepts only:

- a repository-confined `sley.operational.reference_replay.v1` packet whose
  independent oracle, two-run exact parity, ancestry, authority boundary, and
  production deferral remain intact;
- a repository-confined, non-synthetic
  `sley.operational.agent_workflow_comparison.v1` packet whose commit is an
  ancestor of the registry source, whose public case and K1/K3 run digests
  match the current fixtures, and whose private controller digest matches the
  explicitly supplied controller;
- the private approval record, both aggregate reports, and both evidence
  bundles needed to re-run the scrubbed comparison assembler and reproduce the
  retained comparison exactly;
- the exact private execution-provenance record that binds those inputs to an
  asserted observed provider execution and an independent evidence review;
- one retained trial with the equal-control declaration, exact K1/K3 run
  identities, scrubbed disclosure state, and mandatory single-trial and
  partial-limit limitations.

The output contains exactly two named entries. Each entry records workload
class, authority status, artifact path and digest, the bounded result,
promotion deferral, known limitations, a reproducibility command, and
disclosure state. The combined decision records W6 evidence completion while
keeping production promotion, general claims, deployment, publication, and
runtime mutation false.

The public registry schema is extensible. Its synthetic contract fixture is
marked `SYNTHETIC_CONTRACT_FIXTURE` and cannot satisfy the assembler.
Removing that marker is also insufficient because the private evidence must
reassemble to the exact retained comparison.

The registry identity records digests for the registry assembler, comparison
assembler, complete schema set, registry schema, and Sley-owned operational
vocabulary. `registry_source_commit` therefore cannot silently represent dirty
controlling files as committed state; the exact source bytes used for assembly
remain separately bound.

## Consequences

- The required real S12-602 comparison and final registry now exist. Their
  exact private-source reassembly and independent review closed W6; this ADR
  did not and does not bypass provider approval.
- The reviewed final registry unblocked W7 while retaining promotion,
  production, deployment, and publication deferral.
- The assembler writes only JSON to standard output. It does not call a
  provider, mutate the repository, deploy, publish, or grant promotion
  authority.
- Provider origin is an exact operator/independent-review assertion, not a
  cryptographic provider signature. The registry retains that limitation and
  cannot elevate it into a general or production claim.
- The final registry may record a structural advantage, equality,
  disadvantage, or two-arm failure, but every outcome remains visible and
  non-general.
