#!/usr/bin/env bash
set -euo pipefail

ROOT_DIR="$(cd "$(dirname "$0")/.." && pwd)"
CONTRACT="$ROOT_DIR/bin/sley-contract"
SCHEMA_DIR="$ROOT_DIR/docs/schemas"
BASE_FIXTURE="$ROOT_DIR/fixtures/contracts/agent_bench_protocol_replay_nested_owned_path.json"
REPLAY_DIR="$ROOT_DIR/fixtures/sleybench/protocol-v0"
WORK_DIR="$(mktemp -d)"
trap 'rm -rf "$WORK_DIR"' EXIT

fail() {
  echo "SleyBench protocol replay failed: $*" >&2
  exit 1
}

validate_replay() {
  "$CONTRACT" validate \
    --schema sley.agent_bench.protocol_replay.v0 "$1" \
    --schemas "$SCHEMA_DIR" --json >/dev/null
}

semantic_result() {
  jq -cr '
    . as $replay
    | ([range(0; (.turns | length)) as $index
        | .turns[$index].number == ($index + 1)] | all) as $ordered
    | ([.turns[].action | select(.kind == "tool")] | length) as $tool_calls
    | ([.turns[].input_bytes] | add // 0) as $input_bytes
    | ([.turns[].output_bytes] | add // 0) as $output_bytes
    | ([.turns[].context_nodes] | max // 0) as $context_nodes
    | ([.turns[].elapsed_ms] | max // 0) as $wall_ms
    | ([.turns[].action
        | select(.kind == "tool" and .target != ".")
        | .target as $path
        | select(($replay.owned_paths | index($path)) == null)] | length) as $unowned_tools
    | ([.turns[].action
        | select(.kind == "submit")
        | .files | keys[] as $path
        | select(($replay.owned_paths | index($path)) == null)
        | $path] | length) as $unowned_files
    | ([.turns[] | select(.action.kind == "cancel")]) as $cancel_turns
    | if ($ordered | not) then {status:"rejected", code:"TURN_SEQUENCE_VIOLATION"}
      elif (.turns | length) > .limits.max_turns then {status:"rejected", code:"TURN_BUDGET_EXCEEDED"}
      elif $tool_calls > .limits.max_tool_calls then {status:"rejected", code:"TOOL_CALL_BUDGET_EXCEEDED"}
      elif $input_bytes > .limits.max_input_bytes then {status:"rejected", code:"INPUT_BUDGET_EXCEEDED"}
      elif $output_bytes > .limits.max_output_bytes then {status:"rejected", code:"OUTPUT_BUDGET_EXCEEDED"}
      elif $context_nodes > .limits.max_context_nodes then {status:"rejected", code:"CONTEXT_NODE_BUDGET_EXCEEDED"}
      elif $wall_ms > .limits.max_wall_ms then {status:"rejected", code:"WALL_BUDGET_EXCEEDED"}
      elif ($unowned_tools + $unowned_files) > 0 then {status:"rejected", code:"OWNED_PATH_VIOLATION"}
      elif .cancellation.requested and (.cancellation.observed | not) then {status:"rejected", code:"CANCELLATION_NOT_OBSERVED"}
      elif .cancellation.requested and (($cancel_turns | length) != 1 or $cancel_turns[0].number != .cancellation.turn) then {status:"rejected", code:"CANCELLATION_TURN_MISMATCH"}
      elif .cancellation.requested and ([.turns[] | select(.number > $replay.cancellation.turn)] | length) > 0 then {status:"rejected", code:"ACTION_AFTER_CANCELLATION"}
      elif (.cancellation.requested | not) and (($cancel_turns | length) > 0) then {status:"rejected", code:"UNREQUESTED_CANCELLATION"}
      elif .cancellation.requested then {status:"cancelled", code:"CANCELLATION_OBSERVED"}
      else {status:"accepted", code:"PROTOCOL_ACCEPTED"}
      end
  ' "$1"
}

assert_expected() {
  local fixture="$1"
  local actual expected
  validate_replay "$fixture"
  actual="$(semantic_result "$fixture")"
  expected="$(jq -c '.expected' "$fixture")"
  [[ "$actual" == "$expected" ]] || fail "$(basename "$fixture"): expected $expected, got $actual"
}

expect_schema_failure() {
  local fixture="$1"
  if validate_replay "$fixture" 2>/dev/null; then
    fail "$(basename "$fixture") unexpectedly passed schema validation"
  fi
}

assert_expected "$BASE_FIXTURE"
for fixture in "$REPLAY_DIR"/semantic/*.json; do
  assert_expected "$fixture"
done

expect_schema_failure "$REPLAY_DIR/invalid/multiple_actions.json"
if jq -se 'length == 1' "$REPLAY_DIR/invalid/concatenated_responses.jsonl" >/dev/null; then
  fail "concatenated responses unexpectedly passed the one-document boundary"
fi

budget_case() {
  local name="$1"
  local jq_filter="$2"
  local code="$3"
  local path="$WORK_DIR/$name.json"
  jq "$jq_filter | .expected = {status:\"rejected\", code:\"$code\"}" "$BASE_FIXTURE" >"$path"
  assert_expected "$path"
}

budget_case turn_budget '.limits.max_turns = 1' TURN_BUDGET_EXCEEDED
budget_case tool_budget '.limits.max_tool_calls = 0' TOOL_CALL_BUDGET_EXCEEDED
budget_case input_budget '.limits.max_input_bytes = 179' INPUT_BUDGET_EXCEEDED
budget_case output_budget '.limits.max_output_bytes = 239' OUTPUT_BUDGET_EXCEEDED
budget_case context_budget '.limits.max_context_nodes = 7' CONTEXT_NODE_BUDGET_EXCEEDED
budget_case wall_budget '.limits.max_wall_ms = 299' WALL_BUDGET_EXCEEDED
budget_case turn_sequence '.turns[1].number = 3' TURN_SEQUENCE_VIOLATION

echo "SleyBench protocol replays passed"
