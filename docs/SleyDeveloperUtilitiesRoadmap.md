# Sley Developer Utilities Roadmap

Status: planning scaffold for open source utility work.
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
- CLI smoke coverage is manifest-backed under `fixtures/cli_smokes/`.
- `sley new --json` already exposes typed next actions for first-run projects.
- `sley plan --json --graft-templates` and `sley fix --dry-run` already expose
  non-mutating repair surfaces that editor and workbench tools can call.

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

### P0: Build Or Scaffold First

1. `sley-contract-kit`
   - Inputs: `docs/schemas/*.schema.json`, CLI JSON stdout, contract fixtures.
   - Outputs: validation result, schema version inventory, generated type
     bindings, contract drift report.
   - MVP commands:
     - `sley-contract validate --schema sley.verify.report.v0 report.json`
     - `sley-contract inventory docs/schemas`
     - `sley-contract check-fixtures fixtures/contracts`
   - Scaffold:
     - current in-tree bootstrap binary: `src/bin/sley-contract.rs`;
     - `crates/sley-contract-kit/` if a reusable crate split is needed;
     - separate `sley-contract-kit` repo when published.
   - Current bootstrap: `inventory`, `check-fixtures`, and `validate` emit
     versioned JSON Schema validation reports; generated bindings and contract
     drift reports remain future contract-kit work.
   - Validation bootstrap is done when it validates every current contract
     fixture against the matching schema and fails cleanly on a deliberately
     malformed report.

2. `sley-ci`
   - Inputs: project path, selected capabilities, optional seeded runtime
     values, optional smoke manifest.
   - Outputs: pass/fail summary plus machine-readable report artifact.
   - MVP commands:
     - `sley-ci check .`
     - `sley-ci verify --deny-warnings .`
     - `sley-ci smoke fixtures/cli_smokes/manifest.json`
   - Scaffold:
     - `action.yml`;
     - `bin/sley-ci`;
     - `.pre-commit-hooks.yaml`;
     - sample workflow under `examples/github-actions/`.
   - Current bootstrap: in-tree `src/bin/sley-ci.rs` exposes `check`, `verify`,
     and `smoke` wrappers with `schema: "sley.ci.report.v0"` output; GitHub
     Action, pre-commit hook, and sample workflow packaging remain future work.
   - Bootstrap done when generated `sley new --template deploy` and
     `sley new --template agent` projects can run seeded verify gates and a CLI
     smoke manifest through `sley-ci`.

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
     - `sley-conformance report --json --html`;
     - `sley-conformance coverage --require-tag <tag>`.
   - Done when release-readiness gaps become visible without reading the whole
     test file.

### P1: Build After P0 Foundations

7. `sley-agent-bench`
   - Measures whether agent edit loops actually improve with graph/query/lint,
     plan, fix, and graft surfaces.
   - MVP: deterministic tasks where an agent must inspect JSON, propose or
     select a checked repair, run gates, and leave trace evidence.

8. `sley-template-pack`
   - Curated project templates beyond `hello`, `deploy`, and the first in-tree
     `agent` starter.
   - MVP: `library`, `cli`, `service-gate`, `data-pipeline`, and
     `agent-task-pack` templates, all deterministic and verify-ready.

9. `sley-zjx-tools`
   - Inspection and verification utilities for `.zjx` envelopes, graph
     digests, trace receipts, and seal chains.
   - MVP: `inspect`, `verify-digest`, `extract-graph`, and `diff-envelope`.

10. `sley-migrate`
    - Checked migration helpers for language or contract changes.
    - MVP: raw host adapter migration, module declaration insertion, old naming
      cleanup, and schema-version drift reports.

11. `sley-docgen`
    - Generates human docs from `sley query` reports and source examples.
    - MVP: module, task, effect, and capability reference pages.

12. `sley-sandbox-runner`
    - A deterministic replay wrapper around seeded host adapters.
    - MVP: one manifest describing seeded files, tables, secrets, network text,
      shell output, model output, deploy results, and spend results.

### P2: Useful Later, Not First

13. `sley-playground`
    - Browser playground backed by the compiler and workbench components.
    - Delay until LSP, contract kit, and workbench command contracts settle.

14. `sley-package-index`
    - Package registry metadata and discovery.
    - Delay until there are real external packages and a stable module/package
      story.

15. `sley-fuzzer`
    - Grammar, checker, graft, and formatter fuzzing.
    - Valuable, but it depends on a clearer stable grammar and expanded
      conformance oracles.

## Recommended Build Order

1. Scaffold `sley-contract-kit` in-tree as a small Rust crate or `xtask` style
   binary, then split later if it proves useful.
2. Build `sley-ci` as thin wrappers around existing CLI commands. Keep the
   first version boring and transparent.
3. Start `tree-sitter-sley` once the accepted fixture set is enough to define
   the review projection.
4. Build `sley-lsp` as a thin CLI-backed server. Avoid duplicating compiler
   logic in the LSP.
5. Build `sley-workbench` using only the same JSON roots the LSP and agents
   consume.
6. Add `sley-conformance` once schema and smoke gaps become annoying enough to
   justify a dashboard.

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
