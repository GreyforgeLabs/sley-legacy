#!/usr/bin/env bash
set -euo pipefail

ROOT_DIR="$(cd "$(dirname "$0")/.." && pwd)"
CONTRACT="$ROOT_DIR/bin/sley-contract"
SCHEMA_DIR="$ROOT_DIR/docs/schemas"
FIXTURE="$ROOT_DIR/fixtures/contracts/sleybench_mode_summary_v0_synthetic.json"
WORK_DIR="$(mktemp -d)"
trap 'rm -rf "$WORK_DIR"' EXIT

fail() {
  echo "SleyBench mode-summary check failed: $*" >&2
  exit 1
}

check_semantics() {
  jq -e '
    (.counts.passed_count + .counts.failed_count == .counts.case_count) and
    (.partitions.public.case_count + .partitions.private.case_count == .counts.case_count) and
    (.partitions.public.passed_count + .partitions.private.passed_count == .counts.passed_count) and
    ([.rates[] | .numerator <= .denominator] | all) and
    ([.rates[]
      | if .denominator == 0 then
          (.applicable == false and .rate == null)
        else
          (.applicable == true and ((.rate - (.numerator / .denominator)) | fabs) < 0.000000001)
        end] | all)
  ' "$1" >/dev/null
}

"$CONTRACT" validate --schema greyforge.sleybench.mode_summary.v0 "$FIXTURE" \
  --schemas "$SCHEMA_DIR" --json >/dev/null
check_semantics "$FIXTURE"

jq '.counts.failed_count = 1' "$FIXTURE" >"$WORK_DIR/inconsistent-counts.json"
if check_semantics "$WORK_DIR/inconsistent-counts.json" 2>/dev/null; then
  fail "inconsistent pass/fail totals were accepted"
fi

jq '.partitions.public.passed_count = 95' "$FIXTURE" >"$WORK_DIR/inconsistent-partitions.json"
if check_semantics "$WORK_DIR/inconsistent-partitions.json" 2>/dev/null; then
  fail "inconsistent partition totals were accepted"
fi

jq '.rates.parse_at_1.rate = 0.5' "$FIXTURE" >"$WORK_DIR/inconsistent-rate.json"
if check_semantics "$WORK_DIR/inconsistent-rate.json" 2>/dev/null; then
  fail "inconsistent rate arithmetic was accepted"
fi

echo "SleyBench mode-summary semantics passed"
