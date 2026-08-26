# Sley AI Legibility Audit

Status: verified current-state audit
Date: 2026-08-24
Scope: Sley 1.1 public repository and the first downstream deterministic parity workload

## Decision

Sley already has most of the compiler-facing substrate an unfamiliar coding
model needs. The compact bootstrap, deterministic smoke evaluator,
default-deny corpus governance specification, append-only corpus event audit,
fail-closed SleyBench split auditor, production 120-case split, and governed
K1/K3 one-shot baseline are now implemented. The baseline tied at 96/120
strict case passes in both modes. K3 improved compiler-oracle measures but did
not improve strict success and cost about 2.4 times as much as K1. A later
controller-only K3 replay reached 99/120, a model-facing turn-boundary
correction reached 102/120 while reducing concatenated responses from 8 to 0,
and the targeted semantic audit found a general compact expression-syntax
documentation gap. Bootstrap 0.2 corrected that gap; one otherwise controlled
K3 replay reached 109/120 strict and 111/120 oracle passes. Its paired evidence
is directional rather than statistically conclusive, and four original
failures remain attributable to underspecified exact-literal task contracts
rather than model reasoning. A compact Bootstrap 0.3 candidate restored
compile performance from 98/100 to 100/100, but strict and oracle success each
fell by two. It was rejected after its single controlled replay; Bootstrap 0.2
remains active.

Confidence: high for the repository and measured one-model result, low for
general model fluency. This conclusion is based on executable local commands,
schema-validated fixtures and model evidence, independent review, repository
source, and the downstream parity report.

## Evidence Map

| Capability | Current evidence | Finding |
| --- | --- | --- |
| Versioned language authority | `self-hosted/src/loom/bootstrap.sley`; `docs/SleyLanguageSpec.md` | Present. The executable version is `sley 1.2.0`; the language spec documents the current source, type, effect, graph, and runtime model. |
| Compact machine onboarding | `llms.txt`; `docs/AgentQuickstart.md` | Partial. Both are useful, but neither is a version-coupled repository operating contract for an unfamiliar coding model. |
| Structured diagnostics | `sley check --json`; `docs/schemas/sley.diagnostics.report.v0.schema.json` | Present. Diagnostics and repair hints have a schema-backed root. |
| AST and graph exposure | `sley ast --json`; `sley graph --json`; `sley query --json` | Present. Full and bounded structural projections use stable schema IDs. |
| Planned and checked repair | `sley plan --json --graft-templates`; `sley fix --dry-run`; `sley graft --dry-run` | Present. The compiler exposes non-mutating repair previews and checked structural operations. |
| Agent tool boundary | `docs/SleyMcpBridge.md`; `bin/sley-mcp-bridge` | Present and read-only. Query, lint, plan, dry-run graft, and verify are exposed inside an explicit Git root with byte and time bounds. |
| Editor support | `sley-lsp`; `tree-sitter-sley/`; `editors/vscode-sley/` | Present as in-tree bootstraps. |
| Examples and conformance corpus | `examples/`; `fixtures/corpus/manifest.json`; `make v1` | Present but small for model evaluation: 23 accepted and 43 rejected language cases at the audited checkpoint. |
| Deterministic downstream workload | downstream numerology package and `sley-numerology-parity-0.md` | Present on the reconciled local Sley 1.1 integration line. Two fresh 10,000-case runs matched the frozen incumbent oracle with zero mismatches. The incumbent remains authoritative and no production path points to Sley. |
| AI competence benchmark | `sley-agent-bench`; `openclaw-one-shot-v0`; `openclaw-one-shot-v1`; `openclaw-one-shot-v2`; nine `sley.agent_bench.*.v0` schemas; `greyforge.sleybench.mode_summary.v0`; `sleybench-split`; `fixtures/sleybench/baseline-v0`; `docs/SleyBenchBaseline20260823.md`; `docs/SleyBenchK3ControllerAudit20260823.md`; `docs/SleyBenchK3ProtocolCorrection20260823.md`; `docs/SleyBenchK3SemanticAudit20260824.md`; `docs/SleyBenchK3Bootstrap03Audit20260824.md` | Production evidence is present. The final 96/24 split passed all 24 audit checks and both trusted-solution replays. K1 and K3 v0 each completed all 120 cases with 96 strict passes. K3 v1 passed 99/120 after correcting owned nested-path tool validation. K3 v2 passed 102/120 and reduced two-object model responses from 8 to 0. W3 now registers strict public replay and exact combined-summary shapes without copying held-out data or claiming a new run. The semantic audit classified seven Sley-specific syntax failures and four underspecified task contracts; Bootstrap 0.2 reached 109/120 strict. Candidate 0.3 restored 100/100 compile but fell to 107/120 strict and was rejected. |
| Training corpus contract | `docs/SleyCorpusSpec.md`; eight `sley.corpus.*.v0` schemas; `sley-corpus`; `fixtures/corpus_governance` | Strict contracts and tested governance tooling are present. The exact baseline evaluation capture is approved and retained outside public Git, but every benchmark trajectory remains ineligible for training, retrieval, export, or release. General corpus admission remains disabled. |

## Reproduced Commands

The following commands were inspected or executed from a clean worktree at the
audited commit:

```bash
bin/sley --version
bin/sley --help
bin/sley ast --json examples/hello.sley
bin/sley lint --json examples/empty_for_statement.sley
bin/sley-agent-bench run --json --manifest fixtures/sleybench/smoke-v0/case-manifest.json --run-manifest fixtures/sleybench/smoke-v0/run-manifest.json
bin/sley-contract inventory --json
bin/sley-corpus audit --manifest fixtures/corpus_governance/manifest.json --root fixtures/corpus_governance --json
scripts/test-sley-corpus.sh
scripts/test-sleybench-split.sh
bin/sleybench-split verify-public --split fixtures/sleybench/baseline-v0/split.json --json
scripts/check-self-hosted-code.sh
```

Repository evidence also includes the schema map in `docs/contracts.md`, the
agent loop in `docs/AgentQuickstart.md`, and the local v1 target inventory in
`Makefile`. The provider runner, production trusted-solution replay, and full
private audit remain in operator custody outside this foreign-language-free
public tree.

## Gap Analysis

### P0: lock the knowledge contract

1. Maintain root `SLEY_AI.md` as the shortest versioned repository bootstrap.
2. Govern it with `docs/SleyAISpec.md`.
3. Fail the local gate when its Sley version or required authority links drift.

### P0: establish honest evaluation

1. Keep the legacy no-manifest report labeled as contract bootstrap evidence.
2. Use the manifest-mode 24-case suite only to verify evaluator wiring.
3. Keep the governed one-shot adapters and production split pinned. Treat the
   K3 controller and protocol replays as narrow defect measurements, not
   independent model-family results. Require another configured model family
   before any broader fluency claim.

### P1: govern corpus capture

The strict contracts, corpus eligibility lane, chained governance-event audit,
production split, and exact baseline evaluation capture are implemented.
Benchmark trajectories remain evaluation-only. Any corpus admission, export,
or unrelated capture still requires a new exact approval and contamination
audit.

### P2: train only after measurement

Compare bootstrap-only, retrieval, and tool-assisted modes on the same held-out
SleyBench cases. Fine-tuning is eligible only if it produces a reproducible
held-out gain after operational cost is included.

## Downstream Findings

The first deterministic product boundary is stronger than the older planning
baseline implied:

- the incumbent TypeScript oracle and canonical result contract are frozen;
- Sley owns a bounded numerology candidate package behind a line-delimited,
  resource-bounded machine protocol;
- a sanitized 10,000-case corpus has produced exact parity in two runs;
- source, runtime, candidate-corpus, shard, and mismatch-report digests were
  recorded;
- no production configuration points at Sley, so rollback is currently the
  absence of cutover rather than an exercised production fallback.

The machine protocol and numerology intrinsics now coexist with the audited
public Sley 1.1 surface on one local reconciled branch. That branch passed the
full Sley v1 gate and fresh parity/determinism evidence. This is a local Git and
test checkpoint only; it does not imply merge, push, or production adoption.

This proves a credible reference workload. It does not prove production
authority, general Sley fluency, or model competence.

## Audit Boundary

This audit made authorized provider calls only for independent review, adapter
calibration, the K1/K3 baseline, the controlled K3 protocol replay, and the
single controlled bootstrap 0.2 semantic replay.
It made no public change, deployment, runtime mutation, model-training action,
or general model-fluency claim. The
downstream parity and two-run 20,000-case determinism workload remain local
evidence against the reconciled branch.
