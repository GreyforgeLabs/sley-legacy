# ADR-009: Persistent bounded local worker

Status: accepted for S12-502
Owner: Sley maintainers
Date: 2026-08-25

## Decision

S12-502 introduces the stable `sley.worker.v1` JSONL protocol through:

- `sley.worker.request.v1`
- `sley.worker.response.v1`
- `sley.worker.event.v1`
- `sley.worker.session.v1`
- `sley worker start --json`

`loom.worker` owns the protocol identity, operations, lifecycle and request
states, failure classes, contract set, runtime identity, isolation class, and
cache policy. The persistent process is a bounded controller. Each invocation
runs the existing `sley adapter replay` path in a fresh child process, so the
worker does not implement a second parser, checker, evaluator, or authority
model.

## Bindings and authority

Every message binds a unique request ID and nonce, protocol and worker digest,
runtime digest, package and source digests or explicit absence, the exact
contract set, entry point, authority or explicit absence, and wall-clock,
step, call-depth, collection, output, and memory budgets. Handshake permits a
zero digest only as explicit absence before version negotiation. Later
messages require the exact worker and runtime identities.

Control messages carry no package, source, grant, or idempotency authority.
Adapter messages revalidate the repository-confined manifest and source
digests. Invoke requires `local_replay_only`, no external grant, and an
idempotency key. The underlying adapter continues to deny external providers,
repository, index, trace, deployment, and spend mutation.

## Isolation and lifecycle

The controller accepts at most one invocation at a time. Each invoke receives
a fresh `0700` temporary home, a clean environment, closed inherited file
descriptors, a new process group, Linux `prlimit` memory/CPU/file-descriptor
bounds, a new cancellation token, and a fresh output buffer. The controller
retains no loaded package, seed values, trace values, adapter handles, mutable
program state, or result cache between requests.

Manifest, optional replay record, and target source are copied into a private
request snapshot. The copied manifest and source digests are rechecked before
execution, and returned manifest/source identities are checked again before a
result is accepted. Concurrent changes to the original repository paths can
therefore produce only a typed `source_mismatch`, never execution under stale
request bindings.

Child stdout and stderr are drained concurrently under one combined byte
counter. The controller retains at most `max_output_bytes + 1` bytes, using the
last byte only as the overflow sentinel, and kills the child process group as
soon as the bound is exceeded.

The protocol truthfully reports `fresh_subprocess_per_invoke` process
isolation and `namespace_isolation_enforced: false`. User namespaces and
Bubblewrap containment are unavailable on the current host, so Sley does not
claim namespace, filesystem, or network sandboxing. The deterministic adapter
authority boundary remains the defense against external or persistent
mutation.

The controller supports handshake, load, capability inspection, invoke,
bounded progress, cancel, reset, drain, health, and graceful shutdown.
Cancellation terminates the active process group and never contaminates the
next request. A child signal death returns `worker_crashed` while the
controller remains healthy. No automatic retry occurs. Clients may consider a
failure retryable only when the response says so, and must never replay a
consequential request without idempotency protection.

## Caching and clients

S12-502 disables caches. This is slower but avoids stale digest and
cross-request state hazards while the stable protocol is established. Any
future cache must be immutable, digest-addressed, dependency-aware, and must
not bypass runtime, source, contract, authority, or budget validation.

S12-503 now supplies the Python and Node reference clients under ADR-010.
`sley test` and W5 validation reports remain S12-504 and S12-505 work.
External adapters, HTTP,
namespace sandboxing, automatic retry, parallel invocation, and provider
authority are not part of this slice.

## Validation

The focused worker gate runs one persistent session through handshake, load,
capabilities, successful deterministic replay, source mismatch, authority
denial, budget exhaustion, protocol mismatch, preemptive process-group
cancellation, fresh post-cancellation state, isolated child crash, health,
drain refusal, reset, and graceful shutdown. Every stream message validates
against its canonical schema, private seed values are absent, and the worker
remains usable after cancellation and crash. Dedicated regressions flood child
output beyond its declared limit and mutate source after preflight but before
snapshotting; both fail closed without unbounded buffering or stale execution.
