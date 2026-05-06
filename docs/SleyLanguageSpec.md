# Sley Language Specification

Status: v0 executable slice plus module task/type/effect namespace, runtime gates, Sley-level Result flow, typed host fallibility, seeded database host reads/writes, seeded secret values, seeded deploy stage results, seeded spend authorizations, seeded network host text, seeded shell host output, seeded model completions, trace tooling, checked lint tooling, and checked edit-plan tooling

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
and `main` still cannot require ordinary non-gate takes. Non-`Unit` tasks must
return on every statically guaranteed path; the checker accepts a direct
`return` or an `if` statement whose then and else blocks both guarantee a
return.

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

- builtin effects such as `FileRead`, `Network`, `Deploy`, and `Spend` are always
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

`DatabaseWrite` backs `db.try_insert(table, row)`. The adapter accepts a
record or map row, creates the per-run table if needed, inserts the row into
the runtime database state for that execution, and returns
`Result<DbRow, Error>`:

```sley
task main -> Result<Text, Error> uses DatabaseWrite, DatabaseRead {
  bind inserted = call db.try_insert("users", { id: "u3", name: "Lin" })?
  bind row = call db.query_one("select * from users where id = ?", inserted.text("id"))
  return Ok(row.text("name"))
}
```

Empty table names produce `RUNTIME_DB_TABLE_INVALID` as a typed host error.
Missing `DatabaseWrite` or `DbWrite` remains a runtime capability diagnostic.

`SecretRead` backs `secrets.try_get(name)`. The adapter reads exact seeded
secret values supplied by the host and returns `Result<Text, Error>`:

```bash
sley run --cap SecretRead --secret api_key redacted examples/secret_gate.sley
```

```sley
task main -> Result<Text, Error> uses SecretRead {
  bind value = call secrets.try_get("api_key")?

  return Ok(value)
}
```

This is deterministic host I/O, not a real secret backend. It does not read
environment variables, keyrings, vaults, or provider secret stores. Missing
secret seeds produce `RUNTIME_SECRET_NOT_FOUND`; empty names produce
`RUNTIME_SECRET_NAME_INVALID`; missing `SecretRead` remains a runtime
capability diagnostic.

`Deploy` backs `deploy.try_stage(target)`. The adapter reads exact seeded
deployment stage results supplied by the host and returns `Result<Text, Error>`:

```bash
sley run --cap Deploy --deploy-result staging staged examples/deploy_gate.sley
```

```sley
task main -> Result<Text, Error> uses Deploy {
  bind result = call deploy.try_stage("staging")?

  return Ok(result)
}
```

This is deterministic host I/O, not live deployment. It does not upload
artifacts, push branches, start services, mutate infrastructure, or call
providers. Missing target seeds produce `RUNTIME_DEPLOY_RESULT_NOT_FOUND`;
empty targets produce `RUNTIME_DEPLOY_TARGET_INVALID`; missing `Deploy`
remains a runtime capability diagnostic.

`Spend` backs `spend.try_authorize(request)`. The adapter reads exact seeded
spend authorization results supplied by the host and returns
`Result<Text, Error>`:

```bash
sley run --cap Spend --spend-result ads-budget authorized examples/spend_gate.sley
```

```sley
task main -> Result<Text, Error> uses Spend {
  bind result = call spend.try_authorize("ads-budget")?

  return Ok(result)
}
```

This is deterministic host I/O, not live spending. It does not create
transactions, make payments, place market orders, call wallets, buy credits,
mutate cloud-billing state, or call providers. Missing request seeds produce
`RUNTIME_SPEND_RESULT_NOT_FOUND`; empty requests produce
`RUNTIME_SPEND_REQUEST_INVALID`; missing `Spend` remains a runtime capability
diagnostic.

`Network` backs `http.try_get_text(url)`. The adapter reads exact seeded URL
responses supplied by the host and returns `Result<Text, Error>`:

```bash
sley run --cap Network --http-text https://example.test/profile Ada examples/network_gate.sley
```

```sley
task main -> Result<Text, Error> uses Network {
  bind body = call http.try_get_text("https://example.test/profile")?

  return Ok(body)
}
```

This is deterministic host I/O, not live outbound HTTP. Missing response seeds
produce `RUNTIME_HTTP_RESPONSE_NOT_FOUND`; empty URLs produce
`RUNTIME_HTTP_URL_INVALID`; missing `Network` remains a runtime capability
diagnostic.

`Shell` backs `shell.try_run(command)`. The adapter reads exact seeded command
outputs supplied by the host and returns `Result<Text, Error>`:

```bash
sley run --cap Shell --shell-output date 2026-05-05 examples/shell_gate.sley
```

```sley
task main -> Result<Text, Error> uses Shell {
  bind output = call shell.try_run("date")?

  return Ok(output)
}
```

This is deterministic host I/O, not subprocess execution. Missing output seeds
produce `RUNTIME_SHELL_OUTPUT_NOT_FOUND`; empty commands produce
`RUNTIME_SHELL_COMMAND_INVALID`; missing `Shell` remains a runtime capability
diagnostic.

`ModelCall` backs `model.try_complete(prompt)`. The adapter reads exact seeded
prompt completions supplied by the host and returns `Result<Text, Error>`:

```bash
sley run --cap ModelCall --model-output name Ada examples/model_gate.sley
```

```sley
task main -> Result<Text, Error> uses ModelCall {
  bind answer = call model.try_complete("name")?

  return Ok(answer)
}
```

This is deterministic host I/O, not a provider client. Missing output seeds
produce `RUNTIME_MODEL_OUTPUT_NOT_FOUND`; empty prompts produce
`RUNTIME_MODEL_PROMPT_INVALID`; missing `ModelCall` remains a runtime capability
diagnostic.

## Graft Model

Agents should edit Sley by submitting structural grafts against bounded graph
shards. A graft is accepted only after parse, type, effect, authority, lifetime,
and provenance checks pass for the implemented v0 surface.

Current graft operations include adding/removing takes, replacing task bodies,
adding imports/effects/types/tasks, renaming declarations, updating call-sites,
updating, replacing, or removing call arguments, inserting checked task-body
statements, replacing expressions by node id, and deleting checked graph nodes
such as declarations, imports, takes, and statements. `MoveNode` reorders
statements within their existing block, moves statements across existing block
parents with `payload.destination`, reorders takes within their owning task,
moves takes across task take lists with `payload.destination`, and reorders
top-level imports, types, effects, or tasks within their declaration lists. It
can also move top-level types, effects, or tasks into a known loaded or imported
module parent such as `module:app.extra:tasks`.
Unsupported graph movement returns explicit diagnostics until implemented.

Implemented graph-edit payloads:

```json
{ "op": "UpdateCallSites", "target": "task:app.math.double", "payload": { "replacement": "math.twice" } }
{ "op": "UpdateCallSites", "target": "task:app.math.twice", "payload": { "from": "math.double", "replacement": "math.twice", "scope": "module:app.main" } }
{ "op": "UpdateCallArgs", "target": "task:app.math.double", "payload": { "from": "math.double", "source": "\"\"", "position": 1, "scope": "module:app.main" } }
{ "op": "ReplaceCallArg", "target": "task:app.math.double", "payload": { "from": "math.double", "source": "42", "position": 0, "scope": "module:app.main" } }
{ "op": "RemoveCallArg", "target": "task:app.math.double", "payload": { "from": "math.double", "position": 1, "scope": "module:app.main" } }
{ "op": "InsertStatement", "target": "task:app.main.main", "payload": { "position": 1, "source": "set total = total + 1" } }
{ "op": "ReplaceExpression", "target": "block:task:app.main.main:stmt:0:expr:right", "payload": { "source": "41" } }
{ "op": "DeleteNode", "target": "block:task:app.main.main:stmt:1" }
{ "op": "MoveNode", "target": "block:task:app.main.main:stmt:1", "payload": { "parent": "block:task:app.main.main", "position": 0 } }
{ "op": "MoveNode", "target": "block:task:app.main.main:stmt:1:then:stmt:0", "payload": { "parent": "block:task:app.main.main:stmt:1:then", "destination": "block:task:app.main.main", "position": 1 } }
{ "op": "MoveNode", "target": "take:task:app.main.helper:value", "payload": { "parent": "task:app.main.helper:takes", "position": 0 } }
{ "op": "MoveNode", "target": "take:task:app.main.helper:value", "payload": { "parent": "task:app.main.helper:takes", "destination": "task:app.main.main:takes", "position": 0 } }
{ "op": "MoveNode", "target": "task:app.main.helper", "payload": { "parent": "program.tasks", "position": 0 } }
{ "op": "MoveNode", "target": "task:app.main.helper", "payload": { "parent": "module:app.extra:tasks", "position": 0 } }
```

`UpdateCallSites` rewrites call expressions that either resolve to the target
task or match the optional raw `from` callee. `UpdateCallArgs` inserts one
checked argument expression into matching calls; `position` defaults to append.
`ReplaceCallArg` replaces one argument at a required `position`. `RemoveCallArg`
removes one argument at a required `position` from matching calls. For all four
operations, `scope` can limit the rewrite to a task or module.
`InsertStatement` currently targets a task body. `DeleteNode`
can remove declarations, imports, takes, and statements, but rejects expression
targets unless the agent uses `ReplaceExpression` instead. `MoveNode` currently
requires `payload.position`; `payload.parent` is optional but, when present,
must identify the current parent. Statement moves use a block parent such as
`block:task:app.main.main`; `payload.destination` can identify another existing
block parent for checked cross-parent statement movement. A statement cannot be
moved into one of its own child blocks. Take moves accept a parent such as
`task:app.main.helper:takes` and reorder within that task's take list; with
`payload.destination`, they can move into another task take parent such as
`task:app.main.main:takes`. Top-level declaration moves accept
`program.imports`, `program.types`, `program.effects`, or `program.tasks`.
Top-level import, type, effect, and task moves can use
`module:<path>:imports`, `module:<path>:types`, `module:<path>:effects`, or
`module:<path>:tasks` to change ownership and place the declaration at a
module-local position. For example:

```json
{
  "op": "MoveNode",
  "target": "import:app.main:app.shared",
  "payload": { "parent": "module:app.extra:imports", "position": 0 }
}
```

The destination module must already be loaded or imported in the checked
candidate; an all-or-nothing transaction can therefore `AddImport` for the
destination module before `MoveNode`, and project writeback will create the new
checked module file when the moved item is the module's first content.
Otherwise `GRAFT_MODULE_MISSING` rejects the move before mutation. Expression
moves reject with `GRAFT_MOVE_UNSUPPORTED`. Each accepted edit reparses the
payload when applicable, rewrites the AST, refreshes expression source text, and
reruns the checker before returning formatted source.

Accepted grafts written with `sley graft --write` append receipt records to a
local `.sley/trace.jsonl` sidecar unless the caller passes an explicit trace
path. The trace sidecar is a local provenance store. `sley seal` turns the
current source, symbol graph, and trace receipt chain into a deterministic
content-addressed seal.

`RenameDeclaration` can rename tasks, types, effects, and module targets such
as `module:app.extra`. Module rename updates declaration ownership and import
references in the checked candidate. If an import has no explicit alias and the
module leaf changes, writeback preserves existing call qualifiers by adding an
alias for the old leaf.

When the target is a project directory or `sley.toml`, `sley graft` loads the
manifest entry module and transitively imported modules before checking the
graft. Accepted project writeback projects the checked candidate AST back to the
existing module files that own changed imports, types, effects, or tasks, and
can create checked new module files under the project source root when the graft
candidate owns declarations in those modules. It deletes loaded module files
whose imports and declarations are removed from the checked candidate, and can
rename module files by deleting the old loaded module file and creating the new
checked module file. If the renamed module is the project entry module,
writeback updates `sley.toml` to point at the new entry. Unchanged module files
are left byte-for-byte untouched. Project writeback rejects before mutation when
a graft imports a missing module without adding checked declarations for it,
using `PROJECT_WRITEBACK_UNKNOWN_IMPORT`.

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
sley new --json --template deploy --name agent-app agent-app
sley doctor --json <target>
sley plan --json [--graft-templates] [--template-surface <task>] <target>
sley query --json --kind tasks --module app.main <target>
sley lint --json <target>
sley trace --json <target>
sley seal --json <target>
```

AST roots carry `schema: "sley.ast.program.v0"`. Diagnostic reports carry
`schema: "sley.diagnostics.report.v0"`. Full symbol graphs carry
`schema: "sley.symbol_graph.v0"`, graph slices carry
`schema: "sley.symbol_graph.slice.v0"`, graft outcomes carry
`schema: "sley.graft.outcome.v0"`, checked query reports carry
`schema: "sley.query.report.v0"`, checked lint reports carry
`schema: "sley.lint.report.v0"`, trace seals carry
`schema: "sley.trace.seal.v0"`, and ZJX preview envelopes carry
`schema: "sley.zjx.envelope.v0"`.
The CLI smoke manifest carries
`schema: "sley.cli_smoke.manifest.v0"`. Project scaffold reports carry
`schema: "sley.project.scaffold.v0"`. Doctor readiness reports carry
`schema: "sley.doctor.report.v0"`. Edit-plan reports carry
`schema: "sley.edit_plan.report.v0"`.

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
and provenance records. Symbol graph, graph slice, and query schemas also pin
module import/declaration summary shapes so agents can rely on stable import
node ids for graft targets. The remaining schema files currently pin their
top-level contract shape and stable schema IDs.

The compiler conformance corpus lives under `fixtures/corpus/`. Its
`manifest.json` lists every accepted and rejected fixture plus coverage tags.
Accepted fixtures must parse, check, and formatter-round-trip. Rejected
fixtures carry a JSON sidecar listing the diagnostic ids that must remain
stable. The current corpus locks declared and missing authority coverage for
the deterministic seeded host adapters.

The executable CLI conformance smokes live under `fixtures/cli_smokes/`. Their
`manifest.json` lists stable commands, working-directory mode, coverage tags,
and stdout expectations. The integration suite runs the manifest against the
built `sley` binary and locks stable command exits, selected stdout substrings,
JSON root schemas, graph slices, checked query reports, checked lint reports,
doctor readiness reports, edit-plan reports, project scaffolds, ZJX preview
envelopes, graft dry runs, graph-slice replace affordances, checked
`replace_expression` graft templates, and seeded host-adapter execution for
`FileRead`, `FileWrite`, `DatabaseRead`, `DatabaseWrite`, `Network`, `Shell`,
`ModelCall`, `SecretRead`, `Deploy`, and `Spend`.

`sley new` is the v0 project scaffold command. It writes a `sley.toml`,
`README.md`, and entry module source file, refusing to overwrite any of those
paths when they already exist. `--template hello` creates a pure starter.
`--template deploy` creates a deterministic `Deploy`-gated starter that runs
with a seeded `deploy.try_stage` result and does not call providers or mutate
infrastructure. `--json` emits the scaffold report with relative file paths
and exact next-command vectors under `sley.project.scaffold.v0`.

`sley doctor` is the first deterministic helper that consumes the strict
checker plus checked query and lint reports into one agent readiness report.
It reports `ready`, `warnings`, or `blocked`; includes source schema references
for the consumed query and lint surfaces; and gives next-command vectors for
inspection, lint gates, and entrypoint runs. `--deny-warnings` treats lint
findings as blocked while still printing the versioned report.

`sley plan` is the first deterministic pre-edit helper built from the same
strict checker, checked query, and checked lint surfaces. It reports `ready`,
`warnings`, or `blocked`; carries full lint findings; ranks task edit surfaces
with stable task ids, qualified names, takes, call counts, declared effects,
graft target ids, and planning notes; and gives next-command vectors for graph
slice inspection plus post-edit doctor and verify gates. `--deny-warnings`
treats lint findings as blocked while still printing
`schema: "sley.edit_plan.report.v0"`. `--graft-templates` adds starter strict
graft operation payloads for the highest-ranked task surface, plus JSON
pointers naming the fields an agent should edit before running
`sley graft --json --dry-run`; it also consumes the selected task graph slice
and adds `move_statement`/`move_take` templates plus destination variants from
checked `move_affordances`, and checked `delete_statement`/`delete_take`
templates from `delete_affordances` when the starter delete graft validates,
plus checked `replace_expression` templates from `replace_affordances` when the
starter expression graft validates.
`--template-surface <task>` selects a specific task surface by task node id or
qualified task name. For selected tasks with currently resolved inbound callers,
the report also includes an all-or-nothing `RenameDeclaration` plus
`UpdateCallSites` transaction template and an `AddTake` plus `UpdateCallArgs`
transaction template. When the selected task has an unused normal take,
`sley plan --graft-templates` can also emit a `RemoveTake` plus `RemoveCallArg`
transaction template.

A graph slice is a bounded shard around a module, task, type, effect, or import.
Task slices include the selected task AST, visible module declarations,
outbound calls, and inbound calls from other tasks that resolve to the selected
task. This is the first stable agent-facing query surface for graft planning.
Import summaries expose canonical import node ids such as
`import:app.main:app.shared`, so import `MoveNode` and `DeleteNode` grafts can
copy targets directly from graph, graph-slice, or query JSON. Graph slices also
include `move_affordances` for bounded import, type, effect, task, statement,
and take movement planning: each affordance exposes the exact `MoveNode`
target, current parent, current position, in-parent maximum position, and
available destination parents with their insertion limits. Affordances and
destination entries also carry strict starter `MoveNode` operation JSON plus
editable JSON pointers so agents can copy a template, adjust
`/payload/position`, and dry-run the graft. Graph slices also include
`delete_affordances` for import, type, effect, task, statement, and take
`DeleteNode` planning; each delete affordance exposes the exact target, current
parent, current position, starter operation JSON, and editable pointer list.
Graph slices also include task-local `replace_affordances` for
`ReplaceExpression` planning; each replace affordance exposes the exact
expression target, expression kind, parent node id, starter operation JSON, and
editable `/payload/source` pointer.
Call-site and expression grafts now consume node ids and task identities from
this shard.

`sley query` is the first checked graph query report. It parses and checks the
target before emitting results, so semantic failures return the normal
diagnostic report instead of a partial query. `--kind all|modules|tasks|calls`
selects the report body. `--module <module>` filters module summaries, task
summaries, and calls that originate from or resolve into that module.
`--exported` restricts declaration and task summaries to exported declarations.
Task query rows include stable ids, qualified names, takes, return type text,
declared effects, and inbound/outbound call counts. The v0 JSON root is
`sley.query.report.v0`.

`sley lint` is the first checked lint command built on the graph query surface.
It parses and checks the target before emitting results, so semantic failures
return the normal diagnostic report instead of a lint report. The v0 rules are
`unused_private_task`, which warns on non-exported tasks with zero inbound
checked calls excluding the entry module's `main`; `unreachable_private_task`,
which warns on private task islands that are only reachable from other private
tasks rather than from `main` or an exported task; and
`unused_declared_effect`, which warns when a task declares an effect that no
direct host call or resolved called-task effect justifies; `unused_import`,
which warns when an import is not needed by any checked task, type, or effect;
`unused_take`, which warns when a normal task take is never read by the task
body; and `raw_host_adapter`, which warns on legacy diagnostic-failing host
calls that should move to fallible `try_` adapters. `--module <module>` scopes
the lint to one module. `--rule unused-private-task`,
`--rule unreachable-private-task`, `--rule unused-declared-effect`,
`--rule unused-import`, `--rule unused-take`, or `--rule raw-host-adapter`
selects one rule explicitly, and `--deny-warnings` turns findings into a
nonzero CLI exit after printing the report. The v0 JSON root is
`sley.lint.report.v0`.

Diagnostics include machine-readable repair hints for common checker failures
and selected graft planning failures. Hints are intentionally small and
structural: `kind` identifies the action, `target` names the graph node when
available, `effect` names required authority when relevant, and `replacement`
carries compact source, type, graft JSON, or retry guidance. For example,
parse expected-token failures carry `insert_expected_token` hints; missing
names carry `provide_identifier`; empty expression sites carry
`provide_expression`; and top-level/export item mismatches carry
`choose_expected_item`.
unsupported expression `MoveNode` targets carry a `replace_expression` hint so
agents can switch to `ReplaceExpression` instead of retrying an unsupported
move. Unsupported graft shapes also carry `use_supported_graft_operation`
guidance that points agents toward the supported parent, destination, or
operation shape; expression `DeleteNode` targets additionally carry
`replace_expression` because expressions require replacement, not deletion.
Duplicate task/type/effect checker diagnostics and graft namespace collisions
carry `resolve_namespace_conflict` hints so agents rename, remove, or reuse the
existing declaration instead of retrying the same colliding name.
Stale graft precondition failures carry a `refresh_graft_precondition` hint so
agents re-read current target state and rebuild the graft before retrying. Call
argument type mismatches carry both an expression-level `replace_argument` hint
and a structural `replace_call_arg` hint whose `replacement` is a starter
`ReplaceCallArg` graft JSON payload scoped to the current caller and raw
callee text. Call arity mismatches keep the generic
`match_task_arity` hint and add structural `update_call_args` or
`remove_call_arg` hints when a missing or extra argument can be repaired with a
starter graft. Return type mismatches carry both the declaration-level
`change_return_type` hint and expression-level hints; the structural
`replace_expression` hint includes starter `ReplaceExpression` graft JSON for
the returned expression. Binding and assignment type mismatches likewise include
structural `replace_expression` hints for the initializer or assigned
expression, alongside the older type-change or source-level replacement hints.
Missing return paths in non-`Unit` tasks produce `MISSING_RETURN` with
`insert_return` and `replace_task_body` repair hints.
Condition, collection element, map key/value, index key, and record-field
expression mismatches include structural `replace_expression` hints when the
checker has a clear expected replacement type for the offending expression.
Record literal missing/unknown field diagnostics include a whole-record
`replace_expression` hint that preserves known expected fields, fills missing
fields with default starter expressions, and omits unknown fields.
Unary and binary operator mismatches likewise include structural
`replace_expression` hints when a specific operand can be replaced without
guessing between valid type families. If-expression branch type mismatches
include one structural `replace_expression` hint for the then branch and one
for the else branch so agents can choose which branch should conform.
Non-iterable `each` collections and invalid `len` arguments include
conservative structural `replace_expression` starter hints that produce a valid
empty list or empty text argument. Non-indexable collection expressions include
structural `replace_expression` hints when the index expression is already
known to be `Int` or `Text`, selecting an empty list or empty map starter.

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

- runtime gates currently back filesystem text reads/writes, deterministic
  seeded database reads, per-run deterministic database inserts, seeded secret
  values, seeded deployment stage results, seeded spend authorizations, seeded
  network text responses, seeded shell command outputs, and seeded model prompt
  completions
- Sley-level `Result` values, `?` propagation, and typed filesystem,
  database, secret, deploy, spend, network, shell, and model host fallibility
  execute
- trace receipts can be sealed, but sidecar storage is not yet a compressed ZJX
  archive
- the AST JSON Schema covers nested AST and expression variants; the remaining
  JSON Schema files are still narrower v0 root contracts
- `sley lint` currently ships warning-grade private-task graph rules, authority
  hygiene for unused declared effects, import/API hygiene, and raw-host-adapter
  migration warnings; broader style and migration lints remain later work
- `sley plan` emits deterministic ranked task edit surfaces, optional starter
  graft operation templates, rename-plus-call-site and add-take-plus-call-arg
  transaction templates, safe remove-take-plus-call-arg transaction templates
  for unused takes, and post-edit gate commands; it does not yet choose or
  execute a final graft for the agent
- `sley verify` emits a deterministic CI/pre-deploy report over strict check,
  query/lint summaries, and runtime execution; live deploy/provider calls
  remain outside v0
- no `match`, agent declarations, spawn/cast/join, or compressed ZJX archive
  writer yet
- `MoveNode` supports checked in-parent and cross-parent statement movement
  between existing block parents, take reordering within the owning task,
  checked cross-task take movement, top-level declaration ordering, and
  top-level import/type/effect/task movement into known modules; expression
  movement is still an explicit rejection
- project graft writeback updates existing module files, creates checked new
  module files, deletes removed module files, renames module files, and updates
  the project manifest for entry-module renames
