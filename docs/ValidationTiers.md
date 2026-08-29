# Sley validation tiers

Sley validation cost scales with the blast radius of a change. Targeted
validation proves the affected surface. It does not claim that the complete V1
release gate ran.

The same tiers are first-class machine-readable product profiles:

```bash
sley validate --profile quick .
sley validate --changed .
sley validate --changed --explain .
sley validate --profile core .
sley validate --profile release .
```

Each command emits `sley.validation.report.v1`. Reports identify detected
changes, selected subsystems and checks, every skipped check and its reason,
release-gate requirement and execution, cache use, wall time, bounded execution
evidence, and outcome. `--explain` adds a reason for each selected check.
Validation-result caching is disabled. Source-task cache use is reported
separately and does not stand in for executed validation.

## Tier 1: inner loop

Use one of these during routine implementation:

```bash
make quick
make check-changed
```

`make quick` runs shell syntax checks, `git diff --check`, the self-hosting
source-purity check, validation-dispatcher tests, focused parser, checker,
runtime, and lint smokes, and the focused CLI smoke suite. Its performance
budget is under 60 seconds preferred and under two minutes maximum. Record the
measured time when the validation architecture changes.

Measured on the Greyforge development host on 2026-08-22: 50.55 seconds.

`make check-changed` inspects the upstream branch diff when available, staged
and unstaged changes, and untracked files. It prints the changed files,
subsystems, selected Make targets, and the reason the full gate was skipped or
deferred. It unions checks when several subsystems changed. Use `--plan` to
inspect without executing and `--files` to test a proposed change set:

```bash
scripts/check-changed.sh --plan --files \
  self-hosted/src/loom/parser.sley \
  self-hosted/src/loom/checker.sley
```

The dispatcher never invokes `make v1`. Its `--plan-json` mode is the machine
classifier consumed by `sley validate --changed`; the CLI does not maintain a
second path map.

## Tier 2: subsystem and core

After a meaningful core unit or parser/checker/runtime interaction is complete,
run:

```bash
make core
```

`make core` composes the focused parser, checker, runtime, and lint smokes with
contracts, conformance, CLI smoke, and source-purity validation. Run `corpus` or
`examples` separately when those manifests, fixtures, or behaviors changed;
they are not automatic core dependencies.

Measured on the Greyforge development host on 2026-08-22: 78.91 seconds.

For narrower tool changes, run the selected target printed by
`make check-changed`, such as `contracts`, `lsp`, `migrate`, `docgen`,
`sandbox-runner`, `shadow`, `zjx-tools`, `arena`, `agent-bench`, or
`corpus-governance`. CLI launcher or `lib/sley/` changes use `cli-modules`,
which includes clean-package golden behavior. SleyBench evaluator and split-
governance changes both use `agent-bench`. Do not add unrelated tools merely
to make the report look comprehensive.

Run the broader `make test` self-hosted integration suite when a completed core
change needs evidence beyond the focused parser, checker, runtime, and lint
smokes. It is not part of the default inner loop or the bounded `make core`
composition because its runtime is materially higher.

Before handing implementation to another agent, report the affected subsystem,
the Tier 1 or Tier 2 checks that ran, and any plausible impact not covered.

## Tier 3: authoritative V1 gate

```bash
make v1
```

`make v1` remains intact as the complete Sley V1 integration and release gate.
Run it locally only for an explicit operator request, release or public release
candidate, an integration boundary not equivalently covered by CI, a change to
the gate itself, a broad cross-subsystem invariant change, or concrete evidence
that targeted validation is insufficient. Prefer CI when it already supplies
the same evidence.

Measured on the Greyforge development host on 2026-08-22: 2,444.19 seconds
(40 minutes 44 seconds).

The GitHub `Sley v1 Gate` workflow runs `make v1` for pull requests and pushes
to `main`. That is the normal authoritative integration evidence. Do not repeat
the same expensive run locally after ordinary edits.

Only `sley validate --profile release .` invokes this gate. Quick, changed, and
core reports may state that the release gate is required, but they never claim
that it ran or that the repository is release-ready.

## Cross-repository Sley and Siglum work

Use the two change-aware dispatchers plus the focused Siglum Sley integration
tests:

```bash
make check-changed
npm --prefix ../AstroForge run validate:changed
npm --prefix ../AstroForge run test:sley-integration
```

The first two commands select each repository's affected surfaces. Run the
third only when the Sley-to-Siglum protocol, package, ruleset, adapter, or
runtime boundary changed. Full Sley and Siglum gates remain Tier 3 decisions.

## Completion report

Use this distinction explicitly:

```text
Validation tier: Tier 1 / targeted
Affected subsystems: parser, checker
Checks: syntax, self-hosted tests, smoke
Result: all passed
Full Sley V1 gate: not run - not required for this development iteration
```

If the full gate is required but deliberately left to CI, say that instead of
claiming complete local release validation.
