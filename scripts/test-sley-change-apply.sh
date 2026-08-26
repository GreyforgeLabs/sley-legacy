#!/usr/bin/env bash
set -euo pipefail

ROOT_DIR="$(cd "$(dirname "$0")/.." && pwd)"
SLEY="$ROOT_DIR/bin/sley"
CONTRACT="$ROOT_DIR/bin/sley-contract"
SCHEMA_DIR="$ROOT_DIR/docs/schemas"
WORK_DIR="$(mktemp -d)"
TEST_REPO="$WORK_DIR/repository"
TARGET="$TEST_REPO/examples/project"
OPERATION="$WORK_DIR/cross-module.json"

cleanup() {
  local rc=$?
  if [[ "${SLEY_KEEP_TEST_WORK_DIR:-0}" == "1" ]]; then
    echo "change apply test artifacts retained at $WORK_DIR" >&2
  else
    rm -rf "$WORK_DIR"
  fi
  exit "$rc"
}
trap cleanup EXIT

fail() {
  echo "change apply contract failed: $*" >&2
  exit 1
}

expect_diagnostic() {
  local expected_id="$1"
  shift
  local output="$WORK_DIR/error.json"
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
  git -C "$repository" config user.name "Sley Apply Test"
  git -C "$repository" config user.email "sley-apply@example.invalid"
  git -C "$repository" add examples/project
  git -C "$repository" commit -qm "fixture base"
}

build_apply_chain() {
  local repository="$1" operation="$2" output="$3" nonce="$4"
  local target="$repository/examples/project"
  mkdir -p "$output"
  "$SLEY" change inspect --json \
    --goal "apply one bounded structural edit" --actor "operator:test" --nonce "$nonce-inspect" \
    --created-at "2026-08-25T00:00:00Z" --expires-at "2099-01-01T00:00:00Z" \
    "$target" >"$output/inspect.json"
  "$SLEY" change plan --json --repository "$repository" \
    --inspection "$output/inspect.json" --operation "$operation" \
    --requested-outcome "apply the exact structural candidate" --assumption "the inspected base remains current" \
    --non-goal "do not change source outside the candidate" >"$output/plan.json"
  "$SLEY" change preview --json --repository "$repository" --plan "$output/plan.json" >"$output/preview.json"
  "$SLEY" change approval-request --json --repository "$repository" \
    --preview "$output/preview.json" >"$output/request.json"
  "$SLEY" change approve --json --repository "$repository" \
    --request "$output/request.json" --preview "$output/preview.json" --confirm-exact-local-grant \
    --issuer "operator:test" --principal "agent:test" --audience "sley:local" \
    --purpose "authorize only the exact immutable candidate" --nonce "$nonce-grant" \
    --created-at "2026-08-25T00:00:01Z" --expires-at "2098-01-01T00:00:00Z" \
    --ack-review "human_source_review" --ack-review "graph_diff_review" \
    --ack-review "lint_review" --ack-review "authority_review" \
    --revocation-state "not_revoked" --max-wall-clock-seconds "300" >"$output/grant.json"
  "$SLEY" change apply-authorization --json --repository "$repository" \
    --grant "$output/grant.json" --request "$output/request.json" --preview "$output/preview.json" \
    --principal "agent:test" --audience "sley:local" >"$output/authorization.json"
}

initialize_revocation() {
  local repository="$1" output="$2" created_at="$3"
  "$SLEY" change revocation-record --json --repository "$repository" \
    --authorization "$output/authorization.json" --grant "$output/grant.json" \
    --state "not_revoked" --issuer "operator:test" --reason "grant remains locally active" \
    --created-at "$created_at" --expires-at "2098-01-01T00:00:00Z" \
    --confirm-revocation-record >"$output/revocation.json"
}

crash_apply_at() {
  local repository="$1" output="$2" phase="$3"
  if SLEY_ALLOW_TEST_HOOKS=1 SLEY_TEST_CRASH_AT="$phase" "$SLEY" change apply --json --repository "$repository" \
    --authorization "$output/authorization.json" --grant "$output/grant.json" \
    --request "$output/request.json" --preview "$output/preview.json" \
    --principal "agent:test" --audience "sley:local" --confirm-atomic-apply \
    >"$output/crashed-apply.json" 2>/dev/null; then
    fail "crash hook $phase unexpectedly returned apply success"
  fi
}

recover_apply() {
  local repository="$1" output="$2" expected_status="$3"
  local rc=0
  "$SLEY" change recover --json --repository "$repository" \
    --authorization "$output/authorization.json" --grant "$output/grant.json" \
    --request "$output/request.json" --preview "$output/preview.json" \
    --principal "agent:test" --audience "sley:local" --confirm-recovery \
    >"$output/recovered-apply.json" || rc=$?
  if [[ "$expected_status" == "verified" && "$rc" -ne 0 ]]; then
    fail "recovery expected verified success but returned $rc"
  fi
  if [[ "$expected_status" == "rolled_back" && "$rc" -eq 0 ]]; then
    fail "recovered verification failure unexpectedly returned success"
  fi
  "$CONTRACT" validate --schema sley.change.apply.v0 "$output/recovered-apply.json" --schemas "$SCHEMA_DIR" --json >/dev/null
  jq -e --arg status "$expected_status" '.status == $status and .recovery_ref.phase == $status' \
    "$output/recovered-apply.json" >/dev/null || fail "recovered apply status or phase is incomplete"
}

mkdir -p "$TEST_REPO/examples"
cp -a "$ROOT_DIR/examples/project" "$TEST_REPO/examples/project"
jq '.ops |= map(select(.op == "RenameDeclaration" or .op == "UpdateCallSites"))' \
  "$ROOT_DIR/fixtures/transactions/cross_module_preview.json" >"$OPERATION"
git -C "$TEST_REPO" init -q
git -C "$TEST_REPO" config user.name "Sley Apply Test"
git -C "$TEST_REPO" config user.email "sley-apply@example.invalid"
git -C "$TEST_REPO" add examples/project
git -C "$TEST_REPO" commit -qm "fixture base"

"$SLEY" change inspect --json \
  --goal "apply one cross-module structural edit" \
  --actor "operator:test" \
  --nonce "w4-apply-001" \
  --created-at "2026-08-25T00:00:00Z" \
  --expires-at "2099-01-01T00:00:00Z" \
  "$TARGET" >"$WORK_DIR/inspect.json"

"$SLEY" change plan --json \
  --repository "$TEST_REPO" \
  --inspection "$WORK_DIR/inspect.json" \
  --operation "$OPERATION" \
  --requested-outcome "rename the exported task and update its cross-module caller" \
  --assumption "the caller remains in app.main" \
  --non-goal "do not change source outside the exact candidate" \
  >"$WORK_DIR/plan.json"

"$SLEY" change preview --json --repository "$TEST_REPO" \
  --plan "$WORK_DIR/plan.json" >"$WORK_DIR/preview.json"
"$SLEY" change approval-request --json --repository "$TEST_REPO" \
  --preview "$WORK_DIR/preview.json" >"$WORK_DIR/request.json"
"$SLEY" change approve --json --repository "$TEST_REPO" \
  --request "$WORK_DIR/request.json" --preview "$WORK_DIR/preview.json" \
  --confirm-exact-local-grant \
  --issuer "operator:test" --principal "agent:test" --audience "sley:local" \
  --purpose "authorize only this immutable cross-module candidate" \
  --nonce "w4-apply-grant-001" \
  --created-at "2026-08-25T00:00:01Z" --expires-at "2098-01-01T00:00:00Z" \
  --ack-review "human_source_review" --ack-review "graph_diff_review" \
  --ack-review "lint_review" --ack-review "authority_review" \
  --revocation-state "not_revoked" --max-wall-clock-seconds "300" \
  >"$WORK_DIR/grant.json"
"$SLEY" change apply-authorization --json --repository "$TEST_REPO" \
  --grant "$WORK_DIR/grant.json" --request "$WORK_DIR/request.json" \
  --preview "$WORK_DIR/preview.json" --principal "agent:test" --audience "sley:local" \
  >"$WORK_DIR/authorization.json"

"$SLEY" change revocation-record --json --repository "$TEST_REPO" \
  --authorization "$WORK_DIR/authorization.json" --grant "$WORK_DIR/grant.json" \
  --state "not_revoked" --issuer "operator:test" --reason "grant remains locally active" \
  --created-at "2026-08-25T00:00:02Z" --expires-at "2098-01-01T00:00:00Z" \
  --confirm-revocation-record >"$WORK_DIR/revocation.json"

ledger_path="$(jq -r '.ledger_path' "$WORK_DIR/revocation.json")"
cmp -s "$WORK_DIR/revocation.json" "$TEST_REPO/$ledger_path" \
  || fail "revocation ledger does not contain the emitted exact record"
"$CONTRACT" validate --schema sley.change.apply_authorization.v0 "$WORK_DIR/authorization.json" --schemas "$SCHEMA_DIR" --json >/dev/null
"$CONTRACT" validate --schema sley.change.revocation_record.v0 "$WORK_DIR/revocation.json" --schemas "$SCHEMA_DIR" --json >/dev/null
jq -e '
  .status == "active"
  and .state == "not_revoked"
  and .issuer == {id:"operator:test",source:"explicit_cli_assertion"}
  and .mutations == {repository_source:false,git_index:false,trace_sidecars:false,external_systems:false,provider_state:false}
  and .internal_mutations == {internal_transaction_state:true}
' "$WORK_DIR/revocation.json" >/dev/null || fail "active revocation record is incomplete"

expect_diagnostic TRANSACTION_REVOCATION_RECORD_EXISTS change revocation-record --json --repository "$TEST_REPO" \
  --authorization "$WORK_DIR/authorization.json" --grant "$WORK_DIR/grant.json" \
  --state "not_revoked" --issuer "operator:test" --reason "duplicate initialization" \
  --created-at "2026-08-25T00:00:03Z" --expires-at "2098-01-01T00:00:00Z" \
  --confirm-revocation-record

index_path="$(git -C "$TEST_REPO" rev-parse --absolute-git-dir)/index"
index_before="$(sha256sum "$index_path" | awk '{print $1}')"
"$SLEY" change apply --json --repository "$TEST_REPO" \
  --authorization "$WORK_DIR/authorization.json" --grant "$WORK_DIR/grant.json" \
  --request "$WORK_DIR/request.json" --preview "$WORK_DIR/preview.json" \
  --principal "agent:test" --audience "sley:local" --confirm-atomic-apply \
  >"$WORK_DIR/apply.json"
"$CONTRACT" validate --schema sley.change.apply.v0 "$WORK_DIR/apply.json" --schemas "$SCHEMA_DIR" --json >/dev/null
recovery_path="$(jq -r '.recovery_ref.path' "$WORK_DIR/apply.json")"
"$CONTRACT" validate --schema sley.change.recovery.v0 "$TEST_REPO/$recovery_path" --schemas "$SCHEMA_DIR" --json >/dev/null
jq -e '
  .status == "verified"
  and .transaction.state == "verifying"
  and .replay.state == "consumed"
  and .verification.status == "passed"
  and .rollback.status == "not_required"
  and .mutations == {repository_source:true,git_index:false,trace_sidecars:false,external_systems:false,provider_state:false}
  and .internal_mutations == {internal_transaction_state:true}
' "$WORK_DIR/apply.json" >/dev/null || fail "verified atomic apply report is incomplete"
jq -e --arg digest "$(jq -r '.recovery_ref.record_digest' "$WORK_DIR/apply.json")" \
  '.phase == "verified" and .status == "terminal" and .record_digest == $digest' \
  "$TEST_REPO/$recovery_path" >/dev/null || fail "durable verified recovery record is incomplete"
cp "$TEST_REPO/$recovery_path" "$WORK_DIR/recovery-verified.json"
grep -q 'task scale' "$TARGET/src/app/math.sley" || fail "atomic apply did not install renamed declaration"
grep -q 'call math.scale(21)' "$TARGET/src/app/main.sley" || fail "atomic apply did not update the caller"
[[ "$index_before" == "$(sha256sum "$index_path" | awk '{print $1}')" ]] || fail "atomic apply changed the Git index"
expect_diagnostic TRANSACTION_REPLAY_DENIED change apply --json --repository "$TEST_REPO" \
  --authorization "$WORK_DIR/authorization.json" --grant "$WORK_DIR/grant.json" \
  --request "$WORK_DIR/request.json" --preview "$WORK_DIR/preview.json" \
  --principal "agent:test" --audience "sley:local" --confirm-atomic-apply

"$SLEY" change rollback --json --repository "$TEST_REPO" \
  --apply "$WORK_DIR/apply.json" --confirm-rollback >"$WORK_DIR/rollback.json"
"$CONTRACT" validate --schema sley.change.rollback.v0 "$WORK_DIR/rollback.json" --schemas "$SCHEMA_DIR" --json >/dev/null
jq -e '
  .status == "rolled_back"
  and .transaction.state == "rolled_back"
  and .verification.status == "passed"
  and .mutations == {repository_source:true,git_index:false,trace_sidecars:false,external_systems:false,provider_state:false}
  and .internal_mutations == {internal_transaction_state:true}
' "$WORK_DIR/rollback.json" >/dev/null || fail "manual rollback report is incomplete"
git -C "$TEST_REPO" diff --quiet -- examples/project || fail "manual rollback did not restore exact base source"
[[ "$index_before" == "$(sha256sum "$index_path" | awk '{print $1}')" ]] || fail "manual rollback changed the Git index"
expect_diagnostic TRANSACTION_RECOVERY_RECORD_MISMATCH change rollback --json --repository "$TEST_REPO" \
  --apply "$WORK_DIR/apply.json" --confirm-rollback

"$SLEY" change revocation-record --json --repository "$TEST_REPO" \
  --authorization "$WORK_DIR/authorization.json" --grant "$WORK_DIR/grant.json" \
  --state "revoked" --issuer "operator:test" --reason "operator revoked the exact grant" \
  --created-at "2026-08-25T00:00:04Z" --expires-at "2098-01-01T00:00:00Z" \
  --confirm-revocation-record >"$WORK_DIR/revoked.json"
"$CONTRACT" validate --schema sley.change.revocation_record.v0 "$WORK_DIR/revoked.json" --schemas "$SCHEMA_DIR" --json >/dev/null
jq -e '.status == "revoked" and .state == "revoked"' "$WORK_DIR/revoked.json" >/dev/null \
  || fail "revoked record is incomplete"
expect_diagnostic TRANSACTION_REVOCATION_IRREVERSIBLE change revocation-record --json --repository "$TEST_REPO" \
  --authorization "$WORK_DIR/authorization.json" --grant "$WORK_DIR/grant.json" \
  --state "not_revoked" --issuer "operator:test" --reason "attempt to restore authority" \
  --created-at "2026-08-25T00:00:05Z" --expires-at "2098-01-01T00:00:00Z" \
  --confirm-revocation-record

git -C "$TEST_REPO" diff --quiet -- examples/project || fail "manual rollback left repository source changed"
git -C "$TEST_REPO" diff --cached --quiet || fail "authorization or revocation changed the Git index"

ROLLBACK_REPO="$WORK_DIR/verification-rollback-repository"
ROLLBACK_CHAIN="$WORK_DIR/verification-rollback-chain"
ROLLBACK_OPERATION="$WORK_DIR/verification-rollback-operation.json"
prepare_repository "$ROLLBACK_REPO"
cp "$ROOT_DIR/fixtures/transactions/cross_module_preview.json" "$ROLLBACK_OPERATION"
build_apply_chain "$ROLLBACK_REPO" "$ROLLBACK_OPERATION" "$ROLLBACK_CHAIN" "w4-verification-rollback"
initialize_revocation "$ROLLBACK_REPO" "$ROLLBACK_CHAIN" "2026-08-25T00:00:02Z"
if "$SLEY" change apply --json --repository "$ROLLBACK_REPO" \
  --authorization "$ROLLBACK_CHAIN/authorization.json" --grant "$ROLLBACK_CHAIN/grant.json" \
  --request "$ROLLBACK_CHAIN/request.json" --preview "$ROLLBACK_CHAIN/preview.json" \
  --principal "agent:test" --audience "sley:local" --confirm-atomic-apply \
  >"$ROLLBACK_CHAIN/apply.json"; then
  fail "verification-failing candidate unexpectedly returned apply success"
fi
"$CONTRACT" validate --schema sley.change.apply.v0 "$ROLLBACK_CHAIN/apply.json" --schemas "$SCHEMA_DIR" --json >/dev/null
jq -e '
  .status == "rolled_back"
  and .transaction.state == "rolled_back"
  and .verification.status == "failed"
  and (.verification.checks[] | select(.kind == "compiler_lint") | .status) == "failed"
  and .rollback.status == "passed"
  and .replay.state == "consumed"
  and .issues == [{id:"TRANSACTION_VERIFICATION_FAILED",message:"required verification failed; automatic rollback passed"}]
' "$ROLLBACK_CHAIN/apply.json" >/dev/null || fail "automatic verification rollback evidence is incomplete"
git -C "$ROLLBACK_REPO" diff --quiet -- examples/project || fail "automatic verification rollback did not restore base source"
git -C "$ROLLBACK_REPO" diff --cached --quiet || fail "automatic verification rollback changed the Git index"

STALE_REPO="$WORK_DIR/stale-repository"
STALE_CHAIN="$WORK_DIR/stale-chain"
prepare_repository "$STALE_REPO"
build_apply_chain "$STALE_REPO" "$OPERATION" "$STALE_CHAIN" "w4-stale"
initialize_revocation "$STALE_REPO" "$STALE_CHAIN" "2026-08-25T00:00:02Z"
printf '\n// stale after approval\n' >>"$STALE_REPO/examples/project/src/app/math.sley"
stale_digest="$(sha256sum "$STALE_REPO/examples/project/src/app/math.sley" | awk '{print $1}')"
expect_diagnostic TRANSACTION_STALE change apply --json --repository "$STALE_REPO" \
  --authorization "$STALE_CHAIN/authorization.json" --grant "$STALE_CHAIN/grant.json" \
  --request "$STALE_CHAIN/request.json" --preview "$STALE_CHAIN/preview.json" \
  --principal "agent:test" --audience "sley:local" --confirm-atomic-apply
[[ "$stale_digest" == "$(sha256sum "$STALE_REPO/examples/project/src/app/math.sley" | awk '{print $1}')" ]] \
  || fail "stale-plan denial changed source"
[[ ! -e "$STALE_REPO/.sley/replay/$(jq -r '.grant_ref.replay_key | sub("^sha256:";"")' "$STALE_CHAIN/authorization.json")" ]] \
  || fail "stale-plan denial consumed replay"

REVOKED_REPO="$WORK_DIR/revoked-repository"
REVOKED_CHAIN="$WORK_DIR/revoked-chain"
prepare_repository "$REVOKED_REPO"
build_apply_chain "$REVOKED_REPO" "$OPERATION" "$REVOKED_CHAIN" "w4-revoked"
initialize_revocation "$REVOKED_REPO" "$REVOKED_CHAIN" "2026-08-25T00:00:02Z"
"$SLEY" change revocation-record --json --repository "$REVOKED_REPO" \
  --authorization "$REVOKED_CHAIN/authorization.json" --grant "$REVOKED_CHAIN/grant.json" \
  --state "revoked" --issuer "operator:test" --reason "deny before any source effect" \
  --created-at "2026-08-25T00:00:03Z" --expires-at "2098-01-01T00:00:00Z" \
  --confirm-revocation-record >"$REVOKED_CHAIN/revoked.json"
expect_diagnostic TRANSACTION_GRANT_REVOKED change apply --json --repository "$REVOKED_REPO" \
  --authorization "$REVOKED_CHAIN/authorization.json" --grant "$REVOKED_CHAIN/grant.json" \
  --request "$REVOKED_CHAIN/request.json" --preview "$REVOKED_CHAIN/preview.json" \
  --principal "agent:test" --audience "sley:local" --confirm-atomic-apply
git -C "$REVOKED_REPO" diff --quiet -- examples/project || fail "revoked apply changed source"
git -C "$REVOKED_REPO" diff --cached --quiet || fail "revoked apply changed the Git index"

TOPOLOGY_REPO="$WORK_DIR/topology-repository"
TOPOLOGY_CHAIN="$WORK_DIR/topology-chain"
prepare_repository "$TOPOLOGY_REPO"
build_apply_chain "$TOPOLOGY_REPO" "$OPERATION" "$TOPOLOGY_CHAIN" "w4-topology"
initialize_revocation "$TOPOLOGY_REPO" "$TOPOLOGY_CHAIN" "2026-08-25T00:00:02Z"

mv "$TOPOLOGY_REPO/examples/project/sley.toml" "$TOPOLOGY_REPO/examples/project/sley.toml.real"
ln -s "sley.toml.real" "$TOPOLOGY_REPO/examples/project/sley.toml"
expect_diagnostic TRANSACTION_ATOMIC_SYMLINK_UNSUPPORTED change apply --json --repository "$TOPOLOGY_REPO" \
  --authorization "$TOPOLOGY_CHAIN/authorization.json" --grant "$TOPOLOGY_CHAIN/grant.json" \
  --request "$TOPOLOGY_CHAIN/request.json" --preview "$TOPOLOGY_CHAIN/preview.json" \
  --principal "agent:test" --audience "sley:local" --confirm-atomic-apply
rm "$TOPOLOGY_REPO/examples/project/sley.toml"
mv "$TOPOLOGY_REPO/examples/project/sley.toml.real" "$TOPOLOGY_REPO/examples/project/sley.toml"

python3 - "$TOPOLOGY_REPO/examples/project/sley.toml" <<'PY'
import os
import sys
os.setxattr(sys.argv[1], "user.sley_test", b"denied")
PY
expect_diagnostic TRANSACTION_ATOMIC_XATTR_UNSUPPORTED change apply --json --repository "$TOPOLOGY_REPO" \
  --authorization "$TOPOLOGY_CHAIN/authorization.json" --grant "$TOPOLOGY_CHAIN/grant.json" \
  --request "$TOPOLOGY_CHAIN/request.json" --preview "$TOPOLOGY_CHAIN/preview.json" \
  --principal "agent:test" --audience "sley:local" --confirm-atomic-apply
python3 - "$TOPOLOGY_REPO/examples/project/sley.toml" <<'PY'
import os
import sys
os.removexattr(sys.argv[1], "user.sley_test")
PY

mkfifo "$TOPOLOGY_REPO/examples/project/runtime.pipe"
expect_diagnostic TRANSACTION_ATOMIC_SPECIAL_FILE_UNSUPPORTED change apply --json --repository "$TOPOLOGY_REPO" \
  --authorization "$TOPOLOGY_CHAIN/authorization.json" --grant "$TOPOLOGY_CHAIN/grant.json" \
  --request "$TOPOLOGY_CHAIN/request.json" --preview "$TOPOLOGY_CHAIN/preview.json" \
  --principal "agent:test" --audience "sley:local" --confirm-atomic-apply
rm "$TOPOLOGY_REPO/examples/project/runtime.pipe"

printf 'staged topology probe\n' >"$TOPOLOGY_REPO/examples/project/src/app/staged.txt"
git -C "$TOPOLOGY_REPO" add examples/project/src/app/staged.txt
expect_diagnostic TRANSACTION_GIT_INDEX_DIRTY change apply --json --repository "$TOPOLOGY_REPO" \
  --authorization "$TOPOLOGY_CHAIN/authorization.json" --grant "$TOPOLOGY_CHAIN/grant.json" \
  --request "$TOPOLOGY_CHAIN/request.json" --preview "$TOPOLOGY_CHAIN/preview.json" \
  --principal "agent:test" --audience "sley:local" --confirm-atomic-apply
git -C "$TOPOLOGY_REPO" restore --staged examples/project/src/app/staged.txt
rm "$TOPOLOGY_REPO/examples/project/src/app/staged.txt"
[[ ! -e "$TOPOLOGY_REPO/.sley/replay/$(jq -r '.grant_ref.replay_key | sub("^sha256:";"")' "$TOPOLOGY_CHAIN/authorization.json")" ]] \
  || fail "rejected topology consumed replay"

if SLEY_ALLOW_TEST_HOOKS=1 SLEY_TEST_CANCEL_BEFORE_COMMIT=1 "$SLEY" change apply --json --repository "$TOPOLOGY_REPO" \
  --authorization "$TOPOLOGY_CHAIN/authorization.json" --grant "$TOPOLOGY_CHAIN/grant.json" \
  --request "$TOPOLOGY_CHAIN/request.json" --preview "$TOPOLOGY_CHAIN/preview.json" \
  --principal "agent:test" --audience "sley:local" --confirm-atomic-apply \
  >"$TOPOLOGY_CHAIN/cancelled.json" 2>/dev/null; then
  fail "pre-commit cancellation unexpectedly returned apply success"
fi
jq -e '.diagnostics | any(.id == "TRANSACTION_CANCELLED")' "$TOPOLOGY_CHAIN/cancelled.json" >/dev/null \
  || fail "pre-commit cancellation did not emit its typed diagnostic"
cancelled_recovery="$(jq -r '.internal_scopes[] | select(.kind == "recovery_journal") | .path' "$TOPOLOGY_CHAIN/authorization.json")/record.json"
"$CONTRACT" validate --schema sley.change.recovery.v0 "$TOPOLOGY_REPO/$cancelled_recovery" --schemas "$SCHEMA_DIR" --json >/dev/null
jq -e '.phase == "cancelled" and .status == "terminal" and .replay.state == "available" and .mutations.repository_source == false' \
  "$TOPOLOGY_REPO/$cancelled_recovery" >/dev/null || fail "pre-commit cancellation recovery evidence is incomplete"
git -C "$TOPOLOGY_REPO" diff --quiet -- examples/project || fail "pre-commit cancellation changed source"

RECOVERY_REPO="$WORK_DIR/crash-recovery-repository"
RECOVERY_BEFORE="$WORK_DIR/crash-before-exchange"
RECOVERY_AFTER="$WORK_DIR/crash-after-exchange"
RECOVERY_ROLLBACK="$WORK_DIR/crash-before-rollback"
prepare_repository "$RECOVERY_REPO"

build_apply_chain "$RECOVERY_REPO" "$OPERATION" "$RECOVERY_BEFORE" "w4-crash-before"
initialize_revocation "$RECOVERY_REPO" "$RECOVERY_BEFORE" "2026-08-25T00:00:02Z"
crash_apply_at "$RECOVERY_REPO" "$RECOVERY_BEFORE" "before_exchange"
before_recovery_path="$(jq -r '.internal_scopes[] | select(.kind == "recovery_journal") | .path' "$RECOVERY_BEFORE/authorization.json")/record.json"
jq -e '.phase == "commit_point_entered" and .status == "active" and .replay.state == "consumed"' \
  "$RECOVERY_REPO/$before_recovery_path" >/dev/null || fail "before-exchange crash evidence is incomplete"
git -C "$RECOVERY_REPO" diff --quiet -- examples/project || fail "before-exchange crash changed source before recovery"
before_replay_path="$(jq -r '.internal_scopes[] | select(.kind == "replay_ledger") | .path' "$RECOVERY_BEFORE/authorization.json")"
cp "$RECOVERY_REPO/$before_replay_path" "$RECOVERY_BEFORE/replay-marker.json"
jq '.grant_digest="sha256:ffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffff"' \
  "$RECOVERY_BEFORE/replay-marker.json" >"$RECOVERY_REPO/$before_replay_path"
expect_diagnostic TRANSACTION_REPLAY_MARKER_MISMATCH change recover --json --repository "$RECOVERY_REPO" \
  --authorization "$RECOVERY_BEFORE/authorization.json" --grant "$RECOVERY_BEFORE/grant.json" \
  --request "$RECOVERY_BEFORE/request.json" --preview "$RECOVERY_BEFORE/preview.json" \
  --principal "agent:test" --audience "sley:local" --confirm-recovery
cp "$RECOVERY_BEFORE/replay-marker.json" "$RECOVERY_REPO/$before_replay_path"
python3 - "$RECOVERY_REPO/examples/project/sley.toml" <<'PY'
import os
import sys
os.setxattr(sys.argv[1], "user.sley_recovery_test", b"denied")
PY
expect_diagnostic TRANSACTION_ATOMIC_XATTR_UNSUPPORTED change recover --json --repository "$RECOVERY_REPO" \
  --authorization "$RECOVERY_BEFORE/authorization.json" --grant "$RECOVERY_BEFORE/grant.json" \
  --request "$RECOVERY_BEFORE/request.json" --preview "$RECOVERY_BEFORE/preview.json" \
  --principal "agent:test" --audience "sley:local" --confirm-recovery
python3 - "$RECOVERY_REPO/examples/project/sley.toml" <<'PY'
import os
import sys
os.removexattr(sys.argv[1], "user.sley_recovery_test")
PY
recover_apply "$RECOVERY_REPO" "$RECOVERY_BEFORE" "verified"
grep -q 'call math.scale(21)' "$RECOVERY_REPO/examples/project/src/app/main.sley" \
  || fail "before-exchange recovery did not complete the candidate"
cp "$RECOVERY_REPO/$before_recovery_path" "$RECOVERY_BEFORE/recovery-verified.json"
"$SLEY" change rollback --json --repository "$RECOVERY_REPO" \
  --apply "$RECOVERY_BEFORE/recovered-apply.json" --confirm-rollback >"$RECOVERY_BEFORE/rollback.json"
git -C "$RECOVERY_REPO" diff --quiet -- examples/project || fail "rollback after before-exchange recovery did not restore base"

build_apply_chain "$RECOVERY_REPO" "$OPERATION" "$RECOVERY_AFTER" "w4-crash-after"
initialize_revocation "$RECOVERY_REPO" "$RECOVERY_AFTER" "2026-08-25T00:00:02Z"
crash_apply_at "$RECOVERY_REPO" "$RECOVERY_AFTER" "after_exchange"
after_recovery_path="$(jq -r '.internal_scopes[] | select(.kind == "recovery_journal") | .path' "$RECOVERY_AFTER/authorization.json")/record.json"
jq -e '.phase == "commit_point_entered" and .status == "active"' \
  "$RECOVERY_REPO/$after_recovery_path" >/dev/null || fail "after-exchange crash evidence is incomplete"
grep -q 'call math.scale(21)' "$RECOVERY_REPO/examples/project/src/app/main.sley" \
  || fail "after-exchange crash did not leave the exact candidate live"
recover_apply "$RECOVERY_REPO" "$RECOVERY_AFTER" "verified"
"$SLEY" change rollback --json --repository "$RECOVERY_REPO" \
  --apply "$RECOVERY_AFTER/recovered-apply.json" --confirm-rollback >"$RECOVERY_AFTER/rollback.json"
git -C "$RECOVERY_REPO" diff --quiet -- examples/project || fail "rollback after after-exchange recovery did not restore base"

cp "$ROOT_DIR/fixtures/transactions/cross_module_preview.json" "$WORK_DIR/crash-rollback-operation.json"
build_apply_chain "$RECOVERY_REPO" "$WORK_DIR/crash-rollback-operation.json" "$RECOVERY_ROLLBACK" "w4-crash-rollback"
initialize_revocation "$RECOVERY_REPO" "$RECOVERY_ROLLBACK" "2026-08-25T00:00:02Z"
crash_apply_at "$RECOVERY_REPO" "$RECOVERY_ROLLBACK" "before_rollback"
rollback_recovery_path="$(jq -r '.internal_scopes[] | select(.kind == "recovery_journal") | .path' "$RECOVERY_ROLLBACK/authorization.json")/record.json"
jq -e '.phase == "rollback_required" and .status == "active" and .verification.status == "failed"' \
  "$RECOVERY_REPO/$rollback_recovery_path" >/dev/null || fail "before-rollback crash evidence is incomplete"
recover_apply "$RECOVERY_REPO" "$RECOVERY_ROLLBACK" "rolled_back"
git -C "$RECOVERY_REPO" diff --quiet -- examples/project || fail "crash recovery rollback did not restore base"
git -C "$RECOVERY_REPO" diff --cached --quiet || fail "crash recovery changed the Git index"

echo "change apply authorization, atomic commit, cancellation, crash recovery, and verified rollback contracts passed"
