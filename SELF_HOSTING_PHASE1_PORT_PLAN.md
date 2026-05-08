# Sley Self-Hosted Port Plan (Phase 1)

## Objective
Complete the first self-hosted implementation wave so command surfaces remain
stable while removing forbidden foreign-language implementation files.

## Current blocker baseline
- `./scripts/check-self-hosted-code.sh` currently passes.
- `make v1` is the current stage-1 proof gate.
- `bin/sley self-hosting-status --json` reports the stage-2 source ownership
  boundary.
- Foreign-language artifacts count is generated in `SELF_HOSTING_INVENTORY_REPORT.md`.
- `tree-sitter` parser C artifacts have been removed from git-tracked sources in this phase.
- Latest blocker report now shows forbidden implementation extensions at zero.

## Phase 1 target (go/no-go)
- Keep marketing/claim surfaces truthful and prior-art-safe.
- Deliver a POSIX shell stage-1 bootstrap for executable paths while Sley-owned
  semantic modules are ported.
- Maintain command parity for `sley --help`, `sley --version`, `sley doctor`,
  `sley ast`, `sley check`, `sley query`, `sley lint`, `sley run`, and
  `sley verify` with JSON schema-shape-compatible output.
- No forbidden foreign-language implementation files remain.
- Milestone reached: stage-1 command/runtime envelopes are runnable, and
  stage-2 Sley source modules now own bootstrap metadata and lint-rule
  inventory. Remaining migration work is strict Sley-written semantic parity.

## Command parity map (from `llms.txt`, prioritized)
1. `sley check --json <file-or-project>`
2. `sley ast --json <file-or-project>`
3. `sley ast --json --node <node-id> <file-or-project>`
4. `sley graph --json <file-or-project>`
5. `sley graph --json --slice <node-id> <file-or-project>`
6. `sley query --json --kind calls <file-or-project>`
7. `sley lint --json <file-or-project>`
8. `sley plan --json --graft-templates <file-or-project>`
9. `sley new --json --template agent-project --name agent-app --module agent.main agent-app`
10. `sley doctor --json <file-or-project>`

## Delivery units for Phase 1
- Unit A: Parser/AST pipeline port
  - Stage-1 AST command envelope exists in `bin/sley`.
  - Next: replace stage-1 shell parsing with Sley-owned parser semantics.
- Unit B: Diagnostics + validation
  - Implement minimal schema-backed checks from `docs/schemas/*`.
- Unit C: CLI command dispatch
  - Implement argument parser + JSON envelope shape for baseline commands.
- Unit D: Runtime host adapter stubs (deterministic)
  - Seeded outputs for `Database*`, `Secret*`, `Deploy*`, `Spend*`, `Network*`, `Shell*`, `Model*`.

## Exit criteria for this phase
- `./scripts/check-self-hosted-code.sh` exits cleanly.
- `sley doctor --json` and `sley ast --json` return structured responses on a small public fixture set.
- CLI metadata and docs remain aligned with new behavior.
- `make v1` passes without Cargo, Rust, Node, npm, or tree-sitter.
- `bin/sley self-hosting-status --json` reports `strict_self_hosted: false`
  until execution from Sley source is complete.

## Enforcement
- No foreign-language files: keep this as the release-blocking invariant before advancing.
- Run `./scripts/self-hosting-inventory.sh` at the start/end of every migration cycle.
