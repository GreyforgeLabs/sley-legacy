# Sley Self-Hosted Port Plan (Phase 1)

## Objective
Complete the first self-hosted implementation wave so command surfaces remain stable while removing all Rust and C implementation files.

## Current blocker baseline
- `./scripts/check-self-hosted-code.sh` currently fails.
- Foreign-language artifacts count is generated in `SELF_HOSTING_INVENTORY_REPORT.md`.
- `tree-sitter` parser C artifacts have been removed from git-tracked sources in this phase.
- Latest blocker report shows `*.c`/`*.h` now at zero while `*.rs` still remain.

## Phase 1 target (go/no-go)
- Keep marketing/claim surfaces unchanged.
- Deliver a TypeScript/Node runtime skeleton for at least one executable path to prove non-Rust migration pattern.
- Maintain command parity for `sley -- help`, `sley -- version`, and `sley doctor` with JSON schema-shape-compatible output.
- No `*.rs`, `*.c`, or `*.h` files remain.
- Milestone reached: `*.c` and `*.h` files are currently removed; remaining migration work is `*.rs`.

## Command parity map (from `llms.txt`, prioritized)
1. `sley -- check --json <file-or-project>`
2. `sley -- ast --json <file-or-project>`
3. `sley -- ast --json --node <node-id> <file-or-project>`
4. `sley -- graph --json <file-or-project>`
5. `sley -- graph --json --slice <node-id> <file-or-project>`
6. `sley -- query --json --kind calls <file-or-project>`
7. `sley -- lint --json <file-or-project>`
8. `sley -- plan --json --graft-templates <file-or-project>`
9. `sley -- new --json --template agent-project --name agent-app --module agent.main agent-app`
10. `sley -- doctor --json <file-or-project>`

## Delivery units for Phase 1
- Unit A: Parser/AST pipeline port
  - Replace Rust parser and AST data model with self-hosted implementation.
  - Introduce JSON AST schema-compatible payloads.
- Unit B: Diagnostics + validation
  - Implement minimal schema-backed checks from `docs/schemas/*`.
- Unit C: CLI command dispatch
  - Implement argument parser + JSON envelope shape for baseline commands.
- Unit D: Runtime host adapter stubs (deterministic)
  - Seeded outputs for `Database*`, `Secret*`, `Deploy*`, `Spend*`, `Network*`, `Shell*`, `Model*`.

## Exit criteria for this phase
- `./scripts/check-self-hosted-code.sh` exits cleanly.
- `sley -- doctor --json` and `sley -- ast --json` return structured responses on a small public fixture set.
- CLI metadata and docs remain aligned with new behavior.

## Enforcement
- No foreign-language files: keep this as the release-blocking invariant before advancing.
- Run `./scripts/self-hosting-inventory.sh` at the start/end of every migration cycle.
