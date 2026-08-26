# Sley agent guidance

## Validation economy

Use the smallest validation scope that gives meaningful evidence for the
change. During routine implementation, use `make quick` or
`make check-changed`. After a meaningful parser, checker, runtime, contract, or
tooling unit, use the relevant Tier 2 target or `make core`.

`make v1` is the authoritative integration and release certification gate, not
an inner-loop command. Do not run it merely because Sley changed. Run it only
at a justified Tier 3 boundary, and prefer the existing pull-request or `main`
CI gate when it provides equivalent evidence.

Report the tier, affected subsystems, checks run, result, and whether the full
gate was skipped, deferred to CI, or explicitly executed. See
`docs/ValidationTiers.md` for the complete policy and command map.
