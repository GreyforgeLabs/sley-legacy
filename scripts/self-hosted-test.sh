#!/usr/bin/env bash
set -euo pipefail

ROOT_DIR="$(cd "$(dirname "$0")/.." && pwd)"
cd "$ROOT_DIR"
export PATH="$ROOT_DIR/bin:$PATH"

fail() {
  echo "self-hosted test failed: $*" >&2
  exit 1
}

json_field() {
  jq -er "$1" >/dev/null
}

bool_literal_source="$(mktemp)"
trap 'rm -f "$bool_literal_source"' EXIT
printf '%s\n' \
  'module app.bool_literal' \
  '' \
  'task main -> Bool {' \
  '  return true' \
  '}' > "$bool_literal_source"

bash -n bin/sley
./scripts/check-self-hosted-code.sh

bin/sley --help >/dev/null
bin/sley --version | grep -q 'self-hosting-stage2-source'

bin/sley self-hosting-status --json \
  | json_field '.schema == "sley.self_hosting.status.v0" and .status == "bootstrap" and .strict_self_hosted == false and .source_root == "self-hosted/src" and .semantic_source_count >= 6 and (.source_modules | index("loom.bootstrap")) and (.bootstrap_owned_by_sley | index("implementation_version")) and (.bootstrap_owned_by_sley | index("implementation_stage")) and (.bootstrap_owned_by_sley | index("self_hosting_status")) and (.bootstrap_owned_by_sley | index("strict_self_hosted")) and (.bootstrap_owned_by_sley | index("source_modules")) and (.bootstrap_owned_by_sley | index("default_lint_rules")) and (.bootstrap_owned_by_sley | index("core_report_schema_ids")) and (.bootstrap_owned_by_sley | index("diagnostic_ids")) and (.bootstrap_owned_by_sley | index("runtime_seed_values")) and (.bootstrap_owned_by_sley | index("parser_expression_classifiers")) and (.bootstrap_owned_by_sley | index("parser_statement_and_binding_kinds")) and (.bootstrap_owned_by_sley | index("bootstrap_smoke_runtime")) and (.bootstrap_owned_by_sley | index("bootstrap_smoke_task_execution")) and (.bootstrap_owned_by_sley | index("checker_diagnostic_status")) and (.bootstrap_owned_by_sley | index("checker_status_task_execution")) and (.bootstrap_owned_by_sley | index("checker_builtin_type_task_execution")) and (.bootstrap_owned_by_sley | index("checker_unknown_type_diagnostics")) and (.bootstrap_owned_by_sley | index("checker_unknown_task_diagnostics")) and (.bootstrap_owned_by_sley | index("checker_call_diagnostics")) and (.bootstrap_owned_by_sley | index("checker_duplicate_take_diagnostics")) and (.bootstrap_owned_by_sley | index("checker_collection_diagnostics")) and (.bootstrap_owned_by_sley | index("checker_record_diagnostics")) and (.bootstrap_owned_by_sley | index("checker_namespace_diagnostics")) and (.bootstrap_owned_by_sley | index("checker_effect_diagnostics")) and (.bootstrap_owned_by_sley | index("checker_effect_propagation_diagnostics")) and (.bootstrap_owned_by_sley | index("checker_question_diagnostics")) and (.bootstrap_owned_by_sley | index("checker_identifier_resolution_inputs")) and (.bootstrap_owned_by_sley | index("lint_status_task_execution")) and (.bootstrap_owned_by_sley | index("lint_finding_messages")) and (.bootstrap_owned_by_sley | index("runtime_status_task_execution")) and (.bootstrap_owned_by_sley | index("pure_literal_runtime_execution")) and (.bootstrap_owned_by_sley | index("bool_literal_runtime_execution")) and (.bootstrap_owned_by_sley | index("seeded_agent_deploy_task_execution")) and (.bootstrap_owned_by_sley | index("project_ready_task_execution")) and (.bootstrap_owned_by_sley | index("ast_runtime_probe_dispatch")) and (.bootstrap_owned_by_sley | index("self_hosting_report_shape")) and (.bootstrap_owned_by_sley | index("runtime_report_status_and_dispatch"))'

bin/sley self-hosting-status --json \
  | json_field '(.bootstrap_owned_by_sley | index("run_report_shape"))'

bin/sley self-hosting-status --json \
  | json_field '(.bootstrap_owned_by_sley | index("verify_report_shape"))'

bin/sley self-hosting-status --json \
  | json_field '(.bootstrap_owned_by_sley | index("graft_outcome_report_shape"))'

bin/sley self-hosting-status --json \
  | json_field '(.bootstrap_owned_by_sley | index("claim_verify_report_shape"))'

bin/sley self-hosting-status --json \
  | json_field '(.bootstrap_owned_by_sley | index("conformance_report_shape"))'

bin/sley self-hosting-status --json \
  | json_field '(.bootstrap_owned_by_sley | index("conformance_coverage_report_shape"))'

bin/sley self-hosting-status --json \
  | json_field '(.bootstrap_owned_by_sley | index("diagnostics_report_shape"))'

bin/sley self-hosting-status --json \
  | json_field '(.bootstrap_owned_by_sley | index("lint_report_shape"))'

bin/sley self-hosting-status --json \
  | json_field '(.bootstrap_owned_by_sley | index("query_report_shape"))'

bin/sley self-hosting-status --json \
  | json_field '(.bootstrap_owned_by_sley | index("symbol_graph_report_shape"))'

bin/sley self-hosting-status --json \
  | json_field '(.bootstrap_owned_by_sley | index("doctor_report_shape"))'

bin/sley ast --json examples/hello.sley \
  | json_field '.schema == "sley.ast.program.v0" and .module == "app.hello" and .tasks[0].name == "main" and .tasks[0].body.statements[0].expr.expr_kind == "StringLiteral"'

bin/sley ast --json --node block:task:app.hello.main:stmt:0:expr examples/hello.sley \
  | json_field '.schema == "sley.ast.node.v0" and .node_kind == "expression"'

bin/sley ast --json fixtures/corpus/rejected/question_requires_result.sley \
  | json_field '.tasks[0].body.statements[0].kind == "Expr" and .tasks[0].body.statements[0].expr.fallible == true'

bin/sley check --json examples/hello.sley \
  | json_field '.schema == "sley.diagnostics.report.v0" and .status == "ok"'

bin/sley check --json examples/hello.sley \
  | json_field 'keys == (["schema","status","diagnostics"] | sort)'

if bin/sley check --json fixtures/corpus/rejected/unknown_identifier.sley >/tmp/sley-rejected-check.json; then
  fail "rejected unknown_identifier.sley passed check"
fi
jq -er '.status == "error" and .diagnostics[0].id == "UNKNOWN_IDENTIFIER"' /tmp/sley-rejected-check.json >/dev/null

if bin/sley check --json fixtures/corpus/rejected/unknown_type.sley >/tmp/sley-rejected-type-check.json; then
  fail "rejected unknown_type.sley passed check"
fi
jq -er '.status == "error" and .diagnostics[0].id == "UNKNOWN_TYPE"' /tmp/sley-rejected-type-check.json >/dev/null

if bin/sley check --json fixtures/corpus/rejected/unknown_task.sley >/tmp/sley-rejected-task-check.json; then
  fail "rejected unknown_task.sley passed check"
fi
jq -er '.status == "error" and .diagnostics[0].id == "UNKNOWN_TASK" and .diagnostics[0].message == "unknown task `missing`"' /tmp/sley-rejected-task-check.json >/dev/null

if bin/sley check --json fixtures/corpus/rejected/call_arity_mismatch.sley >/tmp/sley-rejected-call-arity-check.json; then
  fail "rejected call_arity_mismatch.sley passed check"
fi
jq -er '.status == "error" and .diagnostics[0].id == "CALL_ARITY_MISMATCH" and .diagnostics[0].message == "call arity mismatch for `helper`"' /tmp/sley-rejected-call-arity-check.json >/dev/null

if bin/sley check --json fixtures/corpus/rejected/call_argument_type_mismatch.sley >/tmp/sley-rejected-call-type-check.json; then
  fail "rejected call_argument_type_mismatch.sley passed check"
fi
jq -er '.status == "error" and .diagnostics[0].id == "CALL_ARGUMENT_TYPE_MISMATCH" and .diagnostics[0].message == "call argument type mismatch for `helper`"' /tmp/sley-rejected-call-type-check.json >/dev/null

if bin/sley check --json fixtures/corpus/rejected/type_mismatch.sley >/tmp/sley-rejected-type-mismatch-check.json; then
  fail "rejected type_mismatch.sley passed check"
fi
jq -er '.status == "error" and [.diagnostics[].id] == ["TYPE_MISMATCH", "RETURN_TYPE_MISMATCH"] and .diagnostics[0].message == "type mismatch `label`" and .diagnostics[1].message == "return type mismatch `corpus.rejected.main`"' /tmp/sley-rejected-type-mismatch-check.json >/dev/null

if bin/sley check --json fixtures/corpus/rejected/type_alias_mismatch.sley >/tmp/sley-rejected-type-alias-mismatch-check.json; then
  fail "rejected type_alias_mismatch.sley passed check"
fi
jq -er '.status == "error" and .diagnostics[0].id == "RETURN_TYPE_MISMATCH" and .diagnostics[0].message == "return type mismatch `corpus.rejected.type_alias_mismatch.main`"' /tmp/sley-rejected-type-alias-mismatch-check.json >/dev/null

if bin/sley check --json fixtures/corpus/rejected/duplicate_take.sley >/tmp/sley-rejected-duplicate-take-check.json; then
  fail "rejected duplicate_take.sley passed check"
fi
jq -er '.status == "error" and .diagnostics[0].id == "DUPLICATE_TAKE" and .diagnostics[0].message == "duplicate take `value`"' /tmp/sley-rejected-duplicate-take-check.json >/dev/null

if bin/sley check --json fixtures/corpus/rejected/module_namespace_conflict.sley >/tmp/sley-rejected-module-namespace-check.json; then
  fail "rejected module_namespace_conflict.sley passed check"
fi
jq -er '.status == "error" and [.diagnostics[].id] == ["DUPLICATE_EFFECT", "DUPLICATE_TYPE", "DUPLICATE_TASK"] and .diagnostics[0].message == "duplicate effect `Audit`" and .diagnostics[1].message == "duplicate type `User`" and .diagnostics[2].message == "duplicate task `main`"' /tmp/sley-rejected-module-namespace-check.json >/dev/null

if bin/sley check --json fixtures/corpus/rejected/unknown_effect.sley >/tmp/sley-rejected-unknown-effect-check.json; then
  fail "rejected unknown_effect.sley passed check"
fi
jq -er '.status == "error" and .diagnostics[0].id == "UNKNOWN_EFFECT" and .diagnostics[0].message == "unknown effect `MissingEffect`"' /tmp/sley-rejected-unknown-effect-check.json >/dev/null

if bin/sley check --json fixtures/corpus/rejected/gate_take_type_mismatch.sley >/tmp/sley-rejected-gate-take-type-check.json; then
  fail "rejected gate_take_type_mismatch.sley passed check"
fi
jq -er '.status == "error" and .diagnostics[0].id == "GATE_TAKE_TYPE_MISMATCH" and .diagnostics[0].message == "gate take must use Gate<Effect>, got `Int`"' /tmp/sley-rejected-gate-take-type-check.json >/dev/null

if bin/sley check --json fixtures/corpus/rejected/gate_effect_undeclared.sley >/tmp/sley-rejected-gate-effect-check.json; then
  fail "rejected gate_effect_undeclared.sley passed check"
fi
jq -er '.status == "error" and .diagnostics[0].id == "GATE_EFFECT_UNDECLARED" and .diagnostics[0].message == "gate effect undeclared `FileRead`"' /tmp/sley-rejected-gate-effect-check.json >/dev/null

if bin/sley check --json fixtures/corpus/rejected/unauthorized_effect.sley >/tmp/sley-rejected-unauthorized-effect-check.json; then
  fail "rejected unauthorized_effect.sley passed check"
fi
jq -er '.status == "error" and .diagnostics[0].id == "EFFECT_UNAUTHORIZED" and .diagnostics[0].message == "effect unauthorized `FileRead`"' /tmp/sley-rejected-unauthorized-effect-check.json >/dev/null

if bin/sley check --json fixtures/corpus/rejected/authority/missing_database_read_effect.sley >/tmp/sley-rejected-missing-db-read-effect-check.json; then
  fail "rejected missing_database_read_effect.sley passed check"
fi
jq -er '.status == "error" and .diagnostics[0].id == "EFFECT_UNAUTHORIZED" and .diagnostics[0].message == "effect unauthorized `DatabaseRead`"' /tmp/sley-rejected-missing-db-read-effect-check.json >/dev/null

if bin/sley check --json fixtures/corpus/rejected/authority/missing_transitive_agent_effect.sley >/tmp/sley-rejected-missing-transitive-agent-effect-check.json; then
  fail "rejected missing_transitive_agent_effect.sley passed check"
fi
jq -er '.status == "error" and .diagnostics[0].id == "EFFECT_UNAUTHORIZED" and .diagnostics[0].message == "effect unauthorized `Network`"' /tmp/sley-rejected-missing-transitive-agent-effect-check.json >/dev/null

if bin/sley check --json fixtures/corpus/rejected/question_requires_result.sley >/tmp/sley-rejected-question-result-check.json; then
  fail "rejected question_requires_result.sley passed check"
fi
jq -er '.status == "error" and .diagnostics[0].id == "QUESTION_REQUIRES_RESULT" and .diagnostics[0].message == "question operator requires Result return in task `corpus.rejected.question_requires_result.main`"' /tmp/sley-rejected-question-result-check.json >/dev/null

if bin/sley check --json fixtures/corpus/rejected/duplicate_record_field.sley >/tmp/sley-rejected-duplicate-record-field-check.json; then
  fail "rejected duplicate_record_field.sley passed check"
fi
jq -er '.status == "error" and .diagnostics[0].id == "DUPLICATE_FIELD" and .diagnostics[0].message == "duplicate record field `name`"' /tmp/sley-rejected-duplicate-record-field-check.json >/dev/null

if bin/sley check --json fixtures/corpus/rejected/duplicate_record_literal_field.sley >/tmp/sley-rejected-duplicate-record-literal-field-check.json; then
  fail "rejected duplicate_record_literal_field.sley passed check"
fi
jq -er '.status == "error" and .diagnostics[0].id == "DUPLICATE_RECORD_LITERAL_FIELD" and .diagnostics[0].message == "duplicate record literal field `name`"' /tmp/sley-rejected-duplicate-record-literal-field-check.json >/dev/null

if bin/sley check --json fixtures/corpus/rejected/record_field_missing.sley >/tmp/sley-rejected-record-field-missing-check.json; then
  fail "rejected record_field_missing.sley passed check"
fi
jq -er '.status == "error" and .diagnostics[0].id == "RECORD_FIELD_MISSING" and .diagnostics[0].message == "record field missing `age`"' /tmp/sley-rejected-record-field-missing-check.json >/dev/null

if bin/sley check --json fixtures/corpus/rejected/record_field_unknown.sley >/tmp/sley-rejected-record-field-unknown-check.json; then
  fail "rejected record_field_unknown.sley passed check"
fi
jq -er '.status == "error" and .diagnostics[0].id == "RECORD_FIELD_UNKNOWN" and .diagnostics[0].message == "record field unknown `active`"' /tmp/sley-rejected-record-field-unknown-check.json >/dev/null

if bin/sley check --json fixtures/corpus/rejected/record_field_type_mismatch.sley >/tmp/sley-rejected-record-field-type-check.json; then
  fail "rejected record_field_type_mismatch.sley passed check"
fi
jq -er '.status == "error" and .diagnostics[0].id == "RECORD_FIELD_TYPE_MISMATCH" and .diagnostics[0].message == "record field type mismatch `age`"' /tmp/sley-rejected-record-field-type-check.json >/dev/null

if bin/sley check --json fixtures/corpus/rejected/record_literal_non_record_type.sley >/tmp/sley-rejected-record-non-record-check.json; then
  fail "rejected record_literal_non_record_type.sley passed check"
fi
jq -er '.status == "error" and .diagnostics[0].id == "RECORD_LITERAL_NON_RECORD_TYPE" and .diagnostics[0].message == "record literal type is not a record `UserId`"' /tmp/sley-rejected-record-non-record-check.json >/dev/null

if bin/sley check --json fixtures/corpus/rejected/unknown_record_field.sley >/tmp/sley-rejected-unknown-record-field-check.json; then
  fail "rejected unknown_record_field.sley passed check"
fi
jq -er '.status == "error" and .diagnostics[0].id == "UNKNOWN_RECORD_FIELD" and .diagnostics[0].message == "unknown record field `email`"' /tmp/sley-rejected-unknown-record-field-check.json >/dev/null

sidecar_oracle_hits="$(
  find fixtures/corpus/rejected -name '*.sley' -print | sort | while read -r rejected_fixture; do
    if bin/sley check --json "$rejected_fixture" >/tmp/sley-rejected-sidecar-oracle-check.json 2>/dev/null; then
      :
    fi
    if jq -e '.diagnostics[]?.message | startswith("rejected corpus fixture expects diagnostic")' /tmp/sley-rejected-sidecar-oracle-check.json >/dev/null; then
      printf '%s\n' "$rejected_fixture"
    fi
  done
)"
if [[ -n "$sidecar_oracle_hits" ]]; then
  fail "rejected fixtures still use diagnostic sidecar oracle: $sidecar_oracle_hits"
fi

bin/sley query --json --kind calls examples/project \
  | json_field '.schema == "sley.query.report.v0" and .calls[0].target == "app.math.double"'

bin/sley query --json --kind calls examples/project \
  | json_field 'keys == (["schema","kind","entry_module","filters","modules","tasks","types","effects","calls"] | sort)'

bin/sley graph --json examples/project \
  | json_field '.schema == "sley.symbol_graph.v0" and .entry_module == "app.main" and .modules[0].imports[0].id == "import:app.main:app.math"'

bin/sley graph --json examples/project \
  | json_field 'keys == (["schema","entry_module","modules"] | sort)'

bin/sley doctor --json examples/project \
  | json_field '.schema == "sley.doctor.report.v0" and .status == "ready" and .summary.task_count == 2'

bin/sley doctor --json examples/project \
  | json_field 'keys == (["schema","status","target","entry_module","summary","diagnostics","query","lint","next_actions"] | sort)'

bin/sley lint --json examples/empty_for_statement.sley \
  | json_field '.schema == "sley.lint.report.v0" and .status == "findings" and .findings[0].id == "EMPTY_FOR_STATEMENT" and (.filters.rules | index("empty_for_statement"))'

bin/sley lint --json examples/empty_for_statement.sley \
  | json_field 'keys == (["schema","status","entry_module","filters","findings"] | sort)'

bin/sley run --json examples/hello.sley \
  | json_field '.schema == "sley.run.report.v0" and .value.kind == "Text" and .value.value == "hello sley"'

bin/sley run --json examples/hello.sley \
  | json_field 'keys == (["schema","status","target","value","diagnostics"] | sort)'

bin/sley run --json "$bool_literal_source" \
  | json_field '.schema == "sley.run.report.v0" and .value.kind == "Bool" and .value.value == true'

bin/sley run --json self-hosted \
  | json_field '.schema == "sley.run.report.v0" and .value.kind == "Int" and .value.value == 51'

bin/sley run --json examples/project \
  | json_field '.schema == "sley.run.report.v0" and .value.kind == "Int" and .value.value == 42'

bin/sley run --json --cap SecretRead --secret api_key redacted --cap Network --http-text https://example.test/profile "owned profile" --cap ModelCall --model-output deploy-plan "owned plan" --cap Deploy --deploy-result staging "owned stage" fixtures/corpus/accepted/agent_deploy_pipeline.sley \
  | json_field '.schema == "sley.run.report.v0" and .value.kind == "Ok" and .value.value.value == "owned profile | owned plan | owned stage"'

bin/sley verify --json examples/project \
  | json_field '.schema == "sley.verify.report.v0" and .status == "passed" and .runtime.value.value == 42'

bin/sley graft --json --dry-run fixtures/ci_smoke_probe/graft_target.sley fixtures/ci_smoke_probe/insert_statement.json \
  | json_field '.schema == "sley.graft.outcome.v0" and .status == "accepted"'

bin/sley graft --json --dry-run fixtures/ci_smoke_probe/graft_target.sley fixtures/ci_smoke_probe/insert_statement.json \
  | json_field 'keys == (["schema","status","source","diagnostics","provenance"] | sort)'

bin/sley claim-verify --json docs/SleyClaimManifest.json \
  | json_field '.schema == "sley.claim.verify.v0" and .status == "passed" and .failed_count == 0'

bin/sley claim-verify --json docs/SleyClaimManifest.json \
  | json_field 'keys == (["schema","status","target","manifest_schema","claim_id","check_count","passed_count","failed_count","checks","issues"] | sort)'

bin/sley-contract inventory --json \
  | json_field '.schema == "sley.contract.inventory.v0" and .status == "passed" and .schema_count > 0'

bin/sley-contract check-fixtures fixtures/contracts --schemas docs/schemas --json \
  | json_field '.schema == "sley.contract.fixture_check.v0" and .status == "passed" and .failed_count == 0'

bin/sley-conformance report --json \
  | json_field '.schema == "sley.conformance.report.v0" and .status == "passed" and .summary.test_count_matches_declared == true'

bin/sley-conformance report --json \
  | json_field 'keys == (["schema","status","summary","validation","schemas","corpus","smoke","onboarding","examples","tests","editor_shims","v1_gate","readiness","release","issues"] | sort)'

bin/sley-conformance coverage --json --require-tag cli:parse \
  | json_field '.schema == "sley.conformance.coverage.v0" and .status == "passed" and (.required_tags | index("cli:parse"))'

bin/sley-conformance coverage --json --require-tag cli:parse \
  | json_field 'keys == (["schema","status","required_tags","missing_tags","tag_inventory","issues"] | sort)'

echo "Self-hosting bootstrap tests passed."
