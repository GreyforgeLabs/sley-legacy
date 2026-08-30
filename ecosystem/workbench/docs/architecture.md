# Architecture

`sley-workbench` is a thin ecosystem tool around Loom.

## Boundary

- Sley compiler: owns parsing, checking, graph construction, linting,
  planning, fixing, verification, tracing, seals, and ZJX envelopes.
- This repository: owns developer ergonomics around `graph and graft workbench`.

## Non-Goals

- No duplicate Sley semantic checker.
- No real deployment, provider, spend, wallet, or secret integration.
- No public release assumption without contract-and-release criteria review.

## Stabilization Strategy

Start with read-only or dry-run surfaces. Add write support only when the
Sley compiler exposes a checked operation and a traceable write path.
