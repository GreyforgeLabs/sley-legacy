# Sley Contract Map

Status: v0 contract map for the current release-candidate surface.
Last checked: 2026-05-07.

This file names the machine-readable JSON roots that external tools may
consume. Schemas live under `docs/schemas/`, representative instances live
under `fixtures/contracts/`, and the local release gate validates both.

## Validation Commands

```bash
cargo run --bin sley-contract -- inventory docs/schemas --json
cargo run --bin sley-contract -- inventory --json
cargo run --bin sley-contract -- check-fixtures fixtures/contracts --json
cargo run --bin sley-conformance -- report --json
make v1
```

`sley-contract` uses `docs/schemas` from the current working directory when it
exists and otherwise falls back to the bundled source schema directory.
Explicit schema paths remain supported for pinned validation.

Current counts:

- Schemas: `37`
- Contract fixtures: `105`
- Schema instances: `108`

## Core Compiler Roots

| Schema ID | Producer | Purpose |
|---|---|---|
| `sley.ast.program.v0` | `sley ast --json` | Full AST projection. |
| `sley.ast.node.v0` | `sley ast --json --node <id>` | Bounded AST node projection. |
| `sley.diagnostics.report.v0` | `sley check --json` and blocked commands | Stable diagnostics. |
| `sley.symbol_graph.v0` | `sley graph --json` | Full symbol graph. |
| `sley.symbol_graph.slice.v0` | `sley graph --json --slice <id>` | Focused graph slice and affordances. |
| `sley.query.report.v0` | `sley query --json` | Checked modules, tasks, types, effects, and calls. |
| `sley.lint.report.v0` | `sley lint --json` | Warning-grade lint findings and checked repairs. |
| `sley.edit_plan.report.v0` | `sley plan --json` | Ranked edit surfaces and graft templates. |
| `sley.graft.outcome.v0` | `sley graft --json` | Accepted or rejected checked graft provenance. |
| `sley.run.report.v0` | `sley run --json` | Deterministic runtime result. |
| `sley.verify.report.v0` | `sley verify --json` | Pre-deploy verification report. |
| `sley.doctor.report.v0` | `sley doctor --json` | Agent-facing readiness summary. |

## Trace, ZJX, And Deploy Roots

| Schema ID | Producer | Purpose |
|---|---|---|
| `sley.trace.receipt.v0` | write-mode commands | JSONL write provenance receipt. |
| `sley.trace.report.v0` | `sley trace --json` | Trace receipt summary. |
| `sley.trace.seal.v0` | `sley seal --json` | Content-addressed source, graph, and trace seal. |
| `sley.zjx.envelope.v0` | `sley zjx --json` | Preview ZJX envelope with graph digest. |
| `sley.zjx.tool.report.v0` | `sley-zjx` | Read-only envelope inspection, digest, extraction, and diff report. |
| `sley.deploy.report.v0` | `sley deploy --json --dry-run` | Local-only deploy package report. |
| `sley.deploy.artifacts.v0` | deploy artifact directory | Manifest for local deploy handoff files. |
| `sley.deploy.artifact_check.v0` | `sley-contract inspect-deploy-artifacts` | Revalidation report for deploy handoff directories. |

## Utility And Release Roots

| Schema ID | Producer | Purpose |
|---|---|---|
| `sley.ci.report.v0` | `sley-ci` | CI wrapper reports for check, lint, doctor, plan, run, verify, deploy, smoke, corpus, and examples. |
| `sley.conformance.report.v0` | `sley-conformance report --json` | Release-readiness summary, including editor-shim validation, `make v1` target inventory, and the hard public-release gate shape. |
| `sley.conformance.coverage.v0` | `sley-conformance coverage --json` | Required tag coverage result. |
| `sley.conformance.manifest.v0` | `fixtures/corpus/manifest.json` | Accepted/rejected corpus manifest. |
| `sley.cli_smoke.manifest.v0` | smoke manifests | CLI smoke manifest. |
| `sley.contract.inventory.v0` | `sley-contract inventory` | Schema inventory. |
| `sley.contract.fixture_check.v0` | `sley-contract check-fixtures` | Fixture validation report. |
| `sley.contract.validate.v0` | `sley-contract validate` | Single-report validation report. |
| `sley.lsp.fix_preview.v0` | `sley-lsp` | Non-mutating editor repair preview payload. |
| `sley.lsp.command_preview.v0` | `sley-lsp` | Non-mutating editor command handoff preview payload. |
| `sley.workbench.report.v0` | `sley-workbench` | Local inspection report and optional HTML panels. |
| `sley.docgen.report.v0` | `sley-docgen` | Generated reference summary and optional Markdown handoff. |
| `sley.agent_bench.report.v0` | `sley-agent-bench` | Deterministic agent repair-loop benchmark. |
| `sley.migrate.report.v0` | `sley-migrate` | Checked migration and schema-drift report. |
| `sley.sandbox.manifest.v0` | sandbox manifests | Deterministic seeded runtime replay input. |
| `sley.sandbox.report.v0` | `sley-sandbox-runner` | Deterministic sandbox replay evidence. |
| `sley.project.scaffold.v0` | `sley new --json` | Project scaffold report and next actions. |

## Stability Rules

- Every JSON root must carry a top-level `schema` string.
- Schema drift must update the matching schema file, representative fixture,
  inventory snapshot, tests, and release docs in the same change.
- Contract fixtures must validate through JSON Schema draft 2020-12.
- Runtime, deploy, sandbox, and benchmark contracts must remain deterministic
  and must not require provider calls, network access, secrets, spend, or live
  deployment.
