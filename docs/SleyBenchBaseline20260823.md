# SleyBench v0 K1/K3 Baseline

Status: completed local evidence
Date: 2026-08-23
Sley version: 1.1
Model: `gpt-5.6-sol`
Thinking: low
Trials: one per mode

## Decision

The first governed SleyBench v0 baseline is complete over all 120 cases in K1
and K3. Both modes passed 96 of 120 cases under the strict whole-case verdict.
K3 improved compiler-oracle measures but did not improve strict whole-case
success, and it cost more than twice as much as K1.

This supports the narrower conclusion that the compact bootstrap is already a
useful interface for this synthetic baseline. It does not support a general
Sley fluency claim, a provider comparison, or a training decision.

## Results

| Measure | K1 bootstrap | K3 retrieval and tools |
| --- | ---: | ---: |
| Strict case pass | 96/120 (80.0%) | 96/120 (80.0%) |
| Public strict pass | 76/96 | 76/96 |
| Private strict pass | 20/24 | 20/24 |
| Oracle `pass@1` | 96/120 (80.0%) | 99/120 (82.5%) |
| Oracle `pass@1` Wilson 95% | 71.96% to 86.18% | 74.72% to 88.26% |
| `compile@1` | 92/100 (92.0%) | 99/100 (99.0%) |
| `repair@1` | 16/20 (80.0%) | 16/20 (80.0%) |
| Repository patch acceptance | 18/20 (90.0%) | 16/20 (80.0%) |
| Syntax hallucination | 8/120 (6.67%) | 1/120 (0.83%) |
| Invalid tool call | 0/120 | 4/120 (3.33%) |
| Valid tool calls | 0 | 68 |
| Input tokens | 214,206 | 531,479 |
| Output tokens | 6,839 | 12,799 |
| API-equivalent cost | $0.496802 | $1.190948 |

Total scored API-equivalent cost was $1.687750. The configured OAuth transport
recorded zero incremental provider spend. Each scored run had a $5 sub-ceiling,
for a maximum planned matrix ceiling of $20.

## Protocol

- Public/private shape: 96 public and 24 private, with 20 cases per family in
  the combined suite.
- Public IDs use family ordinals `001` through `016`; private IDs use `017`
  through `020`.
- The final split audit passed 24 of 24 checks, including exact, canonical,
  structural, semantic-lineage, and global case-ID isolation.
- Trusted-solution replay passed 96 of 96 public and 24 of 24 private cases.
- K1 received one current case, its workspace, and the pinned `SLEY_AI.md`.
- K3 added pinned `docs/AgentQuickstart.md` retrieval and at most one
  controller-mediated, case-allowlisted compiler tool turn.
- The model never received manifests, candidate fixtures, oracles, sibling
  cases, exclusion ledgers, private custody metadata, or persistent session
  history.
- Local independent oracles scored every submitted candidate. Failures,
  malformed responses, scope failures, and tool-protocol failures remained in
  the denominator.
- The reusable provider runner and full private replay tooling remain in
permission-restricted operator custody outside the Sley 1.x compiler/runtime
boundary so its foreign-language-free gate stays true. Consolidated legacy
components are outside that boundary.

Calibration found and corrected two evaluator defects before scored evidence:
OpenClaw appended local guard output after its JSON envelope, and the first K1
prompt did not explicitly label the active mode. A later production audit also
found public/private case-ID reuse. All affected runs are retained as unscored
calibration evidence, and the split auditor now rejects cross-partition ID
reuse.

## Privacy And Review

Private material remains outside public Git in operator-approved custody with
effective `0700` directory and `0600` file permissions. Captures are local
evaluation evidence only, with `training_eligible=false`,
`retrieval_eligible=false`, no backup enrollment, a 365-day retention class,
and a named deletion owner.

Argus passed the privacy and control-plane gate. Vulcan independently reviewed
task quality and the final re-keyed split under reviewer ID
`vulcan-sleybench-v0-triage-2026-08-23`.

The configured one-shot transport provides no persistent local or provider
thread. Run control explicitly set `provider_training_authorized=false`, and
this work made no training opt-in. OpenAI documents that ChatGPT-plan Codex
inherits the account's ChatGPT training data controls. That account-level
toggle was not exposed to the local CLI, so local evidence proves that this run
did not authorize training but cannot attest the pre-existing server-side
account setting. The configured OAuth surface also exposed no Zero Data
Retention or explicit provider-retention flag, so this report makes no ZDR
claim.

## Evidence Pins

- Split manifest: `sha256:5bc6bbcf1b8d132297355979b071df79e72385a9334ec268a683d8b9aaeaafb3`
- Full split audit: `sha256:996a1b5f9ed6d4348b5b35c7701d7e6f1355f5781ed0e6698e78b23f52d1d579`
- K1 summary: `sha256:0a7dfce78cdf7a4089f9179b18f6685e03daa643dcbe09f9d067f7e1ae3ca4e8`
- K3 summary: `sha256:33b9bbed15fb5752a405e364976384de9a21ecc103674ba864738a06e16bf5fe`

The full prompts, responses, case results, events, aggregates, retention
records, and private audit evidence remain in private local custody. They are
not part of this repository.

The follow-up failure-ownership audit and controlled K3 controller replay are
documented in `docs/SleyBenchK3ControllerAudit20260823.md`. That replay used a
versioned controller-only correction and improved strict pass from 96/120 to
99/120 without changing benchmark cases, Sley semantics, or scoring.

The subsequent model-output audit and single controlled protocol replay are
documented in `docs/SleyBenchK3ProtocolCorrection20260823.md`. A versioned
one-object-per-response clarification reduced concatenated responses from 8
to 0 and improved strict pass from 99/120 to 102/120. Parser, controller,
evaluator, benchmark, and oracle behavior remained strict and unchanged.

The follow-on semantic-oracle audit is documented in
`docs/SleyBenchK3SemanticAudit20260824.md`. It classified the 11 K3 v2
check-pass/oracle-fail cases as seven Sley-specific expression-syntax failures
and four underspecified exact-literal task contracts. Bootstrap 0.2 corrected
the compact syntax guidance; one otherwise controlled replay reached 109/120
strict and 111/120 oracle passes without changing semantics, cases, oracles,
controller, evaluator, or corpus.

The subsequent Bootstrap 0.3 audit is documented in
`docs/SleyBenchK3Bootstrap03Audit20260824.md`. The compact candidate restored
compile from 98/100 to 100/100 but reduced strict and oracle success by two
each. It was rejected after the one authorized replay, leaving Bootstrap 0.2
as the active contract.

## Interpretation

K3 reduced syntax failures and raised oracle pass rate by three cases, but its
strict case-pass count tied K1 because tool and response-protocol failures
offset those gains. K3 also reduced repository patch acceptance from 18/20 to
16/20 and used about 2.4 times the API-equivalent cost.

One model and one trial are insufficient for a general capability claim. The
controller follow-up isolated model semantic, output-protocol, scope, and
minimality behavior; the later protocol replay removed the observed
two-object failure without weakening acceptance. The next justified lab target
is the independently classified semantic-oracle set, not further protocol
tuning, harness expansion, or training on benchmark trajectories.
