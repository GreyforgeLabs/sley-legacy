# SleyBench Specification

Status: production split, governed K3 semantic audit, and W3 evidence contracts complete
Version: 0.6
Sley compatibility: 1.2 with Sley 1.2 W3 evidence contracts
Owner: Sley maintainers
Review trigger: language, compiler contract, task-set, evaluator, or scoring change

## 1. Purpose

SleyBench measures what an AI system can actually do with Sley and which part
of the result comes from native familiarity, the compact bootstrap, retrieval,
or compiler tools.

It is not a compiler conformance suite. Compiler tests establish what Sley
does. SleyBench establishes whether a model can use that behavior to produce a
correct, minimal, verified engineering result.

## 2. Current Boundary

`sley-agent-bench run --json` without a manifest preserves the legacy
`sley.agent_bench.report.v0` contract bootstrap. With `--manifest` and
`--run-manifest`, it executes the schema-backed deterministic evaluator over
fresh Git workspaces, compiler oracles, resource ceilings, and hashed evidence.

The shipped `deterministic-fixture-v0` adapter applies trusted repository-owned
candidate fixtures. It deliberately makes no provider or model call, so its
results prove evaluator operation only and MUST NOT be cited as model fluency.

The operator-custody `openclaw-one-shot-v0`, `openclaw-one-shot-v1`, and
`openclaw-one-shot-v2` adapters evaluate K1 and K3 through a configured
provider model with no persistent local session. Version 1 changes only
controller validation so a
read-only tool target may be `.` or an actual owned path in the isolated
workspace; command shapes and case/run allowlists remain fixed. The adapter
implementation remains outside the foreign-language-free Sley 1.x
compiler/runtime boundary. Consolidated legacy components are outside that
boundary.
Version 2 changes only the K3 model-facing turn contract: a response chooses
`tool` or `submit`, and a tool response must stop and wait for the next prompt.
The strict parser still rejects multiple objects. Each turn receives
exactly one case prompt, that case's workspace, the pinned bootstrap, and only
the retrieval or controller-mediated tool report permitted by its mode. It
never injects manifests, candidate fixtures, oracles, sibling cases, exclusion
ledgers, or custody metadata. Provider responses are applied only to declared
owned paths and are scored by local independent oracles.

### 2.1 W3 protocol and summary evidence

`sley.agent_bench.protocol_replay.v0` is the public deterministic replay shape
for the controller defects already measured in operator custody. It requires
normalized owned paths, one action per turn, sequential turn numbers, explicit
turn/tool/input/output/context/wall limits, and an observed cancellation state.
The replay gate rejects concatenated response objects before schema validation
and rejects multi-action objects without weakening the strict parser.

`greyforge.sleybench.mode_summary.v0` registers the exact shape emitted by the
retained 120-case mode summarizer: fixed six-family counts, 96/24 partitions,
rates, resources, evidence digests, and issues. The in-repository fixture is
synthetic. It contains no private case, response, oracle, or retained result
value. Schema registration does not authorize a provider replay or mutate the
frozen suite.

## 3. Evaluation Tiers

### 3.1 Smoke tier

`SleyBench-smoke-v0` contains 24 public cases: four cases from each task family
in Section 5. It validates evaluator wiring, isolation, tool capture, schema
validation, and scoring. Smoke results are diagnostic and support no general
fluency claim.

### 3.2 Baseline tier

`SleyBench-v0` contains 120 cases:

- 96 public development cases, 16 from each task family;
- 24 private held-out cases, four from each task family.

Each reported model/mode pair runs all 120 cases. Stochastic model modes SHOULD
use three independently seeded trials, reported separately and in aggregate.
This yields enough cases to expose category-level failures without pretending
to provide a permanent or exhaustive language census.

### 3.3 Repository tier

Repository tasks are reported separately because their cost, context size, and
failure modes differ materially from single-file cases. The first repository
tier SHOULD contain at least 20 held-out patches across no fewer than five
small, licensed fixture repositories.

## 4. Knowledge Modes

Every run MUST use exactly one mode from `docs/SleyAISpec.md`:

- K0: unaided;
- K1: pinned `SLEY_AI.md` only;
- K2: allowlisted documentation retrieval;
- K3: read-only compiler tools;
- K4: checked writes in an isolated worktree.

The evaluator MUST record the exact bootstrap digest, retrieval corpus digest,
tool allowlist, and system prompt. Comparisons are valid only when task set,
evaluator version, scoring version, and resource budget match.

## 5. Task Families

### F1: syntax and compilation

Create or complete bounded Sley programs that parse and check. Cases cover
modules, tasks, takes, types, bindings, calls, collections, control flow,
results, and effects.

Primary metrics: `parse@1`, `compile@1`, syntax hallucination rate.

### F2: semantic implementation

Implement a deterministic function from a language-neutral contract and tests.
Cases require more than syntax and include boundary values, error flow, and
effect declarations.

Primary metrics: `pass@1`, exact semantic match, minimality.

### F3: diagnosis and repair

Given rejected or linting source, identify the failure and produce the smallest
verified repair. Cases include misleading surface symptoms so success requires
compiler evidence rather than keyword substitution.

Primary metrics: diagnosis accuracy, `repair@1`, regression-free repair,
compiler cycles to success.

### F4: translation and preservation

Translate a small, licensed or synthetic TypeScript/Python/pseudocode function
into Sley while preserving an explicit input/output oracle. Formatting alone
does not count as success.

Primary metrics: compile rate, semantic parity, unsupported-library
hallucination rate.

### F5: structural tool use

Answer or change a program using AST nodes, query rows, graph slices,
diagnostics, edit plans, and dry-run grafts. Cases test stale preconditions,
bounded retrieval, and correct schema selection.

Primary metrics: tool selection accuracy, invalid tool-call rate, successful
patch tool count, raw-edit fallback rate.

### F6: repository patch and integration

Make a bounded multi-file change in an isolated fixture repository, preserve
unrelated state, and pass declared project gates. Some cases require deciding
not to edit because the premise is false or the requested behavior already
exists.

Primary metrics: accepted patch rate, regression rate, scope precision,
evidence completeness.

## 6. Case Contract

Every case MUST have a versioned manifest record containing at least:

```json
{
  "id": "sleybench-v0-f3-001",
  "family": "diagnosis_repair",
  "visibility": "public",
  "sley_version": "1.2",
  "prompt_path": "cases/f3/001/prompt.md",
  "workspace_path": "cases/f3/001/workspace",
  "candidate_path": "cases/f3/001/candidate",
  "owned_paths": ["main.sley"],
  "allowed_context": ["SLEY_AI.md"],
  "allowed_tools": ["check", "lint", "plan", "verify"],
  "limits": {
    "wall_seconds": 180,
    "tool_calls": 20,
    "output_bytes": 1048576,
    "processes": 32,
    "workspace_bytes": 1048576
  },
  "policy": {"kind": "fixture_candidate"},
  "preflight": [],
  "oracle": {
    "commands": [
      {
        "name": "check",
        "argv": ["sley", "check", "--json", "main.sley"],
        "expected_exit": 0,
        "expected_schema": "sley.diagnostics.report.v0",
        "assertions": [{"pointer": "/status", "value": true}]
      }
    ]
  },
  "score_tags": ["compile", "repair"],
  "minimality": {"max_changed_files": 1, "max_changed_lines": 8},
  "training_exclusion_id": "sha256:<prompt-and-workspace-digest>",
  "candidate_digest": "sha256:<candidate-tree-digest>",
  "semantic_lineage_id": "reviewed-lineage-id",
  "semantic_lineage_digest": "sha256:<normalized-semantic-digest>"
}
```

The schema rejects unknown fields, requires positive resource limits, pins
every oracle command, and distinguishes public development cases from private
held-out cases. Baseline cases also require a candidate tree digest plus a
reviewed semantic-lineage ID and normalized digest. The exact training
exclusion digest covers the prompt and initial workspace. The separate
candidate digest commits the trusted solution without making that solution a
training exclusion identity.

A case that intentionally requires no edit MAY represent its candidate as a
Git-absent empty directory only when both minimality limits are zero. A
baseline case MUST additionally pin the SHA-256 empty-tree candidate digest.
Verifiers and evaluators MUST reject any other missing candidate path. This
preserves no-op benchmark semantics across clean Git checkouts, which cannot
retain empty directories.

### 6.1 Split contract

`sley.agent_bench.split_manifest.v0` commits the 96-case public manifest and
ledger by path and digest, and the 24-case private manifest and ledger by opaque
digest only. It requires six public groups of 16, six private groups of four,
`material_in_public_git=false`, an explicit private custody reference, and
semantic-lineage splitting. Pinned state additionally requires a named review
and timestamp.

Case IDs MUST also be globally unique across public and private partitions.
For v0, public family ordinals are `001` through `016` and private family
ordinals are `017` through `020`. Content isolation without ID isolation is an
invalid production split because combined 120-case evidence would be
ambiguous.

`sleybench-split verify-public` checks public case content, candidate digests,
lineage fields, ledger coverage, and commitments. Its report always sets
`verification_level=public_commitment` and
`private_material_verified=false`. It cannot establish a complete split.

`sleybench-split audit` additionally accepts explicit private manifest and
private exclusion-ledger paths outside the public repository. It checks all
120 cases, exact and candidate content digests, family balance, ledger pins,
and exact, canonical, structural, and semantic-lineage isolation. It passes
only for a pinned, approved split with both exclusion ledgers pinned. Failed
audits set `private_material_verified=false`.

## 7. Evaluator Contract

The evaluator MUST:

1. materialize a fresh task workspace from a content-addressed fixture;
2. record clean Git state before the run;
3. inject only the context and tools allowed by the knowledge mode;
4. enforce wall-time, output, process, and tool-call ceilings;
5. capture prompts, responses, tool requests, tool reports, exits, and stderr;
6. apply the candidate patch only inside the isolated workspace;
7. run the pinned compiler and case oracles independently of the model;
8. detect changes outside owned paths and fail scope precision when present;
9. emit a schema-validated result plus hashes for inputs, outputs, and evidence;
10. destroy or retain the workspace according to the run manifest.

`sley-sandbox-runner` is seeded replay evidence, not an OS isolation boundary.
Untrusted generated programs require an actual process/container sandbox with
network disabled, bounded writable storage, resource ceilings, and no ambient
credentials.

## 8. Scoring

### 8.1 Per-case scores

Each case records boolean and numeric fields rather than only one pass flag:

- prompt accepted;
- parse passed;
- check passed;
- oracle tests passed;
- semantic output matched;
- expected diagnosis identified;
- repair was minimal;
- no unrelated files changed;
- required evidence was produced;
- tool calls, compiler cycles, input/output tokens, wall time, and estimated
  provider cost;
- nonexistent syntax, command, schema, standard-library, or capability claims.

### 8.2 Aggregate metrics

Required aggregates are:

- `parse@1`;
- `compile@1`;
- `pass@1`;
- `repair@1`;
- repository patch acceptance;
- regression-free success;
- syntax hallucination rate;
- nonexistent standard-library hallucination rate;
- invalid tool-call rate;
- median and p95 tool calls to success;
- median and p95 context tokens, output tokens, wall time, and cost.

Rates MUST include numerator and denominator and a 95% Wilson interval. Averages
MUST include distribution summaries; a mean alone is insufficient for skewed
cost and latency data. Failed, timed-out, and malformed runs remain in the
denominator.

### 8.3 No compensating aggregate

SleyBench MUST NOT collapse correctness and cost into a single score that lets
cheap failures compensate for correct work. Report correctness first, then
resource use among all runs and among successful runs.

## 9. Held-Out And Contamination Rules

- Private held-out prompts, fixtures, solutions, tests, and semantic variants
  MUST NOT enter public Git history, model prompts outside an authorized run,
  retrieval indexes, fine-tuning data, or synthetic generation seeds.
- Public and private cases MUST be grouped by semantic lineage before splitting;
  superficial renaming does not create independence.
- Each case MUST have exact and normalized semantic digests.
- Suspected contamination MUST be recorded and the affected result reported
  separately, not silently discarded.
- A model trained on SleyBench development cases MAY be measured on the private
  held-out set but MUST be labeled as benchmark-aware.
- Held-out cases SHOULD rotate only on a versioned release, not after seeing an
  inconvenient score.

## 10. Baseline Matrix

The first accepted baseline SHOULD compare at least:

| Run | Purpose |
| --- | --- |
| K0 | Native Sley familiarity |
| K1 | Value of the compact bootstrap |
| K2 | Value of documentation retrieval |
| K3 | Value of read-only compiler tooling |
| K4 | End-to-end checked patch performance |

At least two capable model families SHOULD be evaluated so results do not
become vendor-specific. No provider call, spend, or external model test is
authorized by this specification; each run still requires the operator's
normal execution approval.

## 11. Claim Gates

No general claim such as "AI-fluent," "AI-native models can program Sley," or
"fine-tuning improved Sley" is supported until:

1. all 120 SleyBench-v0 cases run under the claimed knowledge mode;
2. the private held-out subset remains uncontaminated;
3. every report validates against the pinned result schema;
4. failures and timeouts remain in the denominator;
5. the exact model, parameters, prompt, bootstrap, retrieval set, tools,
   evaluator, compiler, and case-set versions are disclosed;
6. the claim states task-family limits and confidence intervals;
7. a second maintainer or independent replay verifies the evidence bundle.

The 24-case smoke tier may support only claims about evaluator operation, never
model fluency.

## 12. Implementation Status And Order

1. Complete: strict JSON Schemas for case manifests, run manifests, event
   JSONL, per-case results, and aggregate reports.
2. Complete: manifest mode dispatches to a real deterministic evaluator while
   no-manifest mode preserves the legacy bootstrap contract.
3. Complete: 24 public smoke cases, four per task family.
4. Complete for the trusted-fixture smoke boundary: clean workspace checks,
   tool/process/wall/output/workspace limits, content hashes, malformed schema
   rejection, scope precision, oracle execution, and retention cleanup.
5. Complete: strict public split commitment and full private audit contracts,
   read-only `sleybench-split`, and synthetic 120-case fail-closed tests.
6. Complete: 96 public and 24 private v0 cases are balanced by family,
   independently reviewed, held in approved custody, globally ID-disjoint,
   and pinned by a full exact-through-semantic audit.
7. Complete: real K1 and K3 baselines are retained for `gpt-5.6-sol` over all
   120 cases. Calibration runs remain explicitly unscored. See
   `docs/SleyBenchBaseline20260823.md`.
8. Complete: a controlled K3 replay under `openclaw-one-shot-v1` measured the
   correction for owned nested-path compiler tool targets without changing
   cases, language semantics, bootstrap, retrieval, or scoring. Strict pass
   improved from 96/120 to 99/120. See
   `docs/SleyBenchK3ControllerAudit20260823.md`.
9. Complete: a controlled K3 replay under `openclaw-one-shot-v2` clarified the
   one-object-per-response turn boundary without changing parser, controller,
   evaluator, cases, oracles, or Sley semantics. Concatenated model responses
   fell from 8 to 0 and strict pass improved from 99/120 to 102/120. See
   `docs/SleyBenchK3ProtocolCorrection20260823.md`.
10. Complete: the 11 K3 v2 check-pass/oracle-fail cases were classified as
   seven Sley-specific expression-syntax failures and four underspecified
   exact-literal task contracts. Bootstrap 0.2 corrected the general compact
   documentation gap; one controlled replay reached 109/120 strict and
   111/120 oracle passes without changing language semantics, cases, oracles,
   controller, evaluator, or corpus. See
   `docs/SleyBenchK3SemanticAudit20260824.md`.
11. Complete, candidate rejected: Bootstrap 0.3 clarified task-parameter
   placement and removed the unenclosed statement-form conditional example.
   One controlled replay restored compile from 98/100 to 100/100 but reduced
   strict success from 109/120 to 107/120. Bootstrap 0.2 remains active; no
   second trial was run. See `docs/SleyBenchK3Bootstrap03Audit20260824.md`.
12. Complete: `docs/SleyCorpusSpec.md` defines default-deny corpus admission,
   provenance, license, privacy, verification, deduplication, contamination,
   retention, deletion, and release rules. Capture remains disabled until its
   implementation gates pass and an exact run is approved.
13. Implemented for Sley 1.2 W3: strict protocol replay and combined
    mode-summary schemas, synthetic fixtures, budget/cancellation negative
    cases, and deterministic replay gates. No benchmark case, oracle, split,
    custody record, or provider adapter changed.
