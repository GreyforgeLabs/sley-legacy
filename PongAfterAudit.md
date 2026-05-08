# PongAfterAudit.md

## Purpose

This file exists to preserve immediate post-audit direction for the Sley self-hosting objective.
It was added to satisfy the active requirement to consult an explicit audit follow-up note and to
provide a single place for current migration assumptions.

## Current snapshot (2026-05-08)

- Foreign-language implementation files are removed from tracked `sley` sources.
- Primary blocker: full command/runtime parity is **not yet complete**.
- Current migration evidence is tracked in:
  - `SELF_HOSTING_MIGRATION_PLAN.md`
  - `SELF_HOSTING_INVENTORY_REPORT.md`
  - `scripts/check-self-hosted-code.sh`
  - `scripts/self-hosted-test.sh`
  - `bin/sley`
  - `self-hosted/src/loom/`

## Immediate stance

Use the migration gates below as the canonical status:
- `./scripts/check-self-hosted-code.sh` currently passes (no tracked foreign extensions found).
- `make v1` is the current local proof gate for stage-1 command/runtime envelopes.
- `bin/sley self-hosting-status --json` is the current source-ownership report;
  it now reports Sley-owned version, rule, report-ID, diagnostic, and runtime
  seed surfaces plus parser expression classifiers, checker diagnostic status,
  lint finding text, runtime dispatch/report vocabulary, and bootstrap smoke
  runtime.
- Remaining migration work is strict Sley-written semantic/runtime execution
  parity, not source-language cleanup.

## Required direction

1. Continue with the phased migration plan in `SELF_HOSTING_MIGRATION_PLAN.md`.
2. Keep CLI and tool claims in repo-facing pages constrained to the current verified state.
3. Preserve token-efficiency and token-boundary messaging, since it is a core positioning claim across
   `sleylang.org`, `sley/README.md`, and `greyforge.tech`.
4. Re-check all Sley documentation + web surfaces whenever parity gates expand.

## Completion criterion

The file is considered complete when:

1. Tracking remains free of forbidden foreign-language sources (`./scripts/check-self-hosted-code.sh` clean).
2. Stage-1 tool surfaces listed in `SELF_HOSTING_INVENTORY_REPORT.md` are runnable through `bin/`.
3. Strict self-hosting is only claimed after the parser, checker, runtime, and command semantics are implemented in Sley source.
4. The marketing claims on `sleylang.org` and GitHub docs match the actual migrated parity surface.
