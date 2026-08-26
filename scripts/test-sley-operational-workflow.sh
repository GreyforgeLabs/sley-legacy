#!/usr/bin/env bash
set -euo pipefail

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
WORK_DIR="$(mktemp -d "$ROOT_DIR/.sley-operational-workflow-test.XXXXXX")"
trap 'rm -rf "$WORK_DIR"' EXIT

CONTROLLER="$WORK_DIR/controller.py"
RAW_EVIDENCE="$WORK_DIR/raw-evidence"
STRUCTURAL_EVIDENCE="$WORK_DIR/structural-evidence"
mkdir -p "$RAW_EVIDENCE" "$STRUCTURAL_EVIDENCE"
printf '%s\n' '#!/usr/bin/env python3' 'raise SystemExit(0)' >"$CONTROLLER"

tree_digest() {
  python3 - "$1" <<'PY'
import hashlib
import sys
from pathlib import Path

root = Path(sys.argv[1])
material = bytearray()
for path in sorted(item for item in root.rglob("*") if item.is_file() and ".git" not in item.parts):
    material.extend(path.relative_to(root).as_posix().encode("utf-8"))
    material.append(0)
    material.extend(hashlib.sha256(path.read_bytes()).hexdigest().encode("ascii"))
    material.append(10)
print("sha256:" + hashlib.sha256(material).hexdigest())
PY
}

PLAN="$WORK_DIR/plan.json"
"$ROOT_DIR/bin/sley" operational-workflow plan \
  --controller "$CONTROLLER" \
  --json >"$PLAN"
jq -e '
  .schema == "sley.operational.agent_workflow_plan.v1"
  and .status == "ready_for_approval"
  and (.identity.controller_digest | startswith("sha256:"))
' "$PLAN" >/dev/null
jq -e '
  .authority.provider_execution_authorized == false
  and .authority.approval_required == true
  and .arms[0].mode == "K1"
  and .arms[0].read_only_tool_turns == 0
  and .arms[1].mode == "K3"
  and .arms[1].read_only_tool_turns == 1
  and (.equal_controls.fields | index("action_budget") | not)
  and (.equal_controls.fields | index("tool_calls") | not)
  and (.equal_controls.fields | index("wall_seconds") | not)
  and .controlled_difference.manifest_limits_fully_enforced == false
  and .controlled_difference.observed_context_consumption_measured == true
  and (.issues | map(.code) | index("MANIFEST_LIMIT_ENFORCEMENT_PARTIAL"))
' "$PLAN" >/dev/null

EXPECTED_FINAL="$WORK_DIR/expected-final"
mkdir -p "$EXPECTED_FINAL"
cp -a "$ROOT_DIR/fixtures/operational/agent-workflow-v1/workspace/." "$EXPECTED_FINAL/"
while IFS= read -r relative; do
  mkdir -p "$EXPECTED_FINAL/$(dirname "$relative")"
  cp "$ROOT_DIR/fixtures/operational/agent-workflow-v1/candidate/$relative" "$EXPECTED_FINAL/$relative"
done < <(jq -r '.cases[0].owned_paths[]' "$ROOT_DIR/fixtures/operational/agent-workflow-v1/case-manifest.json")
EXPECTED_FINAL_DIGEST="$(tree_digest "$EXPECTED_FINAL")"
BARE_CANDIDATE_DIGEST="$(tree_digest "$ROOT_DIR/fixtures/operational/agent-workflow-v1/candidate")"
jq -e \
  --arg expected "$EXPECTED_FINAL_DIGEST" \
  --arg bare "$BARE_CANDIDATE_DIGEST" '
    .identity.candidate_digest == $expected
    and .identity.candidate_digest != $bare
  ' "$PLAN" >/dev/null

if "$ROOT_DIR/bin/sley" operational-workflow plan \
  --repo-root "$ROOT_DIR" \
  --controller "$CONTROLLER" \
  --json >"$WORK_DIR/reserved.out" 2>"$WORK_DIR/reserved.err"; then
  echo "operational workflow unexpectedly accepted a caller repository root" >&2
  exit 1
fi
grep -Fq 'operational-workflow option is reserved for the Sley-owned contract' "$WORK_DIR/reserved.err"

printf '%s\n' 'raw prompt' >"$RAW_EVIDENCE/prompt-1.txt"
printf '%s\n' '{"action":"submit"}' >"$RAW_EVIDENCE/response-1.txt"
printf '%s\n' 'structural prompt' >"$STRUCTURAL_EVIDENCE/prompt-1.txt"
printf '%s\n' '{"action":"tool"}' >"$STRUCTURAL_EVIDENCE/response-1.txt"
printf '%s\n' \
  '<interaction_history>' \
  '[{"model":{"action":"tool","tool":"query","argv":["--json","--kind","tasks","."],"files":{}},"tool_report":{"schema":"sley.query.report.v0","tasks":[]}}]' \
  '</interaction_history>' >"$STRUCTURAL_EVIDENCE/prompt-2.txt"
printf '%s\n' '{"action":"submit"}' >"$STRUCTURAL_EVIDENCE/response-2.txt"

CASE_DIGEST="$(jq -r '.case_manifest_digest' "$ROOT_DIR/fixtures/operational/agent-workflow-v1/run-k1.json")"
RAW_RUN_DIGEST="sha256:$(sha256sum "$ROOT_DIR/fixtures/operational/agent-workflow-v1/run-k1.json" | cut -d' ' -f1)"
STRUCTURAL_RUN_DIGEST="sha256:$(sha256sum "$ROOT_DIR/fixtures/operational/agent-workflow-v1/run-k3.json" | cut -d' ' -f1)"
WORKSPACE_DIGEST="$(jq -r '.identity.workspace_digest' "$PLAN")"
CANDIDATE_DIGEST="$(jq -r '.identity.candidate_digest' "$PLAN")"
PROMPT_DIGEST="$(jq -r '.identity.prompt_digest' "$PLAN")"
BOOTSTRAP_DIGEST="$(jq -r '.identity.bootstrap_digest' "$PLAN")"
EMPTY_DIGEST="sha256:e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855"
APPROVAL="$WORK_DIR/approval.json"
jq -n \
  --arg commit "$(jq -r '.identity.sley_commit' "$PLAN")" \
  --arg case_digest "$CASE_DIGEST" \
  --arg raw_digest "$RAW_RUN_DIGEST" \
  --arg structural_digest "$STRUCTURAL_RUN_DIGEST" \
  --arg controller_digest "$(jq -r '.identity.controller_digest' "$PLAN")" \
  '{
    schema:"greyforge.sley.operational.agent_workflow_approval.private.v1",
    status:"approved",
    workflow_id:"sley-agent-project-cross-module-rename-v1",
    identity:{
      sley_commit:$commit,
      case_manifest_digest:$case_digest,
      raw_run_manifest_digest:$raw_digest,
      structural_run_manifest_digest:$structural_digest,
      controller_digest:$controller_digest
    },
    execution:{
      provider:"openai",
      model:"gpt-5.6-sol",
      thinking:"low",
      run_ids:["sley-operational-agent-workflow-v1-k1","sley-operational-agent-workflow-v1-k3"],
      max_total_spend_usd:2,
      provider_execution_authorized:true,
      operator_authority_confirmed:true
    },
    denied_actions:{repository_mutation:true,product_runtime_mutation:true,deploy:true,publication:true}
  }' >"$APPROVAL"

make_aggregate() {
  local mode="$1" run_id="$2" run_digest="$3" tool_calls="$4" input_tokens="$5" output_tokens="$6" wall_ms="$7" output="$8"
  jq \
    --arg mode "$mode" \
    --arg run "$run_id" \
    --arg case_digest "$CASE_DIGEST" \
    --arg run_digest "$run_digest" \
    --arg workspace_digest "$WORKSPACE_DIGEST" \
    --arg candidate_digest "$CANDIDATE_DIGEST" \
    --arg prompt_digest "$PROMPT_DIGEST" \
    --arg bootstrap_digest "$BOOTSTRAP_DIGEST" \
    --arg empty_digest "$EMPTY_DIGEST" \
    --argjson tool_calls "$tool_calls" \
    --argjson input_tokens "$input_tokens" \
    --argjson output_tokens "$output_tokens" \
    --argjson wall_ms "$wall_ms" \
    --slurpfile manifest "$ROOT_DIR/fixtures/operational/agent-workflow-v1/case-manifest.json" '
      .status = "passed"
      | .run_id = $run
      | .suite_id = "sley-operational-agent-workflow-v1"
      | .mode = $mode
      | .adapter = {kind:"openclaw_one_shot",id:"openclaw-one-shot-v2",model_measurement_applicable:true}
      | .counts.case_count = 1
      | .counts.passed_count = 1
      | .counts.failed_count = 0
      | .counts.family_counts = {
          syntax_compilation:0,
          semantic_implementation:0,
          diagnosis_repair:0,
          translation_preservation:0,
          structural_tool_use:0,
          repository_patch_integration:1
        }
      | .resources = {
          tool_calls_total:$tool_calls,
          compiler_cycles_total:5,
          wall_ms_total:$wall_ms,
          input_tokens_total:$input_tokens,
          output_tokens_total:$output_tokens,
          estimated_cost_usd_total:0.001
        }
      | .evidence.case_manifest_digest = $case_digest
      | .evidence.run_manifest_digest = $run_digest
      | .evidence.bootstrap_digest = $bootstrap_digest
      | .cases = [(.cases[0]
        | .run_id = $run
        | .case_id = "sleybench-v1-f6-001"
        | .family = "repository_patch_integration"
        | .mode = $mode
        | .adapter_id = "openclaw-one-shot-v2"
        | .status = "passed"
        | .score_tags = ["parse","compile","pass","semantic","structural","repository_patch"]
        | .scores.prompt_accepted = true
        | .scores.parse_passed = true
        | .scores.check_passed = true
        | .scores.oracle_tests_passed = true
        | .scores.semantic_output_matched = true
        | .scores.scope_precise = true
        | .scores.required_evidence_produced = true
        | .measurements.tool_calls = $tool_calls
        | .measurements.compiler_cycles = 5
        | .measurements.input_tokens = $input_tokens
        | .measurements.output_tokens = $output_tokens
        | .measurements.wall_ms = $wall_ms
        | .measurements.changed_files = 2
        | .measurements.changed_lines = 4
        | .measurements.estimated_cost_usd = 0.001
        | .measurements.invalid_tool_call = false
        | .measurements.model_measurement_applicable = true
        | .workspace.trusted_fixture = false
        | .evidence.training_exclusion_id = $prompt_digest
        | .evidence.initial_tree_digest = $workspace_digest
        | .evidence.candidate_digest = $candidate_digest
        | .evidence.final_tree_digest = $candidate_digest
        | .evidence.prompt_digest = $prompt_digest
        | .steps = (
            (($manifest[0].cases[0].preflight | map([., "preflight"]))
            + ($manifest[0].cases[0].oracle.commands | map([., "oracle"])))
            | map({
                name:.[0].name,
                phase:.[1],
                argv:.[0].argv,
                expected_exit:.[0].expected_exit,
                actual_exit:.[0].expected_exit,
                status:"passed",
                stdout_schema:.[0].expected_schema,
                assertion_count:(.[0].assertions | length),
                assertion_passed_count:(.[0].assertions | length),
                output_bytes:1,
                stdout_digest:$empty_digest,
                stderr_digest:$empty_digest,
                issues:[]
              })
          )
        | .issues = [])]
    ' "$ROOT_DIR/fixtures/contracts/agent_bench_aggregate_failed.json" >"$output"
  local step_material step_digest temporary
  step_material="$(jq -cS '.cases[0].steps' "$output")"
  step_digest="sha256:$(printf '%s' "$step_material" | sha256sum | cut -d' ' -f1)"
  temporary="$output.tmp"
  jq --arg step_digest "$step_digest" '.cases[0].evidence.step_evidence_digest = $step_digest' "$output" >"$temporary"
  mv "$temporary" "$output"
}

tree_digest() {
  local root="$1" material="$WORK_DIR/tree-material.$RANDOM" path relative digest
  : >"$material"
  while IFS= read -r -d '' path; do
    relative="${path#"$root"/}"
    digest="$(sha256sum "$path" | cut -d' ' -f1)"
    printf '%s\0%s\n' "$relative" "$digest" >>"$material"
  done < <(find "$root" -type f -not -path '*/.git/*' -print0 | sort -z)
  printf 'sha256:%s\n' "$(sha256sum "$material" | cut -d' ' -f1)"
}

make_provenance() {
  local output="$1" raw_aggregate="$2" raw_evidence="$3" structural_aggregate="$4" structural_evidence="$5"
  local approval_digest raw_aggregate_digest raw_evidence_digest raw_attempts
  local structural_aggregate_digest structural_evidence_digest structural_attempts
  approval_digest="sha256:$(sha256sum "$APPROVAL" | cut -d' ' -f1)"
  raw_aggregate_digest="sha256:$(sha256sum "$raw_aggregate" | cut -d' ' -f1)"
  raw_evidence_digest="$(tree_digest "$raw_evidence")"
  raw_attempts="$(find "$raw_evidence" -maxdepth 1 -type f -name 'prompt-[12].txt' | wc -l)"
  structural_aggregate_digest="sha256:$(sha256sum "$structural_aggregate" | cut -d' ' -f1)"
  structural_evidence_digest="$(tree_digest "$structural_evidence")"
  structural_attempts="$(find "$structural_evidence" -maxdepth 1 -type f -name 'prompt-[12].txt' | wc -l)"
  jq -n \
    --arg commit "$(jq -r '.identity.sley_commit' "$PLAN")" \
    --arg case_digest "$CASE_DIGEST" \
    --arg raw_run_digest "$RAW_RUN_DIGEST" \
    --arg structural_run_digest "$STRUCTURAL_RUN_DIGEST" \
    --arg controller_digest "$(jq -r '.identity.controller_digest' "$PLAN")" \
    --arg approval_digest "$approval_digest" \
    --arg raw_aggregate_digest "$raw_aggregate_digest" \
    --arg raw_evidence_digest "$raw_evidence_digest" \
    --argjson raw_attempts "$raw_attempts" \
    --arg structural_aggregate_digest "$structural_aggregate_digest" \
    --arg structural_evidence_digest "$structural_evidence_digest" \
    --argjson structural_attempts "$structural_attempts" '
      {
        schema:"greyforge.sley.operational.agent_workflow_execution.private.v1",
        status:"completed",
        workflow_id:"sley-agent-project-cross-module-rename-v1",
        identity:{
          sley_commit:$commit,
          case_manifest_digest:$case_digest,
          raw_run_manifest_digest:$raw_run_digest,
          structural_run_manifest_digest:$structural_run_digest,
          controller_digest:$controller_digest,
          execution_approval_digest:$approval_digest
        },
        execution:{
          provider:"openai",
          model:"gpt-5.6-sol",
          thinking:"low",
          run_ids:["sley-operational-agent-workflow-v1-k1","sley-operational-agent-workflow-v1-k3"],
          controller_executed:true,
          provider_execution_observed:true
        },
        evidence:{
          raw_aggregate_digest:$raw_aggregate_digest,
          raw_evidence_bundle_digest:$raw_evidence_digest,
          raw_inference_attempts:$raw_attempts,
          structural_aggregate_digest:$structural_aggregate_digest,
          structural_evidence_bundle_digest:$structural_evidence_digest,
          structural_inference_attempts:$structural_attempts
        },
        review:{
          independent:true,
          reviewer_role:"independent_evidence_reviewer",
          reviewer_id:"synthetic-test-reviewer",
          verdict:"passed",
          high_findings:0,
          medium_findings:0,
          raw_evidence_inspected:true,
          provider_origin_inspected:true
        },
        assurance:{assertion_based:true,cryptographically_verified:false}
      }
    ' >"$output"
}

RAW_AGGREGATE="$WORK_DIR/raw.json"
STRUCTURAL_AGGREGATE="$WORK_DIR/structural.json"
make_aggregate K1 sley-operational-agent-workflow-v1-k1 "$RAW_RUN_DIGEST" 0 100 20 1000 "$RAW_AGGREGATE"
make_aggregate K3 sley-operational-agent-workflow-v1-k3 "$STRUCTURAL_RUN_DIGEST" 1 180 30 1200 "$STRUCTURAL_AGGREGATE"
PROVENANCE="$WORK_DIR/execution-provenance.json"
make_provenance "$PROVENANCE" "$RAW_AGGREGATE" "$RAW_EVIDENCE" "$STRUCTURAL_AGGREGATE" "$STRUCTURAL_EVIDENCE"

COMPARISON="$WORK_DIR/comparison.json"
"$ROOT_DIR/bin/sley" operational-workflow compare \
  --controller "$CONTROLLER" \
  --raw-aggregate "$RAW_AGGREGATE" \
  --raw-evidence "$RAW_EVIDENCE" \
  --structural-aggregate "$STRUCTURAL_AGGREGATE" \
  --structural-evidence "$STRUCTURAL_EVIDENCE" \
  --approval-record "$APPROVAL" \
  --execution-provenance "$PROVENANCE" \
  --json >"$COMPARISON"
jq -e '
  .schema == "sley.operational.agent_workflow_comparison.v1"
  and .status == "passed"
  and .arms.raw_source.strict_success == true
  and .arms.sley_structural.strict_success == true
  and .arms.sley_structural.structural_response_bytes > 0
  and .deltas.prompt_bytes > 0
  and .decision.outcome == "both_accepted"
  and .decision.structural_context_effect == "expansion"
  and .decision.promotion_decision == "deferred"
  and .decision.general_claim_supported == false
  and .controls.equal_task_context_spend_budget_verified == true
  and .methodology.manifest_limit_enforcement == "partial_controller_bounds"
  and .arms.raw_source.inference_attempts == 1
  and .arms.sley_structural.inference_attempts == 2
  and (.identity.execution_provenance_digest | startswith("sha256:"))
  and (.issues | map(.code) | index("PROVIDER_ORIGIN_ASSERTION_NOT_CRYPTOGRAPHIC"))
  and (.issues | map(.code) | index("STRUCTURAL_CONTEXT_EXPANDED"))
' "$COMPARISON" >/dev/null

CANDIDATE_MISMATCH_AGGREGATE="$WORK_DIR/raw-candidate-mismatch.json"
jq '.cases[0].evidence.candidate_digest = "sha256:aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa"' "$RAW_AGGREGATE" >"$CANDIDATE_MISMATCH_AGGREGATE"
if "$ROOT_DIR/bin/sley" operational-workflow compare \
  --controller "$CONTROLLER" \
  --raw-aggregate "$CANDIDATE_MISMATCH_AGGREGATE" \
  --raw-evidence "$RAW_EVIDENCE" \
  --structural-aggregate "$STRUCTURAL_AGGREGATE" \
  --structural-evidence "$STRUCTURAL_EVIDENCE" \
  --approval-record "$APPROVAL" \
  --execution-provenance "$PROVENANCE" \
  --json >"$WORK_DIR/candidate-mismatch.out" 2>"$WORK_DIR/candidate-mismatch.err"; then
  echo "comparison with a mismatched candidate digest unexpectedly passed" >&2
  exit 1
fi
grep -Fq 'CASE_EVIDENCE_MISMATCH' "$WORK_DIR/candidate-mismatch.err"

ORACLE_MISMATCH_AGGREGATE="$WORK_DIR/raw-oracle-mismatch.json"
jq '.cases[0].steps[1].status = "failed"' "$RAW_AGGREGATE" >"$ORACLE_MISMATCH_AGGREGATE"
if "$ROOT_DIR/bin/sley" operational-workflow compare \
  --controller "$CONTROLLER" \
  --raw-aggregate "$ORACLE_MISMATCH_AGGREGATE" \
  --raw-evidence "$RAW_EVIDENCE" \
  --structural-aggregate "$STRUCTURAL_AGGREGATE" \
  --structural-evidence "$STRUCTURAL_EVIDENCE" \
  --approval-record "$APPROVAL" \
  --execution-provenance "$PROVENANCE" \
  --json >"$WORK_DIR/oracle-mismatch.out" 2>"$WORK_DIR/oracle-mismatch.err"; then
  echo "comparison with mismatched oracle evidence unexpectedly passed" >&2
  exit 1
fi
grep -Fq 'ORACLE_EVIDENCE_MISMATCH' "$WORK_DIR/oracle-mismatch.err"

NO_TOOL_AGGREGATE="$WORK_DIR/structural-no-tool.json"
make_aggregate K3 sley-operational-agent-workflow-v1-k3 "$STRUCTURAL_RUN_DIGEST" 0 110 20 900 "$NO_TOOL_AGGREGATE"
NO_TOOL_EVIDENCE="$WORK_DIR/structural-no-tool-evidence"
mkdir -p "$NO_TOOL_EVIDENCE"
cp "$STRUCTURAL_EVIDENCE/prompt-1.txt" "$STRUCTURAL_EVIDENCE/response-1.txt" "$NO_TOOL_EVIDENCE/"
NO_TOOL_COMPARISON="$WORK_DIR/no-tool-comparison.json"
NO_TOOL_PROVENANCE="$WORK_DIR/no-tool-provenance.json"
make_provenance "$NO_TOOL_PROVENANCE" "$RAW_AGGREGATE" "$RAW_EVIDENCE" "$NO_TOOL_AGGREGATE" "$NO_TOOL_EVIDENCE"
"$ROOT_DIR/bin/sley" operational-workflow compare \
  --controller "$CONTROLLER" \
  --raw-aggregate "$RAW_AGGREGATE" \
  --raw-evidence "$RAW_EVIDENCE" \
  --structural-aggregate "$NO_TOOL_AGGREGATE" \
  --structural-evidence "$NO_TOOL_EVIDENCE" \
  --approval-record "$APPROVAL" \
  --execution-provenance "$NO_TOOL_PROVENANCE" \
  --json >"$NO_TOOL_COMPARISON"
jq -e '
  .arms.raw_source.strict_success == true
  and .arms.sley_structural.strict_success == false
  and .arms.sley_structural.accepted_change_tokens == null
  and .decision.outcome == "structural_disadvantage_observed"
  and .decision.recommendation == "defer_structural_interface"
  and (.issues | map(.code) | index("STRUCTURAL_TOOL_NOT_EXERCISED"))
' "$NO_TOOL_COMPARISON" >/dev/null

MISSING_HISTORY_EVIDENCE="$WORK_DIR/structural-missing-history"
mkdir -p "$MISSING_HISTORY_EVIDENCE"
cp "$STRUCTURAL_EVIDENCE/prompt-1.txt" "$STRUCTURAL_EVIDENCE/response-1.txt" "$STRUCTURAL_EVIDENCE/response-2.txt" "$MISSING_HISTORY_EVIDENCE/"
if "$ROOT_DIR/bin/sley" operational-workflow compare \
  --controller "$CONTROLLER" \
  --raw-aggregate "$RAW_AGGREGATE" \
  --raw-evidence "$RAW_EVIDENCE" \
  --structural-aggregate "$STRUCTURAL_AGGREGATE" \
  --structural-evidence "$MISSING_HISTORY_EVIDENCE" \
  --approval-record "$APPROVAL" \
  --execution-provenance "$PROVENANCE" \
  --json >"$WORK_DIR/missing-history.out" 2>"$WORK_DIR/missing-history.err"; then
  echo "comparison without structural history unexpectedly passed" >&2
  exit 1
fi
grep -Fq 'EVIDENCE_INCOMPLETE: exercised structural turn requires prompt-2.txt' "$WORK_DIR/missing-history.err"

INVALID_APPROVAL="$WORK_DIR/invalid-approval.json"
jq '.identity.controller_digest = "sha256:aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa"' "$APPROVAL" >"$INVALID_APPROVAL"
if "$ROOT_DIR/bin/sley" operational-workflow compare \
  --controller "$CONTROLLER" \
  --raw-aggregate "$RAW_AGGREGATE" \
  --raw-evidence "$RAW_EVIDENCE" \
  --structural-aggregate "$STRUCTURAL_AGGREGATE" \
  --structural-evidence "$STRUCTURAL_EVIDENCE" \
  --approval-record "$INVALID_APPROVAL" \
  --execution-provenance "$PROVENANCE" \
  --json >"$WORK_DIR/invalid-approval.out" 2>"$WORK_DIR/invalid-approval.err"; then
  echo "comparison with a mismatched approval record unexpectedly passed" >&2
  exit 1
fi
grep -Fq 'APPROVAL_RECORD_INVALID' "$WORK_DIR/invalid-approval.err"

INVALID_PROVENANCE="$WORK_DIR/invalid-provenance.json"
jq '.review.medium_findings = 1' "$PROVENANCE" >"$INVALID_PROVENANCE"
if "$ROOT_DIR/bin/sley" operational-workflow compare \
  --controller "$CONTROLLER" \
  --raw-aggregate "$RAW_AGGREGATE" \
  --raw-evidence "$RAW_EVIDENCE" \
  --structural-aggregate "$STRUCTURAL_AGGREGATE" \
  --structural-evidence "$STRUCTURAL_EVIDENCE" \
  --approval-record "$APPROVAL" \
  --execution-provenance "$INVALID_PROVENANCE" \
  --json >"$WORK_DIR/invalid-provenance.out" 2>"$WORK_DIR/invalid-provenance.err"; then
  echo "comparison with invalid execution provenance unexpectedly passed" >&2
  exit 1
fi
grep -Fq 'EXECUTION_PROVENANCE_INVALID' "$WORK_DIR/invalid-provenance.err"

if "$ROOT_DIR/bin/sley" operational-workflow compare \
  --controller "$CONTROLLER" \
  --raw-aggregate "$RAW_AGGREGATE" \
  --raw-evidence "$RAW_EVIDENCE" \
  --structural-aggregate "$STRUCTURAL_AGGREGATE" \
  --structural-evidence "$STRUCTURAL_EVIDENCE" \
  --json >"$WORK_DIR/no-approval.out" 2>"$WORK_DIR/no-approval.err"; then
  echo "comparison without an approval record unexpectedly passed" >&2
  exit 1
fi
grep -Fq 'compare requires both aggregate/evidence pairs, --approval-record, and --execution-provenance' "$WORK_DIR/no-approval.err"

if [[ -n "${SLEY_OPERATIONAL_TEST_EXPORT_DIR:-}" ]]; then
  EXPORT_DIR="$SLEY_OPERATIONAL_TEST_EXPORT_DIR"
  [[ ! -e "$EXPORT_DIR" && ! -L "$EXPORT_DIR" ]] || {
    echo "operational workflow export target must not already exist" >&2
    exit 1
  }
  mkdir -p "$EXPORT_DIR/raw-evidence" "$EXPORT_DIR/structural-evidence"
  cp "$CONTROLLER" "$EXPORT_DIR/controller.py"
  cp "$RAW_AGGREGATE" "$EXPORT_DIR/raw-aggregate.json"
  cp "$STRUCTURAL_AGGREGATE" "$EXPORT_DIR/structural-aggregate.json"
  cp "$APPROVAL" "$EXPORT_DIR/approval.json"
  cp "$PROVENANCE" "$EXPORT_DIR/execution-provenance.json"
  cp "$COMPARISON" "$EXPORT_DIR/comparison.json"
  cp "$RAW_EVIDENCE"/* "$EXPORT_DIR/raw-evidence/"
  cp "$STRUCTURAL_EVIDENCE"/* "$EXPORT_DIR/structural-evidence/"
fi

echo "sley operational workflow tests passed"
