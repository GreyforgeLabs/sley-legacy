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
- `sley query`
- `sley lint`
- `sley doctor`
- `sley plan`
- `sley verify`
- `sley trace`
- `sley seal`
- `sley zjx`
- `sley graft`
- `sley new`
- explicit non-mutating `sley graft --dry-run`; plain graft preview remains
  non-mutating unless `--write` is supplied
- `sley.toml` project manifests for multi-file module graphs
- non-destructive `sley new` project scaffolding with `hello` and
  deterministic `deploy` starter templates
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
  `row.bool`, and `row.get`; `DatabaseWrite` backs `db.try_insert` for
  deterministic per-run table mutation
- deterministic secret host seeding with `sley run --cap SecretRead --secret
  NAME TEXT`; `secrets.try_get` returns seeded secret values as
  `Result<Text, Error>` without reading environment variables, keyrings, or
  real secret stores
- deterministic deploy host seeding with `sley run --cap Deploy
  --deploy-result TARGET TEXT`; `deploy.try_stage` returns seeded deployment
  stage results as `Result<Text, Error>` without uploading, starting services,
  or calling deployment providers
- deterministic spend host seeding with `sley run --cap Spend --spend-result
  REQUEST TEXT`; `spend.try_authorize` returns seeded spend authorization
  results as `Result<Text, Error>` without transactions, payments, market
  orders, wallet calls, credits, or provider spend
- deterministic network host seeding with `sley run --cap Network --http-text
  URL TEXT`; `http.try_get_text` returns seeded text responses as
  `Result<Text, Error>` without live outbound network access
- deterministic shell host seeding with `sley run --cap Shell --shell-output
  COMMAND TEXT`; `shell.try_run` returns seeded command output as
  `Result<Text, Error>` without executing subprocesses
- deterministic model host seeding with `sley run --cap ModelCall
  --model-output PROMPT TEXT`; `model.try_complete` returns seeded completions
  as `Result<Text, Error>` without provider calls
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
- checked JSON query reports with `sley query --kind all|modules|tasks|calls`,
  optional `--module` and `--exported` filters, and
  `schema: "sley.query.report.v0"` for tool-facing graph inspection
- checked JSON lint reports with `sley lint`, optional `--module` and
  `--rule unused-private-task`, `--rule unreachable-private-task`, or
  `--rule unused-declared-effect`, or `--rule raw-host-adapter` filters, and
  `schema: "sley.lint.report.v0"` for warning-grade graph and authority lints
- checked JSON readiness reports with `sley doctor`, consuming strict
  diagnostics plus query and lint summaries, and
  `schema: "sley.doctor.report.v0"` for agent pre-edit gates
- checked JSON edit-plan reports with `sley plan`, consuming strict
  diagnostics plus query and lint findings into ranked task edit surfaces,
  graft target ids, and `schema: "sley.edit_plan.report.v0"`
- checked JSON verification reports with `sley verify`, consuming strict
  diagnostics, query summaries, lint findings, and deterministic runtime
  execution into `schema: "sley.verify.report.v0"` for CI and pre-deploy gates
- JSON project scaffold reports with `sley new --json`, `hello` and `deploy`
  templates, relative created-file paths, next-command vectors, and
  `schema: "sley.project.scaffold.v0"`
- JSONL trace sidecars for accepted graft receipts when `sley graft --write`
  applies a change, plus content-addressed trace seals with `sley seal`
- project-aware `sley graft --write <project>` source-file writeback for
  accepted edits that resolve to existing modules, plus checked creation of new
  module files declared by the graft candidate, deletion of removed module
  files no longer referenced by the checked candidate, and module rename
  writeback with manifest entry updates when needed
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
- locked JSON contract snapshots under `fixtures/contracts/`, including
  checked query, lint, doctor, edit-plan, verify, and project scaffold reports
- external v0 JSON Schema files under `docs/schemas/`
- manifest-backed accepted/rejected synthetic conformance corpus fixtures under
  `fixtures/corpus/`, including declared and missing authority cases for the
  seeded host adapter surface
- manifest-backed CLI smoke conformance cases under `fixtures/cli_smokes/`,
  covering stable command output, JSON roots, graph/ZJX surfaces, doctor
  readiness, edit-plan surfaces, verify pre-deploy gates, project scaffolding,
  graft dry runs, and seeded host-adapter execution
- compact agent onboarding pack in `llms.txt`

Project form:

```bash
sley new --template deploy --name agent-app agent-app
cd agent-app
sley check --json .
sley doctor --json .
sley plan --json .
sley verify --json --cap Deploy --deploy-result staging staged .
sley lint --json --deny-warnings .
sley run --json --cap Deploy --deploy-result staging staged .
```

`sley new` refuses to overwrite existing `sley.toml`, `README.md`, or entry
source files. `--template hello` creates a pure starter project.
`--template deploy` creates a deterministic deployment-gated starter that
uses the seeded `deploy.try_stage` adapter; it does not call deployment
providers or mutate infrastructure.

```toml
[project]
name = "module-demo"
root = "src"
entry = "app.main"
```

Module `app.main` resolves to `src/app/main.sley`. `sley parse`,
`sley check`, `sley run`, `sley ast`, `sley graph`, `sley query`,
`sley lint`, `sley doctor`, `sley plan`, `sley verify`, `sley seal`,
`sley zjx`, and `sley graft` accept either a single `.sley` file or a project
directory containing `sley.toml`. Project graft
writeback projects the checked candidate back to existing owning module files
and leaves unchanged module files alone. It can create checked new module
files, delete removed loaded module files, rename module files, and update
`sley.toml` when the manifest entry module is renamed.

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
Fallible variants `db.try_query_one` and `db.try_query` return
`Result<DbRow, Error>` and `Result<List<DbRow>, Error>`.
`DatabaseWrite` backs `db.try_insert(table, row)`, which accepts a record or
map row, creates the per-run table if needed, inserts the row, and returns
`Result<DbRow, Error>`:

```sley
task main -> Result<Text, Error> uses DatabaseWrite, DatabaseRead {
  bind inserted = call db.try_insert("users", { id: "u3", name: "Lin" })?
  bind row = call db.query_one("select * from users where id = ?", inserted.text("id"))
  return Ok(row.text("name"))
}
```

Seeded secret runtime:

```bash
sley run --json --cap SecretRead --secret api_key redacted examples/secret_gate.sley
```

`secrets.try_get(name)` reads an exact seeded secret value and returns
`Result<Text, Error>`. This is a deterministic v0 host adapter, not a real
secret backend. Missing secret seeds produce `RUNTIME_SECRET_NOT_FOUND` as a
typed host error; empty names produce `RUNTIME_SECRET_NAME_INVALID`; missing
`SecretRead` remains a runtime capability diagnostic.

```sley
task main -> Result<Text, Error> uses SecretRead {
  bind value = call secrets.try_get("api_key")?

  return Ok(value)
}
```

Seeded deploy runtime:

```bash
sley run --json --cap Deploy --deploy-result staging staged examples/deploy_gate.sley
```

`deploy.try_stage(target)` reads an exact seeded deployment stage result and
returns `Result<Text, Error>`. This is a deterministic v0 host adapter, not a
deployment client. It does not upload artifacts, push branches, start services,
mutate infrastructure, or call providers. Missing target seeds produce
`RUNTIME_DEPLOY_RESULT_NOT_FOUND` as a typed host error; empty targets produce
`RUNTIME_DEPLOY_TARGET_INVALID`; missing `Deploy` remains a runtime capability
diagnostic.

```sley
task main -> Result<Text, Error> uses Deploy {
  bind result = call deploy.try_stage("staging")?

  return Ok(result)
}
```

Seeded spend runtime:

```bash
sley run --json --cap Spend --spend-result ads-budget authorized examples/spend_gate.sley
```

`spend.try_authorize(request)` reads an exact seeded spend authorization result
and returns `Result<Text, Error>`. This is a deterministic v0 host adapter, not
a payment, broker, wallet, cloud-billing, or market-order client. It does not
move money, buy credits, place orders, call providers, or mutate external
accounts. Missing request seeds produce `RUNTIME_SPEND_RESULT_NOT_FOUND` as a
typed host error; empty requests produce `RUNTIME_SPEND_REQUEST_INVALID`;
missing `Spend` remains a runtime capability diagnostic.

```sley
task main -> Result<Text, Error> uses Spend {
  bind result = call spend.try_authorize("ads-budget")?

  return Ok(result)
}
```

Seeded network runtime:

```bash
sley run --json --cap Network --http-text https://example.test/profile Ada examples/network_gate.sley
```

`http.try_get_text(url)` reads an exact seeded URL response and returns
`Result<Text, Error>`. This is a deterministic v0 host adapter, not a live HTTP
client. Missing response seeds produce `RUNTIME_HTTP_RESPONSE_NOT_FOUND` as a
typed host error; empty URLs produce `RUNTIME_HTTP_URL_INVALID`; missing
`Network` remains a runtime capability diagnostic.

```sley
task main -> Result<Text, Error> uses Network {
  bind body = call http.try_get_text("https://example.test/profile")?

  return Ok(body)
}
```

Seeded shell runtime:

```bash
sley run --json --cap Shell --shell-output date 2026-05-05 examples/shell_gate.sley
```

`shell.try_run(command)` reads an exact seeded command output and returns
`Result<Text, Error>`. This is a deterministic v0 host adapter, not a
subprocess runner. Missing output seeds produce `RUNTIME_SHELL_OUTPUT_NOT_FOUND`
as a typed host error; empty commands produce
`RUNTIME_SHELL_COMMAND_INVALID`; missing `Shell` remains a runtime capability
diagnostic.

```sley
task main -> Result<Text, Error> uses Shell {
  bind output = call shell.try_run("date")?

  return Ok(output)
}
```

Seeded model runtime:

```bash
sley run --json --cap ModelCall --model-output name Ada examples/model_gate.sley
```

`model.try_complete(prompt)` reads an exact seeded prompt completion and
returns `Result<Text, Error>`. This is a deterministic v0 host adapter, not a
provider client. Missing output seeds produce `RUNTIME_MODEL_OUTPUT_NOT_FOUND`
as a typed host error; empty prompts produce
`RUNTIME_MODEL_PROMPT_INVALID`; missing `ModelCall` remains a runtime
capability diagnostic.

```sley
task main -> Result<Text, Error> uses ModelCall {
  bind answer = call model.try_complete("name")?

  return Ok(answer)
}
```

Result flow:

```sley
task main -> Result<Int, Error> {
  bind value = Ok(41)?
  return Ok(value + 1)
}
```

`Ok(value)` and `Err(error)` are real runtime values. `expr?` unwraps `Ok` and
propagates `Err` as the current task's result. Host adapters now expose typed
fallible variants for recoverable failures:

```sley
task load -> Result<Text, Error> uses FileRead {
  take path: Text

  bind text = fs.try_read_text(path)?
  return Ok(text)
}
```

The standard runtime `Error` payload is a record with `code: Text` and
`message: Text`. `fs.try_read_text`, `fs.try_write_text`, `db.try_query_one`,
`db.try_query`, `db.try_insert`, `http.try_get_text`, `shell.try_run`,
`model.try_complete`, `secrets.try_get`, `deploy.try_stage`, and
`spend.try_authorize` return
`Result<T, Error>`. Missing
capabilities and gate scope violations remain diagnostics because they are
authority failures, not recoverable host values. The legacy raw adapters still
return their direct values and keep diagnostic failure behavior.

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
  root-scoped filesystem handlers, `DatabaseRead` has a deterministic
  seeded-table adapter, and `DatabaseWrite` has a deterministic per-run insert
  adapter. `Network` has a deterministic seeded text adapter, `Shell` has a
  deterministic seeded command-output adapter, and `ModelCall` has a
  deterministic seeded prompt-completion adapter. `SecretRead` has a
  deterministic seeded secret-value adapter. The `Deploy` adapter returns
  deterministic seeded stage results, and the `Spend` adapter returns
  deterministic seeded authorization results. None of the seeded host adapters
  perform live network, shell, model, secret, deploy, or spend actions.
- Sley-level `Result` values and `?` propagation execute, and fallible
  filesystem, database, network, shell, model, secret, deploy, and spend host
  variants return typed `Error` records.
- Project graft writeback supports existing module files, checked new module
  file creation under the project source root, deletion of removed module
  files once they are no longer imported, module rename writeback, and
  `sley.toml` entry updates when the entry module is renamed. Grafts that import
  modules without adding checked declarations for those modules still reject
  before any source or trace mutation.
- The AST JSON Schema now covers declarations, statements, expressions, type
  expressions, spans, and provenance recursively. Other external JSON Schema
  files remain narrower v0 root-contract schemas.
- `MoveNode` currently reorders statements within their existing block,
  moves statements across existing block parents with `payload.destination`,
  reorders takes within their owning task, reorders top-level imports, types,
  effects, or tasks within their declaration lists, and moves top-level types,
  effects, or tasks into known module parents such as `module:app.extra:tasks`.
  An all-or-nothing transaction can add the destination import before the move
  and create the new module file from the moved declaration. Cross-task take
  movement and expression movement still reject explicitly; expression movement
  rejection carries a `replace_expression` repair hint.

The current release-readiness phase is underway for the executable slice. The
gold corpus and CLI smoke suite now have manifests with required coverage tags
for seeded host adapters, stable JSON roots, graph/ZJX output, graft dry runs,
checked graph query reports, doctor readiness, verify pre-deploy gates,
edit-plan surfaces, private-task lint rules, authority hygiene, and raw-host
migration warnings. The next logical phase is to broaden style and migration
lints before broadening the language again.
