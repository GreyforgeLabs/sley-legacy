#!/usr/bin/env bash
set -euo pipefail

ROOT_DIR="$(cd "$(dirname "$0")/.." && pwd)"
BRIDGE="$ROOT_DIR/bin/sley-mcp-bridge"
WORK_DIR="$(mktemp -d "$ROOT_DIR/.sley-mcp-test.XXXXXXXX")"
trap 'rm -rf -- "$WORK_DIR"' EXIT

fail() {
  printf 'sley MCP bridge test failed: %s\n' "$*" >&2
  exit 1
}

initialize_message() {
  jq -cn '{jsonrpc:"2.0",id:1,method:"initialize",params:{protocolVersion:"2025-06-18",capabilities:{},clientInfo:{name:"sley-mcp-test",version:"1"}}}'
}

initialized_message() {
  jq -cn '{jsonrpc:"2.0",method:"notifications/initialized"}'
}

run_bridge() {
  local name="$1"
  shift
  "$@" "$BRIDGE" --root "$ROOT_DIR" > "$WORK_DIR/$name.out" 2> "$WORK_DIR/$name.err"
}

[[ -x "$BRIDGE" ]] || fail "bridge is not executable"
bash -n "$BRIDGE"
if "$BRIDGE" --root . </dev/null > "$WORK_DIR/relative-root.out" 2> "$WORK_DIR/relative-root.err"; then
  fail "bridge accepted a relative repository root"
fi
if "$BRIDGE" --root "$ROOT_DIR/examples" </dev/null > "$WORK_DIR/subdir-root.out" 2> "$WORK_DIR/subdir-root.err"; then
  fail "bridge accepted a Git repository subdirectory as its root"
fi

{
  initialize_message
  initialized_message
  jq -cn '{jsonrpc:"2.0",id:2,method:"tools/list",params:{}}'
} | run_bridge lifecycle env

jq -s -e '
  length == 2 and
  .[0].result.protocolVersion == "2025-06-18" and
  .[0].result.capabilities.tools.listChanged == false and
  ([.[1].result.tools[].name] == ["sley_query","sley_lint","sley_plan","sley_propose_graft","sley_verify","sley_change_inspect","sley_change_plan","sley_change_preview","sley_change_approval_request","sley_change_apply_authorization","sley_change_apply","sley_change_recover","sley_change_rollback","sley_change_review"]) and
  ([.[1].result.tools[].inputSchema."$schema"] | all(. == "https://json-schema.org/draft/2020-12/schema")) and
  ([.[1].result.tools[].outputSchema."$schema"] | all(. == "https://json-schema.org/draft/2020-12/schema")) and
  (.[1].result.tools[] | select(.name == "sley_propose_graft") | .inputSchema.properties.operation.properties.op.enum | index("InsertStatement")) != null and
  ([.[1].result.tools[] | select(.name | IN("sley_change_apply","sley_change_recover","sley_change_rollback")) | .annotations.destructiveHint] | all(. == true)) and
  ([.[1].result.tools[] | select(.name | IN("sley_change_inspect","sley_change_plan","sley_change_preview","sley_change_approval_request","sley_change_apply_authorization")) | .annotations.readOnlyHint] | all(. == true)) and
  ([.[1].result.tools[].name] | index("sley_change_approve") == null) and
  ([.[1].result.tools[].name] | index("sley_change_revocation_record") == null)
' "$WORK_DIR/lifecycle.out" >/dev/null || fail "lifecycle or tool catalog mismatch"
[[ "$(wc -l < "$WORK_DIR/lifecycle.out" | tr -d '[:space:]')" -eq 2 ]] || fail "stdout framing emitted an unexpected line count"
while IFS= read -r line; do jq -e '.jsonrpc == "2.0"' <<< "$line" >/dev/null || fail "stdout contained a non-protocol line"; done < "$WORK_DIR/lifecycle.out"

operation_json="$(jq -c . "$ROOT_DIR/fixtures/ci_smoke_probe/insert_statement.json")"
graft_target_digest_before="$(sha256sum "$ROOT_DIR/fixtures/ci_smoke_probe/graft_target.sley" | awk '{print $1}')"
{
  initialize_message
  initialized_message
  jq -cn '{jsonrpc:"2.0",id:10,method:"tools/call",params:{name:"sley_query",arguments:{path:"examples/hello.sley",kind:"tasks"}}}'
  jq -cn '{jsonrpc:"2.0",id:11,method:"tools/call",params:{name:"sley_lint",arguments:{path:"examples/hello.sley"}}}'
  jq -cn '{jsonrpc:"2.0",id:12,method:"tools/call",params:{name:"sley_plan",arguments:{path:"examples/collections.sley"}}}'
  jq -cn --argjson operation "$operation_json" '{jsonrpc:"2.0",id:13,method:"tools/call",params:{name:"sley_propose_graft",arguments:{path:"fixtures/ci_smoke_probe/graft_target.sley",operation:$operation}}}'
  jq -cn '{jsonrpc:"2.0",id:14,method:"tools/call",params:{name:"sley_verify",arguments:{path:"examples/hello.sley"}}}'
} | run_bridge calls env

jq -s -e '
  length == 6 and
  .[1].result.structuredContent.schema == "sley.query.report.v0" and
  .[2].result.structuredContent.schema == "sley.lint.report.v0" and
  .[3].result.structuredContent.schema == "sley.edit_plan.report.v0" and
  .[4].result.structuredContent.schema == "sley.graft.outcome.v0" and
  .[5].result.structuredContent.schema == "sley.verify.report.v0" and
  ([.[1:][] | .result.content[0].text | fromjson | .schema] == ["sley.query.report.v0","sley.lint.report.v0","sley.edit_plan.report.v0","sley.graft.outcome.v0","sley.verify.report.v0"]) and
  ([.[1:][] | .result.isError] | all(. == false))
' "$WORK_DIR/calls.out" >/dev/null || fail "read-only tool call contract mismatch"
graft_target_digest_after="$(sha256sum "$ROOT_DIR/fixtures/ci_smoke_probe/graft_target.sley" | awk '{print $1}')"
[[ "$graft_target_digest_before" == "$graft_target_digest_after" ]] || fail "dry-run graft changed its source target"

injection_sentinel="$WORK_DIR/argument-was-evaluated"
injection_argument="\$(touch $injection_sentinel)"
{
  initialize_message
  initialized_message
  jq -cn --arg module "$injection_argument" '{jsonrpc:"2.0",id:15,method:"tools/call",params:{name:"sley_query",arguments:{path:"examples/hello.sley",module:$module}}}'
} | run_bridge injection env
[[ ! -e "$injection_sentinel" ]] || fail "tool argument was evaluated as shell source"
jq -s -e 'length == 2 and .[1].jsonrpc == "2.0" and .[1].id == 15' "$WORK_DIR/injection.out" >/dev/null || fail "injection probe disrupted protocol output"

{
  printf '%s\n' '{not-json'
  printf '%s\n' '[]'
  jq -cn '{jsonrpc:"2.0",method:"tools/list",params:{}}'
  jq -cn '{jsonrpc:"2.0",id:20,method:"tools/list",params:{}}'
  initialize_message
  initialize_message
  initialized_message
  jq -cn '{jsonrpc:"2.0",id:21,method:"tools/list",params:[]}'
  jq -cn '{jsonrpc:"2.0",id:22,method:"tools/call",params:{name:"sley_query",arguments:{path:"examples/hello.sley",module:""}}}'
  jq -cn '{jsonrpc:"2.0",id:true,method:"ping",params:{}}'
  jq -cn '{jsonrpc:"2.0",id:23,method:"missing/method",params:{}}'
} | run_bridge malformed env

jq -s -e '
  length == 10 and
  .[0].error.code == -32700 and
  .[1].error.code == -32600 and
  .[2].error.code == -32600 and
  .[3].error.code == -32002 and
  .[4].result.protocolVersion == "2025-06-18" and
  .[5].error.code == -32600 and
  .[6].error.code == -32602 and
  .[7].error.code == -32602 and
  .[8].error.code == -32600 and
  .[8].id == null and
  .[9].error.code == -32601
' "$WORK_DIR/malformed.out" >/dev/null || fail "malformed input handling mismatch"

ln -s /etc "$WORK_DIR/outside-link"
relative_work_dir="${WORK_DIR#"$ROOT_DIR/"}"
{
  initialize_message
  initialized_message
  jq -cn '{jsonrpc:"2.0",id:30,method:"tools/call",params:{name:"sley_query",arguments:{path:"../etc/passwd"}}}'
  jq -cn '{jsonrpc:"2.0",id:31,method:"tools/call",params:{name:"sley_query",arguments:{path:"/etc/passwd"}}}'
  jq -cn --arg path "$relative_work_dir/outside-link/passwd" '{jsonrpc:"2.0",id:32,method:"tools/call",params:{name:"sley_query",arguments:{path:$path}}}'
  jq -cn --arg path $'examples/hello.sley\n../etc/passwd' '{jsonrpc:"2.0",id:33,method:"tools/call",params:{name:"sley_query",arguments:{path:$path}}}'
} | run_bridge paths env

jq -s -e 'length == 5 and ([.[1:][] | .result.isError] | all(. == true)) and ([.[1:][] | .result.content[0].text | startswith("SLEY_MCP_PATH_DENIED:")] | all)' "$WORK_DIR/paths.out" >/dev/null || fail "path confinement mismatch"

{
  initialize_message
  initialized_message
  jq -cn '{jsonrpc:"2.0",method:"notifications/cancelled",params:{requestId:99,reason:"test cancellation"}}'
  jq -cn '{jsonrpc:"2.0",id:40,method:"ping",params:{}}'
} | run_bridge cancellation env
jq -s -e 'length == 2 and .[1].id == 40 and .[1].result == {}' "$WORK_DIR/cancellation.out" >/dev/null || fail "cancellation notification disrupted the session"
grep -Fq 'ignored cancellation for inactive request 99: test cancellation' "$WORK_DIR/cancellation.err" || fail "cancellation was not logged to stderr"

PYTHONDONTWRITEBYTECODE=1 "$ROOT_DIR/scripts/test-sley-mcp-cancellation.sh" "$ROOT_DIR" "$WORK_DIR"

{
  initialize_message
  initialized_message
  jq -cn '{jsonrpc:"2.0",id:50,method:"tools/call",params:{name:"sley_query",arguments:{path:"examples/project"}}}'
} | run_bridge response_limit env SLEY_MCP_MAX_RESPONSE_BYTES=64
jq -s -e '.[1].result.isError == true and (.[1].result.content[0].text | startswith("SLEY_MCP_RESPONSE_TOO_LARGE:"))' "$WORK_DIR/response_limit.out" >/dev/null || fail "large response limit mismatch"

initialize_message | run_bridge request_limit env SLEY_MCP_MAX_REQUEST_BYTES=64
jq -e '.error.code == -32600 and (.error.message | contains("request exceeds"))' "$WORK_DIR/request_limit.out" >/dev/null || fail "large request limit mismatch"

{
  printf '%080d\n' 0
  jq -cn '{jsonrpc:"2.0",id:51,method:"ping",params:{}}'
} | run_bridge request_limit_recovery env SLEY_MCP_MAX_REQUEST_BYTES=64
jq -s -e 'length == 2 and .[0].error.code == -32600 and .[1].id == 51 and .[1].result == {}' "$WORK_DIR/request_limit_recovery.out" >/dev/null || fail "large request frame recovery mismatch"

if SLEY_MCP_MAX_REQUEST_BYTES=67108865 "$BRIDGE" --root "$ROOT_DIR" </dev/null > "$WORK_DIR/request_limit_cap.out" 2> "$WORK_DIR/request_limit_cap.err"; then
  fail "bridge accepted a request limit above its configured ceiling"
fi
grep -Fq 'must not exceed 67108864' "$WORK_DIR/request_limit_cap.err" || fail "request limit ceiling diagnostic mismatch"

{
  initialize_message
  initialized_message
  jq -cn '{jsonrpc:"2.0",id:60,method:"tools/call",params:{name:"sley_query",arguments:{path:"self-hosted"}}}'
} | run_bridge timeout env SLEY_DISABLE_SOURCE_CACHE=1 SLEY_MCP_TOOL_TIMEOUT_SECONDS=1
jq -s -e '.[1].result.isError == true and (.[1].result.content[0].text | startswith("SLEY_MCP_TOOL_TIMEOUT:"))' "$WORK_DIR/timeout.out" >/dev/null || fail "tool timeout mismatch"

printf 'Sley MCP bridge tests passed.\n'
