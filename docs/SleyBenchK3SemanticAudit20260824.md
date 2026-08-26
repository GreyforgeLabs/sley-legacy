# SleyBench K3 Semantic-Oracle Audit

Status: completed local evidence
Date: 2026-08-24
Suite: SleyBench v0, 96 public and 24 private
Model: `gpt-5.6-sol`, low thinking
Owner: Sley maintainers

## Decision

The 11 K3 v2 cases that passed `sley check` but failed their semantic oracle do
not represent one model-reasoning defect. Seven are Sley-specific expression
syntax failures. Four are underspecified task/oracle contracts whose prompts
require an exact numeric literal without supplying that value in the prompt or
workspace. No case demonstrates a standard-library misunderstanding, and no
case provides enough evidence to assign ordinary algorithmic reasoning as its
primary cause.

The smallest general correction was `SLEY_AI.md` bootstrap version 0.2. It now
states the executable Boolean spellings (`&&`, `||`, and `!`), rejects the
foreign words `and`, `or`, `not`, and `then`, shows the current single-line
expression conditional and multiline statement alternative, and warns that a
clean stage-1 check may still contain a non-executable `Raw` expression.
Language semantics, compiler behavior, benchmark cases, oracles, evaluator,
controller, corpus, and training state did not change.

One controlled K3 replay under otherwise identical v2 conditions fixed five
of the seven targeted Sley-syntax cases. Strict success rose from 102/120 to
109/120 and oracle success from 103/120 to 111/120, but paired exact tests did
not cross the conventional 0.05 threshold. Keep the documentation correction
as directly accurate operating guidance; do not present this single stochastic
trial as a general model-capability result.

## Captured-Evidence Taxonomy

The classification used retained v2 prompts, responses, submitted sources,
compiler reports, and independent oracle results before any correction was
made. Private cases were inspected in operator custody and are reported only
as redacted counts.

| Primary root cause | Public | Private | Total |
| --- | ---: | ---: | ---: |
| Misunderstood Sley expression syntax/semantics | 6 | 1 | 7 |
| Other: underspecified task/oracle exact literal | 3 | 1 | 4 |
| Incorrect ordinary algorithm/reasoning | 0 | 0 | 0 |
| Standard-library or API misunderstanding | 0 | 0 | 0 |
| **Total** | **9** | **2** | **11** |

The seven Sley-specific failures comprise one semantic-implementation case and
six translation-preservation cases. They used a foreign Boolean word or
`then`, or formatted the current expression conditional over multiple lines.
All seven passed `sley check`; their AST evidence contained a full or partial
`Raw` fallback and execution failed as unsupported source.

The four diagnosis/repair cases are different. Each prompt names a required or
intended exact literal but never states its value. The trusted candidate and
oracle contain one unstated number; the model selected another. Those are
benchmark-contract defects, not demonstrated failures of ordinary reasoning.
The fact that the evaluator labels them semantic-oracle failures describes the
stage that rejected the answer, not the cause of the answer.

## Public Case Map

| Case | Primary cause | K3 v2 | Bootstrap 0.2 replay |
| --- | --- | ---: | ---: |
| `sleybench-v0-f2-009` | Foreign Boolean operator spelling | Fail | Pass |
| `sleybench-v0-f3-004` | Unstated exact literal | Fail | Fail |
| `sleybench-v0-f3-007` | Unstated exact literal | Fail | Fail |
| `sleybench-v0-f3-010` | Unstated exact literal | Fail | Pass |
| `sleybench-v0-f4-003` | Foreign Boolean operator spelling | Fail | Pass |
| `sleybench-v0-f4-005` | Foreign conditional spelling | Fail | Fail |
| `sleybench-v0-f4-008` | Foreign Boolean operator spelling | Fail | Pass |
| `sleybench-v0-f4-011` | Foreign Boolean operator spelling | Fail | Pass |
| `sleybench-v0-f4-012` | Multiline expression conditional | Fail | Pass |

The held-out results remain redacted: the one Sley-syntax case stayed failed,
and the one underspecified-literal case stayed failed. Across all 11 original
cases, five of seven syntax-rooted cases and one of four underspecified cases
passed. The latter is not evidence that the documentation supplied the hidden
value; it is a stochastic match to an unstated oracle choice.

## Recurring Patterns And Ownership

Two system weaknesses overlapped on all seven Sley-specific cases:

1. Bootstrap 0.1 did not enumerate the small executable expression vocabulary
   needed to translate common Python, TypeScript, and pseudocode forms.
2. The stage-1 parser/checker accepts unsupported expression text through a
   `Raw` fallback, while the runtime fails closed. Consequently, `sley check`
   provided false reassurance and no actionable syntax diagnostic.

The first is a compact-documentation deficiency and is corrected. The second
is a compiler diagnostic deficiency and remains open. Fixing it would require
parser/checker work and broader compiler validation, so it is outside this
documentation-only goal.

The four exact-literal failures are ordinary evaluation-design weaknesses,
not Sley language weaknesses. Correcting them would mutate benchmark cases or
oracles and could affect held-out governance, so they are catalogued rather
than changed.

## Controlled Replay

The replay used the same 120 cases, public/private split, `openclaw-one-shot-v2`
adapter, system prompt, model, thinking level, retrieval document, compiler
tools, evaluator, scoring, ownership, ceilings, four-worker setting, privacy,
retention, and training controls as K3 v2. Normalizing the run ID and bootstrap
digest makes both partition controls identical. The bootstrap digest is the
only substantive input change.

| Measure | K3 v2 | Bootstrap 0.2 | Delta |
| --- | ---: | ---: | ---: |
| Strict case pass | 102/120 | 109/120 | +7 |
| Public strict pass | 81/96 | 89/96 | +8 |
| Private strict pass | 21/24 | 20/24 | -1 |
| Oracle `pass@1` | 103/120 | 111/120 | +8 |
| `compile@1` | 100/100 | 98/100 | -2 |
| Repository patch acceptance | 18/20 | 18/20 | 0 |
| Syntax hallucination | 0/120 | 2/120 | +2 |
| Invalid tool calls | 1/120 | 4/120 | +3 |
| Valid tool calls | 83 | 55 | -28 |
| Input tokens | 582,018 | 530,541 | -51,477 |
| Output tokens | 13,630 | 11,610 | -2,020 |
| API-equivalent cost | $1.300336 | $1.177182 | -$0.123154 |

Paired strict transitions were 11 fail-to-pass and four pass-to-fail (exact
two-sided McNemar p=0.11846923828125). Oracle transitions were 11
fail-to-pass and three pass-to-fail (p=0.057373046875). Compile transitions
were zero fail-to-pass and two pass-to-fail (p=0.5). Repository acceptance had
one transition in each direction (p=1.0). These stochastic regressions remain
in the denominator; no response or case was discarded, repaired, or rerun.

## Future Problem Ledger Candidates

- Add an actionable checker diagnostic when an executable expression remains
  `Raw`, especially for foreign operators and keywords. Keep runtime fail-closed.
- Decide whether multiline expression conditionals should be supported or
  specifically rejected with a formatting/grammar diagnostic. This is a
  language/compiler decision, not part of the present correction.
- Review the four exact-literal task/oracle contracts through the benchmark's
  held-out-safe case governance. Do not infer or disclose private values.
- Register and test the combined mode-summary schema. The two partition
  aggregates validate against `sley.agent_bench.aggregate.v0`, but the retained
  summary producer emits `greyforge.sleybench.mode_summary.v0` and the public
  repository currently has no matching registered schema.

No further K3 replay, training, corpus admission, case/oracle edit, language
change, deployment, publication, merge, or push is justified by this audit.

## Evidence Pins

- Bootstrap 0.2: `sha256:fd048d403b43ff05c28da4b68ac7831f2b227506d168637d4a5c1757c9ae2fcd`
- Public control: `sha256:799d49103452874d3650374b29d2b59dca88a95d633b5af56d4da91dfd501728`
- Private control: `sha256:90f49d9772d5fd120b93d6131ed662eff6934113043e0e3f47837c89e54b369e`
- Public aggregate: `sha256:180bfcbc3e2c96e372e32212526e861c441ac9313ea04b2e52616d42de9265f2`
- Private aggregate: `sha256:04d731641a960648575429ad77fddfa0dc668fe450d91f11c1feb89dee35d507`
- Combined summary: `sha256:77df2308dcf4d27831a773c1bf32572870a7908071a0a6bc9938274b3535621f`

Full prompts, responses, per-case results, private identities, events,
aggregates, controls, and the detailed classification remain in
permission-restricted operator custody. This report contains only public case
identities and scrubbed held-out aggregates.
