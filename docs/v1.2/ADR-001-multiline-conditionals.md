# ADR-001: multiline conditional policy

Status: accepted for Sley 1.2
Date: 2026-08-24
Owners: Sley maintainers

## Context

The K3 semantic audit found model-generated `if condition then value else
value` expressions and inline task parameters. It did not establish that Sley
needs another conditional grammar. Sley already has a compact value-producing
conditional and a multiline statement form with explicit returns.

The pre-W1 checker preserved unsupported spellings as `Raw` without rejecting
them. That made a discoverability problem look like a grammar gap.

## Decision

Sley 1.2 does not add a multiline conditional-expression grammar.

- Use `if condition { value } else { value }` when an expression produces a
  value on one source line.
- Use statement form with explicit `return` statements for multiline branches.
- Treat `then` spelling as `CONTROL_FLOW_EXPRESSION_BOUNDARY`.
- Treat executable control flow outside a task as
  `MODULE_CONTROL_FLOW_NOT_ALLOWED`.
- Keep `Raw` as a loss-preserving parser representation, but report executable
  unsupported `Raw` as `UNSUPPORTED_RAW_EXPRESSION`.

The decision can be reopened only with real-user friction evidence showing
that diagnostics, examples, formatting, and mechanical spec lookup are
insufficient. Benchmark convenience alone is not enough.

## Accepted forms

```sley
return if ready { 1 } else { 0 }
```

```sley
if ready {
  return 1
} else {
  return 0
}
```

## Consequences

- W1 remains a diagnostic correctness change, not a syntax expansion.
- The formatter and runtime need no new conditional behavior.
- W2 must make the accepted forms mechanically discoverable from the new
  diagnostic IDs.
- Bootstrap 0.2 remains pinned; this ADR does not authorize prompt mutation or
  a successor bootstrap replay.
