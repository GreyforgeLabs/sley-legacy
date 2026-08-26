# Sley 1.2 operational evidence

Status: active methodology baseline
Owner: Sley maintainers
Review trigger: benchmark, worker, dogfood, or claim change

## Evidence rule

Semantic proof and operational evidence answer different questions. Compiler,
oracle, and test evidence establish correctness for the tested boundary.
Runtime, context, token, and review measures establish whether Sley produces a
practical advantage. Neither substitutes for the other.

Every comparison records:

- incumbent and candidate source/runtime digests;
- canonical input/output contract and independent oracle;
- frozen corpus or property suite;
- authority state;
- cold and warm latency, throughput, memory, and host-boundary cost where
  applicable;
- strict task success, tool calls, compiler cycles, invalid actions, repair
  loops, context bytes, and observable tokens;
- failures and regressions in the denominator;
- privacy, training, retrieval, retention, and disclosure eligibility;
- promotion, deferral, or rejection rationale.

Accepted Change Tokens are defined and the S12-602 assembler now computes the
measure for accepted attempts:

```text
ACT = total observable agent tokens / accepted correct changes
```

ACT is never reported without strict correctness and workload scope. A failed
single-case arm has no finite per-arm ACT value, but all of its observable
tokens remain recorded in the attempted-work denominator.

## Current admissible claims

- One `gpt-5.6-sol` low-thinking trial over a governed 120-case split produced
  the retained Bootstrap 0.2 result of 109/120 strict and 111/120 oracle passes.
- Controller and response-contract corrections removed named mechanistic
  failures without weakening validation.
- Bootstrap 0.3 was rejected because broad strict/oracle performance regressed
  despite recovering compile performance.
- At Sley commit `73bc5b0`, the bounded Siglum numerology candidate matched the
  frozen independent TypeScript oracle over two fresh deterministic 10,000-case
  runs with zero mismatches through the local machine-host boundary.

These facts do not establish general Sley fluency, provider superiority,
training value, production latency, production authority, or broad Greyforge
adoption.

## Evidence gates for W6

1. Replay one independent-oracle workload through the bounded worker.
2. Record cold/warm execution and resource limits.
3. Run one controlled agent task with bounded structural context.
4. Preserve all failures and compare against an incumbent/raw-file path under
   equal budgets.
5. Issue an explicit non-authoritative promotion/deferral decision.

No evidence capture authorizes provider spend, corpus admission, model
training, publication, or product cutover.

## S12-601 bounded reference replay

The machine-readable packet is
`reports/operational/siglum-numerology-reference-v1.json`. It binds Sley commit
`73bc5b0`, Siglum commit `2721acb`, the candidate source, runtime, frozen input
corpus, candidate output corpus, 20 shard digests, pin set, and retained prior
parity report.

The fresh replay established:

- two complete 10,000-case runs, each with 10,000 exact matches and zero
  mismatches;
- identical result, mismatch-report, corpus, source, runtime, and shard
  digests across both runs;
- 5,084 ms cold persistent-host response and 80/81 ms warm responses;
- 229,800 ms and 230,345 ms full-run duration, or 43.52 and 43.41 cases per
  second on this reference host;
- 1,691,553,792 bytes sampled peak process-tree RSS for the full controller and
  122,748,928 bytes for the persistent probe;
- a 32 GiB per-process virtual-address ceiling, distinct from measured RSS,
  plus 120-second shard, step, call-depth, collection, response, output, and
  file-descriptor limits;
- credential-environment scrubbing, but no claim of OS network namespace
  isolation.

Negative evidence remains part of the packet. The first replay attempt exposed
multiline typed record constructors as unsupported `Raw`; commit `b97fba8`
made those nodes structural without widening the generic recovery allowlist.
Full attempts under 4 GiB and 8 GiB virtual-address ceilings failed before
corpus execution during Node/V8 WebAssembly reservation. A bounded startup
sweep established 32 GiB as the minimum tested passing ceiling, while sampled
RSS remained far lower.

S12-601 is non-authoritative evidence only. Production shadow, production
latency, fallback, rollback, and promotion authority are still absent. The
packet therefore defers promotion and makes no production-adoption claim.

## S12-602 controlled agent workflow infrastructure

The public fixture under `fixtures/operational/agent-workflow-v1` defines one
real bounded repository-maintenance task: rename an exported Sley task and its
cross-module call site while preserving signature, effects, and behavior.

The no-call plan compares:

- K1 incumbent workflow: complete raw workspace, no compiler tool turn;
- K3 Sley workflow: the same raw workspace plus exactly one read-only task
  query and one possible follow-up submission turn.

Both arms pin `gpt-5.6-sol` with low thinking, Bootstrap 0.2, the same prompt,
workspace, owned paths, empty retrieval set, strict oracle, model context
window, and 1 USD per-arm spend ceiling. K3's
one additional structural turn is the declared action-budget difference. The
pricing fields are retained comparison estimates and must be verified before
any approved execution. The plan records the exact private controller digest
without embedding its path or contents.

The unchanged private controller does not generically enforce every resource
limit declared by the case manifest. Those declarations are not reported as
verified equal controls. The defensible equality claim is limited to the task,
model context window, and enforced spend ceiling; tool and inference turns are
the controlled difference.

The independent oracle is stronger than compilation or the runner's
informational candidate digest. It requires the exact renamed pipeline task
inventory and signature/effects, exact updated main call inventory, locked
effect-mocked behavior, and a two-file/four-line minimality boundary.
For provenance binding, the candidate and final-tree digests cover the complete
materialized workspace after overlaying those two owned candidate files onto
the baseline. The changed-file directory is not misrepresented as a complete
final workspace.

`sley.operational.agent_workflow_plan.v1` and
`sley.operational.agent_workflow_comparison.v1` are registered extensible
contracts with synthetic public fixtures. The scrubbed assembler reports
strict success, ACT, tokens, prompt/response bytes, supplied workspace bytes,
structural response bytes, wall time, tool calls, compiler cycles, invalid
actions, repair-loop observability, changed files/lines, environment, and
authority. It embeds no raw prompt, response, tool report, approval text, user
data, or credential.

Comparison assembly accepts only a private approval record whose exact commit,
case, run-manifest, controller, model, spend, and denied-action fields match the
plan. A K3 aggregate that reports a structural query must also supply the
corresponding second-turn interaction history and bounded task-query report.
The emitted environment is explicitly scoped to the comparison host, and the
context result is classified as a saving, equality, or expansion.

It also accepts only an exact private execution-provenance record binding the
approval, controller, aggregate and evidence digests, observed inference
attempts, provider/model assertion, and a passing independent evidence review.
This is not a cryptographic provider attestation; the comparison retains that
limitation and cannot turn it into a general or production claim.

Deterministic infrastructure tests pass, including a negative K3 attempt that
does not exercise the structural query. That attempt is marked unsuccessful
and retained in the denominator.

The approved real comparison is
`reports/operational/agent-workflow-comparison-v1.json`. In one retained trial,
both arms produced the accepted two-file/four-line change and passed every
oracle step. K1 used 3,404 tokens, 12,381 prompt bytes, zero tool calls, and
13,223 ms. K3 used 7,815 tokens, 28,888 prompt bytes, one read-only structural
query, and 36,810 ms. The structural arm therefore expanded context and cost
4,411 additional tokens and 23,587 additional milliseconds in this trial.
That negative result is retained. It supports neither a structural advantage
claim nor a general model-fluency or production claim.

## W6 closeout

- `S12-601`: complete; the retained packet and public wrapper passed
  independent Vulcan QA with no remaining High or Medium findings.
- `S12-602`: complete. The approved K1/K3 execution, exact private evidence,
  reviewed assertion-based provenance, and scrubbed real comparison are
  retained. Provider origin is not cryptographically attested.
- `S12-603`: the fail-closed decision/registry infrastructure is implemented.
  `sley operational-evidence assemble` requires the exact S12-601 packet, a
  real non-synthetic S12-602 comparison, matching public fixture and private
  controller digests, exact reassembly from the private approval, aggregates,
  evidence bundles, and reviewed execution-provenance assertion, ancestor
  commits, retained failures and limitations,
  scrubbed disclosure, and non-authoritative decisions. The synthetic registry
  fixture validates shape only. Tier 2 `make core` passed with 94 schemas,
  182/182 contract fixtures, 259/259 declared integration checks, and all 37
  v1 targets. Vulcan's two Medium provenance/source-identity findings were
  corrected; its blocking recheck returned PASS with no remaining High or
  Medium findings. The final registry is
  `reports/operational/evidence-registry-v1.json`. Vulcan reassembled the real
  comparison from the exact nested evidence roots, verified disclosure and
  registry binding, and returned PASS with 0 High and 0 Medium findings.
  W6 is complete and W7 release closeout is unblocked. Production promotion,
  provider execution, runtime mutation, deployment, and publication remain
  unauthorized by the registry.
