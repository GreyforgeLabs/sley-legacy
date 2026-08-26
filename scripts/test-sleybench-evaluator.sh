#!/usr/bin/env bash
set -euo pipefail

repo_root="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
case_manifest="$repo_root/fixtures/sleybench/smoke-v0/case-manifest.json"
run_manifest="$repo_root/fixtures/sleybench/smoke-v0/run-manifest.json"
work_root="$(mktemp -d)"
trap 'rm -rf -- "$work_root"' EXIT

fail() {
  printf 'sleybench evaluator test failed: %s\n' "$1" >&2
  exit 1
}

generated_suite="$work_root/generated-smoke"
"$repo_root/scripts/generate-sleybench-smoke.sh" "$generated_suite" >/dev/null
diff -qr "$repo_root/fixtures/sleybench/smoke-v0" "$generated_suite" >/dev/null \
  || fail "committed smoke fixtures do not match the deterministic generator"

refresh_run_digest() {
  local cases="$1" run="$2" digest tmp
  digest="sha256:$(sha256sum "$cases" | awk '{print $1}')"
  tmp="$run.tmp"
  jq --arg digest "$digest" '.case_manifest_digest = $digest' "$run" > "$tmp"
  mv "$tmp" "$run"
}

expect_failure() {
  local label="$1"
  shift
  if "$@" >"$work_root/$label.stdout" 2>"$work_root/$label.stderr"; then
    fail "$label unexpectedly passed"
  fi
}

full_output="$work_root/full-output"
full_report="$work_root/full-report.json"
"$repo_root/bin/sley-agent-bench" run --json \
  --manifest "$case_manifest" --run-manifest "$run_manifest" \
  --output-dir "$full_output" > "$full_report"

jq -e '
  .schema == "sley.agent_bench.aggregate.v0" and
  .status == "passed" and
  .counts.case_count == 24 and
  .counts.passed_count == 24 and
  .counts.failed_count == 0 and
  ([.counts.family_counts[]] | all(. == 4)) and
  .rates.parse_at_1.rate == 1 and
  .rates.compile_at_1.rate == 1 and
  .rates.pass_at_1.rate == 1 and
  .rates.syntax_hallucination.applicable == false and
  .rates.stdlib_hallucination.applicable == false and
  .rates.invalid_tool_call.applicable == false and
  .adapter.model_measurement_applicable == false and
  all(.cases[]; .workspace.clean_before and (.evidence[] | startswith("sha256:")))
' "$full_report" >/dev/null || fail "24-case smoke aggregate is incomplete"

"$repo_root/bin/sley-contract" validate \
  --schema sley.agent_bench.aggregate.v0 "$full_report" \
  --schemas "$repo_root/docs/schemas" --json >/dev/null

if find "$full_output/workspaces" -mindepth 1 -print -quit | grep -q .; then
  fail "non-retained smoke run left a case workspace"
fi

expect_failure unknown-field "$repo_root/bin/sley-contract" validate \
  --schema sley.agent_bench.case_manifest.v0 \
  "$repo_root/fixtures/sleybench/smoke-v0/malformed/case-manifest-unknown-field.json" \
  --schemas "$repo_root/docs/schemas" --json
expect_failure zero-limit "$repo_root/bin/sley-contract" validate \
  --schema sley.agent_bench.case_manifest.v0 \
  "$repo_root/fixtures/sleybench/smoke-v0/malformed/case-manifest-zero-limit.json" \
  --schemas "$repo_root/docs/schemas" --json

bad_run="$work_root/bad-run.json"
jq '.case_manifest_digest = "sha256:0000000000000000000000000000000000000000000000000000000000000000"' \
  "$run_manifest" > "$bad_run"
expect_failure bad-manifest-digest "$repo_root/bin/sley-agent-bench" run --json \
  --manifest "$case_manifest" --run-manifest "$bad_run" \
  --case sleybench-smoke-f1-001

scope_suite="$work_root/scope-suite"
cp -a "$repo_root/fixtures/sleybench/smoke-v0" "$scope_suite"
printf 'out of scope\n' > "$scope_suite/cases/sleybench-smoke-f1-001/candidate/UNOWNED.txt"
expect_failure scope-violation "$repo_root/bin/sley-agent-bench" run --json \
  --manifest "$scope_suite/case-manifest.json" \
  --run-manifest "$scope_suite/run-manifest.json" \
  --case sleybench-smoke-f1-001
jq -e '.cases[0].issues | any(.code == "SLEYBENCH_SCOPE_VIOLATION")' \
  "$work_root/scope-violation.stdout" >/dev/null \
  || fail "scope violation did not produce evidence"

limit_suite="$work_root/limit-suite"
cp -a "$repo_root/fixtures/sleybench/smoke-v0" "$limit_suite"
jq '(.cases[] | select(.id == "sleybench-smoke-f1-001") | .limits.tool_calls) = 1' \
  "$limit_suite/case-manifest.json" > "$limit_suite/case-manifest.tmp"
mv "$limit_suite/case-manifest.tmp" "$limit_suite/case-manifest.json"
refresh_run_digest "$limit_suite/case-manifest.json" "$limit_suite/run-manifest.json"
expect_failure tool-limit "$repo_root/bin/sley-agent-bench" run --json \
  --manifest "$limit_suite/case-manifest.json" \
  --run-manifest "$limit_suite/run-manifest.json" \
  --case sleybench-smoke-f1-001
jq -e '.cases[0].steps | any(.issues[]?.code == "SLEYBENCH_TOOL_LIMIT")' \
  "$work_root/tool-limit.stdout" >/dev/null \
  || fail "tool-call ceiling did not produce evidence"

workspace_suite="$work_root/workspace-suite"
cp -a "$repo_root/fixtures/sleybench/smoke-v0" "$workspace_suite"
jq '(.cases[] | select(.id == "sleybench-smoke-f5-001") | .limits.workspace_bytes) = 1024' \
  "$workspace_suite/case-manifest.json" > "$workspace_suite/case-manifest.tmp"
mv "$workspace_suite/case-manifest.tmp" "$workspace_suite/case-manifest.json"
refresh_run_digest "$workspace_suite/case-manifest.json" "$workspace_suite/run-manifest.json"
expect_failure workspace-limit "$repo_root/bin/sley-agent-bench" run --json \
  --manifest "$workspace_suite/case-manifest.json" \
  --run-manifest "$workspace_suite/run-manifest.json" \
  --case sleybench-smoke-f5-001
jq -e '.cases[0].issues | any(.code == "SLEYBENCH_WORKSPACE_LIMIT")' \
  "$work_root/workspace-limit.stdout" >/dev/null \
  || fail "workspace-byte ceiling did not produce evidence"

wall_suite="$work_root/wall-suite"
cp -a "$repo_root/fixtures/sleybench/smoke-v0" "$wall_suite"
jq '(.cases[] | select(.id == "sleybench-smoke-f1-001") | .limits.wall_seconds) = 1' \
  "$wall_suite/case-manifest.json" > "$wall_suite/case-manifest.tmp"
mv "$wall_suite/case-manifest.tmp" "$wall_suite/case-manifest.json"
refresh_run_digest "$wall_suite/case-manifest.json" "$wall_suite/run-manifest.json"
expect_failure wall-limit "$repo_root/bin/sley-agent-bench" run --json \
  --manifest "$wall_suite/case-manifest.json" \
  --run-manifest "$wall_suite/run-manifest.json" \
  --case sleybench-smoke-f1-001
jq -e '.cases[0] | ([.issues[].code, .steps[].issues[]?.code] | any(. == "SLEYBENCH_WALL_LIMIT"))' \
  "$work_root/wall-limit.stdout" >/dev/null \
  || fail "wall-time ceiling did not produce evidence"

process_suite="$work_root/process-suite"
cp -a "$repo_root/fixtures/sleybench/smoke-v0" "$process_suite"
jq '(.cases[] | select(.id == "sleybench-smoke-f1-001") | .limits.processes) = 1' \
  "$process_suite/case-manifest.json" > "$process_suite/case-manifest.tmp"
mv "$process_suite/case-manifest.tmp" "$process_suite/case-manifest.json"
refresh_run_digest "$process_suite/case-manifest.json" "$process_suite/run-manifest.json"
expect_failure process-limit "$repo_root/bin/sley-agent-bench" run --json \
  --manifest "$process_suite/case-manifest.json" \
  --run-manifest "$process_suite/run-manifest.json" \
  --case sleybench-smoke-f1-001
jq -e '.cases[0].steps | any(.issues[]?.code == "SLEYBENCH_PROCESS_LIMIT")' \
  "$work_root/process-limit.stdout" >/dev/null \
  || fail "process ceiling did not produce evidence"

output_suite="$work_root/output-suite"
cp -a "$repo_root/fixtures/sleybench/smoke-v0" "$output_suite"
jq '(.cases[] | select(.id == "sleybench-smoke-f5-004") | .limits.output_bytes) = 1024' \
  "$output_suite/case-manifest.json" > "$output_suite/case-manifest.tmp"
mv "$output_suite/case-manifest.tmp" "$output_suite/case-manifest.json"
refresh_run_digest "$output_suite/case-manifest.json" "$output_suite/run-manifest.json"
expect_failure output-limit "$repo_root/bin/sley-agent-bench" run --json \
  --manifest "$output_suite/case-manifest.json" \
  --run-manifest "$output_suite/run-manifest.json" \
  --case sleybench-smoke-f5-004
jq -e '.cases[0].steps | any(.issues[]?.code == "SLEYBENCH_OUTPUT_LIMIT")' \
  "$work_root/output-limit.stdout" >/dev/null \
  || fail "output-byte ceiling did not produce evidence"

retained_run="$work_root/retained-run.json"
jq '.retain_workspaces = true' "$run_manifest" > "$retained_run"
retained_output="$work_root/retained-output"
"$repo_root/bin/sley-agent-bench" run --json \
  --manifest "$case_manifest" --run-manifest "$retained_run" \
  --case sleybench-smoke-f5-001 --output-dir "$retained_output" \
  > "$work_root/retained-report.json"
[[ -d "$retained_output/workspaces/sleybench-smoke-f5-001/.git" ]] \
  || fail "explicit retention did not preserve the isolated workspace"
jq -e '.cases[0].workspace.retained == true and .cases[0].workspace.path_exposed == false' \
  "$work_root/retained-report.json" >/dev/null \
  || fail "retention boundary was not reported"

if find "$repo_root/fixtures/sleybench/smoke-v0/cases" -path '*/candidate/*' \
  -type f ! -name '*.sley' -print -quit | grep -q .; then
  fail "candidate fixtures contain non-source oracle material"
fi

printf 'SleyBench evaluator ready: cases=24 families=6 malformed=2 limits=5 scope=1 retention=2\n'
