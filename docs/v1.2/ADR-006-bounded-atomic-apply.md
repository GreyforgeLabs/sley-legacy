# ADR-006: bounded atomic apply and verified rollback

Status: accepted for S12-404 implementation
Owner: Sley maintainers
Date: 2026-08-25

## Decision

Sley atomic apply v0 accepts only a Linux, same-filesystem transaction whose
authorized source paths have one non-repository common root. The sole source
commit point is `renameat2(RENAME_EXCHANGE)` between that live common root and
the corresponding root in a durable candidate tree under `.sley`.

The concurrency boundary is cooperative local repository ownership, enforced
by one Sley apply lock. It is not a security boundary against a hostile process
running as the repository owner. The syscall adapter still resolves parent
directories from the repository file descriptor with `O_NOFOLLOW`, pins both
leaf identities, checks type and device immediately before exchange, and
verifies the expected inode swap immediately afterward.

The compiler owns lifecycle, authority, replay, revocation, verification,
metadata, mutation, and recovery-report semantics. The host owns only bounded
filesystem inspection, no-follow internal state writes, fsync, cooperative
locking, and the exchange syscall.

Apply requires the original preview, approval request, grant, a separately
derived `sley.change.apply_authorization.v0` artifact, explicit matching
principal and audience, and an active exact local revocation record. It
regenerates the request and grant chain, rechecks the base source and graph,
consumes replay once, records durable recovery phases, performs one exchange,
then requires candidate source, graph, compiler check, and compiler lint to
pass. Required verification failure exchanges the preserved preimage back and
verifies the exact base digests.

## Rejected topologies

Apply fails closed for repository-root exchanges, multiple common roots,
cross-filesystem candidates, symlinks, special files, submodules, extended
attributes, staged changes inside the common root, and unsupported platforms
observed at preflight and the pinned commit boundary.
It does not mutate the Git index, trace sidecars, external systems, or provider
state.

## Recovery boundary

The recovery record is fsynced before the commit point and updated through
prepared, commit, applied, verification, and rollback phases. Verified manual
rollback is implemented. `sley change recover` validates the original exact
grant chain, apply authorization, recovery self-digest, replay marker, current
revocation state for a resumed forward commit, and live/preserved tree
placement. It classifies placement from exact source and graph digests instead
of trusting the last journal phase, rechecks topology before a resumed forward
exchange, resumes verification, and completes either a verified apply or a
verified rollback. A pre-commit interruption with no consumed replay marker is
closed as cancelled without source mutation.

The focused crash matrix terminates apply immediately before exchange,
immediately after exchange, and immediately before automatic rollback. It also
proves fail-closed replay-marker tamper handling and topology drift rejection.
