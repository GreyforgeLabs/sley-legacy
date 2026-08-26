# ADR-004: Immutable local candidate preview

Status: accepted for S12-402
Date: 2026-08-25
Owner: Sley maintainers

## Decision

Sley change transactions progress through `inspect -> plan -> preview`. Planning
and preview are separate strict contracts because operator rationale belongs in
the plan while candidate evidence must remain compiler-derived and
reproducible.

`sley change plan` accepts one valid inspection, one strict all-or-nothing
operation document, an explicit repository root, the requested outcome,
assumptions, and non-goals. It verifies repository identity, committed base,
source graph, source content, and expiry before emitting
`sley.change.plan.v0`. Compiler facts and operator rationale are separate
fields in the plan.

`sley change preview` accepts one valid plan and the same explicit repository
root. It revalidates the plan digest and transaction base, creates a disposable
copy outside the repository, executes only the operation-to-executor mappings
owned by `loom.transaction`, runs compiler check, lint, and structural graph
diff, then deletes the copy. The temporary path is excluded from candidate
identity and all report fields.

The preview emits the complete candidate source projection, stable unified
source diff, graph/type/call/effect/authority changes, diagnostics, focused
test results, validation state, review requirements, and a candidate digest.
Repository source, Git index, trace sidecars, external systems, and provider
state remain false mutation boundaries.

## Initial operation floor

S12-402 supports one ordered cross-module operation set:

- `RenameDeclaration`
- `UpdateCallSites`
- `AddTake` at position zero
- `UpdateCallArgs` at position zero

Unsupported operations, extra fields, missing targets, symlinks, path escape,
stale bases, expiry, digest mismatch, no-op operations, compiler failure, and
owned-path violations fail closed with typed diagnostics.

## Consequences

Candidate evidence is immutable and reproducible but has no write authority.
Graph-diff rename ambiguity and lint findings remain explicit review
requirements. S12-403 adds exact local grant issuance under ADR-005. Apply,
rollback, review packet sealing, and MCP transaction parity remain unavailable
until S12-404 through S12-406.
