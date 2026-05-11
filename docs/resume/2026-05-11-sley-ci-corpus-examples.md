# Sley CI Corpus/Examples Checkpoint - 2026-05-11

Status: local checkpoint from Codex resume `019e1799-3642-7dd0-b6c3-55a66d7df677`.

## Changed

- `sley-ci corpus --json fixtures/corpus/manifest.json` now emits per-case
  `sley.ci.report.v0` steps instead of a one-step synthetic pass.
- Corpus accepted fixtures run `sley check --json` and `sley format` round-trip
  checks.
- Corpus rejected fixtures run through checker logic when the bootstrap can
  prove the rule; the corpus no longer relies on `.json` diagnostic sidecars
  for rejected fixture IDs.
- Top-level fallible expression statements now enter the AST as parser-owned
  `Expr` statements, and `QUESTION_REQUIRES_RESULT` is enforced by checker
  logic instead of its sidecar.
- Record literals now emit checker-owned diagnostics for missing fields,
  unknown fields, field type mismatches, non-record type literals, and simple
  `value.field` misses.
- Binding and return type-flow fixtures now emit checker-owned `TYPE_MISMATCH`
  and `RETURN_TYPE_MISMATCH` diagnostics for the covered literal, identifier,
  list, and transparent alias shapes.
- The former `sley check` fallback that manufactured rejected fixture
  diagnostics from `.json` sidecars has been removed. Sidecars now serve only
  as `sley-ci corpus` expected-ID metadata.
- `sley verify --json` now reads its top-level report field order from
  `self-hosted/src/loom/reports.sley`, and self-hosting status records
  `verify_report_shape` as Sley-owned bootstrap evidence.
- `sley graph --json` now reads its schema ID and top-level report field order
  from `self-hosted/src/loom/reports.sley`, and self-hosting status records
  `symbol_graph_report_shape` as Sley-owned bootstrap evidence.
- `sley graft --json` now reads its schema ID and top-level report field order
  from `self-hosted/src/loom/reports.sley`, and self-hosting status records
  `graft_outcome_report_shape` as Sley-owned bootstrap evidence.
- `sley claim-verify --json` now reads its schema ID and top-level report field
  order from `self-hosted/src/loom/reports.sley`, and self-hosting status
  records `claim_verify_report_shape` as Sley-owned bootstrap evidence.
- `sley-conformance report --json` now reads its schema ID and top-level report
  field order from `self-hosted/src/loom/reports.sley`, and self-hosting status
  records `conformance_report_shape` as Sley-owned bootstrap evidence.
- `sley-conformance coverage --json` now reads its schema ID and top-level
  report field order from `self-hosted/src/loom/reports.sley`, and
  self-hosting status records `conformance_coverage_report_shape` as Sley-owned
  bootstrap evidence.
- `sley-ci ... --json` reports now read their schema ID and top-level report
  field order from `self-hosted/src/loom/reports.sley`, and self-hosting status
  records `ci_report_shape` as Sley-owned bootstrap evidence.
- `sley-contract inventory|validate|check-fixtures --json` reports now read
  their schema IDs and top-level report field order from
  `self-hosted/src/loom/reports.sley`, and self-hosting status records
  `contract_report_shapes` as Sley-owned bootstrap evidence.
- `sley deploy --json --dry-run` now reads its schema ID and top-level report
  field order from `self-hosted/src/loom/reports.sley`, and self-hosting status
  records `deploy_report_shape` as Sley-owned bootstrap evidence.
- `sley-contract inspect-deploy-artifacts --json` now reads its report schema
  and top-level field order from `self-hosted/src/loom/reports.sley`, validates
  deploy artifact manifests and file digests, and self-hosting status records
  `deploy_artifact_check_report_shape` as Sley-owned bootstrap evidence.
- `sley-migrate report --json` now reads its schema ID and top-level report
  field order from `self-hosted/src/loom/reports.sley`, emits schema-valid
  migration and schema-drift reports for the covered stage-1 cases, and
  self-hosting status records `migrate_report_shape` as Sley-owned bootstrap
  evidence.
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

Rejected fixture enforcement is still a stage-1 bootstrap implementation, but
the rejected corpus no longer uses diagnostic sidecar oracles:
`UNKNOWN_IDENTIFIER`, `UNKNOWN_TYPE`, `UNKNOWN_TASK`, `CALL_ARITY_MISMATCH`,
`CALL_ARGUMENT_TYPE_MISMATCH`, `TYPE_MISMATCH`, `RETURN_TYPE_MISMATCH`,
`DUPLICATE_TAKE`, `DUPLICATE_EFFECT`,
`DUPLICATE_FIELD`, `DUPLICATE_MAP_KEY`, `DUPLICATE_RECORD_LITERAL_FIELD`,
`DUPLICATE_TYPE`, `DUPLICATE_TASK`, `RECORD_FIELD_MISSING`,
`RECORD_FIELD_UNKNOWN`, `RECORD_FIELD_TYPE_MISMATCH`,
`RECORD_LITERAL_NON_RECORD_TYPE`, `UNKNOWN_RECORD_FIELD`, `UNKNOWN_EFFECT`,
`GATE_TAKE_TYPE_MISMATCH`, `GATE_EFFECT_UNDECLARED`, `EFFECT_UNAUTHORIZED`,
`INDEX_NOT_INT`, `INDEX_KEY_TYPE_MISMATCH`, `LIST_ELEMENT_TYPE_MISMATCH`,
`MAP_KEY_TYPE_MISMATCH`, `MAP_VALUE_TYPE_MISMATCH`, `MISSING_RETURN`, and
`QUESTION_REQUIRES_RESULT` now come from checker logic. This turns the corpus
gate into an executable manifest gate, but it is not yet a strict semantic
checker implementation for every rejected language rule.

## Next Slice

The next useful hardening slice is to move checker or report-builder behavior
from shell/JQ execution into executable Sley source while keeping the corpus and
contract report shapes stable from the outside.
