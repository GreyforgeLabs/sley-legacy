# Sley v0

Sley is an agent-native structural programming language. The Loom compiler reads
human-reviewable `.sley` source, exposes typed graph-shaped AST data, checks
task/effect/binding semantics, runs pure v0 tasks, and accepts verified grafts
instead of blind text edits.

Implemented now:

- `sley parse`
- `sley format`
- `sley check`
- `sley run`
- `sley ast`
- `sley graph`
- `sley trace`
- `sley zjx`
- `sley graft`
- `sley.toml` project manifests for multi-file module graphs
- module, import, type, effect, and task declarations
- import aliases with `import app.math as math`
- exported declarations with `export task`, `export type`, and `export effect`
- `task` declarations with explicit `take` inputs in the task body
- canonical binding forms for the first executable slice: `bind`, `state`,
  `tally`, and `forge`, plus the wider binding-kind enum for the Sley ontology
- `slot` fields in record type declarations
- structured expressions for literals, identifiers, unary/binary operators,
  `if` expressions, `call`, field access, list literals, indexing, record
  literals, map literals, and `?`
- statement-level `if`/`else`, `while`, `each`, and `set` for explicit mutable
  binding kinds
- builtin `len` for lists, maps, and text
- static checks for duplicate declarations, unknown types, unknown effects,
  simple return mismatches, `?` result flow, host-effect authority, module
  task/type/effect visibility, ambiguous imported task/type/effect references,
  task call arity/types, called-task effect propagation, lexical locals,
  immutable binding protection, operator operand types, `if`
  condition/branch types, and typed record literal fields, list element types,
  map key/value types, indexing, control-flow conditions, `each` collections,
  and `set` mutation types
- runtime evaluation for zero-take pure `main`, literal values, pure task calls,
  lexical locals, `set`, operators, `if` expressions, statement-level
  `if`/`else`, `while`, `each`, `forge` blocks, list literals, map literals,
  indexing, `len`, record literals, and record field access
- project loading for a manifest entry module plus transitively imported
  `.sley` modules under the configured source root
- checker and runtime task lookup by entry module, local module, full module
  path, and import alias
- checker type and effect lookup by local module, imported exported name, full
  module path, and import alias
- JSON module symbol graph output for module imports and exported declarations,
  plus bounded graph slices with `sley graph --slice <node-id>`
- JSONL trace sidecars for accepted graft receipts when `sley graft --write`
  applies a change
- a first ZJX-ready JSON envelope command for graph snapshots, optional graph
  slices, and trace receipts
- structural graft operations for adding/removing takes, replacing task bodies,
  adding imports/effects/types/tasks, and renaming declarations

Project form:

```toml
[project]
name = "module-demo"
root = "src"
entry = "app.main"
```

Module `app.main` resolves to `src/app/main.sley`. `sley parse`,
`sley check`, `sley run`, `sley ast`, `sley graph`, and `sley zjx` accept either a
single `.sley` file or a project directory containing `sley.toml`.

Module visibility:

```sley
module app.main

import app.math as math

task main -> Int {
  return call math.double(21)
}
```

```sley
module app.math

export task double -> Int {
  take value: Int

  return value * 2
}
```

Tasks in the same module are visible by simple name. Imported tasks must be
exported. Calls may use a simple imported name when exactly one import exports
that task, an import alias such as `math.double`, or a full module path such as
`app.math.double`.

Types and custom effects follow the same namespace boundary. Same-module
type/effect names are visible by simple name. Imported type/effect declarations
must use `export type` or `export effect`. Imported references may use an
unambiguous simple name, an import alias such as `math.User` or `math.Read`, or
the full module path such as `app.math.User`.

Known v0 limits:

- Expression parsing still falls back to raw nodes for unsupported syntax such
  as lambdas, pattern matching, and multi-statement expression blocks.
- Trace storage is a local JSONL sidecar, not yet a content-addressed seal or a
  compressed `.zjx` archive.
- `sley zjx` emits the first Sley ZJX envelope payload as JSON with
  `compression=none`; the binary compressed archive handoff remains a later
  integration step.
- `run` only supports pure execution. Host calls and capability-backed runtime
  objects are still represented as raw values unless a future host boundary is
  supplied.
- `UpdateCallSites`, `InsertStatement`, `ReplaceExpression`, `MoveNode`, and
  `DeleteNode` are declared but return explicit unsupported-operation
  diagnostics.

The next logical phase is graph-slice graft execution: implement
`UpdateCallSites`, `InsertStatement`, and `ReplaceExpression` against bounded
symbol graph slices, while freezing the JSON contracts those operations depend
on.
