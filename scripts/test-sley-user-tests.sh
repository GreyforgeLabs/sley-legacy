#!/usr/bin/env bash
set -euo pipefail

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
SLEY="$ROOT_DIR/bin/sley"
FIXTURE="$ROOT_DIR/fixtures/user_tests"
WORKDIR="$(mktemp -d)"
trap 'rm -rf "$WORKDIR"' EXIT

fail() {
  echo "Sley user-test conformance failed: $*" >&2
  exit 1
}

full_report="$WORKDIR/full.json"
"$SLEY" test --json "$FIXTURE" > "$full_report"
jq -e '
  .schema == "sley.test.report.v1" and
  .status == "passed" and
  .suite == {"id":"sley-user-test-fixture","version":"1.0.0"} and
  .discovery.convention == "sley.test.json_or_star.sley-test.json" and
  .discovery.ordered_manifests == ["runtime.sley-test.json", "sley.test.json"] and
  .summary == {
    "discovered_case_count":6,
    "selected_case_count":6,
    "execution_count":11,
    "passed_count":11,
    "failed_count":0,
    "skipped_count":0
  } and
  ([.cases[].kind] | unique) == ["adapter_replay","diagnostic","differential","effect_mock","property","table"] and
  .coverage.tasks.method == "entrypoint_execution" and
  .coverage.branches.method == "passing_manifest_witness" and
  .coverage.branches.observed == [] and
  .coverage.branches.uncovered == ["block:task:test.suite.main:stmt:4:else"] and
  .coverage.effects.method == "runtime_authority" and
  .coverage.effects.uncovered == [] and
  .coverage.capabilities.method == "adapter_authority" and
  .coverage.capabilities.uncovered == [] and
  ([.coverage[] | .unknown] | all(length == 0))
' "$full_report" >/dev/null || fail "full suite report lost a case kind, ordering rule, or coverage distinction"
"$ROOT_DIR/bin/sley-contract" validate \
  --schema sley.test.report.v1 "$full_report" --schemas "$ROOT_DIR/docs/schemas" --json \
  | jq -e '.status == "passed"' >/dev/null || fail "full suite report did not validate"

focused_one="$WORKDIR/focused-one.json"
focused_two="$WORKDIR/focused-two.json"
"$SLEY" test --json --changed-node task:test.suite.classify "$FIXTURE" > "$focused_one"
"$SLEY" test --json --changed-node task:test.suite.classify "$FIXTURE" > "$focused_two"
jq -e '
  .status == "passed" and
  .selection.mode == "changed_nodes" and
  .selection.requested_nodes == ["task:test.suite.classify"] and
  .selection.expanded_nodes == ["task:test.suite.classify"] and
  .selection.selected_cases == ["classify-table"] and
  .summary.selected_case_count == 1 and
  .summary.execution_count == 2
' "$focused_one" >/dev/null || fail "changed-node focus selected the wrong evidence"
[[ "$(jq -r '.report_digest' "$focused_one")" == "$(jq -r '.report_digest' "$focused_two")" ]] \
  || fail "identical focused runs produced different report digests"
cmp -s "$focused_one" "$focused_two" || fail "identical focused runs produced different JSON"

uncovered_report="$WORKDIR/uncovered.json"
if "$SLEY" test --json --changed-node task:test.suite.missing "$FIXTURE" > "$uncovered_report" 2>/dev/null; then
  fail "an uncovered changed node unexpectedly passed"
fi
jq -e '
  .status == "failed" and
  .selection.uncovered_nodes == ["task:test.suite.missing"] and
  .issues == [{
    "code":"changed_node_uncovered",
    "message":"changed nodes selected no test evidence: task:test.suite.missing"
  }]
' "$uncovered_report" >/dev/null || fail "uncovered changed-node failure was not typed"

mixed_report="$WORKDIR/mixed.json"
if "$SLEY" test --json \
  --changed-node task:test.suite.classify \
  --changed-node task:test.suite.missing \
  "$FIXTURE" > "$mixed_report" 2>/dev/null; then
  fail "a mixed covered and uncovered changed-node request unexpectedly passed"
fi
jq -e '
  .status == "failed" and
  .selection.selected_cases == ["classify-table"] and
  .selection.uncovered_nodes == ["task:test.suite.missing"] and
  .issues[0].code == "changed_node_uncovered"
' "$mixed_report" >/dev/null || fail "mixed changed-node selection hid its uncovered node"

symlink_fixture="$WORKDIR/symlink-fixture"
cp -R "$FIXTURE" "$symlink_fixture"
cp "$symlink_fixture/src/test/suite.sley" "$WORKDIR/outside-source.sley"
rm -f "$symlink_fixture/src/test/suite.sley"
ln -s "$WORKDIR/outside-source.sley" "$symlink_fixture/src/test/suite.sley"
if "$SLEY" test --json "$symlink_fixture" > "$WORKDIR/symlink.json" 2> "$WORKDIR/symlink.err"; then
  fail "a nested source symlink unexpectedly passed suite discovery"
fi
rg -q 'Sley source must not be a symlink' "$WORKDIR/symlink.err" \
  || fail "nested source symlink did not fail at the source confinement boundary"

human_report="$WORKDIR/human.txt"
"$SLEY" test --changed-node file:tests/invalid.sley "$FIXTURE" > "$human_report"
rg -q '^Sley test passed: 1 passed, 0 failed, 5 skipped$' "$human_report" \
  || fail "human test summary was not stable"

source_digest="$(jq -r '.source.digest' "$full_report")"
jq --arg source_digest "$source_digest" \
  '.final.source_digest = $source_digest' \
  "$ROOT_DIR/fixtures/contracts/change_review_verified.json" > "$WORKDIR/review-core-stage.json"
review_digest="$(jq 'del(.review_id,.review_digest)' "$WORKDIR/review-core-stage.json" | jq -cS . | sha256sum | awk '{print "sha256:" $1}')"
review_id="review:${review_digest#sha256:}"
jq --arg review_id "$review_id" --arg review_digest "$review_digest" \
  '.review_id = $review_id | .review_digest = $review_digest' \
  "$WORKDIR/review-core-stage.json" > "$WORKDIR/review.json"
bundle_digest="$(jq -cS . "$WORKDIR/review.json" | sha256sum | awk '{print "sha256:" $1}')"
jq --arg source_digest "$source_digest" --arg review_id "$review_id" --arg review_digest "$review_digest" '
  .source_digest = $source_digest |
  .review_ref.schema = "sley.change.review.v0" |
  .review_ref.review_id = $review_id |
  .review_ref.review_digest = $review_digest
' "$ROOT_DIR/fixtures/contracts/change_transaction_seal_verified.json" \
  | jq --arg bundle_digest "$bundle_digest" '.bundle_digest = $bundle_digest' > "$WORKDIR/seal-core-stage.json"
seal_digest="$(jq 'del(.seal_id,.seal_digest)' "$WORKDIR/seal-core-stage.json" | jq -cS . | sha256sum | awk '{print "sha256:" $1}')"
seal_id="transaction-seal:${seal_digest#sha256:}"
jq --arg seal_id "$seal_id" --arg seal_digest "$seal_digest" \
  '.seal_id = $seal_id | .seal_digest = $seal_digest' \
  "$WORKDIR/seal-core-stage.json" > "$WORKDIR/seal.json"

packet="$WORKDIR/review-packet.json"
"$SLEY" test bind-review --json \
  --review "$WORKDIR/review.json" \
  --seal "$WORKDIR/seal.json" \
  --report "$full_report" > "$packet"
jq -e --arg source_digest "$source_digest" '
  .schema == "sley.review.packet.v1" and
  .status == "ready" and
  .final_source_digest == $source_digest and
  .test_ref.status == "passed" and
  .test_ref.source_digest == $source_digest and
  .binding == {
    "transaction_match":true,
    "review_match":true,
    "seal_match":true,
    "source_match":true,
    "test_passed":true
  } and
  .authority == {"mode":"read_only_evidence_binding","mutation_authority":false}
' "$packet" >/dev/null || fail "review packet did not bind exact passing test evidence"
"$ROOT_DIR/bin/sley-contract" validate \
  --schema sley.review.packet.v1 "$packet" --schemas "$ROOT_DIR/docs/schemas" --json \
  | jq -e '.status == "passed"' >/dev/null || fail "review packet did not validate"

jq '.suite.version = "tampered"' "$full_report" > "$WORKDIR/tampered-report.json"
if "$SLEY" test bind-review --json \
  --review "$WORKDIR/review.json" \
  --seal "$WORKDIR/seal.json" \
  --report "$WORKDIR/tampered-report.json" >/dev/null 2>&1; then
  fail "schema-valid test report tampering unexpectedly rebound to the review"
fi

echo "first-class Sley discovery, six case kinds, focused selection, coverage, reports, and review binding passed"
