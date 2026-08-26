#!/usr/bin/env bash
set -euo pipefail

ROOT_DIR="$(cd "$(dirname "$0")/.." && pwd)"
CORPUS="$ROOT_DIR/bin/sley-corpus"
CONTRACT="$ROOT_DIR/bin/sley-contract"
FIXTURE="$ROOT_DIR/fixtures/corpus_governance"
WORK_DIR="$(mktemp -d "$ROOT_DIR/.sley-corpus-test.XXXXXXXX")"
trap 'rm -rf -- "$WORK_DIR"' EXIT

fail() {
  printf 'Sley corpus test failed: %s\n' "$*" >&2
  exit 1
}

expect_failure() {
  local name="$1"
  shift
  if "$@" >"$WORK_DIR/$name.json" 2>"$WORK_DIR/$name.err"; then
    fail "$name unexpectedly passed"
  fi
}

cp "$FIXTURE"/* "$WORK_DIR"/

"$CORPUS" admission-check \
  --record "$WORK_DIR/record.json" \
  --exclusions "$WORK_DIR/exclusions.json" \
  --root "$WORK_DIR" --json >"$WORK_DIR/admission-1.json"
"$CORPUS" admission-check \
  --record "$WORK_DIR/record.json" \
  --exclusions "$WORK_DIR/exclusions.json" \
  --root "$WORK_DIR" --json >"$WORK_DIR/admission-2.json"
cmp -s "$WORK_DIR/admission-1.json" "$WORK_DIR/admission-2.json" || fail "admission report is not deterministic"
jq -e '.status == "passed" and .record_count == 1 and .summary.failed_count == 0' "$WORK_DIR/admission-1.json" >/dev/null
"$CONTRACT" validate --schema sley.corpus.audit_report.v0 "$WORK_DIR/admission-1.json" \
  --schemas "$ROOT_DIR/docs/schemas" --json | jq -e '.status == "passed"' >/dev/null

"$CORPUS" audit --manifest "$WORK_DIR/manifest.json" --root "$WORK_DIR" --json >"$WORK_DIR/audit.json"
jq -e '.status == "passed" and .record_count == 1 and .summary.failed_count == 0' "$WORK_DIR/audit.json" >/dev/null
"$CONTRACT" validate --schema sley.corpus.audit_report.v0 "$WORK_DIR/audit.json" \
  --schemas "$ROOT_DIR/docs/schemas" --json | jq -e '.status == "passed"' >/dev/null

record_before="$(sha256sum "$WORK_DIR/record.json")"
"$CORPUS" deletion-plan \
  --record "$WORK_DIR/record.json" \
  --requested-at 2026-08-23T00:00:00Z \
  --requested-by sley-maintainers \
  --reason fixture-removal-test --json >"$WORK_DIR/deletion-1.json"
"$CORPUS" deletion-plan \
  --record "$WORK_DIR/record.json" \
  --requested-at 2026-08-23T00:00:00Z \
  --requested-by sley-maintainers \
  --reason fixture-removal-test --json >"$WORK_DIR/deletion-2.json"
record_after="$(sha256sum "$WORK_DIR/record.json")"
[[ "$record_before" == "$record_after" ]] || fail "deletion plan mutated its record"
cmp -s "$WORK_DIR/deletion-1.json" "$WORK_DIR/deletion-2.json" || fail "deletion plan is not deterministic"
jq -e '
  .status == "passed" and .dry_run == true and .mutation_performed == false and
  ([.targets[].kind] | index("record")) != null and
  ([.targets[].kind] | index("sidecar")) != null and
  ([.targets[].kind] | index("retrieval_entry")) != null and
  ([.targets[].kind] | index("export_eligibility")) != null and
  ([.targets[].kind] | index("tombstone")) != null
' "$WORK_DIR/deletion-1.json" >/dev/null
"$CONTRACT" validate --schema sley.corpus.deletion_plan.v0 "$WORK_DIR/deletion-1.json" \
  --schemas "$ROOT_DIR/docs/schemas" --json | jq -e '.status == "passed"' >/dev/null

jq '.unexpected = true' "$WORK_DIR/record.json" >"$WORK_DIR/unknown-record.json"
expect_failure unknown-field "$CONTRACT" validate --schema sley.corpus.record.v0 \
  "$WORK_DIR/unknown-record.json" --schemas "$ROOT_DIR/docs/schemas" --json
jq -e '.status == "failed" and ([.issues[].code] | index("json_schema_violation")) != null' \
  "$WORK_DIR/unknown-field.json" >/dev/null

jq '.digests.structural = "sha256:92b6f5ba2c833eb255ec2ae0b07f5bace0b0e5119d06d5a6ab34808cbc466087"' \
  "$WORK_DIR/record.json" >"$WORK_DIR/contaminated-record.json"
expect_failure contaminated "$CORPUS" admission-check \
  --record "$WORK_DIR/contaminated-record.json" --exclusions "$WORK_DIR/exclusions.json" \
  --root "$WORK_DIR" --json
jq -e '.status == "failed" and ([.issues[].code] | index("benchmark_contamination")) != null' \
  "$WORK_DIR/contaminated.json" >/dev/null

jq '.benchmark_split_status = "incomplete"' "$WORK_DIR/exclusions.json" >"$WORK_DIR/incomplete-exclusions.json"
expect_failure incomplete-split "$CORPUS" admission-check \
  --record "$WORK_DIR/record.json" --exclusions "$WORK_DIR/incomplete-exclusions.json" \
  --root "$WORK_DIR" --json
jq -e '.status == "failed" and ([.issues[].code] | index("benchmark_split_incomplete")) != null' \
  "$WORK_DIR/incomplete-split.json" >/dev/null

printf '\ncorrupt\n' >>"$WORK_DIR/oracle.json"
expect_failure sidecar-digest "$CORPUS" admission-check \
  --record "$WORK_DIR/record.json" --exclusions "$WORK_DIR/exclusions.json" \
  --root "$WORK_DIR" --json
jq -e '.status == "failed" and ([.issues[].code] | index("sidecar_validation_failed")) != null' \
  "$WORK_DIR/sidecar-digest.json" >/dev/null

cp "$FIXTURE/oracle.json" "$WORK_DIR/oracle.json"
jq '.records[0].digest = "sha256:0000000000000000000000000000000000000000000000000000000000000000"' \
  "$WORK_DIR/manifest.json" >"$WORK_DIR/bad-manifest.json"
expect_failure manifest-pin "$CORPUS" audit --manifest "$WORK_DIR/bad-manifest.json" --root "$WORK_DIR" --json
jq -e '.status == "failed" and ([.issues[].code] | index("record_manifest_pin_failed")) != null' \
  "$WORK_DIR/manifest-pin.json" >/dev/null

jq '
  .record_id = "sley-corpus-fixture-eval-001" |
  .partition = "evaluation_public"
' "$WORK_DIR/record.json" >"$WORK_DIR/evaluation-record.json"
evaluation_digest="sha256:$(sha256sum "$WORK_DIR/evaluation-record.json" | awk '{print $1}')"
jq --arg digest "$evaluation_digest" '
  .records += [{
    record_id:"sley-corpus-fixture-eval-001",
    path:"evaluation-record.json",
    digest:$digest,
    partition:"evaluation_public",
    semantic_lineage_id:"corpus-governance-lineage-001",
    semantic_lineage_digest:"sha256:89079d49d755140fabf0b9fa54d90f73693cde655a3ba5c68fa823e217057f48"
  }] |
  .partition_counts.evaluation_public = 1
' "$WORK_DIR/manifest.json" >"$WORK_DIR/overlap-manifest.json"
expect_failure split-overlap "$CORPUS" audit --manifest "$WORK_DIR/overlap-manifest.json" --root "$WORK_DIR" --json
jq -e '.status == "failed" and ([.issues[].code] | index("split_lineage_overlap")) != null' \
  "$WORK_DIR/split-overlap.json" >/dev/null

audit_input="$WORK_DIR/audit-event-1.json"
cp "$ROOT_DIR/fixtures/contracts/corpus_audit_event_input_governance.json" "$audit_input"
touch "$WORK_DIR/audit-empty.jsonl"
"$CORPUS" audit-log --log "$WORK_DIR/audit-empty.jsonl" \
  --log-id sley-corpus-governance --json >"$WORK_DIR/audit-empty-report.json"
jq -e '
  .status == "passed" and .event_count == 0 and .head_digest == null and
  .mutation_performed == false and .log_digest_before == .log_digest_after
' "$WORK_DIR/audit-empty-report.json" >/dev/null
"$CONTRACT" validate --schema sley.corpus.mutation_audit_report.v0 \
  "$WORK_DIR/audit-empty-report.json" --schemas "$ROOT_DIR/docs/schemas" --json \
  | jq -e '.status == "passed"' >/dev/null

touch "$WORK_DIR/no-confirm.jsonl"
no_confirm_before="$(sha256sum "$WORK_DIR/no-confirm.jsonl")"
expect_failure no-confirm "$CORPUS" append-event \
  --log "$WORK_DIR/no-confirm.jsonl" --event "$audit_input" \
  --log-id sley-corpus-governance --expected-head none --json
no_confirm_after="$(sha256sum "$WORK_DIR/no-confirm.jsonl")"
[[ "$no_confirm_before" == "$no_confirm_after" ]] || fail "unconfirmed append mutated its log"
jq -e '
  .status == "failed" and .mutation_performed == false and
  ([.issues[].code] | index("append_confirmation_required")) != null
' "$WORK_DIR/no-confirm.json" >/dev/null

touch "$WORK_DIR/audit-a.jsonl" "$WORK_DIR/audit-b.jsonl"
for audit_log in "$WORK_DIR/audit-a.jsonl" "$WORK_DIR/audit-b.jsonl"; do
  "$CORPUS" append-event --log "$audit_log" --event "$audit_input" \
    --log-id sley-corpus-governance --expected-head none \
    --confirm-append-only-audit-event --json >"$audit_log.report.json"
done
cmp -s "$WORK_DIR/audit-a.jsonl" "$WORK_DIR/audit-b.jsonl" || fail "first append is not deterministic"
cmp -s "$WORK_DIR/audit-a.jsonl.report.json" "$WORK_DIR/audit-b.jsonl.report.json" \
  || fail "append report leaks path state or is not deterministic"
jq -e '
  .status == "passed" and .event_count == 1 and .mutation_performed == true and
  .head_digest == .appended_event_digest and .log_digest_before != .log_digest_after
' "$WORK_DIR/audit-a.jsonl.report.json" >/dev/null
"$CONTRACT" validate --schema sley.corpus.mutation_audit_report.v0 \
  "$WORK_DIR/audit-a.jsonl.report.json" --schemas "$ROOT_DIR/docs/schemas" --json \
  | jq -e '.status == "passed"' >/dev/null

head_digest="$(jq -r '.appended_event_digest' "$WORK_DIR/audit-a.jsonl.report.json")"
jq '
  .event_id = "event-20260823-002" |
  .kind = "reclassification" |
  .occurred_at = "2026-08-23T00:01:00Z" |
  .transition = {from:"admitted", to:"train"} |
  .reason_code = "fixture-train-partition"
' "$audit_input" >"$WORK_DIR/audit-event-2.json"
"$CORPUS" append-event --log "$WORK_DIR/audit-a.jsonl" \
  --event "$WORK_DIR/audit-event-2.json" --log-id sley-corpus-governance \
  --expected-head "$head_digest" --confirm-append-only-audit-event --json \
  >"$WORK_DIR/append-2.json"
jq -e '
  .status == "passed" and .event_count == 2 and .mutation_performed == true and
  .head_digest == .appended_event_digest
' "$WORK_DIR/append-2.json" >/dev/null
"$CORPUS" audit-log --log "$WORK_DIR/audit-a.jsonl" \
  --log-id sley-corpus-governance --json >"$WORK_DIR/audit-chain.json"
jq -e '
  .status == "passed" and .event_count == 2 and .mutation_performed == false and
  ([.checks[] | select(.id == "digest_chain") | .status] | index("passed")) != null and
  ([.checks[] | select(.id == "subject_state_continuity") | .status] | index("passed")) != null
' "$WORK_DIR/audit-chain.json" >/dev/null

current_head="$(jq -r '.appended_event_digest' "$WORK_DIR/append-2.json")"
jq '
  .event_id = "event-20260823-003" |
  .kind = "reclassification" |
  .occurred_at = "2026-08-23T00:02:00Z" |
  .transition = {from:"candidate", to:"development"} |
  .reason_code = "fixture-discontinuous-state"
' "$audit_input" >"$WORK_DIR/audit-event-discontinuous.json"
continuity_before="$(sha256sum "$WORK_DIR/audit-a.jsonl")"
expect_failure subject-continuity "$CORPUS" append-event \
  --log "$WORK_DIR/audit-a.jsonl" --event "$WORK_DIR/audit-event-discontinuous.json" \
  --log-id sley-corpus-governance --expected-head "$current_head" \
  --confirm-append-only-audit-event --json
continuity_after="$(sha256sum "$WORK_DIR/audit-a.jsonl")"
[[ "$continuity_before" == "$continuity_after" ]] || fail "state-discontinuous append mutated its log"
jq -e '
  .status == "failed" and .mutation_performed == false and
  ([.issues[].code] | index("subject_state_discontinuity")) != null
' "$WORK_DIR/subject-continuity.json" >/dev/null

stale_before="$(sha256sum "$WORK_DIR/audit-a.jsonl")"
expect_failure stale-head "$CORPUS" append-event \
  --log "$WORK_DIR/audit-a.jsonl" --event "$WORK_DIR/audit-event-2.json" \
  --log-id sley-corpus-governance --expected-head none \
  --confirm-append-only-audit-event --json
stale_after="$(sha256sum "$WORK_DIR/audit-a.jsonl")"
[[ "$stale_before" == "$stale_after" ]] || fail "stale-head append mutated its log"
jq -e '
  .status == "failed" and .mutation_performed == false and
  ([.issues[].code] | index("stale_audit_log_head")) != null
' "$WORK_DIR/stale-head.json" >/dev/null

jq -c 'if .sequence == 1 then .reason_code = "tampered" else . end' \
  "$WORK_DIR/audit-a.jsonl" >"$WORK_DIR/audit-tampered.jsonl"
expect_failure tampered-log "$CORPUS" audit-log \
  --log "$WORK_DIR/audit-tampered.jsonl" --log-id sley-corpus-governance --json
jq -e '
  .status == "failed" and .mutation_performed == false and
  ([.issues[].code] | index("audit_event_digest_mismatch")) != null
' "$WORK_DIR/tampered-log.json" >/dev/null

jq '.unexpected = true' "$audit_input" >"$WORK_DIR/audit-event-unknown.json"
expect_failure audit-event-unknown-result "$CORPUS" append-event \
  --log "$WORK_DIR/unknown-log.jsonl" --event "$WORK_DIR/audit-event-unknown.json" \
  --log-id sley-corpus-governance --expected-head none \
  --confirm-append-only-audit-event --json
[[ ! -e "$WORK_DIR/unknown-log.jsonl" ]] || fail "invalid event input created a log"
jq -e '
  .status == "failed" and .mutation_performed == false and
  ([.issues[].code] | index("json_schema_violation")) != null
' "$WORK_DIR/audit-event-unknown-result.json" >/dev/null
"$CONTRACT" validate --schema sley.corpus.mutation_audit_report.v0 \
  "$WORK_DIR/audit-event-unknown-result.json" --schemas "$ROOT_DIR/docs/schemas" --json \
  | jq -e '.status == "passed"' >/dev/null

touch "$WORK_DIR/symlink-target.jsonl"
ln -s "$WORK_DIR/symlink-target.jsonl" "$WORK_DIR/symlink-log.jsonl"
symlink_before="$(sha256sum "$WORK_DIR/symlink-target.jsonl")"
expect_failure symlink-log "$CORPUS" append-event \
  --log "$WORK_DIR/symlink-log.jsonl" --event "$audit_input" \
  --log-id sley-corpus-governance --expected-head none \
  --confirm-append-only-audit-event --json
symlink_after="$(sha256sum "$WORK_DIR/symlink-target.jsonl")"
[[ "$symlink_before" == "$symlink_after" ]] || fail "symlink rejection mutated its target"
jq -e '.status == "failed" and .mutation_performed == false' "$WORK_DIR/symlink-log.json" >/dev/null

printf 'Sley corpus governance tests passed.\n'
