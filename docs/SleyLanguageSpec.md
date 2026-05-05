# Sley Language Specification

Status: v0 executable slice plus module task namespace semantics

Sley is a human-readable, agent-writable structural language. The canonical
program model is a typed graph. `.sley` source is the stable review projection,
and ZJX is the intended compact transport/cache envelope for graph shards,
grafts, traces, receipts, and repeated compiler-agent snapshots.

## Naming

- `Sley`: the language
- `Loom`: the compiler, scheduler, and runtime engine
- `task`: arena-backed executable unit
- `take`: task input binding
- `bind`: immutable local binding
- `state`: mutable local lifecycle state
- `tally`: reducer/accumulator binding
- `slot`: structural field
- `forge`: temporary isolated arena block
- `graft`: verified structural edit
- `trace`: accepted provenance chain
- `seal`: immutable content-addressed artifact
- `ZJX`: native compressed structural transport/cache envelope

## Source

```sley
module app.compute

task classify -> Text {
  take score: Int

  return if score >= 90 { "excellent" } else { "steady" }
}

task main -> Text {
  bind score = 40 + 55
  return call classify(score)
}
```

The surface syntax is intentionally not C-like. Task inputs live as explicit
`take` nodes in task bodies. Local names use binding kinds rather than generic
variable declarations. Mutable updates require mutable binding kinds such as
`state` or `tally`.

## Binding Ontology

The canonical binding vocabulary is:

```text
take bind state cell knot slot gate lease veil dial flag memo cache derive flow
port tally hole draft taint witness seal anchor view cursor
```

The v0 executable syntax supports `take`, `bind`, `state`, `tally`, `slot`, and
`forge` directly. The AST already carries `BindingKind` so later syntax can land
without replacing the graph model.

Hard rules:

- no `var`
- no generic mutable local declaration
- no silent shared state
- no hidden authority in ordinary bindings

## Task Semantics

Tasks have names, explicit `take` inputs, return types, declared effects, and a
block of statements. Pure zero-take `main` can run in the current interpreter.

```sley
task sum -> Int {
  take values: List<Int>

  state index = 0
  tally total = 0
  while index < len(values) {
    set total = total + values[index]
    set index = index + 1
  }
  return total
}
```

`set` is only valid for mutable binding kinds. `bind` is immutable.

## Module Semantics

Sley modules declare their module path and may import other modules.

```sley
module app.main

import app.math as math

task main -> Int {
  return call math.double(21)
}
```

Imported module declarations are private unless exported:

```sley
module app.math

export task double -> Int {
  take value: Int

  return value * 2
}
```

Task lookup rules:

- same-module tasks are visible by simple name
- imported tasks must be `export task`
- simple imported task calls are valid only when one imported module exports
  that task name
- alias-qualified calls use the import alias or default last module segment,
  such as `math.double`
- fully qualified calls use the module path, such as `app.math.double`
- private imported calls produce `PRIVATE_TASK`
- ambiguous simple imported calls produce `AMBIGUOUS_TASK`

`export type` and `export effect` are parsed, formatted, and represented in the
graph. Their cross-module semantic identity is intentionally behind task
identity and remains a near-term compiler milestone.

## Graft Model

Agents should edit Sley by submitting structural grafts against bounded graph
shards. A graft is accepted only after parse, type, effect, authority, lifetime,
and provenance checks pass for the implemented v0 surface.

Current graft operations include adding/removing takes, replacing task bodies,
adding imports/effects/types/tasks, and renaming declarations. Unsupported graph
operations return explicit diagnostics until implemented.

## ZJX Boundary

Sley semantic identity is the canonical typed graph bytes. ZJX is the default
wire/cache/storage envelope for graph movement, not the semantic identity until
both the graph encoding and ZJX canonical encoding are frozen.

Correct model:

```text
canonical_graph_bytes -> semantic hash
canonical_graph_bytes -> ZJX envelope
```

Initial ZJX payloads should carry bounded graph shards, binding tables, symbol
tables, diagnostics, graft bundles, graft receipts, traces, and repeated module
snapshots.

## Current Gaps

- type and custom-effect namespace resolution is not yet as strict as task
  namespace resolution
- durable trace sidecars are not written yet
- host capabilities are checked statically but not backed by runtime gate values
- no `match`, agent declarations, spawn/cast/join, or ZJX envelope command yet
