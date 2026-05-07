# Sley Improvement Plan

Date: 2026-05-05
Status: working design note
Scope: agent usability, self-hosting path, compiler feedback loop, binding ontology governance

## Short Version

Sley should not depend on Codex already knowing Sley from training data.

The first Rust Loom should act like a strict teacher, referee, and translator:
it reads Sley, exposes typed graph shards, accepts or rejects grafts, and gives
machine-readable diagnostics. Agents learn Sley through that loop.

The language does not "live in Rust" forever. Rust is the trusted bootstrap
implementation. Sley lives in its specification, typed graph model, examples,
tests, diagnostics, and eventually self-hosted compiler passes.

## Explain It Like I Am 5

Think of Sley as a new game.

Codex has not played this game before, so we do not ask Codex to guess all the
rules from memory.

Instead, we give Codex:

- the rulebook: the Sley spec
- a picture of the board: the typed graph or AST
- legal moves: graft operations
- a referee: the Rust Loom
- correction notes: diagnostics and repair hints

At first, the referee is written in Rust because Rust is already strong,
stable, and known. That does not mean Sley is secretly Rust. It means Rust is
holding the training wheels while Sley grows.

Later, parts of the referee can be rewritten in Sley. When Sley can explain and
check more of itself, the training wheels come off one by one.

## Correct Mental Model

Wrong:

```text
Sley exists only inside the Rust compiler.
```

Better:

```text
Sley semantics -> implemented first by Rust Loom
Sley source -> human review projection
Sley graph -> canonical program structure
ZJX -> compact transport/cache envelope
Rust Loom -> bootstrap oracle and safety gate
Future Sley Loom passes -> self-hosted implementation pieces
```

The Rust compiler is the first trustworthy executable version of the rules. It
should stay in charge until Sley has enough tests, examples, graph stability,
diagnostics, and self-hosted passes to safely take over parts of itself.

## The Real Training Data Strategy

The training-data problem is real: a new language has little or no model
pretraining corpus.

Sley should reduce that problem by making the agent workflow structural instead
of relying on raw text prediction.

The agent should not need to "remember" Sley perfectly. It should be able to:

1. Read compact docs and examples.
2. Request a bounded graph shard.
3. Propose a graft operation.
4. Receive structured diagnostics.
5. Repair the graft.
6. Repeat until the Loom accepts it.

This makes Sley learnable in-context.

The goal is not to beat pretraining with vibes. The goal is to replace guessing
with a tight compiler-mediated feedback loop.

## Improvement 1: Agent-First Tool Contract

Codex and other agents need a small stable command contract:

```bash
sley parse --json <target>
sley check --json <target>
sley ast --json <target>
sley ast --json --node <node-id> <target>
sley graph --json <target>
sley graph --json --slice <node-id> <target>
sley new --json [--template hello|library|cli|service-gate|data-pipeline|deploy|spend-gate|agent|agent-task-pack|agent-project] [--name <name>] [--module <module>] <path>
sley doctor --json [--deny-warnings] <target>
sley plan --json [--deny-warnings] [--graft-templates] [--template-surface <surface>] [--emit-graft <kind>] <target>
sley fix --json --kind <kind> [--template-surface <surface>] [--name <name>] [--type <type>] [--module <module>] [--source <source>|--source-file <path>] [--position <n>] [--dry-run|--write] [--trace <trace.jsonl>] <target>
sley verify --json [--deny-warnings] [runtime gates/seeds] <target>
sley deploy --json --dry-run [--artifacts-dir <dir>] [runtime gates/seeds] <target>
sley query --json [--kind all|modules|tasks|types|effects|calls] [--module <module>] <target>
sley lint --json [--rule unused-private-task|unreachable-private-task|unused-declared-effect|unused-import|unused-take|unused-private-type|unused-private-effect|raw-host-adapter|missing-module-declaration|unchecked-result|unchecked-result-binding|unused-effectful-binding|unqualified-imported-call|unused-pure-binding|unused-pure-expression-statement|mutable-binding-never-set|self-assignment-statement|overwritten-set-statement|redundant-initial-set-statement|constant-if-expression|constant-if-statement|constant-false-if-statement|constant-false-while-statement|constant-comparison-expression|constant-arithmetic-expression|constant-text-concatenation-expression|constant-list-index-expression|constant-map-index-expression|constant-record-field-access-expression|constant-len-expression|constant-not-expression|empty-if-statement|empty-else-statement|empty-for-statement|empty-forge-statement|identity-binary-expression|redundant-boolean-comparison|absorbing-boolean-expression|idempotent-boolean-expression|self-comparison-expression|double-negation-expression|negated-comparison-expression|redundant-boolean-if-expression|redundant-boolean-if-statement|same-branch-if-expression|same-branch-if-statement|unreachable-statement|absorbing-arithmetic-expression] [--module <module>] <target>
sley-ci lint --json [--deny-warnings] [--rule <rule>] [--module <module>] <target>
sley-ci doctor --json [--deny-warnings] <target>
sley-ci plan --json [--deny-warnings] [--graft-templates] [--template-surface <surface>] <target>
sley-ci run --json [runtime gates/seeds] <target>
sley-ci corpus --json <fixtures/corpus|fixtures/corpus/manifest.json>
sley-ci examples --json examples
sley-conformance report --json [--corpus-manifest <fixtures/corpus|fixtures/corpus/manifest.json>] [--smoke-manifest <fixtures/cli_smokes|fixtures/cli_smokes/manifest.json>] [--editor-shim-root <editors/vscode-sley>] [--makefile <Makefile>]
sley-conformance report --json --require-public-release-ready
sley-conformance coverage --json --require-tag <tag>
sley-contract inventory [docs/schemas] --json
sley-contract check-fixtures <fixtures-dir> [--schemas <dir>] --json
sley-contract validate --schema <schema-id> <report.json> [--schemas <dir>] --json
sley-contract inspect-deploy-artifacts <artifacts-dir> [--schemas <dir>] --json
make smoke
make public-release-check
make v1
make syntax
sley-lsp
sley-workbench --json [--html <path>] [--slice <node-id>] <target>
sley-docgen reference --json [--markdown <path>] [--module <module>] [--exported-only] <target>
sley-shadow report --json [--module <module>] [--rule <rule>] <target>
sley-agent-bench run --json [--case <name>] [--keep-workdir] [--sley-bin <path>]
sley-migrate report --json [--schemas <dir> --fixtures <dir>] <target>
sley-sandbox-runner run --json [--keep-workdir] <manifest.json>
sley trace --json <target>
sley seal --json <target>
sley zjx --json [--slice <node-id>] <target>
sley-zjx validate --json <zjx-envelope.json>
sley-zjx inspect --json <zjx-envelope.json>
sley-zjx verify-digest --json <zjx-envelope.json>
sley-zjx extract-graph --json [--output <graph.json>] <zjx-envelope.json>
sley-zjx diff-envelope --json <left-envelope.json> <right-envelope.json>
sley graft --json <target> <graft.json>
sley graft --json --dry-run <target> <graft.json>
sley format <target>
```

Rules:

- JSON output must be stable and versioned.
- AST program roots, bounded AST node reports, diagnostic reports, symbol
  graphs, graph slices, query reports, lint reports, run reports, doctor
  reports, verify reports, deploy dry-run reports, deploy artifact manifests,
  deploy artifact check reports, CI reports, project scaffold reports, trace
  reports, trace seals, docgen reports, sandbox manifests, sandbox-runner
  reports, ZJX envelopes, ZJX tool reports, and graft outcomes carry v0 schema
  IDs.
- AST program/node, diagnostic-report, symbol-graph, graph-slice, query-report,
  lint-report, run-report, graft-outcome, trace-report, trace-seal,
  ZJX-envelope, ZJX-tool-report, doctor-report, project-scaffold, agent-bench,
  migrate, docgen, sandbox manifest/report, and CI/deploy report snapshots,
  LSP fix-preview and command-preview payloads, workbench reports, plus contract
  inventory/fixture-check/validate report snapshots, are locked under
  `fixtures/contracts/`.
- JSON Schema files live under `docs/schemas/`; the AST schema covers nested
  declarations, statements, expressions, type expressions, spans, and
  provenance, the query schema exposes strict task/take/type/effect/call row
  definitions, the run schema pins recursive runtime value payloads, the trace
  report schema wraps trace receipt records, the diagnostic schema exposes a
  shared diagnostic record, the edit-plan schema pins strict graft operation
  and transaction template envelopes reused by graph-slice affordances,
  the graph-slice schema links
  focus, task, call-summary, inbound-call, and insert/move/delete/replace
  affordance payloads to shared contracts, `sley-ci`
  check/lint/doctor/plan/run/verify/deploy/smoke/corpus/examples reports,
  LSP fix-preview and command-preview payloads, workbench reports, docgen
  reports, agent-bench reports, migrate reports, sandbox manifests, sandbox-runner reports,
  `sley-conformance` report/coverage roots, and `sley-contract`
  inventory/fixture-check/validate/deploy-artifact-check reports have versioned
  schemas and representative fixtures, the graft outcome and trace receipt
  schemas pin accepted provenance records, the ZJX envelope and tool report
  schemas pin graph, slice, trace receipt, digest verification, extraction, and
  diff handoff refs with contract snapshots, the deploy artifact manifest schema
  pins report/seal/package file digests, and the remaining schema files are
  still root-contract v0 shapes.
- The repo-level `Makefile` exposes `make v1` as a local release gate over
  formatting, whitespace diff checks, full Rust tests, contract fixtures,
  release-manifest validation, conformance summary reporting, declared
  integration-test count drift, corpus conformance, packaged examples, CLI
  smokes, the focused LSP integration tests, deterministic workbench,
  agent-bench, raw-host and unchecked-result migration, docgen,
  sandbox-runner, shadow, and ZJX tool replays.
- `sley-conformance report` carries explicit required corpus and smoke release
  tags, inventories editor-shim package validation and the `make v1` target
  set, and fails if either manifest or the local gate drops required evidence,
  including seeded and scoped runtime authority smoke evidence.
- `sley-conformance report` carries local/public v1 readiness tracks so agents
  can distinguish passed executable gates from operator-controlled public
  release metadata blockers.
- `sley-conformance report --require-public-release-ready` is the explicit
  public-cut gate for license and repository metadata; ordinary executable v1
  conformance remains advisory on those operator decisions.
- `.github/actions/sley-v1/action.yml`, `.github/workflows/v1.yml`, and
  `.pre-commit-config.yaml` route hosted CI and local pre-commit checks through
  the same `make v1` gate.
- `sley-lsp` exposes a stdio language-server loop over current compiler
  surfaces: parse/check/lint diagnostics with `sley.toml` project context and
  open-buffer overlays, all-open-buffer and watched project-file diagnostic
  refresh, formatting, document symbols, folding ranges, declaration metadata
  and resolved project task-call hover, semantic tokens, workspace symbols,
  selection ranges, project completions, import/call definition jumps, project
  task signature help, project task parameter inlay hints, exact-range task
  references, document highlights, import document links, prepared
  cursor-aware project task rename edits,
  edit-plan code actions, non-mutating command-preview code lenses, and
  non-mutating preview payloads whose editor repair previews reuse the strict
  edit-plan graft contracts.
- `editors/vscode-sley` exposes a private local VS Code shim with `.sley`
  language metadata, basic TextMate highlighting, and a `vscode-languageclient`
  bridge to the current `sley-lsp` binary. It is validation-gated through
  `make v1` and `sley-conformance report` but remains unpublished until release
  metadata is approved.
- `sley-workbench` exposes a local read-only inspection report and optional
  static HTML page over doctor/query/lint/plan/graph/graph-slice panels. Its
  report schema links embedded panel rows back to the source doctor, query,
  lint, edit-plan, graph, and graph-slice contracts, and edit-plan template
  rows carry preview, write, and post-fix gate commands plus a local
  repair-focus selector for repair handoff.
- `sley-docgen` exposes checked Markdown reference generation over
  query-derived module, task, type, effect, and host capability docs. Its
  report schema reuses the strict query task/type/effect row definitions and
  carries seeded `--cap` args for deterministic host setup. Project-root
  reference reports can be narrowed with checked module/export filters that
  block unknown modules explicitly.
- `sley-agent-bench` exposes a deterministic local benchmark for the
  agent-facing edit loop, including JSON inspection, checked repair selection,
  write-mode fix, post-fix gates, trace receipt, seal, and ZJX evidence.
- `sley-migrate` exposes checked source migration reports over edit-plan
  templates and optional schema/fixture drift checks; migration operations
  reuse the strict edit-plan graft operation schema.
- `sley-sandbox-runner` exposes deterministic manifest-backed runtime replays
  over seeded capabilities, files, tables, secrets, network text, shell output,
  model output, deploy results, and spend results.
- `sley-zjx` exposes read-only preview-envelope validation, inspection, graph
  digest verification, graph extraction, and envelope diff reports.
- Focused utility integration tests validate live docgen, migrate,
  agent-bench, sandbox-runner, workbench, and ZJX JSON reports against their
  declared schemas.
- Diagnostics include stable IDs, node IDs, spans where possible, and repair
  hints for common checker failures.
- `sley doctor` is the first deterministic helper consuming strict check,
  `sley.query.report.v0`, and `sley.lint.report.v0` into a single readiness
  report for pre-edit agent planning; reports with checked calls now include
  `inspect_calls` next-actions for strict call-row inspection, and warning
  and denied-warning reports now include `plan_lint_repairs` next-actions that
  call checked
  `sley plan --json --graft-templates <target>` repair planning, plus
  `preview_lint_repair` dry-run fix commands when exactly one checked lint
  repair exists; those preview actions keep `command` non-mutating and add
  optional `write_command` vectors for the matching write. Ready reports now
  include `verify_gate` next-actions before entrypoint runs, with seeded
  `--cap` args when the entrypoint declares effects.
- `sley new --json` emits typed scaffold `next_actions` plus legacy
  `next_commands`; the deploy and agent starters' generated action sequences
  are executed in integration coverage so first-run check, doctor, query, plan,
  lint, warning-denying verify, run, deploy dry-run package, seal, ZJX package
  gates, and the agent starter's `sley-ci run`/`sley-ci verify`/`sley-ci deploy`
  handoffs cannot silently drift. The template pack now also covers pure
  `library`, `cli`, and `data-pipeline` starts plus seeded `service-gate`,
  `deploy`, and `spend-gate` authority starts, split-task
  `agent-task-pack`, and multi-module `agent-project` starts.
- `sley new --template agent-project` and `examples/agent_project` provide a
  packaged multi-module agent deployment starter that keeps secret, network,
  model, and deploy authority explicit across imported task boundaries and
  passes seeded check, lint, run, verify, deploy, examples conformance, scoped
  imported-host authority, and CLI smoke gates.
- `docs/AgentQuickstart.md` captures the shortest first-run path from
  `sley new --template agent-project` through JSON inspection, warning-denying
  verify, local dry-run deploy artifacts, and artifact contract inspection, so
  agents can start writing and packaging Sley code without reading the full
  release surface inventory first.
- The generated multi-module quickstart path is now a required smoke coverage
  tag, which keeps the documented first-run workflow tied to the executable
  v1 gate.
- The accepted/rejected synthetic gold corpus now includes split-task agent
  authority fixtures for transitive deploy, spend, and data mutation effect
  propagation, so helper-task authority drift is covered outside the large CLI
  smoke manifest too.
- The synthetic gold corpus now includes accepted/rejected module namespace
  fixtures for exported declarations and duplicate type/effect/task
  diagnostics.
- `sley-ci smoke --repo-root .` now resolves the repo root before `{repo}`
  expansion and accepts either a smoke directory or `manifest.json`, keeping
  temp-cwd smoke cases portable across direct agent invocation and the `make v1`
  absolute-root path.
- `sley plan` consumes the same checked surfaces into ranked task edit
  surfaces, call-row inspection next-actions, post-edit gate commands, and
  optional starter graft operation templates, rename-plus-call-site
  transactions, and add-take-plus-call-arg transactions, plus safe
  remove-take-plus-call-arg transactions for unused
  takes and lint-driven delete templates plus cleanup transactions for unused
  private type/effect declarations, plus lint-driven `AddModuleDeclaration`
  templates for missing module declarations with target/project-aware module
  name inference, plus `delete_unused_private_task` templates for dead private
  tasks, `delete_dead_private_tasks` cleanup transactions for dead private task
  groups and unreachable cycles, `remove_unused_take` templates for unused
  takes that validate through checked `RemoveTake`, and
  `remove_unused_declared_effect` templates for unused task-declared effects
  that validate through checked `RemoveTaskEffect`, and `delete_unused_import`
  templates for unused imports that validate through checked `DeleteNode`, plus
  `migrate_raw_host_adapter` templates for eligible raw host calls that can
  move to fallible `try_` adapters with `?`, plus
  `qualify_imported_call` templates for simple imported task calls that should
  be alias- or module-qualified, with editable JSON pointers; it also consumes
  selected graph-slice movement affordances as
  `move_statement`,
  `move_take`, and destination-variant templates when legal graph-slice
  destinations exist. Agents can target a specific task surface by node id or
  qualified name for task-body templates including a checked
  `insert_statement` starter, a block node id backed by graph-slice insert
  affordances for a checked `insert_statement`
  starter, a statement node id for direct checked graph-slice move/delete and
  `replace_statement` templates, a take node id for direct checked graph-slice
  move/delete templates, an expression node id for a checked no-op
  `replace_expression` starter, the `program` surface for checked
  declaration/import starters, the `program` missing-module surface, or a lint
  finding node id.
  Agents can use `--emit-graft <kind>` to print
  one matching operation or transaction JSON directly for dry-run or write-mode
  `sley graft`.
- `sley fix` consumes the same plan templates, selects one named operation or
  transaction kind, applies it through the graft checker, and emits
  `sley.graft.outcome.v0`; it is non-mutating by default and mutates only with
  explicit `--write`. Exact block, nested block, statement, nested statement,
  take, and expression node surfaces are covered by non-mutating fix dry-run
  smokes so agents can execute a chosen checked node template without
  hand-authoring graft JSON. Single-operation templates can override editable
  `/payload/name`, `/payload/type`, `/payload/module`, `/payload/source`, and
  `/payload/position` fields with `--name`, `--type`, `--module`,
  `--source`/`--source-file`, and `--position`.
  Accepted write-mode fixes use the default trace sidecar or an explicit
  `--trace <trace.jsonl>` receipt path; dry-run fixes never append receipts.
- `sley verify` is the deterministic CI/pre-deploy helper consuming strict
  check, `sley.query.report.v0`, `sley.lint.report.v0`, and seeded runtime
  execution into one pass/warnings/blocked report; warning and denied-warning
  reports include `plan_lint_repairs` next-actions and unambiguous dry-run fix
  previews before deployment review, while passed reports include seal and ZJX
  handoff next-actions. Embedded runtime values reuse the strict
  `sley.run.report.v0` value contract.
- `sley deploy --dry-run` is the local deploy package helper consuming strict
  verify plus trace seal and ZJX package summaries into
  `sley.deploy.report.v0`. It is explicitly non-live: no provider calls,
  external mutations, infrastructure changes, or spend, and live deployment
  remains behind operator approval. `--artifacts-dir <dir>` writes local
  `deploy-report.json`, `seal.json`, `zjx-envelope.json`, and digest-bearing
  `manifest.json` handoff files only after the dry-run package is ready;
  artifact manifests pin report, seal, and package roles to their expected
  schemas. `sley-ci run` wraps deterministic runtime execution under
  `sley.ci.report.v0`, `sley-ci deploy` passes the same flag through to the
  wrapped deploy command, and `sley-contract inspect-deploy-artifacts`
  revalidates the handoff
  manifest, schemas, and digests. `sley-contract` validation commands default
  to repo-local or bundled source schemas while keeping explicit schema
  overrides for pinned validation.
- Grafts support dry-run by default and an explicit `--dry-run` flag.
- Graft input JSON is strict: unknown operation or payload fields reject instead
  of being silently ignored.
- Accepted write-mode grafts emit trace receipts, and `sley seal` produces a
  content-addressed digest over source, graph, and receipt content.
- Runtime `Ok(value)`/`Err(error)` values and `?` propagation are implemented
  for Sley-level `Result` flow. Filesystem, seeded database reads, per-run
  database inserts, seeded network text responses, seeded shell command output,
  seeded model completions, seeded secret values, and seeded deployment stage
  results now expose fallible variants returning typed `Error` records.
- Source text should remain the human review projection, not the primary agent
  edit surface.

## Improvement 2: Strong Diagnostics And Repair Hints

Every common compiler rejection should be useful to an agent.

Good diagnostic shape:

```json
{
  "id": "EFFECT_UNAUTHORIZED",
  "severity": "error",
  "node": "task:app.profile.get_user",
  "span": {"line": 12, "column": 10},
  "message": "task `get_user` requires `DatabaseRead`",
  "repair_hints": [
    {
      "kind": "add_required_effect",
      "target": "task:app.profile.get_user",
      "effect": "DatabaseRead"
    }
  ]
}
```

Priority diagnostic families:

- parse errors with expected tokens: implemented with `insert_expected_token`,
  `provide_identifier`, `provide_expression`, and expected-item hints
- unknown identifiers: implemented with parse-valid `declare_binding` and
  `declare_mutable_binding` starter declarations
- unknown tasks: implemented with parse-valid `declare_or_import_task`
  starter declarations
- unknown types: implemented with parse-valid `declare_or_import_type`
  starter declarations
- binding and assignment type mismatch: implemented with type-change hints and
  structural `ReplaceExpression`
- condition, collection, index, and record-field expression mismatches:
  implemented with structural `ReplaceExpression` hints where the expected
  replacement type is clear
- record literal missing/unknown fields: implemented with whole-record
  structural `ReplaceExpression` hints that preserve known fields
- if branch type mismatch: implemented with alternative structural
  `ReplaceExpression` hints for the then and else branch expressions
- non-iterable `each` collections and invalid `len` arguments: implemented with
  conservative structural `ReplaceExpression` starter hints
- non-indexable collection expressions: implemented with structural
  `ReplaceExpression` hints when the index type selects list or map shape
- unary and binary operator mismatches: implemented with structural
  `ReplaceExpression` hints where a specific operand replacement type is clear
- call arity mismatch: implemented with `match_task_arity`, `UpdateCallArgs`,
  and `RemoveCallArg`
- call argument type mismatch: implemented with `replace_argument` and
  caller-scoped `ReplaceCallArg`
- call arity contraction after parameter removal: implemented with
  `RemoveCallArg` and safe remove-take transaction templates for unused takes
- return type mismatch: implemented with `change_return_type`,
  `replace_return_expression`, and structural `ReplaceExpression`
- missing return paths in non-`Unit` tasks: implemented with `MISSING_RETURN`,
  `insert_return`, and `replace_task_body`
- immutable binding mutation: implemented with typed parse-valid
  `use_mutable_binding_kind` starter declarations
- undeclared effects: implemented with parse-valid `declare_or_import_effect`
  starter declarations
- unauthorized host authority: implemented with `add_required_effect`
- private imported task/type/effect: implemented with export hints
- ambiguous imported task/type/effect: implemented with qualification hints
- stale graft preconditions: implemented with `refresh_graft_precondition`
  hints that tell agents to re-read current target state before retrying
- unsupported graft operation: implemented with `use_supported_graft_operation`
  hints, plus `replace_expression` guidance for expression move/delete attempts
- module namespace conflicts: implemented with `resolve_namespace_conflict`
  hints for duplicate checker declarations and graft add/rename collisions

## Improvement 3: Graft-First Editing

Agents should be trained to prefer grafts over raw file edits.

Minimum useful graft operations:

- `AddTake`
- `AddModuleDeclaration`
- `RemoveTake`
- `ReplaceTaskBody`
- `AddTask`
- `RenameDeclaration`
- `AddImport`
- `AddEffectDeclaration`
- `RemoveTaskEffect`
- `AddTypeDeclaration`
- `UpdateCallSites`
- `UpdateCallArgs`
- `ReplaceCallArg`
- `RemoveCallArg`
- `InsertStatement`
- `ReplaceStatement`
- `ReplaceExpression`
- `MoveNode`
- `DeleteNode`

`AddModuleDeclaration`, `AddTask`, `AddTypeDeclaration`,
`AddEffectDeclaration`, `RemoveTaskEffect`, `UpdateCallSites`, `UpdateCallArgs`,
`ReplaceCallArg`, `RemoveCallArg`, `InsertStatement`, `ReplaceStatement`,
`ReplaceExpression`, `DeleteNode`, and `MoveNode` are now implemented for the
v0 in-memory checked program.
`AddModuleDeclaration` turns module-less source into an explicit module and is
surfaced through `sley plan --graft-templates`/`sley fix` for
`missing_module_declaration`; the starter module name comes from the target
file stem or project-relative source path. `DeleteNode` supports checked
deletion of declarations, imports, takes, and statements.
`MoveNode` supports checked in-parent statement reordering and top-level
declaration ordering. Project-aware multi-file writeback now updates existing
module files, can add an import to an existing on-disk module file that was not
yet loaded through the entry import graph after validating its module
declaration, creates checked new module files declared by the graft candidate,
and deletes removed module files once the checked candidate no longer imports
them. It also supports module rename by rewriting module ownership and imports,
creating the renamed checked module file, and deleting the old loaded module
file. Entry-module rename updates `sley.toml` during checked project writeback.
Bare imports to missing modules still reject before mutation. Top-level import,
type, effect, and task movement into known modules is implemented for checked
project writeback, including AddImport-then-MoveNode transactions that create
the destination module file from the moved item. Cross-parent statement
movement between existing block parents is implemented with
`payload.destination`. Take reordering within the owning task and checked
cross-task take movement into another task's take list are implemented.
Expression movement and broader graph-contract hardening remain open.
Unsupported expression movement returns a `replace_expression` repair hint so
agents can plan the supported structural edit.
Graph, graph-slice, and query module import summaries now expose canonical
import node ids so agents can copy import graft targets directly from the
machine contract instead of reconstructing them.
`sley plan --graft-templates --template-surface module:<name>` and declaration
list parents such as `module:<name>:tasks` now consume the module graph-slice
affordances directly, exposing checked top-level import/type/effect/task move
and delete templates at module granularity. Direct declaration ids such as
`type:app.module.Name`, `effect:app.module.Audit`, and
`import:app.module:app.shared` use the same graph-slice move/delete
affordance path scoped to the selected declaration.
Graph slices also expose bounded `InsertStatement` affordances for task-local
block insertion, plus bounded `MoveNode` affordances for import, type, effect,
task, statement, and take movement planning, including exact parent ids and
destination insertion limits, starter operation JSON, and editable JSON
pointers. They also expose bounded `DeleteNode` affordances for import, type,
effect, task, statement, and take deletion planning, plus `ReplaceStatement`
affordances for whole task-local statement replacement and `ReplaceExpression`
affordances for task-local expression replacement; `sley plan
--graft-templates` filters selected task-internal delete, statement replace,
and expression replace templates through the checker before surfacing them.
`sley plan --graft-templates --template-surface program` now emits checked
`add_task`, `add_type_declaration`, `add_effect_declaration`, and `add_import`
starters so agents can add new declarations and imports without hand-authoring
graft JSON; `sley fix` can execute the source-backed declaration starters with
`--source` or `--source-file`, name-backed starters with `--name`, and
import-backed starters with `--module`.

The important rule is not that all operations exist immediately. The important
rule is that unsupported operations reject cleanly with explicit diagnostics.

## Improvement 4: Self-Hosting Ladder

Do not jump from Rust bootstrap to full Sley self-hosting in one leap.

Use a staged ladder:

1. **Rust-only oracle**
   Rust Loom parses, checks, formats, runs, and applies grafts.

2. **Sley standard library**
   Pure libraries and examples are written in Sley and validated by Rust Loom.

3. **Sley compiler helpers**
   Non-authoritative helper passes are written in Sley: graph queries,
   lint rules, migrations, fixture generators, and docs examples.

4. **Shadow compiler passes**
   Sley implementations run beside Rust implementations and must match output.

5. **Promoted Sley passes**
   Specific compiler passes become authoritative after conformance evidence.

6. **Self-hosted Loom core**
   Sley can build, inspect, and evolve large parts of its own compiler.

7. **Rust remains recovery oracle**
   Even after self-hosting, keep a small Rust or frozen reference implementation
   as the recovery checker until the ecosystem is mature.

## Improvement 5: Binding Ontology Governance

Agents may propose new binding kinds, but they should not directly mutate the
stable core ontology.

Core binding kinds should be human-ratified and versioned.

New binding kind proposal template:

```text
name:
status: proposed | experimental | stable | deprecated
purpose:
mutability:
authority behavior:
lifetime behavior:
sharing behavior:
checker rules:
runtime rules:
serialization shape:
example:
counterexample:
migration impact:
why existing binding kinds are insufficient:
```

Recommended process:

1. Agent proposes a binding kind as an experimental extension.
2. Loom validates that the proposal has semantics, examples, and checker rules.
3. Human reviews whether it deserves core status.
4. Experimental use collects evidence.
5. Promotion requires tests, docs, examples, and migration notes.

This keeps the ontology from becoming random model-generated vocabulary.

## Improvement 6: Canonical Agent Onboarding Pack

Codex will use Sley better if each repo exposes a compact onboarding pack:

- `README.md`
- `docs/SleyLanguageSpec.md`
- `llms.txt` or equivalent compact model-facing summary
- `examples/*.sley`
- `fixtures/grafts/*.json`
- `tests/*`
- `sley check --json`
- `sley ast --json`
- `sley graft --json`

The pack should include a short "agent rules" section:

```text
Prefer grafts over raw text edits.
Run `sley check --json` after each proposed change.
Use `sley ast --json --node` before editing a bounded target.
Do not invent binding kinds.
Do not add effects to silence errors unless the authority is semantically real.
Keep source formatting controlled by `sley format`.
```

The current repo-level pack is inventoried by `sley-conformance report` so
missing agent bootstrap files fail the release-readiness surface instead of
remaining an undocumented handoff risk.

## Improvement 7: Synthetic Gold Corpus

The synthetic corpus lives under `fixtures/corpus/`. It has a
`manifest.json` that lists each fixture and its coverage tags. It should keep
growing deliberately, with accepted fixtures that must parse/check/round-trip
and rejected fixtures that lock expected diagnostic IDs. The current corpus
already covers declared and missing authority for all deterministic seeded host
adapters, an accepted agent deploy pipeline that composes SecretRead, Network,
ModelCall, and Deploy, and accepted/rejected split-task agent authority fixtures
that lock transitive effect propagation, including a spend helper boundary,
plus accepted/rejected module namespace fixtures for exported declarations and
duplicate declaration diagnostics.

Corpus categories:

- small pure functions
- records and slots
- list and map logic
- result flow with `?`
- effect propagation
- authority and gate examples
- successful grafts
- rejected stale grafts
- rejected unauthorized grafts
- refactor migrations
- module namespace examples
- formatter round trips
- seeded host authority acceptance and rejection

Each corpus item should include:

- source
- AST or graph shard
- expected diagnostics
- accepted grafts
- rejected grafts
- final formatted output
- human explanation

The manifest is part of the release gate: new corpus files should not be added
silently outside it, and required release coverage tags should remain explicit.
The same manifest is executable through `sley-ci corpus --json fixtures/corpus`
or `sley-ci corpus --json fixtures/corpus/manifest.json`, which runs accepted
fixtures through strict check and formatter round-trip steps and rejected
fixtures through diagnostic ID checks against their sidecars.

This corpus becomes the real bridge from "Codex does not know Sley" to "agents
can operate Sley reliably."

## Improvement 8: CLI Smoke Conformance

The executable CLI smoke suite lives under `fixtures/cli_smokes/`. Its
`manifest.json` lists stable command lines, optional temp-directory execution,
optional allowlisted Sley utility binaries, optional temp setup files, coverage
tags, stdout substrings, JSON pointer/value expectations, and JSON pointer
absence expectations.
`sley-ci smoke --json --repo-root .
fixtures/cli_smokes` accepts the suite directory, and the Rust integration
suite runs those cases against the built `sley` binary plus selected sibling
utility binaries such as `sley-ci`, `sley-conformance`, `sley-contract`,
`sley-migrate`, `sley-docgen`, `sley-workbench`, `sley-sandbox-runner`,
`sley-shadow`, `sley-agent-bench`, `sley-zjx`, and `sley-lsp`. The lightweight
`fixtures/ci_smoke_probe` manifest now separately locks the `sley-ci smoke`
wrapper contract across parse, query, graft dry-run, and seeded
multi-capability agent runtime authority cases.
The broad smoke surface also pins direct and `sley-ci lint` module-filter
failure paths so typo filters remain visible to both command surfaces.

The current smoke manifest covers:

- parse, format, check, run, ast, graph, graph-slice, query, lint, trace, seal,
  zjx, and graft dry-run/write commands
- stable JSON roots for AST programs, diagnostics with shared diagnostic
  records, symbol graphs, graph slices, query reports, lint reports, run
  reports, trace reports, trace receipts, trace seals, graft outcomes with
  strict accepted provenance records, query task/take/type/effect/call row
  definitions, graph-slice focus/task/call payloads, graph-slice affordance operations,
  `sley-ci` reports, `sley-conformance` coverage reports, `sley-contract` JSON
  Schema validation reports, `sley-migrate` checked migration reports,
  `sley-docgen`, `sley-workbench`, `sley-sandbox-runner`, `sley-shadow`,
  `sley-agent-bench`, `sley-zjx` utility reports, `sley-lsp` help/startup, deploy artifact checks,
  and ZJX preview envelopes with graph digest and nested handoff refs
- query report direct task/take/type/effect/call row definitions
- doctor/plan call-bearing reports route agents to strict
  `sley query --kind calls` next-actions
- a write/query/verify smoke for the call-row-driven
  `rename_and_update_call_sites` transaction through `sley fix --write`
- a write/query/verify smoke for `remove_take_and_remove_call_arg`, proving
  unused-take cleanup can also update resolved callers
- graph-slice insert and replace affordances plus checked `insert_statement`,
  `replace_statement`, and `replace_expression` graft templates in edit-plan
  reports
- program-surface declaration/import templates for checked `add_task`,
  `add_type_declaration`, `add_effect_declaration`, and `add_import` starters,
  plus `add_task` emission and dry-run fix execution with source, name, and
  module overrides
- checked `sley fix --name`, `--type`, `--module`, `--source`,
  `--source-file`, and `--position` payload overrides for single-operation
  templates, including unsupported override diagnostics
- starter, service, deploy, and single/multi-module agent scaffold typed
  next-actions, first-run sequence execution, strict seeded
  `verify --json --deny-warnings` readiness, and local
  `sley deploy --dry-run` package reports, including optional deploy artifact
  manifests plus scaffold-level and passed-verify seal/ZJX handoff
  next-actions plus the agent scaffolds' `sley-ci run`/`sley-ci verify`/
  `sley-ci deploy` handoffs
- a standalone dogfood agent deploy pipeline example with strict
  check/lint/run/verify/deploy dry-run coverage across SecretRead, Network,
  ModelCall, and Deploy
- doctor/verify warning next-actions that route agents to checked
  `sley plan --json --graft-templates` lint repair plans and unambiguous
  `sley fix --dry-run` previews with explicit `write_command` vectors, plus a
  staged write-and-verify smoke for the previewed unused-private-task repair
  path and a project-level previewed unused-import repair path, including
  generated starter, service, deploy, and agent scaffold quickstarts
  re-verified with local or seeded authority
- lint-driven declaration delete templates, cleanup transactions, and direct
  declaration surface targeting in edit-plan reports
- lint-driven missing-module `AddModuleDeclaration` templates, module-name
  inference, and checked `sley fix` dry runs
- lint-driven unused-private-task `DeleteNode` templates and checked `sley fix`
  dry runs
- dead private task cleanup transactions for grouped unused/unreachable private
  task deletion
- lint-driven unused-take `RemoveTake` templates and checked `sley fix` dry
  runs
- lint-driven unused-declared-effect `RemoveTaskEffect` templates and checked
  `sley fix` dry runs
- lint-driven unused-import `DeleteNode` templates and checked `sley fix` dry
  runs, plus a write-mode project cleanup smoke that proves
  `delete_unused_import` clears lint before `sley verify --deny-warnings`
- manifest-staged temp files for write-mode smokes, including
  direct `sley graft --write --trace <trace.jsonl>` source mutation,
  `sley fix --write --trace <trace.jsonl>` receipt redirection, and follow-up
  `sley trace --trace <trace.jsonl>` receipt inspection plus
  `sley seal --trace <trace.jsonl>` sealing and
  `sley zjx --trace <trace.jsonl>` envelope transport over a non-empty receipt
  chain with a recomputable graph digest
- project `AddImport` writeback into an existing on-disk module file that was
  not yet loaded through the entry import graph, through both direct graft JSON
  and the `sley fix --write --kind add_import --module <module>` path, followed
  by strict project checks
- raw-host adapter migration templates that rewrite eligible raw host calls to
  fallible `try_` adapters with checked `?` propagation
- unchecked-result migration templates that add checked `?` propagation when
  the owning task can return `Result`
- unqualified imported-call style templates that rewrite simple imported calls
  to alias- or module-qualified calls before future imports can make them
  ambiguous, with a write/query/verify smoke that proves `sley fix --write`
  clears strict lint on a temp project
- unused pure binding templates that delete unread side-effect-free `bind`
  statements through checked `DeleteNode`, with lint/plan/fix-write/verify
  CLI smoke coverage
- unused pure expression statement templates that delete no-op side-effect-free
  expression statements through checked `DeleteNode`, with
  lint/plan/fix-write/verify CLI smoke coverage
- mutable binding style warnings through `mutable_binding_never_set`, plus a
  checked `convert_mutable_binding_to_bind` transaction that rewrites a
  never-set mutable local into `bind` through lint/plan/fix-write/verify CLI
  smoke coverage
- self-assignment statement warnings through `self_assignment_statement`, plus
  checked `delete_self_assignment_statement` templates that remove no-op
  `set name = name` mutations through lint/plan/fix-write/verify CLI smoke
  coverage
- overwritten set statement warnings through `overwritten_set_statement`, plus
  checked `delete_overwritten_set_statement` templates that remove dead `set`
  statements immediately overwritten before any read through
  lint/plan/fix-write/verify CLI smoke coverage
- redundant initial set warnings through `redundant_initial_set_statement`,
  plus checked `fold_redundant_initial_set_into_binding` transactions that
  fold a safe immediate `set` into the mutable initializer while preserving
  later real mutation, and checked `convert_redundant_initial_set_to_bind`
  transactions when no later mutation remains
- constant-if expression style warnings through `constant_if_expression`, plus
  checked `simplify_constant_if_expression` templates that replace
  expression-level `if true/false` with the branch that executes
- constant-if statement style warnings through `constant_if_statement`, plus
  checked `simplify_constant_if_statement` templates that replace a
  statement-level `if true/false` with its single executing branch statement
- constant-false if statement warnings through `constant_false_if_statement`,
  plus checked `delete_constant_false_if_statement` templates that remove
  never-executed `if false { ... }` statements without `else` branches through
  lint/plan/fix-write/verify CLI smoke coverage
- constant-false while statement warnings through
  `constant_false_while_statement`, plus checked
  `delete_constant_false_while_statement` templates that remove never-executed
  `while false` statements through lint/plan/fix-write/verify CLI smoke
  coverage
- constant comparison expression warnings through
  `constant_comparison_expression`, plus checked
  `simplify_constant_comparison_expression` templates that replace literal
  comparisons with their boolean result through lint/plan/fix-write/verify CLI
  smoke coverage
- constant arithmetic expression warnings through
  `constant_arithmetic_expression`, plus checked
  `simplify_constant_arithmetic_expression` templates that replace safe numeric
  literal arithmetic with its result through lint/plan/fix-write/verify CLI
  smoke coverage
- absorbing arithmetic expression warnings through
  `absorbing_arithmetic_expression`, plus checked
  `simplify_absorbing_arithmetic_expression` templates that replace delete-safe
  multiplication by zero with the zero literal through lint/plan/fix-write/verify
  CLI smoke coverage
- constant text concatenation expression warnings through
  `constant_text_concatenation_expression`, plus checked
  `simplify_constant_text_concatenation_expression` templates that replace text
  literal concatenation with one text literal through lint/plan/fix-write/verify
  CLI smoke coverage
- constant list index expression warnings through
  `constant_list_index_expression`, plus checked
  `simplify_constant_list_index_expression` templates that replace literal list
  indexes over scalar literals with the indexed literal through
  lint/plan/fix-write/verify CLI smoke coverage
- constant map index expression warnings through
  `constant_map_index_expression`, plus checked
  `simplify_constant_map_index_expression` templates that replace literal map
  indexes over scalar literal values with the indexed literal through
  lint/plan/fix-write/verify CLI smoke coverage
- constant record field access expression warnings through
  `constant_record_field_access_expression`, plus checked
  `simplify_constant_record_field_access_expression` templates that replace
  literal record field access over scalar literal values with the selected
  field literal through lint/plan/fix-write/verify CLI smoke coverage
- constant len expression warnings through `constant_len_expression`, plus
  checked `simplify_constant_len_expression` templates that replace literal
  `len` calls over literal text, literal lists, or literal maps with the
  literal length through lint/plan/fix-write/verify CLI smoke coverage
- constant not expression warnings through `constant_not_expression`, plus
  checked `simplify_constant_not_expression` templates that replace `!true` or
  `!false` with the resulting boolean literal through lint/plan/fix-write/verify
  CLI smoke coverage
- empty-if statement warnings through `empty_if_statement`, plus checked
  `delete_empty_if_statement` templates that remove no-op `if` statements with
  delete-safe conditions and empty branches through lint/plan/fix-write/verify
  CLI smoke coverage
- empty-else statement warnings through `empty_else_statement`, plus checked
  `remove_empty_else_statement` templates that remove no-op empty `else`
  branches while preserving the checked `if` condition and non-empty then
  branch through lint/plan/fix-write/verify CLI smoke coverage
- empty-for statement warnings through `empty_for_statement`, plus checked
  `delete_empty_for_statement` templates that remove `for` loops over literal
  empty lists through lint/plan/fix-write/verify CLI smoke coverage
- empty-forge statement warnings through `empty_forge_statement`, plus checked
  `delete_empty_forge_statement` templates that remove no-op `forge { }`
  starter blocks through lint/plan/fix-write/verify CLI smoke coverage
- identity binary expression style warnings through `identity_binary_expression`,
  plus checked `simplify_identity_binary_expression` templates that replace
  `x + 0`, `x * 1`, `flag && true`, `flag || false`, or empty-text
  concatenation forms with the non-identity side
- redundant boolean comparison style warnings through
  `redundant_boolean_comparison`, plus checked
  `simplify_redundant_boolean_comparison` templates that replace
  `flag == true`, `flag != false`, `flag == false`, and `flag != true` forms
  with the boolean expression or its negation
- absorbing boolean expression style warnings through
  `absorbing_boolean_expression`, plus checked
  `simplify_absorbing_boolean_expression` templates that replace
  `false && expr` and `true || expr`, or delete-safe left-side forms such as
  `expr && false` and `expr || true`, with the absorbing literal
- idempotent boolean expression style warnings through
  `idempotent_boolean_expression`, plus checked
  `simplify_idempotent_boolean_expression` templates that replace delete-safe
  `expr && expr` and `expr || expr` forms with one operand
- self-comparison expression style warnings through
  `self_comparison_expression`, plus checked
  `simplify_self_comparison_expression` templates that replace delete-safe
  `expr == expr`, `expr != expr`, `expr < expr`, `expr <= expr`,
  `expr > expr`, and `expr >= expr` forms with `true` or `false`
- double negation expression style warnings through
  `double_negation_expression`, plus checked
  `simplify_double_negation_expression` templates that replace `!!expr` with
  the inner boolean expression
- negated comparison expression style warnings through
  `negated_comparison_expression`, plus checked
  `simplify_negated_comparison_expression` templates that invert comparisons
  such as `!(limit >= 3)` into `limit < 3`
- redundant boolean-if expression style warnings through
  `redundant_boolean_if_expression`, plus checked
  `simplify_redundant_boolean_if_expression` templates that replace
  `if flag { true } else { false }` with `flag` and the inverted form with
  `!flag`
- redundant boolean-if statement style warnings through
  `redundant_boolean_if_statement`, plus checked
  `simplify_redundant_boolean_if_statement` templates that replace
  `if flag { return true } else { return false }` with `return flag` and the
  inverted form with `return !flag`
- same-branch if expression style warnings through
  `same_branch_if_expression`, plus checked
  `simplify_same_branch_if_expression` templates that replace
  `if condition { value } else { value }` with `value` when the condition is
  delete-safe
- same-branch if statement style warnings through
  `same_branch_if_statement`, plus checked
  `simplify_same_branch_if_statement` templates that replace an `if` statement
  whose branches contain the same single statement when the condition is
  delete-safe
- unreachable statement warnings through `unreachable_statement`, plus checked
  `delete_unreachable_statement` templates that delete dead statements after a
  guaranteed return
- private declaration hygiene through the checked `unused_private_type` and
  `unused_private_effect` lint rules
- import hygiene through the checked `unused_import` lint rule
- API hygiene through the checked `unused_take` lint rule
- deterministic seeded execution for `FileRead`, `FileWrite`, `DatabaseRead`,
  `DatabaseWrite`, `Network`, `Shell`, `ModelCall`, `SecretRead`, `Deploy`,
  and `Spend`, including manifest-backed scoped acceptance and scope-denial
  diagnostics for every non-file seeded host adapter
- local deploy package dry-runs that prove verify, seal, and ZJX package
  summaries without live deployment authority and can write explicit local
  report/seal/package handoff artifacts with a digest manifest plus a
  deterministic reinspection command
- temp-directory execution and temp setup files for write-mode cases so release
  tests do not mutate the repo checkout

`sley-ci examples --json examples` broadens the release gate into packaged
examples. It checks every `sley.toml` example as a project root, checks
standalone `.sley` examples directly, and formatter-round-trips every shipped
`.sley` source under `examples/`.

## Improvement 9: Checked Graph Query Reports

`sley query` is the first explicit tooling command on top of the checked symbol
graph. Unlike `sley graph`, it runs the checker before emitting the report, so
tool consumers do not treat an invalid program as a reliable semantic surface.

The v0 query report carries `schema: "sley.query.report.v0"` and supports:

- `--kind all|modules|tasks|types|effects|calls`
- `--module <module>` filtering
- `--exported` filtering for declaration and task summaries
- task rows with stable ids, qualified names, takes, return types, declared
  effects, and inbound/outbound call counts
- type rows with stable ids, qualified names, rendered type values, and record
  fields
- effect rows with stable ids and qualified names
- strict call rows with caller, expression, source, callee, status, optional
  target, and optional candidates

Unknown module filters fail with `QUERY_MODULE_FILTER_NOT_FOUND` instead of
returning an empty successful query report.

This is the immediate substrate for lints, migration hints, project dashboards,
and eventually non-authoritative Sley helper passes.

## Improvement 10: Checked Lint Reports

`sley lint` is the first checked helper command built from the graph query
surface. It runs the checker before linting, emits
`schema: "sley.lint.report.v0"`, and keeps warning-grade lint output separate
from hard checker diagnostics.
Unknown module filters fail with `LINT_MODULE_FILTER_NOT_FOUND` instead of
returning a clean lint report for an empty slice.

The v0 lint rules are:

- `unused_private_task`: a non-exported task with zero inbound checked calls is
  reported unless it is the entry module's `main` task.
- `unreachable_private_task`: a non-exported task with inbound private calls is
  reported when it is not reachable from `main` or any exported task.
- `unused_declared_effect`: a task-declared effect is reported when no direct
  host call or resolved called-task effect justifies it.
- `unused_import`: an import is reported when no checked task, type, or effect
  uses it.
- `unused_take`: a normal task take is reported when the task body never reads
  it.
- `unused_private_type`: a non-exported type is reported when no checked task,
  type, or record literal references it.
- `unused_private_effect`: a non-exported effect is reported when no checked
  task declares it.
- `raw_host_adapter`: a legacy diagnostic-failing host call is reported when a
  fallible `try_` adapter should replace it.
- `missing_module_declaration`: a source file that relies on the implicit
  `main` module is reported so deployable/project code gets stable graph ids.
- `unchecked_result`: an expression statement that discards a fallible host or
  user-task `Result` is reported so agents propagate, return, or bind the
  failure path explicitly.
- `unchecked_result_binding`: a local `bind` that captures a fallible host or
  user-task `Result` and is never read is reported so agents do not silently
  park a recoverable failure path.
- `unused_effectful_binding`: a local `bind` that unwraps a fallible host or
  user-task `Result` with `?` but never reads the bound value is reported so
  agents can keep the effect explicit without leaving dead recovered values.
- `unqualified_imported_call`: a resolved call to an imported task through a
  simple name is reported so agents qualify it through the import alias or
  module segment before future imports can change name resolution.
- `unused_pure_binding`: an unread local `bind` with a delete-safe initializer
  is reported so agents can remove dead local scaffolding without changing
  authority work or fallible execution.
- `unused_pure_expression_statement`: a delete-safe pure expression statement
  is reported so agents can remove no-op local computations without dropping
  authority work or fallible execution.
- `mutable_binding_never_set`: a mutable local such as `state` or `tally` is
  reported when it is never assigned with `set`; prefer `bind` unless real
  mutation is needed.
- `self_assignment_statement`: a no-op `set name = name` mutation is reported
  so agents can delete it without dropping a real state update.
- `overwritten_set_statement`: a delete-safe `set name = ...` immediately
  followed by another `set name = ...` is reported so agents can remove dead
  local mutation noise before deploy verification.
- `redundant_initial_set_statement`: a mutable local initialized and then
  immediately assigned a delete-safe replacement is reported so agents can
  fold the replacement into the initializer when later real mutation remains.
- `constant_if_expression`: an expression-level `if true/false { ... } else { ... }`
  is reported so agents can replace it with the branch that actually executes.
- `constant_if_statement`: a statement-level `if true/false { ... } else { ... }`
  with one statement in the executing branch is reported so agents can replace
  the whole control-flow wrapper with that checked statement.
- `constant_false_if_statement`: a statement-level `if false { ... }` without
  an `else` branch is reported so agents can delete never-executed dead
  branches without dropping runtime work.
- `constant_false_while_statement`: a `while false { ... }` statement is
  reported so agents can remove never-executed loop bodies without touching
  authority work that would otherwise run.
- `constant_comparison_expression`: a checked literal comparison such as
  `1 < 2`, `3 >= 7`, or `"a" == "b"` is reported so agents can replace it
  with its boolean result without overlapping boolean-literal or
  self-comparison style rules.
- `constant_arithmetic_expression`: checked numeric literal arithmetic such as
  `2 + 3`, `10 - 4`, or `6 / 2.0` is reported so agents can replace it with
  its numeric result without overlapping identity-expression cleanup or
  folding divide-by-zero.
- `constant_text_concatenation_expression`: checked text literal concatenation
  such as `"Sley " + "agents"` is reported so agents can replace it with one
  escaped text literal before runtime work is involved.
- `constant_list_index_expression`: checked literal list indexing such as
  `[10, 20, 30][1]` is reported when the list contains only scalar literals and
  the nonnegative integer index is in range, so agents can replace it with the
  selected literal without dropping runtime work.
- `constant_map_index_expression`: checked literal map indexing such as
  `map { "one": "ready" }["one"]` is reported when all keys are text literals,
  all values are scalar literals, and the indexed key is present, so agents can
  replace it with the selected literal without dropping runtime work.
- `constant_record_field_access_expression`: checked literal record field
  access such as `User { name: "Ada" }.name` is reported when the receiver is a
  record literal with scalar literal values and the field is present, so agents
  can replace it with the selected literal without dropping runtime work.
- `constant_len_expression`: checked literal `len` calls such as
  `len(["a", "b"])` are reported when the argument is a literal text, a list
  with scalar literal items, or a map with text literal keys and scalar literal
  values, so agents can replace it with the literal length without dropping
  runtime work.
- `constant_not_expression`: checked literal boolean negation such as `!true`
  is reported so agents can replace it with the resulting boolean literal
  without dropping runtime work.
- `empty_if_statement`: an `if` statement with a delete-safe condition and
  empty branches is reported so agents can delete no-op control flow without
  dropping calls, indexing, `?`, division, or remainder work.
- `empty_else_statement`: an `if` statement with a non-empty then branch and
  empty `else` branch is reported so agents can remove no-op fallback syntax
  without changing the checked branch that can execute.
- `empty_for_statement`: a `for item in [] { ... }` statement is reported so
  agents can remove never-executed loop bodies without evaluating or dropping
  any non-empty collection expression.
- `empty_forge_statement`: a no-op `forge { }` statement is reported so
  agents can delete placeholder starter blocks before readiness or deploy gates.
- `identity_binary_expression`: a checked identity binary expression such as
  `x + 0`, `x * 1`, `flag && true`, `flag || false`, `"" + name`, or
  `name + ""` is reported so agents can replace it with the non-identity side.
- `redundant_boolean_comparison`: a checked comparison against `true` or
  `false` is reported so agents can replace it with the boolean expression or
  its negation.
- `absorbing_boolean_expression`: a checked `false && expr`, `true || expr`,
  delete-safe `expr && false`, or delete-safe `expr || true` form is reported
  so agents can replace it with the absorbing literal without dropping
  authority work or recoverable failures.
- `idempotent_boolean_expression`: a checked delete-safe `expr && expr` or
  `expr || expr` form is reported so agents can replace it with one operand
  without dropping authority work or recoverable failures.
- `absorbing_arithmetic_expression`: a checked multiplication by zero with a
  delete-safe nonliteral side is reported so agents can replace it with the
  zero literal without dropping authority work or recoverable failures.
- `self_comparison_expression`: a checked delete-safe `expr == expr`,
  `expr != expr`, `expr < expr`, `expr <= expr`, `expr > expr`, or
  `expr >= expr` form is reported so agents can replace it with the constant
  boolean result without dropping authority work or recoverable failures.
- `double_negation_expression`: a checked `!!expr` form is reported so agents
  can replace it with the inner boolean expression.
- `negated_comparison_expression`: a checked `!(left op right)` comparison is
  reported so agents can replace it with the inverse comparison without
  changing operand evaluation.
- `redundant_boolean_if_expression`: a checked expression-level
  `if flag { true } else { false }` or inverted boolean branch pair is
  reported so agents can replace it with the condition or its negation.
- `redundant_boolean_if_statement`: a checked statement-level
  `if flag { return true } else { return false }` or inverted boolean return
  pair is reported so agents can replace it with a direct return of the
  condition or its negation.
- `same_branch_if_expression`: a checked expression-level `if` with identical
  branch source is reported when the condition is delete-safe, so agents can
  replace it with either branch without dropping authority work or recoverable
  failures.
- `same_branch_if_statement`: a checked statement-level `if` with identical
  single-statement branches is reported when the condition is delete-safe, so
  agents can replace the control-flow wrapper with either branch statement.
- `unreachable_statement`: a statement after a guaranteed `return` in the same
  block is reported so agents can remove dead task-body code.

The command supports `--module <module>`, `--rule unused-private-task`,
`--rule unreachable-private-task`, `--rule unused-declared-effect`,
`--rule unused-import`, `--rule unused-take`, `--rule unused-private-type`,
`--rule unused-private-effect`, `--rule raw-host-adapter`,
`--rule missing-module-declaration`, `--rule unchecked-result`,
`--rule unchecked-result-binding`,
`--rule unused-effectful-binding`,
`--rule unqualified-imported-call`, `--rule unused-pure-binding`,
`--rule unused-pure-expression-statement`,
`--rule mutable-binding-never-set`, `--rule self-assignment-statement`,
`--rule overwritten-set-statement`, `--rule redundant-initial-set-statement`,
`--rule constant-if-expression`,
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
`--rule identity-binary-expression`, `--rule redundant-boolean-comparison`,
`--rule absorbing-boolean-expression`, `--rule idempotent-boolean-expression`,
`--rule self-comparison-expression`,
`--rule double-negation-expression`, `--rule negated-comparison-expression`,
`--rule redundant-boolean-if-expression`,
`--rule redundant-boolean-if-statement`, `--rule same-branch-if-expression`,
`--rule same-branch-if-statement`,
`--rule unreachable-statement`,
`--rule absorbing-arithmetic-expression`,
and
`--deny-warnings` lets CI turn findings into a failing exit after the JSON
report is printed.

This is not production lint coverage yet. It is the first stable surface for
agent-facing hygiene, authority lints, migration hints, and non-authoritative
Sley helper passes such as `sley-shadow` that consume `sley.query.report.v0`
and `sley.lint.report.v0`.

## Improvement 11: ZJX Boundary Discipline

ZJX should be the transport/cache envelope, not the semantic identity, until the
canonical graph encoding and canonical ZJX encoding are frozen.

Correct pipeline:

```text
canonical graph bytes -> semantic hash
canonical graph bytes -> ZJX envelope
ZJX envelope -> transport/cache/storage
```

Do not make the compressed envelope define the language semantics too early.
That would couple language correctness to compression implementation details.

## Improvement 12: Safety Against Bad Self-Evolution

The recursive loop needs brakes.

Hard gates:

- no accepted graft without parse/check success
- no compiler change without conformance tests
- no ontology change without human ratification
- no authority/effect change without explicit semantic reason
- no public claim that Sley is production-ready until runtime and host gates are
  mature
- no self-hosted pass becomes authoritative until it matches the Rust oracle on
  a large fixture set

Failure modes to watch:

- agents adding effects just to silence diagnostics
- binding vocabulary bloat
- unstable JSON schemas
- text edits bypassing graft provenance
- ZJX format churn becoming language churn
- self-hosted passes matching happy paths but missing rejection cases

## Practical Next Milestones

Near-term:

1. Harden capability-backed host adapters beyond root-scoped filesystem gates,
   deterministic seeded database surfaces, seeded network text, seeded shell
   command output, seeded model completions, seeded secret values, seeded
   deployment stage results, and seeded spend authorizations. Database host
   adapters now require exact table capability scopes; URL capability scopes
   match exact URLs or path boundaries; the remaining non-file seeded host
   adapters support deterministic exact-or-delimiter text capability scopes over
   secret names, shell commands, model prompts, deploy targets, and spend
   requests.
   Preserve `Result<T, Error>` surfaces for recoverable host failures and keep
   authority failures as diagnostics.
2. Grow the accepted/rejected synthetic gold corpus and CLI smoke manifest with
   graft, module, and runtime authority cases.
3. Extend `sley-shadow` and other deterministic helpers that consume
   `sley.query.report.v0` and `sley.lint.report.v0`, then broaden authority,
   style, and migration lints.
4. Extend graph-slice grafts around insert, move, delete, and replace planning.
5. Harden graph-slice grafts for broader graph-contract checks.

Medium-term:

1. Extend graph-slice grafts beyond call-site, statement, expression, move,
   delete, and replace edits.
2. Move trace seals and ZJX preview payloads into a compressed binary `.zjx`
   handoff.
3. Expand runtime host capability values beyond seeded v0 adapters: deploy
   beyond seeded stage results, spend beyond seeded authorization text, secret
   beyond seeded values, model beyond seeded completions, shell beyond seeded
   command output, network beyond seeded text, and database write beyond
   per-run inserts.
4. Broaden release gates beyond the current corpus, packaged examples, and CLI
   smokes into migration fixtures and the final public-release metadata cut.
5. Shadow selected compiler helper passes in Sley.

Long-term:

1. Write Sley lints and graph query helpers in Sley.
2. Shadow Rust checker passes with Sley equivalents.
3. Promote self-hosted passes only after conformance evidence.
4. Keep a frozen recovery oracle.

## Bottom Line

Keeping the first Loom in Rust is not a retreat from Sley. It is how Sley gets a
strict, trustworthy teacher while agents learn the language.

The path is:

```text
Rust bootstrap -> structural agent loop -> gold corpus -> Sley helper passes ->
shadow compiler passes -> promoted self-hosting -> recovery oracle
```

That is the safe version of an agent-native language that can eventually improve
itself without turning into unreviewable model-written mush.
