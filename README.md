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
- `sley fix`
- `sley verify`
- `sley deploy`
- `sley trace`
- `sley seal`
- `sley zjx`
- `sley graft`
- `sley new`
- explicit non-mutating `sley graft --dry-run`; plain graft preview remains
  non-mutating unless `--write` is supplied
- `sley.toml` project manifests for multi-file module graphs
- non-destructive `sley new` project scaffolding with `hello`, `library`,
  `cli`, deterministic `service-gate`, `data-pipeline`, deterministic
  `deploy`, deterministic `spend-gate`, deterministic `agent`,
  deterministic `agent-task-pack`, and multi-module deterministic
  `agent-project` starter templates
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
- runtime gates with `sley run --cap EFFECT[=SCOPE]`; effectful tasks reject
  without matching gates, `Gate<Effect>` takes are injected at runtime, and
  `fs.read_text`/`fs.write_text` are backed by root-scoped file capabilities
  while non-file seeded adapters can be narrowed by deterministic text scopes
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
- JSON AST output with `schema: "sley.ast.program.v0"` plus bounded
  `sley ast --node <node-id>` reports with `schema: "sley.ast.node.v0"` for
  task, take, block, statement, and expression inspection before grafting
- JSON module symbol graph output for module imports and exported declarations,
  plus bounded graph slices with `sley graph --slice <node-id>`; symbol graph
  and graph-slice JSON carry v0 schema IDs, including schema-linked focus,
  task, and call payloads plus insert, move, delete, and replace affordances
  for structural graft planning
- checked JSON query reports with
  `sley query --kind all|modules|tasks|types|effects|calls`, optional
  `--module` and `--exported` filters, strict task/take/type/effect/call row
  definitions, and `schema: "sley.query.report.v0"` for tool-facing graph
  inspection
- checked JSON lint reports with `sley lint`, optional `--module` and
  `--rule unused-private-task`, `--rule unreachable-private-task`,
  `--rule unused-declared-effect`, `--rule unused-import`,
  `--rule unused-take`, `--rule unused-private-type`,
  `--rule unused-private-effect`, `--rule raw-host-adapter`,
  `--rule missing-module-declaration`, `--rule unchecked-result`, or
  `--rule unqualified-imported-call`, `--rule unused-pure-binding`,
  `--rule unused-pure-expression-statement`, or
  `--rule mutable-binding-never-set`, `--rule self-assignment-statement`,
  `--rule overwritten-set-statement`,
  `--rule redundant-initial-set-statement`, `--rule constant-if-expression`,
  `--rule constant-if-statement`,
  `--rule constant-false-if-statement`,
  `--rule constant-false-while-statement`,
  `--rule constant-comparison-expression`,
  `--rule constant-arithmetic-expression`,
  `--rule constant-text-concatenation-expression`,
  `--rule constant-list-index-expression`,
  `--rule constant-map-index-expression`,
  `--rule constant-record-field-access-expression`,
  `--rule constant-len-expression`,
  `--rule constant-not-expression`,
  `--rule empty-if-statement`,
  `--rule empty-else-statement`,
  `--rule empty-for-statement`,
  `--rule empty-forge-statement`,
  `--rule identity-binary-expression`, or
  `--rule redundant-boolean-comparison`, or
  `--rule absorbing-boolean-expression`, or
  `--rule idempotent-boolean-expression`, or
  `--rule self-comparison-expression`, or
  `--rule double-negation-expression`, or
  `--rule negated-comparison-expression`, or
  `--rule redundant-boolean-if-expression`, or
  `--rule redundant-boolean-if-statement`, or
  `--rule same-branch-if-expression`, or
  `--rule same-branch-if-statement`, or
  `--rule unreachable-statement`, or
  `--rule absorbing-arithmetic-expression`
  filters, and
  `schema: "sley.lint.report.v0"` for warning-grade graph and authority lints
- in-tree `sley-contract` utility scaffold with `inventory`, `check-fixtures`,
  `validate`, and `inspect-deploy-artifacts` JSON validation commands over
  `docs/schemas/`, `fixtures/contracts/`, release manifests, and local deploy artifact
  directories, using versioned JSON report roots for downstream contract-kit
  work
- in-tree `sley-ci` utility with `check`, `lint`, `doctor`, `plan`, `run`,
  `verify`, `deploy`, `smoke`, `corpus`, and `examples` wrappers over existing
  Sley check/lint/doctor/plan/run/verify/deploy gates, CLI smoke manifests,
  the accepted/rejected compiler conformance corpus, and packaged examples,
  emitting
  `schema: "sley.ci.report.v0"` for CI and pre-commit integration
- in-tree `sley-conformance` utility with `report` and `coverage` commands for
  schema/fixture instance coverage, contract validation status, corpus coverage
  tags, smoke coverage tags, packaged example counts, and the declared
  integration-test count, plus non-gating public-release packaging blockers,
  with `--require-public-release-ready` available as the explicit public-cut
  gate once license and repository metadata are settled,
  emitting
  `schema: "sley.conformance.report.v0"` and
  `schema: "sley.conformance.coverage.v0"`
- in-tree `tree-sitter-sley` syntax grammar bootstrap with checked
  Tree-sitter corpus tests, highlight queries, generated parser artifacts, and
  fixture parsing over current examples plus accepted compiler corpus
- in-tree `sley-lsp` stdio language-server bootstrap with full-document sync,
  compiler diagnostics plus lint warnings, formatting edits, document symbols,
  declaration hover, checked edit-plan code actions, and a non-mutating
  `sley.fix.preview` command for editor repair previews
- in-tree `sley-workbench` local inspection bootstrap that emits
  `schema: "sley.workbench.report.v0"` and optional static HTML over
  doctor/query/lint/plan/graph panels without writing source files
- in-tree `sley-docgen` reference generator that emits
  `schema: "sley.docgen.report.v0"` and optional Markdown over checked
  module, task, type, effect, and host capability reference data
- in-tree `sley-agent-bench` deterministic repair-loop benchmark that emits
  `schema: "sley.agent_bench.report.v0"` after proving a JSON inspect, lint,
  plan, checked fix, verify, seal, and ZJX handoff path
- in-tree `sley-migrate` checked migration report utility that emits
  `schema: "sley.migrate.report.v0"` over source migration templates and
  optional schema/fixture drift checks without writing source files
- in-tree `sley-sandbox-runner` deterministic replay utility that consumes
  `schema: "sley.sandbox.manifest.v0"` manifests and emits
  `schema: "sley.sandbox.report.v0"` over seeded capability, file, table,
  secret, network, shell, model, deploy, and spend runs without external
  provider calls
- in-tree `sley-zjx` read-only envelope utility with `inspect`,
  `verify-digest`, `extract-graph`, and `diff-envelope` commands over preview
  ZJX JSON envelopes, emitting `schema: "sley.zjx.tool.report.v0"`
- checked JSON readiness reports with `sley doctor`, consuming strict
  diagnostics plus query and lint summaries, with call-bearing reports routing
  agents to strict `sley query --kind calls` inspection, warning next-actions
  that route agents to `sley plan --json --graft-templates`, and, when exactly
  one checked lint repair exists, a non-mutating `sley fix --dry-run` preview
  plus an explicit `write_command` for the matching `sley fix --write`, and
  `schema: "sley.doctor.report.v0"` for agent pre-edit gates
- checked JSON edit-plan reports with `sley plan`, consuming strict
  diagnostics plus query and lint findings into ranked task edit surfaces,
  graft target ids, call-row inspection next-actions, optional
  `--graft-templates` starter operation payloads, rename-plus-call-site and
  add-take-plus-call-arg transaction templates for targeted caller surfaces,
  safe remove-take-plus-call-arg transaction templates for unused takes,
  lint-driven delete templates and cleanup
  transactions for unused private types/effects, checked
  `delete_unused_private_task` templates for dead private tasks, checked
  `delete_dead_private_tasks` cleanup transactions for dead private task
  groups and unreachable cycles, checked
  `remove_unused_take` templates for unused takes with no invalid caller
  fallout, checked
  `remove_unused_declared_effect` templates for unused task-declared effects,
  checked
  `delete_unused_import` templates for unused imports, lint-driven
  `AddModuleDeclaration` templates for explicit module style fixes with module
  names inferred from the target file or project-relative path,
  `migrate_raw_host_adapter` templates for eligible raw host calls that can
  move to fallible `try_` adapters with `?`,
  `propagate_unchecked_result` templates for discarded `Result` expression
  statements that can be safely rewritten with `?`,
  `qualify_imported_call` templates for simple imported task calls that should
  be alias- or module-qualified,
  `delete_unused_pure_expression_statement` templates for no-op pure
  expression statements,
  `delete_self_assignment_statement` templates for no-op `set name = name`
  mutations,
  `delete_overwritten_set_statement` templates for dead `set` statements
  immediately overwritten before any read,
  `fold_redundant_initial_set_into_binding` transactions for mutable
  initializers immediately replaced by a safe first `set`, plus
  `convert_redundant_initial_set_to_bind` transactions when no later mutation
  remains,
  `simplify_constant_if_expression` templates for constant `if` expressions
  that can be replaced by the branch that executes,
  `simplify_constant_if_statement` templates for constant `if` statements that
  can be replaced by their single executing branch statement,
  `delete_constant_false_if_statement` templates for never-executed `if false`
  statements without an `else` branch,
  `delete_constant_false_while_statement` templates for never-executed
  `while false` statements,
  `simplify_constant_comparison_expression` templates for literal comparisons
  that can be replaced by their boolean result,
  `simplify_constant_arithmetic_expression` templates for numeric literal
  arithmetic that can be replaced by its result,
  `simplify_absorbing_arithmetic_expression` templates for delete-safe
  multiplication by zero that can be replaced by the zero literal,
  `simplify_constant_text_concatenation_expression` templates for text literal
  concatenation that can be replaced by one text literal,
  `simplify_constant_list_index_expression` templates for literal list indexes
  that can be replaced by the indexed scalar literal,
  `simplify_constant_map_index_expression` templates for literal map indexes
  that can be replaced by the indexed scalar literal,
  `simplify_constant_record_field_access_expression` templates for literal
  record field access that can be replaced by the selected scalar literal,
  `simplify_constant_len_expression` templates for literal `len` calls that
  can be replaced by the literal length,
  `simplify_constant_not_expression` templates for literal boolean negation
  that can be replaced by the resulting boolean literal,
  `delete_empty_if_statement` templates for no-op `if` statements with
  delete-safe conditions and empty branches,
  `remove_empty_else_statement` templates for no-op empty `else` branches,
  `delete_empty_for_statement` templates for `for` statements over literal
  empty lists,
  `delete_empty_forge_statement` templates for no-op `forge { }` starter
  blocks,
  `simplify_identity_binary_expression` templates for identity binary
  expressions, including empty-text concatenation, that can be replaced by the
  non-identity side,
  `simplify_redundant_boolean_comparison` templates for boolean comparisons
  against `true` or `false`,
  `simplify_absorbing_boolean_expression` templates for absorbing boolean
  literals that can collapse a short-circuiting expression safely,
  `simplify_idempotent_boolean_expression` templates for delete-safe
  `expr && expr` and `expr || expr` forms,
  `simplify_self_comparison_expression` templates for delete-safe `expr == expr`,
  `expr != expr`, `expr < expr`, `expr <= expr`, `expr > expr`, and
  `expr >= expr` comparisons,
  `simplify_double_negation_expression` templates for `!!expr` forms,
  `simplify_negated_comparison_expression` templates for `!(left op right)`
  comparison inversions,
  `simplify_redundant_boolean_if_expression` templates for boolean `if`
  expressions that can be replaced by the condition or its negation,
  `simplify_redundant_boolean_if_statement` templates for boolean `if`
  statements that can return the condition or its negation directly,
  `simplify_same_branch_if_expression` templates for same-branch `if`
  expressions with delete-safe conditions,
  `simplify_same_branch_if_statement` templates for same-branch `if`
  statements with delete-safe conditions,
  `delete_unreachable_statement` templates for dead statements after a
  guaranteed return,
  `--template-surface <surface>` targeting for task, block, statement, take,
  expression, program, or lint surfaces, including checked task-body and block
  `insert_statement` starters, checked statement-surface `replace_statement`
  starters, and checked program-surface `add_task`, `add_type_declaration`,
  `add_effect_declaration`, and `add_import` starters, `--emit-graft <kind>`
  for direct operation/transaction JSON handoff to `sley graft`, and
  `schema: "sley.edit_plan.report.v0"`
- checked JSON fix previews with `sley fix --kind <kind>`, consuming the same
  plan graft templates and applying one named operation or transaction through
  the graft checker, including exact block/statement/take/expression node
  surfaces selected with `--template-surface`, checked `--name`, `--type`,
  `--module`, `--source`, `--source-file`, and `--position` payload overrides for
  single-operation templates that expose those editable fields, and including
  `add_module_declaration` for `missing_module_declaration` lint findings with contextual module-name
  inference; default and `--dry-run` mode stay non-mutating, while `--write`
  uses the same checked writeback, trace receipts, optional
  `--trace <trace.jsonl>` receipt path, and
  `schema: "sley.graft.outcome.v0"` as `sley graft`
- checked JSON verification reports with `sley verify`, consuming strict
  diagnostics, query summaries, lint findings, and deterministic runtime
  execution into `schema: "sley.verify.report.v0"` for CI and pre-deploy gates;
  warning or denied-warning reports point to checked graft-template repair
  planning and expose a dry-run fix preview plus explicit `write_command` when
  the repair is unambiguous, while passed reports point to `sley seal --json`
  and `sley zjx --json` handoff artifacts
- schema-versioned runtime reports with `sley run --json`, carrying successful
  deterministic execution values under `schema: "sley.run.report.v0"`
- local-only deploy dry-run reports with `sley deploy --json --dry-run`,
  composing strict verify, trace seal, and ZJX package summaries into
  `schema: "sley.deploy.report.v0"` while recording that live deployment,
  external mutation, and provider calls are not allowed without explicit
  operator approval; `--artifacts-dir <dir>` writes local `deploy-report.json`,
  `seal.json`, `zjx-envelope.json`, and a digest-bearing `manifest.json`
  handoff file only after the dry-run package is ready, and the ready report
  points agents to `sley-contract inspect-deploy-artifacts`
- JSON project scaffold reports with `sley new --json`, `hello`, `library`,
  `cli`, `service-gate`, `data-pipeline`, `deploy`, `spend-gate`, `agent`,
  `agent-task-pack`, and `agent-project` templates, relative created-file paths,
  next-command vectors, typed next-action reasons through check, doctor, query,
  plan, lint, verify, run, seal, and ZJX package steps, and
  `schema: "sley.project.scaffold.v0"`
- JSONL trace sidecars for accepted graft/fix receipts when
  `sley graft --write` or `sley fix --write` applies a change, with
  `--trace <trace.jsonl>` for explicit receipt paths, plus content-addressed
  trace seals with `sley seal`
- project-aware `sley graft --write <project>` source-file writeback for
  accepted edits that resolve to existing modules, `AddImport` writes to
  existing on-disk module files that were not yet loaded through the entry
  import graph, plus checked creation of new module files declared by the graft
  candidate, deletion of removed module files no longer referenced by the
  checked candidate, and module rename writeback with manifest entry updates
  when needed
- a first ZJX-ready JSON envelope command for graph snapshots, recomputable
  graph digests, optional graph slices, and trace receipts
- a future-runtime lock spec at `docs/SleyZjxRuntimeLockSpec.md`, requiring
  `zjx-sley` to validate Sley structure before compressing Sley artifacts
- structural graft operations for adding explicit module declarations,
  adding/removing takes, removing task-declared effects, replacing task bodies, adding
  imports/effects/types/tasks, renaming declarations, updating call-sites,
  updating/replacing/removing call arguments, inserting checked task-body
  statements, replacing checked statements and nested expressions by node id,
  deleting checked graph nodes such as declarations, imports, takes, and
  statements, and moving checked statements or top-level declaration order
  within their current parent
- strict graft input JSON for the v0 operation shapes; unknown graft fields are
  rejected instead of silently ignored
- versioned JSON report roots for deterministic `sley run --json` results,
  diagnostics with a shared diagnostic record schema, and graft outcomes with
  strict accepted provenance records
- repair hints on common checker diagnostics, including unknown identifiers,
  unknown tasks, type mismatches, return mismatches, missing return paths, call
  argument mismatches, condition mismatches, effect authority, and private or
  ambiguous names
- locked JSON contract snapshots under `fixtures/contracts/`, including
  symbol graphs, graft outcomes, ZJX envelopes, ZJX tool reports, checked run,
  query, lint, doctor, edit-plan, verify, deploy dry-run, deploy artifact
  check, CI, and project scaffold reports, LSP fix-preview payloads, workbench
  reports, docgen reports, agent-bench reports, migrate reports, sandbox
  manifests and sandbox-runner reports, plus contract inventory, fixture-check,
  and validate reports
- external v0 JSON Schema files under `docs/schemas/`, including strict
  query task/take/type/effect/call row definitions,
  strict runtime value definitions for `sley.run.report.v0`,
  `sley-ci` check/lint/doctor/plan/run/verify/deploy/smoke/corpus/examples reports,
  `sley.lsp.fix_preview.v0` preview-command payloads,
  `sley.workbench.report.v0` local inspection reports,
  `sley.docgen.report.v0` generated reference reports,
  `sley.agent_bench.report.v0` deterministic agent-loop benchmark reports,
  `sley.migrate.report.v0` checked migration reports,
  `sley.sandbox.manifest.v0` deterministic replay manifests,
  `sley.sandbox.report.v0` sandbox-runner replay reports,
  `sley.zjx.tool.report.v0` envelope inspection reports,
  `sley-contract` inventory/fixture-check/validate/deploy-artifact-check reports,
  edit-plan graft operation and transaction template envelopes reused by graph
  slice affordances, graph-slice focus/task/call summary refs, a shared
  diagnostic record schema, a trace report schema over standalone trace
  receipts, plus ZJX graph, slice, and trace receipt handoff refs backed by
  fixtures
- manifest-backed accepted/rejected synthetic conformance corpus fixtures under
  `fixtures/corpus/`, including declared and missing authority cases for the
  seeded host adapter surface and an accepted agent deploy pipeline that
  composes SecretRead, Network, ModelCall, and Deploy, plus accepted/rejected
  split-task fixtures for transitive authority propagation, including spend
  helper boundaries
- manifest-backed CLI smoke conformance cases under `fixtures/cli_smokes/`,
  covering stable command output, JSON roots, graph/ZJX surfaces, doctor
  readiness, edit-plan surfaces, verify pre-deploy gates, project scaffolding,
  graft/fix dry runs, direct graft writes, temp setup files for write-mode
  smokes, explicit graft/fix trace receipts, trace seals over non-empty receipt
  chains, ZJX envelopes carrying graph digests and schema-backed trace
  receipts, ZJX tool reports, passed-verify next-actions for seal and ZJX
  handoff artifacts,
  doctor/verify warning next-actions that route to lint repair plans and
  unambiguous dry-run fix previews with explicit
  `write_command` vectors, a staged previewed-repair write smoke that proves
  strict verify readiness afterward, a project-level previewed unused-import
  repair write followed by strict verify, generated scaffold quickstarts that
  re-verify with local or seeded authority, statement-surface
  `replace_statement` planning and fix dry runs, program-surface
  declaration/import planning and `add_task`/effect/import fix dry runs with
  name/source/module overrides,
  missing-module checked repair templates with module-name inference,
  lint-driven fix writes that clear warnings before verify, including empty-if
  and empty-else cleanup, a standalone agent deploy pipeline example with
  check/lint/run/verify/deploy dry-run coverage, and seeded host-adapter
  execution
- packaged example conformance through `sley-ci examples --json examples`,
  covering project-root checks for `sley.toml` examples, standalone file checks,
  and formatter round trips for every shipped `.sley` source under `examples/`
- repo-level `Makefile` with `make v1` wrapping fmt, whitespace diff check,
  full Rust tests, contract fixture and release-manifest validation,
  conformance summary reporting, corpus conformance, packaged example
  conformance, CLI smoke conformance, the focused LSP integration test,
  deterministic utility replays for workbench, agent-bench, migration, docgen,
  sandbox-runner, and ZJX tools, plus Tree-sitter syntax parsing
- GitHub Actions and pre-commit entry points that run the same `make v1` gate
  through `.github/actions/sley-v1/action.yml`, `.github/workflows/v1.yml`,
  and `.pre-commit-config.yaml`; the action installs Rust and Node tooling
  before the gate
- compact agent onboarding pack in `llms.txt`
- release-facing `CHANGELOG.md` and `docs/contracts.md` files covering the
  current contract roots, validation commands, and release-candidate deltas

Repository release gate:

```bash
make v1
```

The default GitHub workflow calls the local composite Sley v1 action, and the
local pre-commit configuration calls the same gate, so CI and local review use
the same release surface.
For a public release cut, run the stricter metadata gate after the operator has
chosen license and repository values:

```bash
make public-release-check
```

Project form:

```bash
sley new --template agent-project --name agent-app --module agent.main agent-app
cd agent-app
sley check --json .
sley doctor --json .
sley query --json --kind tasks .
sley plan --json .
sley lint --json --deny-warnings .
sley verify --json --deny-warnings --cap SecretRead --secret api_key redacted --cap Network --http-text https://example.test/profile "profile ready" --cap ModelCall --model-output deploy-plan "plan approved" --cap Deploy --deploy-result staging staged .
sley-ci check --json .
sley-ci lint --json --deny-warnings .
sley-ci doctor --json --deny-warnings .
sley-ci plan --json --graft-templates .
sley-ci verify --json --deny-warnings --cap SecretRead --secret api_key redacted --cap Network --http-text https://example.test/profile "profile ready" --cap ModelCall --model-output deploy-plan "plan approved" --cap Deploy --deploy-result staging staged .
sley-ci run --json --cap SecretRead --secret api_key redacted --cap Network --http-text https://example.test/profile "profile ready" --cap ModelCall --model-output deploy-plan "plan approved" --cap Deploy --deploy-result staging staged .
sley run --json --cap SecretRead --secret api_key redacted --cap Network --http-text https://example.test/profile "profile ready" --cap ModelCall --model-output deploy-plan "plan approved" --cap Deploy --deploy-result staging staged .
sley deploy --json --dry-run --artifacts-dir .sley/deploy --cap SecretRead --secret api_key redacted --cap Network --http-text https://example.test/profile "profile ready" --cap ModelCall --model-output deploy-plan "plan approved" --cap Deploy --deploy-result staging staged .
sley-ci deploy --json --dry-run --artifacts-dir .sley/ci-deploy --cap SecretRead --secret api_key redacted --cap Network --http-text https://example.test/profile "profile ready" --cap ModelCall --model-output deploy-plan "plan approved" --cap Deploy --deploy-result staging staged .
sley-ci corpus --json fixtures/corpus
sley-ci examples --json examples
sley-conformance report --json
sley-conformance report --json --corpus-manifest fixtures/corpus --smoke-manifest fixtures/cli_smokes --smoke-manifest fixtures/ci_smoke_probe
sley-conformance coverage --json --require-tag cli:check --require-tag json:sley.trace.receipt.v0
sley-contract inspect-deploy-artifacts .sley/deploy --schemas docs/schemas --json
make smoke
make syntax
sley-lsp
sley-workbench --json --html .sley/workbench.html .
sley-docgen reference --json --markdown .sley/reference.md .
sley-agent-bench run --json
sley-migrate report --json --schemas docs/schemas --fixtures fixtures/contracts .
sley-sandbox-runner run --json fixtures/contracts/sandbox_manifest_agent_pipeline.json
sley seal --json .
sley zjx --json .
sley-zjx inspect --json envelope.json
sley-zjx verify-digest --json envelope.json
```

Standalone dogfood example:

```bash
sley check --json examples/agent_deploy_pipeline.sley
sley doctor --json examples/agent_deploy_pipeline.sley
sley query --json --kind calls examples/agent_deploy_pipeline.sley
sley lint --json --deny-warnings examples/agent_deploy_pipeline.sley
sley run --json --cap SecretRead --secret api_key redacted --cap Network --http-text https://example.test/profile "profile ready" --cap ModelCall --model-output deploy-plan "plan approved" --cap Deploy --deploy-result staging staged examples/agent_deploy_pipeline.sley
sley verify --json --deny-warnings --cap SecretRead --secret api_key redacted --cap Network --http-text https://example.test/profile "profile ready" --cap ModelCall --model-output deploy-plan "plan approved" --cap Deploy --deploy-result staging staged examples/agent_deploy_pipeline.sley
sley deploy --json --dry-run --artifacts-dir .sley/dogfood-deploy --cap SecretRead --secret api_key redacted --cap Network --http-text https://example.test/profile "profile ready" --cap ModelCall --model-output deploy-plan "plan approved" --cap Deploy --deploy-result staging staged examples/agent_deploy_pipeline.sley
sley-contract inspect-deploy-artifacts .sley/dogfood-deploy --schemas docs/schemas --json
```

Packaged multi-module agent project:

```bash
sley check --json examples/agent_project
sley lint --json --deny-warnings examples/agent_project
sley query --json --kind calls examples/agent_project
sley run --json --cap SecretRead --secret api_key redacted --cap Network --http-text https://example.test/profile "profile ready" --cap ModelCall --model-output deploy-plan "plan approved" --cap Deploy --deploy-result staging staged examples/agent_project
sley verify --json --deny-warnings --cap SecretRead --secret api_key redacted --cap Network --http-text https://example.test/profile "profile ready" --cap ModelCall --model-output deploy-plan "plan approved" --cap Deploy --deploy-result staging staged examples/agent_project
sley deploy --json --dry-run --artifacts-dir .sley/agent-project-deploy --cap SecretRead --secret api_key redacted --cap Network --http-text https://example.test/profile "profile ready" --cap ModelCall --model-output deploy-plan "plan approved" --cap Deploy --deploy-result staging staged examples/agent_project
```

`sley new` refuses to overwrite existing `sley.toml`, `README.md`, or entry
source files. `--template hello`, `--template library`, `--template cli`, and
`--template data-pipeline` create pure starters. `--template service-gate`
creates a deterministic network-gated starter that runs with seeded
`http.try_get_text` data. `--template deploy` creates a deterministic
deployment-gated starter that uses the seeded `deploy.try_stage` adapter; it
does not call deployment providers or mutate infrastructure. `--template
spend-gate` creates a deterministic spend-gated starter that uses seeded
`spend.try_authorize` authorization data; it does not create transactions,
payments, market orders, wallet calls, credits, or provider spend. `--template agent`
and `--template agent-task-pack` create deterministic single-module agentic
starters. `--template agent-project` creates a deterministic multi-module
agentic starter with an entry module and imported pipeline module. These
agentic starters compose seeded secret, network, model, and deploy authority
through explicit gates; they do not read real secret stores, call live
networks or model providers, or mutate deployment infrastructure.

```toml
[project]
name = "module-demo"
root = "src"
entry = "app.main"
```

Module `app.main` resolves to `src/app/main.sley`. `sley parse`,
`sley check`, `sley run`, `sley ast`, `sley graph`, `sley query`,
`sley lint`, `sley doctor`, `sley plan`, `sley fix`, `sley verify`,
`sley deploy`, `sley seal`, `sley zjx`, and `sley graft` accept either a
single `.sley` file or a project directory containing `sley.toml`. Project graft
writeback projects the checked candidate back to existing owning module files
and leaves unchanged module files alone. It can add imports to existing
on-disk module files that were not yet loaded through the entry import graph,
create checked new module files, delete removed loaded module files, rename
module files, and update `sley.toml` when the manifest entry module is renamed.
`sley run --json` emits `sley.run.report.v0` on successful deterministic
execution with the runtime value under `/value`.

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
  `compression=none` and a recomputable graph digest; the binary compressed
  archive handoff remains a later integration step. `sley-zjx` can inspect
  those preview envelopes, recompute the embedded graph digest, extract the
  symbol graph, and diff two envelopes without writing source files.
- The future compressed `zjx-sley` runtime must not trust envelope metadata as
  the file lock. It must follow `docs/SleyZjxRuntimeLockSpec.md`: reconstruct
  and validate Sley graph structure, recompute the graph hash, reject unknown
  or opaque fields, and refuse generic CSV, JSONL, checkpoint, backup, or blob
  payloads disguised as Sley artifacts.
- Runtime host support is intentionally narrow: `FileRead`/`FileWrite` have
  root-scoped filesystem handlers, `DatabaseRead` has a deterministic
  seeded-table adapter, and `DatabaseWrite` has a deterministic per-run insert
  adapter. `Network` has a deterministic seeded text adapter, `Shell` has a
  deterministic seeded command-output adapter, and `ModelCall` has a
  deterministic seeded prompt-completion adapter. `SecretRead` has a
  deterministic seeded secret-value adapter. The `Deploy` adapter returns
  deterministic seeded stage results, and the `Spend` adapter returns
  deterministic seeded authorization results. Non-file adapters can be scoped
  by resource-key prefix with `--cap EFFECT=SCOPE`, and scope mismatches remain
  authority diagnostics. None of the seeded host adapters perform live network,
  shell, model, secret, deploy, or spend actions.
- Sley-level `Result` values and `?` propagation execute, and fallible
  filesystem, database, network, shell, model, secret, deploy, and spend host
  variants return typed `Error` records.
- Project graft writeback supports existing module files, `AddImport` writes to
  existing on-disk module files that were not yet loaded through the entry
  import graph, checked new module file creation under the project source root,
  deletion of removed module files once they are no longer imported, module
  rename writeback, and `sley.toml` entry updates when the entry module is
  renamed. Grafts that import truly missing modules without adding checked
  declarations for those modules still reject before any source or trace
  mutation.
- The AST JSON Schema now covers declarations, statements, expressions, type
  expressions, spans, and provenance recursively. The edit-plan schema also
  pins strict graft operation and transaction template envelopes, including
  `ReplaceStatement`. The graft outcome schema pins strict accepted provenance
  records. Other external JSON Schema files remain narrower v0 root-contract
  schemas.
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
for seeded host adapters, stable JSON roots, graph/ZJX output, graft and fix
dry runs, checked run reports, checked graph query reports including strict
task/take/type/effect/call row definitions and graph-slice focus/task/call
payloads plus affordances with strict graft operations, doctor readiness,
call-inspection next-actions,
verify and deploy dry-run pre-deploy gates,
edit-plan surfaces, call-site rename and remove-take/call-arg transaction
write/query/verify, typed scaffold next-actions for starter, deploy, service,
and agent quickstarts, strict seeded verify readiness for the generated gated
starters, deploy dry-run package reports with optional local artifact
manifests, scaffold-level seal/ZJX handoff actions,
temp setup files for write-mode CLI smokes, direct graft write smokes,
the lightweight `sley-ci smoke` probe for parse, query, graft dry-run, and
seeded multi-capability agent runtime authority,
project `AddImport` writeback into existing unloaded module files plus
follow-up strict project checks through direct graft JSON and `sley fix --write`,
explicit graft/fix write trace reports and receipts, non-empty trace seal receipts,
ZJX envelopes carrying schema-backed trace receipts, passed-verify seal/ZJX
next-actions, lint-driven declaration delete templates and cleanup transactions,
lint-driven unused-private-task delete templates,
dead private task cleanup transactions,
lint-driven unused-take remove templates,
lint-driven unused-declared-effect remove templates,
lint-driven unused-import delete templates,
lint-driven module declaration fixes with inferred module names,
write-mode unused-import cleanup through `sley fix --write` plus post-fix
verify,
raw-host adapter migration templates, unchecked-result propagation templates,
unqualified imported-call qualification templates with write/query/verify
coverage, unused pure binding delete templates with write/verify coverage,
unused pure expression statement delete templates with write/verify coverage,
mutable-binding conversion transactions with write/verify coverage,
constant-if expression simplification templates with write/verify coverage,
constant-if statement simplification templates with write/verify coverage,
constant-false if statement delete templates with write/verify coverage,
constant-false while statement delete templates with write/verify coverage,
constant comparison expression simplification templates with write/verify
coverage,
constant arithmetic expression simplification templates with write/verify
coverage,
absorbing arithmetic expression simplification templates with write/verify
coverage,
constant text concatenation expression simplification templates with
write/verify coverage,
constant list index expression simplification templates with write/verify
coverage,
constant map index expression simplification templates with write/verify
coverage,
constant record field access expression simplification templates with
write/verify coverage,
constant len expression simplification templates with write/verify coverage,
constant not expression simplification templates with write/verify coverage,
empty-if statement delete templates with write/verify coverage,
empty-else statement removal templates with write/verify coverage,
empty-for statement delete templates with write/verify coverage,
empty-forge statement delete templates with write/verify coverage,
no-op self-assignment statement delete templates with write/verify coverage,
overwritten set statement delete templates with write/verify coverage,
identity binary expression simplification templates, including empty-text
concatenation cleanup, with write/verify coverage,
redundant boolean comparison simplification templates with write/verify
coverage,
double negation expression simplification templates with write/verify coverage,
negated comparison expression simplification templates with write/verify
coverage,
redundant boolean-if statement simplification templates with write/verify
coverage,
same-branch if expression and statement simplification templates with
write/verify coverage,
unreachable statement delete templates with write/verify coverage,
private-task lint rules,
declaration/import/API hygiene, authority hygiene, and explicit module style
warnings. The next logical phase is to keep broadening style and migration
lints before broadening the language again.
