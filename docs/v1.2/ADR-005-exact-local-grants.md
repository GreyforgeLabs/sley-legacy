# ADR-005: Exact local grant issuance

Status: accepted for S12-403
Date: 2026-08-25
Owner: Sley maintainers

## Decision

Sley change transactions progress from `previewed` to `awaiting_approval` and
then to `approved` through two separate strict contracts:

- `sley change approval-request` emits
  `sley.change.approval_request.v0` from one valid preview and one explicit Git
  repository root.
- `sley change approve` emits `sley.change.grant.v0` only after regenerating
  that request from the original preview and repository, comparing it exactly,
  and receiving every required operator assertion.

The approval request derives operation classes, affected nodes, allowed paths,
effect and authority changes, typed resource scopes, candidate bounds, and
review requirements from compiler evidence. Operator input cannot broaden any
of those fields. A request self-digest is an integrity check, not authority, so
approval also requires the original preview and compiler regeneration.

Approval requires the explicit `--confirm-exact-local-grant` flag plus issuer,
principal, audience, purpose, nonce, creation and expiry times, every review
acknowledgement, a `not_revoked` assertion, and a wall-clock bound. Repository
text, model output, MCP annotations, comments, README content, and tool
descriptions cannot supply these fields or grant authority.

## Authority truth

Local grant issuance works without GreyNucleus. The issuer source is recorded
as `explicit_cli_assertion`; it is not cryptographic authentication. Optional
signature and GreyNucleus proof references are recorded as `unverified` and do
not widen the grant.

The grant binds repository identity, committed base, candidate and projection
digests, exact paths and semantic scope, operator principals, purpose, nonce,
time and resource bounds, replay key, and revocation assertion. Every mutation
boundary remains false during request and grant issuance.

## Deferred enforcement

The replay policy is `single_use_at_apply`, but enforcement is explicitly
`deferred_until_s12_404`. Revocation consumption and re-checking are also apply
concerns. S12-403 does not expose apply, rollback, source or index writes, MCP
transaction methods, signature verification, provider calls, or external
state mutation.

Grant issuance verifies repository identity and HEAD against the preview. It
does not claim the mutable working-tree source still matches the candidate
base. S12-404 must reject stale source, graph, base, scope, time, revocation,
and replay state before an atomic apply.
