# Sley Language Specification

Status: v0 executable slice plus module task/type/effect namespace, runtime gates, Sley-level Result flow, typed host fallibility, seeded database host reads, and trace tooling

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
`forge` directly. Task inputs may also use `take gate`, `take veil`,
`take taint`, and `take view` qualifiers; only `take gate` currently has
runtime capability semantics. The AST already carries `BindingKind` so later
syntax can land without replacing the graph model.

Hard rules:

- no `var`
- no generic mutable local declaration
- no silent shared state
- no hidden authority in ordinary bindings

## Task Semantics

Tasks have names, explicit `take` inputs, return types, declared effects, and a
block of statements. Pure zero-take `main` can run in the current interpreter.
Effectful `main` can run only when the caller supplies matching runtime gates,
and `main` still cannot require ordinary non-gate takes.

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

## Result And Error Flow

`Result<T, E>` is the standard fallible return shape. `Ok(value)` and
`Err(error)` construct runtime result values. In a task returning `Result`, the
postfix `?` operator unwraps `Ok(value)` and propagates `Err(error)` as the
current task's return value.

```sley
task parse_score -> Result<Int, Error> {
  take raw: Text

  if raw == "bad" {
    return Err("bad score")
  }
  return Ok(41)
}

task main -> Result<Int, Error> {
  bind score = call parse_score("41")?
  return Ok(score + 1)
}
```

The checker enforces that `?` appears only inside tasks returning `Result`.
Runtime propagation is implemented for Sley-level `Ok`/`Err` values and for
fallible host adapters.

The standard runtime `Error` payload is a record:

```sley
{ code: Text, message: Text }
```

Recoverable host failures use this shape inside `Err(error)`. Missing runtime
capabilities and gate-scope denials remain diagnostics because they are
authority failures, not recoverable program values.

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

- builtin types such as `Int`, `Text`, `List`, `Map`, `Result`, `Error`,
  `Gate`, and `DbRow` are always visible by simple name
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

## Runtime Gate Semantics

Sley separates static authority from runtime authority. A task declares the
effects it may use with `uses EffectName`; the runtime then requires explicit
gate values before executing any task with effects.

```sley
task read -> Text uses FileRead {
  take gate fs: Gate<FileRead>
  take path: Text

  return fs.read_text(path)
}

task main -> Text uses FileRead {
  return call read("/tmp/sley/input.txt")
}
```

`take gate name: Gate<Effect>` binds a first-class gate value inside the task
but does not count as a normal call argument. The checker treats `call read(x)`
as valid in the example above because only `path` is an ordinary take. The gate
effect must also appear in the task's `uses` list; otherwise the checker emits
`GATE_EFFECT_UNDECLARED`.

The CLI grants runtime gates with repeated `--cap EFFECT[=ROOT]` flags:

```bash
sley run --cap FileRead=/tmp/sley program.sley
```

Without a matching gate, effectful execution fails with
`RUNTIME_CAPABILITY_REQUIRED` or `RUNTIME_GATE_REQUIRED`. `FileRead` and
`FileWrite` currently back `fs.read_text(path)`, `fs.write_text(path, text)`,
`fs.try_read_text(path)`, and `fs.try_write_text(path, text)`. Raw filesystem
calls return direct values and surface host I/O failures as diagnostics.
`try_` filesystem calls return `Result<T, Error>` and surface recoverable I/O
failures as `Err({ code, message })`. When a gate has a root, filesystem host
calls reject paths outside that root with `RUNTIME_CAPABILITY_SCOPE_DENIED`.

`DatabaseRead` currently backs deterministic seeded-table reads. The runtime
does not open a real database connection in v0; the host supplies JSON rows:

```bash
sley run --cap DatabaseRead --db-table users=examples/users.json examples/db_gate.sley
```

The seed file must be a JSON array of row objects. `db.query_one(sql, value)`,
`db.query(sql)`, `db.try_query_one(sql, value)`, and `db.try_query(sql)`
support simple `select * from table` queries and optional `where field = ?`
filters. `db.query_one` returns a `DbRow`; `db.query` returns `List<DbRow>`.
`db.try_query_one` returns `Result<DbRow, Error>`; `db.try_query` returns
`Result<List<DbRow>, Error>`. Row values expose typed accessors:

```sley
bind row = call db.query_one("select * from users where id = ?", id)
return row.text("name")
```

Missing table seeds produce `RUNTIME_DB_TABLE_NOT_FOUND`; empty `query_one`
results produce `RUNTIME_DB_ROW_NOT_FOUND`; unsupported query forms produce
`RUNTIME_DB_QUERY_UNSUPPORTED`. Raw database calls report those as runtime
diagnostics. `try_` database calls convert them into typed `Error` records.

## Graft Model

Agents should edit Sley by submitting structural grafts against bounded graph
shards. A graft is accepted only after parse, type, effect, authority, lifetime,
and provenance checks pass for the implemented v0 surface.

Current graft operations include adding/removing takes, replacing task bodies,
adding imports/effects/types/tasks, renaming declarations, updating call-sites,
inserting checked task-body statements, replacing expressions by node id, and
deleting checked graph nodes such as declarations, imports, takes, and
statements. `MoveNode` reorders statements within their existing block and
top-level imports, types, effects, or tasks within their declaration lists.
Unsupported graph movement returns explicit diagnostics until implemented.

Implemented graph-edit payloads:

```json
{ "op": "UpdateCallSites", "target": "task:app.math.double", "payload": { "replacement": "math.twice" } }
{ "op": "UpdateCallSites", "target": "task:app.math.twice", "payload": { "from": "math.double", "replacement": "math.twice", "scope": "module:app.main" } }
{ "op": "InsertStatement", "target": "task:app.main.main", "payload": { "position": 1, "source": "set total = total + 1" } }
{ "op": "ReplaceExpression", "target": "block:task:app.main.main:stmt:0:expr:right", "payload": { "source": "41" } }
{ "op": "DeleteNode", "target": "block:task:app.main.main:stmt:1" }
{ "op": "MoveNode", "target": "block:task:app.main.main:stmt:1", "payload": { "parent": "block:task:app.main.main", "position": 0 } }
{ "op": "MoveNode", "target": "task:app.main.helper", "payload": { "parent": "program.tasks", "position": 0 } }
```

`UpdateCallSites` rewrites call expressions that either resolve to the target
task or match the optional raw `from` callee. `scope` can limit the rewrite to a
task or module. `InsertStatement` currently targets a task body. `DeleteNode`
can remove declarations, imports, takes, and statements, but rejects expression
targets unless the agent uses `ReplaceExpression` instead. `MoveNode` currently
requires `payload.position`; `payload.parent` is optional but, when present,
must identify the current parent. Statement moves use a block parent such as
`block:task:app.main.main`; top-level declaration moves accept `program.imports`,
`program.types`, `program.effects`, or `program.tasks`. Expression moves, take
moves, and cross-parent moves reject with `GRAFT_MOVE_UNSUPPORTED`. Each
accepted edit reparses the payload when applicable, rewrites the AST, refreshes
expression source text, and reruns the checker before returning formatted
source.

Accepted grafts written with `sley graft --write` append receipt records to a
local `.sley/trace.jsonl` sidecar unless the caller passes an explicit trace
path. The trace sidecar is a local provenance store. `sley seal` turns the
current source, symbol graph, and trace receipt chain into a deterministic
content-addressed seal.

When the target is a project directory or `sley.toml`, `sley graft` loads the
manifest entry module and transitively imported modules before checking the
graft. Accepted project writeback projects the checked candidate AST back to the
existing module files that own changed imports, types, effects, or tasks.
Unchanged module files are left byte-for-byte untouched. Project writeback
rejects before mutation when a graft candidate would create a new module file or
import a module outside the loaded project, using
`PROJECT_WRITEBACK_UNKNOWN_MODULE` or `PROJECT_WRITEBACK_UNKNOWN_IMPORT`.

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
sley seal --json <target>
```

AST roots carry `schema: "sley.ast.program.v0"`. Diagnostic reports carry
`schema: "sley.diagnostics.report.v0"`. Full symbol graphs carry
`schema: "sley.symbol_graph.v0"`, graph slices carry
`schema: "sley.symbol_graph.slice.v0"`, graft outcomes carry
`schema: "sley.graft.outcome.v0"`, trace seals carry
`schema: "sley.trace.seal.v0"`, and ZJX preview envelopes carry
`schema: "sley.zjx.envelope.v0"`.

Current v0 trace seal JSON has this root shape:

```json
{
  "schema": "sley.trace.seal.v0",
  "target": "examples/hello.sley",
  "source_digest": "sha256:...",
  "graph_digest": "sha256:...",
  "trace_digest": "sha256:...",
  "seal_digest": "sha256:...",
  "module_count": 1,
  "task_count": 1,
  "receipt_count": 0
}
```

The v0 JSON contracts are locked by small snapshots under
`fixtures/contracts/` and JSON Schema files under `docs/schemas/`. The AST
program schema now recursively describes imports,
types, effects, tasks, takes, statements, expressions, type expressions, spans,
and provenance records. The other schema files currently pin their top-level
contract shape and stable schema IDs.

The compiler conformance corpus starts under `fixtures/corpus/`. Accepted
fixtures must parse, check, and formatter-round-trip. Rejected fixtures carry a
JSON sidecar listing the diagnostic ids that must remain stable.

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

- runtime gates currently back filesystem text reads/writes and deterministic
  seeded database reads only; database writes, network, shell, model, secret,
  deploy, and spending effects still need host adapters
- Sley-level `Result` values, `?` propagation, and typed filesystem/database
  host fallibility execute; other host domains still need `Result<T, Error>`
  adapters
- trace receipts can be sealed, but sidecar storage is not yet a compressed ZJX
  archive
- the AST JSON Schema covers nested AST and expression variants; the remaining
  JSON Schema files are still narrower v0 root contracts
- no `match`, agent declarations, spawn/cast/join, or compressed ZJX archive
  writer yet
- `MoveNode` supports checked in-parent statement reordering and top-level
  declaration ordering; cross-parent movement, take movement, and expression
  movement are still explicit rejections
- project graft writeback updates existing module files only; new module file
  creation, module rename, and cross-parent declaration movement remain later
  steps
