# ADR-003: Local transaction state and read-only inspection

Status: accepted for the first W4 slice
Date: 2026-08-25

## Decision

The Loom transaction core owns the Sley 1.2 lifecycle vocabulary and starts
with an explicit, read-only `working_tree` base mode. The first exposed command
is:

```bash
sley change inspect --json \
  --goal "<goal>" \
  --actor "<principal>" \
  --nonce "<nonce>" \
  --created-at "<RFC3339 UTC>" \
  --expires-at "<RFC3339 UTC>" \
  <target>
```

Inspection advances only from `opened` to `inspected`. It binds the report to
one Git repository with a committed HEAD, a structural working-tree source digest, graph and
trace digests, compiler and contract versions, actor, goal, nonce, creation
time, and expiry. It serializes bounded structural context without creating a
candidate or grant.

The complete success vocabulary is `opened`, `inspected`, `planned`,
`previewed`, `awaiting_approval`, `approved`, `applying`, `applied`,
`verifying`, and `sealed`. Failure vocabulary is `rejected`, `stale`, `denied`,
`apply_failed`, `verification_failed`, `rolled_back`, `cancelled`, and
`expired`. Later slices must reject transitions not admitted by the Loom-owned
transition model.

## Safety boundary

`change inspect` cannot mutate repository source, the Git index, trace
sidecars, external systems, or provider state. Symlink targets, targets outside
a Git repository, trace inputs outside the repository, malformed bindings, and
repositories without a committed base, missing or invalid trace inputs,
unknown options, and malformed bindings fail with typed diagnostics.

No `plan`, `preview`, `approve`, `apply`, `verify`, `review`, `rollback`, or MCP
write claim is made by this slice. Real writes remain blocked until immutable
candidate identity, exact grants, stale-base rejection, atomic multi-file
writeback, verified rollback, and review evidence all exist through the same
semantic core.

## Contract status

`sley.transaction.inspect.v0` is strict and registered as extensible. It is not
promoted to stable v1 because the later candidate, grant, apply, rollback, and
seal contracts have not been cut. Activating this root removes the broad
`sley.transaction.*` reservation; unimplemented transaction identifiers remain
experimental by default and must not be fabricated.

## Validation

The `transaction-contracts` gate validates the live report and representative
fixture, hashes sample source and the Git index before and after inspection,
and exercises missing bindings, unknown options, invalid nonces, invalid time
order, symlinks, non-Git targets, out-of-repository traces, and unsupported
apply. Contract, compatibility, change-classifier, and self-hosted syntax gates
remain required around the slice.
