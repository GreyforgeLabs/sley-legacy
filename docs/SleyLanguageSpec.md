# Sley Language Specification

Status: v0 executable slice plus module task/type/effect namespace, runtime gates with filesystem roots and non-file seeded resource scopes, Sley-level Result flow, typed host fallibility, seeded database host reads/writes, seeded secret values, seeded deploy stage results, seeded spend authorizations, seeded network host text, seeded shell host output, seeded model completions, trace tooling, checked lint tooling, and checked edit-plan tooling

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

The CLI grants runtime gates with repeated `--cap EFFECT[=SCOPE]` flags:

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
For non-filesystem host effects, `=SCOPE` is a deterministic text prefix over
the seeded resource key: database table, URL, secret name, shell command, model
prompt, deploy target, or spend request. A scoped host adapter that tries to
touch a non-matching seeded resource fails with
`RUNTIME_CAPABILITY_SCOPE_DENIED`; this remains an authority diagnostic, not a
recoverable `Result` error.

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

Current graft operations include adding explicit module declarations,
adding/removing takes, removing task-declared effects, replacing task bodies,
adding imports/effects/types/tasks, renaming declarations, updating call-sites,
updating, replacing, or removing call arguments, inserting checked task-body
statements, replacing checked statements and expressions by node id, and
deleting checked graph nodes such as declarations, imports, takes, and
statements. `MoveNode` reorders statements within their existing block, moves
statements across existing block
parents with `payload.destination`, reorders takes within their owning task,
moves takes across task take lists with `payload.destination`, and reorders
top-level imports, types, effects, or tasks within their declaration lists. It
can also move top-level types, effects, or tasks into a known loaded or imported
module parent such as `module:app.extra:tasks`.
Unsupported graph movement returns explicit diagnostics until implemented.

Implemented graph-edit payloads:

```json
{ "op": "AddModuleDeclaration", "payload": { "name": "app.main" } }
{ "op": "UpdateCallSites", "target": "task:app.math.double", "payload": { "replacement": "math.twice" } }
{ "op": "UpdateCallSites", "target": "task:app.math.twice", "payload": { "from": "math.double", "replacement": "math.twice", "scope": "module:app.main" } }
{ "op": "UpdateCallArgs", "target": "task:app.math.double", "payload": { "from": "math.double", "source": "\"\"", "position": 1, "scope": "module:app.main" } }
{ "op": "ReplaceCallArg", "target": "task:app.math.double", "payload": { "from": "math.double", "source": "42", "position": 0, "scope": "module:app.main" } }
{ "op": "RemoveCallArg", "target": "task:app.math.double", "payload": { "from": "math.double", "position": 1, "scope": "module:app.main" } }
{ "op": "InsertStatement", "target": "task:app.main.main", "payload": { "position": 1, "source": "set total = total + 1" } }
{ "op": "InsertStatement", "target": "block:task:app.main.main:stmt:1:then", "payload": { "position": 0, "source": "forge { }" } }
{ "op": "ReplaceStatement", "target": "block:task:app.main.main:stmt:0", "payload": { "source": "bind answer = 42" } }
{ "op": "ReplaceExpression", "target": "block:task:app.main.main:stmt:0:expr:right", "payload": { "source": "41" } }
{ "op": "DeleteNode", "target": "block:task:app.main.main:stmt:1" }
{ "op": "MoveNode", "target": "block:task:app.main.main:stmt:1", "payload": { "parent": "block:task:app.main.main", "position": 0 } }
{ "op": "MoveNode", "target": "block:task:app.main.main:stmt:1:then:stmt:0", "payload": { "parent": "block:task:app.main.main:stmt:1:then", "destination": "block:task:app.main.main", "position": 1 } }
{ "op": "MoveNode", "target": "take:task:app.main.helper:value", "payload": { "parent": "task:app.main.helper:takes", "position": 0 } }
{ "op": "MoveNode", "target": "take:task:app.main.helper:value", "payload": { "parent": "task:app.main.helper:takes", "destination": "task:app.main.main:takes", "position": 0 } }
{ "op": "MoveNode", "target": "task:app.main.helper", "payload": { "parent": "program.tasks", "position": 0 } }
{ "op": "MoveNode", "target": "task:app.main.helper", "payload": { "parent": "module:app.extra:tasks", "position": 0 } }
```

`AddModuleDeclaration` adds an explicit module header to a module-less source
and rewrites default `main` ownership to the selected module; it rejects when a
module is already declared or the module name collides. `sley plan` and
`sley fix` choose the starter module name from the project-relative source path
when the file sits under a `sley.toml` source root, otherwise from the `.sley`
file stem. `UpdateCallSites` rewrites call expressions that either resolve to
the target task or match the optional raw `from` callee. `UpdateCallArgs`
inserts one
checked argument expression into matching calls; `position` defaults to append.
`ReplaceCallArg` replaces one argument at a required `position`. `RemoveCallArg`
removes one argument at a required `position` from matching calls. For all four
operations, `scope` can limit the rewrite to a task or module.
`InsertStatement` targets a task body or an exact block node id such as
`block:task:app.main.main:stmt:1:then`; `sley plan --template-surface
<block-id>` emits a checked starter template. `ReplaceStatement` targets an
exact statement node id and replaces it with exactly one checked statement
parsed from `payload.source`. `DeleteNode` can remove declarations, imports,
takes, and statements, but rejects expression targets unless the agent uses
`ReplaceExpression` instead. `MoveNode` currently
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

Accepted grafts written with `sley graft --write` and plan-consuming fixes
written with `sley fix --write` append receipt records to a local
`.sley/trace.jsonl` sidecar unless the caller passes an explicit
`--trace <trace.jsonl>` path. Dry-run paths never append receipts. The trace
sidecar is a local provenance store. `sley seal` turns the current source,
symbol graph, and trace receipt chain into a deterministic content-addressed
seal.

`RenameDeclaration` can rename tasks, types, effects, and module targets such
as `module:app.extra`. Module rename updates declaration ownership and import
references in the checked candidate. If an import has no explicit alias and the
module leaf changes, writeback preserves existing call qualifiers by adding an
alias for the old leaf.

When the target is a project directory or `sley.toml`, `sley graft` loads the
manifest entry module and transitively imported modules before checking the
graft. Accepted project writeback projects the checked candidate AST back to the
existing module files that own changed imports, types, effects, or tasks, and
can add imports to existing on-disk module files that were not yet loaded
through the entry import graph after validating their module declarations. It
can create checked new module files under the project source root when the
graft candidate owns declarations in those modules. It deletes loaded module
files whose imports and declarations are removed from the checked candidate,
and can rename module files by deleting the old loaded module file and creating
the new checked module file. If the renamed module is the project entry module,
writeback updates `sley.toml` to point at the new entry. Unchanged module files
are left byte-for-byte untouched. Project writeback rejects before mutation
when a graft imports a truly missing module without adding checked declarations
for it, using `PROJECT_WRITEBACK_UNKNOWN_IMPORT`.

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
  "diagnostics": [],
  "source": "task main -> Int {\n  return 1\n}\n",
  "provenance": []
}
```

Rejected graft outcomes use the same schema and `status: "rejected"`, omit
`source`, emit `provenance: []`, and include diagnostics.
Accepted provenance entries are strict records with `graft_id`, `actor`,
`timestamp`, `operation`, non-empty `targets`, and `result: "accepted"`.

## Graph And Trace Tooling

The Loom exposes the program graph as inspectable JSON:

```bash
sley ast --json <target>
sley ast --json --node task:app.main.main <target>
sley graph --json <target>
sley graph --json --slice task:app.main.main <target>
sley new --json --template agent --name agent-app agent-app
sley doctor --json <target>
sley plan --json [--graft-templates] [--template-surface <surface>] [--emit-graft <kind>] <target>
sley fix --json --kind <kind> [--template-surface <surface>] [--name <name>] [--type <type>] [--module <module>] [--source <source>|--source-file <path>] [--position <n>] [--dry-run|--write] [--trace <trace.jsonl>] <target>
sley query --json --kind tasks --module app.main <target>
sley query --json --kind types --module app.main <target>
sley query --json --kind effects --module app.main <target>
sley lint --json <target>
sley deploy --json --dry-run [--artifacts-dir <dir>] <target>
sley trace --json <target>
sley seal --json <target>
```

AST roots carry `schema: "sley.ast.program.v0"`. Bounded AST node reports from
`sley ast --json --node <node-id>` carry `schema: "sley.ast.node.v0"` and wrap
the selected program, import, type, effect, task, take, block, statement, or
expression node with its id, node kind, module, parent, and raw AST value.
Diagnostic reports carry
`schema: "sley.diagnostics.report.v0"` and expose the shared diagnostic record
used by graft, doctor, plan, verify, and runtime diagnostic arrays. Full symbol
graphs carry
`schema: "sley.symbol_graph.v0"`, graph slices carry
`schema: "sley.symbol_graph.slice.v0"`, graft outcomes carry
`schema: "sley.graft.outcome.v0"`, checked query reports carry
`schema: "sley.query.report.v0"`, checked lint reports carry
`schema: "sley.lint.report.v0"`, run reports carry
`schema: "sley.run.report.v0"`, trace reports carry
`schema: "sley.trace.report.v0"`, trace receipts carry
`schema: "sley.trace.receipt.v0"`, trace seals carry
`schema: "sley.trace.seal.v0"`, ZJX preview envelopes carry
`schema: "sley.zjx.envelope.v0"`, ZJX tool inspection reports carry
`schema: "sley.zjx.tool.report.v0"`, and deterministic agent-loop benchmark
reports carry `schema: "sley.agent_bench.report.v0"`. Checked migration
reports carry `schema: "sley.migrate.report.v0"`.
The CLI smoke manifest carries
`schema: "sley.cli_smoke.manifest.v0"`. The accepted/rejected compiler corpus
manifest carries `schema: "sley.conformance.manifest.v0"`. Project scaffold reports carry
`schema: "sley.project.scaffold.v0"`. Doctor readiness reports carry
`schema: "sley.doctor.report.v0"`. Edit-plan reports carry
`schema: "sley.edit_plan.report.v0"`. CI wrapper reports carry
`schema: "sley.ci.report.v0"` for check, lint, doctor, plan, run, verify,
deploy, smoke, corpus, and examples wrappers.
Conformance visibility reports carry `schema: "sley.conformance.report.v0"`;
coverage-tag checks carry `schema: "sley.conformance.coverage.v0"`.
The deploy wrapper passes `--artifacts-dir <dir>` through to `sley deploy`
when local handoff files are requested. Deploy dry-run reports carry
`schema: "sley.deploy.report.v0"`; deploy artifact manifests carry
`schema: "sley.deploy.artifacts.v0"`. Deploy artifact check reports carry
`schema: "sley.deploy.artifact_check.v0"`. Contract utility reports carry
`schema: "sley.contract.inventory.v0"`,
`schema: "sley.contract.fixture_check.v0"`, or
`schema: "sley.contract.validate.v0"`.

Current `sley trace --json` reports have this root shape:

```json
{
  "schema": "sley.trace.report.v0",
  "status": "ok",
  "target": "examples/hello.sley",
  "trace_path": "examples/.sley/trace.jsonl",
  "receipt_count": 0,
  "receipts": []
}
```

Current v0 trace receipt JSONL records have this root shape:

```json
{
  "schema": "sley.trace.receipt.v0",
  "target": "examples/hello.sley",
  "written_at": "2026-05-05T00:00:00Z",
  "provenance": []
}
```

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
`fixtures/contracts/` and JSON Schema files under `docs/schemas/`, including
contract inventory/fixture-check/validate fixtures, graft outcome fixtures, and
full symbol graph and ZJX envelope fixtures for handoff roots. The AST program
schema now recursively describes imports,
types, effects, tasks, takes, statements, expressions, type expressions, spans,
and provenance records. The diagnostic schema pins the shared diagnostic and
repair-hint shape. The query schema exposes strict task, take, type, effect,
and call summary row definitions for downstream bindings. Contract utility
schemas pin inventory, fixture root-check, and single-report root-match
results. Symbol graph, graph slice, and query schemas also pin module
import/declaration summary shapes so agents can rely on stable import node ids
for graft targets. The edit-plan schema pins strict graft operation and
transaction template envelopes, and graph-slice insert/move/delete/replace
affordance operations reuse that strict graft operation schema. Graph-slice
focus, optional task, and inbound/outbound call summaries reuse AST and query report
contracts. The graft outcome schema pins strict accepted provenance records.
The trace receipt schema pins the JSONL receipt record for accepted write
provenance. The ZJX envelope schema pins the graph digest, graph root, optional
graph slice root, and trace receipt schema used for handoff. The LSP
fix-preview schema pins the non-mutating editor command payload, the workbench
schema pins the local inspection report root, the agent-bench schema pins
deterministic repair-loop evidence, and the migrate schema pins checked
migration commands plus schema-drift rows. The remaining schema files
currently pin their top-level contract shape and stable schema IDs.

The compiler conformance corpus lives under `fixtures/corpus/`. Its
`manifest.json` lists every accepted and rejected fixture plus coverage tags.
Accepted fixtures must parse, check, and formatter-round-trip. Rejected
fixtures carry a JSON sidecar listing the diagnostic ids that must remain
stable. The current corpus locks declared and missing authority coverage for
the deterministic seeded host adapters. `sley-ci corpus --json
fixtures/corpus/manifest.json` exposes the same accepted/rejected gate as a
machine-readable CI report: accepted cases run strict check plus formatter
round-trip checks, while rejected cases must fail with the expected diagnostic
ids from their sidecars.

The executable CLI conformance smokes live under `fixtures/cli_smokes/`. Their
`manifest.json` lists stable commands, working-directory mode, optional temp
setup files, coverage tags, and stdout expectations. The integration suite runs
the manifest against the built `sley` binary and locks stable command exits,
selected stdout substrings, JSON root schemas, graph slices, checked query
reports, checked lint reports, run reports, doctor readiness reports,
edit-plan reports, project scaffolds, ZJX preview envelopes, graft dry runs
and direct graft writes, write-mode fix trace receipts, non-empty trace
receipt seals, ZJX envelopes carrying graph digests and schema-backed trace
receipts, ZJX tool reports, graph-slice insert and replace affordances, checked
`insert_statement`, `replace_statement`, and `replace_expression` graft
templates, lint-driven fix
writes that clear warnings before verify, program-surface declaration/import
template planning and dry-run fixes with name/source/module overrides, deploy
dry-run reports, typed starter/deploy/agent scaffold next-actions, and
seeded host-adapter
execution for `FileRead`, `FileWrite`, `DatabaseRead`, `DatabaseWrite`,
`Network`, `Shell`, `ModelCall`, `SecretRead`, `Deploy`, and `Spend`.

The packaged example gate is exposed through `sley-ci examples --json
examples`. It checks each `sley.toml` example as a project root, checks
standalone `.sley` examples directly, and formatter-round-trips every shipped
`.sley` source under `examples/`.

The release-readiness view is exposed through `sley-conformance report
--json`. It inventories schema IDs, fixture and manifest schema instances,
contract fixture validation status, release manifest validation status, corpus
tags, smoke tags, packaged example counts, and declared integration-test count
drift under one stable conformance root. `sley-conformance coverage --json
--require-tag <tag>` checks explicit coverage tags across the corpus and smoke
manifests for focused release gates.

The source review projection is also backed by the in-tree `tree-sitter-sley`
bootstrap. Its grammar tracks module/import declarations, type/effect/task
declarations, authority takes, bindings, control flow, call/try expressions,
records, maps, lists, comments, and highlight queries for editor integration.
The compiler remains the semantic authority; Tree-sitter is for syntax review,
tokenization, and editor ergonomics.

The editor feedback loop is backed by the in-tree `sley-lsp` bootstrap. It
speaks stdio LSP framing, publishes parse/check diagnostics plus lint warnings,
formats documents with the compiler formatter, exposes declaration symbols and
hover text, and returns checked edit-plan code actions with a non-mutating
`sley.fix.preview` command. The server reuses compiler modules directly; it is
not a separate semantic implementation.

The local inspection loop is backed by the in-tree `sley-workbench` bootstrap.
It emits `schema: "sley.workbench.report.v0"` and can write an explicit static
HTML report containing doctor, query, lint, edit-plan, and graph panels. The
workbench is read-only for source code; HTML output requires `--html <path>`.

The local agent-loop benchmark is backed by the in-tree `sley-agent-bench`
bootstrap. `sley-agent-bench run --json` emits
`schema: "sley.agent_bench.report.v0"` after running a deterministic repair
case through check, query, lint, checked repair planning, write-mode fix,
post-fix lint/verify, trace receipt counting, seal, and ZJX evidence. It shells
to the selected `sley` binary and never calls external providers.

The local migration report loop is backed by the in-tree `sley-migrate`
bootstrap. `sley-migrate report --json <target>` shells out to no providers
and uses the checked edit-plan template surface to report module declaration,
raw host adapter, imported-call naming, and unchecked-result propagation
migration candidates. With `--schemas <dir> --fixtures <dir>`, it also reports
schema IDs without fixture instances and fixture instances without matching
schema files. The report is advisory and does not write source files.

`sley new` is the v0 project scaffold command. It writes a `sley.toml`,
`README.md`, and entry module source file, refusing to overwrite any of those
paths when they already exist. `--template hello`, `--template library`,
`--template cli`, and `--template data-pipeline` create pure starters.
`--template service-gate` creates a deterministic `Network`-gated starter that
runs with seeded `http.try_get_text` data. `--template deploy` creates a
deterministic `Deploy`-gated starter that runs with a seeded
`deploy.try_stage` result and does not call providers or mutate
infrastructure. `--template agent` and `--template agent-task-pack` create
deterministic agentic starters that use seeded `secrets.try_get`,
`http.try_get_text`, `model.try_complete`, and `deploy.try_stage` calls under
explicit `SecretRead`, `Network`, `ModelCall`, and `Deploy` gates without
reading real secret stores, calling live networks or model providers, or
mutating deployment infrastructure. `--json` emits the scaffold report with
relative file paths, exact next-command vectors, and typed next-action reasons
under `sley.project.scaffold.v0`. The gated deploy, service, and agent
scaffolds' generated next actions are checked first-run sequences: strict
check, doctor readiness, task query, edit plan, lint gate, seeded verification
with denied warnings, seeded run, local deploy dry-run package where
applicable, seal, and ZJX package. The agent scaffolds also include
`sley-ci run`, `sley-ci verify`, and `sley-ci deploy --dry-run` next actions
over the same deterministic seeds.

`sley run --json` emits `sley.run.report.v0` on successful execution. The
report carries `status: "passed"`, the target path, a recursive runtime value
payload, and an empty diagnostics array; parse/check/runtime failures continue
to use the diagnostics report path.
`sley-ci run --json` wraps the same deterministic runtime execution under
`sley.ci.report.v0` for local CI and pre-commit gates.

`sley deploy --dry-run` is the v0 local deploy package command. It refuses to
run unless `--dry-run` is present. It consumes the same deterministic runtime
gates and seed flags as `sley run` and `sley verify`, runs strict verification
with denied warnings, and only when verification passes builds a trace seal and
ZJX package summary. Its JSON root is `sley.deploy.report.v0`. The report
records `live_deploy_allowed=false`, `external_mutations=false`,
`provider_calls=false`, and `requires_operator_approval=true`; it does not
upload artifacts, start services, push branches, call providers, spend money,
or mutate infrastructure. When `--artifacts-dir <dir>` is supplied and the
dry-run package is ready, the command writes local `deploy-report.json`,
`seal.json`, `zjx-envelope.json`, and `manifest.json` handoff files and records
their paths in the deploy report. The manifest records each artifact path,
schema, and content digest so another agent can verify the directory as one
local handoff bundle before any operator-approved deployment step.
`sley-contract inspect-deploy-artifacts <dir> --schemas docs/schemas --json`
revalidates that bundle later by checking the manifest schema, each artifact
schema, and each recorded file digest.

`sley doctor` is the first deterministic helper that consumes the strict
checker plus checked query and lint reports into one agent readiness report.
It reports `ready`, `warnings`, or `blocked`; includes source schema references
for the consumed query and lint surfaces; and gives next-command vectors for
task inspection, strict call-row inspection when checked calls exist, lint
gates, checked `sley plan --json --graft-templates` repair planning when lint
findings exist, non-mutating `sley fix --dry-run` previews when exactly one
checked lint repair exists, explicit optional `write_command` vectors for the
matching `sley fix --write`, and entrypoint runs.
`--deny-warnings` treats lint findings as blocked while still printing the
versioned report.

`sley plan` is the first deterministic pre-edit helper built from the same
strict checker, checked query, and checked lint surfaces. It reports `ready`,
`warnings`, or `blocked`; carries full lint findings; ranks task edit surfaces
with stable task ids, qualified names, takes, call counts, declared effects,
graft target ids, and planning notes; and gives next-command vectors for graph
slice inspection, strict call-row inspection when checked calls exist, plus
post-edit doctor and verify gates. `--deny-warnings` treats lint findings as
blocked while still printing
`schema: "sley.edit_plan.report.v0"`. `--graft-templates` adds starter strict
graft operation payloads for the highest-ranked task surface, plus JSON
pointers naming the fields an agent should edit before running
`sley graft --json --dry-run`; it also consumes the selected task graph slice
and adds `move_statement`/`move_take` templates plus destination variants from
checked `move_affordances`, and checked `delete_statement`/`delete_take`
templates from `delete_affordances` when the starter delete graft validates,
plus checked `replace_statement` and `replace_expression` templates from
`replace_affordances` when the starter replace graft validates. It also turns
checked
`unused_private_task` lint findings into `delete_unused_private_task`
`DeleteNode` templates when the task delete validates against the checked
candidate. Checked `unused_private_type` and `unused_private_effect` lint
findings become `DeleteNode` templates when the declaration delete validates
against the checked candidate. When more than one unused private type/effect
declaration can be deleted, the report also includes an all-or-nothing
`delete_unused_private_declarations` transaction template.
When lint proves multiple dead private tasks through `unused_private_task` or
`unreachable_private_task`, the report can include an all-or-nothing
`delete_dead_private_tasks` transaction template so disconnected cycles are
deleted together instead of one invalid intermediate task at a time.
`unused_take` lint findings become checked `remove_unused_take` `RemoveTake`
templates when the take can be removed without leaving invalid call sites.
`unused_declared_effect` lint findings carry exact `effect-use:<task>:<index>`
node ids and become checked `remove_unused_declared_effect`
`RemoveTaskEffect` templates when removing the declared effect preserves
authority correctness.
`unused_import` lint findings become checked `delete_unused_import`
`DeleteNode` templates when the import delete validates against the checked
candidate.
`missing_module_declaration` lint findings become checked
`add_module_declaration` graft templates on the `program` surface, with the
starter module name inferred from project-relative path context when available.
`raw_host_adapter` lint findings become checked
`migrate_raw_host_adapter` graft templates when the raw host call can be
rewritten to the fallible `try_` adapter with `?` and the checked candidate
still passes.
`unchecked_result` lint findings become checked
`propagate_unchecked_result` graft templates when the discarded `Result`
expression can be rewritten with `?` and the checked candidate still passes.
`unqualified_imported_call` lint findings become checked
`qualify_imported_call` graft templates when a simple imported task call can be
rewritten through its import alias or module segment and the checked candidate
still passes.
`unused_pure_binding` lint findings become checked
`delete_unused_pure_binding` `DeleteNode` templates when deleting the unread
binding statement preserves a checked program.
`unused_pure_expression_statement` lint findings become checked
`delete_unused_pure_expression_statement` `DeleteNode` templates when deleting
the no-op pure expression statement preserves a checked program.
`self_assignment_statement` lint findings become checked
`delete_self_assignment_statement` `DeleteNode` templates when deleting a
no-op `set name = name` mutation preserves a checked program.
`overwritten_set_statement` lint findings become checked
`delete_overwritten_set_statement` `DeleteNode` templates when deleting a
delete-safe `set` statement immediately overwritten before any read preserves a
checked program.
`redundant_initial_set_statement` lint findings become checked
`fold_redundant_initial_set_into_binding` transaction templates when replacing
the mutable binding initializer and deleting the immediate `set` preserves a
checked program while later mutation remains. When no later mutation remains,
they become checked `convert_redundant_initial_set_to_bind` transaction
templates that replace the mutable binding with `bind` and delete the
immediate `set`.
`constant_if_expression` lint findings become checked
`simplify_constant_if_expression` `ReplaceExpression` templates when replacing
an expression-level `if true/false` with the branch that executes preserves a
checked program.
`constant_if_statement` lint findings become checked
`simplify_constant_if_statement` `ReplaceStatement` templates when replacing a
statement-level `if true/false` with its single executing branch statement
preserves a checked program.
`constant_false_if_statement` lint findings become checked
`delete_constant_false_if_statement` `DeleteNode` templates when deleting a
never-executed `if false` statement without an `else` branch preserves a
checked program.
`constant_false_while_statement` lint findings become checked
`delete_constant_false_while_statement` `DeleteNode` templates when deleting a
never-executed `while false` statement preserves a checked program.
`constant_comparison_expression` lint findings become checked
`simplify_constant_comparison_expression` `ReplaceExpression` templates when
replacing a literal comparison with its boolean result preserves a checked
program.
`constant_arithmetic_expression` lint findings become checked
`simplify_constant_arithmetic_expression` `ReplaceExpression` templates when
replacing safe numeric literal arithmetic with its result preserves a checked
program.
`absorbing_arithmetic_expression` lint findings become checked
`simplify_absorbing_arithmetic_expression` `ReplaceExpression` templates when
replacing delete-safe multiplication by zero with the zero literal preserves a
checked program.
`constant_text_concatenation_expression` lint findings become checked
`simplify_constant_text_concatenation_expression` `ReplaceExpression`
templates when replacing text literal concatenation with one escaped text
literal preserves a checked program.
`constant_list_index_expression` lint findings become checked
`simplify_constant_list_index_expression` `ReplaceExpression` templates when
replacing an in-range literal list index over scalar literals with the selected
literal preserves a checked program.
`constant_map_index_expression` lint findings become checked
`simplify_constant_map_index_expression` `ReplaceExpression` templates when
replacing a present literal map key over scalar literal values with the selected
literal preserves a checked program.
`constant_record_field_access_expression` lint findings become checked
`simplify_constant_record_field_access_expression` `ReplaceExpression`
templates when replacing a present literal record field access over scalar
literal values with the selected literal preserves a checked program.
`constant_len_expression` lint findings become checked
`simplify_constant_len_expression` `ReplaceExpression` templates when
replacing a literal `len` call over literal text, list, or map values with the
literal length preserves a checked program.
`constant_not_expression` lint findings become checked
`simplify_constant_not_expression` `ReplaceExpression` templates when
replacing literal boolean negation with the resulting boolean literal preserves
a checked program.
`empty_if_statement` lint findings become checked
`delete_empty_if_statement` `DeleteNode` templates when deleting a no-op `if`
statement with a delete-safe condition and empty branches preserves a checked
program.
`empty_for_statement` lint findings become checked
`delete_empty_for_statement` `DeleteNode` templates when deleting a `for`
statement over a literal empty list preserves a checked program.
`empty_forge_statement` lint findings become checked
`delete_empty_forge_statement` `DeleteNode` templates when deleting a no-op
`forge { }` starter block preserves a checked program.
`identity_binary_expression` lint findings become checked
`simplify_identity_binary_expression` `ReplaceExpression` templates when
replacing `x + 0`, `x * 1`, `flag && true`, `flag || false`, or empty-text
concatenation with the non-identity side preserves a checked program.
`redundant_boolean_comparison` lint findings become checked
`simplify_redundant_boolean_comparison` `ReplaceExpression` templates when
replacing comparisons against `true` or `false` with the boolean expression or
its negation preserves a checked program.
`absorbing_boolean_expression` lint findings become checked
`simplify_absorbing_boolean_expression` `ReplaceExpression` templates when
replacing short-circuiting absorbing boolean expressions with `true` or `false`
preserves a checked program and any removed evaluated side is delete-safe.
`self_comparison_expression` lint findings become checked
`simplify_self_comparison_expression` `ReplaceExpression` templates when
replacing delete-safe `expr == expr`, `expr != expr`, `expr < expr`,
`expr <= expr`, `expr > expr`, or `expr >= expr` comparisons with `true` or
`false` preserves a checked program.
`double_negation_expression` lint findings become checked
`simplify_double_negation_expression` `ReplaceExpression` templates when
replacing `!!expr` with the inner boolean expression preserves a checked
program.
`negated_comparison_expression` lint findings become checked
`simplify_negated_comparison_expression` `ReplaceExpression` templates when
replacing `!(left op right)` with the inverse comparison preserves a checked
program.
`redundant_boolean_if_expression` lint findings become checked
`simplify_redundant_boolean_if_expression` `ReplaceExpression` templates when
replacing `if flag { true } else { false }` with `flag`, or the inverted form
with `!flag`, preserves a checked program.
`redundant_boolean_if_statement` lint findings become checked
`simplify_redundant_boolean_if_statement` `ReplaceStatement` templates when
replacing `if flag { return true } else { return false }` with `return flag`,
or the inverted form with `return !flag`, preserves a checked program.
`same_branch_if_expression` lint findings become checked
`simplify_same_branch_if_expression` `ReplaceExpression` templates when
replacing `if condition { value } else { value }` with `value` preserves a
checked program and the removed condition is delete-safe.
`same_branch_if_statement` lint findings become checked
`simplify_same_branch_if_statement` `ReplaceStatement` templates when replacing
an `if` statement whose branches contain the same single statement preserves a
checked program and the removed condition is delete-safe.
`unreachable_statement` lint findings become checked
`delete_unreachable_statement` `DeleteNode` templates when deleting a
statement after a guaranteed return preserves a checked program.
`--template-surface program` emits checked `add_task`,
`add_type_declaration`, `add_effect_declaration`, and `add_import` starters for
adding new declarations and imports to the current program module.
`--template-surface <surface>` selects a specific task surface by task node id
or qualified task name, a block node id backed by graph-slice insert
affordances, a statement node id for direct checked graph-slice move/delete and
`replace_statement` templates, a take node id for direct checked graph-slice
move/delete templates, an expression node id for a checked no-op
`replace_expression` starter template, the `program` declaration/import and
missing-module surface, or a lint surface by lint finding node id such as
`import:app.main:app.stale`,
`type:app.module.Name`, or `effect:app.module.Name`. For selected tasks with
currently resolved inbound callers, the report also includes an
all-or-nothing `RenameDeclaration` plus `UpdateCallSites` transaction template
and an `AddTake` plus `UpdateCallArgs` transaction template. When the selected
task has an unused normal take, `sley plan --graft-templates` can also emit a
`RemoveTake` plus `RemoveCallArg` transaction template. `--emit-graft <kind>`
prints one matching operation or transaction JSON directly, so agents can save
it as the graft input for `sley graft --json --dry-run` or
`sley graft --json --write`; ambiguous or missing kinds reject instead of
guessing.

`sley fix --kind <kind>` is the first deterministic plan-consuming fixer. It
builds checked plan graft templates internally, selects exactly one named
operation or transaction, applies it through the same graft checker, and emits
the normal `sley.graft.outcome.v0` root. Exact block, statement, take, and
expression node surfaces, plus the `program` declaration/import surface, can be
selected with `--template-surface` and executed without hand-authoring graft
JSON. Single-operation templates can also accept
`--name <name>`, `--type <type>`, `--module <module>`,
`--source <source>`, `--source-file <path>`, and `--position <n>` overrides
when their editable payload fields expose `/payload/name`, `/payload/type`,
`/payload/module`, `/payload/source`, or `/payload/position`; unsupported
overrides reject with
`FIX_OVERRIDE_UNSUPPORTED` before any write path. Default and `--dry-run` mode
are non-mutating.
`--write` uses the same project or file writeback and trace receipt path as
`sley graft --write`, including explicit
`--trace <trace.jsonl>` receipt redirection. The CLI smoke surface covers a
call-row-driven `rename_and_update_call_sites` write followed by strict call
query and verification, plus `remove_take_and_remove_call_arg` write, query,
and verify for unused-take cleanup that must also rewrite resolved callers,
plus `qualify_imported_call` write/query/verify coverage that proves imported
call style cleanup clears strict lint.

A graph slice is a bounded shard around a module, task, type, effect, or import.
Task slices include the selected task AST, visible module declarations,
outbound calls, and inbound calls from other tasks that resolve to the selected
task. This is the first stable agent-facing query surface for graft planning.
Import summaries expose canonical import node ids such as
`import:app.main:app.shared`, so import `MoveNode` and `DeleteNode` grafts can
copy targets directly from graph, graph-slice, or query JSON. Graph slices also
include `insert_affordances` for task-local block insertion planning: each
affordance exposes the exact block target, maximum insertion position, starter
`InsertStatement` operation JSON, and editable `/payload/source` plus
`/payload/position` pointers. Graph slices also include `move_affordances` for
bounded import, type, effect, task, statement, and take movement planning: each
affordance exposes the exact `MoveNode`
target, current parent, current position, in-parent maximum position, and
available destination parents with their insertion limits. Affordances and
destination entries also carry strict starter `MoveNode` operation JSON plus
editable JSON pointers so agents can copy a template, adjust
`/payload/position`, and dry-run the graft. Graph slices also include
`delete_affordances` for import, type, effect, task, statement, and take
`DeleteNode` planning; each delete affordance exposes the exact target, current
parent, current position, starter operation JSON, and editable pointer list.
Graph slices also include task-local `replace_affordances` for
`ReplaceStatement` and `ReplaceExpression` planning; each replace affordance
exposes the exact statement or expression target, target kind, parent node id,
starter operation JSON, and editable `/payload/source` pointer.
Call-site, statement, and expression grafts now consume node ids and task
identities from this shard.

`sley query` is the first checked graph query report. It parses and checks the
target before emitting results, so semantic failures return the normal
diagnostic report instead of a partial query.
`--kind all|modules|tasks|types|effects|calls` selects the report body.
`--module <module>` filters module summaries, task/type/effect summaries, and
calls that originate from or resolve into that module.
`--exported` restricts declaration and task summaries to exported declarations.
Task query rows include stable ids, qualified names, takes, return type text,
declared effects, and inbound/outbound call counts. Type query rows include
stable ids, qualified names, rendered type values, and record fields; effect
query rows include stable ids and qualified names. Call query rows include
stable caller, expression, source, callee, status, target, and candidate
fields. The v0 JSON root is `sley.query.report.v0`.

`sley lint` is the first checked lint command built on the graph query surface.
It parses and checks the target before emitting results, so semantic failures
return the normal diagnostic report instead of a lint report. The v0 rules are
`unused_private_task`, which warns on non-exported tasks with zero inbound
checked calls excluding the entry module's `main`; `unreachable_private_task`,
which warns on private task islands that are only reachable from other private
tasks rather than from `main` or an exported task; and
`unused_declared_effect`, which warns when a task declares an effect that no
direct host call or resolved called-task effect justifies and points at an
exact `effect-use:<task>:<index>:<effect>` node; `unused_import`,
which warns when an import is not needed by any checked task, type, or effect;
`unused_take`, which warns when a normal task take is never read by the task
body; `unused_private_type`, which warns when a non-exported type is not
referenced by any checked task, type, or record literal;
`unused_private_effect`, which warns when a non-exported effect is not declared
by any checked task; `raw_host_adapter`, which warns on legacy
diagnostic-failing host calls that should move to fallible `try_` adapters;
`missing_module_declaration`, which warns when a source file relies on the
implicit `main` module instead of declaring a stable module name;
`unchecked_result`, which warns when an expression statement discards a
fallible host or user-task `Result` instead of propagating, returning, or
binding it; and `unqualified_imported_call`, which warns when a resolved call
uses a simple imported task name instead of an alias- or module-qualified
callee; and `unused_pure_binding`, which warns when a local `bind` statement
has an unread delete-safe initializer with no calls, raw fragments, `?`,
indexing, division, or remainder operation; and
`unused_pure_expression_statement`, which warns when a statement evaluates a
delete-safe pure expression and discards it; and `mutable_binding_never_set`,
which warns when a mutable local such as `state` or `tally` is never assigned
with `set`; and `self_assignment_statement`, which warns when a no-op
`set name = name` mutation can be deleted; and `overwritten_set_statement`,
which warns when a delete-safe `set name = ...` statement is immediately
overwritten by another `set name = ...` before any read; and
`redundant_initial_set_statement`, which warns when a mutable local initializer
is immediately replaced by a delete-safe first `set` before any read while
later real mutation remains; and
`constant_if_expression`, which warns when an expression-level
`if true/false { ... } else { ... }` can be replaced with the branch that
executes; and `constant_if_statement`, which warns when a statement-level
`if true/false { ... } else { ... }` can be replaced by its single executing
branch statement; and
`constant_false_if_statement`, which warns when a
never-executed `if false { ... }` statement without an `else` branch can be
deleted; and `constant_false_while_statement`, which warns when a
`while false { ... }` statement can be removed as never-executed code; and
`constant_comparison_expression`, which warns when a checked literal comparison
can be replaced by its boolean result; and
`constant_arithmetic_expression`, which warns when checked numeric literal
arithmetic can be replaced by its result without overlapping identity cleanup
or folding divide-by-zero; and
`absorbing_arithmetic_expression`, which warns when checked multiplication by
zero can be replaced by the zero literal without dropping authority work or
recoverable failures; and
`constant_text_concatenation_expression`, which warns when checked text literal
concatenation can be replaced by one escaped text literal; and
`constant_list_index_expression`, which warns when an in-range literal list
index over scalar literals can be replaced by the selected literal without
dropping runtime work; and
`constant_map_index_expression`, which warns when a present literal map key
over scalar literal values can be replaced by the selected literal without
dropping runtime work; and
`constant_record_field_access_expression`, which warns when a present literal
record field access over scalar literal values can be replaced by the selected
literal without dropping runtime work; and
`constant_len_expression`, which warns when a literal `len` call over literal
text, list, or map values can be replaced by the literal length without
dropping runtime work; and
`constant_not_expression`, which warns when literal boolean negation can be
replaced by the resulting boolean literal without dropping runtime work; and
`empty_if_statement`, which warns when an `if` statement with a delete-safe
condition and empty branches can be removed as no-op control flow; and
`empty_for_statement`, which warns when a `for item in [] { ... }` statement
can be removed as never-executed code; and
`empty_forge_statement`, which warns when a no-op `forge { }` statement can be
removed before readiness or deploy gates; and
`identity_binary_expression`, which warns when a checked identity
binary expression such as `x + 0`, `x * 1`, `flag && true`, `flag || false`,
`"" + name`, or `name + ""` can be replaced with the non-identity side; and
`redundant_boolean_comparison`, which warns when a checked comparison against
`true` or `false` can be replaced with the boolean expression or its negation;
and `absorbing_boolean_expression`, which warns when a checked
short-circuiting absorbing boolean expression can be replaced with `true` or
`false` without dropping a side that would still be evaluated;
and `self_comparison_expression`, which warns when a checked delete-safe
self-comparison, including strict and non-strict self-ordering, can be replaced
with `true` or `false`;
and `double_negation_expression`, which warns when a checked `!!expr` form can
be replaced with the inner boolean expression; and
`negated_comparison_expression`, which warns when a checked negated comparison
can be replaced with its inverse comparison; and
`redundant_boolean_if_expression`, which warns when a checked expression-level
boolean `if` can be replaced with the condition or its negation; and
`redundant_boolean_if_statement`, which warns when a checked statement-level
boolean `if` can return the condition or its negation directly; and
`same_branch_if_expression`, which warns when a checked expression-level `if`
has identical branches and a delete-safe condition.
`same_branch_if_statement`, which warns when a checked statement-level `if`
has identical single-statement branches and a delete-safe condition.
`unreachable_statement`, which warns when a statement appears after a
guaranteed `return` in the same block.
`--module <module>` scopes the lint to one module. `--rule unused-private-task`,
`--rule unreachable-private-task`, `--rule unused-declared-effect`,
`--rule unused-import`, `--rule unused-take`, `--rule unused-private-type`,
`--rule unused-private-effect`, `--rule raw-host-adapter`,
`--rule missing-module-declaration`, `--rule unchecked-result`, or
`--rule unqualified-imported-call`, `--rule unused-pure-binding`, or
`--rule unused-pure-expression-statement`, or
`--rule mutable-binding-never-set`, `--rule self-assignment-statement`, or
`--rule overwritten-set-statement`, or
`--rule redundant-initial-set-statement`, or `--rule constant-if-expression`, or
`--rule constant-if-statement`, or
`--rule constant-false-if-statement`, or
`--rule constant-false-while-statement`, or
`--rule constant-comparison-expression`, or
`--rule constant-arithmetic-expression`, or
`--rule absorbing-arithmetic-expression`, or
`--rule constant-text-concatenation-expression`, or
`--rule constant-list-index-expression`, or
`--rule constant-map-index-expression`, or
`--rule constant-record-field-access-expression`, or
`--rule constant-len-expression`, or
`--rule constant-not-expression`, or
`--rule empty-if-statement`, or
`--rule empty-for-statement`, or
`--rule empty-forge-statement`, or
`--rule identity-binary-expression`, or
`--rule redundant-boolean-comparison`, or
`--rule absorbing-boolean-expression`, or `--rule self-comparison-expression`, or
`--rule double-negation-expression`, or
`--rule negated-comparison-expression`, or
`--rule redundant-boolean-if-expression`, or
`--rule redundant-boolean-if-statement`, or
`--rule same-branch-if-expression`, or
`--rule same-branch-if-statement`, or
`--rule unreachable-statement`, or
`--rule absorbing-arithmetic-expression` selects one rule explicitly, and
`--deny-warnings` turns findings into a nonzero CLI exit after printing the
report. The v0 JSON root is `sley.lint.report.v0`.

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
recomputable graph digests, optional graph slices, and trace receipts; it is
not yet a compressed `.zjx` archive.

The current `sley-zjx` utility is read-only for source files and preview
envelopes. It can inspect envelope metadata, recompute and verify the embedded
symbol-graph digest, extract the graph JSON to stdout or an explicit output
path, and diff two envelopes by target, digest, module IDs, task IDs, and trace
receipt count. Its report root is `sley.zjx.tool.report.v0`.

The future compressed Sley runtime must validate Sley structure before packing.
It must not treat envelope metadata as proof of Sley scope. The lock boundary
is specified in `docs/SleyZjxRuntimeLockSpec.md`: reconstruct Sley graph
structure, recompute the graph hash, reject unknown or opaque fields, and
refuse generic data disguised as Sley artifacts before compression begins.

## Current Gaps

- runtime gates currently back filesystem text reads/writes, deterministic
  seeded database reads, per-run deterministic database inserts, seeded secret
  values, seeded deployment stage results, seeded spend authorizations, seeded
  network text responses, seeded shell command outputs, and seeded model prompt
  completions; non-file seeded adapters now support deterministic text-prefix
  scopes over their seeded resource keys
- Sley-level `Result` values, `?` propagation, and typed filesystem,
  database, secret, deploy, spend, network, shell, and model host fallibility
  execute
- trace receipts can be sealed, but sidecar storage is not yet a compressed ZJX
  archive
- the AST JSON Schema covers nested AST and expression variants; the remaining
  JSON Schema files are still narrower v0 root contracts
- `sley lint` currently ships warning-grade private-task graph rules, authority
  hygiene for unused declared effects, private declaration/import/API hygiene,
  raw-host-adapter and unchecked-result migration warnings with checked
  propagation templates, unqualified imported-call qualification templates,
  unused pure binding cleanup templates, unused pure expression statement
  cleanup templates, self-assignment statement cleanup templates, overwritten
  set statement cleanup templates,
  constant-false if and while statement cleanup templates,
  empty-if statement cleanup templates,
  empty-for statement cleanup templates, empty-forge statement cleanup
  templates, redundant boolean-if statement cleanup templates,
  unreachable statement cleanup templates, and mutable
  binding conversion transactions, constant-if expression and statement
  simplification, constant arithmetic and absorbing arithmetic simplification,
  identity binary expression including empty-text concatenation,
  redundant boolean
  comparison, double negation simplification, negated comparison
  simplification, and redundant boolean-if expression/statement simplification
  templates, plus explicit module style warnings;
  broader style and migration lints remain later work
- `sley plan` emits deterministic ranked task edit surfaces, optional starter
  graft operation templates, rename-plus-call-site and add-take-plus-call-arg
  transaction templates, safe remove-take-plus-call-arg transaction templates
  for unused takes, lint-driven delete templates and cleanup transactions for
  unused private types/effects, lint-driven module declaration templates with
  inferred module names, and post-edit gate commands; `sley fix` can execute
  one named checked template by explicit kind, including missing-module
  declaration repair, but broad autonomous repair selection remains later work
- `sley verify` emits a deterministic CI/pre-deploy report over strict check,
  query/lint summaries, and runtime execution; warning and denied-warning
  reports route agents to checked lint repair planning and dry-run fix previews
  before deployment review when the repair is unambiguous; preview actions keep
  `command` dry-run and add optional `write_command` for the mutating command.
  The CLI smoke suite locks previewed lint-repair writes followed by strict
  verify for file and project targets, including unused pure binding cleanup,
  unused pure expression statement cleanup, constant-false while statement
  cleanup, constant-if statement simplification, constant-false if statement
  cleanup, constant arithmetic expression cleanup, absorbing arithmetic
  expression cleanup, constant text concatenation cleanup, constant list index
  cleanup,
  constant map index cleanup, constant record field access cleanup,
  constant len cleanup,
  constant not cleanup,
  empty-if statement cleanup,
  empty-for statement cleanup,
  empty-forge statement cleanup,
  unreachable statement cleanup, and mutable binding conversion,
  plus generated scaffold quickstarts re-verified with local or seeded
  authority, including strict seeded verify reports for `service-gate`,
  `deploy`, `agent`, and `agent-task-pack`, passed-verify next-actions for
  `sley seal --json` and `sley zjx --json` handoff artifacts, and
  `sley deploy --dry-run` local deploy package reports over verify, seal, and
  ZJX summaries where applicable; live deploy/provider calls remain outside v0
- no `match`, agent declarations, spawn/cast/join, or compressed ZJX archive
  writer yet
- `MoveNode` supports checked in-parent and cross-parent statement movement
  between existing block parents, take reordering within the owning task,
  checked cross-task take movement, top-level declaration ordering, and
  top-level import/type/effect/task movement into known modules; expression
  movement is still an explicit rejection
- project graft writeback updates existing module files, adds imports to
  existing on-disk module files that were not yet loaded through the entry
  import graph, creates checked new module files, deletes removed module files,
  renames module files, and updates the project manifest for entry-module
  renames
