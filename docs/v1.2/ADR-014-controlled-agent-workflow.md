# ADR-014: Controlled agent structural workflow

Status: accepted for S12-602 infrastructure
Date: 2026-08-26
Owner: Sley maintainers

## Decision

S12-602 compares one public cross-module Sley rename under two isolated
interfaces:

- K1 receives the complete raw workspace and no compiler tool turn.
- K3 receives the same raw workspace and may use exactly one read-only
  `sley query --json --kind tasks .` turn before final submission.

This is a raw-file incumbent workflow versus a Sley structural augmentation.
It is not a comparison between two programming languages and does not claim
that the structural response replaces raw source. The current controller sends
the raw workspace in both arms, so structural context may expand total context.
That result is measured and retained rather than assumed to be a saving.

## Equal controls

Both arms pin the same:

- model and thinking level;
- public prompt, workspace, owned paths, and trusted candidate;
- compiler bootstrap and empty retrieval set;
- strict oracle and minimality boundary;
- model context window through the same model;
- spend ceiling;
- private one-shot controller digest.

The controlled interface difference is zero tool turns and one inference turn
for K1 versus one read-only structural turn and at most two inference turns for
K3. The action budget is therefore a declared controlled difference, not an
equal control. Observed input/output tokens, prompt/response bytes, structural
response bytes, wall time, tool calls, compiler cycles, invalid actions, and
accepted change tokens are reported. The equal spend ceiling bounds both
attempts even when the structural arm consumes a second inference turn.

The case manifest retains wall, output, process, tool, and workspace limits,
but the unchanged private runner does not generically enforce all of them.
They are therefore declared inputs rather than verified equal controls. This
partial-enforcement boundary is emitted in both the plan and comparison.

## Independent oracle

The public case is a two-file rename of
`agent.pipeline.draft_plan` to `agent.pipeline.compose_plan`. Acceptance
requires all of the following:

1. the candidate project passes `sley check`;
2. the exact `agent.pipeline` task inventory contains the renamed task with
   the original take, return type, effects, and call counts and contains no old
   task;
3. the exact `agent.main` call inventory resolves the renamed call site and
   contains no old call target;
4. a locked effect-mocked `sley test` preserves the prior result;
5. only the two owned files change and the total diff is at most four changed
   lines.

The private runner's informational `candidate_digest` is not treated as the
oracle. Acceptance is derived from the independent compiler, structural, and
behavioral checks above. The digest remains an exact provenance pin: it is
computed from the complete baseline workspace with the candidate's two owned
files overlaid, matching the private runner's materialized final-workspace
identity. A digest of the changed-file overlay alone is not a valid final-tree
identity.

## Evidence and disclosure

`sley operational-workflow plan --controller PATH --json` validates all pins
and emits a no-call approval packet. It always reports provider execution as
unauthorized and approval required.

After exact operator approval and isolated K1/K3 execution,
`sley operational-workflow compare` requires both aggregate/evidence pairs and
a private approval record whose exact plan identities, model, spend ceiling,
and denied actions are checked before assembly. The public comparison contains
only measurements and content digests. Raw prompts, responses, tool reports,
and approval text remain in permission-restricted operator custody.

Comparison also requires a private execution-provenance record binding the
approval, controller, K1/K3 aggregates, evidence-bundle digests, inference
attempt counts, provider/model assertion, and an independent evidence review
with no High or Medium finding. This is assertion-based rather than a
cryptographic provider attestation, so that limitation remains in every
comparison and later registry.

K3 is accepted only when exactly one structural query was exercised. Failure
to use it remains a failed attempt in the denominator. A failed attempt has no
finite per-arm ACT value, but its observed tokens remain recorded. The
one-shot controller has no post-submit repair path, so repair loops are
reported as zero with that observability limitation stated explicitly.
When K3 reports a query, its second prompt must contain the matching bounded
task-query action and a `sley.query.report.v0`; missing or contradictory private
evidence fails closed. The comparison classifies structural prompt context as
a saving, equality, or expansion. Hardware fields describe the comparison host
only.

## Authority and claims

Infrastructure implementation and deterministic synthetic validation do not
authorize provider execution or spend. The exact two-arm run requires fresh
operator approval. No result from one public case and one trial supports a
general model-fluency, structural-advantage, production-adoption, or promotion
claim. Production, deploy, publication, and repository mutation authority are
always false in the comparison packet, and the promotion decision remains
deferred.
