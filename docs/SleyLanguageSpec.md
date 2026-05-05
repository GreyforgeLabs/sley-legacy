# Sley Language Specification

Status: v0 executable slice plus module task/type/effect namespace and trace tooling

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

Type lookup rules:

- builtin types such as `Int`, `Text`, `List`, `Map`, `Result`, and `Error`
  are always visible by simple name
- same-module declared types are visible by simple name
- imported types must be `export type`
- simple imported type references are valid only when one imported module
  exports that type name
- alias-qualified types use the import alias or default last module segment,
  such as `math.User`
- fully qualified types use the module path, such as `app.math.User`
- private imported types produce `PRIVATE_TYPE`
- ambiguous simple imported types produce `AMBIGUOUS_TYPE`

Custom effect lookup follows the same boundary:

- builtin effects such as `FileRead`, `Network`, and `Deploy` are always
  visible by simple name
- same-module declared effects are visible by simple name
- imported effects must be `export effect`
- simple imported custom-effect references must be unambiguous
- alias-qualified and fully qualified effects are allowed
- private imported effects produce `PRIVATE_EFFECT`
- ambiguous simple imported effects produce `AMBIGUOUS_EFFECT`

The checker normalizes resolved type/effect names to fully qualified semantic
identity before comparing task signatures, local annotations, record literals,
record fields, and called-task effects. This means `math.User` and
`app.math.User` resolve to the same type when they name the same exported
declaration.

## Graft Model

Agents should edit Sley by submitting structural grafts against bounded graph
shards. A graft is accepted only after parse, type, effect, authority, lifetime,
and provenance checks pass for the implemented v0 surface.

Current graft operations include adding/removing takes, replacing task bodies,
adding imports/effects/types/tasks, renaming declarations, updating call-sites,
inserting checked task-body statements, and replacing expressions by node id.
Unsupported graph operations return explicit diagnostics until implemented.

Implemented graph-edit payloads:

```json
{ "op": "UpdateCallSites", "target": "task:app.math.double", "payload": { "replacement": "math.twice" } }
{ "op": "UpdateCallSites", "target": "task:app.math.twice", "payload": { "from": "math.double", "replacement": "math.twice", "scope": "module:app.main" } }
{ "op": "InsertStatement", "target": "task:app.main.main", "payload": { "position": 1, "source": "set total = total + 1" } }
{ "op": "ReplaceExpression", "target": "block:task:app.main.main:stmt:0:expr:right", "payload": { "source": "41" } }
```

`UpdateCallSites` rewrites call expressions that either resolve to the target
task or match the optional raw `from` callee. `scope` can limit the rewrite to a
task or module. `InsertStatement` currently targets a task body. Each accepted
edit reparses the payload, rewrites the AST, refreshes expression source text,
and reruns the checker before returning formatted source.

Accepted grafts written with `sley graft --write` append receipt records to a
local `.sley/trace.jsonl` sidecar unless the caller passes an explicit trace
path. The trace sidecar is a local provenance store, not a final seal.

`sley graft --dry-run` is an explicit non-mutating preview mode. Plain
`sley graft` remains non-mutating by default; only `--write` changes source or
appends trace receipts. `--dry-run` and `--write` are mutually exclusive.

Graft input JSON is strict for the v0 contract. Unknown fields in transaction,
operation, or payload objects reject during deserialization instead of being
ignored. This prevents agents from believing unsupported intent metadata was
honored.

Current v0 graft outcome JSON has this root shape:

```json
{
  "schema": "sley.graft.outcome.v0",
  "status": "accepted",
  "source": "task main -> Int {\n  return 1\n}\n",
  "provenance": []
}
```

Rejected graft outcomes use the same schema and `status: "rejected"`, omit
`source`, clear `provenance`, and include diagnostics.

## Graph And Trace Tooling

The Loom exposes the program graph as inspectable JSON:

```bash
sley ast --json <target>
sley ast --json --node task:app.main.main <target>
sley graph --json <target>
sley graph --json --slice task:app.main.main <target>
sley trace --json <target>
```

AST roots carry `schema: "sley.ast.program.v0"`. Diagnostic reports carry
`schema: "sley.diagnostics.report.v0"`. Full symbol graphs carry
`schema: "sley.symbol_graph.v0"`, graph slices carry
`schema: "sley.symbol_graph.slice.v0"`, graft outcomes carry
`schema: "sley.graft.outcome.v0"`, and ZJX preview envelopes carry
`schema: "sley.zjx.envelope.v0"`.

The v0 JSON contracts are locked by small snapshots under
`fixtures/contracts/` and root-contract JSON Schema files under
`docs/schemas/`. The schema files currently pin top-level contract shape and
stable schema IDs; exhaustive nested expression and statement schemas remain a
later hardening step.

A graph slice is a bounded shard around a module, task, type, effect, or import.
Task slices include the selected task AST, visible module declarations,
outbound calls, and inbound calls from other tasks that resolve to the selected
task. This is the first stable agent-facing query surface for graft planning.
Call-site and expression grafts now consume node ids and task identities from
this shard.

Diagnostics include machine-readable repair hints for common checker failures.
Hints are intentionally small and structural: `kind` identifies the action,
`target` names the graph node when available, `effect` names required authority
when relevant, and `replacement` carries a compact source or type suggestion.

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

The current `sley zjx` command emits a preview JSON payload with
`compression=none`. It is a ZJX-ready semantic envelope for graph snapshots,
optional graph slices, and trace receipts; it is not yet a compressed `.zjx`
archive.

## Current Gaps

- host capabilities are checked statically but not backed by runtime gate values
- trace sidecars are not yet content-addressed seals
- external JSON Schema files pin v0 root contracts but do not yet exhaustively
  describe every nested AST and expression variant
- no `match`, agent declarations, spawn/cast/join, or compressed ZJX archive
  writer yet
- `MoveNode` and `DeleteNode` are still declared graft operations rather than
  implemented graph mutations
- project-aware graft writeback for multi-file module bundles is not yet
  implemented
