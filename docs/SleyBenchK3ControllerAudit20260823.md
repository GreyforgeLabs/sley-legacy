# SleyBench K3 Controller Audit

Status: completed local evidence
Date: 2026-08-23
Suite: SleyBench v0, 96 public and 24 private
Model: `gpt-5.6-sol`
Thinking: low

## Decision

K3's original compiler and oracle gains did not improve strict pass because
case churn and model/controller protocol failures offset them. K1 and K3 each
passed 96 of 120 strict cases, but they did not pass the same cases. K3 gained
six strict cases and lost six, while its oracle gained six and lost three.

The audit found one concrete local controller defect. The operator-custody
`openclaw-one-shot-v0` adapter hardcoded `main.sley` or `.` as tool targets and
rejected four valid public repository cases that requested their allowed owned
nested source path. `openclaw-one-shot-v1` now permits `.` or an existing,
regular, nonsymlink owned file while preserving fixed command shapes and both
case and run tool allowlists.

A single controlled K3 v1 replay passed 99 of 120 strict cases, an improvement
of three over K3 v0. Oracle pass improved from 99 to 105. Both partitions
improved, so the measured gain is not a public/private split artifact. This is
one stochastic replay of one model, not a general capability claim.

## Baseline Outcome Audit

| Outcome | K1 public | K1 private | K3 public | K3 private |
| --- | ---: | ---: | ---: | ---: |
| Strict pass | 76/96 | 20/24 | 76/96 | 20/24 |
| Oracle pass | 76/96 | 20/24 | 77/96 | 22/24 |

The identical 96/120 strict result was independently present in each
partition, with both modes at 76/96 public and 20/24 private. The private
partition was slightly easier for both modes, but neither mode's result
depended on one partition compensating for the other.

Strict case transitions from K1 to K3 were:

| Transition | Public | Private | Total |
| --- | ---: | ---: | ---: |
| Fail to fail | 16 | 2 | 18 |
| Fail to pass | 4 | 2 | 6 |
| Pass to fail | 4 | 2 | 6 |
| Pass to pass | 72 | 18 | 90 |

Oracle transitions were 18 fail to fail, six fail to pass, three pass to fail,
and 93 pass to pass. K3 therefore produced a net oracle gain of three but no
net strict gain.

## Baseline Failure Ownership

Every strict failure received one primary stage using protocol failure before
compiler failure, compiler failure before semantic oracle failure, and
minimality as its own strict-only gate.

| Primary stage | K1 public | K1 private | K1 total | K3 public | K3 private | K3 total |
| --- | ---: | ---: | ---: | ---: | ---: | ---: |
| Compile or precheck | 6 | 2 | 8 | 1 | 0 | 1 |
| Semantic oracle | 14 | 2 | 16 | 10 | 2 | 12 |
| Model output protocol | 0 | 0 | 0 | 5 | 1 | 6 |
| Controller tool protocol | 0 | 0 | 0 | 4 | 0 | 4 |
| Minimality | 0 | 0 | 0 | 0 | 1 | 1 |
| Total strict failures | 20 | 4 | 24 | 20 | 4 | 24 |

All six K3 model-output protocol failures concatenated a tool object and a
submit object, despite the one-object-per-turn contract. Two of those cases
passed their oracle without an accepted final submission. The other strict
only losses were one private minimality failure and the four controller
rejections. K3's parser remained intentionally strict; accepting concatenated
objects would hide a model protocol failure and make the evidence less
comparable.

## Controller Defect Replay

Offline replay reproduced all four K3 controller rejections with the same
cause: the requested public repository path was owned and allowed by the case,
but was absent from the controller's hardcoded target list. After the narrow
v1 correction, all four recorded calls passed controller validation.

The correction did not change Sley syntax or semantics, benchmark cases,
oracles, scoring, bootstrap, retrieval, prompt contract, provider, model,
thinking level, or tool-call limit. The adapter identifier was versioned so
the changed control surface cannot be confused with v0 evidence.

## Controlled K3 v1 Replay

| Measure | K3 v0 | K3 v1 | Change |
| --- | ---: | ---: | ---: |
| Strict case pass | 96/120 | 99/120 | +3 |
| Public strict pass | 76/96 | 78/96 | +2 |
| Private strict pass | 20/24 | 21/24 | +1 |
| Oracle `pass@1` | 99/120 | 105/120 | +6 |
| `compile@1` | 99/100 | 100/100 | +1 |
| Repository patch acceptance | 16/20 | 19/20 | +3 |
| Invalid tool call | 4/120 | 1/120 | -3 |
| API-equivalent cost | $1.190948 | $1.173114 | -$0.017834 |

K3 v0 to v1 strict transitions were 15 fail to fail, nine fail to pass, six
pass to fail, and 90 pass to pass. Oracle transitions were 14 fail to fail,
seven fail to pass, one pass to fail, and 98 pass to pass.

Three of the four originally rejected public cases passed with one valid tool
call in v1. The remaining case requested an unowned root path rather than its
owned nested path, so the corrected controller properly rejected it. That
failure belongs to the model, not the harness.

## Remaining K3 v1 Failures

| Primary stage | Public | Private | Total |
| --- | ---: | ---: | ---: |
| Semantic oracle | 9 | 2 | 11 |
| Model output protocol | 8 | 0 | 8 |
| Controller tool protocol | 1 | 0 | 1 |
| Minimality | 0 | 1 | 1 |
| Total strict failures | 18 | 3 | 21 |

All eight v1 model-output failures again concatenated a tool object and a
submit object. Five passed the oracle without a valid final protocol turn. The
minimality failure also passed its oracle. These six cases explain the gap
between 105 oracle passes and 99 strict passes.

After the local controller correction, the main bottleneck is model behavior:
11 semantic-oracle failures, eight one-object protocol failures, one correctly
rejected scope error, and one minimality miss. No further harness or benchmark
change is justified by this audit.

## Controls And Evidence

- Public and private manifests retained the original suite digests, K3
  bootstrap, retrieval digest, system-prompt digest, model, thinking level,
  privacy flags, and $5 partition ceilings.
- The model received one current case per isolated call and never received
  candidates, oracles, manifests, sibling cases, or private custody metadata.
- Private material remains outside public Git under permission-restricted
  operator custody. The complete replay tree was normalized to `0700`
  directories and `0600` files.
- Run controls retain `provider_training_authorized=false`, no persistent
  provider thread, no local session persistence, and minimum-retention intent.
- The configured transport recorded zero incremental provider spend. The
  API-equivalent estimate for this replay was $1.173114, below its $10 combined
  partition ceiling and the operator's $20 goal ceiling.
- Vulcan independently returned PASS with no blockers after checking aggregate
  arithmetic, evidence digests, the controller boundary, custody permissions,
  contract validation, and the public diff for private leakage.

Evidence digests:

- Public aggregate: `sha256:182b799113b20775dc263f85cd12afe1bce03e6a1d7dd08d7f1c1afd2cd3c657`
- Private aggregate: `sha256:9c5e4fc445586bf5aa5335b9ed1ffa1feeda2973e4d546844eb8bc2f75d32830`
- Combined summary: `sha256:6f124ea9e223922031d43a4e8d8b0f133da8d3b1a88ea0122d4776bf12708e37`
- Aggregate audit: `sha256:dfea9ef71bac5c843ebfd242c72c63387e7006b16cdb539a105bab43e4923dfc`
- Independent review: `sha256:78f5e472e080f658a92a8bae860549df1b06d6be003add1a3905582cc932b138`

The full prompts, responses, results, events, retention records, and private
case evidence remain in local custody and are not part of this repository.
