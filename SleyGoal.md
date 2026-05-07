Make Sley the best programming language in the world for agents. In this
thread, optimize for agent success rate, structural edit safety, compiler
strictness, readable source projection, stable machine-readable contracts,
deterministic execution, explicit authority gates, checked query/lint surfaces,
manifest-backed conformance, and a credible self-hosting path. Do not optimize
for syntax novelty unless it improves the compiler-mediated agent workflow.

Work in `/home/greyforge/sley`.

Read these first:

- `README.md`
- `llms.txt`
- `docs/SleyLanguageSpec.md`
- `docs/BrandingAssets.md` when touching public-facing Sley identity, images,
  profile surfaces, banners, or post art
- `SleyCompiler.md`
- `SleyImprove.md`
- `Cargo.toml`
- `src/parser.rs`
- `src/ast.rs`
- `src/checker.rs`
- `src/runtime.rs`
- `src/graft.rs`
- `src/symbols.rs`
- `src/query.rs`
- `src/lint.rs`
- `src/trace.rs`
- `src/zjx.rs`
- `src/main.rs`
- `tests/sley_v0.rs`
- `Makefile`
- `.github/actions/sley-v1/action.yml`
- `.github/workflows/v1.yml`
- `.pre-commit-config.yaml`
- `docs/schemas/*.schema.json`
- `fixtures/contracts/*.json`
- `fixtures/cli_smokes/manifest.json`
- `fixtures/corpus/**/*.json`
- `fixtures/grafts/*.json`
- `examples/*.sley`

Current verified surface:

- `cargo fmt -- --check` passes.
- `cargo test` passes.
- Current integration coverage is 324 tests in the core `tests/sley_v0.rs`
  conformance file, with additional focused integration tests for LSP,
  workbench, agent bench, docgen, migration reports, sandbox replay, project
  templates, and ZJX envelope tools. The focused utility tests validate live
  JSON reports against their declared schemas.
- `make v1` wraps fmt, whitespace diff check, full Rust tests, contract
  fixture and release-manifest validation, conformance summary reporting,
  corpus conformance, packaged example conformance, broad CLI smoke
  conformance, the lightweight `sley-ci smoke` wrapper probe, the focused LSP
  integration tests, VS Code editor-shim validation, deterministic utility
  replays for workbench, agent-bench, raw-host, imported-call naming,
  unchecked-result expression, and unchecked-result binding migration,
  docgen, sandbox-runner, and ZJX tools, and Tree-sitter syntax parsing.
- The synthetic gold corpus currently has 20 accepted fixtures and 22 rejected
  fixtures, including accepted/rejected split-task agent authority cases that
  lock transitive effect propagation for deploy, spend, and data mutation, plus
  result-flow fixtures that lock `?` propagation acceptance and
  `QUESTION_REQUIRES_RESULT` rejection, module namespace fixtures that lock
  exported declaration success, transparent non-record type alias fixtures, and
  duplicate type/effect/task diagnostics.
- `.github/actions/sley-v1/action.yml`, `.github/workflows/v1.yml`, and
  `.pre-commit-config.yaml` run the `make v1` gate so local and hosted checks
  use the same release surface.
- `sley query --json` emits `schema: "sley.query.report.v0"` and supports
  `--kind all|modules|tasks|types|effects|calls`, `--module <module>`, and
  `--exported`, including strict task/take/type/effect/call row definitions.
  Unknown module filters fail with `QUERY_MODULE_FILTER_NOT_FOUND` instead of
  producing empty successful reports.
- `sley run --json` emits `schema: "sley.run.report.v0"` with a strict
  recursive runtime value payload and empty diagnostics on successful
  deterministic execution. Verify reports reuse the same strict value contract
  when embedding runtime results. Runtime gates use `--cap EFFECT[=SCOPE]`;
  file effects treat the scope as a filesystem root, database host adapters
  require exact table scopes, URL scopes match exact URLs or path boundaries,
  and the remaining non-file seeded host adapters enforce deterministic
  exact-or-delimiter text scopes over secret names, shell commands, model
  prompts, deploy targets, and spend requests. Scope mismatches remain
  authority diagnostics.
- `sley-contract` is available as an in-tree contract utility scaffold with
  `inventory`, `check-fixtures`, `validate`, and `inspect-deploy-artifacts`
  JSON Schema validation commands over `docs/schemas/`, `fixtures/contracts/`,
  release manifests, and local deploy artifact directories. Contract inventory
  currently tracks 38 schemas, 116 contract fixtures, and 119 schema instances.
- `sley-ci` is available as an in-tree CI wrapper with `check`, `lint`,
  `doctor`, `plan`, `run`, `verify`, `deploy`, `smoke`, `corpus`, and
  `examples` commands that emit
  `schema: "sley.ci.report.v0"` over
  existing Sley check/lint/doctor/plan/run/verify/deploy, CLI smoke manifest,
  accepted/rejected corpus, and packaged example gates, including deploy
  artifact directory pass-through. `sley-ci smoke` and `sley-ci corpus` accept
  either explicit `manifest.json` files or manifest directories, and
  `sley-ci lint` is smoke-pinned for strict module-filter failures with the
  wrapped diagnostic ID summarized inside the CI step; denied lint findings
  from lint, doctor, plan, and verify surfaces also expose their finding IDs
  inside the step.
- `sley-conformance` is available as an in-tree conformance visibility helper
  with `report` and `coverage` commands that emit
  `schema: "sley.conformance.report.v0"` and
  `schema: "sley.conformance.coverage.v0"` over schema/fixture instances,
  contract validation status, checked migration fixture counts, release
  manifests, corpus tags, smoke tags, the compact agent onboarding pack, and
  packaged example counts, plus editor-shim package validation and a check that
  the declared integration coverage count matches the test file. It also
  inventories the `make v1` target set and emits local/public v1 readiness
  tracks with gated-check completion percentages. It fails when required local
  gate targets disappear. `report` now fails when
  required corpus or smoke release tags disappear, including seeded and scoped
  runtime authority smoke tags, strict module-filter diagnostic tags, and
  `ci:lint` wrapper coverage.
  `report --require-public-release-ready` turns
  unresolved public-release packaging blockers into an explicit nonzero gate
  for final release cuts, and the regular report carries next actions for the
  operator-controlled metadata decisions. `--corpus-manifest` and repeated
  `--smoke-manifest` arguments accept either explicit manifest files or
  directories containing `manifest.json`.
- `tree-sitter-sley` is available as an in-tree syntax grammar bootstrap with
  `npm test` coverage for Tree-sitter parser generation, exact syntax corpus
  trees, highlight query validation, and parsing of current `.sley` examples
  plus accepted compiler corpus fixtures.
- `sley-lsp` is available as an in-tree stdio language-server bootstrap with
  full-document sync, project-aware diagnostics over `sley.toml` workspaces
  with open-buffer overlays, all-open-buffer and watched project-file
  diagnostic refresh, formatting, document symbols, declaration metadata and
  resolved project task-call hover, folding ranges, semantic tokens,
  workspace symbols, selection ranges, project completions, import/call
  definition jumps, project task signature help, project task parameter inlay
  hints, exact-range task references, document highlights,
  import document links, prepared cursor-aware project task rename edits,
  checked edit-plan code actions, non-mutating command-preview code lenses,
  and non-mutating preview commands whose editor repair previews reuse the
  strict edit-plan graft contracts.
- `editors/vscode-sley` is available as a private local VS Code shim that
  contributes `.sley` language metadata, basic TextMate highlighting, and a
  `vscode-languageclient` bridge to the current `sley-lsp` server. It is
  validation-gated locally and inventoried by `sley-conformance report`, but
  remains unpublished until public release metadata is approved.
- `sley-workbench` is available as an in-tree local inspection bootstrap with
  JSON and optional static HTML panels over doctor, query, lint, edit-plan, and
  graph and graph-slice data; its report schema links embedded doctor, query,
  lint, edit-plan, graph, and graph-slice panels back to their source
  contracts, and edit-plan template rows carry preview, write, and post-fix
  gate commands plus a local repair-focus selector for repair handoff.
- `sley-docgen` is available as an in-tree checked reference generator with
  `schema: "sley.docgen.report.v0"` and optional Markdown over module, task,
  type, effect, and host capability docs from `sley.query.report.v0`,
  including seeded `--cap` args for deterministic host setup; its schema links
  generated task/type/effect rows back to the strict query row definitions, and
  project-root references can be narrowed with checked module/export filters
  that block unknown modules explicitly.
- `sley-shadow` is available as an in-tree non-authoritative helper replay with
  `schema: "sley.shadow.report.v0"` over checked `sley.query.report.v0` and
  `sley.lint.report.v0` data for single-file and project-root targets; it
  supports module- and rule-scoped replay, links lint findings to query rows,
  blocks unknown modules explicitly, and derives seeded `--cap` args for
  effectful tasks while leaving Rust Loom as the semantic oracle.
- `sley-agent-bench` is available as an in-tree deterministic repair-loop
  benchmark with `schema: "sley.agent_bench.report.v0"` over JSON inspection,
  lint failure, checked edit-plan repair selection, `sley fix --write`, strict
  post-fix lint/verify gates, trace receipts, seal digests, and ZJX handoff
  evidence.
- `sley-migrate` is available as an in-tree checked migration report utility
  with `schema: "sley.migrate.report.v0"` over module declaration insertion,
  raw host adapter migration, imported-call naming cleanup,
  unchecked-result expression and binding propagation candidates, and optional
  schema/fixture drift
  reports; checked migration operations reuse the strict edit-plan graft
  operation schema.
- `sley-sandbox-runner` is available as an in-tree deterministic replay
  utility with `schema: "sley.sandbox.report.v0"` over
  `schema: "sley.sandbox.manifest.v0"` manifests that seed capabilities,
  files, tables, secrets, network text, shell output, model output, deploy
  results, and spend results without external provider calls.
- `sley-zjx` is available as an in-tree read-only envelope utility with
  `validate`, `inspect`, `verify-digest`, `extract-graph`, and
  `diff-envelope` commands over preview ZJX JSON envelopes and
  `schema: "sley.zjx.tool.report.v0"`.
- `sley deploy --json --dry-run` emits `schema: "sley.deploy.report.v0"` and
  composes strict verify, trace seal, and ZJX package summaries into a
  local-only deploy package report that forbids live deployment, provider
  calls, external mutation, and spend without explicit operator approval.
  `--artifacts-dir <dir>` writes local `deploy-report.json`, `seal.json`,
  `zjx-envelope.json`, and digest-bearing `manifest.json` handoff files after
  the dry-run package is ready; artifact manifests pin report, seal, and
  package file roles to their expected schemas, and the ready report points
  agents to `sley-contract inspect-deploy-artifacts` to revalidate that handoff
  directory after it moves between agent sessions.
- `sley new --json --template hello|library|cli|service-gate|data-pipeline|deploy|spend-gate|agent|agent-task-pack|agent-project`
  emits `schema: "sley.project.scaffold.v0"` and creates deterministic
  first-run starter projects, including pure library/CLI/data-pipeline
  quickstarts, seeded network service, deploy, and spend quickstarts, and
  seeded single-module and multi-module agent quickstarts that compose SecretRead, Network,
  ModelCall, and Deploy authority without
  real providers.
- The generated multi-module `agent-project` quickstart path is smoke-pinned
  from scaffold through check, doctor, query, plan, lint, seeded run,
  warning-denying verify, dry-run deploy artifact packaging, and artifact
  contract inspection.
- `sley lint --json` emits `schema: "sley.lint.report.v0"` and supports
  `--module <module>`, `--rule unused-private-task`,
  `--rule unreachable-private-task`, `--rule unused-declared-effect`,
  `--rule unused-import`, `--rule unused-take`, `--rule unused-private-type`,
  `--rule unused-private-effect`, `--rule raw-host-adapter`,
  `--rule missing-module-declaration`, `--rule unchecked-result`,
  `--rule unchecked-result-binding`,
  `--rule unused-effectful-binding`,
  `--rule unqualified-imported-call`, `--rule unused-pure-binding`,
  `--rule unused-pure-expression-statement`,
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
  `--rule identity-binary-expression`,
  `--rule redundant-boolean-comparison`,
  `--rule absorbing-boolean-expression`,
  `--rule idempotent-boolean-expression`,
  `--rule self-comparison-expression`,
  `--rule double-negation-expression`,
  `--rule negated-comparison-expression`,
  `--rule redundant-boolean-if-expression`,
  `--rule redundant-boolean-if-statement`,
  `--rule same-branch-if-expression`,
  `--rule same-branch-if-statement`, `--rule unreachable-statement`,
  `--rule absorbing-arithmetic-expression`,
  `--rule empty-while-statement`, and
  `--deny-warnings`. Unknown module filters fail with
  `LINT_MODULE_FILTER_NOT_FOUND` instead of producing clean empty-slice reports.
- The current lint rules are `unused_private_task` and
  `unreachable_private_task`, `unused_declared_effect`, and
  `unused_import`, `unused_take`, `unused_private_type`,
  `unused_private_effect`, `raw_host_adapter`,
  `missing_module_declaration`, `unchecked_result`,
  `unchecked_result_binding`, `unused_effectful_binding`, and
  `unqualified_imported_call`, `unused_pure_binding`,
  `unused_pure_expression_statement`, and `mutable_binding_never_set`,
  `constant_if_expression`,
  `constant_if_statement`,
  `constant_false_if_statement`,
  `constant_false_while_statement`,
  `constant_comparison_expression`,
  `constant_arithmetic_expression`,
  `constant_text_concatenation_expression`,
  `constant_list_index_expression`,
  `constant_map_index_expression`,
  `constant_record_field_access_expression`,
  `constant_len_expression`,
  `constant_not_expression`,
  `empty_if_statement`,
  `empty_else_statement`,
  `empty_for_statement`,
  `empty_forge_statement`,
  `identity_binary_expression`, `redundant_boolean_comparison`, and
  `absorbing_boolean_expression`, `idempotent_boolean_expression`,
  `self_comparison_expression`,
  `double_negation_expression`, `negated_comparison_expression`,
  `redundant_boolean_if_expression`,
  `redundant_boolean_if_statement`, `same_branch_if_expression`, and
  `same_branch_if_statement`, and `unreachable_statement`, and
  `absorbing_arithmetic_expression`, `self_assignment_statement`, and
  `overwritten_set_statement`, and `redundant_initial_set_statement`, and
  `empty_while_statement`.
- CLI smoke coverage is manifest-backed under
  `fixtures/cli_smokes/manifest.json`, with a separate
  `fixtures/ci_smoke_probe/manifest.json` contract for the `sley-ci smoke`
  wrapper over parse, query, graft dry-run, and seeded multi-capability agent
  runtime authority. The broad suite can exercise allowlisted sibling utility
  binaries and now smokes `sley-contract` inventory, validate, fixture-check,
  deploy-artifact inspection flows, plus `sley-migrate` raw-host,
  imported-call naming cleanup, schema-drift, unchecked-result expression, and
  unchecked-result binding migration reports, plus `sley-ci`, `sley-conformance`,
  `sley-docgen`, `sley-workbench`, `sley-sandbox-runner`, `sley-shadow`,
  `sley-agent-bench`, `sley-zjx` utility reports, and `sley-lsp` help/startup. It also includes
  graph-slice insert and replace affordances, checked `insert_statement`,
  `replace_statement`, and `replace_expression` graft templates, task-body
  insert graft emission, direct block, statement, take, and expression node
  surface targeting with expression and statement `--emit-graft`, direct
  statement-surface delete graft emission,
  program-surface declaration/import templates for checked `add_task`,
  `add_type_declaration`, `add_effect_declaration`, and `add_import` starters, direct
  program-surface `add_task` graft emission, exact node-surface
  `sley fix --dry-run` previews including nested block and statement nodes,
  checked `sley fix --name`, `--type`, `--module`, `--source`, `--source-file`, and
  `--position` payload overrides, unsupported override diagnostics, and
  lint-driven declaration delete templates and cleanup transactions, including
  direct declaration surface targeting, direct graft JSON emission, and checked
  `sley fix` dry-run execution from `sley plan --graft-templates`, plus
  lint-driven missing-module declaration templates with target/project-aware
  module-name inference and checked fix dry runs, checked raw-host adapter
  migration templates that rewrite eligible raw calls to fallible `try_`
  calls with `?`, and typed starter, service, deploy, and agent scaffold
  next-actions whose generated first-run sequences are executed by tests,
  including strict seeded `sley verify --json --deny-warnings` readiness smokes
  and local `sley deploy --json --dry-run` package reports for generated gated
  projects and `sley-ci run`/`sley-ci verify`/`sley-ci deploy` handoffs for the
  seeded agent projects, plus warning-state doctor/verify next-actions that route to
  `sley plan --json --graft-templates` plus unambiguous
  `sley fix --dry-run` previews carrying explicit `write_command` vectors with
  a staged write-and-verify smoke for a previewed repair plus a project-level
  previewed unused-import write-and-verify smoke and generated scaffold
  quickstarts re-verified with local or seeded authority, plus ready-state
  doctor `verify_gate` next-actions with seeded caps before entrypoint runs, plus
  scaffold-level and passed-verify seal/ZJX handoff next-actions, doctor/plan
  call-bearing reports that route agents to strict call-row inspection, a
  write/query/verify smoke for the call-row-driven rename-and-update-call-sites
  transaction, a write/query/verify smoke for the unused-take-plus-call-arg
  removal transaction, explicit deploy artifact manifest handoff writes plus
  digest/schema reinspection for dry-run packages, and checked
  `unchecked_result` migration templates that turn discarded `Result`
  expression statements into explicit `?` propagation when valid, plus checked
  `propagate_unchecked_result_binding` templates that rewrite unread fallible
  `Result` bindings as `?` expression statements, and checked
  `drop_unused_effectful_binding_value` templates that drop unread already
  checked fallible-call bindings, plus checked
  `unqualified_imported_call` style templates that qualify imported task calls
  through their import alias or module segment, including a write/query/verify
  smoke that proves the repair clears strict lint, plus checked `unused_import`
  and
  `unused_private_task` delete templates, fix dry runs, and a write-mode
  `delete_unused_import` cleanup that clears lint before
  `sley verify --deny-warnings`, plus checked `unused_pure_binding`
  `DeleteNode` templates with lint/plan/fix-write/verify smoke coverage, plus
  checked `unused_pure_expression_statement` `DeleteNode` templates with
  lint/plan/fix-write/verify smoke coverage, plus
  checked `mutable_binding_never_set` style findings and a
  `convert_mutable_binding_to_bind` transaction with plan/fix-write/verify
  smoke coverage, plus checked `self_assignment_statement` findings and
  `delete_self_assignment_statement` templates with lint/plan/fix-write/verify
  smoke coverage, plus checked `overwritten_set_statement` findings and
  `delete_overwritten_set_statement` templates with lint/plan/fix-write/verify
  smoke coverage, plus checked `redundant_initial_set_statement` findings and
  `fold_redundant_initial_set_into_binding` and
  `convert_redundant_initial_set_to_bind` transactions with lint/plan/fix-write/verify
  smoke coverage, plus checked `constant_if_expression` style findings and
  `simplify_constant_if_expression` templates with lint/plan/fix-write/verify
  smoke coverage, plus checked `constant_if_statement` style findings and
  `simplify_constant_if_statement` templates with lint/plan/fix-write/verify
  smoke coverage, plus checked `constant_false_if_statement` dead-branch
  findings and `delete_constant_false_if_statement` templates with
  lint/plan/fix-write/verify
  smoke coverage, plus checked `constant_false_while_statement` dead-loop
  findings and `delete_constant_false_while_statement` templates with
  lint/plan/fix-write/verify
  smoke coverage, plus checked `constant_comparison_expression` style findings
  and `simplify_constant_comparison_expression` templates with
  lint/plan/fix-write/verify
  smoke coverage, plus checked `constant_arithmetic_expression` style findings
  and `simplify_constant_arithmetic_expression` templates with
  lint/plan/fix-write/verify
  smoke coverage, plus checked `absorbing_arithmetic_expression` style findings
  and `simplify_absorbing_arithmetic_expression` templates with
  lint/plan/fix-write/verify
  smoke coverage, plus checked `constant_text_concatenation_expression` style
  findings and `simplify_constant_text_concatenation_expression` templates with
  lint/plan/fix-write/verify
  smoke coverage, plus checked `constant_list_index_expression` style findings
  and `simplify_constant_list_index_expression` templates with
  lint/plan/fix-write/verify
  smoke coverage, plus checked `constant_map_index_expression` style findings
  and `simplify_constant_map_index_expression` templates with
  lint/plan/fix-write/verify
  smoke coverage, plus checked `constant_record_field_access_expression` style
  findings and `simplify_constant_record_field_access_expression` templates
  with lint/plan/fix-write/verify
  smoke coverage, plus checked `constant_len_expression` style findings and
  `simplify_constant_len_expression` templates with lint/plan/fix-write/verify
  smoke coverage, plus checked `constant_not_expression` style findings and
  `simplify_constant_not_expression` templates with lint/plan/fix-write/verify
  smoke coverage, plus checked `empty_if_statement` no-op control-flow
  findings and `delete_empty_if_statement` templates with
  lint/plan/fix-write/verify smoke coverage, plus checked
  `empty_else_statement` no-op else-branch findings and
  `remove_empty_else_statement` templates with lint/plan/fix-write/verify
  smoke coverage, plus checked
  `empty_for_statement` dead-loop findings and
  `delete_empty_for_statement` templates with lint/plan/fix-write/verify
  smoke coverage, plus checked `empty_forge_statement` no-op block findings and
  `delete_empty_forge_statement` templates with lint/plan/fix-write/verify
  smoke coverage, plus checked `empty_while_statement` warning coverage for
  empty loops with delete-safe pure conditions, plus checked
  `identity_binary_expression` style findings and
  `simplify_identity_binary_expression` templates with lint/plan/fix-write/verify
  smoke coverage, including empty-text concatenation cleanup, plus checked
  `redundant_boolean_comparison` style findings and
  `simplify_redundant_boolean_comparison` templates with lint/plan/fix-write/verify
  smoke coverage, plus checked `absorbing_boolean_expression` style findings
  and `simplify_absorbing_boolean_expression` templates with lint/plan/fix-write/verify
  smoke coverage, plus checked `idempotent_boolean_expression` style findings
  and `simplify_idempotent_boolean_expression` templates with
  lint/plan/fix-write/verify smoke coverage, plus checked
  `self_comparison_expression` style findings and
  `simplify_self_comparison_expression` templates, including strict and
  non-strict self-ordering cleanup, with lint/plan/fix-write/verify
  smoke coverage, plus checked `double_negation_expression` style findings and
  `simplify_double_negation_expression` templates with lint/plan/fix-write/verify
  smoke coverage, plus checked `negated_comparison_expression` style findings
  and `simplify_negated_comparison_expression` templates with
  lint/plan/fix-write/verify
  smoke coverage, plus checked `redundant_boolean_if_expression` style findings
  and `simplify_redundant_boolean_if_expression` templates with lint/plan/fix-write/verify
  smoke coverage, plus checked `redundant_boolean_if_statement` style findings
  and `simplify_redundant_boolean_if_statement` templates with lint/plan/fix-write/verify
  smoke coverage, plus checked `same_branch_if_expression` style findings and
  `simplify_same_branch_if_expression` templates with lint/plan/fix-write/verify
  smoke coverage, plus checked `same_branch_if_statement` style findings and
  `simplify_same_branch_if_statement` templates with lint/plan/fix-write/verify
  smoke coverage, plus checked `unreachable_statement` dead-code findings and
  `delete_unreachable_statement` templates with lint/plan/fix-write/verify
  smoke coverage, plus
  checked dead private task cleanup
  transactions for grouped unused/unreachable private task deletion, plus
  checked `unused_take` and `unused_declared_effect` remove templates and fix
  dry runs, plus manifest-staged temp files for write-mode CLI smokes,
  direct `sley graft --write --trace <path>` source mutation and receipt
  redirection, explicit `sley fix --write --trace <path>` receipt
  redirection, follow-up `sley trace --trace <path>` receipt inspection, and
  `sley seal --trace <path>` plus `sley zjx --trace <path>` over non-empty
  receipt chains with recomputable graph digests while
  `sley fix --dry-run --trace <path>` remains non-mutating, plus project
  `AddImport` writeback into an existing on-disk module file that was not yet
  loaded through the entry import graph through both direct graft JSON and
  `sley fix --write --kind add_import --module <module>` followed by strict
  project checks, plus scoped seeded host capability acceptance and
  scope-denial diagnostics for DatabaseRead, DatabaseWrite, DbRead/DbWrite
  aliases, Network, Shell, ModelCall, SecretRead, Deploy, and Spend under the
  runtime smoke surface,
  plus a standalone dogfood agent deploy pipeline example with strict
  check/doctor/query/lint/run/verify/deploy
  dry-run and contract-inspected local artifact coverage across SecretRead,
  Network, ModelCall, and Deploy.
- Stable JSON roots now include bounded AST node reports, query reports, lint
  reports, run reports, doctor reports, edit-plan reports, verify reports,
  deploy dry-run reports, deploy artifact manifests, deploy artifact check
  reports, project scaffold reports, `sley-ci` reports including corpus and
  examples gates, `sley-conformance` report/coverage roots, `sley-contract`
  utility reports with locked inventory/fixture-check/validate fixtures, and
  docgen reports, shadow reports, agent-bench reports, migrate reports, sandbox manifests,
  sandbox-runner reports, and the CLI smoke manifest in addition to AST
  program, diagnostics, graph, graph slice, trace report, trace receipt, trace
  seal, graft outcome, and ZJX envelope roots. The edit-plan schema now pins strict
  graft operation and transaction template envelopes,
  the query schema now exposes strict task/take/type/effect/call row
  definitions,
  graph-slice add/insert/move/delete/replace affordance operations reuse that
  strict graft operation schema,
  symbol graph, graph-slice, and graft outcome handoff roots have locked
  contract fixtures,
  graph-slice focus, task, and call summary payloads are schema-linked,
  the diagnostics schema exposes the shared diagnostic record used by
  graft/doctor/plan/verify report schemas, the trace report schema wraps trace
  receipt records, the graft outcome and trace receipt schemas pin accepted
  provenance records, the agent-bench schema pins deterministic repair-loop
  evidence, the migrate schema pins checked migration commands and schema drift
  evidence, and the ZJX envelope plus ZJX tool report roots now carry
  recomputable graph digest and inspection contracts with locked fixtures for
  graph, slice, trace receipt, and envelope handoff contents.

Product thesis:

Sley should make agents better programmers by replacing fragile raw text edits
with a tight compiler-mediated loop:

1. Read compact docs, examples, schemas, and manifests.
2. Inspect typed AST, symbol graph, query reports, lint reports, or graph
   slices.
3. Propose a strict structural graft or a narrow source change.
4. Run checker, formatter, query, lint, relevant runtime smokes, corpus gates,
   and packaged example gates.
5. Read stable diagnostics, repair hints, query summaries, and lint findings.
6. Iterate until Loom accepts the change and the conformance surface is updated.
7. Leave human-reviewable source, machine-readable contracts, trace receipts,
   and content-addressed seals when applicable.

Hard truth:

Codex and other agents do not have broad Sley pretraining. Sley must teach
itself through tooling: schemas, graph slices, query reports, lint reports,
repair hints, examples, conformance fixtures, CLI smoke manifests, dry-run
grafts, deterministic runtime seeds, and stable diagnostics. If a feature
cannot be learned or verified through that loop, it is not agent-native yet.

Non-negotiable constraints:

- Source text is the human review projection.
- The typed graph is the canonical agent work surface.
- Prefer structural grafts over raw text edits when the operation fits the
  graft surface.
- Plain graft preview must stay non-mutating unless `--write` is supplied.
- Unknown graft fields must continue to reject instead of being silently
  ignored.
- Authority failures must remain diagnostics, not recoverable userland errors.
- Recoverable host failures should use typed `Result<T, Error>` flows.
- Do not add effects merely to silence diagnostics; authority must be real.
- Do not invent binding kinds casually. The stable vocabulary is governed by
  `docs/SleyLanguageSpec.md`.
- Keep runtime host adapters deterministic in v0. Seeded database, secret,
  deploy, spend, network, shell, and model adapters must not call real external
  systems, and scoped capability mismatches must remain diagnostics.
- Do not publish, deploy, spend, call providers, or mutate external systems
  without explicit operator approval.
- Preserve unrelated local changes. This repo may already be dirty.
- Keep tests, schemas, contract snapshots, and CLI smoke manifests
  authoritative.

World-best criteria:

Sley is moving toward "best programming language for agents" only when it
improves measurable agent work, not when it merely accumulates syntax.

Judge work by:

- Can an agent inspect the relevant program slice without loading the whole
  codebase?
- Can an agent ask `sley query --json` for module, task, type, effect, and call
  facts before editing?
- Can an agent ask `sley lint --json` for warning-grade hygiene before and
  after edits?
- Can an agent propose a legal edit as a graft instead of patching raw text?
- Does Loom reject illegal edits with stable IDs, spans, node IDs, and repair
  hints?
- Does the formatter preserve a clean human-reviewable projection?
- Does `check --json` give enough information for the next repair attempt?
- Do query and lint JSON reports stay stable enough for future helper passes?
- Are effects explicit and enforced at check/runtime boundaries?
- Are runtime examples deterministic and reproducible?
- Are accepted and rejected examples locked into conformance tests?
- Are JSON schemas stable and versioned?
- Are CLI smoke expectations manifest-backed?
- Can future agents resume from the conformance-inventoried onboarding pack,
  schemas, manifests, examples, and tests without private chat context?

High-leverage work lanes:

1. Agent tool contract:
   - keep `parse`, `format`, `check`, `run`, `ast`, `graph`, `query`, `lint`,
     `trace`, `seal`, `zjx`, and `graft` stable;
   - make every JSON root schema-versioned;
   - keep `query` and `lint` suitable for tool-facing helper passes.

2. Query and lint consumption:
   - keep expanding `sley-shadow` and other deterministic helper tooling that
     consumes `sley.query.report.v0` and `sley.lint.report.v0` without becoming
     semantic authority;
   - broaden authority, style, migration, and reachability lints only when the
     output contract is clear;
   - preserve `--deny-warnings` as the CI gate for warning-grade lint output.

3. Diagnostics and repair:
   - improve parse errors, unknown names, type mismatches, return mismatches,
     call arity/type errors, private imports, ambiguous imports, effect
     authority errors, stale grafts, unsupported graft operations, and lint
     findings;
   - include repair hints that suggest valid structural edits without bypassing
     authority.

4. Graft-first editing:
   - expand graph-slice grafts where tests prove safety;
   - harden project writeback beyond existing modules;
   - keep dry-run paths explicit and trustworthy;
   - preserve trace receipts and seals for accepted writes.

5. Conformance and contracts:
   - grow accepted/rejected fixtures;
   - lock contract snapshots under `fixtures/contracts/`;
   - keep JSON schemas under `docs/schemas/`;
   - keep CLI smoke coverage manifest-backed under
     `fixtures/cli_smokes/manifest.json`;
   - update schemas, snapshots, manifests, and tests in the same change when a
     JSON output contract changes.

6. Runtime authority:
   - keep seeded v0 adapters deterministic;
   - keep real external calls out of v0 tests;
   - preserve the distinction between authority diagnostics and recoverable
     `Result` host failures;
   - test both authorized and unauthorized paths for authority changes.

7. Self-hosting ladder:
   - keep Rust Loom as the strict oracle;
   - write Sley helper/lint/query passes only after the graph/query/lint
     contracts support them;
   - run self-hosted passes in shadow mode before promotion;
   - preserve a recovery oracle.

Default first moves in any new session:

1. Run:

```bash
cargo fmt -- --check
cargo test
```

2. Inspect the relevant command behavior before editing:

```bash
cargo run -- check --json examples/hello.sley
cargo run -- graph --json examples/hello.sley
cargo run -- ast --json examples/hello.sley
cargo run -- query --json --kind tasks examples/project
cargo run -- lint --json examples/hello.sley
```

3. If changing grafts, inspect or create a dry-run graft first:

```bash
cargo run -- graft --json --dry-run <target> <graft.json>
```

4. If changing query or lint output:

```bash
cargo run -- query --json --kind all <target>
cargo run -- lint --json <target>
cargo run -- lint --json --rule unused-private-task <target>
cargo run -- lint --json --rule unreachable-private-task <target>
cargo run -- lint --json --rule unused-private-type <target>
cargo run -- lint --json --rule unused-private-effect <target>
cargo run -- lint --json --rule raw-host-adapter <target>
cargo run -- lint --json --rule missing-module-declaration <target>
cargo run -- lint --json --rule unchecked-result <target>
cargo run -- lint --json --rule unqualified-imported-call <target>
cargo run -- lint --json --rule unused-pure-binding <target>
cargo run -- lint --json --rule unused-pure-expression-statement <target>
cargo run -- lint --json --rule mutable-binding-never-set <target>
cargo run -- lint --json --rule self-assignment-statement <target>
cargo run -- lint --json --rule overwritten-set-statement <target>
cargo run -- lint --json --rule redundant-initial-set-statement <target>
cargo run -- lint --json --rule constant-if-expression <target>
cargo run -- lint --json --rule constant-if-statement <target>
cargo run -- lint --json --rule constant-false-if-statement <target>
cargo run -- lint --json --rule constant-false-while-statement <target>
cargo run -- lint --json --rule constant-comparison-expression <target>
cargo run -- lint --json --rule constant-arithmetic-expression <target>
cargo run -- lint --json --rule absorbing-arithmetic-expression <target>
cargo run -- lint --json --rule constant-text-concatenation-expression <target>
cargo run -- lint --json --rule constant-list-index-expression <target>
cargo run -- lint --json --rule constant-map-index-expression <target>
cargo run -- lint --json --rule constant-record-field-access-expression <target>
cargo run -- lint --json --rule constant-len-expression <target>
cargo run -- lint --json --rule constant-not-expression <target>
cargo run -- lint --json --rule empty-if-statement <target>
cargo run -- lint --json --rule empty-else-statement <target>
cargo run -- lint --json --rule empty-for-statement <target>
cargo run -- lint --json --rule empty-forge-statement <target>
cargo run -- lint --json --rule identity-binary-expression <target>
cargo run -- lint --json --rule redundant-boolean-comparison <target>
cargo run -- lint --json --rule absorbing-boolean-expression <target>
cargo run -- lint --json --rule idempotent-boolean-expression <target>
cargo run -- lint --json --rule self-comparison-expression <target>
cargo run -- lint --json --rule double-negation-expression <target>
cargo run -- lint --json --rule negated-comparison-expression <target>
cargo run -- lint --json --rule redundant-boolean-if-expression <target>
cargo run -- lint --json --rule redundant-boolean-if-statement <target>
cargo run -- lint --json --rule same-branch-if-expression <target>
cargo run -- lint --json --rule same-branch-if-statement <target>
cargo run -- lint --json --rule unreachable-statement <target>
cargo run -- query --json --kind types <target>
cargo run -- query --json --kind effects <target>
```

5. If changing runtime authority, test both the authorized and unauthorized
   path.

6. If changing JSON output, update schemas, locked contract fixtures, CLI smoke
   manifests, and tests in the same change.

Validation commands to prefer:

```bash
make v1
make public-release-check
cargo fmt -- --check
cargo test
cargo run -- check --json examples/hello.sley
cargo run -- run --json examples/hello.sley
cargo run -- graph --json examples/hello.sley
cargo run -- query --json --kind tasks examples/project
cargo run -- lint --json examples/hello.sley
cargo run -- seal --json examples/hello.sley
cargo run -- zjx --json examples/hello.sley
cargo run -- run --json --cap DatabaseRead --db-table users=examples/users.json examples/db_gate.sley
cargo run -- run --json --cap DatabaseRead --cap DatabaseWrite --db-table users=examples/users.json examples/db_write_gate.sley
cargo run -- run --json --cap Network --http-text https://example.test/profile Ada examples/network_gate.sley
cargo run -- run --json --cap Shell --shell-output date 2026-05-05 examples/shell_gate.sley
cargo run -- run --json --cap ModelCall --model-output name Ada examples/model_gate.sley
cargo run -- run --json --cap SecretRead --secret api_key redacted examples/secret_gate.sley
cargo run -- run --json --cap Deploy --deploy-result staging staged examples/deploy_gate.sley
cargo run -- run --json --cap SecretRead --secret api_key redacted --cap Network --http-text https://example.test/profile "profile ready" --cap ModelCall --model-output deploy-plan "plan approved" --cap Deploy --deploy-result staging staged examples/agent_deploy_pipeline.sley
cargo run -- run --json --cap SecretRead --secret api_key redacted --cap Network --http-text https://example.test/profile "profile ready" --cap ModelCall --model-output deploy-plan "plan approved" --cap Deploy --deploy-result staging staged examples/agent_project
cargo run -- run --json --cap Spend --spend-result ads-budget authorized examples/spend_gate.sley
```

Done when:

- The chosen change improves Sley's agent edit loop, not just its syntax.
- Tests pass, or failures are documented with exact command output summaries.
- JSON schema, contract, or manifest changes are updated deliberately.
- New diagnostics or lint findings include stable IDs and useful repair hints
  where practical.
- New query/lint behavior is covered by schemas, snapshots, CLI smokes, or
  direct tests.
- New graft behavior is covered by accepted and rejected tests.
- Runtime authority changes include positive and negative coverage.
- Documentation is updated only where it helps future agents use Sley
  correctly.
- The final summary states:
  - behavioral delta;
  - files changed;
  - validation run;
  - remaining risks;
  - next best task.
