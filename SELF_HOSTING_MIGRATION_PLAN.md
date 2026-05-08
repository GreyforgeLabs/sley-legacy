# Sley Self-Hosted Migration Plan (Objective #2)

## Current state snapshot (as of 2026-05-08)

- Rust source files: 50 (`*.rs`)
- C source files: 1 (`*.c`)
- C headers: 3 (`*.h`)
- No current Swift/C++/Python core files outside vendored toolchain artifacts.

## Required target state

- Zero foreign-language source files in `/home/greyforge/sley`:
  - no `*.rs`, `*.c`, `*.h`, `*.cpp`, `*.swift`, `*.m`, etc.
- CLI, parser, checker, planner, runtime, lint, verify, and tool binaries remain provided by the same stack but implemented in the self-hosted stack language runtime.
- Marketing/public contract claims continue to stay valid while private internals are migrated.

## Immediate blocker discovered

The repository currently contains an active Rust implementation with full compiler/runtime/tooling surface, plus Tree-sitter C artifacts under `tree-sitter-sley/src/tree_sitter`. This is incompatible with the current objective wording (“no Rust code or any other foreign code in repo”).

## Migration guardrails

Use:

```bash
./scripts/check-self-hosted-code.sh
```

This script performs a repo-wide foreign-language file scan and fails if any extension outside the allow-list is present.

## Phased migration (no accidental partial migrations)

1. **Inventory + parity map**
   - Map every command surface from `llms.txt` and `Makefile` to target runtime equivalents.
   - Classify all required parser, AST, checker, planner, runtime host adapters, and reporting formats.
2. **Parser and AST rewrite (stack-native)**
   - Replace Rust parser pipeline with grammar + parser runtime in the chosen self-hosted stack.
   - Keep report schema versions unchanged.
3. **Static analysis + tooling parity**
   - Recreate checker/lint/format/doctor/verify behavior in self-hosted toolchain.
   - Keep command semantics for public-facing options stable.
4. **Runtime host adapters + execution engine**
   - Rebuild deterministic host adapters for secret/db/http/model/network/shell/deploy/spend.
   - Preserve `--cap` behavior and error shape.
5. **Migration validation**
   - Add golden tests for major commands against existing `.sley` fixtures.
   - Keep schema IDs and report structure unchanged.
6. **Cutover + cleanup**
   - Remove legacy Rust/C artifacts only when parity stage complete.
   - Re-run full external integration checks before public release promotion.

## Progress checklist

- [ ] Phase 1 parity map created
- [ ] Self-hosted parser/AST bootstrapped
- [ ] CLI command compatibility achieved
- [ ] Runtime host adapter parity achieved
- [ ] Tooling and schema parity validated
- [ ] Rust/C foreign files fully removed
- [ ] `./scripts/check-self-hosted-code.sh` exits cleanly
- [ ] Website and OpenForge claims updated once migration is stable
