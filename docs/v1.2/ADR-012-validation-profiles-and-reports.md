# ADR-012: machine-readable validation profiles and reports

Status: accepted
Date: 2026-08-25

## Decision

Sley 1.2 exposes repository-local validation through four named profiles:
`quick`, `changed`, `core`, and `release`. `loom.validation` owns their names,
check inventory, profile membership, release authority, cache policy, and the
stable `sley.validation.report.v1` identity.

The existing Makefile remains the execution authority. `quick`, `core`, and
`release` invoke `make quick`, `make core`, and `make v1`. `changed` reuses
`scripts/check-changed.sh`; its `--plan-json` output is the sole machine path
classifier. The CLI does not reimplement Make prerequisites or path rules.

Every report records detected changes and subsystems, selected checks and
subsystems, every skipped check with a reason, release-gate requirement and
execution, cache truth, aggregate wall time, outcome, and bounded step
evidence. Executor output is represented by its byte count and SHA-256 digest,
not embedded as an unbounded log.

`quick`, `changed`, and `core` never claim release readiness. Only the
`release` profile executes the authoritative `make v1` gate. Validation-result
caching is disabled. Source-task caching is reported separately and is limited
to loading Sley-owned contract vocabulary.

## Boundaries

- The v1 command accepts only the Sley repository root.
- Changed paths must be relative and confined to that root.
- No profile grants repository mutation, provider, network, deploy, or spend
  authority.
- Narrow changed validation does not select Arena unless an Arena-owned path
  changed.
- A failed executor still emits a schema-valid report and returns the executor
  exit status.

## Consequences

Profile selection and skip reasoning are now inspectable by tools. The stable
report does not promise per-check timing because Make executes each profile as
an aggregate. The existing human `make check-changed` plan remains available,
and rollback can remove the CLI/report layer without changing Make semantics.
