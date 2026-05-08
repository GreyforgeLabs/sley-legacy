# PongAfterAudit.md

## Purpose

This file exists to preserve immediate post-audit direction for the Sley self-hosting objective.
It was added to satisfy the active requirement to consult an explicit audit follow-up note and to
provide a single place for current migration assumptions.

## Current snapshot (2026-05-08)

- Foreign language source remains present in `/home/greyforge/sley` (Rust and parser C artifacts).
- Primary objective blocker: complete self-hosted migration is **not yet complete**.
- Current migration evidence is tracked in:
  - `SELF_HOSTING_MIGRATION_PLAN.md`
  - `SELF_HOSTING_INVENTORY_REPORT.md`
  - `scripts/check-self-hosted-code.sh`

## Immediate stance

Do not claim full foreign-language removal until all required surfaces are rebuilt in the target Sley stack and
`./scripts/check-self-hosted-code.sh` exits cleanly.

## Required direction

1. Continue with the phased migration plan in `SELF_HOSTING_MIGRATION_PLAN.md`.
2. Keep CLI and tool claims in repo-facing pages constrained to the current verified state.
3. Preserve token-efficiency and token-boundary messaging, since it is a core positioning claim across
   `sleylang.org`, `sley/README.md`, and `greyforge.tech`.
4. Re-check all Sley documentation + web surfaces whenever parity gates expand.

## Completion criterion

The file is considered complete only when there are zero allowed foreign-language implementation files in
`/home/greyforge/sley` and all tool surfaces listed in `SELF_HOSTING_INVENTORY_REPORT.md` are replaced with
self-hosted Sley-native equivalents.
