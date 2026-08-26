# Sley AI Interface Specification

Status: active foundation specification
Version: 0.1
Sley compatibility: 1.1
Owner: Sley maintainers
Review trigger: Sley language version, compiler contract, or benchmark protocol change

## 1. Purpose

This specification defines how unfamiliar AI coding systems learn, inspect,
edit, repair, and verify Sley without relying on native model familiarity.

The design objective is compiler-grounded competence:

```text
small versioned bootstrap
  -> bounded structural retrieval
  -> schema-backed compiler feedback
  -> previewed structural change
  -> deterministic verification
  -> evidence-bearing handoff
```

Prompt fluency is not authority. A model may propose a change; the Sley
compiler, project tests, and human owner decide whether that change is accepted.

## 2. Normative Language

The words **MUST**, **MUST NOT**, **SHOULD**, **SHOULD NOT**, and **MAY** are
normative requirements in this document.

## 3. Authority And Document Roles

Sley AI clients MUST use the following precedence:

1. `self-hosted/src/loom/*.sley` and executable compiler behavior;
2. `docs/SleyLanguageSpec.md` for the normative language description;
3. JSON schemas under `docs/schemas/` for machine contract shape;
4. `docs/contracts.md` for the producer-to-schema map;
5. root `SLEY_AI.md` for compact operating guidance;
6. `docs/AgentQuickstart.md` and checked examples for workflows;
7. generated summaries, prompts, transcripts, and model memory.

When prose and executable behavior disagree, the client MUST report the drift.
It MUST NOT invent connective semantics or silently treat a prompt as language
authority.

`llms.txt` is a public discovery summary. `SLEY_AI.md` is the repo-local,
version-coupled operating bootstrap. Neither replaces the language spec or
compiler contracts.

## 4. Knowledge Modes

Every AI evaluation and material AI-authored change MUST record one mode:

| Mode | Context and tools |
| --- | --- |
| K0: unaided | Task only; no Sley-specific bootstrap, retrieval, or tools. Used only to measure native familiarity. |
| K1: bootstrap | Task plus the exact pinned `SLEY_AI.md`. |
| K2: retrieval | K1 plus allowlisted retrieval from the language spec, contract map, schemas, and checked examples. |
| K3: read-only tools | K2 plus compiler inspection, diagnostics, query, plan, dry-run graft, and verify. |
| K4: checked write | K3 plus explicit write-mode fix or graft inside an isolated task worktree, followed by required gates. |

Results from different knowledge modes MUST NOT be pooled into one fluency
number. K0 measures model familiarity; K1 measures bootstrap quality; K2
measures documentation retrieval; K3 and K4 measure the compiler-assisted
engineering system.

## 5. `SLEY_AI.md` Contract

The repository-root bootstrap MUST:

- name its status, audited Sley version, and last verification date;
- stay at or below 240 lines;
- link to the language spec, contract map, schemas, quickstart, benchmark spec,
  and AI interface spec;
- define the smallest valid source example;
- list the inspect-before-edit workflow;
- name the JSON roots most important to agents;
- describe the diagnostic and repair loop;
- list common false assumptions and prohibited shortcuts;
- distinguish read-only inspection, dry-run preview, and write authority;
- include exact local validation commands;
- contain no private paths, credentials, user data, provider secrets, or
  environment-specific runtime claims.

The bootstrap SHOULD fit comfortably in one model context together with a
small task. Detail belongs behind links, not in duplicated prose.

## 6. Machine-Readable Compiler Interface

AI clients SHOULD prefer JSON commands and consume the top-level `schema`
field before any other data. Required foundation roots include:

- `sley.ast.program.v0` and `sley.ast.node.v0`;
- `sley.diagnostics.report.v0`;
- `sley.symbol_graph.v0` and `sley.symbol_graph.slice.v0`;
- `sley.query.report.v0`;
- `sley.lint.report.v0`;
- `sley.edit_plan.report.v0`;
- `sley.graft.outcome.v0`;
- `sley.verify.report.v0`;
- `sley.trace.receipt.v0` and `sley.trace.seal.v0`;
- the `sley.transaction.inspect.v0` and `sley.change.*.v0` local governed
  transaction roots used by the CLI/MCP path.

A client MUST fail closed on an unknown major schema identity or on a report
that fails the matching JSON Schema. Contract drift MUST update its schema,
fixture, documentation, and consumer tests together.

The MCP bridge MAY be used for K3 and named governed local transaction calls.
It MUST remain bounded to an explicit Git repository root. Any mutation MUST
delegate to the matching confirmed `sley change` command with operator-issued
authority; the bridge MUST NOT gain arbitrary write, shell, network, provider,
deploy, authority-issuance, or generic command capability for client
convenience.

## 7. Context Retrieval

Retrieval SHOULD begin with the smallest surface capable of answering the
question:

1. `sley query --json` for declarations and calls;
2. `sley graph --json --slice <node-id>` for one structural neighborhood;
3. `sley ast --json --node <node-id>` for exact syntax structure;
4. `sley check --json` or `sley lint --json` for current defects;
5. `sley plan --json --graft-templates` for compiler-generated edit options;
6. selected spec/schema sections only when the contracts do not answer the
   semantic question.

Clients SHOULD NOT load the entire repository or full AST when a bounded query
or graph slice is sufficient. Retrieval artifacts MUST record the Sley version,
schema IDs, target digest where available, and exact source paths used.

## 8. Diagnostic-Guided Repair Protocol

The default repair loop is:

```text
inspect current Git state
  -> check/lint using JSON
  -> identify diagnostic ID and structural surface
  -> request plan or graph slice
  -> dry-run the selected fix/graft
  -> inspect candidate and provenance
  -> write only with task authority
  -> check + lint + project tests + verify
  -> retain trace/seal evidence when required
```

Clients MUST use diagnostic IDs, node IDs, repair hints, and checked graft
templates when available. They MUST refresh a stale graph or graft precondition
instead of repeatedly guessing. Raw text editing is permitted when no checked
operation expresses the intended change, but it MUST pass the same post-edit
gates and be reported as raw-edit fallback.

## 9. Change And Authority Rules

- Inspection and planning are read-only by default.
- `fix` and `graft` MUST be previewed before `--write` in autonomous workflows.
- Write-mode evaluation MUST use an isolated worktree or disposable task copy.
- Existing user changes MUST be preserved and excluded from benchmark scoring.
- Runtime capabilities do not authorize host actions outside the explicitly
  seeded or operator-approved boundary.
- No AI bootstrap or benchmark may bypass approval for public actions, provider
  spend, deployment, secrets, or external systems.

## 10. Data And Training Records

No model interaction becomes training data by default. A retained record MUST
include:

- stable record and task IDs;
- Sley and schema versions;
- knowledge mode;
- task prompt and allowlisted context identifiers;
- model/provider identifier and decoding parameters when applicable;
- tool calls and schema-validated outputs;
- original source, candidate patch, diagnostics, tests, and final verdict;
- provenance, license, privacy classification, and deduplication key;
- whether the sample is human-authored, model-authored, synthetic, translated,
  or downstream-derived.

Private user data, credentials, endpoints, proprietary prompts, and unsanitized
downstream inputs MUST NOT enter a public or reusable corpus. Evaluation cases
MUST be excluded from training and retrieval indexes by stable IDs and semantic
near-duplicate checks.

`docs/SleyCorpusSpec.md` is the governing admission, provenance, licensing,
privacy, deduplication, contamination, retention, deletion, and release
contract. This section summarizes required record content; it does not itself
authorize capture or retention.

## 11. Benchmark Dependency

Model-training, fine-tuning, and general Sley-fluency claims remain blocked
until SleyBench has its versioned held-out set and baseline reports. The
24-case manifest-mode smoke evaluator is now real, but its deterministic
fixture adapter is not a model and cannot establish model competence.

The split manifest and `sleybench-split` verifier now fail closed across public
commitments and full private audits. This is governance infrastructure, not a
claim that the production 96 public and 24 private cases exist. A public-only
report explicitly leaves `private_material_verified=false`.

The no-manifest `sley-agent-bench` path remains a legacy report-contract
bootstrap. Neither that report nor deterministic-fixture smoke rates may be
presented as model `parse@1`, `compile@1`, `repair@1`, or repository-patch
performance.

## 12. Version Coupling

Every Sley language release MUST review:

- `SLEY_AI.md` and its declared Sley version;
- this specification;
- `docs/SleyBenchSpec.md` and benchmark compatibility;
- `docs/SleyCorpusSpec.md` and corpus compatibility;
- command names and schema IDs cited by the bootstrap;
- common-failure guidance and checked examples;
- benchmark exclusions and contamination ledger.

`scripts/check-ai-foundation.sh` is the minimum deterministic drift gate. A
passing link/version check proves consistency of the bootstrap metadata; it
does not prove semantic completeness or model competence.

## 13. Acceptance Gates

The AI foundation is ready for initial evaluation only when:

1. `SLEY_AI.md` passes its deterministic link, length, and version checks.
2. Every cited command exists in the current CLI or companion-tool inventory.
3. The schema and contract fixtures pass.
4. The Sley local v1 gate passes.
5. SleyBench smoke cases execute in an isolated environment.
6. The production SleyBench split passes a full private audit without placing
   private material in public Git.
7. At least one K1 and one K3 baseline report are retained with complete
   provenance.
8. No benchmark case is present in training or retrieval inputs for the mode
   being scored.
9. Any reusable trajectory or corpus input passes `docs/SleyCorpusSpec.md`.

## 14. Anti-Goals

This specification does not require:

- training a model from scratch;
- making prompts or model outputs language authority;
- hiding compiler failures to improve benchmark scores;
- increasing Sley production territory before product gates pass;
- publishing low-quality synthetic repositories or corpus spam;
- exposing private downstream data;
- treating one model, provider, or benchmark score as the language strategy.
