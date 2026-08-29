# Sley AI Bootstrap

Status: active
Sley version: 1.2.1
Bootstrap version: 0.2
Last verified: 2026-08-26

Use this file when you are an AI coding system encountering Sley without
reliable native familiarity. It is an operating map, not the language spec.

## What Sley Is

Sley is a self-hosted, agent-native structural programming language for
compiler-mediated, human-reviewed software change. Human-readable `.sley`
source is the review projection; compiler-exposed AST, graph, diagnostics,
plans, grafts, traces, and schemas are the machine work surface.

Compiler behavior and Sley-owned source outrank this bootstrap. If they
disagree, report the drift and follow the compiler evidence.

## Read Next

- Normative language model: `docs/SleyLanguageSpec.md`
- JSON producer/schema map: `docs/contracts.md`
- Machine schemas: `docs/schemas/`
- First agent workflow: `docs/AgentQuickstart.md`
- AI interface and authority rules: `docs/SleyAISpec.md`
- Evaluation protocol: `docs/SleyBenchSpec.md`
- Corpus admission and retention: `docs/SleyCorpusSpec.md`

Do not load all of those files by default. Start here, inspect the target with
JSON tools, then retrieve only the relevant section or schema.

## Smallest Valid Program

```sley
module app.hello

task main -> Text {
  return "hello sley"
}
```

## Core Expression Syntax

Sley uses symbolic Boolean operators: `&&` for and, `||` for or, and `!` for
not. The words `and`, `or`, `not`, and `then` are not executable Sley
operators or keywords.

Use the current single-line expression form when a conditional produces a
value:

```sley
return if ready { 1 } else { 0 }
```

Use statement form with explicit returns for multiline branches:

```sley
if ready {
  return 1
} else {
  return 0
}
```

The stage-1 checker may classify unsupported foreign expression syntax as a
`Raw` fallback without rejecting it. A clean `sley check` therefore does not
make foreign operator spellings executable; use the documented Sley forms.

Core declarations are `module`, `import`, `type`, `effect`, and `task`.
Parameters are declared with `take`. Calls use `call`. Bindings use `bind`,
`set`, `gate`, or `forge` according to their semantics. Do not infer C, Rust,
TypeScript, or Python syntax when the Sley spec or compiler can answer.

## Inspect Before Editing

From a repository with `bin/` on `PATH`:

```bash
sley check --json <target>
sley query --json --kind tasks <target>
sley graph --json --slice <node-id> <target>
sley ast --json --node <node-id> <target>
sley lint --json <target>
sley plan --json --graft-templates <target>
```

Use the top-level `schema` field to select a parser. Prefer a bounded query,
graph slice, or AST node over loading a full repository or full AST.

## Important JSON Roots

| Purpose | Schema |
| --- | --- |
| Full/bounded AST | `sley.ast.program.v0`, `sley.ast.node.v0` |
| Compiler diagnostics | `sley.diagnostics.report.v0` |
| Full/bounded graph | `sley.symbol_graph.v0`, `sley.symbol_graph.slice.v0` |
| Declarations and calls | `sley.query.report.v0` |
| Lint findings | `sley.lint.report.v0` |
| Planned edits | `sley.edit_plan.report.v0` |
| Checked graft outcome | `sley.graft.outcome.v0` |
| Verification | `sley.verify.report.v0` |
| Write provenance | `sley.trace.receipt.v0`, `sley.trace.seal.v0` |
| Corpus governance | `sley.corpus.record.v0`, `sley.corpus.manifest.v0`, `sley.corpus.audit_event.v0`, `sley.corpus.mutation_audit_report.v0` |
| Benchmark split | `sley.agent_bench.split_manifest.v0`, `sley.agent_bench.split_report.v0` |

Validate reports with the matching schema under `docs/schemas/`. Unknown major
schema identities fail closed.

## Repair Loop

1. Inspect Git state and preserve unrelated changes.
2. Run `check` and `lint` with JSON output.
3. Use the diagnostic ID, node ID, repair hints, and graph slice.
4. Ask `plan` for checked graft templates.
5. Preview with `sley fix --dry-run` or `sley graft --dry-run`.
6. Inspect the candidate and provenance.
7. Use `--write` only when the task authorizes mutation.
8. Run check, lint, project tests, and `sley verify --json`.
9. Retain trace/seal evidence when the workflow requires it.

Refresh stale node IDs or graft preconditions. Do not retry guessed edits
against stale structure.

## Authority Boundary

- `query`, `ast`, `graph`, `check`, `lint`, `plan`, dry-run graft/fix, and the
  MCP bridge are inspection or preview surfaces.
- `fix --write` and `graft --write` mutate source and require task authority.
- Seeded runtime capabilities are deterministic fixtures. They do not grant
  real network, provider, secret, spend, deploy, or host authority.
- `sley-mcp-bridge` is intentionally read-only and bounded to one explicit Git
  root.

## Common Wrong Assumptions

- Sley is not Rust, TypeScript, Python, or a prompt DSL.
- Source text is not the only program representation; structural IDs and graph
  contracts are first-class work surfaces.
- A clean parse does not prove checking, effects, authority, or tests.
- A compiler repair hint is a candidate, not permission to write.
- `sley-sandbox-runner` is deterministic seeded replay, not OS isolation.
- `sley-agent-bench` without manifests is the legacy deterministic contract
  bootstrap. Manifest mode executes trusted fixture candidates and independent
  compiler oracles, but still makes no model call and proves no model fluency.
- Existing examples are training/onboarding evidence, not a held-out benchmark.
- `sley-corpus` eligibility, manifest-audit, deletion-planning, and `audit-log`
  commands are read-only. `append-event` writes only a confirmation-gated,
  digest-chained governance event. It does not perform the corpus action, and a
  passed report is not authority to retain, train on, export, or delete data.
- `sleybench-split verify-public` proves only public commitments. Only a passed
  full audit may set `private_material_verified=true`, and private evidence must
  remain outside public Git.
- Do not invent standard-library functions, effects, schema IDs, or graft kinds;
  query the checked surfaces.

## Minimal Verification

```bash
export PATH="$(pwd)/bin:$PATH"
bin/sley --version
sley check --json examples/hello.sley
sley ast --json examples/hello.sley
sley-contract inventory --json
scripts/check-ai-foundation.sh
make v1
```

`make v1` is the complete local repository gate. A smaller successful command
does not prove the whole repository or AI foundation is ready.

## Model And Training Claims

Use the knowledge modes and evaluator rules in `docs/SleyAISpec.md` and
`docs/SleyBenchSpec.md`, and apply the default-deny admission rules in
`docs/SleyCorpusSpec.md`. Keep unaided, bootstrap, retrieval, read-only tool,
and checked-write results separate. Do not train, fine-tune, retain model
trajectories for reuse, or claim general Sley fluency from compiler fixtures,
the legacy report, or deterministic-fixture smoke results.
