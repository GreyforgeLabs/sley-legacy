# ADR-010: Generated worker reference clients

Status: accepted for S12-503
Owner: Sley maintainers
Date: 2026-08-25

## Target state

S12-503 provides small Python and Node reference clients for the stable
`sley.worker.v1` JSONL stdio protocol. They are host integrations over the
reviewed S12-502 worker, not a second worker runtime or a generalized Sley SDK.

The affected owners are `clients/python`, `clients/node`, the deterministic
model generator, the focused client gate, change-aware validation routing, and
the W5 status evidence. The clients may start an explicitly selected Sley
command and may operate only through the nine canonical worker operations.
They do not open provider, network, deploy, spend, or repository-mutation
authority.

## Generated contract boundary

`clients/generate-worker-clients.py` reads these canonical schemas directly:

- `sley.worker.request.v1`
- `sley.worker.response.v1`
- `sley.worker.event.v1`
- `sley.worker.session.v1`

It generates Python `TypedDict` models, TypeScript read-only types, and the
Node runtime protocol constants. The generator derives root and nested object
fields, required versus optional fields, literals, enums, unions, arrays,
references, protocol identity, operation inventory, and contract versions
from the schemas. The focused gate runs the generator in drift-check mode.

Handwritten code owns only stdio process management, ordered JSONL framing,
request construction, response correlation, client-side stream bounds, and
ergonomic lifecycle methods. It does not duplicate failure interpretation,
authority decisions, digest verification, budget enforcement, adapter
behavior, or worker state transitions.

## Package surfaces

The Python package is `sley-worker-client`, imported as
`sley_worker_client`. It requires Python 3.12 or newer and has no runtime
dependencies. The Node package is `@sley/worker-client`, requires Node 20 or
newer, uses only Node built-ins, and remains private during the prerelease
work. S12-503 builds local packages only; registry publication remains outside
the package.

Both clients expose typed request, response, event, session, binding, budget,
payload, issue, cache, and isolation models plus:

- exact control and adapter request builders;
- handshake and digest negotiation;
- load, capabilities, invoke, cancel, reset, drain, health, and shutdown;
- ordered event and response retention under an explicit host memory bound;
- strict protocol, schema discriminator, UTF-8 JSONL, and event-sequence
  checks.

## Cancellation and retry

Cancellation is a separate canonical control request bound to an active
request ID. Client wait timeouts never become worker cancellation or timeout
results. The invoke response remains authoritative and is not suppressed by a
successful cancel response.

Neither client automatically retries any request. A response marked
`retryable: true` is information for an explicit host policy, not permission
for the client to replay it. Any future retry helper must remain bounded and
must not replay consequential effects without a fresh permitted request and
idempotency protection.

## Validation and recovery

The focused gate regenerates and compiles the Python models, syntax-checks the
Node runtime, builds a local Python wheel, installs it into a fresh virtual
environment, packs the Node package, installs it into a fresh offline npm
project, and runs both clients against the canonical local worker.

Each clean environment proves handshake, load, capabilities, successful
invoke, deterministic source rejection, preemptive cancellation, retryable
child-crash reporting without replay, health, and graceful shutdown. Worker
request counts prove that the clients did not retry. Private seeded values are
absent from retained streams.

Recovery is deletion of the two local client directories and their validation
wiring. The stable protocol and S12-502 worker remain unchanged.

## Exclusions

S12-503 does not add HTTP transport, daemon management, parallel invocation,
cache APIs, namespace claims, external adapters, provider authority, Rust or
Zig clients, public package publication, or automatic retry.
