# Sley v0

Sley is an AI-native structural programming language. The Loom compiler reads
human-reviewable `.sley` source, exposes typed graph-shaped AST data, checks
task/effect/binding semantics, runs pure v0 tasks, and accepts verified grafts
instead of blind text edits.

Implemented now:

- `sley parse`
- `sley format`
- `sley check`
- `sley run`
- `sley ast`
- `sley graft`
- `sley.toml` project manifests for multi-file module graphs
- module, import, type, effect, and task declarations
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
  simple return mismatches, `?` result flow, host-effect authority, task call
  arity/types, called-task effect propagation, lexical locals, immutable
  binding protection, operator operand types, `if` condition/branch types, and
  typed record literal fields, list element types, map key/value types,
  indexing, control-flow conditions, `each` collections, and `set` mutation
  types
- runtime evaluation for zero-take pure `main`, literal values, pure task calls,
  lexical locals, `set`, operators, `if` expressions, statement-level
  `if`/`else`, `while`, `each`, `forge` blocks, list literals, map literals,
  indexing, `len`, record literals, and record field access
- project loading for a manifest entry module plus transitively imported
  `.sley` modules under the configured source root
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
`sley check`, `sley run`, and `sley ast` accept either a single `.sley`
file or a project directory containing `sley.toml`.

Known v0 limits:

- Expression parsing still falls back to raw nodes for unsupported syntax such
  as lambdas, pattern matching, and multi-statement expression blocks.
- Project imports are bundled into one v0 namespace. Cross-module visibility is
  intentionally simple, so duplicate task/type/effect names across imported
  modules are still rejected.
- Durable trace storage is not complete. Accepted graft provenance is returned
  by the graft command, but not written into a sidecar store.
- `run` only supports pure execution. Host calls and capability-backed runtime
  objects are still represented as raw values unless a future host boundary is
  supplied.
- `UpdateCallSites`, `InsertStatement`, `ReplaceExpression`, `MoveNode`, and
  `DeleteNode` are declared but return explicit unsupported-operation
  diagnostics.

The next logical phase is real module namespace semantics: qualified task
resolution, import aliases, exported names, and runtime/checker lookup keyed by
fully qualified task identity instead of the current bundled v0 namespace.
