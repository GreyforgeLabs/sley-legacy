# Sley Developer Utilities Roadmap

Status: in-tree bootstrap implemented; v1 gate hardening map.
Date: 2026-05-06

## Purpose

Sley now has enough stable compiler surface to support external developer
tools. The useful move is not to build a large product shell first. The useful
move is to package the contracts, checks, editor loop, and graph/graft
inspection surfaces that make Sley safe for human and agent contributors.

This roadmap ranks utility ideas by direct usefulness to Sley developers,
dependency order, and how well each tool reinforces the existing compiler
contract.

Current evidence base:

- `sley check`, `query`, `lint`, `doctor`, `plan`, `fix`, `verify`, `trace`,
  `seal`, `zjx`, `graft`, and `new` already exist.
- Stable JSON roots and JSON schemas live under `docs/schemas/`.
- Contract snapshots live under `fixtures/contracts/`.
- CLI smoke coverage is manifest-backed under `fixtures/cli_smokes/`, including
  allowlisted sibling utility binary cases for `sley-ci`, `sley-conformance`,
  `sley-contract`, `sley-migrate`, `sley-docgen`, `sley-workbench`,
  `sley-sandbox-runner`, `sley-shadow`, `sley-agent-bench`, `sley-zjx`, and `sley-lsp`.
- `sley new --json` already exposes typed next actions for first-run projects.
- `sley plan --json --graft-templates` and `sley fix --dry-run` already expose
  non-mutating repair surfaces that editor and workbench tools can call.
- `make v1` now runs the focused LSP integration tests plus deterministic
  workbench, agent-bench, migration, docgen, sandbox-runner, shadow, and ZJX tool
  replays in addition to contracts, conformance, corpus, examples, smoke, and
  syntax gates.
- `make public-release-check` is the explicit public-cut metadata gate and is
  expected to fail until license and repository metadata are operator-approved.

## Ranking Rules

Rank higher when a utility:

1. improves the everyday edit, check, repair, or review loop;
2. consumes existing stable JSON contracts instead of inventing a new protocol;
3. reduces contributor mistakes before code reaches the compiler repo;
4. can be scaffolded without real provider calls, external spend, deployment,
   or public posting;
5. creates reusable infrastructure for later tools.

Do not prioritize tools that mainly create surface-area theater. A Sley
developer utility must either make edits safer, make contracts clearer, make
conformance stronger, or make onboarding faster.

## Must-Have Utilities

| Rank | Utility | Working Name | Primary User | Why It Is Must-Have | First Build Shape |
|---:|---|---|---|---|---|
| 1 | Contract kit | `sley-contract-kit` | tool authors, compiler contributors | Every external tool depends on stable JSON roots. Package schemas, validate reports, and generate typed bindings before rich tools drift. | Rust crate plus optional npm package that embeds `docs/schemas/`, validates CLI JSON, and snapshots schema versions. |
| 2 | CI and pre-commit pack | `sley-ci` | repo maintainers, app developers | Contributors need one obvious gate for `format`, `check`, `lint --deny-warnings`, `verify`, schema validation, and smoke manifests. | GitHub Action, reusable shell runner, and pre-commit hooks that call the current CLI. |
| 3 | Syntax grammar | `tree-sitter-sley` | editor users, docs, review tools | Source remains the human review projection. Highlighting and structural tokenization are basic developer ergonomics. | Tree-sitter grammar with corpus fixtures from `examples/` and `fixtures/corpus/accepted/`. |
| 4 | Language server | `sley-lsp` | app developers, agent operators | The fastest developer loop is editor diagnostics, format, graph lookup, and checked code actions backed by `sley plan` and `sley fix`. | Thin Rust or TypeScript LSP that shells to `sley --json`, returns diagnostics, formatting, symbols, hover, and code actions. |
| 5 | Graph and graft workbench | `sley-workbench` | compiler contributors, agent tool builders | Sley's canonical work surface is the typed graph. Developers need to inspect slices, preview grafts, and compare traces visually. | Local web app or TUI that opens a target, shows AST/graph/query/lint/plan JSON, previews grafts, and never writes unless explicitly confirmed. |
| 6 | Conformance dashboard | `sley-conformance` | compiler maintainers | Sley cannot grow safely if fixtures, schemas, and CLI smoke coverage drift. Maintainers need coverage visibility and contract diffs. | Static report generator over corpus manifest, smoke manifest, schemas, snapshots, and test counts. |

The first six are the minimum credible open source utility set. They map
directly to Sley's current thesis: source review, typed graph inspection,
checked repairs, stable machine contracts, and manifest-backed conformance.

## Priority Backlog

### P0: Built In-Tree, Split Later Only If Useful

1. `sley-contract-kit`
   - Inputs: `docs/schemas/*.schema.json`, CLI JSON stdout, contract fixtures,
     release manifests.
   - Outputs: validation result, schema version inventory, generated type
     bindings, contract drift report.
   - MVP commands:
     - `sley-contract validate --schema sley.verify.report.v0 report.json`
     - `sley-contract inventory`
     - `sley-contract check-fixtures fixtures/contracts`
     - `sley-contract validate --schema sley.conformance.manifest.v0 fixtures/corpus/manifest.json`
   - Scaffold:
     - current in-tree bootstrap binary: `src/bin/sley-contract.rs`;
     - `crates/sley-contract-kit/` if a reusable crate split is needed;
     - separate `sley-contract-kit` repo when published.
   - Current bootstrap: `inventory`, `check-fixtures`, `validate`, and
     `inspect-deploy-artifacts` emit versioned JSON Schema validation reports;
     contract validation commands default to the repo or bundled source schema
     directory while preserving explicit schema overrides;
     `make v1` validates contract fixtures plus corpus and smoke manifests;
     generated bindings and reusable package splits remain future contract-kit
     work.
   - Validation bootstrap is done when it validates every current contract
     fixture and release manifest against the matching schema and fails cleanly on a deliberately
     malformed report.

2. `sley-ci`
   - Inputs: project path, selected capabilities, optional seeded runtime
     values, optional smoke manifest, optional corpus manifest, optional
     examples root.
   - Outputs: pass/fail summary plus machine-readable report artifact.
   - MVP commands:
     - `sley-ci check .`
     - `sley-ci verify --deny-warnings .`
     - `sley-ci deploy --dry-run .`
     - `sley-ci smoke fixtures/cli_smokes/manifest.json`
     - `sley-ci corpus fixtures/corpus/manifest.json`
     - `sley-ci examples examples`
   - Scaffold:
     - `.github/actions/sley-v1/action.yml`;
     - `bin/sley-ci`;
     - `.pre-commit-config.yaml`;
     - default workflow under `.github/workflows/`.
   - Current bootstrap: in-tree `src/bin/sley-ci.rs` exposes `check`, `lint`,
     `doctor`, `plan`, `run`, `verify`, `deploy`, `smoke`, `corpus`, and `examples`
     wrappers with
     `schema: "sley.ci.report.v0"` output. `.github/actions/sley-v1/action.yml`,
     `.github/workflows/v1.yml`, and `.pre-commit-config.yaml` now run the
     repo-level `make v1` gate. The composite action installs stable Rust and
     Node, and the syntax target bootstraps Tree-sitter npm dependencies with
     `npm ci` when needed. A reusable standalone action package remains future
     work.
   - Bootstrap done when generated `sley new --template deploy` and
     `sley new --template agent` projects can run seeded verify gates, local
     deploy dry-run package reports, a CLI smoke manifest, and the
     accepted/rejected corpus manifest and packaged examples through `sley-ci`.
     The repo-level `Makefile` now exposes `make v1` to run the current local
     gate stack, including focused utility replays and Tree-sitter syntax
     parsing, in one command.

3. `tree-sitter-sley`
   - Inputs: `.sley` source fixtures.
   - Outputs: parse tree, syntax highlight queries, editor package metadata.
   - MVP coverage:
     - modules and imports;
     - task/type/effect declarations;
     - takes, gates, binding forms, calls, result flow, control flow;
     - comments once syntax supports them.
   - Scaffold:
     - `grammar.js`;
     - `queries/highlights.scm`;
     - `test/corpus/*.txt` generated from current examples.
   - Current bootstrap: in-tree `tree-sitter-sley/` provides `grammar.js`,
     highlight queries, generated parser artifacts, exact corpus tests, and a
     fixture parser smoke over current examples plus accepted compiler corpus
     sources through `npm test` or `make syntax`.
   - Done when all accepted source fixtures parse and rejected examples fail
     predictably enough for editor recovery.

4. `sley-lsp`
   - Inputs: file or project path, editor buffer text, current `sley` binary.
   - Outputs: diagnostics, symbols, formatting edits, hover text, code actions.
   - MVP features:
     - publish diagnostics from `sley check --json`;
     - format document through `sley format`;
     - document symbols from `sley query --json --kind all`;
     - code actions from `sley plan --json --graft-templates`;
     - non-mutating action preview through `sley fix --dry-run`.
   - Scaffold:
     - `sley-lsp-server`;
     - editor extension shim only after the server protocol is stable.
   - Current bootstrap: in-tree `src/bin/sley-lsp.rs` speaks stdio LSP
     framing, tracks full-document buffers, publishes compiler diagnostics plus
     lint warnings with `sley.toml` project context and unsaved open-buffer
     overlays, republishes diagnostics for all open project buffers after
     open-buffer and watched project-file changes, returns formatting edits,
     document symbols, folding ranges, declaration metadata, resolved
     project task-call hover, host-call capability hover, semantic tokens,
     workspace symbols, project completions, selection ranges, import/call
     definition jumps, project task
     signature help, project task parameter inlay hints, exact-range task
     references, document highlights, import document links, prepared
     cursor-aware project task rename edits, edit-plan code actions,
     non-mutating command-preview code lenses, and non-mutating preview commands
     for repair and command handoff.
     `editors/vscode-sley` contributes a private local VS Code shim with
     `.sley` language metadata, basic TextMate highlighting, and a
     `vscode-languageclient` bridge to this server, with package validation
     surfaced through `sley-conformance report`.
     `tests/sley_lsp.rs` drives the server over real JSON-RPC frames.
   - Done when an example project receives diagnostics and at least one checked
     lint repair code action without the server writing files directly.

5. `sley-workbench`
   - Inputs: target path and optional trace path.
   - Outputs: local inspection UI, graft preview, trace/seal/ZJX panels.
   - MVP views:
     - readiness summary from `doctor` and `verify`;
     - query tables for modules, tasks, types, effects, and calls;
     - graph slice viewer for one selected node;
     - lint findings and checked repair templates;
     - trace receipt and seal inspection.
   - Scaffold:
     - local-only web app backed by a small command runner;
     - no external network calls;
     - write mode disabled by default.
   - Current bootstrap: in-tree `src/bin/sley-workbench.rs` emits
     `schema: "sley.workbench.report.v0"` plus optional static HTML over
     doctor/query/lint/plan/graph/graph-slice panels. Edit-plan template rows
     carry non-mutating preview commands, explicit write commands, and post-fix
     check/lint/verify gates, and static HTML includes a local repair-focus
     selector for lint findings. It reads compiler data directly and writes
     only the requested HTML report path, never source files.
   - Done when a developer can open `examples/project`, select a lint finding,
     preview the checked fix, and inspect the post-fix gate commands.

6. `sley-conformance`
   - Inputs: corpus manifest, smoke manifest, schemas, snapshots, `cargo test`
     output.
   - Outputs: static HTML/Markdown report and JSON summary.
   - MVP checks:
     - coverage tag inventory;
     - missing schema-to-fixture mappings;
     - contract snapshot age and JSON Schema validation status;
     - smoke manifest coverage holes;
     - accepted/rejected corpus counts by feature.
   - Scaffold:
     - `sley-conformance report --json --html <path>`;
     - `sley-conformance coverage --json --require-tag <tag>`.
   - Current bootstrap: in-tree `src/bin/sley-conformance.rs` emits
     `sley.conformance.report.v0` and `sley.conformance.coverage.v0`, validates
     contract fixtures and release manifests through `sley-contract`, and
     inventories schema instances, corpus tags, smoke tags, packaged examples,
     the compact agent onboarding pack, editor-shim package validation, the
     `make v1` target set, declared integration-test count drift,
     local/public v1 readiness tracks, and public-release metadata blockers
     plus their next actions. The
     repo-level `make v1` gate runs the JSON report, while
     `make public-release-check` turns unresolved public blockers into the
     explicit nonzero release-cut gate.
   - Done when release-readiness gaps become visible without reading the whole
     test file.

### P1: Build After P0 Foundations

7. `sley-agent-bench`
   - Measures whether agent edit loops actually improve with graph/query/lint,
     plan, fix, and graft surfaces.
   - MVP: deterministic tasks where an agent must inspect JSON, propose or
     select a checked repair, run gates, and leave trace evidence.
   - Current bootstrap: in-tree `src/bin/sley-agent-bench.rs` runs a
     deterministic unused-private-task repair loop through check, query, lint,
     plan, checked `sley fix --write`, post-fix lint/verify, seal, and ZJX,
     emits `schema: "sley.agent_bench.report.v0"`, and is covered by an
     integration test plus a locked contract fixture.

8. `sley-template-pack`
   - Curated project templates beyond `hello`, `deploy`, and the first in-tree
     `agent` starter.
   - MVP: `library`, `cli`, `service-gate`, `data-pipeline`,
     `spend-gate`, `agent-task-pack`, and `agent-project` templates, all deterministic and
     verify-ready.
   - Current bootstrap: these templates are available through `sley new`, are
     listed in `sley.project.scaffold.v0`, and are covered by a CLI integration
     test that scaffolds, checks, lints, verifies, and runs each template with
     deterministic seeds where needed.

9. `sley-zjx-tools`
   - Inspection and verification utilities for `.zjx` envelopes, graph
     digests, trace receipts, and seal chains.
   - MVP: `validate`, `inspect`, `verify-digest`, `extract-graph`, and
     `diff-envelope`.
   - Current bootstrap: in-tree `src/bin/sley-zjx.rs` implements the MVP
     commands for current `sley.zjx.envelope.v0` preview JSON envelopes, emits
     `schema: "sley.zjx.tool.report.v0"`, and is covered by integration tests
     for validation, inspection, digest verification, graph extraction,
     envelope diffing, and tampered digest rejection.

10. `sley-migrate`
    - Checked migration helpers for language or contract changes.
    - MVP: raw host adapter migration, module declaration insertion, old naming
      cleanup, and schema-version drift reports.
    - Current bootstrap: in-tree `src/bin/sley-migrate.rs` reports checked
      source migrations from existing edit-plan templates for module
      declarations, raw host adapters, imported-call naming cleanup, and
      unchecked-result propagation, emits `schema: "sley.migrate.report.v0"`,
      and can optionally compare schema files against contract fixtures for
      drift without mutating source files.

11. `sley-docgen`
    - Generates human docs from `sley query` reports and source examples.
    - MVP: module, task, effect, and capability reference pages.
    - Current bootstrap: in-tree `src/bin/sley-docgen.rs` emits
      `schema: "sley.docgen.report.v0"` and optional Markdown from checked
      `sley.query.report.v0` module/task/type/effect summaries plus the
      canonical host capability contract table, including seeded `--cap`
      argument fragments. It supports checked module/export filters over files
      and project roots, blocks unknown module filters explicitly, and is
      covered by integration tests plus locked contract fixtures.

12. `sley-shadow`
    - Non-authoritative helper replay over checked query and lint reports.
    - MVP: link lint findings to query rows and surface seeded authority args
      for effectful tasks.
    - Current bootstrap: in-tree `src/bin/sley-shadow.rs` emits
      `schema: "sley.shadow.report.v0"` from checked `sley.query.report.v0`
      and `sley.lint.report.v0` data, supports module- and rule-scoped
      replay, blocks unknown module filters explicitly, is covered by
      integration tests, and has locked contract fixtures plus single-file,
      project-root, module-filtered, and rule-filtered CLI smoke coverage.

13. `sley-sandbox-runner`
    - A deterministic replay wrapper around seeded host adapters.
    - MVP: one manifest describing seeded files, tables, secrets, network text,
      shell output, model output, deploy results, and spend results.
    - Current bootstrap: in-tree `src/bin/sley-sandbox-runner.rs` runs
      manifest-backed deterministic verify replays over capability grants and
      inline file, table, secret, HTTP, shell, model, deploy, and spend seeds,
      emits `schema: "sley.sandbox.report.v0"` from
      `schema: "sley.sandbox.manifest.v0"`, and is covered by an integration
      test plus contract fixtures.

### P2: Useful Later, Not First

14. `sley-playground`
    - Browser playground backed by the compiler and workbench components.
    - Delay until LSP, contract kit, and workbench command contracts settle.

15. `sley-package-index`
    - Package registry metadata and discovery.
    - Delay until there are real external packages and a stable module/package
      story.

15. `sley-fuzzer`
    - Grammar, checker, graft, and formatter fuzzing.
    - Valuable, but it depends on a clearer stable grammar and expanded
      conformance oracles.

## Recommended Build Order

The P0/P1 bootstraps are now in-tree. Do not restart them as separate projects
unless a public packaging decision or external consumer proves the split is
worth the maintenance cost.

1. Keep `make v1` as the boring local truth: schemas, fixtures, corpus,
   examples, smoke manifests, utility replays, LSP, and syntax must keep
   passing together.
2. Use `make public-release-check` only for release cuts; it should remain
   blocked until the operator chooses license and repository metadata.
3. Extend `sley-lsp` from the current in-tree server toward project-wide
   workspace support, editor extension shims, and richer hover/details while
   keeping compiler modules as the semantic authority.
4. Extend `sley-workbench` from the current static local report toward richer
   local selectors for lint findings, graph targets, and graph slices without
   enabling writes by default.
5. Split `sley-contract-kit`, `sley-ci`, or Tree-sitter packages only after the
   public release posture is settled.

This order keeps every richer tool dependent on the compiler's existing
machine contracts, not on an unreviewed duplicate parser or private protocol.

## Scaffolding Contract For Each Utility

Every utility repo should start with:

- `README.md` with status, supported Sley compiler version, and exact commands;
- `LICENSE`;
- `CHANGELOG.md`;
- `docs/contracts.md` naming consumed Sley JSON roots;
- `examples/` using current Sley examples or generated scaffold projects;
- tests that run against the local `sley` binary or pinned fixture JSON;
- a `justfile`, `Makefile`, or equivalent with `fmt`, `test`, and `smoke`;
- no real network, provider, deploy, wallet, spend, or secret access in tests.

Versioning rule:

- utility versions may move quickly;
- consumed Sley JSON schema versions must be explicit;
- any accepted schema drift must update fixtures and tests in the same change.

Write-mode rule:

- inspection tools default to read-only;
- any file mutation must show the exact `sley fix --dry-run` or
  `sley graft --dry-run` evidence first;
- write paths must preserve trace receipts when the compiler supports them.

## What To Pre-Build Now

Best immediate pre-build package:

1. `sley-contract-kit`
   - lowest dependency risk;
   - directly useful to every later tool;
   - can be validated entirely against current schemas and fixtures.

Best immediate developer-facing package:

2. `sley-ci`
   - easiest to use in external repos;
   - gives Sley contributors one obvious quality gate;
   - mostly wraps commands that already exist and pass.

Best immediate adoption package:

3. `tree-sitter-sley`
   - gives visible editor value;
   - helps code review without changing compiler behavior;
   - can start narrow and grow from accepted fixtures.

Do not start with the playground or package index. They are attractive, but
they create more surface area than they prove. The current priority is trust:
contracts, gates, syntax review, editor diagnostics, and graph/graft
inspection.

Current release packaging docs now include `CHANGELOG.md` and
`docs/contracts.md`. `sley-conformance report --json` now surfaces
local/public v1 readiness tracks plus non-gating public-release blockers for
license and repository metadata. Repository license selection remains an
explicit operator decision before public release.

## Completion Gates

A utility is ready for first public open source release only when:

- it declares the Sley compiler version or JSON schema versions it consumes;
- it has deterministic local tests;
- it does not require private Greyforge paths;
- it does not require public posting, deployment, provider calls, spend, or
  secrets;
- it has examples that work against the current Sley repo;
- it documents failure modes and exact validation commands;
- it improves one of the measured Sley loops: edit, inspect, repair, verify,
  conform, or onboard.
