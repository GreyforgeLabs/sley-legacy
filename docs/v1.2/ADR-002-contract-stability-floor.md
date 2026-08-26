# ADR-002: Sley 1.2 contract stability floor

Status: accepted for W3
Date: 2026-08-25

## Decision

Sley 1.2 promotes only four externally useful report roots to stable v1:
diagnostics, bounded symbol-graph slices, verification, and machine responses.
Their existing v0 producers remain unchanged. A producer may advertise v1 only
after it emits and validates that root; schema registration alone is not a
producer or cutover claim.

The machine-readable policy is `CONTRACT_STABILITY.json`. Unlisted schemas
resolve to `experimental`. The current SleyBench aggregate, strict protocol
replay, and retained mode-summary shape are `extensible` v0 evidence. W4
transaction authority and W5 worker/provider namespaces are reserved, not
invented by W3.

Stable v1 schemas reuse frozen v0 field definitions so v0 instances can be
promoted by changing only the schema discriminator. The compatibility gate
validates this in both directions and rejects missing required fields, unknown
fields, and discriminator substitution. Referenced v0 dependency schemas may
not drift incompatibly; any such drift fails the stable compatibility gate.

## Compatibility rules

- Stable v1 shapes and semantics are frozen. Field additions or removals, type
  changes, and semantic reinterpretation require a new schema identifier.
- Extensible v0 evidence may gain a successor, but strict parsing is not
  weakened in place and external stability is not promised.
- Experimental roots are deterministic in-tree contracts, not external
  compatibility promises.
- The strict protocol replay records observed bounded behavior. It does not
  claim preemptive process cancellation or provider execution authority.

## Consequences

The contract floor can be tested before transaction and worker semantics
exist. Existing v0 clients do not cut over implicitly, and future waves cannot
smuggle authority semantics into W3 evidence schemas.
