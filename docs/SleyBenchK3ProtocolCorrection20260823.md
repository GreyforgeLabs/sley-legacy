# SleyBench K3 Model-Output Protocol Correction

Status: completed local evidence
Date: 2026-08-23
Suite: SleyBench v0, 96 public and 24 private
Owner: Sley maintainers

## Target-State Contract

Desired end state: K3 presents the existing one-object-per-response contract
unambiguously, while the strict parser continues to reject every response that
contains more than one protocol object.

Owned surfaces are the permission-restricted model-runner prompt and regression
test, the public run-manifest adapter allowlist and contract fixture, and this
scrubbed evidence report. Sley semantics, benchmark cases, corpus, split,
oracles, evaluator scoring, controller tool shapes and allowlists, ownership,
privacy, retention, and training controls are protected and must not change.

Validation proceeds from the private runner regression to a change-aware local
gate, then one controlled 120-case K3 replay with the same
model, thinking level, bootstrap, retrieval, cases, oracles, ceilings, and
partition custody. Rollback is removal of the v2 adapter surface and continued
use of the retained v1 evidence; no stored evidence is overwritten.

## Initial Failure Classification

The eight v1 failures are five public syntax-and-compilation cases and three
public semantic-implementation cases. Every failure has the same output shape:

- first K3 turn;
- two adjacent, individually valid JSON objects with no separator;
- one `tool(check)` object followed by one `submit` object;
- identical candidate file maps in both objects;
- no accepted tool call, controller action, or invalid-tool classification
  because strict JSON parsing fails first.

Five compile-only cases passed their independent oracle against the unchanged
workspace; the three semantic cases did not. These outcomes remain failures
because no valid final protocol object was accepted.

## Root-Cause Finding

Confidence: high.

The failure is model-facing rather than controller- or evaluator-owned. K1 has
no corresponding two-object failures. K3 said a model could request a tool and
“then submit,” but did not explicitly require the model to stop after the tool
object and wait for the next prompt. Both examples also carry complete file
maps, making the two stages easy to reproduce as a single chained response.

The v2 correction adds only the missing turn boundary: choose `tool` or
`submit` for the current response; after `tool`, emit only that object, stop,
and wait for the tool report before submitting on the next response. The
strict parser is unchanged and still rejects concatenated objects.

The runner selects this wording only for `openclaw-one-shot-v2`. K1 and K3 v1
retain their exact historical system-prompt digests, so the prior baseline
remains reproducible rather than being silently reinterpreted by new prompt
text.

## Controlled Replay

The single approved replay used the same `gpt-5.6-sol` model, low thinking
level, local one-shot transport, four workers, benchmark cases, split,
bootstrap, retrieval material, case tool allowlists, evaluator, oracles,
scoring, ownership rules, data policy, and $5 per-partition ceiling as K3 v1.
Only the run ID, adapter ID, and system-prompt digest changed. Normalizing
those three fields makes each v1/v2 partition control identical.

| Measure | K3 v1 | K3 protocol v2 | Delta |
| --- | ---: | ---: | ---: |
| Strict case pass | 99/120 | 102/120 | +3 |
| Public strict pass | 78/96 | 81/96 | +3 |
| Private strict pass | 21/24 | 21/24 | 0 |
| Oracle `pass@1` | 105/120 | 103/120 | -2 |
| `compile@1` | 100/100 | 100/100 | 0 |
| Repository patch acceptance | 19/20 | 18/20 | -1 |
| Two-object responses | 8 | 0 | -8 |
| Other malformed responses | 0 | 0 | 0 |
| Invalid tool calls | 1/120 | 1/120 | 0 |
| Valid tool calls | 65 | 83 | +18 |
| Input tokens | 523,327 | 582,018 | +58,691 |
| Output tokens | 12,646 | 13,630 | +984 |
| API-equivalent cost | $1.173114 | $1.300336 | +$0.127222 |

All 203 v2 responses were individually valid single JSON objects. Each of the
original eight affected cases completed one accepted `check` request followed
by a separate submit response. Six then passed strictly; the remaining two
failed only their unchanged semantic oracle. No invalid response was selected,
discarded, repaired, or canonicalized.

The paired strict transitions were six fail-to-pass and three pass-to-fail
(exact two-sided McNemar p=0.5078125). Oracle transitions were one
fail-to-pass and three pass-to-fail (p=0.625). The one-trial result therefore
does not establish a general capability gain or a material oracle regression.
It does establish the narrower mechanistic result: the named protocol failure
fell from eight occurrences to zero while compiler performance stayed perfect.

## Decision And Headroom

Accept `openclaw-one-shot-v2` as the corrected K3 response contract. Confidence
is high that the original failure was an ambiguous model-facing turn boundary,
not a controller or evaluator defect. Confidence is low that the observed
three-case net strict gain will reproduce exactly in another stochastic trial.

The correction recovered three net strict passes for $0.127222 additional
API-equivalent cost, or $0.042407 per net pass, while remaining far below the
$10 combined replay ceiling. That is economical for this bounded correction,
but further prompt tuning has little justified headroom: the targeted protocol
count is already zero and another provider replay is not warranted. The next
lab target is the 11 semantic-oracle failures independently classified in the
K3 v1 failure audit, with no corpus, training, case, or oracle redesign implied.

## Evidence Pins

- K3 v2 system contract: `sha256:8a33353f992710c6423910c170064d95115646de657cdcc7b88dc487882f7c85`
- Permission-restricted runner: `sha256:4e675d08e9eea7207abb5f67a6dc1eb0f654a633804abce79404c9306fd395a6`
- Permission-restricted regression: `sha256:ab1bae5da780a595edfb17a10c99fb2f8fc487682e13e9ce13c4aeb7b2e02f42`
- Public control: `sha256:8cbc83bc4dbe53a07e261b608ea51ca5ebf5bd2f80cbe114818fa8be200dd5c4`
- Private control: `sha256:6d232788cf0f804dbf248da0949e959add50d57ce2843d91ded8d9d2fc590c57`
- Public aggregate: `sha256:779ede663eaff38528e2331960669b0cc237a8e394a568ca7f8033ebc7ced040`
- Private aggregate: `sha256:9f7ee01c710fb07f862e12d37ca13e1311ccb120b68ebbd8aa784641bb54b14f`
- Combined summary: `sha256:7e48a35ae2fd8ec59e6e505470f377b466c8dba74d90c72c0178c9fcb65c71e8`

Full prompts, responses, per-case results, events, aggregates, controls, and
private audit material remain in permission-restricted operator custody. This
report contains only scrubbed aggregate evidence.
