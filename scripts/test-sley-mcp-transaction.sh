#!/usr/bin/env bash
set -euo pipefail

ROOT_DIR="$(cd "$(dirname "$0")/.." && pwd)"
SLEY="$ROOT_DIR/bin/sley"
BRIDGE="$ROOT_DIR/bin/sley-mcp-bridge"
WORK_DIR="$(mktemp -d)"
TEST_REPO="$WORK_DIR/repository"
EVIDENCE="$TEST_REPO/.sley-mcp-evidence"

cleanup() {
  local rc=$?
  if [[ "${SLEY_KEEP_TEST_WORK_DIR:-0}" == "1" ]]; then
    printf 'Sley MCP transaction test artifacts retained at %s\n' "$WORK_DIR" >&2
  else
    rm -rf -- "$WORK_DIR"
  fi
  exit "$rc"
}
trap cleanup EXIT

fail() {
  printf 'Sley MCP transaction parity failed: %s\n' "$*" >&2
  exit 1
}

initialize_message() {
  jq -cn '{jsonrpc:"2.0",id:1,method:"initialize",params:{protocolVersion:"2025-06-18",capabilities:{},clientInfo:{name:"sley-mcp-transaction-test",version:"1"}}}'
}

initialized_message() {
  jq -cn '{jsonrpc:"2.0",method:"notifications/initialized"}'
}

run_mcp_call() {
  local name="$1" tool="$2" args_json="$3"
  {
    initialize_message
    initialized_message
    jq -cn --arg tool "$tool" --argjson args "$args_json" '{jsonrpc:"2.0",id:2,method:"tools/call",params:{name:$tool,arguments:$args}}'
  } | SLEY_MCP_TOOL_TIMEOUT_SECONDS=600 "$BRIDGE" --root "$TEST_REPO" \
    >"$WORK_DIR/$name.response" 2>"$WORK_DIR/$name.stderr"
  jq -s -e 'length == 2 and .[1].jsonrpc == "2.0" and .[1].id == 2 and .[1].result.content[0].type == "text"' \
    "$WORK_DIR/$name.response" >/dev/null || fail "$name did not return one framed MCP tool result"
}

extract_success() {
  local name="$1" output="$2" schema="$3"
  jq -s -e --arg schema "$schema" '
    .[1].result.isError == false
    and .[1].result.structuredContent.schema == $schema
    and (.[1].result.content[0].text | fromjson) == .[1].result.structuredContent
  ' "$WORK_DIR/$name.response" >/dev/null || fail "$name did not preserve the exact CLI success report"
  jq -s -c '.[1].result.structuredContent' "$WORK_DIR/$name.response" >"$output"
}

expect_structured_diagnostic() {
  local name="$1" diagnostic_id="$2"
  jq -s -e --arg id "$diagnostic_id" '
    .[1].result.isError == true
    and .[1].result.structuredContent.schema == "sley.diagnostics.report.v0"
    and (.[1].result.content[0].text | fromjson) == .[1].result.structuredContent
    and any(.[1].result.structuredContent.diagnostics[]; .id == $id and .severity == "error")
  ' "$WORK_DIR/$name.response" >/dev/null || fail "$name did not preserve diagnostic $diagnostic_id"
}

canonical_equal() {
  local left="$1" right="$2" label="$3"
  cmp -s <(jq -cS . "$left") <(jq -cS . "$right") || fail "$label diverged between CLI and MCP"
}

mkdir -p "$TEST_REPO/examples"
cp -a "$ROOT_DIR/examples/project" "$TEST_REPO/examples/project"
git -C "$TEST_REPO" init -q
git -C "$TEST_REPO" config user.name "Sley MCP Transaction Test"
git -C "$TEST_REPO" config user.email "sley-mcp-transaction@example.invalid"
git -C "$TEST_REPO" add examples/project
git -C "$TEST_REPO" commit -qm "fixture base"
mkdir -p "$EVIDENCE"

jq '.ops |= map(select(.op == "RenameDeclaration" or .op == "UpdateCallSites"))' \
  "$ROOT_DIR/fixtures/transactions/cross_module_preview.json" >"$EVIDENCE/operation.json"

source_before="$(find "$TEST_REPO/examples/project" -type f -print0 | sort -z | xargs -0 sha256sum | sha256sum | awk '{print $1}')"
index_path="$(git -C "$TEST_REPO" rev-parse --absolute-git-dir)/index"
index_before="$(sha256sum "$index_path" | awk '{print $1}')"

inspect_args="$(jq -cn '{path:"examples/project",goal:"complete one bounded MCP transaction",actor:"agent:mcp-test",nonce:"mcp-parity-001",created_at:"2026-08-25T00:00:00Z",expires_at:"2099-01-01T00:00:00Z"}')"
run_mcp_call inspect sley_change_inspect "$inspect_args"
extract_success inspect "$EVIDENCE/inspect.json" sley.transaction.inspect.v0
"$SLEY" change inspect --json --goal "complete one bounded MCP transaction" --actor "agent:mcp-test" \
  --nonce "mcp-parity-001" --created-at "2026-08-25T00:00:00Z" --expires-at "2099-01-01T00:00:00Z" \
  "$TEST_REPO/examples/project" >"$EVIDENCE/inspect-cli.json"
canonical_equal "$EVIDENCE/inspect-cli.json" "$EVIDENCE/inspect.json" "transaction inspection"

plan_args="$(jq -cn '{inspection:".sley-mcp-evidence/inspect.json",operation:".sley-mcp-evidence/operation.json",requested_outcome:"rename the exported task and update its caller",assumptions:["the inspected base remains current"],non_goals:["do not change source outside the candidate"]}')"
run_mcp_call plan sley_change_plan "$plan_args"
extract_success plan "$EVIDENCE/plan.json" sley.change.plan.v0
"$SLEY" change plan --json --repository "$TEST_REPO" --inspection "$EVIDENCE/inspect.json" \
  --operation "$EVIDENCE/operation.json" --requested-outcome "rename the exported task and update its caller" \
  --assumption "the inspected base remains current" --non-goal "do not change source outside the candidate" \
  >"$EVIDENCE/plan-cli.json"
canonical_equal "$EVIDENCE/plan-cli.json" "$EVIDENCE/plan.json" "transaction plan"

preview_args='{"plan":".sley-mcp-evidence/plan.json"}'
run_mcp_call preview sley_change_preview "$preview_args"
extract_success preview "$EVIDENCE/preview.json" sley.change.preview.v0
"$SLEY" change preview --json --repository "$TEST_REPO" --plan "$EVIDENCE/plan.json" >"$EVIDENCE/preview-cli.json"
canonical_equal "$EVIDENCE/preview-cli.json" "$EVIDENCE/preview.json" "immutable preview"

request_args='{"preview":".sley-mcp-evidence/preview.json"}'
run_mcp_call request sley_change_approval_request "$request_args"
extract_success request "$EVIDENCE/request.json" sley.change.approval_request.v0
"$SLEY" change approval-request --json --repository "$TEST_REPO" --preview "$EVIDENCE/preview.json" >"$EVIDENCE/request-cli.json"
canonical_equal "$EVIDENCE/request-cli.json" "$EVIDENCE/request.json" "approval request"

"$SLEY" change approve --json --repository "$TEST_REPO" \
  --request "$EVIDENCE/request.json" --preview "$EVIDENCE/preview.json" --confirm-exact-local-grant \
  --issuer "operator:test" --principal "agent:mcp-test" --audience "sley:mcp-local" \
  --purpose "authorize only this immutable MCP candidate" --nonce "mcp-grant-001" \
  --created-at "2026-08-25T00:00:01Z" --expires-at "2098-01-01T00:00:00Z" \
  --ack-review "human_source_review" --ack-review "graph_diff_review" \
  --ack-review "lint_review" --ack-review "authority_review" \
  --revocation-state "not_revoked" --max-wall-clock-seconds "600" >"$EVIDENCE/grant.json"

authorization_args="$(jq -cn '{grant:".sley-mcp-evidence/grant.json",request:".sley-mcp-evidence/request.json",preview:".sley-mcp-evidence/preview.json",principal:"agent:mcp-test",audience:"sley:mcp-local"}')"
run_mcp_call authorization sley_change_apply_authorization "$authorization_args"
extract_success authorization "$EVIDENCE/authorization.json" sley.change.apply_authorization.v0
"$SLEY" change apply-authorization --json --repository "$TEST_REPO" --grant "$EVIDENCE/grant.json" \
  --request "$EVIDENCE/request.json" --preview "$EVIDENCE/preview.json" \
  --principal "agent:mcp-test" --audience "sley:mcp-local" >"$EVIDENCE/authorization-cli.json"
canonical_equal "$EVIDENCE/authorization-cli.json" "$EVIDENCE/authorization.json" "apply authorization"

[[ "$source_before" == "$(find "$TEST_REPO/examples/project" -type f -print0 | sort -z | xargs -0 sha256sum | sha256sum | awk '{print $1}')" ]] \
  || fail "read-only MCP transaction tools changed source"
[[ "$index_before" == "$(sha256sum "$index_path" | awk '{print $1}')" ]] || fail "read-only MCP transaction tools changed the Git index"

"$SLEY" change revocation-record --json --repository "$TEST_REPO" \
  --authorization "$EVIDENCE/authorization.json" --grant "$EVIDENCE/grant.json" \
  --state "not_revoked" --issuer "operator:test" --reason "operator keeps the exact MCP grant active" \
  --created-at "2026-08-25T00:00:02Z" --expires-at "2098-01-01T00:00:00Z" \
  --confirm-revocation-record >"$EVIDENCE/revocation.json"

apply_args_false="$(jq -cn '{authorization:".sley-mcp-evidence/authorization.json",grant:".sley-mcp-evidence/grant.json",request:".sley-mcp-evidence/request.json",preview:".sley-mcp-evidence/preview.json",principal:"agent:mcp-test",audience:"sley:mcp-local",confirm_atomic_apply:false}')"
run_mcp_call apply_unconfirmed sley_change_apply "$apply_args_false"
expect_structured_diagnostic apply_unconfirmed TRANSACTION_ATOMIC_APPLY_CONFIRMATION_REQUIRED
[[ "$source_before" == "$(find "$TEST_REPO/examples/project" -type f -print0 | sort -z | xargs -0 sha256sum | sha256sum | awk '{print $1}')" ]] \
  || fail "unconfirmed MCP apply changed source"

ln -s "$EVIDENCE/plan.json" "$EVIDENCE/plan-link.json"
run_mcp_call symlink_plan sley_change_preview '{"plan":".sley-mcp-evidence/plan-link.json"}'
jq -s -e '.[1].result.isError == true and (.[1].result.content[0].text | startswith("SLEY_MCP_PATH_DENIED:")) and (.[1].result | has("structuredContent") | not)' \
  "$WORK_DIR/symlink_plan.response" >/dev/null || fail "MCP accepted a symlink transaction artifact"

run_mcp_call outside_plan sley_change_preview '{"plan":"/etc/passwd"}'
jq -s -e '.[1].result.isError == true and (.[1].result.content[0].text | startswith("SLEY_MCP_PATH_DENIED:"))' \
  "$WORK_DIR/outside_plan.response" >/dev/null || fail "MCP accepted an outside-root transaction artifact"

apply_args="$(printf '%s\n' "$apply_args_false" | jq -c '.confirm_atomic_apply=true')"
run_mcp_call apply sley_change_apply "$apply_args"
extract_success apply "$EVIDENCE/apply.json" sley.change.apply.v0
jq -e '.status == "verified" and .verification.status == "passed"' "$EVIDENCE/apply.json" >/dev/null \
  || fail "MCP apply did not preserve verified CLI semantics"
[[ "$source_before" != "$(find "$TEST_REPO/examples/project" -type f -print0 | sort -z | xargs -0 sha256sum | sha256sum | awk '{print $1}')" ]] \
  || fail "confirmed MCP apply did not change authorized source"
[[ "$index_before" == "$(sha256sum "$index_path" | awk '{print $1}')" ]] || fail "MCP apply changed the Git index"

run_mcp_call verify sley_verify '{"path":"examples/project"}'
extract_success verify "$EVIDENCE/verify.json" sley.verify.report.v0
jq -e '.status == "passed"' "$EVIDENCE/verify.json" >/dev/null || fail "post-apply MCP verification failed"

recover_args="$(jq -cn '{authorization:".sley-mcp-evidence/authorization.json",grant:".sley-mcp-evidence/grant.json",request:".sley-mcp-evidence/request.json",preview:".sley-mcp-evidence/preview.json",principal:"agent:mcp-test",audience:"sley:mcp-local",confirm_recovery:false}')"
run_mcp_call recover_unconfirmed sley_change_recover "$recover_args"
expect_structured_diagnostic recover_unconfirmed TRANSACTION_RECOVERY_CONFIRMATION_REQUIRED

review_args_false="$(jq -cn '{authorization:".sley-mcp-evidence/authorization.json",grant:".sley-mcp-evidence/grant.json",request:".sley-mcp-evidence/request.json",preview:".sley-mcp-evidence/preview.json",apply:".sley-mcp-evidence/apply.json",principal:"agent:mcp-test",audience:"sley:mcp-local",confirm_review_packet:false}')"
run_mcp_call review_unconfirmed sley_change_review "$review_args_false"
expect_structured_diagnostic review_unconfirmed TRANSACTION_REVIEW_CONFIRMATION_REQUIRED

review_args="$(printf '%s\n' "$review_args_false" | jq -c '.confirm_review_packet=true')"
run_mcp_call review sley_change_review "$review_args"
extract_success review "$EVIDENCE/review.json" sley.change.review.v0
transaction_hex="$(jq -r '.transaction.transaction_id | sub("^sha256:";"")' "$EVIDENCE/review.json")"
review_dir="$TEST_REPO/.sley/transactions/$transaction_hex/review"
[[ "$(find "$review_dir" -mindepth 1 -maxdepth 1 -type f -printf '%f\n' | sort | tr '\n' ' ')" == \
  "bundle.json evidence.zjx.json packet.md seal.json " ]] || fail "MCP review did not publish the exact terminal artifact set"

rollback_args="$(jq -cn '{apply:".sley-mcp-evidence/apply.json",confirm_rollback:true}')"
run_mcp_call sealed_rollback sley_change_rollback "$rollback_args"
expect_structured_diagnostic sealed_rollback TRANSACTION_ALREADY_SEALED

printf 'Sley MCP governed transaction parity passed.\n'
