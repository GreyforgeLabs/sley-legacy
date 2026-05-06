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
- Expanded `sley new` templates for library, CLI, service-gate, data-pipeline,
  deploy, agent, agent-task-pack, and multi-module agent-project starters.
- `sley-workbench`, `sley-lsp`, `sley-conformance`, `sley-contract`, and
  `sley-ci` bootstraps for the local v1 release gate.
- `docs/contracts.md`, a contract map for the current schema-backed JSON roots.
- Non-gating public-release packaging blockers in
  `sley-conformance report --json`, covering license and repository metadata
  decisions separately from executable conformance.
- `examples/agent_project`, a packaged multi-module agent deployment project
  covering imported task authority, seeded execution, strict linting, seeded
  verification, and dry-run deploy packaging.
- CLI smoke coverage for the packaged agent project across check, lint, call
  query, doctor, seeded run, seeded verify, and deploy artifact packaging.
- CLI smoke coverage for scoped host authority crossing imported agent-project
  task boundaries, including a deterministic imported-module scope-denial case.
- Integration coverage for the `agent-project` scaffold when the requested
  entry module is already named `pipeline`.
- Accepted and rejected synthetic corpus fixtures for split-task agent
  authority, locking transitive effect propagation.
- Expanded `fixtures/ci_smoke_probe` from a parse-only probe into a
  deterministic `sley-ci smoke` contract covering parse, query, graft dry-run,
  and seeded multi-capability agent runtime authority.

### Changed

- `make v1` now includes checks for LSP, workbench, agent bench, migrate,
  docgen, sandbox runner, ZJX tools, contract fixtures, conformance, corpus,
  examples, CLI smokes, and Tree-sitter syntax parsing.
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
- Ready deploy dry-run reports with artifact directories now include an
  `inspect_deploy_artifacts` next action for validating handoff bundles before
  operator approval.
- `sley lint --rule empty-else-statement` now flags no-op empty `else`
  branches and `sley plan --graft-templates` emits a checked
  `remove_empty_else_statement` repair.
- Contract inventory now tracks 36 schemas, 99 contract fixtures, and 102 schema
  instances through the conformance report.
- The Rust package metadata now declares its supported Rust floor, description,
  README, keywords, categories, and `publish = false` until publication
  decisions are explicit.

### Notes

- Live provider calls, live deployment, external spend, real secret reads, and
  compressed binary ZJX archive writing remain outside the v0 executable slice.
- License selection is intentionally not asserted here; it requires an explicit
  operator decision before public release.
