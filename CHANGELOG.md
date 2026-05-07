# Changelog

All notable Sley changes for the current release-candidate line are tracked
here. Dates use UTC-independent calendar dates from the local repo history.

## Unreleased

### Added

- `sley-docgen reference`, a checked JSON and optional Markdown reference
  generator over query-derived module, task, type, effect, and capability docs.
- `sley-sandbox-runner run`, a manifest-backed deterministic replay utility
  for seeded runtime capability and host adapter checks.
- `sley-migrate report`, a checked source migration and schema drift report
  utility.
- `sley-agent-bench run`, a deterministic agent repair-loop benchmark over
  query, lint, plan, fix, verify, seal, and ZJX evidence.
- `sley-zjx`, a read-only inspection utility for preview ZJX envelopes.
- `sley-shadow report`, a non-authoritative helper replay over checked query
  and lint reports with lint-link evidence, module/rule filters, and seeded
  authority args.
- Expanded `sley new` templates for library, CLI, service-gate, data-pipeline,
  deploy, agent, agent-task-pack, and multi-module agent-project starters.
- `sley-workbench`, `sley-lsp`, `sley-conformance`, `sley-contract`, and
  `sley-ci` bootstraps for the local v1 release gate.
- `editors/vscode-sley`, a private local VS Code shim for `.sley` language
  metadata, basic highlighting, and `sley-lsp` startup.
- `docs/contracts.md`, a contract map for the current schema-backed JSON roots.
- Non-gating public-release packaging blockers in
  `sley-conformance report --json`, covering license and repository metadata
  decisions separately from executable conformance.
- `examples/agent_project`, a packaged multi-module agent deployment project
  covering imported task authority, seeded execution, strict linting, seeded
  verification, and dry-run deploy packaging.
- `docs/AgentQuickstart.md`, a concise local path from `sley new` through
  JSON inspection, seeded run, warning-denying verify, dry-run deploy
  artifacts, and artifact contract inspection.
- CLI smoke coverage for the packaged agent project across check, lint, call
  query, doctor, seeded run, seeded verify, and deploy artifact packaging.
- Required CLI smoke coverage for the generated multi-module agent quickstart
  path from scaffold through inspection, strict lint, seeded run, verify,
  dry-run deploy artifacts, and artifact contract inspection.
- CLI smoke coverage for scoped host authority crossing imported agent-project
  task boundaries, including a deterministic imported-module scope-denial case.
- Integration coverage for the `agent-project` scaffold when the requested
  entry module is already named `pipeline`.
- Accepted and rejected synthetic corpus fixtures for split-task agent
  authority, locking transitive effect propagation.
- Expanded `fixtures/ci_smoke_probe` from a parse-only probe into a
  deterministic `sley-ci smoke` contract covering parse, query, graft dry-run,
  and seeded multi-capability agent runtime authority.
- Accepted and rejected synthetic corpus fixtures for module namespace coverage,
  including exported declarations and duplicate type/effect/task diagnostics.

### Changed

- `make v1` now runs deterministic workbench, agent-bench, migration, docgen,
  sandbox-runner, shadow, ZJX tool replays, and VS Code editor-shim validation plus
  the focused LSP integration tests, contract fixtures, conformance, corpus,
  examples, CLI smokes, and Tree-sitter syntax parsing.
- `sley-conformance report` now inventories `editors/vscode-sley` package
  metadata and runs the deterministic editor-shim validator as part of the
  release-readiness report.
- `sley-conformance report` now inventories the `make v1` target set and fails
  if required local release-gate targets disappear or are undefined.
- `sley-conformance report` now requires smoke tags for seeded runtime
  authority, scoped host capabilities, and scope-denial diagnostics.
- `sley-conformance report` now inventories the compact agent onboarding pack
  so missing bootstrap files are visible in release-readiness evidence.
- `sley-conformance report` now emits machine-readable public-release
  `next_actions` for operator-controlled license and repository metadata
  blockers.
- `sley-conformance report` now emits local/public v1 readiness tracks with
  gated-check completion percentages and exact gate commands.
- `sley-workbench` static HTML now includes a graph target selector that
  highlights focused graph rows and emits the exact `--slice` command for
  rerendering a focused graph slice.
- `sley-lsp` hover now shows required capabilities and seeded `--cap` hints
  when the cursor is on a known host call.
- `sley-docgen` capability rows now include seeded `--cap` argument fragments
  in JSON and generated Markdown.
- `sley doctor` ready reports now include a `verify_gate` next action before
  entrypoint runs, including seeded `--cap` args for effectful entrypoints.
- `sley-lsp` diagnostics now load `sley.toml` project context with unsaved
  open-buffer overlays, so valid imported calls resolve in editor buffers and
  missing imported tasks are diagnosed without requiring a save.
- `sley-lsp` now republishes diagnostics for all open project buffers after an
  open-buffer change, so dependent files react to unsaved imported-module edits.
- `sley-lsp` hover now resolves project task call sites to their target task
  signature, visibility, effect list, and node id.
- `sley-lsp` now handles watched project-file change notifications by
  refreshing diagnostics for all open buffers against the current project graph.
- `sley-lsp` now exposes full-document semantic tokens for namespaces, task
  functions, parameters, variables, keywords, literals, comments, and operators.
- `sley-lsp` now exposes non-mutating command-preview code lenses for
  `doctor`, `verify --deny-warnings`, `deploy --dry-run`, and task graph
  slices.
- `sley.lsp.command_preview.v0` now has a JSON Schema and representative
  contract fixture for editor command handoff payloads.
- `sley-lsp` now exposes definition jumps for imported modules and resolved
  task calls across project files.
- `sley-lsp` now exposes project-aware document links from import module names
  to their resolved project files.
- `sley-lsp` now returns deterministic completions for Sley keywords, host
  calls, project modules, local tasks, and visible imported tasks.
- `sley-lsp` now exposes project-aware `workspace/symbol` results for modules,
  tasks, types, and effects.
- `sley-lsp` now exposes folding ranges for braced tasks, record type
  declarations, and statement blocks.
- `sley-lsp` now exposes selection ranges for import modules, task names, and
  task call expressions.
- `sley-lsp` now exposes project-aware task parameter inlay hints for resolved
  local and imported task calls.
- `sley-lsp` now exposes exact-range project-aware task references and
  same-document task highlights for declarations and resolved call sites.
- `sley-lsp` now returns project-aware task signature help for local and
  imported task calls, including active parameter tracking in call arguments.
- `sley-lsp` now prepares cursor-aware project task rename ranges and returns
  workspace edits for declarations and resolved call sites.
- `sley-lsp` declaration hover now includes module, return type, takes,
  effects, export status, and node IDs for editor-side inspection.
- `sley-workbench` static HTML now renders focused graph-slice summaries,
  call edges, and graft affordance tables when `--slice <node-id>` is used.
- `sley-workbench` edit-plan template rows now carry explicit non-mutating
  preview commands, write commands, and post-fix check/lint/verify gate
  commands for repair handoff.
- `sley-workbench` static HTML now includes a local repair-focus selector that
  highlights the selected lint finding and filters matching repair templates.
- Added `make public-release-check` as the explicit failing gate for public
  release cuts until license and repository metadata blockers are resolved.
- Utility integration tests now validate live docgen, migrate, agent-bench,
  sandbox-runner, workbench, and ZJX JSON reports against their declared
  schemas. `sley.workbench.report.v0` now accepts the current nested
  `sley.lint.report.v0` `findings` status.
- `sley.docgen.report.v0` now schema-links generated task, type, effect, and
  diagnostic rows to the strict query and diagnostic contract definitions.
- `sley.migrate.report.v0` now schema-links checked migration operations to
  the strict edit-plan graft operation contract.
- `sley.workbench.report.v0` now schema-links embedded doctor actions,
  query rows, lint findings, edit-plan surfaces/actions, graph modules, and
  graph slices to their source contracts.
- `sley.lsp.fix_preview.v0` now schema-links editor preview operations and
  transactions to the strict edit-plan graft contracts, with live LSP preview
  validation in the integration test.
- `sley.verify.report.v0` now schema-links embedded runtime values to the
  strict `sley.run.report.v0` value contract.
- Deploy artifact manifests and artifact-check reports now pin report, seal,
  and package file roles to their expected schema ids.
- The local syntax gate bootstraps Tree-sitter npm dependencies with `npm ci`
  when needed, and the GitHub composite action installs stable Node before
  running `make v1`.
- Plain-text `sley-conformance report` output now lists public-release blockers
  directly instead of only reporting the blocker count.
- `sley-conformance report --require-public-release-ready` now turns
  unresolved public-release blockers into a nonzero release-cut gate while the
  ordinary v1 executable gate remains advisory on metadata decisions.
- `sley-ci smoke --repo-root <path>` now resolves the repo root before
  expanding `{repo}`, so smoke cases that run from the temp cwd work with
  relative repo-root arguments, and `sley-ci smoke` now accepts a smoke
  directory containing `manifest.json`.
- `sley-ci corpus` now accepts either `fixtures/corpus` or the explicit
  `fixtures/corpus/manifest.json` path.
- `make smoke` now runs both the broad CLI smoke suite and the lightweight
  `sley-ci smoke` wrapper probe.
- `sley-conformance report` and `sley-conformance coverage` now accept
  manifest directories for `--corpus-manifest` and `--smoke-manifest`.
- Ready deploy dry-run reports with artifact directories now include an
  `inspect_deploy_artifacts` next action for validating handoff bundles before
  operator approval.
- `sley-contract` validation commands now default to repo-local or bundled
  source schemas while preserving explicit schema overrides for pinned
  validation.
- `sley-ci smoke` manifests can now select allowlisted sibling Sley utility
  binaries, and the broad smoke suite covers `sley-contract` inventory,
  validate, fixture-check, deploy-artifact inspection flows, and `sley-migrate`
  raw-host and unchecked-result reports, plus `sley-ci`, `sley-conformance`,
  `sley-docgen`, `sley-workbench`, `sley-sandbox-runner`, `sley-shadow`,
  `sley-agent-bench`, `sley-zjx` utility reports, and `sley-lsp` help/startup.
- `sley lint --rule empty-else-statement` now flags no-op empty `else`
  branches and `sley plan --graft-templates` emits a checked
  `remove_empty_else_statement` repair.
- `sley-docgen reference --module <module>` now titles filtered project-root
  references by the selected module and is pinned by contract plus smoke
  coverage.
- Contract inventory now tracks 38 schemas, 110 contract fixtures, and 113 schema
  instances through the conformance report.
- The Rust package metadata now declares its supported Rust floor, description,
  README, keywords, categories, and `publish = false` until publication
  decisions are explicit.

### Notes

- Live provider calls, live deployment, external spend, real secret reads, and
  compressed binary ZJX archive writing remain outside the v0 executable slice.
- License selection is intentionally not asserted here; it requires an explicit
  operator decision before public release.
