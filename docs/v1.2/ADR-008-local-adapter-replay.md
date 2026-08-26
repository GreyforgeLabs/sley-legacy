# ADR-008: Minimum local adapter and deterministic replay

Status: accepted for S12-501
Owner: Sley maintainers
Date: 2026-08-25

## Decision

S12-501 introduces one provider-neutral, local-only adapter contract through:

- `sley.adapter.manifest.v0`
- `sley.adapter.replay.v0`
- `sley.adapter.report.v0`
- `sley adapter replay --json [--record RECORD] MANIFEST`

`loom.adapter` owns lifecycle states, event phases, typed failure codes, seed
families, supported effects, the local authority mode, and replay policy. The
host validates those Sley-owned values, resolves only repository-confined
regular targets and records, and delegates execution to the existing seeded
`sley verify` runtime. It does not create a second evaluator.

## Authority boundary

The only accepted authority mode is `local_replay_only`. Repository, Git
index, trace, external, provider, deployment, and spend mutation flags must be
false. Ephemeral runtime state must be true. A manifest that requests live
execution or any provider/external/deploy/spend mutation fails with
`ADAPTER_LIVE_PROVIDER_DENIED` before adapter execution.

Capabilities bind one declared effect and operation to an exact seeded scope.
Missing effects fail through the existing runtime authority check. Missing
seed families and scope/seed mismatches fail before runtime dispatch. Seed
values stay in the private manifest. Text seed values are SHA-256 projected
before the verifier subprocess is created, so raw values do not enter child
process arguments; replay records expose only digests and counts.

## Replay and bounds

The manifest, adapter, runtime, target, capabilities, seeds, runtime output,
and record have canonical SHA-256 identities. A replay succeeds only when the
observed record equals the manifest pin and, when supplied, the separate
record artifact.

The host enforces input and output bytes, wall time, static source steps,
static call depth/cycle denial, and observed structural call count. The
deterministic cancellation fixture observes cancellation at a declared adapter
boundary. Preemptive persistent-request cancellation belongs to S12-502.

Single-file targets must be regular non-symlink files. Directory targets are
walked without following links and rejected if any nested file or directory is
a symlink, keeping static accounting and target digests on the same confined
source set.

## Isolation claim

S12-501 is deterministic seeded replay, not an isolated persistent worker.
Every report states `os_isolation_enforced: false`, `persistent_worker: false`,
and names S12-502 as the isolation owner. The existing
`sley-sandbox-runner` remains compatible and unchanged.

The committed example selects only SecretRead, Network, ModelCall, and Deploy
seeded effects. Additional local effect selections require their own focused
fixtures before they can be claimed. External provider adapters remain
deferred and are not required for Sley 1.2 GA.

## Validation

The focused adapter gate covers happy replay, exact record matching, redaction,
missing capability, scope denial, missing seed, replay mismatch, cancellation,
budget exhaustion, live-provider denial, manifest tamper, record tamper, schema
validation, and source/index no-mutation evidence. Change-aware routing selects
the adapter and contract gates.
