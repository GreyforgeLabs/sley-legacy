# SleyBench K3 Bootstrap 0.3 Audit

Status: completed local evidence; candidate rejected
Date: 2026-08-24
Suite: SleyBench v0, 96 public and 24 private
Model: `gpt-5.6-sol`, low thinking
Owner: Sley maintainers

## Decision

Bootstrap 0.3 is not promoted. It restored `compile@1` from 98/100 to 100/100,
removed both syntax hallucinations, and fixed the two expression cases that
still failed under Bootstrap 0.2. It did not preserve the requested broad
reliability: strict success fell from 109/120 to 107/120 and oracle success
fell from 111/120 to 109/120. Bootstrap 0.2 therefore remains the active
`SLEY_AI.md` contract.

The paired strict and oracle differences are statistically inconclusive. That
does not rescue the candidate: this goal required preserving 109 strict while
recovering compile performance, and the one authorized replay did not meet
that target. No second trial, case repair, or prompt tuning was performed.

Confidence is high that the two compile regressions exposed ambiguous
parameter-placement guidance, moderate that the unenclosed statement example
contributed to the two remaining expression failures, and low that the seven
new strict failures were caused by Bootstrap 0.3 rather than ordinary
one-trial variation.

## Existing-Evidence Audit

The two compile regressions between K3 v2 and Bootstrap 0.2 were public cases
`sleybench-v0-f1-013` and `sleybench-v0-f2-014`. Both had compiled under K3 v2.
Both Bootstrap 0.2 responses independently invented the same invalid shape:
`take` was appended to a task header instead of declared inside its body. The
compiler rejected both candidates. This is a model-facing wording ambiguity,
not a Sley semantic or runtime limitation.

The two Sley-expression cases still failing after Bootstrap 0.2 comprised one
public and one held-out translation case. Both replaced a value-producing
expression with a standalone module-level `if`, matching the bootstrap's
unenclosed multiline statement-form example. Both passed stage-1 checking
through partial or full `Raw` fallback and then failed independent execution.
This is an interaction between guidance sections plus an existing checker
diagnostic limitation.

No evidence shows that the bootstrap's total size alone caused either pattern.
The candidate was nevertheless made smaller, not larger, before replay.

## Candidate

Bootstrap 0.3 made three general changes only:

- the single minimal program demonstrated `take` inside a task body;
- the separate unenclosed multiline `if` example was removed;
- conditional-value guidance preserved the expression inside its enclosing
  task and identified module-level `if` as invalid.

It did not mention benchmark cases or answers.

| Footprint | Bootstrap 0.2 | Candidate 0.3 | Delta |
| --- | ---: | ---: | ---: |
| Lines | 180 | 175 | -5 |
| Words | 959 | 956 | -3 |
| Bytes | 6,901 | 6,883 | -18 |
| `o200k_base` tokens | 1,709 | 1,707 | -2 |

The candidate digest was
`sha256:83f4b74c95f006c366044fc6fc6118bf8aac6152821211f5a8628cb3d09b6fdd`.
The retained Bootstrap 0.2 digest is
`sha256:fd048d403b43ff05c28da4b68ac7831f2b227506d168637d4a5c1757c9ae2fcd`.

## Controlled Replay

The single replay used the same 120 cases, 96/24 split,
`openclaw-one-shot-v2` adapter, system prompt, model, thinking level,
retrieval document, compiler tools, evaluator, scoring, ownership, ceilings,
four-worker setting, privacy, retention, and training controls as Bootstrap
0.2. Normalizing the run ID and bootstrap digest makes both partition controls
identical.

| Measure | Bootstrap 0.2 | Candidate 0.3 | Delta |
| --- | ---: | ---: | ---: |
| Strict case pass | 109/120 | 107/120 | -2 |
| Public strict pass | 89/96 | 85/96 | -4 |
| Private strict pass | 20/24 | 22/24 | +2 |
| Oracle `pass@1` | 111/120 | 109/120 | -2 |
| `compile@1` | 98/100 | 100/100 | +2 |
| Repository patch acceptance | 18/20 | 18/20 | 0 |
| Syntax hallucination | 2/120 | 0/120 | -2 |
| Invalid tool calls | 4/120 | 4/120 | 0 |
| Valid tool calls | 55 | 64 | +9 |
| Input tokens | 530,541 | 556,788 | +26,247 |
| Output tokens | 11,610 | 12,413 | +803 |
| API-equivalent cost | $1.177182 | $1.237706 | +$0.060524 |

Incremental provider spend recorded by both partitions was zero.

Strict transitions were five fail-to-pass and seven pass-to-fail (exact
two-sided McNemar p=0.7744140625). Oracle transitions were four fail-to-pass
and six pass-to-fail (p=0.75390625). Compile transitions were two fail-to-pass
and zero pass-to-fail (p=0.5). No case was discarded, repaired, or rerun.

## Targeted Expression Result

The candidate fixed both named compile regressions and both expression cases
that still failed under Bootstrap 0.2. It did not improve the complete original
seven-case expression set: three cases that passed under 0.2 failed semantic
evaluation under 0.3 despite using executable Sley syntax.

| Public case | Bootstrap 0.2 | Candidate 0.3 |
| --- | ---: | ---: |
| `sleybench-v0-f2-009` | Pass | Fail |
| `sleybench-v0-f4-003` | Pass | Fail |
| `sleybench-v0-f4-005` | Fail | Pass |
| `sleybench-v0-f4-008` | Pass | Pass |
| `sleybench-v0-f4-011` | Pass | Fail |
| `sleybench-v0-f4-012` | Pass | Pass |
| Held-out expression case | Fail | Pass |
| **Total** | **5/7** | **4/7** |

The experimental candidate therefore left three expression-set failures,
versus two under the retained bootstrap. The new failures used valid symbolic
operators but produced the wrong Boolean meaning; they were not recurrences of
the foreign-operator or module-level-statement syntax defects.

## Broader Regressions

The seven strict pass-to-fail transitions comprised six public semantic/oracle
failures with syntactically valid Sley and one disallowed structural tool call.
One semantic regression was an already classified underspecified exact-literal
contract. None demonstrates a new language or compiler regression, and the
single trial cannot assign them causally to the compact wording change.

That uncertainty cuts both ways. It is not evidence that broad reliability was
preserved. Promotion would substitute a lower observed strict result for the
accepted 109/120 result to recover two compile cases, contrary to this goal's
acceptance rule.

## Stop Decision And Problem Ledger

Compact AI-facing guidance did not cleanly solve the complete expression set.
Stop prompt tuning here. Retain these implementation candidates without acting
on them in this goal:

- emit an actionable diagnostic when an executable expression remains `Raw`,
  especially for foreign Boolean or conditional spellings;
- make the expression-versus-statement boundary and invalid module-level
  control flow explicit in parser/checker diagnostics;
- improve diagnostics for invalid task-parameter placement;
- decide separately whether multiline conditional expressions should be
  supported or rejected with a dedicated grammar diagnostic.

The four underspecified exact-literal contracts remain unchanged and are not
part of this correction. No Raw diagnostic, multiline-conditional change,
language feature, case, oracle, parser, controller, corpus, training,
deployment, publication, merge, or push was performed.

## Evidence Pins

- Public control: `sha256:cc2791f5e1745137dbf760a624a9f4a0790c0b327372a088673451a48a48dac1`
- Private control: `sha256:9f1b32a49075a074693f1a36c1d2b706500c8927f78504b286d9fa009ccf6e3b`
- Public aggregate: `sha256:652dcd43bc5f4fb99675f61e39ac2be3b66bb7ff68f0fe7c854f0761ad39a9b7`
- Private aggregate: `sha256:5b556f48079b7f8b87b850f3b617716f6528a73c0dc38dfbccf606241917089d`

Full prompts, responses, per-case results, private identities, events,
aggregates, controls, and detailed comparison evidence remain in
permission-restricted operator custody. This report contains only public case
identities and scrubbed held-out aggregates.
