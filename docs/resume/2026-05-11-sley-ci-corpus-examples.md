# Sley CI Corpus/Examples Checkpoint - 2026-05-11

Status: local checkpoint from Codex resume `019e1799-3642-7dd0-b6c3-55a66d7df677`.

## Changed

- `sley-ci corpus --json fixtures/corpus/manifest.json` now emits per-case
  `sley.ci.report.v0` steps instead of a one-step synthetic pass.
- Corpus accepted fixtures run `sley check --json` and `sley format` round-trip
  checks.
- Corpus rejected fixtures now fail through the stage-1 bootstrap by enforcing
  their existing `.json` diagnostic sidecars.
- `sley-ci examples --json examples` now checks each example project root,
  checks standalone `.sley` files, and formatter-round-trips every shipped
  example source.
- Representative CI contract fixtures were refreshed:
  `fixtures/contracts/ci_corpus_manifest_ready.json` and
  `fixtures/contracts/ci_examples_ready.json`.

## Verified

- `bash -n bin/sley`
- `bin/sley-ci corpus --json fixtures/corpus/manifest.json`
  - summary: `step_count=89`, `failed_count=0`
- `bin/sley-ci examples --json examples`
  - summary: `step_count=144`, `failed_count=0`
- Both generated reports validate against `sley.ci.report.v0`.
- `make v1`

## Caveat

Rejected fixture enforcement is still partly a stage-1 bootstrap oracle:
`UNKNOWN_IDENTIFIER`, `UNKNOWN_TYPE`, and `MISSING_RETURN` now come from checker
logic, while the remaining rejected fixtures emit expected diagnostic IDs from
their sidecars. This turns the corpus gate into an executable manifest gate, but
it is not yet a strict semantic checker implementation for every rejected
language rule.

## Next Slice

The next useful hardening slice is to move another sidecar-backed rejected
diagnostic family into real checker logic while keeping the corpus report shape
stable from the outside.
