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
artifact_dir="$(mktemp -d)"
artifact_check_report="$(mktemp)"
migrate_report="$(mktemp)"
docgen_report="$(mktemp)"
workbench_report="$(mktemp)"
sandbox_report="$(mktemp)"
agent_bench_report="$(mktemp)"
zjx_tool_report="$(mktemp)"
trap 'rm -f "$bool_literal_source" "$artifact_check_report" "$migrate_report" "$docgen_report" "$workbench_report" "$sandbox_report" "$agent_bench_report" "$zjx_tool_report"; rm -rf "$artifact_dir"' EXIT
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
  | json_field '.schema == "sley.self_hosting.status.v0" and .status == "bootstrap" and .strict_self_hosted == false and .source_root == "self-hosted/src" and .semantic_source_count == (.source_modules | length) and .semantic_source_count >= 6 and (.source_modules | index("loom.bootstrap")) and (.bootstrap_owned_by_sley | index("implementation_version")) and (.bootstrap_owned_by_sley | index("implementation_stage")) and (.bootstrap_owned_by_sley | index("self_hosting_status")) and (.bootstrap_owned_by_sley | index("strict_self_hosted")) and (.bootstrap_owned_by_sley | index("source_modules")) and (.bootstrap_owned_by_sley | index("semantic_source_count_task_execution")) and (.bootstrap_owned_by_sley | index("default_lint_rules")) and (.bootstrap_owned_by_sley | index("core_report_schema_ids")) and (.bootstrap_owned_by_sley | index("diagnostic_ids")) and (.bootstrap_owned_by_sley | index("runtime_seed_values")) and (.bootstrap_owned_by_sley | index("parser_expression_classifiers")) and (.bootstrap_owned_by_sley | index("parser_classifier_task_execution")) and (.bootstrap_owned_by_sley | index("parser_id_task_execution")) and (.bootstrap_owned_by_sley | index("parser_statement_and_binding_kinds")) and (.bootstrap_owned_by_sley | index("bootstrap_smoke_runtime")) and (.bootstrap_owned_by_sley | index("bootstrap_smoke_task_execution")) and (.bootstrap_owned_by_sley | index("checker_diagnostic_status")) and (.bootstrap_owned_by_sley | index("checker_status_task_execution")) and (.bootstrap_owned_by_sley | index("checker_builtin_type_task_execution")) and (.bootstrap_owned_by_sley | index("checker_message_task_execution")) and (.bootstrap_owned_by_sley | index("checker_unknown_type_diagnostics")) and (.bootstrap_owned_by_sley | index("checker_unknown_task_diagnostics")) and (.bootstrap_owned_by_sley | index("checker_call_diagnostics")) and (.bootstrap_owned_by_sley | index("checker_duplicate_take_diagnostics")) and (.bootstrap_owned_by_sley | index("checker_collection_diagnostics")) and (.bootstrap_owned_by_sley | index("checker_record_diagnostics")) and (.bootstrap_owned_by_sley | index("checker_namespace_diagnostics")) and (.bootstrap_owned_by_sley | index("checker_effect_diagnostics")) and (.bootstrap_owned_by_sley | index("checker_effect_propagation_diagnostics")) and (.bootstrap_owned_by_sley | index("checker_question_diagnostics")) and (.bootstrap_owned_by_sley | index("checker_identifier_resolution_inputs")) and (.bootstrap_owned_by_sley | index("lint_status_task_execution")) and (.bootstrap_owned_by_sley | index("lint_finding_messages")) and (.bootstrap_owned_by_sley | index("runtime_status_task_execution")) and (.bootstrap_owned_by_sley | index("pure_literal_runtime_execution")) and (.bootstrap_owned_by_sley | index("bool_literal_runtime_execution")) and (.bootstrap_owned_by_sley | index("text_concat_runtime_execution")) and (.bootstrap_owned_by_sley | index("list_len_runtime_task_execution")) and (.bootstrap_owned_by_sley | index("len_expression_runtime_execution")) and (.bootstrap_owned_by_sley | index("if_expression_runtime_execution")) and (.bootstrap_owned_by_sley | index("if_statement_runtime_execution")) and (.bootstrap_owned_by_sley | index("seeded_agent_deploy_task_execution")) and (.bootstrap_owned_by_sley | index("project_ready_task_execution")) and (.bootstrap_owned_by_sley | index("ast_runtime_probe_dispatch")) and (.bootstrap_owned_by_sley | index("self_hosting_report_shape")) and (.bootstrap_owned_by_sley | index("runtime_report_status_and_dispatch"))'

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
  | json_field '(.bootstrap_owned_by_sley | index("ci_report_shape"))'

bin/sley self-hosting-status --json \
  | json_field '(.bootstrap_owned_by_sley | index("contract_report_shapes"))'

bin/sley self-hosting-status --json \
  | json_field '(.bootstrap_owned_by_sley | index("deploy_report_shape"))'

bin/sley self-hosting-status --json \
  | json_field '(.bootstrap_owned_by_sley | index("deploy_artifact_check_report_shape"))'

bin/sley self-hosting-status --json \
  | json_field '(.bootstrap_owned_by_sley | index("migrate_report_shape"))'

bin/sley self-hosting-status --json \
  | json_field '(.bootstrap_owned_by_sley | index("docgen_report_shape"))'

bin/sley self-hosting-status --json \
  | json_field '(.bootstrap_owned_by_sley | index("workbench_report_shape"))'

bin/sley self-hosting-status --json \
  | json_field '(.bootstrap_owned_by_sley | index("sandbox_report_shape"))'

bin/sley self-hosting-status --json \
  | json_field '(.bootstrap_owned_by_sley | index("agent_bench_report_shape"))'

bin/sley self-hosting-status --json \
  | json_field '(.bootstrap_owned_by_sley | index("zjx_tool_report_shape"))'

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

bin/sley self-hosting-status --json \
  | json_field '(.bootstrap_owned_by_sley | index("list_index_runtime_task_execution"))'

bin/sley self-hosting-status --json \
  | json_field '(.bootstrap_owned_by_sley | index("list_index_expression_runtime_execution"))'

bin/sley self-hosting-status --json \
  | json_field '(.bootstrap_owned_by_sley | index("map_index_runtime_task_execution"))'

bin/sley self-hosting-status --json \
  | json_field '(.bootstrap_owned_by_sley | index("map_index_expression_runtime_execution"))'

bin/sley self-hosting-status --json \
  | json_field '(.bootstrap_owned_by_sley | index("record_field_runtime_task_execution"))'

bin/sley self-hosting-status --json \
  | json_field '(.bootstrap_owned_by_sley | index("record_field_expression_runtime_execution"))'

bin/sley self-hosting-status --json \
  | json_field '(.bootstrap_owned_by_sley | index("record_field_call_runtime_execution"))'

bin/sley run --json fixtures/corpus/accepted/records_and_calls.sley \
  | json_field '.status == "passed" and .value.kind == "Text" and .value.value == "Ada"'

bin/sley self-hosting-status --json \
  | json_field '(.bootstrap_owned_by_sley | index("bool_comparison_runtime_task_execution"))'

bin/sley self-hosting-status --json \
  | json_field '(.bootstrap_owned_by_sley | index("bool_comparison_runtime_execution"))'

bin/sley self-hosting-status --json \
  | json_field '(.bootstrap_owned_by_sley | index("bool_and_runtime_task_execution"))'

bin/sley self-hosting-status --json \
  | json_field '(.bootstrap_owned_by_sley | index("bool_and_expression_runtime_execution"))'

bin/sley self-hosting-status --json \
  | json_field '(.bootstrap_owned_by_sley | index("bool_not_runtime_task_execution"))'

bin/sley self-hosting-status --json \
  | json_field '(.bootstrap_owned_by_sley | index("not_expression_runtime_execution"))'

bin/sley self-hosting-status --json \
  | json_field '(.bootstrap_owned_by_sley | index("double_not_expression_runtime_execution"))'

bin/sley self-hosting-status --json \
  | json_field '(.bootstrap_owned_by_sley | index("negated_comparison_runtime_execution"))'

bin/sley self-hosting-status --json \
  | json_field '(.bootstrap_owned_by_sley | index("int_arithmetic_runtime_task_execution"))'

bin/sley self-hosting-status --json \
  | json_field '(.bootstrap_owned_by_sley | index("constant_arithmetic_runtime_execution"))'

bin/sley self-hosting-status --json \
  | json_field '(.bootstrap_owned_by_sley | index("absorbing_arithmetic_runtime_execution"))'

bin/sley self-hosting-status --json \
  | json_field '(.bootstrap_owned_by_sley | index("bound_arithmetic_runtime_execution"))'

bin/sley self-hosting-status --json \
  | json_field '(.bootstrap_owned_by_sley | index("int_comparison_runtime_task_execution"))'

bin/sley self-hosting-status --json \
  | json_field '(.bootstrap_owned_by_sley | index("constant_comparison_runtime_execution"))'

bin/sley self-hosting-status --json \
  | json_field '(.bootstrap_owned_by_sley | index("bool_if_runtime_task_execution"))'

bin/sley self-hosting-status --json \
  | json_field '(.bootstrap_owned_by_sley | index("bool_if_expression_runtime_execution"))'

bin/sley self-hosting-status --json \
  | json_field '(.bootstrap_owned_by_sley | index("bool_if_statement_runtime_execution"))'

bin/sley self-hosting-status --json \
  | json_field '(.bootstrap_owned_by_sley | index("int_if_runtime_task_execution"))'

bin/sley self-hosting-status --json \
  | json_field '(.bootstrap_owned_by_sley | index("int_if_expression_runtime_execution"))'

bin/sley self-hosting-status --json \
  | json_field '(.bootstrap_owned_by_sley | index("int_if_statement_runtime_execution"))'

bin/sley self-hosting-status --json \
  | json_field '(.bootstrap_owned_by_sley | index("literal_if_statement_runtime_execution"))'

bin/sley self-hosting-status --json \
  | json_field '(.bootstrap_owned_by_sley | index("comparison_if_statement_runtime_execution"))'

bin/sley self-hosting-status --json \
  | json_field '(.bootstrap_owned_by_sley | index("constant_false_while_runtime_execution"))'

bin/sley self-hosting-status --json \
  | json_field '(.bootstrap_owned_by_sley | index("state_set_runtime_execution"))'

bin/sley self-hosting-status --json \
  | json_field '(.bootstrap_owned_by_sley | index("tally_set_runtime_execution"))'

bin/sley self-hosting-status --json \
  | json_field '(.bootstrap_owned_by_sley | index("project_call_runtime_execution"))'

bin/sley self-hosting-status --json \
  | json_field '(.bootstrap_owned_by_sley | index("unqualified_project_call_runtime_execution"))'

bin/sley run --json examples/unqualified_import_call_project \
  | json_field '.status == "passed" and .value.kind == "Int" and .value.value == 42'

bin/sley self-hosting-status --json \
  | json_field '(.bootstrap_owned_by_sley | index("bound_local_call_runtime_execution"))'

bin/sley run --json fixtures/corpus/accepted/pure_main.sley \
  | json_field '.status == "passed" and .value.kind == "Int" and .value.value == 42'

bin/sley self-hosting-status --json \
  | json_field '(.bootstrap_owned_by_sley | index("int_identity_runtime_task_execution"))'

bin/sley self-hosting-status --json \
  | json_field '(.bootstrap_owned_by_sley | index("bound_int_return_runtime_execution"))'

bin/sley ast --json examples/hello.sley \
  | json_field '.schema == "sley.ast.program.v0" and .module == "app.hello" and .tasks[0].id == "task:app.hello.main" and .tasks[0].name == "main" and .tasks[0].body.statements[0].id == "block:task:app.hello.main:stmt:0" and .tasks[0].body.statements[0].expr.expr_kind == "StringLiteral"'

bin/sley ast --json examples/constant_text_concatenation_expression.sley \
  | json_field '.schema == "sley.ast.program.v0" and .tasks[0].body.statements[0].expr.expr_kind == "Raw" and .tasks[0].body.statements[0].expr.source == "\"Sley \" + \"agents\""'

bin/sley ast --json examples/empty_for_statement.sley \
  | json_field '.schema == "sley.ast.program.v0" and .tasks[0].body.statements[1].collection.expr_kind == "ListLiteral" and .tasks[0].body.statements[1].collection.items == []'

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

bin/sley run --json examples/constant_text_concatenation_expression.sley \
  | json_field '.schema == "sley.run.report.v0" and .value.kind == "Text" and .value.value == "Sley agents"'

bin/sley run --json examples/constant_len_expression.sley \
  | json_field '.schema == "sley.run.report.v0" and .value.kind == "Int" and .value.value == 3'

bin/sley run --json examples/constant_arithmetic_expression.sley \
  | json_field '.schema == "sley.run.report.v0" and .value.kind == "Int" and .value.value == 5'

bin/sley run --json examples/absorbing_arithmetic_expression.sley \
  | json_field '.schema == "sley.run.report.v0" and .value.kind == "Int" and .value.value == 0'

bin/sley run --json examples/identity_binary_expression.sley \
  | json_field '.schema == "sley.run.report.v0" and .value.kind == "Int" and .value.value == 41'

bin/sley run --json examples/unreachable_statement.sley \
  | json_field '.schema == "sley.run.report.v0" and .value.kind == "Int" and .value.value == 41'

bin/sley run --json examples/unused_pure_expression_statement.sley \
  | json_field '.schema == "sley.run.report.v0" and .value.kind == "Int" and .value.value == 41'

bin/sley run --json examples/constant_list_index_expression.sley \
  | json_field '.schema == "sley.run.report.v0" and .value.kind == "Int" and .value.value == 20'

bin/sley run --json examples/constant_map_index_expression.sley \
  | json_field '.schema == "sley.run.report.v0" and .value.kind == "Text" and .value.value == "done"'

bin/sley run --json examples/constant_record_field_access_expression.sley \
  | json_field '.schema == "sley.run.report.v0" and .value.kind == "Text" and .value.value == "Ada"'

bin/sley run --json examples/constant_boolean_comparison_expression.sley \
  | json_field '.schema == "sley.run.report.v0" and .value.kind == "Bool" and .value.value == false'

bin/sley run --json examples/constant_comparison_expression.sley \
  | json_field '.schema == "sley.run.report.v0" and .value.kind == "Bool" and .value.value == true'

bin/sley run --json examples/redundant_boolean_comparison.sley \
  | json_field '.schema == "sley.run.report.v0" and .value.kind == "Bool" and .value.value == true'

bin/sley run --json examples/self_comparison_expression.sley \
  | json_field '.schema == "sley.run.report.v0" and .value.kind == "Bool" and .value.value == true'

bin/sley run --json examples/absorbing_boolean_expression.sley \
  | json_field '.schema == "sley.run.report.v0" and .value.kind == "Bool" and .value.value == false'

bin/sley run --json examples/idempotent_boolean_expression.sley \
  | json_field '.schema == "sley.run.report.v0" and .value.kind == "Bool" and .value.value == true'

bin/sley run --json examples/constant_not_expression.sley \
  | json_field '.schema == "sley.run.report.v0" and .value.kind == "Bool" and .value.value == false'

bin/sley run --json examples/double_negation_expression.sley \
  | json_field '.schema == "sley.run.report.v0" and .value.kind == "Bool" and .value.value == true'

bin/sley run --json examples/negated_comparison_expression.sley \
  | json_field '.schema == "sley.run.report.v0" and .value.kind == "Bool" and .value.value == false'

bin/sley run --json examples/redundant_boolean_if_expression.sley \
  | json_field '.schema == "sley.run.report.v0" and .value.kind == "Bool" and .value.value == true'

bin/sley run --json examples/redundant_boolean_if_statement.sley \
  | json_field '.schema == "sley.run.report.v0" and .value.kind == "Bool" and .value.value == true'

bin/sley run --json examples/constant_if_expression.sley \
  | json_field '.schema == "sley.run.report.v0" and .value.kind == "Int" and .value.value == 41'

bin/sley run --json examples/same_branch_if_expression.sley \
  | json_field '.schema == "sley.run.report.v0" and .value.kind == "Int" and .value.value == 42'

bin/sley run --json examples/constant_if_statement.sley \
  | json_field '.schema == "sley.run.report.v0" and .value.kind == "Int" and .value.value == 42'

bin/sley run --json examples/constant_false_if_statement.sley \
  | json_field '.schema == "sley.run.report.v0" and .value.kind == "Int" and .value.value == 42'

bin/sley run --json examples/same_branch_if_statement.sley \
  | json_field '.schema == "sley.run.report.v0" and .value.kind == "Int" and .value.value == 41'

bin/sley run --json examples/constant_false_while_statement.sley \
  | json_field '.schema == "sley.run.report.v0" and .value.kind == "Int" and .value.value == 41'

bin/sley run --json examples/redundant_initial_set_statement.sley \
  | json_field '.schema == "sley.run.report.v0" and .value.kind == "Int" and .value.value == 2'

bin/sley run --json examples/self_assignment_statement.sley \
  | json_field '.schema == "sley.run.report.v0" and .value.kind == "Int" and .value.value == 2'

bin/sley run --json examples/mutable_binding_style.sley \
  | json_field '.schema == "sley.run.report.v0" and .value.kind == "Int" and .value.value == 21'

bin/sley run --json self-hosted \
  | json_field '.schema == "sley.run.report.v0" and .value.kind == "Int" and .value.value == 51'

bin/sley run --json examples/project \
  | json_field '.schema == "sley.run.report.v0" and .value.kind == "Int" and .value.value == 42'

bin/sley run --json --cap SecretRead --secret api_key redacted --cap Network --http-text https://example.test/profile "owned profile" --cap ModelCall --model-output deploy-plan "owned plan" --cap Deploy --deploy-result staging "owned stage" fixtures/corpus/accepted/agent_deploy_pipeline.sley \
  | json_field '.schema == "sley.run.report.v0" and .value.kind == "Ok" and .value.value.value == "owned profile | owned plan | owned stage"'

bin/sley verify --json examples/project \
  | json_field '.schema == "sley.verify.report.v0" and .status == "passed" and .runtime.value.value == 42'

bin/sley deploy --json --dry-run --artifacts-dir /tmp/sley-deploy-report-shape examples/hello.sley \
  | json_field '.schema == "sley.deploy.report.v0" and .status == "ready" and .mode == "dry_run" and .target == "examples/hello.sley" and .policy.live_deploy_allowed == false and .verify.schema == "sley.verify.report.v0" and .summary.verify_status == "passed"'

bin/sley deploy --json --dry-run --artifacts-dir /tmp/sley-deploy-report-shape examples/hello.sley \
  | json_field 'keys == (["schema","status","mode","target","environment","policy","summary","verify","seal","package","artifacts","next_actions"] | sort)'

printf '%s\n' '{"schema":"sley.deploy.report.v0"}' > "$artifact_dir/deploy-report.json"
printf '%s\n' '{"schema":"sley.trace.seal.v0"}' > "$artifact_dir/seal.json"
printf '%s\n' '{"schema":"sley.zjx.envelope.v0"}' > "$artifact_dir/zjx-envelope.json"
report_digest="sha256:$(sha256sum "$artifact_dir/deploy-report.json" | awk '{print $1}')"
seal_digest="sha256:$(sha256sum "$artifact_dir/seal.json" | awk '{print $1}')"
package_digest="sha256:$(sha256sum "$artifact_dir/zjx-envelope.json" | awk '{print $1}')"
jq -n \
  --arg artifact_dir "$artifact_dir" \
  --arg report_digest "$report_digest" \
  --arg seal_digest "$seal_digest" \
  --arg package_digest "$package_digest" '
  {
    schema:"sley.deploy.artifacts.v0",
    target:"examples/hello.sley",
    environment:"staging",
    mode:"dry_run",
    policy:{
      live_deploy_allowed:false,
      external_mutations:false,
      provider_calls:false,
      requires_operator_approval:true
    },
    summary:{verify_status:"passed"},
    files:{
      report:{path:($artifact_dir + "/deploy-report.json"), schema:"sley.deploy.report.v0", digest:$report_digest},
      seal:{path:($artifact_dir + "/seal.json"), schema:"sley.trace.seal.v0", digest:$seal_digest},
      package:{path:($artifact_dir + "/zjx-envelope.json"), schema:"sley.zjx.envelope.v0", digest:$package_digest}
    }
  }' > "$artifact_dir/manifest.json"

bin/sley-contract inspect-deploy-artifacts "$artifact_dir" --schemas docs/schemas --json > "$artifact_check_report"
json_field '.schema == "sley.deploy.artifact_check.v0" and .status == "passed" and .validation_level == "json_schema_draft_2020_12" and .manifest_schema == "sley.deploy.artifacts.v0" and .summary.file_count == 3 and .summary.passed_count == 3 and .summary.failed_count == 0 and .summary.issue_count == 0' < "$artifact_check_report"
json_field 'keys == (["schema","status","validation_level","artifacts_dir","manifest_path","manifest_schema","summary","files","issues"] | sort)' < "$artifact_check_report"

bin/sley-contract validate --schema sley.deploy.artifact_check.v0 "$artifact_check_report" --schemas docs/schemas --json \
  | json_field '.schema == "sley.contract.validate.v0" and .status == "passed" and .requested_schema == "sley.deploy.artifact_check.v0" and .report_schema == "sley.deploy.artifact_check.v0"'

bin/sley-migrate report --json examples/raw_host_migration.sley > "$migrate_report"
json_field '.schema == "sley.migrate.report.v0" and .status == "migrations" and .source_schema == "sley.edit_plan.report.v0" and .summary.migration_count == 1 and .summary.raw_host_adapter_count == 1 and .migrations[0].kind == "migrate_raw_host_adapter" and .migrations[0].operation.op == "ReplaceExpression"' < "$migrate_report"
json_field 'keys == (["schema","status","target","source_schema","summary","migrations","schema_drift","diagnostics","issues"] | sort)' < "$migrate_report"

bin/sley-contract validate --schema sley.migrate.report.v0 "$migrate_report" --schemas docs/schemas --json \
  | json_field '.schema == "sley.contract.validate.v0" and .status == "passed" and .requested_schema == "sley.migrate.report.v0" and .report_schema == "sley.migrate.report.v0"'

bin/sley-docgen reference --json --module agent.pipeline examples/agent_project > "$docgen_report"
json_field '.schema == "sley.docgen.report.v0" and .status == "generated" and .source_schema == "sley.query.report.v0" and .filters.module == "agent.pipeline" and .filters.exported_only == false and .summary.module_count == 1 and .summary.task_count == 3 and .summary.capability_count == 10 and .documents[0].title == "Sley Reference: agent.pipeline" and .tasks[0].qualified_name == "agent.pipeline.collect_profile" and .tasks[0].inbound_call_count == 1' < "$docgen_report"
json_field 'keys == (["schema","status","target","source_schema","summary","filters","documents","modules","tasks","types","effects","capabilities","diagnostics","issues"] | sort)' < "$docgen_report"

bin/sley-contract validate --schema sley.docgen.report.v0 "$docgen_report" --schemas docs/schemas --json \
  | json_field '.schema == "sley.contract.validate.v0" and .status == "passed" and .requested_schema == "sley.docgen.report.v0" and .report_schema == "sley.docgen.report.v0"'

if bin/sley-docgen reference --json --module agent.typo examples/agent_project > "$docgen_report"; then
  fail "docgen unknown module filter passed"
fi
json_field '.schema == "sley.docgen.report.v0" and .status == "blocked" and .filters.module == "agent.typo" and .summary.module_count == 0 and .summary.task_count == 0 and .summary.issue_count == 1 and .issues[0].code == "DOCGEN_MODULE_FILTER_NOT_FOUND"' < "$docgen_report"

bin/sley-contract validate --schema sley.docgen.report.v0 "$docgen_report" --schemas docs/schemas --json \
  | json_field '.schema == "sley.contract.validate.v0" and .status == "passed" and .requested_schema == "sley.docgen.report.v0" and .report_schema == "sley.docgen.report.v0"'

bin/sley-workbench --json --slice task:app.agent_deploy_pipeline.main examples/agent_deploy_pipeline.sley > "$workbench_report"
json_field '.schema == "sley.workbench.report.v0" and .status == "ready" and .summary.module_count == 1 and .summary.call_count == 7 and .graph_slice.schema == "sley.symbol_graph.slice.v0" and .graph_slice.focus.id == "task:app.agent_deploy_pipeline.main"' < "$workbench_report"

bin/sley-contract validate --schema sley.workbench.report.v0 "$workbench_report" --schemas docs/schemas --json \
  | json_field '.schema == "sley.contract.validate.v0" and .status == "passed" and .requested_schema == "sley.workbench.report.v0" and .report_schema == "sley.workbench.report.v0"'

bin/sley-workbench --json examples/unused_private_task.sley > "$workbench_report"
json_field '.schema == "sley.workbench.report.v0" and .status == "warnings" and .lint.status == "findings" and .lint.findings[0].id == "UNUSED_PRIVATE_TASK" and .lint.findings[0].plan_command[1] == "plan" and .lint.findings[0].plan_command[5] == "task:app.tasks.orphan" and .lint.findings[0].plan_command[6] == "examples/unused_private_task.sley"' < "$workbench_report"

bin/sley-contract validate --schema sley.workbench.report.v0 "$workbench_report" --schemas docs/schemas --json \
  | json_field '.schema == "sley.contract.validate.v0" and .status == "passed" and .requested_schema == "sley.workbench.report.v0" and .report_schema == "sley.workbench.report.v0"'

bin/sley-sandbox-runner run --json fixtures/contracts/sandbox_manifest_agent_pipeline.json > "$sandbox_report"
json_field '.schema == "sley.sandbox.report.v0" and .status == "warnings" and .manifest_schema == "sley.sandbox.manifest.v0" and .target == "examples/agent_deploy_pipeline.sley" and .summary.capability_count == 4 and .summary.seed_count == 4 and .summary.issue_count == 1 and .verify.schema == "sley.verify.report.v0" and .verify.status == "passed" and .issues[0].code == "SANDBOX_OS_ISOLATION_NOT_ENFORCED"' < "$sandbox_report"

bin/sley-contract validate --schema sley.sandbox.report.v0 "$sandbox_report" --schemas docs/schemas --json \
  | json_field '.schema == "sley.contract.validate.v0" and .status == "passed" and .requested_schema == "sley.sandbox.report.v0" and .report_schema == "sley.sandbox.report.v0"'

bin/sley-agent-bench run --json --case unused-private-task-repair > "$agent_bench_report"
json_field '.schema == "sley.agent_bench.report.v0" and .status == "passed" and .case_count == 1 and .passed_count == 1 and .failed_count == 0 and .cases[0].name == "unused-private-task-repair" and .cases[0].status == "passed" and .cases[0].trace_receipt_count == 1' < "$agent_bench_report"

bin/sley-contract validate --schema sley.agent_bench.report.v0 "$agent_bench_report" --schemas docs/schemas --json \
  | json_field '.schema == "sley.contract.validate.v0" and .status == "passed" and .requested_schema == "sley.agent_bench.report.v0" and .report_schema == "sley.agent_bench.report.v0"'

bin/sley-zjx inspect --json fixtures/contracts/zjx_hello_ready.json > "$zjx_tool_report"
json_field '.schema == "sley.zjx.tool.report.v0" and .status == "passed" and .command == "inspect" and .digest.matches == true and .envelopes[0].graph_digest_match == true' < "$zjx_tool_report"

bin/sley-contract validate --schema sley.zjx.tool.report.v0 "$zjx_tool_report" --schemas docs/schemas --json \
  | json_field '.schema == "sley.contract.validate.v0" and .status == "passed" and .requested_schema == "sley.zjx.tool.report.v0" and .report_schema == "sley.zjx.tool.report.v0"'

bin/sley graft --json --dry-run fixtures/ci_smoke_probe/graft_target.sley fixtures/ci_smoke_probe/insert_statement.json \
  | json_field '.schema == "sley.graft.outcome.v0" and .status == "accepted"'

bin/sley graft --json --dry-run fixtures/ci_smoke_probe/graft_target.sley fixtures/ci_smoke_probe/insert_statement.json \
  | json_field 'keys == (["schema","status","source","diagnostics","provenance"] | sort)'

bin/sley claim-verify --json docs/SleyClaimManifest.json \
  | json_field '.schema == "sley.claim.verify.v0" and .status == "passed" and .failed_count == 0'

bin/sley claim-verify --json docs/SleyClaimManifest.json \
  | json_field 'keys == (["schema","status","target","manifest_schema","claim_id","check_count","passed_count","failed_count","checks","issues"] | sort)'

bin/sley-ci smoke --json --repo-root "$PWD" fixtures/ci_smoke_probe/manifest.json \
  | json_field '.schema == "sley.ci.report.v0" and .status == "passed" and .command == "smoke" and .manifest == "fixtures/ci_smoke_probe/manifest.json" and .summary.failed_count == 0'

bin/sley-ci smoke --json --repo-root "$PWD" fixtures/ci_smoke_probe/manifest.json \
  | json_field 'keys == (["schema","status","command","manifest","summary","steps","issues"] | sort)'

bin/sley-contract inventory --json \
  | json_field '.schema == "sley.contract.inventory.v0" and .status == "passed" and .schema_count > 0'

bin/sley-contract inventory --json \
  | json_field 'keys == (["schema","status","schema_dir","schema_count","schemas"] | sort)'

bin/sley-contract validate --schema sley.query.report.v0 fixtures/contracts/query_project_tasks.json --schemas docs/schemas --json \
  | json_field '.schema == "sley.contract.validate.v0" and .status == "passed" and .requested_schema == "sley.query.report.v0" and .report_schema == "sley.query.report.v0"'

bin/sley-contract validate --schema sley.query.report.v0 fixtures/contracts/query_project_tasks.json --schemas docs/schemas --json \
  | json_field 'keys == (["schema","status","validation_level","requested_schema","report_path","schema_dir","report_schema","issues"] | sort)'

bin/sley-contract check-fixtures fixtures/contracts --schemas docs/schemas --json \
  | json_field '.schema == "sley.contract.fixture_check.v0" and .status == "passed" and .failed_count == 0'

bin/sley-contract check-fixtures fixtures/contracts --schemas docs/schemas --json \
  | json_field 'keys == (["schema","status","validation_level","schema_dir","fixtures_dir","fixture_count","passed_count","failed_count","fixtures"] | sort)'

bin/sley-conformance report --json \
  | json_field '.schema == "sley.conformance.report.v0" and .status == "passed" and .summary.test_count_matches_declared == true'

bin/sley-conformance report --json \
  | json_field 'keys == (["schema","status","summary","validation","schemas","corpus","smoke","onboarding","examples","tests","editor_shims","v1_gate","readiness","release","issues"] | sort)'

bin/sley-conformance coverage --json --require-tag cli:parse \
  | json_field '.schema == "sley.conformance.coverage.v0" and .status == "passed" and (.required_tags | index("cli:parse"))'

bin/sley-conformance coverage --json --require-tag cli:parse \
  | json_field 'keys == (["schema","status","required_tags","missing_tags","tag_inventory","issues"] | sort)'

echo "Self-hosting bootstrap tests passed."
