# ADR-013: bounded operational reference replay

Status: accepted for S12-601

## Decision

Sley 1.2 replays `siglum.numerology.reference-v1` as an offline,
non-authoritative operational workload through `sley reference-replay
siglum-numerology --siglum-root PATH`. The caller supplies the checkout root;
the CLI owns and cannot be redirected away from the repository pin set.

The retained Siglum TypeScript implementation and its frozen outputs remain
the independent oracle. Sley does not regenerate its own expected values. The
replay requires the exact Siglum commit, artifact hashes, corpus manifest, 20
shard hashes, candidate source digest, and retained parity-report identity in
`fixtures/siglum/numerology-reference-v1/pins.json`.

The execution boundary has two parts:

1. a persistent `sley machine` process receives three or more identical
   requests to distinguish cold startup from warm request handling;
2. the existing Siglum differential harness executes the complete frozen
   10,000-case corpus twice using fresh bounded shard processes.

Both paths use `sley.machine.invoke.v0`. The controller applies wall-clock,
address-space, output, open-file, evaluator-step, call-depth, collection, and
response limits. It records that process limits are enforced while OS network
namespace isolation is not. Child processes receive an allowlisted environment
that excludes provider credentials and unrelated secrets.

## Contracts

- `sley.operational.reference_pins.v1` is the stable external identity and
  oracle pin set.
- `sley.operational.reference_replay.v1` is the stable, disclosure-safe
  evidence root.
- Raw corpus rows, oracle results, user data, and credentials are not embedded
  in the Sley report.

The stable report requires two deterministic complete-corpus runs, 10,000
exact matches, zero mismatches, cold/warm observations, resource limits,
negative evidence, and an explicit production-promotion decision.

## Authority

The command has local replay authority only. It grants no repository or
product-runtime mutation, network, provider, deploy, spend, production shadow,
or promotion authority. The incumbent TypeScript implementation remains
authoritative.

## Failure behavior

Commit drift, tracked Siglum changes, artifact or shard hash drift, an invalid
oracle report, a candidate mismatch, nondeterministic output, a resource-limit
failure, or an incomplete two-run replay fails the command. Retained historical
evidence cannot substitute for a fresh successful execution.

The first two full S12-601 attempts established that 4 GiB and 8 GiB
address-space caps are below Node/V8's WebAssembly reservation requirement on
the reference host. A bounded startup sweep also failed below 32 GiB during
WebAssembly reservation or V8 heap commit and passed at 32 GiB. The controller
therefore retains an explicit 32 GiB per-process virtual-address ceiling and
separately samples process-tree RSS for both the persistent probe and full
replay. The ceiling is not an observed resident-memory claim.

## Consequences

The W6 evidence can support the narrow claim that the pinned Sley candidate
replays the frozen numerology reference workload correctly on the observed
local host. It cannot support a production, latency-SLO, fallback, rollback,
general-adoption, or provider claim.
