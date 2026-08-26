# ADR-007: terminal review packet and transaction seal

Status: accepted for S12-405 implementation
Owner: Sley maintainers
Date: 2026-08-25

## Decision

`sley change review` is the only S12-405 publication surface. It accepts the
original preview, approval request, grant, apply authorization, terminal apply
report, explicit repository, principal, and audience. A manual rollback report
is required only when a verified apply was later rolled back. An optional trace
must be a regular in-repository file whose every document validates as
`sley.trace.receipt.v0`.

Review validates the exact historical grant chain without treating later grant
expiry as evidence tampering. It still regenerates the approval request,
recomputes every self-digest, binds repository identity and HEAD, and checks the
exact authorization, recovery record, apply report, optional rollback report,
single-use replay state, and revocation state observed at apply. Historical
review does not reactivate expired authority or permit another write.

Only two outcomes are reviewable:

- `verified_apply`: the durable record is terminal and verified, the live tree
  exactly matches the candidate source and graph, and the preserved tree
  exactly matches the base.
- `verified_rollback`: the durable record is terminal and rolled back, the live
  tree exactly matches the base, and the preserved tree exactly matches the
  candidate.

Active, cancelled, failed, ambiguous, inconsistent, or digest-mismatched states
fail closed before packet publication.

## Artifact set

With `--confirm-review-packet`, Sley atomically publishes exactly four regular
files under `.sley/transactions/<transaction-id>/review/`:

- `packet.md`: bounded human-readable outcome, authority, validation, trace,
  and final-digest summary with no source content.
- `bundle.json`: strict `sley.change.review.v0` machine evidence.
- `evidence.zjx.json`: existing `sley.zjx.envelope.v0` graph and optional trace
  evidence.
- `seal.json`: strict `sley.change.transaction_seal.v0` content-addressed seal.

The directory is built and fsynced under a no-follow staging directory, then
renamed once into its final location. Existing review directories are never
replaced. Review writes no repository source, Git index, trace sidecar,
external-system, or provider state.

## Digest chain

The review bundle carries portable artifact locators and canonical JSON
digests for preview, request, grant, authorization, apply, optional rollback,
recovery, and ZJX evidence. It also carries the transaction, candidate,
request, grant, authorization, recovery, and optional rollback identity chain.

The transaction seal binds:

- final structural source and graph digests;
- the existing `sley.trace.seal.v0` seal and trace digests;
- apply and optional rollback report digests;
- review identity and canonical bundle digest;
- human packet byte digest;
- canonical ZJX evidence digest; and
- the final durable recovery event timestamp.

`sley.trace.seal.v0` is unchanged. The transaction seal references it instead
of redefining its semantics.

## Terminality and rollback

Publishing the review directory seals the transaction. Manual rollback refuses
an already sealed transaction, because allowing source reversal after terminal
evidence publication would make the seal false. Operators who need rollback
must do so before review, then review the verified rollback outcome.

## ZJX boundary

The evidence file honestly retains the existing `zjx-preview-json` format with
`compression: none`. S12-405 does not change frozen ZJX semantics or claim a
compressed archive. Broader archive integration remains separate work.

## Deferred surface

S12-405 adds no MCP write method, provider call, network dependency, deploy,
publication, or new source-mutation transition. CLI/MCP semantic parity remains
owned by S12-406.
