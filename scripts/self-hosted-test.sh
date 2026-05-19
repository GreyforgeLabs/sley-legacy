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
empty_module_source="$(mktemp)"
add_take_operation="$(mktemp)"
artifact_dir="$(mktemp -d)"
artifact_check_report="$(mktemp)"
migrate_report="$(mktemp)"
docgen_report="$(mktemp)"
workbench_report="$(mktemp)"
sandbox_report="$(mktemp)"
agent_bench_report="$(mktemp)"
zjx_tool_report="$(mktemp)"
runtime_report="$(mktemp)"
ast_missing_report="$(mktemp)"
ci_report="$(mktemp)"
trap 'rm -f "$bool_literal_source" "$empty_module_source" "$add_take_operation" "$artifact_check_report" "$migrate_report" "$docgen_report" "$workbench_report" "$sandbox_report" "$agent_bench_report" "$zjx_tool_report" "$runtime_report" "$ast_missing_report" "$ci_report"; rm -rf "$artifact_dir"' EXIT
printf '%s\n' \
  'module app.bool_literal' \
  '' \
  'task main -> Bool {' \
  '  return true' \
  '}' > "$bool_literal_source"
printf '%s\n' \
  'module app.empty' > "$empty_module_source"
printf '%s\n' \
  '{"op":"AddTake","target":"","payload":{"name":"value","type":"Text"}}' > "$add_take_operation"

bash -n bin/sley
./scripts/check-self-hosted-code.sh

bin/sley --help >/dev/null
bin/sley --version | grep -q 'self-hosting-stage2-source'

bin/sley self-hosting-status --json \
  | json_field '.schema == "sley.self_hosting.status.v0" and .status == "bootstrap" and .strict_self_hosted == false and .source_root == "self-hosted/src" and .semantic_source_count == (.source_modules | length) and .semantic_source_count >= 6 and (.source_modules | index("loom.bootstrap")) and (.bootstrap_owned_by_sley | index("implementation_version")) and (.bootstrap_owned_by_sley | index("implementation_stage")) and (.bootstrap_owned_by_sley | index("self_hosting_status")) and (.bootstrap_owned_by_sley | index("strict_self_hosted")) and (.bootstrap_owned_by_sley | index("source_modules")) and (.bootstrap_owned_by_sley | index("semantic_source_count_task_execution")) and (.bootstrap_owned_by_sley | index("default_lint_rules")) and (.bootstrap_owned_by_sley | index("core_report_schema_ids")) and (.bootstrap_owned_by_sley | index("diagnostic_ids")) and (.bootstrap_owned_by_sley | index("runtime_seed_values")) and (.bootstrap_owned_by_sley | index("parser_expression_classifiers")) and (.bootstrap_owned_by_sley | index("parser_classifier_task_execution")) and (.bootstrap_owned_by_sley | index("parser_id_task_execution")) and (.bootstrap_owned_by_sley | index("parser_call_expression_prefix_task_execution")) and (.bootstrap_owned_by_sley | index("query_call_expression_prefix_task_execution")) and (.bootstrap_owned_by_sley | index("symbol_graph_call_expression_prefix_task_execution")) and (.bootstrap_owned_by_sley | index("parser_statement_and_binding_kinds")) and (.bootstrap_owned_by_sley | index("bootstrap_smoke_runtime")) and (.bootstrap_owned_by_sley | index("bootstrap_smoke_task_execution")) and (.bootstrap_owned_by_sley | index("checker_diagnostic_status")) and (.bootstrap_owned_by_sley | index("checker_status_task_execution")) and (.bootstrap_owned_by_sley | index("checker_builtin_type_task_execution")) and (.bootstrap_owned_by_sley | index("checker_builtin_types_task_execution")) and (.bootstrap_owned_by_sley | index("checker_static_type_names_task_execution")) and (.bootstrap_owned_by_sley | index("checker_effect_aliases_task_execution")) and (.bootstrap_owned_by_sley | index("checker_host_effect_needles_task_execution")) and (.bootstrap_owned_by_sley | index("checker_gate_binding_kind_task_execution")) and (.bootstrap_owned_by_sley | index("checker_repair_hint_kind_task_execution")) and (.bootstrap_owned_by_sley | index("checker_identifier_expr_kind_task_execution")) and (.bootstrap_owned_by_sley | index("checker_call_expression_prefix_task_execution")) and (.bootstrap_owned_by_sley | index("checker_message_task_execution")) and (.bootstrap_owned_by_sley | index("checker_unknown_type_diagnostics")) and (.bootstrap_owned_by_sley | index("checker_unknown_task_diagnostics")) and (.bootstrap_owned_by_sley | index("checker_call_diagnostics")) and (.bootstrap_owned_by_sley | index("checker_duplicate_take_diagnostics")) and (.bootstrap_owned_by_sley | index("checker_collection_diagnostics")) and (.bootstrap_owned_by_sley | index("checker_record_diagnostics")) and (.bootstrap_owned_by_sley | index("checker_namespace_diagnostics")) and (.bootstrap_owned_by_sley | index("checker_effect_diagnostics")) and (.bootstrap_owned_by_sley | index("checker_effect_propagation_diagnostics")) and (.bootstrap_owned_by_sley | index("checker_question_diagnostics")) and (.bootstrap_owned_by_sley | index("checker_identifier_resolution_inputs")) and (.bootstrap_owned_by_sley | index("lint_status_task_execution")) and (.bootstrap_owned_by_sley | index("lint_finding_messages")) and (.bootstrap_owned_by_sley | index("runtime_status_task_execution")) and (.bootstrap_owned_by_sley | index("pure_literal_runtime_execution")) and (.bootstrap_owned_by_sley | index("bool_literal_runtime_execution")) and (.bootstrap_owned_by_sley | index("text_concat_runtime_execution")) and (.bootstrap_owned_by_sley | index("list_len_runtime_task_execution")) and (.bootstrap_owned_by_sley | index("len_expression_runtime_execution")) and (.bootstrap_owned_by_sley | index("if_expression_runtime_execution")) and (.bootstrap_owned_by_sley | index("if_statement_runtime_execution")) and (.bootstrap_owned_by_sley | index("seeded_agent_deploy_task_execution")) and (.bootstrap_owned_by_sley | index("project_ready_task_execution")) and (.bootstrap_owned_by_sley | index("ast_runtime_probe_dispatch")) and (.bootstrap_owned_by_sley | index("self_hosting_report_shape")) and (.bootstrap_owned_by_sley | index("runtime_report_status_and_dispatch"))'

bin/sley self-hosting-status --json \
  | json_field '(.bootstrap_owned_by_sley | index("runtime_host_effect_needles_dispatch_execution")) and (.bootstrap_owned_by_sley | index("runtime_authority_host_effect_needles_execution"))'

bin/sley self-hosting-status --json \
  | json_field '(.bootstrap_owned_by_sley | index("runtime_authority_effect_aliases_task_execution"))'

bin/sley self-hosting-status --json \
  | json_field '(.bootstrap_owned_by_sley | index("runtime_diagnostic_messages_task_execution"))'

bin/sley self-hosting-status --json \
  | json_field '(.bootstrap_owned_by_sley | index("runtime_diagnostic_fallback_removal_task_execution"))'

if bin/sley run --json --cap DbRead=orders --db-table users=examples/users.json examples/db_gate.sley > "$runtime_report"; then
  fail "database read with a denied runtime scope should fail"
fi
jq -er '.status == "error" and .diagnostics[0].id == "RUNTIME_CAPABILITY_SCOPE_DENIED" and .diagnostics[0].message == "DatabaseRead/DbRead scope does not allow `users`"' "$runtime_report" >/dev/null

if grep -Eq 'RUNTIME_(CAPABILITY_REQUIRED_ID|CAPABILITY_SCOPE_DENIED_ID|TAKE_REQUIRED_ID|CAPABILITY_REQUIRED_MESSAGE_PREFIX|CAPABILITY_SCOPE_DENIED_MESSAGE_MIDDLE|CAPABILITY_SCOPE_DENIED_MESSAGE_SUFFIX|TAKE_REQUIRED_MESSAGE_PREFIX|TAKE_REQUIRED_MESSAGE_SUFFIX)="\$\{RUNTIME_' bin/sley; then
  fail "runtime diagnostic messages must come from loom.runtime without shell fallback literals"
fi

if grep -Eq 'os\.environ\.get\("SLEY_RUNTIME_(DEFAULT_DATABASE_TABLE|CAPABILITY_REQUIRED_ID|CAPABILITY_SCOPE_DENIED_ID|CAPABILITY_REQUIRED_MESSAGE_PREFIX|CAPABILITY_SCOPE_DENIED_MESSAGE_MIDDLE|CAPABILITY_SCOPE_DENIED_MESSAGE_SUFFIX|TAKE_REQUIRED_ID|TAKE_REQUIRED_MESSAGE_PREFIX|TAKE_REQUIRED_MESSAGE_SUFFIX)",' bin/sley; then
  fail "runtime diagnostic values must not fall back to Python host literals"
fi

bin/sley self-hosting-status --json \
  | json_field '(.bootstrap_owned_by_sley | index("runtime_default_database_table_task_execution"))'

bin/sley self-hosting-status --json \
  | json_field '(.bootstrap_owned_by_sley | index("runtime_value_kinds_task_execution"))'

if grep -Eq 'RUNTIME_(INT|TEXT|BOOL|RAW|UNIT|OK_TEXT|OK_INT)_VALUE_KIND="\$\{RUNTIME_[A-Z_]+:-' bin/sley; then
  fail "runtime value-kind names must come from loom.runtime without shell fallback literals"
fi

bin/sley self-hosting-status --json \
  | json_field '(.bootstrap_owned_by_sley | index("runtime_self_hosted_target_aliases_task_execution"))'

bin/sley run --json self-hosted/ \
  | json_field '.status == "passed" and .value.kind == "Int" and .value.value >= 1'

bin/sley run --json self-hosted/sley.toml \
  | json_field '.status == "passed" and .value.kind == "Int" and .value.value >= 1'

if grep -Fq "RUNTIME_SELF_HOSTED_TARGETS_JSON='[\"self-hosted\"" bin/sley; then
  fail "self-hosted runtime target aliases must come from loom.runtime without shell fallback literals"
fi

bin/sley self-hosting-status --json \
  | json_field '(.bootstrap_owned_by_sley | index("runtime_status_names_task_execution"))'

bin/sley run --json examples/hello.sley \
  | json_field '.status == "passed" and .value.kind == "Text" and .value.value == "hello sley"'

if grep -Eq 'RUNTIME_(PASSED|FAILED|SKIPPED)_STATUS="\$\{RUNTIME_(PASSED|FAILED|SKIPPED)_STATUS:-' bin/sley; then
  fail "runtime status names must come from loom.runtime without shell fallback literals"
fi

bin/sley self-hosting-status --json \
  | json_field '(.bootstrap_owned_by_sley | index("runtime_target_probe_sources_task_execution"))'

bin/sley run --json examples/project \
  | json_field '.status == "passed" and .value.kind == "Int" and .value.value == 42'

bin/sley run --json --cap SecretRead --cap Network --cap ModelCall --cap Deploy examples/agent_deploy_pipeline.sley \
  | json_field '.status == "passed" and .value.kind == "Ok" and .value.value.kind == "Text" and .value.value.value == "profile ready | plan approved | staged"'

if grep -Fq 'RUNTIME_AGENT_DEPLOY_SUFFIX="${RUNTIME_AGENT_DEPLOY_SUFFIX:-' bin/sley \
  || grep -Fq 'RUNTIME_PROJECT_READY_CALL_PROBE="${RUNTIME_PROJECT_READY_CALL_PROBE:-' bin/sley \
  || grep -Fq 'RUNTIME_PROJECT_READY_BINDING_PROBE="${RUNTIME_PROJECT_READY_BINDING_PROBE:-' bin/sley; then
  fail "runtime target probes must come from loom.runtime without shell fallback literals"
fi

bin/sley self-hosting-status --json \
  | json_field '(.bootstrap_owned_by_sley | index("runtime_default_value_sources_task_execution"))'

bin/sley run --json examples/hello.sley \
  | json_field '.status == "passed" and .value.kind == "Text" and .value.value == "hello sley"'

bin/sley run --json examples/raw_host_migration.sley \
  | json_field '.status == "passed" and .value.kind == "Ok" and .value.value.kind == "Text" and .value.value.value == "hello sley"'

bin/sley run --json --cap Shell --shell-output date "owned date" examples/shell_gate.sley \
  | json_field '.status == "passed" and .value.kind == "Ok" and .value.value.kind == "Text" and .value.value.value == "owned date"'

if grep -Eq 'RUNTIME_(HELLO_VALUE|PROJECT_READY_VALUE|DEFAULT_PROFILE|DEFAULT_MODEL_PLAN|DEFAULT_DEPLOY_RESULT|DEFAULT_RAW_VALUE|DEFAULT_FILE_WRITE_TEXT|DEFAULT_DATABASE_WRITE_TEXT|DEFAULT_AGENT_DATA_WRITE_TEXT|DEFAULT_DATABASE_READ_TEXT|DEFAULT_DATABASE_TABLE|DEFAULT_SHELL_TEXT|DEFAULT_SECRET_TEXT|DEFAULT_SPEND_AUTHORIZATION_TEXT)="\$\{RUNTIME_' bin/sley; then
  fail "runtime default values must come from loom.runtime without shell fallback literals"
fi

bin/sley self-hosting-status --json \
  | json_field '(.bootstrap_owned_by_sley | index("lint_declared_effect_aliases_task_execution"))'

bin/sley self-hosting-status --json \
  | json_field '(.bootstrap_owned_by_sley | index("checker_call_parser_prefix_task_execution"))'

bin/sley self-hosting-status --json \
  | json_field '(.bootstrap_owned_by_sley | index("checker_message_parser_prefix_task_execution"))'

bin/sley self-hosting-status --json \
  | json_field '(.bootstrap_owned_by_sley | index("checker_status_parser_prefix_task_execution"))'

bin/sley self-hosting-status --json \
  | json_field '(.bootstrap_owned_by_sley | index("parser_classifier_parser_prefix_task_execution"))'

bin/sley self-hosting-status --json \
  | json_field '(.bootstrap_owned_by_sley | index("semantic_source_count_parser_prefix_task_execution")) and (.bootstrap_owned_by_sley | index("bootstrap_smoke_parser_prefix_task_execution"))'

bin/sley self-hosting-status --json \
  | json_field '(.bootstrap_owned_by_sley | index("self_hosting_report_builder")) and (.bootstrap_owned_by_sley | index("self_hosting_report_builder_task_execution")) and ((.blockers | index("extend Sley-owned report builders across remaining command reports")) == null) and ((.blockers | index("replace shell JSON shaping with Sley-owned report builders")) == null)'

bin/sley self-hosting-status --json \
  | json_field '(.bootstrap_owned_by_sley | index("parser_declaration_id_task_execution")) and (.bootstrap_owned_by_sley | index("parser_take_id_task_execution"))'

bin/sley self-hosting-status --json \
  | json_field '(.bootstrap_owned_by_sley | index("parser_take_surface_id_task_execution"))'

bin/sley self-hosting-status --json \
  | json_field '(.bootstrap_owned_by_sley | index("parser_module_surface_id_task_execution"))'

bin/sley self-hosting-status --json \
  | json_field '(.bootstrap_owned_by_sley | index("parser_module_task_list_surface_id_task_execution"))'

bin/sley self-hosting-status --json \
  | json_field '(.bootstrap_owned_by_sley | index("parser_module_declaration_list_surface_id_task_execution"))'

bin/sley self-hosting-status --json \
  | json_field '(.bootstrap_owned_by_sley | index("parser_block_task_surface_id_task_execution"))'

bin/sley self-hosting-status --json \
  | json_field '(.bootstrap_owned_by_sley | index("parser_block_fallback_surface_id_task_execution"))'

bin/sley self-hosting-status --json \
  | json_field '(.bootstrap_owned_by_sley | index("parser_call_target_task_id_task_execution"))'

bin/sley self-hosting-status --json \
  | json_field '(.bootstrap_owned_by_sley | index("parser_call_argument_surface_id_task_execution"))'

bin/sley self-hosting-status --json \
  | json_field '(.bootstrap_owned_by_sley | index("parser_expression_side_surface_id_task_execution"))'

bin/sley self-hosting-status --json \
  | json_field '(.bootstrap_owned_by_sley | index("parser_task_statement_surface_id_task_execution"))'

bin/sley self-hosting-status --json \
  | json_field '(.bootstrap_owned_by_sley | index("parser_task_fallback_surface_id_task_execution"))'

bin/sley self-hosting-status --json \
  | json_field '(.bootstrap_owned_by_sley | index("parser_branch_statement_surface_id_task_execution"))'

bin/sley self-hosting-status --json \
  | json_field '(.bootstrap_owned_by_sley | index("parser_lint_statement_surface_id_task_execution"))'

bin/sley self-hosting-status --json \
  | json_field '(.bootstrap_owned_by_sley | index("parser_migrate_surface_id_task_execution"))'

bin/sley self-hosting-status --json \
  | json_field '(.bootstrap_owned_by_sley | index("parser_fix_migration_surface_id_task_execution"))'

bin/sley self-hosting-status --json \
  | json_field '(.bootstrap_owned_by_sley | index("migrate_qualify_call_parser_prefix_task_execution")) and (.bootstrap_owned_by_sley | index("fix_qualify_call_parser_prefix_task_execution"))'

bin/sley self-hosting-status --json \
  | json_field '(.bootstrap_owned_by_sley | index("symbol_graph_call_arg_parser_prefix_task_execution")) and (.bootstrap_owned_by_sley | index("plan_call_arg_parser_prefix_task_execution"))'

bin/sley self-hosting-status --json \
  | json_field '(.bootstrap_owned_by_sley | index("fix_update_call_sites_parser_prefix_task_execution")) and (.bootstrap_owned_by_sley | index("fix_remove_call_arg_parser_prefix_task_execution"))'

bin/sley self-hosting-status --json \
  | json_field '(.bootstrap_owned_by_sley | index("parser_fix_style_surface_id_task_execution"))'

bin/sley self-hosting-status --json \
  | json_field '(.bootstrap_owned_by_sley | index("parser_fix_empty_surface_id_task_execution"))'

bin/sley self-hosting-status --json \
  | json_field '(.bootstrap_owned_by_sley | index("parser_fix_unused_unreachable_surface_id_task_execution"))'

bin/sley self-hosting-status --json \
  | json_field '(.bootstrap_owned_by_sley | index("parser_fix_constant_control_surface_id_task_execution"))'

bin/sley self-hosting-status --json \
  | json_field '(.bootstrap_owned_by_sley | index("parser_fix_constant_scalar_surface_id_task_execution"))'

bin/sley self-hosting-status --json \
  | json_field '(.bootstrap_owned_by_sley | index("parser_fix_constant_derived_surface_id_task_execution"))'

bin/sley self-hosting-status --json \
  | json_field '(.bootstrap_owned_by_sley | index("parser_fix_algebra_boolean_surface_id_task_execution"))'

bin/sley self-hosting-status --json \
  | json_field '(.bootstrap_owned_by_sley | index("parser_fix_boolean_branch_surface_id_task_execution"))'

bin/sley self-hosting-status --json \
  | json_field '(.bootstrap_owned_by_sley | index("parser_expression_id_task_execution")) and (.bootstrap_owned_by_sley | index("parser_control_expression_id_task_execution"))'

bin/sley self-hosting-status --json \
  | json_field '(.bootstrap_owned_by_sley | index("diagnostics_report_builder")) and (.bootstrap_owned_by_sley | index("diagnostics_report_builder_task_execution")) and (.bootstrap_owned_by_sley | index("run_report_builder")) and (.bootstrap_owned_by_sley | index("run_report_builder_task_execution"))'

bin/sley self-hosting-status --json \
  | json_field '(.bootstrap_owned_by_sley | index("query_report_builder")) and (.bootstrap_owned_by_sley | index("query_report_builder_task_execution")) and (.bootstrap_owned_by_sley | index("lint_report_builder")) and (.bootstrap_owned_by_sley | index("lint_report_builder_task_execution")) and (.bootstrap_owned_by_sley | index("doctor_report_builder")) and (.bootstrap_owned_by_sley | index("doctor_report_builder_task_execution")) and (.bootstrap_owned_by_sley | index("verify_report_builder")) and (.bootstrap_owned_by_sley | index("verify_report_builder_task_execution"))'

bin/sley self-hosting-status --json \
  | json_field '(.bootstrap_owned_by_sley | index("parser_ast_program_report_builder")) and (.bootstrap_owned_by_sley | index("parser_ast_program_report_builder_task_execution"))'

bin/sley self-hosting-status --json \
  | json_field '(.bootstrap_owned_by_sley | index("parser_expression_statement_parser_prefix_task_execution"))'

bin/sley self-hosting-status --json \
  | json_field '(.bootstrap_owned_by_sley | index("parser_ast_node_report_builder")) and (.bootstrap_owned_by_sley | index("parser_ast_node_report_builder_task_execution")) and (.bootstrap_owned_by_sley | index("parser_ast_node_kind_task_execution")) and (.bootstrap_owned_by_sley | index("parser_ast_node_not_found_diagnostic"))'

bin/sley self-hosting-status --json \
  | json_field '(.bootstrap_owned_by_sley | index("parser_ast_node_parent_id_task_execution"))'

bin/sley self-hosting-status --json \
  | json_field '(.bootstrap_owned_by_sley | index("parser_ast_node_not_found_message"))'

bin/sley self-hosting-status --json \
  | json_field '(.bootstrap_owned_by_sley | index("parser_message_parser_prefix_task_execution"))'

bin/sley self-hosting-status --json \
  | json_field '(.bootstrap_owned_by_sley | index("symbol_graph_report_builder")) and (.bootstrap_owned_by_sley | index("symbol_graph_report_builder_task_execution")) and (.bootstrap_owned_by_sley | index("claim_verify_report_builder")) and (.bootstrap_owned_by_sley | index("claim_verify_report_builder_task_execution")) and (.bootstrap_owned_by_sley | index("migrate_report_builder")) and (.bootstrap_owned_by_sley | index("migrate_report_builder_task_execution")) and (.bootstrap_owned_by_sley | index("docgen_report_builder")) and (.bootstrap_owned_by_sley | index("docgen_report_builder_task_execution")) and (.bootstrap_owned_by_sley | index("sandbox_report_builder")) and (.bootstrap_owned_by_sley | index("sandbox_report_builder_task_execution")) and (.bootstrap_owned_by_sley | index("agent_bench_report_builder")) and (.bootstrap_owned_by_sley | index("agent_bench_report_builder_task_execution")) and (.bootstrap_owned_by_sley | index("deploy_report_builder")) and (.bootstrap_owned_by_sley | index("deploy_report_builder_task_execution"))'

bin/sley self-hosting-status --json \
  | json_field '(.bootstrap_owned_by_sley | index("conformance_report_builder")) and (.bootstrap_owned_by_sley | index("conformance_report_builder_task_execution")) and (.bootstrap_owned_by_sley | index("conformance_coverage_report_builder")) and (.bootstrap_owned_by_sley | index("conformance_coverage_report_builder_task_execution"))'

bin/sley self-hosting-status --json \
  | json_field '(.bootstrap_owned_by_sley | index("graft_outcome_report_builder")) and (.bootstrap_owned_by_sley | index("graft_outcome_report_builder_task_execution")) and (.bootstrap_owned_by_sley | index("ci_report_builder")) and (.bootstrap_owned_by_sley | index("ci_report_builder_task_execution")) and (.bootstrap_owned_by_sley | index("contract_inventory_report_builder")) and (.bootstrap_owned_by_sley | index("contract_inventory_report_builder_task_execution")) and (.bootstrap_owned_by_sley | index("contract_validate_report_builder")) and (.bootstrap_owned_by_sley | index("contract_validate_report_builder_task_execution")) and (.bootstrap_owned_by_sley | index("contract_fixture_check_report_builder")) and (.bootstrap_owned_by_sley | index("contract_fixture_check_report_builder_task_execution")) and (.bootstrap_owned_by_sley | index("deploy_artifact_check_report_builder")) and (.bootstrap_owned_by_sley | index("deploy_artifact_check_report_builder_task_execution")) and (.bootstrap_owned_by_sley | index("workbench_report_builder")) and (.bootstrap_owned_by_sley | index("workbench_report_builder_task_execution")) and (.bootstrap_owned_by_sley | index("zjx_tool_report_builder")) and (.bootstrap_owned_by_sley | index("zjx_tool_report_builder_task_execution"))'

bin/sley self-hosting-status --json \
  | json_field '(.bootstrap_owned_by_sley | index("run_report_shape"))'

bin/sley self-hosting-status --json \
  | json_field '(.bootstrap_owned_by_sley | index("runtime_status_parser_prefix_task_execution"))'

bin/sley self-hosting-status --json \
  | json_field '(.bootstrap_owned_by_sley | index("runtime_project_probe_parser_prefix_task_execution")) and (.bootstrap_owned_by_sley | index("runtime_project_binding_parser_prefix_task_execution"))'

bin/sley self-hosting-status --json \
  | json_field '(.bootstrap_owned_by_sley | index("runtime_project_probe_fallback_parser_prefix_task_execution")) and (.bootstrap_owned_by_sley | index("runtime_project_binding_fallback_parser_prefix_task_execution"))'

bin/sley self-hosting-status --json \
  | json_field '(.bootstrap_owned_by_sley | index("unit_runtime_task_execution"))'

bin/sley self-hosting-status --json \
  | json_field '(.bootstrap_owned_by_sley | index("unit_main_runtime_execution"))'

bin/sley run --json examples/declaration_hygiene.sley \
  | json_field '.status == "passed" and .value.kind == "Unit" and (.value | has("value") | not)'

bin/sley self-hosting-status --json \
  | json_field '(.bootstrap_owned_by_sley | index("main_take_runtime_diagnostic"))'

if bin/sley run --json examples/unused_take.sley > "$runtime_report"; then
  fail "main with ordinary runtime takes should not execute without arguments"
fi
jq -er '.schema == "sley.diagnostics.report.v0" and .status == "error" and .diagnostics[0].id == "RUNTIME_TAKE_REQUIRED" and .diagnostics[0].message == "`main` requires unsupported runtime take `value`"' "$runtime_report" >/dev/null

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
  | json_field '(.bootstrap_owned_by_sley | index("docgen_capabilities_host_effect_needles_task_execution"))'

bin/sley self-hosting-status --json \
  | json_field '(.bootstrap_owned_by_sley | index("docgen_call_expression_prefix_task_execution"))'

bin/sley self-hosting-status --json \
  | json_field '(.bootstrap_owned_by_sley | index("docgen_call_tail_parser_prefix_task_execution"))'

bin/sley self-hosting-status --json \
  | json_field '(.bootstrap_owned_by_sley | index("workbench_report_shape"))'

bin/sley self-hosting-status --json \
  | json_field '(.bootstrap_owned_by_sley | index("workbench_call_expression_prefix_task_execution"))'

bin/sley self-hosting-status --json \
  | json_field '(.bootstrap_owned_by_sley | index("workbench_call_tail_parser_prefix_task_execution"))'

bin/sley self-hosting-status --json \
  | json_field '(.bootstrap_owned_by_sley | index("sandbox_report_shape"))'

bin/sley self-hosting-status --json \
  | json_field '(.bootstrap_owned_by_sley | index("agent_bench_report_shape"))'

bin/sley self-hosting-status --json \
  | json_field '(.bootstrap_owned_by_sley | index("zjx_tool_report_shape"))'

bin/sley self-hosting-status --json \
  | json_field '(.bootstrap_owned_by_sley | index("diagnostics_report_shape"))'

bin/sley self-hosting-status --json \
  | json_field '(.bootstrap_owned_by_sley | index("lint_status_parser_prefix_task_execution"))'

bin/sley self-hosting-status --json \
  | json_field '(.bootstrap_owned_by_sley | index("lint_report_shape"))'

bin/sley self-hosting-status --json \
  | json_field '(.bootstrap_owned_by_sley | index("lint_declared_effect_host_needles_task_execution"))'

bin/sley self-hosting-status --json \
  | json_field '(.bootstrap_owned_by_sley | index("lint_call_expression_prefix_task_execution"))'

bin/sley self-hosting-status --json \
  | json_field '(.bootstrap_owned_by_sley | index("query_report_shape"))'

bin/sley self-hosting-status --json \
  | json_field '(.bootstrap_owned_by_sley | index("symbol_graph_report_shape"))'

bin/sley self-hosting-status --json \
  | json_field '(.bootstrap_owned_by_sley | index("doctor_report_shape"))'

bin/sley self-hosting-status --json \
  | json_field '(.bootstrap_owned_by_sley | index("ok_int_runtime_task_execution"))'

bin/sley self-hosting-status --json \
  | json_field '(.bootstrap_owned_by_sley | index("ok_text_runtime_task_execution"))'

bin/sley self-hosting-status --json \
  | json_field '(.bootstrap_owned_by_sley | index("text_equal_runtime_task_execution"))'

bin/sley self-hosting-status --json \
  | json_field '(.bootstrap_owned_by_sley | index("result_flow_runtime_execution"))'

bin/sley self-hosting-status --json \
  | json_field '(.bootstrap_owned_by_sley | index("result_flow_parser_prefix_runtime_execution"))'

bin/sley run --json fixtures/corpus/accepted/result_flow.sley \
  | json_field '.status == "passed" and .value.kind == "Ok" and .value.value.kind == "Int" and .value.value.value == 42'

bin/sley self-hosting-status --json \
  | json_field '(.bootstrap_owned_by_sley | index("text_identity_runtime_task_execution"))'

bin/sley self-hosting-status --json \
  | json_field '(.bootstrap_owned_by_sley | index("host_call_text_runtime_execution"))'

bin/sley self-hosting-status --json \
  | json_field '(.bootstrap_owned_by_sley | index("runtime_host_default_texts_task_execution"))'

bin/sley self-hosting-status --json \
  | json_field '(.bootstrap_owned_by_sley | index("spend_prefix_runtime_task_execution"))'

bin/sley self-hosting-status --json \
  | json_field '(.bootstrap_owned_by_sley | index("direct_file_read_runtime_execution"))'

bin/sley self-hosting-status --json \
  | json_field '(.bootstrap_owned_by_sley | index("direct_file_read_parser_prefix_runtime_execution"))'

bin/sley self-hosting-status --json \
  | json_field '(.bootstrap_owned_by_sley | index("direct_file_read_result_runtime_execution"))'

bin/sley run --json examples/file_gate.sley \
  | json_field '.status == "passed" and .value.kind == "Text" and .value.value == "hello sley"'

bin/sley run --json examples/raw_host_migration.sley \
  | json_field '.status == "passed" and .value.kind == "Ok" and .value.value.kind == "Text" and .value.value.value == "hello sley"'

bin/sley run --json --cap Network --http-text https://example.test/profile "owned profile" examples/network_gate.sley \
  | json_field '.status == "passed" and .value.kind == "Ok" and .value.value.value == "owned profile"'

bin/sley run --json --cap Shell --shell-output date "owned date" examples/shell_gate.sley \
  | json_field '.status == "passed" and .value.kind == "Ok" and .value.value.value == "owned date"'

bin/sley run --json --cap ModelCall --model-output name "owned model" examples/model_gate.sley \
  | json_field '.status == "passed" and .value.kind == "Ok" and .value.value.value == "owned model"'

bin/sley run --json --cap SecretRead --secret api_key "owned secret" examples/secret_gate.sley \
  | json_field '.status == "passed" and .value.kind == "Ok" and .value.value.value == "owned secret"'

bin/sley run --json --cap Deploy --deploy-result staging "owned stage" examples/deploy_gate.sley \
  | json_field '.status == "passed" and .value.kind == "Ok" and .value.value.value == "owned stage"'

bin/sley run --json --cap Spend --spend-result ads-budget authorized examples/spend_gate.sley \
  | json_field '.status == "passed" and .value.kind == "Ok" and .value.value.value == "authorized"'

bin/sley run --json --cap Spend --spend-result ads-budget authorized fixtures/corpus/accepted/agent_spend_authority.sley \
  | json_field '.status == "passed" and .value.kind == "Ok" and .value.value.value == "budget gate: authorized"'

bin/sley self-hosting-status --json \
  | json_field '(.bootstrap_owned_by_sley | index("list_index_runtime_task_execution"))'

bin/sley self-hosting-status --json \
  | json_field '(.bootstrap_owned_by_sley | index("list_index_text_runtime_task_execution"))'

bin/sley self-hosting-status --json \
  | json_field '(.bootstrap_owned_by_sley | index("list_index_expression_runtime_execution"))'

bin/sley self-hosting-status --json \
  | json_field '(.bootstrap_owned_by_sley | index("bound_list_index_sum_runtime_execution"))'

bin/sley run --json fixtures/corpus/accepted/type_alias_transparency.sley \
  | json_field '.status == "passed" and .value.kind == "Int" and .value.value == 42'

bin/sley self-hosting-status --json \
  | json_field '(.bootstrap_owned_by_sley | index("map_index_runtime_task_execution"))'

bin/sley self-hosting-status --json \
  | json_field '(.bootstrap_owned_by_sley | index("map_index_int_runtime_task_execution"))'

bin/sley self-hosting-status --json \
  | json_field '(.bootstrap_owned_by_sley | index("map_index_expression_runtime_execution"))'

bin/sley self-hosting-status --json \
  | json_field '(.bootstrap_owned_by_sley | index("collection_index_sum_runtime_execution"))'

bin/sley run --json fixtures/corpus/accepted/collections_indexing.sley \
  | json_field '.status == "passed" and .value.kind == "Int" and .value.value == 8'

bin/sley self-hosting-status --json \
  | json_field '(.bootstrap_owned_by_sley | index("record_field_runtime_task_execution"))'

bin/sley self-hosting-status --json \
  | json_field '(.bootstrap_owned_by_sley | index("record_field_expression_runtime_execution"))'

bin/sley self-hosting-status --json \
  | json_field '(.bootstrap_owned_by_sley | index("record_field_call_runtime_execution"))'

bin/sley self-hosting-status --json \
  | json_field '(.bootstrap_owned_by_sley | index("record_field_call_parser_prefix_runtime_execution"))'

bin/sley run --json fixtures/corpus/accepted/records_and_calls.sley \
  | json_field '.status == "passed" and .value.kind == "Text" and .value.value == "Ada"'

bin/sley self-hosting-status --json \
  | json_field '(.bootstrap_owned_by_sley | index("local_call_runtime_execution"))'

bin/sley self-hosting-status --json \
  | json_field '(.bootstrap_owned_by_sley | index("local_call_parser_prefix_runtime_execution"))'

bin/sley run --json examples/unused_private_task.sley \
  | json_field '.status == "passed" and .value.kind == "Int" and .value.value == 1'

bin/sley run --json examples/dead_private_tasks.sley \
  | json_field '.status == "passed" and .value.kind == "Int" and .value.value == 1'

bin/sley self-hosting-status --json \
  | json_field '(.bootstrap_owned_by_sley | index("zero_arg_project_call_runtime_execution"))'

bin/sley self-hosting-status --json \
  | json_field '(.bootstrap_owned_by_sley | index("zero_arg_project_call_parser_prefix_runtime_execution"))'

bin/sley run --json examples/unused_import_project \
  | json_field '.status == "passed" and .value.kind == "Int" and .value.value == 7'

bin/sley run --json examples/duplicate_import_project \
  | json_field '.status == "passed" and .value.kind == "Text" and .value.value == "ready"'

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
  | json_field '(.bootstrap_owned_by_sley | index("int_equal_runtime_task_execution"))'

bin/sley self-hosting-status --json \
  | json_field '(.bootstrap_owned_by_sley | index("int_greater_equal_runtime_task_execution"))'

bin/sley self-hosting-status --json \
  | json_field '(.bootstrap_owned_by_sley | index("constant_comparison_runtime_execution"))'

bin/sley self-hosting-status --json \
  | json_field '(.bootstrap_owned_by_sley | index("bool_if_runtime_task_execution"))'

bin/sley self-hosting-status --json \
  | json_field '(.bootstrap_owned_by_sley | index("bool_if_expression_runtime_execution"))'

bin/sley self-hosting-status --json \
  | json_field '(.bootstrap_owned_by_sley | index("bool_if_statement_runtime_execution"))'

bin/sley self-hosting-status --json \
  | json_field '(.bootstrap_owned_by_sley | index("text_if_runtime_task_execution"))'

bin/sley self-hosting-status --json \
  | json_field '(.bootstrap_owned_by_sley | index("compute_text_call_runtime_execution"))'

bin/sley self-hosting-status --json \
  | json_field '(.bootstrap_owned_by_sley | index("compute_text_call_parser_prefix_runtime_execution"))'

bin/sley run --json examples/compute.sley \
  | json_field '.status == "passed" and .value.kind == "Text" and .value.value == "excellent"'

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
  | json_field '(.bootstrap_owned_by_sley | index("while_list_sum_runtime_execution"))'

bin/sley self-hosting-status --json \
  | json_field '(.bootstrap_owned_by_sley | index("while_list_sum_parser_prefix_runtime_execution"))'

bin/sley run --json examples/collections.sley \
  | json_field '.status == "passed" and .value.kind == "Int" and .value.value == 10'

bin/sley self-hosting-status --json \
  | json_field '(.bootstrap_owned_by_sley | index("state_set_runtime_execution"))'

bin/sley self-hosting-status --json \
  | json_field '(.bootstrap_owned_by_sley | index("tally_set_runtime_execution"))'

bin/sley self-hosting-status --json \
  | json_field '(.bootstrap_owned_by_sley | index("each_sum_runtime_execution"))'

bin/sley run --json fixtures/corpus/accepted/mutable_sum.sley \
  | json_field '.status == "passed" and .value.kind == "Int" and .value.value == 10'

bin/sley self-hosting-status --json \
  | json_field '(.bootstrap_owned_by_sley | index("each_map_sum_runtime_execution"))'

bin/sley run --json examples/maps_for.sley \
  | json_field '.status == "passed" and .value.kind == "Int" and .value.value == 15'

bin/sley self-hosting-status --json \
  | json_field '(.bootstrap_owned_by_sley | index("project_call_runtime_execution"))'

bin/sley self-hosting-status --json \
  | json_field '(.bootstrap_owned_by_sley | index("project_call_parser_prefix_runtime_execution"))'

bin/sley self-hosting-status --json \
  | json_field '(.bootstrap_owned_by_sley | index("unqualified_project_call_runtime_execution"))'

bin/sley run --json examples/unqualified_import_call_project \
  | json_field '.status == "passed" and .value.kind == "Int" and .value.value == 42'

bin/sley self-hosting-status --json \
  | json_field '(.bootstrap_owned_by_sley | index("file_entry_project_call_runtime_execution"))'

bin/sley run --json examples/unqualified_import_call_project/src/app/main.sley \
  | json_field '.status == "passed" and .value.kind == "Int" and .value.value == 42'

bin/sley self-hosting-status --json \
  | json_field '(.bootstrap_owned_by_sley | index("file_entry_runtime_authority_execution"))'

bin/sley self-hosting-status --json \
  | json_field '(.bootstrap_owned_by_sley | index("bound_local_call_runtime_execution"))'

bin/sley self-hosting-status --json \
  | json_field '(.bootstrap_owned_by_sley | index("runtime_main_call_parser_prefix_task_execution"))'

bin/sley run --json fixtures/corpus/accepted/pure_main.sley \
  | json_field '.status == "passed" and .value.kind == "Int" and .value.value == 42'

bin/sley self-hosting-status --json \
  | json_field '(.bootstrap_owned_by_sley | index("int_identity_runtime_task_execution"))'

bin/sley self-hosting-status --json \
  | json_field '(.bootstrap_owned_by_sley | index("bound_int_return_runtime_execution"))'

bin/sley ast --json examples/hello.sley \
  | json_field '.schema == "sley.ast.program.v0" and .module == "app.hello" and .tasks[0].id == "task:app.hello.main" and .tasks[0].name == "main" and .tasks[0].body.statements[0].id == "block:task:app.hello.main:stmt:0" and .tasks[0].body.statements[0].expr.id == "block:task:app.hello.main:stmt:0:expr" and .tasks[0].body.statements[0].expr.expr_kind == "StringLiteral"'

bin/sley ast --json examples/declaration_hygiene.sley \
  | json_field '.types[0].id == "type:app.hygiene.Orphan" and .effects[0].id == "effect:app.hygiene.OrphanEffect"'

bin/sley ast --json examples/unused_take.sley \
  | json_field '.tasks[0].takes[0].id == "take:app.takes.main.value" and .tasks[0].takes[1].id == "take:app.takes.main.unused"'

bin/sley lint --json --rule unused_take examples/unused_take.sley \
  | json_field '.schema == "sley.lint.report.v0" and .findings[0].node == "take:task:app.takes.main:1:unused"'

bin/sley ast --json examples/constant_text_concatenation_expression.sley \
  | json_field '.schema == "sley.ast.program.v0" and .tasks[0].body.statements[0].expr.expr_kind == "Raw" and .tasks[0].body.statements[0].expr.source == "\"Sley \" + \"agents\""'

bin/sley ast --json examples/empty_for_statement.sley \
  | json_field '.schema == "sley.ast.program.v0" and .tasks[0].body.statements[1].collection.id == "block:task:app.empty_for.main:stmt:1:collection" and .tasks[0].body.statements[1].collection.expr_kind == "ListLiteral" and .tasks[0].body.statements[1].collection.items == []'

bin/sley ast --json examples/empty_while_statement.sley \
  | json_field '.tasks[0].body.statements[2].condition.id == "block:task:app.empty_while.main:stmt:2:condition"'

bin/sley ast --json --node block:task:app.hello.main:stmt:0:expr examples/hello.sley \
  | json_field '.schema == "sley.ast.node.v0" and .node_kind == "expression"'

bin/sley ast --json --node task:app.hello.main examples/hello.sley \
  | json_field '.schema == "sley.ast.node.v0" and .node_kind == "task" and (.parent | not)'

bin/sley ast --json --node block:task:app.hello.main:stmt:0 examples/hello.sley \
  | json_field '.schema == "sley.ast.node.v0" and .node_kind == "statement" and .parent == "task:app.hello.main"'

if bin/sley ast --json --node missing examples/hello.sley > "$ast_missing_report"; then
  fail "missing AST node unexpectedly succeeded"
fi
jq -er '.schema == "sley.diagnostics.report.v0" and .diagnostics[0].id == "AST_NODE_NOT_FOUND" and .diagnostics[0].message == "AST node not found `missing`"' "$ast_missing_report" >/dev/null

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
jq -er '.status == "error" and .diagnostics[0].id == "UNKNOWN_TASK" and .diagnostics[0].message == "unknown task `missing`" and .diagnostics[0].repair_hints[0].kind == "declare_or_import_task" and (.diagnostics[0].repair_hints[0].replacement | contains("take arg0: Int")) and (.diagnostics[0].repair_hints[0].replacement | contains("take arg1: Text"))' /tmp/sley-rejected-task-check.json >/dev/null

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
jq -er '.status == "error" and [.diagnostics[].id] == ["TYPE_MISMATCH", "RETURN_TYPE_MISMATCH"] and .diagnostics[0].message == "type mismatch `label`" and .diagnostics[1].message == "return type mismatch `corpus.rejected.main`" and [.diagnostics[1].repair_hints[].kind] == ["inspect_return_type", "replace_task_body", "replace_expression"]' /tmp/sley-rejected-type-mismatch-check.json >/dev/null

if bin/sley check --json fixtures/corpus/rejected/missing_return.sley >/tmp/sley-rejected-missing-return-check.json; then
  fail "rejected missing_return.sley passed check"
fi
jq -er '.status == "error" and .diagnostics[0].id == "MISSING_RETURN" and [.diagnostics[0].repair_hints[].kind] == ["insert_return", "replace_task_body"]' /tmp/sley-rejected-missing-return-check.json >/dev/null

bin/sley self-hosting-status --json \
  | json_field '(.bootstrap_owned_by_sley | index("checker_repair_hint_fallback_removal_task_execution"))'

if grep -Eq 'CHECKER_(DECLARE_OR_IMPORT_TASK|INSPECT_RETURN_TYPE|INSERT_RETURN|REPLACE_TASK_BODY|REPLACE_EXPRESSION)_HINT_KIND="\$\{CHECKER_[A-Z_]+:-' bin/sley; then
  fail "checker repair hint kinds must come from loom.checker without shell fallback literals"
fi

bin/sley self-hosting-status --json \
  | json_field '(.bootstrap_owned_by_sley | index("checker_builtin_type_fallback_removal_task_execution"))'

bin/sley check --json examples/result_flow.sley \
  | json_field '.status == "ok" and (.diagnostics | length) == 0'

if grep -Eq 'CHECKER_(INT|TEXT|BOOL|UNIT|RESULT|ERROR|GATE|LIST|MAP)_TYPE="\$\{CHECKER_[A-Z_]+:-' bin/sley; then
  fail "checker builtin type names must come from loom.checker without shell fallback literals"
fi

if grep -Fq 'eval_checker_builtin_types_json || printf' bin/sley; then
  fail "checker builtin type list must come from loom.checker without shell fallback JSON"
fi

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
  | json_field '.schema == "sley.query.report.v0" and .calls[0].target == "app.math.double" and .calls[0].callee == "math.double" and .calls[0].source == "call math.double(21)"'

bin/sley query --json --kind calls examples/project \
  | json_field 'keys == (["schema","kind","entry_module","filters","modules","tasks","types","effects","calls"] | sort)'

bin/sley graph --json examples/project \
  | json_field '.schema == "sley.symbol_graph.v0" and .entry_module == "app.main" and .modules[0].imports[0].id == "import:app.main:app.math"'

bin/sley graph --json examples/project \
  | json_field 'keys == (["schema","entry_module","modules"] | sort)'

bin/sley graph --json --slice task:app.main.main examples/project \
  | json_field '([.add_affordances[]? | select(.target_kind == "task") | .target] | index("module:app.main:tasks")) and ([.move_affordances[]? | select(.target_kind == "task") | .operation.payload.parent] | index("module:app.main:tasks"))'

bin/sley graph --json --slice task:app.main.main examples/project \
  | json_field '([.insert_affordances[]? | .target] | index("block:task:app.main.main")) and ([.insert_affordances[]? | .operation.target] | index("block:task:app.main.main")) and ([.move_affordances[]? | select(.target_kind == "statement") | .operation.payload.parent] | index("block:task:app.main.main"))'

bin/sley graph --json --slice task:app.main.main examples/project \
  | json_field '([.call_site_affordances[]? | .task_target] | index("task:app.math.double")) and ([.call_arg_affordances[]? | .task_target] | index("task:app.math.double")) and ([.call_arg_affordances[]? | .target] | index("block:task:app.main.main:stmt:0:expr:arg:0")) and ([.replace_affordances[]? | select(.target_kind == "Call") | .target] | index("block:task:app.main.main:stmt:0:expr"))'

bin/sley plan --json --graft-templates examples/project \
  | json_field '([.graft_templates[]? | select(.kind == "add_task") | .surface] | index("module:app.main:tasks")) and ([.graft_templates[]? | select(.kind == "move_task") | .operation.payload.parent] | index("module:app.main:tasks"))'

bin/sley plan --json --graft-templates examples/project \
  | json_field '([.task_surfaces[]?.graft_targets[]?] | index("block:task:app.main.main")) and ([.graft_templates[]? | select(.kind == "insert_statement") | .surface] | index("block:task:app.main.main")) and ([.graft_templates[]? | select(.kind == "move_statement") | .operation.payload.parent] | index("block:task:app.main.main")) and ([.graft_templates[]? | select(.kind == "move_statement") | .operation.target] | index("block:task:app.main.main:stmt:0")) and ([.graft_templates[]? | select(.kind == "delete_statement") | .operation.target] | index("block:task:app.main.main:stmt:0"))'

bin/sley plan --json --graft-templates --template-surface block:task:app.collections.sum examples/collections.sley \
  | json_field '([.graft_templates[]? | select(.kind == "insert_statement" and .surface == "block:task:app.collections.sum" and .operation.target == "block:task:app.collections.sum") | .operation.payload.position] | index(4))'

bin/sley plan --json --graft-templates examples/agent_deploy_pipeline.sley \
  | json_field '.graft_templates[0].surface == "task:app.agent_deploy_pipeline.main" and .graft_templates[0].operation.target == "task:app.agent_deploy_pipeline.main" and .next_actions[0].command[4] == "task:app.agent_deploy_pipeline.main"'

bin/sley plan --json --graft-templates --template-surface task:app.constant_false_if.main examples/constant_false_if_statement.sley \
  | json_field '([.graft_templates[]? | select(.kind == "move_statement_destination") | .operation.target] | index("block:task:app.constant_false_if.main:stmt:0:then:stmt:0")) and ([.graft_templates[]? | select(.kind == "replace_expression") | .surface] | index("block:task:app.constant_false_if.main:stmt:0:expr"))'

bin/sley plan --json --graft-templates examples/project \
  | json_field '([.graft_templates[]? | select(.kind == "replace_call_arg") | .operation.target] | index("task:app.math.double")) and ([.graft_templates[]? | select(.kind == "replace_call_arg") | .operation.payload.scope] | index("task:app.main.main"))'

bin/sley plan --json --emit-graft replace_call_arg examples/project \
  | json_field '.target == "task:app.math.double" and .payload.scope == "task:app.main.main"'

bin/sley graft --json --dry-run "$empty_module_source" "$add_take_operation" \
  | json_field '([.provenance[]?.targets[]?] | index("task:app.main.main"))'

bin/sley graph --json --slice module:app.math examples/project \
  | json_field '([.add_affordances[]? | .target] | index("module:app.math:imports")) and ([.add_affordances[]? | .target] | index("module:app.math:types")) and ([.add_affordances[]? | .target] | index("module:app.math:effects"))'

bin/sley plan --json --graft-templates --template-surface module:app.hygiene examples/declaration_hygiene.sley \
  | json_field '([.graft_templates[]? | .surface] | index("module:app.hygiene")) and ([.graft_templates[]? | select(.kind == "move_type") | .operation.payload.parent] | index("module:app.hygiene:types")) and ([.graft_templates[]? | select(.kind == "move_effect") | .operation.payload.parent] | index("module:app.hygiene:effects"))'

bin/sley doctor --json examples/project \
  | json_field '.schema == "sley.doctor.report.v0" and .status == "ready" and .summary.task_count == 2'

bin/sley doctor --json examples/project \
  | json_field 'keys == (["schema","status","target","entry_module","summary","diagnostics","query","lint","next_actions"] | sort)'

bin/sley lint --json examples/empty_for_statement.sley \
  | json_field '.schema == "sley.lint.report.v0" and .status == "findings" and .findings[0].id == "EMPTY_FOR_STATEMENT" and (.filters.rules | index("empty_for_statement"))'

bin/sley self-hosting-status --json \
  | json_field '(.bootstrap_owned_by_sley | index("lint_empty_finding_fallback_removal_task_execution"))'

bin/sley lint --json --rule empty_for_statement examples/empty_for_statement.sley \
  | json_field '([.findings[]? | select(.id == "EMPTY_FOR_STATEMENT" and .rule == "empty_for_statement" and .message == "task `app.empty_for.main` has a for statement over an empty list" and .hint == "delete this never-executed for statement") | .node] | index("block:task:app.empty_for.main:stmt:1"))'

bin/sley lint --json --rule empty_while_statement examples/empty_while_statement.sley \
  | json_field '([.findings[]? | select(.id == "EMPTY_WHILE_STATEMENT" and .rule == "empty_while_statement" and .message == "task `app.empty_while.main` has an empty while statement" and .hint == "delete this empty loop") | .node] | index("block:task:app.empty_while.main:stmt:2"))'

bin/sley lint --json --rule empty_forge_statement examples/empty_forge_statement.sley \
  | json_field '([.findings[]? | select(.id == "EMPTY_FORGE_STATEMENT" and .rule == "empty_forge_statement" and .message == "task `app.empty_forge.main` has an empty forge block" and .hint == "delete this empty isolated arena") | .node] | index("block:task:app.empty_forge.main:stmt:1"))'

bin/sley lint --json --rule empty_if_statement examples/empty_if_statement.sley \
  | json_field '([.findings[]? | select(.id == "EMPTY_IF_STATEMENT" and .rule == "empty_if_statement" and .message == "task `app.empty_if.main` has an empty if branch" and .hint == "delete the empty branch or add a meaningful statement") | .node] | index("block:task:app.empty_if.main:stmt:1"))'

bin/sley lint --json --rule empty_else_statement examples/empty_else_statement.sley \
  | json_field '([.findings[]? | select(.id == "EMPTY_ELSE_STATEMENT" and .rule == "empty_else_statement" and .message == "task `app.empty_else.main` has an empty else branch" and .hint == "remove this no-op else branch") | .node] | index("block:task:app.empty_else.main:stmt:1"))'

if grep -Eq 'LINT_(EMPTY_FOR|EMPTY_WHILE|EMPTY_FORGE|EMPTY_IF|EMPTY_ELSE|OK_STATUS|FINDINGS_STATUS|EMPTY_FOR_RULE|EMPTY_WHILE_RULE|EMPTY_FORGE_RULE|EMPTY_IF_RULE|EMPTY_ELSE_RULE|EMPTY_FOR_MESSAGE|EMPTY_WHILE_MESSAGE|EMPTY_FORGE_MESSAGE|EMPTY_IF_MESSAGE|EMPTY_ELSE_MESSAGE|EMPTY_FOR_HINT|EMPTY_WHILE_HINT|EMPTY_FORGE_HINT|EMPTY_IF_HINT|EMPTY_ELSE_HINT)="\$\{LINT_' bin/sley; then
  fail "lint empty finding values must come from loom.lint without shell fallback literals"
fi

bin/sley lint --json --rule unchecked_result examples/unchecked_result.sley \
  | json_field '([.findings[]? | select(.rule == "unchecked_result") | .node] | index("block:task:app.unchecked.main:stmt:0:expr"))'

bin/sley lint --json --rule unchecked_result examples/result_flow.sley \
  | json_field '.status == "ok" and (.findings | length) == 0'

bin/sley lint --json --rule unused_import examples/unused_import_project \
  | json_field '([.findings[]? | select(.rule == "unused_import") | .node] | index("import:app.main:app.stale"))'

bin/sley lint --json --rule unqualified_imported_call examples/unqualified_import_call_project \
  | json_field '([.findings[]? | select(.rule == "unqualified_imported_call") | .replacement] | index("call math.double(21)"))'

bin/sley lint --json --rule unreachable_statement examples/unreachable_statement.sley \
  | json_field '([.findings[]? | select(.rule == "unreachable_statement") | .node] | index("block:task:app.unreachable_statement.main:stmt:2"))'

bin/sley lint --json --rule unused_pure_expression_statement examples/unused_pure_expression_statement.sley \
  | json_field '([.findings[]? | select(.rule == "unused_pure_expression_statement") | .node] | index("block:task:app.unused_expr.main:stmt:1"))'

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

bin/sley self-hosting-status --json \
  | json_field '(.bootstrap_owned_by_sley | index("file_entry_agent_pipeline_runtime_execution"))'

if bin/sley run --json examples/agent_project/src/agent/main.sley > "$runtime_report"; then
  fail "file entry agent project should require seeded runtime authority"
fi
jq -er '.status == "error" and .diagnostics[0].id == "RUNTIME_CAPABILITY_REQUIRED" and (.diagnostics[0].message | contains("SecretRead")) and (.diagnostics[0].message | contains("Network")) and (.diagnostics[0].message | contains("ModelCall")) and (.diagnostics[0].message | contains("Deploy"))' "$runtime_report" >/dev/null

bin/sley run --json --cap SecretRead --secret api_key redacted --cap Network --http-text https://example.test/profile "owned profile" --cap ModelCall --model-output deploy-plan "owned plan" --cap Deploy --deploy-result staging "owned stage" examples/agent_project/src/agent/main.sley \
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
json_field '([.migrations[]? | select(.kind == "migrate_raw_host_adapter") | .surface] | index("block:task:main.main:stmt:0:expr"))' < "$migrate_report"
json_field 'keys == (["schema","status","target","source_schema","summary","migrations","schema_drift","diagnostics","issues"] | sort)' < "$migrate_report"

bin/sley-migrate report --json examples/unqualified_import_call_project \
  | json_field '([.migrations[]? | select(.kind == "qualify_imported_call") | .surface] | index("block:task:app.main.main:stmt:0:expr"))'

bin/sley-migrate report --json examples/unchecked_result_binding.sley \
  | json_field '([.migrations[]? | select(.kind == "propagate_unchecked_result_binding") | .surface] | index("block:task:app.unchecked_binding.main:stmt:0"))'

bin/sley-migrate report --json examples/unchecked_result.sley \
  | json_field '([.migrations[]? | select(.kind == "propagate_unchecked_result") | .surface] | index("block:task:app.unchecked.main:stmt:0:expr"))'

bin/sley fix --json --kind migrate_raw_host_adapter --dry-run examples/hello.sley \
  | json_field '([.provenance[]?.targets[]?] | index("block:task:app.raw_migration.main:stmt:0:expr"))'

bin/sley fix --json --kind qualify_imported_call --dry-run examples/hello.sley \
  | json_field '([.provenance[]?.targets[]?] | index("block:task:app.main.main:stmt:0:expr"))'

bin/sley fix --json --kind propagate_unchecked_result_binding --dry-run examples/hello.sley \
  | json_field '([.provenance[]?.targets[]?] | index("block:task:app.unchecked_binding.main:stmt:0"))'

bin/sley fix --json --kind propagate_unchecked_result --dry-run examples/hello.sley \
  | json_field '([.provenance[]?.targets[]?] | index("block:task:app.unchecked.main:stmt:0:expr"))'

bin/sley fix --json --kind convert_mutable_binding_to_bind --dry-run examples/hello.sley \
  | json_field '([.provenance[]?.targets[]?] | index("block:task:app.mutable_fix.main:stmt:0")) and ([.provenance[]?.targets[]?] | index("block:task:app.mutable_fix.main"))'

bin/sley fix --json --kind delete_self_assignment_statement --dry-run examples/hello.sley \
  | json_field '([.provenance[]?.targets[]?] | index("block:task:app.self_assignment_fix.main:stmt:1"))'

bin/sley fix --json --kind convert_redundant_initial_set_to_bind --dry-run examples/hello.sley \
  | json_field '([.provenance[]?.targets[]?] | index("block:task:app.redundant_initial_set_fix.main:stmt:0")) and ([.provenance[]?.targets[]?] | index("block:task:app.redundant_initial_set_fix.main:stmt:1"))'

bin/sley fix --json --kind fold_redundant_initial_set_into_binding --dry-run examples/hello.sley \
  | json_field '([.provenance[]?.targets[]?] | index("block:task:app.redundant_initial_set_fix.main:stmt:0:expr")) and ([.provenance[]?.targets[]?] | index("block:task:app.redundant_initial_set_fix.main:stmt:1"))'

bin/sley fix --json --kind delete_empty_for_statement --dry-run examples/hello.sley \
  | json_field '([.provenance[]?.targets[]?] | index("block:task:app.empty_for.main:stmt:1"))'

bin/sley fix --json --kind delete_empty_forge_statement --dry-run examples/hello.sley \
  | json_field '([.provenance[]?.targets[]?] | index("block:task:app.empty_forge.main:stmt:1"))'

bin/sley fix --json --kind delete_empty_if_statement --dry-run examples/hello.sley \
  | json_field '([.provenance[]?.targets[]?] | index("block:task:app.empty_if.main:stmt:1"))'

bin/sley fix --json --kind delete_empty_while_statement --dry-run examples/hello.sley \
  | json_field '([.provenance[]?.targets[]?] | index("block:task:app.empty_while.main:stmt:2"))'

bin/sley fix --json --kind remove_empty_else_statement --dry-run examples/hello.sley \
  | json_field '([.provenance[]?.targets[]?] | index("block:task:app.empty_else.main:stmt:1"))'

bin/sley fix --json --kind delete_unreachable_statement --dry-run examples/hello.sley \
  | json_field '([.provenance[]?.targets[]?] | index("block:task:app.unreachable_statement.main:stmt:2"))'

bin/sley fix --json --kind delete_unused_pure_binding --dry-run examples/hello.sley \
  | json_field '([.provenance[]?.targets[]?] | index("block:task:app.bindings.main:stmt:0"))'

bin/sley fix --json --kind delete_unused_pure_expression_statement --dry-run examples/hello.sley \
  | json_field '([.provenance[]?.targets[]?] | index("block:task:app.unused_expr.main:stmt:1"))'

bin/sley fix --json --kind drop_unused_effectful_binding_value --dry-run examples/hello.sley \
  | json_field '([.provenance[]?.targets[]?] | index("block:task:app.effectful_cleanup.main:stmt:0"))'

bin/sley fix --json --kind simplify_constant_if_expression --dry-run examples/hello.sley \
  | json_field '([.provenance[]?.targets[]?] | index("block:task:app.constant_fix.main:stmt:0:expr"))'

bin/sley fix --json --kind simplify_constant_if_statement --dry-run examples/hello.sley \
  | json_field '([.provenance[]?.targets[]?] | index("block:task:app.constant_if_statement_fix.main:stmt:1"))'

bin/sley fix --json --kind delete_constant_false_if_statement --dry-run examples/hello.sley \
  | json_field '([.provenance[]?.targets[]?] | index("block:task:app.constant_false_if_fix.main:stmt:0"))'

bin/sley fix --json --kind delete_constant_false_while_statement --dry-run examples/hello.sley \
  | json_field '([.provenance[]?.targets[]?] | index("block:task:app.false_while_fix.main:stmt:1"))'

bin/sley fix --json --kind simplify_constant_comparison_expression --dry-run examples/hello.sley \
  | json_field '([.provenance[]?.targets[]?] | index("block:task:app.constant_compare_fix.main:stmt:0:expr"))'

bin/sley fix --json --kind simplify_constant_boolean_comparison_expression --dry-run examples/hello.sley \
  | json_field '([.provenance[]?.targets[]?] | index("block:task:app.constant_bool_compare_fix.main:stmt:0:expr"))'

bin/sley fix --json --kind simplify_constant_arithmetic_expression --dry-run examples/hello.sley \
  | json_field '([.provenance[]?.targets[]?] | index("block:task:app.constant_arithmetic_fix.main:stmt:0:expr"))'

bin/sley fix --json --kind simplify_absorbing_arithmetic_expression --dry-run examples/hello.sley \
  | json_field '([.provenance[]?.targets[]?] | index("block:task:app.absorbing_arithmetic_fix.main:stmt:0:expr"))'

bin/sley fix --json --kind simplify_constant_text_concatenation_expression --dry-run examples/hello.sley \
  | json_field '([.provenance[]?.targets[]?] | index("block:task:app.constant_text_concat_fix.main:stmt:0:expr"))'

bin/sley fix --json --kind simplify_constant_list_index_expression --dry-run examples/hello.sley \
  | json_field '([.provenance[]?.targets[]?] | index("block:task:app.constant_list_index_fix.main:stmt:0:expr"))'

bin/sley fix --json --kind simplify_constant_map_index_expression --dry-run examples/hello.sley \
  | json_field '([.provenance[]?.targets[]?] | index("block:task:app.constant_map_index_fix.main:stmt:0:expr"))'

bin/sley fix --json --kind simplify_constant_record_field_access_expression --dry-run examples/hello.sley \
  | json_field '([.provenance[]?.targets[]?] | index("block:task:app.constant_record_field_access_fix.main:stmt:0:expr"))'

bin/sley fix --json --kind simplify_constant_len_expression --dry-run examples/hello.sley \
  | json_field '([.provenance[]?.targets[]?] | index("block:task:app.constant_len_fix.main:stmt:0:expr"))'

bin/sley fix --json --kind simplify_constant_not_expression --dry-run examples/hello.sley \
  | json_field '([.provenance[]?.targets[]?] | index("block:task:app.constant_not_fix.main:stmt:0:expr"))'

bin/sley fix --json --kind simplify_identity_binary_expression --dry-run examples/hello.sley \
  | json_field '([.provenance[]?.targets[]?] | index("block:task:app.identity_fix.main:stmt:1:expr"))'

bin/sley fix --json --kind simplify_redundant_boolean_comparison --dry-run examples/hello.sley \
  | json_field '([.provenance[]?.targets[]?] | index("block:task:app.boolean_fix.main:stmt:1:expr"))'

bin/sley fix --json --kind simplify_absorbing_boolean_expression --dry-run examples/hello.sley \
  | json_field '([.provenance[]?.targets[]?] | index("block:task:app.absorbing_boolean_fix.main:stmt:0:expr"))'

bin/sley fix --json --kind simplify_idempotent_boolean_expression --dry-run examples/hello.sley \
  | json_field '([.provenance[]?.targets[]?] | index("block:task:app.idempotent_boolean_fix.main:stmt:1:expr"))'

bin/sley fix --json --kind simplify_self_comparison_expression --dry-run examples/hello.sley \
  | json_field '([.provenance[]?.targets[]?] | index("block:task:app.self_compare_fix.main:stmt:0:expr"))'

bin/sley fix --json --kind simplify_double_negation_expression --dry-run examples/hello.sley \
  | json_field '([.provenance[]?.targets[]?] | index("block:task:app.double_fix.main:stmt:1:expr"))'

bin/sley fix --json --kind simplify_negated_comparison_expression --dry-run examples/hello.sley \
  | json_field '([.provenance[]?.targets[]?] | index("block:task:app.negated_compare_fix.main:stmt:1:expr"))'

bin/sley fix --json --kind simplify_redundant_boolean_if_expression --dry-run examples/hello.sley \
  | json_field '([.provenance[]?.targets[]?] | index("block:task:app.boolean_if_fix.main:stmt:1:expr"))'

bin/sley fix --json --kind simplify_redundant_boolean_if_statement --dry-run examples/hello.sley \
  | json_field '([.provenance[]?.targets[]?] | index("block:task:app.boolean_if_statement_fix.main:stmt:1"))'

bin/sley fix --json --kind simplify_same_branch_if_expression --dry-run examples/hello.sley \
  | json_field '([.provenance[]?.targets[]?] | index("block:task:app.same_branch_fix.main:stmt:2:expr:left"))'

bin/sley fix --json --kind simplify_same_branch_if_statement --dry-run examples/hello.sley \
  | json_field '([.provenance[]?.targets[]?] | index("block:task:app.same_branch_statement_fix.main:stmt:1"))'

bin/sley-contract validate --schema sley.migrate.report.v0 "$migrate_report" --schemas docs/schemas --json \
  | json_field '.schema == "sley.contract.validate.v0" and .status == "passed" and .requested_schema == "sley.migrate.report.v0" and .report_schema == "sley.migrate.report.v0"'

bin/sley-docgen reference --json --module agent.pipeline examples/agent_project > "$docgen_report"
json_field '.schema == "sley.docgen.report.v0" and .status == "generated" and .source_schema == "sley.query.report.v0" and .filters.module == "agent.pipeline" and .filters.exported_only == false and .summary.module_count == 1 and .summary.task_count == 3 and .summary.capability_count == 10 and .documents[0].title == "Sley Reference: agent.pipeline" and .tasks[0].qualified_name == "agent.pipeline.collect_profile" and .tasks[0].inbound_call_count == 1 and .tasks[1].inbound_call_count == 1 and .tasks[2].inbound_call_count == 1' < "$docgen_report"
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
json_field '.schema == "sley.workbench.report.v0" and .status == "ready" and .summary.module_count == 1 and .summary.call_count == 7 and (.query.calls | length) == 4 and .query.calls[0].source == "call secrets.try_get(\"api_key\")?" and .graph_slice.schema == "sley.symbol_graph.slice.v0" and .graph_slice.focus.id == "task:app.agent_deploy_pipeline.main" and (.graph_slice.outbound_calls | length) == 4' < "$workbench_report"

bin/sley-contract validate --schema sley.workbench.report.v0 "$workbench_report" --schemas docs/schemas --json \
  | json_field '.schema == "sley.contract.validate.v0" and .status == "passed" and .requested_schema == "sley.workbench.report.v0" and .report_schema == "sley.workbench.report.v0"'

bin/sley-workbench --json examples/unused_private_task.sley > "$workbench_report"
json_field '.schema == "sley.workbench.report.v0" and .status == "warnings" and .lint.status == "findings" and .lint.findings[0].id == "UNUSED_PRIVATE_TASK" and .lint.findings[0].plan_command[1] == "plan" and .lint.findings[0].plan_command[5] == "task:app.tasks.orphan" and .lint.findings[0].plan_command[6] == "examples/unused_private_task.sley"' < "$workbench_report"

bin/sley-contract validate --schema sley.workbench.report.v0 "$workbench_report" --schemas docs/schemas --json \
  | json_field '.schema == "sley.contract.validate.v0" and .status == "passed" and .requested_schema == "sley.workbench.report.v0" and .report_schema == "sley.workbench.report.v0"'

if bin/sley-ci verify --json examples/agent_deploy_pipeline.sley > "$ci_report"; then
  fail "ci verify agent deploy pipeline passed without runtime gates"
fi
json_field '.schema == "sley.ci.report.v0" and .status == "failed" and .steps[0].diagnostics[0].node == "task:app.agent_deploy_pipeline.main"' < "$ci_report"

bin/sley-contract validate --schema sley.ci.report.v0 "$ci_report" --schemas docs/schemas --json \
  | json_field '.schema == "sley.contract.validate.v0" and .status == "passed" and .requested_schema == "sley.ci.report.v0" and .report_schema == "sley.ci.report.v0"'

bin/sley-sandbox-runner run --json fixtures/contracts/sandbox_manifest_agent_pipeline.json > "$sandbox_report"
json_field '.schema == "sley.sandbox.report.v0" and .status == "warnings" and .manifest_schema == "sley.sandbox.manifest.v0" and .target == "examples/agent_deploy_pipeline.sley" and .summary.capability_count == 4 and .summary.seed_count == 4 and .summary.issue_count == 1 and .verify.schema == "sley.verify.report.v0" and .verify.status == "passed" and .issues[0].code == "SANDBOX_OS_ISOLATION_NOT_ENFORCED"' < "$sandbox_report"

bin/sley-contract validate --schema sley.sandbox.report.v0 "$sandbox_report" --schemas docs/schemas --json \
  | json_field '.schema == "sley.contract.validate.v0" and .status == "passed" and .requested_schema == "sley.sandbox.report.v0" and .report_schema == "sley.sandbox.report.v0"'

bin/sley-agent-bench run --json --case unused-private-task-repair > "$agent_bench_report"
json_field '.schema == "sley.agent_bench.report.v0" and .status == "passed" and .case_count == 1 and .passed_count == 1 and .failed_count == 0 and .cases[0].name == "unused-private-task-repair" and .cases[0].status == "passed" and .cases[0].trace_receipt_count == 1 and .cases[0].selected_repair.surface == "task:app.bench.orphan"' < "$agent_bench_report"

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
