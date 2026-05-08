# Sley Self-Hosted Migration Plan

## Current state snapshot (as of 2026-05-08)

- Rust source files: 0 (`*.rs`)
- C source files: 0 (`*.c`)
- C headers: 0 (`*.h`)
- JavaScript, TypeScript, and Python implementation files: 0.
- Runnable stage-1 CLI/tooling bootstrap: `bin/sley` plus companion wrappers.
- Sley-owned stage-2 semantic source modules: `self-hosted/src/loom/*.sley`.
- Bootstrap values now read from Sley source:
  - `implementation_version`
  - `implementation_stage`
  - `self_hosting_status`
  - `strict_self_hosted`
  - `source_modules`
  - the `self-hosting-status` ownership list
  - `default_lint_rules`
  - core report schema IDs
  - diagnostic and lint finding IDs
  - runtime seed values
  - parser expression classifier names and patterns
  - parser statement and binding kind names
  - self-hosted bootstrap smoke runtime value
  - `loom.bootstrap.smoke` source task execution for the self-hosted smoke
  - checker diagnostic statuses and unknown-identifier message construction
  - `loom.checker.diagnostic_status` source task execution for checker report status
  - `loom.lint.lint_status` source task execution for lint report status
  - lint finding statuses, messages, and hints
  - `loom.runtime.runtime_status` source task execution for run report status
  - pure `main` literal return execution through the AST-backed runtime path
  - runtime report statuses, value-kind tags, and current dispatch probes
- Current local proof gate: `make v1`.

## Required target state

- Zero forbidden foreign-language source files in this repo:
  - no Rust, C/C++, Swift, Go, Java/Kotlin, C#, Ruby, PHP, Python,
    JavaScript, or TypeScript implementation files.
- CLI, parser, checker, planner, runtime, lint, verify, and companion tools
  keep runnable local JSON command envelopes throughout the migration.
- Strict self-hosting is complete only when parser/checker/runtime semantics
  are implemented in Sley source, with the POSIX shell surface reduced to a
  bootstrap loader and test harness.
- Marketing/public contract claims continue to stay valid while private internals are migrated.

## Current blocker

The active Rust implementation and Tree-sitter C artifacts are removed from
tracked sources. Stage-1 command envelopes are runnable and stage-2 Sley source
modules now own initial bootstrap metadata, report IDs, diagnostics, runtime
seed values, and rule inventory, but strict
compiler-written-in-Sley parity is not complete.

## Migration guardrails

Use:

```bash
./scripts/check-self-hosted-code.sh
```

This script performs a repo-wide foreign-language source scan and fails if any
forbidden extension is present.

## Phased migration (no accidental partial migrations)

1. **Inventory + parity map**
   - Map every command surface from `llms.txt` and `Makefile` to target runtime equivalents.
   - Classify all required parser, AST, checker, planner, runtime host adapters, and reporting formats.
2. **Parser and AST rewrite**
   - Replace the Rust parser pipeline with Sley-owned parser/checker modules.
   - Keep report schema versions unchanged.
3. **Static analysis + tooling parity**
   - Recreate checker/lint/format/doctor/verify behavior in Sley source.
   - Keep command semantics for public-facing options stable.
4. **Runtime host adapters + execution engine**
   - Rebuild deterministic host adapters for secret/db/http/model/network/shell/deploy/spend.
   - Preserve `--cap` behavior and error shape.
5. **Migration validation**
   - Add golden tests for major commands against existing `.sley` fixtures.
   - Keep schema IDs and report structure unchanged.
6. **Cutover + cleanup**
   - Keep forbidden foreign-language implementation files absent.
   - Reduce shell bootstrap scope once Sley-owned semantics can drive the
     command surface directly.
   - Re-run full external integration checks before public release promotion.

## Progress checklist

- [x] Phase 1 parity map created
- [x] Stage-1 parser/AST command envelope bootstrapped
- [x] Stage-1 CLI command compatibility achieved
- [x] Stage-1 deterministic runtime envelope bootstrapped
- [x] Stage-1 tooling and schema smoke validation added
- [x] Stage-2 Sley-owned semantic source modules added
- [x] Bootstrap version/stage/status fields, source-module list, status ownership list, report schema IDs, diagnostics, runtime seeds, parser classifiers and AST kind names, checker diagnostics/status task execution, lint finding/status task execution/text, runtime report dispatch/status task execution, pure literal runtime execution, lint-rule inventory, and bootstrap smoke value/task execution read from Sley source
- [x] Rust/C/JS/TS/Python foreign implementation files fully removed
- [x] `./scripts/check-self-hosted-code.sh` exits cleanly
- [x] README and llms command surfaces updated away from Cargo/Node claims
- [ ] Strict Sley-written parser/checker/runtime parity achieved
- [ ] Website and OpenForge claims updated once strict migration is stable
