#!/usr/bin/env bash
set -euo pipefail

ROOT_DIR="$(cd "$(dirname "$0")/.." && pwd)"
SLEY="$ROOT_DIR/bin/sley"
CONTRACT="$ROOT_DIR/bin/sley-contract"
SCHEMA_DIR="$ROOT_DIR/docs/schemas"
WORK_DIR="$(mktemp -d)"
OPERATION="$WORK_DIR/cross-module.json"

cleanup() {
  local rc=$?
  if [[ "${SLEY_KEEP_TEST_WORK_DIR:-0}" == "1" ]]; then
    echo "change review test artifacts retained at $WORK_DIR" >&2
  else
    rm -rf "$WORK_DIR"
  fi
  exit "$rc"
}
trap cleanup EXIT

fail() {
  echo "change review contract failed: $*" >&2
  exit 1
}

expect_diagnostic() {
  local expected_id="$1"
  shift
  local output="$WORK_DIR/error-$expected_id.json"
  if "$SLEY" "$@" >"$output" 2>/dev/null; then
    fail "command unexpectedly succeeded for $expected_id"
  fi
  jq -e --arg id "$expected_id" '
    .schema == "sley.diagnostics.report.v0"
    and .status == "error"
    and any(.diagnostics[]; .id == $id and .severity == "error")
  ' "$output" >/dev/null || fail "missing typed diagnostic $expected_id"
}

prepare_repository() {
  local repository="$1"
  mkdir -p "$repository/examples"
  cp -a "$ROOT_DIR/examples/project" "$repository/examples/project"
  git -C "$repository" init -q
  git -C "$repository" config user.name "Sley Review Test"
  git -C "$repository" config user.email "sley-review@example.invalid"
  git -C "$repository" add examples/project
  git -C "$repository" commit -qm "fixture base"
}

build_chain() {
  local repository="$1" output="$2" nonce="$3"
  mkdir -p "$output"
  "$SLEY" change inspect --json \
    --goal "review one bounded structural edit" --actor "operator:test" --nonce "$nonce-inspect" \
    --created-at "2026-08-25T00:00:00Z" --expires-at "2099-01-01T00:00:00Z" \
    "$repository/examples/project" >"$output/inspect.json"
  "$SLEY" change plan --json --repository "$repository" \
    --inspection "$output/inspect.json" --operation "$OPERATION" \
    --requested-outcome "rename the exported task and update its caller" \
    --assumption "the inspected base remains current" --non-goal "do not change source outside the candidate" \
    >"$output/plan.json"
  "$SLEY" change preview --json --repository "$repository" --plan "$output/plan.json" >"$output/preview.json"
  "$SLEY" change approval-request --json --repository "$repository" \
    --preview "$output/preview.json" >"$output/request.json"
  "$SLEY" change approve --json --repository "$repository" \
    --request "$output/request.json" --preview "$output/preview.json" --confirm-exact-local-grant \
    --issuer "operator:test" --principal "agent:test" --audience "sley:local" \
    --purpose "authorize only this immutable candidate" --nonce "$nonce-grant" \
    --created-at "2026-08-25T00:00:01Z" --expires-at "2098-01-01T00:00:00Z" \
    --ack-review "human_source_review" --ack-review "graph_diff_review" \
    --ack-review "lint_review" --ack-review "authority_review" \
    --revocation-state "not_revoked" --max-wall-clock-seconds "300" >"$output/grant.json"
  "$SLEY" change apply-authorization --json --repository "$repository" \
    --grant "$output/grant.json" --request "$output/request.json" --preview "$output/preview.json" \
    --principal "agent:test" --audience "sley:local" >"$output/authorization.json"
  "$SLEY" change revocation-record --json --repository "$repository" \
    --authorization "$output/authorization.json" --grant "$output/grant.json" \
    --state "not_revoked" --issuer "operator:test" --reason "grant remains locally active" \
    --created-at "2026-08-25T00:00:02Z" --expires-at "2098-01-01T00:00:00Z" \
    --confirm-revocation-record >"$output/revocation.json"
  "$SLEY" change apply --json --repository "$repository" \
    --authorization "$output/authorization.json" --grant "$output/grant.json" \
    --request "$output/request.json" --preview "$output/preview.json" \
    --principal "agent:test" --audience "sley:local" --confirm-atomic-apply >"$output/apply.json"
}

review_args() {
  local repository="$1" output="$2"
  printf '%s\n' \
    change review --json --repository "$repository" \
    --authorization "$output/authorization.json" --grant "$output/grant.json" \
    --request "$output/request.json" --preview "$output/preview.json" --apply "$output/apply.json" \
    --principal agent:test --audience sley:local
}

validate_digest_chain() {
  local review_dir="$1"
  local review_digest seal_digest packet_digest bundle_digest zjx_digest trace_digest
  review_digest="$(jq 'del(.review_id,.review_digest)' "$review_dir/bundle.json" | jq -cS . | sha256sum | awk '{print "sha256:" $1}')"
  seal_digest="$(jq 'del(.seal_id,.seal_digest)' "$review_dir/seal.json" | jq -cS . | sha256sum | awk '{print "sha256:" $1}')"
  packet_digest="$(sha256sum "$review_dir/packet.md" | awk '{print "sha256:" $1}')"
  bundle_digest="$(jq -cS . "$review_dir/bundle.json" | sha256sum | awk '{print "sha256:" $1}')"
  zjx_digest="$(jq -cS . "$review_dir/evidence.zjx.json" | sha256sum | awk '{print "sha256:" $1}')"
  trace_digest="$(jq '.trace_receipts // []' "$review_dir/evidence.zjx.json" | jq -cS . | sha256sum | awk '{print "sha256:" $1}')"
  jq -e --arg review "$review_digest" --arg seal "$seal_digest" --arg packet "$packet_digest" \
    --arg bundle "$bundle_digest" --arg zjx "$zjx_digest" --arg trace "$trace_digest" '
    .seal_digest == $seal
    and .review_ref.review_digest == $review
    and .packet_digest == $packet
    and .bundle_digest == $bundle
    and .zjx_digest == $zjx
    and .trace_seal_ref.trace_digest == $trace
  ' "$review_dir/seal.json" >/dev/null || fail "content-addressed seal chain does not recompute"
}

jq '.ops |= map(select(.op == "RenameDeclaration" or .op == "UpdateCallSites"))' \
  "$ROOT_DIR/fixtures/transactions/cross_module_preview.json" >"$OPERATION"

APPLY_REPO="$WORK_DIR/apply-repository"
APPLY_CHAIN="$WORK_DIR/apply-chain"
prepare_repository "$APPLY_REPO"
build_chain "$APPLY_REPO" "$APPLY_CHAIN" "review-apply-001"

mapfile -t APPLY_ARGS < <(review_args "$APPLY_REPO" "$APPLY_CHAIN")
expect_diagnostic TRANSACTION_REVIEW_CONFIRMATION_REQUIRED "${APPLY_ARGS[@]}"

jq '.candidate.candidate_source_digest = "sha256:0000000000000000000000000000000000000000000000000000000000000000"' \
  "$APPLY_CHAIN/apply.json" >"$APPLY_CHAIN/apply-tampered.json"
TAMPERED_ARGS=(change review --json --repository "$APPLY_REPO" \
  --authorization "$APPLY_CHAIN/authorization.json" --grant "$APPLY_CHAIN/grant.json" \
  --request "$APPLY_CHAIN/request.json" --preview "$APPLY_CHAIN/preview.json" --apply "$APPLY_CHAIN/apply-tampered.json" \
  --principal agent:test --audience sley:local --confirm-review-packet)
expect_diagnostic TRANSACTION_REVIEW_CHAIN_MISMATCH "${TAMPERED_ARGS[@]}"
jq '.verification.status = "failed"' "$APPLY_CHAIN/apply.json" >"$APPLY_CHAIN/apply-verification-tampered.json"
VERIFICATION_TAMPER_ARGS=(change review --json --repository "$APPLY_REPO" \
  --authorization "$APPLY_CHAIN/authorization.json" --grant "$APPLY_CHAIN/grant.json" \
  --request "$APPLY_CHAIN/request.json" --preview "$APPLY_CHAIN/preview.json" --apply "$APPLY_CHAIN/apply-verification-tampered.json" \
  --principal agent:test --audience sley:local --confirm-review-packet)
expect_diagnostic TRANSACTION_REVIEW_TERMINAL_MISMATCH "${VERIFICATION_TAMPER_ARGS[@]}"

mkdir -p "$APPLY_REPO/.sley/review-test"
cp "$ROOT_DIR/fixtures/contracts/trace_receipt_hello.json" "$APPLY_REPO/.sley/review-test/trace.jsonl"
printf '%s\n' '{"schema":"sley.trace.receipt.v0","unexpected":true}' >"$APPLY_REPO/.sley/review-test/invalid.jsonl"
INVALID_TRACE_ARGS=("${APPLY_ARGS[@]}" --trace "$APPLY_REPO/.sley/review-test/invalid.jsonl" --confirm-review-packet)
expect_diagnostic TRANSACTION_TRACE_RECEIPT_INVALID "${INVALID_TRACE_ARGS[@]}"

index_path="$(git -C "$APPLY_REPO" rev-parse --absolute-git-dir)/index"
source_before="$(find "$APPLY_REPO/examples/project" -type f -print0 | sort -z | xargs -0 sha256sum | sha256sum | awk '{print $1}')"
index_before="$(sha256sum "$index_path" | awk '{print $1}')"
recovery_path="$(jq -r '.recovery_ref.path' "$APPLY_CHAIN/apply.json")"
recovery_before="$(sha256sum "$APPLY_REPO/$recovery_path" | awk '{print $1}')"
rollback_race_ready="$WORK_DIR/rollback-race.ready"
rollback_race_release="$WORK_DIR/rollback-race.release"
rollback_race_output="$WORK_DIR/rollback-race.json"
mkfifo "$rollback_race_release"
SLEY_ALLOW_TEST_HOOKS=1 \
  SLEY_TEST_ROLLBACK_BEFORE_LOCK_READY="$rollback_race_ready" \
  SLEY_TEST_ROLLBACK_BEFORE_LOCK_RELEASE="$rollback_race_release" \
  "$SLEY" change rollback --json --repository "$APPLY_REPO" \
    --apply "$APPLY_CHAIN/apply.json" --confirm-rollback >"$rollback_race_output" 2>/dev/null &
rollback_race_pid=$!
for _ in $(seq 1 200); do
  [[ -f "$rollback_race_ready" ]] && break
  sleep 0.05
done
[[ -f "$rollback_race_ready" ]] || fail "rollback did not reach the pre-lock race barrier"
"$SLEY" "${APPLY_ARGS[@]}" --trace "$APPLY_REPO/.sley/review-test/trace.jsonl" --confirm-review-packet \
  >"$APPLY_CHAIN/review.json"
printf 'continue\n' >"$rollback_race_release"
if wait "$rollback_race_pid"; then
  fail "rollback crossed a review seal published before lock acquisition"
fi
jq -e '
  .schema == "sley.diagnostics.report.v0"
  and .status == "error"
  and any(.diagnostics[]; .id == "TRANSACTION_ALREADY_SEALED" and .severity == "error")
' "$rollback_race_output" >/dev/null || fail "interleaved rollback did not fail with TRANSACTION_ALREADY_SEALED"

transaction_hex="$(jq -r '.transaction.transaction_id | sub("^sha256:";"")' "$APPLY_CHAIN/review.json")"
review_dir="$APPLY_REPO/.sley/transactions/$transaction_hex/review"
[[ "$(find "$review_dir" -mindepth 1 -maxdepth 1 -type f -printf '%f\n' | sort | tr '\n' ' ')" == \
  "bundle.json evidence.zjx.json packet.md seal.json " ]] || fail "review directory does not contain exactly four regular artifacts"
"$CONTRACT" validate --schema sley.change.review.v0 "$review_dir/bundle.json" --schemas "$SCHEMA_DIR" --json >/dev/null
"$CONTRACT" validate --schema sley.change.transaction_seal.v0 "$review_dir/seal.json" --schemas "$SCHEMA_DIR" --json >/dev/null
"$CONTRACT" validate --schema sley.zjx.envelope.v0 "$review_dir/evidence.zjx.json" --schemas "$SCHEMA_DIR" --json >/dev/null
jq -e '
  .terminal_outcome == "verified_apply"
  and .final.state == "candidate"
  and .validation_summary.status == "passed"
  and .trace_summary.receipt_count == 1
  and .mutation_summary == {attempted_change:true,repository_source_changed:true,rollback_performed:false,review_artifacts_written:true}
  and all(.no_mutation_boundary[]; . == false)
  and (.issues | length) == 0
' "$review_dir/bundle.json" >/dev/null || fail "verified apply review bundle is incomplete"
grep -q '^Goal: review one bounded structural edit$' "$review_dir/packet.md" || fail "human review packet lost the bounded goal"
if grep -Fq '\n' "$review_dir/packet.md"; then
  fail "human review packet contains escaped newline text"
fi
validate_digest_chain "$review_dir"
[[ "$source_before" == "$(find "$APPLY_REPO/examples/project" -type f -print0 | sort -z | xargs -0 sha256sum | sha256sum | awk '{print $1}')" ]] \
  || fail "review changed repository source"
[[ "$index_before" == "$(sha256sum "$index_path" | awk '{print $1}')" ]] || fail "review changed the Git index"
[[ "$recovery_before" == "$(sha256sum "$APPLY_REPO/$recovery_path" | awk '{print $1}')" ]] || fail "review changed durable recovery state"

expect_diagnostic TRANSACTION_REVIEW_PUBLISH_FAILED "${APPLY_ARGS[@]}" --trace "$APPLY_REPO/.sley/review-test/trace.jsonl" --confirm-review-packet
expect_diagnostic TRANSACTION_ALREADY_SEALED change rollback --json --repository "$APPLY_REPO" \
  --apply "$APPLY_CHAIN/apply.json" --confirm-rollback
printf '\n' >>"$APPLY_REPO/examples/project/src/app/math.sley"
expect_diagnostic TRANSACTION_REVIEW_FINAL_DIGEST_MISMATCH "${APPLY_ARGS[@]}" --confirm-review-packet

ROLLBACK_REPO="$WORK_DIR/rollback-repository"
ROLLBACK_CHAIN="$WORK_DIR/rollback-chain"
prepare_repository "$ROLLBACK_REPO"
build_chain "$ROLLBACK_REPO" "$ROLLBACK_CHAIN" "review-rollback-001"
rollback_recovery_path="$(jq -r '.recovery_ref.path' "$ROLLBACK_CHAIN/apply.json")"
cp "$ROLLBACK_REPO/$rollback_recovery_path" "$ROLLBACK_CHAIN/recovery-terminal.json"
recovery_core="$(jq '.phase="verifying" | .status="active" | del(.record_digest)' "$ROLLBACK_REPO/$rollback_recovery_path")"
recovery_digest="$(printf '%s\n' "$recovery_core" | jq -cS . | sha256sum | awk '{print "sha256:" $1}')"
printf '%s\n' "$recovery_core" | jq --arg digest "$recovery_digest" '.record_digest=$digest' >"$ROLLBACK_REPO/$rollback_recovery_path"
mapfile -t ROLLBACK_ARGS < <(review_args "$ROLLBACK_REPO" "$ROLLBACK_CHAIN")
expect_diagnostic TRANSACTION_REVIEW_NOT_TERMINAL "${ROLLBACK_ARGS[@]}" --confirm-review-packet
cp "$ROLLBACK_CHAIN/recovery-terminal.json" "$ROLLBACK_REPO/$rollback_recovery_path"

"$SLEY" change rollback --json --repository "$ROLLBACK_REPO" \
  --apply "$ROLLBACK_CHAIN/apply.json" --confirm-rollback >"$ROLLBACK_CHAIN/rollback.json"
"$SLEY" "${ROLLBACK_ARGS[@]}" --rollback "$ROLLBACK_CHAIN/rollback.json" --confirm-review-packet \
  >"$ROLLBACK_CHAIN/review.json"
rollback_hex="$(jq -r '.transaction.transaction_id | sub("^sha256:";"")' "$ROLLBACK_CHAIN/review.json")"
rollback_review_dir="$ROLLBACK_REPO/.sley/transactions/$rollback_hex/review"
"$CONTRACT" validate --schema sley.change.review.v0 "$rollback_review_dir/bundle.json" --schemas "$SCHEMA_DIR" --json >/dev/null
"$CONTRACT" validate --schema sley.change.transaction_seal.v0 "$rollback_review_dir/seal.json" --schemas "$SCHEMA_DIR" --json >/dev/null
jq -e '
  .terminal_outcome == "verified_rollback"
  and .final.state == "restored_base"
  and .validation_summary.rollback.status == "passed"
  and .mutation_summary.rollback_performed == true
  and any(.issues[]; .id == "TRANSACTION_MANUAL_ROLLBACK")
' "$rollback_review_dir/bundle.json" >/dev/null || fail "verified rollback review bundle is incomplete"
validate_digest_chain "$rollback_review_dir"
git -C "$ROLLBACK_REPO" diff --quiet -- examples/project || fail "reviewed rollback did not preserve the exact base"

echo "change review packets, terminal seals, no-mutation boundaries, and verified rollback evidence passed"
