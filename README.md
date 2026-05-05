# Sley v0

Sley is an agent-native structural programming language. The Loom compiler reads
human-reviewable `.sley` source, exposes typed graph-shaped AST data, checks
task/effect/binding semantics, runs pure and explicitly gated v0 tasks, and
accepts verified grafts instead of blind text edits.

Implemented now:

- `sley parse`
- `sley format`
- `sley check`
- `sley run`
- `sley ast`
- `sley graph`
- `sley trace`
- `sley seal`
- `sley zjx`
- `sley graft`
- explicit non-mutating `sley graft --dry-run`; plain graft preview remains
  non-mutating unless `--write` is supplied
- `sley.toml` project manifests for multi-file module graphs
- module, import, type, effect, and task declarations
- import aliases with `import app.math as math`
- exported declarations with `export task`, `export type`, and `export effect`
- `task` declarations with explicit `take` inputs in the task body
- `take gate name: Gate<Effect>` runtime capability inputs that do not count as
  ordinary call arguments
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
  indexing, `len`, record literals, record field access, explicit
  `Ok(value)`/`Err(error)` result values, and `?` propagation for Sley-level
  results
- runtime gates with `sley run --cap EFFECT[=ROOT]`; effectful tasks reject
  without matching gates, `Gate<Effect>` takes are injected at runtime, and
  `fs.read_text`/`fs.write_text` are backed by root-scoped file capabilities
- deterministic database host seeding with `sley run --cap DatabaseRead
  --db-table TABLE=rows.json`; `db.query_one` and `db.query` read seeded JSON
  rows, and `DbRow` values expose `row.text`, `row.int`, `row.float`,
  `row.bool`, and `row.get`
- project loading for a manifest entry module plus transitively imported
  `.sley` modules under the configured source root
- checker and runtime task lookup by entry module, local module, full module
  path, and import alias
- checker type and effect lookup by local module, imported exported name, full
  module path, and import alias
- JSON AST output with `schema: "sley.ast.program.v0"`
- JSON module symbol graph output for module imports and exported declarations,
  plus bounded graph slices with `sley graph --slice <node-id>`; symbol graph
  and graph-slice JSON carry v0 schema IDs
- JSONL trace sidecars for accepted graft receipts when `sley graft --write`
  applies a change, plus content-addressed trace seals with `sley seal`
- project-aware `sley graft --write <project>` source-file writeback for
  accepted edits that resolve to existing modules in a multi-file project
- a first ZJX-ready JSON envelope command for graph snapshots, optional graph
  slices, and trace receipts
- structural graft operations for adding/removing takes, replacing task bodies,
  adding imports/effects/types/tasks, renaming declarations, updating
  call-sites, inserting checked task-body statements, replacing nested
  expressions by node id, deleting checked graph nodes such as declarations,
  imports, takes, and statements, and moving checked statements or top-level
  declaration order within their current parent
- strict graft input JSON for the v0 operation shapes; unknown graft fields are
  rejected instead of silently ignored
- versioned JSON report roots for diagnostics and graft outcomes
- repair hints on common checker diagnostics, including unknown identifiers,
  unknown tasks, type mismatches, return mismatches, call argument mismatches,
  condition mismatches, effect authority, and private or ambiguous names
- locked JSON contract snapshots under `fixtures/contracts/`
- external v0 JSON Schema files under `docs/schemas/`
- accepted/rejected synthetic corpus fixtures under `fixtures/corpus/`
- compact agent onboarding pack in `llms.txt`

Project form:

```toml
[project]
name = "module-demo"
root = "src"
entry = "app.main"
```

Module `app.main` resolves to `src/app/main.sley`. `sley parse`,
`sley check`, `sley run`, `sley ast`, `sley graph`, `sley seal`, `sley zjx`,
and `sley graft` accept either a single `.sley` file or a project directory
containing `sley.toml`. Project graft writeback projects the checked candidate
back to existing owning module files and leaves unchanged module files alone.

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

Seeded database runtime:

```bash
sley run --json --cap DatabaseRead --db-table users=examples/users.json examples/db_gate.sley
```

`db.query_one` returns one `DbRow`, `db.query` returns `List<DbRow>`, and row
accessors such as `row.text("name")` read typed fields from the seeded JSON
rows. This is a deterministic v0 host adapter, not a real database connection.

Result flow:

```sley
task main -> Result<Int, Error> {
  bind value = Ok(41)?
  return Ok(value + 1)
}
```

`Ok(value)` and `Err(error)` are real runtime values. `expr?` unwraps `Ok` and
propagates `Err` as the current task's result. Host adapter failures still
surface as runtime diagnostics in v0; mapping host failures into typed
`Error` values is the next bridge.

Known v0 limits:

- Expression parsing still falls back to raw nodes for unsupported syntax such
  as lambdas, pattern matching, and multi-statement expression blocks.
- Trace storage is still a local JSONL sidecar. `sley seal` now produces a
  content-addressed seal over the source, graph, and trace receipts, but the
  compressed `.zjx` archive remains a later integration step.
- `sley zjx` emits the first Sley ZJX envelope payload as JSON with
  `compression=none`; the binary compressed archive handoff remains a later
  integration step.
- Runtime host support is intentionally narrow: `FileRead`/`FileWrite` have
  root-scoped filesystem handlers, and `DatabaseRead` has a deterministic
  seeded-table adapter. Network, shell, model, secret, deploy, database write,
  and spending effects still need dedicated host adapters.
- Sley-level `Result` values and `?` propagation execute now, but host adapter
  failures still return runtime diagnostics instead of typed `Error` values.
- Project graft writeback supports existing module files. Grafts that would
  create unknown module files or import modules outside the loaded project
  reject before any source or trace mutation.
- The AST JSON Schema now covers declarations, statements, expressions, type
  expressions, spans, and provenance recursively. Other external JSON Schema
  files remain narrower v0 root-contract schemas.
- `MoveNode` currently reorders statements within their existing block and
  top-level imports, types, effects, or tasks within their declaration lists.
  Cross-parent movement, take movement, and expression movement still reject
  explicitly.

The next logical phase is the host-fallibility bridge: define the standard
runtime `Error` payload, let filesystem and database adapters return typed
`Result<T, Error>` values where appropriate, and then add the next
capability-backed adapters on top of that error flow.
