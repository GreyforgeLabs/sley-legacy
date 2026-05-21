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

self_hosting_status_field() {
  jq -er "$1" "$self_hosting_status_report" >/dev/null
}

bool_literal_source="$(mktemp)"
generic_top_level_multiply_source="$(mktemp)"
generic_top_level_comparison_source="$(mktemp)"
generic_balanced_parentheses_source="$(mktemp)"
generic_parenthesized_bool_and_source="$(mktemp)"
generic_bool_or_source="$(mktemp)"
parser_feature_source="$(mktemp)"
parser_statement_source="$(mktemp)"
parser_binding_source="$(mktemp)"
parser_for_payload_source="$(mktemp)"
parser_condition_source="$(mktemp)"
parser_block_body_source="$(mktemp)"
generic_source_main_source="$(mktemp)"
generic_state_set_source="$(mktemp)"
generic_if_assignment_source="$(mktemp)"
generic_result_err_source="$(mktemp)"
generic_each_source="$(mktemp)"
generic_for_source="$(mktemp)"
generic_while_source="$(mktemp)"
qualified_task_call_source="$(mktemp)"
empty_module_source="$(mktemp)"
unknown_take_type_source="$(mktemp)"
add_take_operation="$(mktemp)"
self_hosting_status_report="$(mktemp)"
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
trap 'rm -f "$bool_literal_source" "$generic_top_level_multiply_source" "$generic_top_level_comparison_source" "$generic_balanced_parentheses_source" "$generic_parenthesized_bool_and_source" "$generic_bool_or_source" "$parser_feature_source" "$parser_statement_source" "$parser_binding_source" "$parser_for_payload_source" "$parser_condition_source" "$parser_block_body_source" "$generic_source_main_source" "$generic_state_set_source" "$generic_if_assignment_source" "$generic_result_err_source" "$generic_each_source" "$generic_for_source" "$generic_while_source" "$qualified_task_call_source" "$empty_module_source" "$unknown_take_type_source" "$add_take_operation" "$self_hosting_status_report" "$artifact_check_report" "$migrate_report" "$docgen_report" "$workbench_report" "$sandbox_report" "$agent_bench_report" "$zjx_tool_report" "$runtime_report" "$ast_missing_report" "$ci_report"; rm -rf "$artifact_dir"' EXIT
printf '%s\n' \
  'module app.bool_literal' \
  '' \
  'task main -> Bool {' \
  '  return true' \
  '}' > "$bool_literal_source"
printf '%s\n' \
  'module app.generic_top_level_multiply' \
  '' \
  'task main -> Int {' \
  '  return 1 * (2 + 3)' \
  '}' > "$generic_top_level_multiply_source"
printf '%s\n' \
  'module app.generic_top_level_comparison' \
  '' \
  'task main -> Bool {' \
  '  return 1 + 1 == 2' \
  '}' > "$generic_top_level_comparison_source"
printf '%s\n' \
  'module app.generic_balanced_parentheses' \
  '' \
  'task main -> Bool {' \
  '  return (1 + 1) == (1 + 1)' \
  '}' > "$generic_balanced_parentheses_source"
printf '%s\n' \
  'module app.generic_parenthesized_bool_and' \
  '' \
  'task main -> Bool {' \
  '  bind ready = true' \
  '  return (ready == true) && ready' \
  '}' > "$generic_parenthesized_bool_and_source"
printf '%s\n' \
  'module app.generic_bool_or' \
  '' \
  'task main -> Bool {' \
  '  return false || true' \
  '}' > "$generic_bool_or_source"
printf '%s\n' \
  'module app.parser_features' \
  'import app.parser_helpers as helpers' \
  '' \
  'export type Profile = {' \
  '  slot name: Text' \
  '}' \
  '' \
  'effect Audit {' \
  '}' \
  '' \
  'task main -> Text {' \
  '  bind count = 42' \
  '  bind text = "hello"' \
  '  bind flag = true' \
  '  bind names = []' \
  '  call noop(count)' \
  '  bind first = names[0]' \
  '  bind title = profile.name' \
  '' \
  '  return text' \
  '}' > "$parser_feature_source"
printf '%s\n' \
  'module app.parser_statements' \
  '' \
  'task main -> Int {' \
  '  state count = 1' \
  '  set count = 2' \
  '  call noop()' \
  '  for item in [] {' \
  '  }' \
  '  while false {' \
  '  }' \
  '  if true {' \
  '  }' \
  '  forge {' \
  '  }' \
  '  return count' \
  '}' > "$parser_statement_source"
printf '%s\n' \
  'module app.parser_bindings' \
  '' \
  'task main -> Int uses FileRead {' \
  '  take gate files: Gate<FileRead>' \
  '  take seed: Int' \
  '' \
  '  bind base = seed' \
  '  state total = base' \
  '  tally seen = total' \
  '' \
  '  return seen' \
  '}' > "$parser_binding_source"
printf '%s\n' \
  'module app.parser_for_payload' \
  '' \
  'task main -> Int {' \
  '  bind values = [1, 2]' \
  '  for score in values {' \
  '  }' \
  '  return 0' \
  '}' > "$parser_for_payload_source"
printf '%s\n' \
  'module app.parser_conditions' \
  '' \
  'task main -> Int {' \
  '  while index < 10 {' \
  '  }' \
  '  if ready {' \
  '  }' \
  '  return 0' \
  '}' > "$parser_condition_source"
printf '%s\n' \
  'module app.parser_block_bodies' \
  '' \
  'task main -> Int {' \
  '  state total = 0' \
  '  for value in values {' \
  '    set total = value' \
  '  }' \
  '  while ready {' \
  '    set total = total + 1' \
  '  }' \
  '  if ready {' \
  '    return total' \
  '  } else {' \
  '    return 0' \
  '  }' \
  '}' > "$parser_block_body_source"
printf '%s\n' \
  'module app.generic_source_main' \
  '' \
  'task greet -> Text {' \
  '  take name: Text' \
  '' \
  '  return "hi " + name' \
  '}' \
  '' \
  'task main -> Text {' \
  '  return if true { call greet("Ada") } else { "bad" }' \
  '}' > "$generic_source_main_source"
printf '%s\n' \
  'module app.generic_state_set' \
  '' \
  'task main -> Text {' \
  '  state label = "hi"' \
  '  set label = label + " Ada"' \
  '' \
  '  return label' \
  '}' > "$generic_state_set_source"
printf '%s\n' \
  'module app.generic_if_assignment' \
  '' \
  'task main -> Text {' \
  '  state label = ""' \
  '  if false {' \
  '    set label = "bad"' \
  '  } else {' \
  '    set label = "ready"' \
  '  }' \
  '' \
  '  return label' \
  '}' > "$generic_if_assignment_source"
printf '%s\n' \
  'module app.generic_result_err' \
  '' \
  'task main -> Result<Text, Error> {' \
  '  if true {' \
  '    return Err("boom")' \
  '  }' \
  '' \
  '  return Ok("ready")' \
  '}' > "$generic_result_err_source"
printf '%s\n' \
  'module app.generic_each' \
  '' \
  'task main -> Text {' \
  '  bind names = ["Ada", "Lovelace"]' \
  '  state label = ""' \
  '  each name in names {' \
  '    set label = label + name' \
  '  }' \
  '' \
  '  return label' \
  '}' > "$generic_each_source"
printf '%s\n' \
  'module app.generic_for' \
  '' \
  'task main -> Text {' \
  '  state label = ""' \
  '  for name in ["Ada", "Lovelace"] {' \
  '    set label = label + name' \
  '  }' \
  '' \
  '  return label' \
  '}' > "$generic_for_source"
printf '%s\n' \
  'module app.generic_while' \
  '' \
  'task join -> Text {' \
  '  take names: List<Text>' \
  '' \
  '  state index = 0' \
  '  state label = ""' \
  '  while index < len(names) {' \
  '    set label = label + names[index]' \
  '    set index = index + 1' \
  '  }' \
  '' \
  '  return label' \
  '}' \
  '' \
  'task main -> Text {' \
  '  bind names = ["Ada", "Lovelace"]' \
  '  return call join(names)' \
  '}' > "$generic_while_source"
printf '%s\n' \
  'module app.qualified_policy' \
  '' \
  'task main -> Unit {' \
  '  call external.missing()' \
  '}' > "$qualified_task_call_source"
printf '%s\n' \
  'module app.empty' > "$empty_module_source"
printf '%s\n' \
  'module app.unknown_take_type' \
  '' \
  'task main -> Int {' \
  '  take value: MissingTakeType' \
  '' \
  '  return 1' \
  '}' > "$unknown_take_type_source"
printf '%s\n' \
  '{"op":"AddTake","target":"","payload":{"name":"value","type":"Text"}}' > "$add_take_operation"

bash -n bin/sley
./scripts/check-self-hosted-code.sh

bin/sley --help >/dev/null
bin/sley --version | grep -q 'self-hosting-stage2-source'
bin/sley self-hosting-status --json > "$self_hosting_status_report"

self_hosting_status_field '.schema == "sley.self_hosting.status.v0" and .status == "bootstrap" and .strict_self_hosted == false and .source_root == "self-hosted/src" and .semantic_source_count == (.source_modules | length) and .semantic_source_count >= 6 and (.source_modules | index("loom.bootstrap")) and (.bootstrap_owned_by_sley | index("implementation_version")) and (.bootstrap_owned_by_sley | index("implementation_stage")) and (.bootstrap_owned_by_sley | index("self_hosting_status")) and (.bootstrap_owned_by_sley | index("strict_self_hosted")) and (.bootstrap_owned_by_sley | index("source_modules")) and (.bootstrap_owned_by_sley | index("semantic_source_count_task_execution")) and (.bootstrap_owned_by_sley | index("default_lint_rules")) and (.bootstrap_owned_by_sley | index("core_report_schema_ids")) and (.bootstrap_owned_by_sley | index("diagnostic_ids")) and (.bootstrap_owned_by_sley | index("runtime_seed_values")) and (.bootstrap_owned_by_sley | index("parser_expression_classifiers")) and (.bootstrap_owned_by_sley | index("parser_classifier_task_execution")) and (.bootstrap_owned_by_sley | index("parser_id_task_execution")) and (.bootstrap_owned_by_sley | index("parser_call_expression_prefix_task_execution")) and (.bootstrap_owned_by_sley | index("query_call_expression_prefix_task_execution")) and (.bootstrap_owned_by_sley | index("symbol_graph_call_expression_prefix_task_execution")) and (.bootstrap_owned_by_sley | index("parser_statement_and_binding_kinds")) and (.bootstrap_owned_by_sley | index("bootstrap_smoke_runtime")) and (.bootstrap_owned_by_sley | index("bootstrap_smoke_task_execution")) and (.bootstrap_owned_by_sley | index("checker_diagnostic_status")) and (.bootstrap_owned_by_sley | index("checker_status_task_execution")) and (.bootstrap_owned_by_sley | index("checker_builtin_type_task_execution")) and (.bootstrap_owned_by_sley | index("checker_builtin_types_task_execution")) and (.bootstrap_owned_by_sley | index("checker_static_type_names_task_execution")) and (.bootstrap_owned_by_sley | index("checker_effect_aliases_task_execution")) and (.bootstrap_owned_by_sley | index("checker_host_effect_needles_task_execution")) and (.bootstrap_owned_by_sley | index("checker_gate_binding_kind_task_execution")) and (.bootstrap_owned_by_sley | index("checker_repair_hint_kind_task_execution")) and (.bootstrap_owned_by_sley | index("checker_identifier_expr_kind_task_execution")) and (.bootstrap_owned_by_sley | index("checker_call_expression_prefix_task_execution")) and (.bootstrap_owned_by_sley | index("checker_message_task_execution")) and (.bootstrap_owned_by_sley | index("checker_unknown_type_diagnostics")) and (.bootstrap_owned_by_sley | index("checker_unknown_task_diagnostics")) and (.bootstrap_owned_by_sley | index("checker_call_diagnostics")) and (.bootstrap_owned_by_sley | index("checker_duplicate_take_diagnostics")) and (.bootstrap_owned_by_sley | index("checker_collection_diagnostics")) and (.bootstrap_owned_by_sley | index("checker_record_diagnostics")) and (.bootstrap_owned_by_sley | index("checker_namespace_diagnostics")) and (.bootstrap_owned_by_sley | index("checker_effect_diagnostics")) and (.bootstrap_owned_by_sley | index("checker_effect_propagation_diagnostics")) and (.bootstrap_owned_by_sley | index("checker_question_diagnostics")) and (.bootstrap_owned_by_sley | index("checker_identifier_resolution_inputs")) and (.bootstrap_owned_by_sley | index("lint_status_task_execution")) and (.bootstrap_owned_by_sley | index("lint_finding_messages")) and (.bootstrap_owned_by_sley | index("runtime_status_task_execution")) and (.bootstrap_owned_by_sley | index("pure_literal_runtime_execution")) and (.bootstrap_owned_by_sley | index("bool_literal_runtime_execution")) and (.bootstrap_owned_by_sley | index("text_concat_runtime_execution")) and (.bootstrap_owned_by_sley | index("list_len_runtime_task_execution")) and (.bootstrap_owned_by_sley | index("len_expression_runtime_execution")) and (.bootstrap_owned_by_sley | index("if_expression_runtime_execution")) and (.bootstrap_owned_by_sley | index("if_statement_runtime_execution")) and (.bootstrap_owned_by_sley | index("seeded_agent_deploy_task_execution")) and (.bootstrap_owned_by_sley | index("project_ready_task_execution")) and (.bootstrap_owned_by_sley | index("ast_runtime_probe_dispatch")) and (.bootstrap_owned_by_sley | index("self_hosting_report_shape")) and (.bootstrap_owned_by_sley | index("runtime_report_status_and_dispatch"))'

self_hosting_status_field '(.bootstrap_owned_by_sley | index("bootstrap_source_metadata_task_execution")) and (.bootstrap_owned_by_sley | index("lint_source_metadata_task_execution"))'

if grep -Eq 'extract_sley_[A-Za-z0-9_]+ "\$SELF_HOSTED_SOURCE_ROOT/loom/[A-Za-z_]+\.sley"' bin/sley; then
  fail "self-hosted module metadata must execute through Sley source tasks, not raw host extractors"
fi

self_hosting_status_field '(.bootstrap_owned_by_sley | index("runtime_host_effect_needles_dispatch_execution")) and (.bootstrap_owned_by_sley | index("runtime_authority_host_effect_needles_execution"))'

self_hosting_status_field '(.bootstrap_owned_by_sley | index("runtime_authority_effect_aliases_task_execution"))'

self_hosting_status_field '(.bootstrap_owned_by_sley | index("runtime_diagnostic_messages_task_execution"))'

self_hosting_status_field '(.bootstrap_owned_by_sley | index("runtime_diagnostic_fallback_removal_task_execution"))'

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

self_hosting_status_field '(.bootstrap_owned_by_sley | index("runtime_default_database_table_task_execution"))'

self_hosting_status_field '(.bootstrap_owned_by_sley | index("runtime_value_kinds_task_execution"))'
self_hosting_status_field '(.bootstrap_owned_by_sley | index("err_text_runtime_task_execution"))'

if grep -Eq 'RUNTIME_(INT|TEXT|BOOL|RAW|UNIT|OK_TEXT|OK_INT|ERR_TEXT)_VALUE_KIND="\$\{RUNTIME_[A-Z_]+:-' bin/sley; then
  fail "runtime value-kind names must come from loom.runtime without shell fallback literals"
fi

self_hosting_status_field '(.bootstrap_owned_by_sley | index("runtime_self_hosted_target_aliases_task_execution"))'

bin/sley run --json self-hosted/ \
  | json_field '.status == "passed" and .value.kind == "Int" and .value.value >= 1'

bin/sley run --json self-hosted/sley.toml \
  | json_field '.status == "passed" and .value.kind == "Int" and .value.value >= 1'

if grep -Fq "RUNTIME_SELF_HOSTED_TARGETS_JSON='[\"self-hosted\"" bin/sley; then
  fail "self-hosted runtime target aliases must come from loom.runtime without shell fallback literals"
fi

self_hosting_status_field '(.bootstrap_owned_by_sley | index("runtime_source_metadata_task_execution"))'

if grep -Fq 'extract_sley_string_task "$SELF_HOSTED_SOURCE_ROOT/loom/runtime.sley"' bin/sley \
  || grep -Fq 'extract_sley_list_task_json "$SELF_HOSTED_SOURCE_ROOT/loom/runtime.sley"' bin/sley; then
  fail "runtime metadata must execute through Sley source tasks, not raw host extractors"
fi

if ! grep -Fq 'sley_source_task loom.runtime' bin/sley \
  || ! grep -Fq 'sley_source_list_task_json loom.runtime' bin/sley; then
  fail "runtime metadata source task dispatch is missing"
fi

self_hosting_status_field '(.bootstrap_owned_by_sley | index("runtime_status_names_task_execution"))'

bin/sley run --json examples/hello.sley \
  | json_field '.status == "passed" and .value.kind == "Text" and .value.value == "hello sley"'

if grep -Eq 'RUNTIME_(PASSED|FAILED|SKIPPED)_STATUS="\$\{RUNTIME_(PASSED|FAILED|SKIPPED)_STATUS:-' bin/sley; then
  fail "runtime status names must come from loom.runtime without shell fallback literals"
fi

self_hosting_status_field '(.bootstrap_owned_by_sley | index("runtime_target_probe_sources_task_execution"))'

bin/sley run --json examples/project \
  | json_field '.status == "passed" and .value.kind == "Int" and .value.value == 42'

bin/sley run --json --cap SecretRead --cap Network --cap ModelCall --cap Deploy examples/agent_deploy_pipeline.sley \
  | json_field '.status == "passed" and .value.kind == "Ok" and .value.value.kind == "Text" and .value.value.value == "profile ready | plan approved | staged"'

if grep -Fq 'RUNTIME_AGENT_DEPLOY_SUFFIX="${RUNTIME_AGENT_DEPLOY_SUFFIX:-' bin/sley \
  || grep -Fq 'RUNTIME_PROJECT_READY_CALL_PROBE="${RUNTIME_PROJECT_READY_CALL_PROBE:-' bin/sley \
  || grep -Fq 'RUNTIME_PROJECT_READY_BINDING_PROBE="${RUNTIME_PROJECT_READY_BINDING_PROBE:-' bin/sley; then
  fail "runtime target probes must come from loom.runtime without shell fallback literals"
fi

self_hosting_status_field '(.bootstrap_owned_by_sley | index("runtime_default_value_sources_task_execution"))'

bin/sley run --json examples/hello.sley \
  | json_field '.status == "passed" and .value.kind == "Text" and .value.value == "hello sley"'

bin/sley run --json examples/raw_host_migration.sley \
  | json_field '.status == "passed" and .value.kind == "Ok" and .value.value.kind == "Text" and .value.value.value == "hello sley"'

bin/sley run --json --cap Shell --shell-output date "owned date" examples/shell_gate.sley \
  | json_field '.status == "passed" and .value.kind == "Ok" and .value.value.kind == "Text" and .value.value.value == "owned date"'

if grep -Eq 'RUNTIME_(HELLO_VALUE|PROJECT_READY_VALUE|DEFAULT_PROFILE|DEFAULT_MODEL_PLAN|DEFAULT_DEPLOY_RESULT|DEFAULT_RAW_VALUE|DEFAULT_FILE_WRITE_TEXT|DEFAULT_DATABASE_WRITE_TEXT|DEFAULT_AGENT_DATA_WRITE_TEXT|DEFAULT_DATABASE_READ_TEXT|DEFAULT_DATABASE_TABLE|DEFAULT_SHELL_TEXT|DEFAULT_SECRET_TEXT|DEFAULT_SPEND_AUTHORIZATION_TEXT)="\$\{RUNTIME_' bin/sley; then
  fail "runtime default values must come from loom.runtime without shell fallback literals"
fi

self_hosting_status_field '(.bootstrap_owned_by_sley | index("runtime_run_fallback_removal_task_execution"))'

if grep -Eq 'eval_(seeded_agent_deploy_value|runtime_[A-Za-z0-9_]+_task|project_ready_value_task)[^\n]*\|\| printf' bin/sley \
  || grep -Eq 'eval_runtime_status_task 0 \|\| printf' bin/sley; then
  fail "runtime command values must execute through loom.runtime without host printf fallbacks"
fi

self_hosting_status_field '(.bootstrap_owned_by_sley | index("lint_declared_effect_aliases_task_execution"))'

self_hosting_status_field '(.bootstrap_owned_by_sley | index("checker_call_parser_prefix_task_execution"))'

self_hosting_status_field '(.bootstrap_owned_by_sley | index("checker_message_parser_prefix_task_execution"))'

self_hosting_status_field '(.bootstrap_owned_by_sley | index("checker_status_parser_prefix_task_execution"))'

self_hosting_status_field '(.bootstrap_owned_by_sley | index("parser_classifier_parser_prefix_task_execution"))'

self_hosting_status_field '(.bootstrap_owned_by_sley | index("parser_classifier_source_task_execution"))'

self_hosting_status_field '(.bootstrap_owned_by_sley | index("parser_feature_classifier_task_execution"))'

self_hosting_status_field '(.bootstrap_owned_by_sley | index("parser_ast_expression_feature_dispatch_execution"))'

if ! self_hosting_status_field '(.bootstrap_owned_by_sley | index("parser_expression_dispatch_plan_task_execution")) and (.bootstrap_owned_by_sley | index("parser_ast_expression_dispatch_plan_execution"))'; then
  fail "parser expression dispatch plan markers are missing"
fi

if ! self_hosting_status_field '(.bootstrap_owned_by_sley | index("parser_expression_source_pattern_plan_task_execution")) and (.bootstrap_owned_by_sley | index("parser_ast_expression_source_pattern_execution"))'; then
  fail "parser expression source pattern markers are missing"
fi

if ! self_hosting_status_field '(.bootstrap_owned_by_sley | index("parser_operator_feature_classifier_task_execution")) and (.bootstrap_owned_by_sley | index("parser_ast_binary_expression_dispatch_execution")) and (.bootstrap_owned_by_sley | index("parser_binary_operator_name_task_execution")) and (.bootstrap_owned_by_sley | index("parser_unary_operator_name_task_execution")) and (.bootstrap_owned_by_sley | index("parser_unary_expression_dispatch_execution")) and (.bootstrap_owned_by_sley | index("parser_ast_schema_expression_dispatch_execution"))'; then
  fail "parser binary-expression source classifier markers are missing"
fi

if ! self_hosting_status_field '(.bootstrap_owned_by_sley | index("parser_expression_surface_feature_classifier_task_execution")) and (.bootstrap_owned_by_sley | index("parser_ast_call_expression_dispatch_execution")) and (.bootstrap_owned_by_sley | index("parser_bare_call_expression_dispatch_execution")) and (.bootstrap_owned_by_sley | index("parser_ast_member_index_expression_dispatch_execution")) and (.bootstrap_owned_by_sley | index("parser_record_literal_expression_dispatch_execution")) and (.bootstrap_owned_by_sley | index("parser_try_expression_dispatch_execution")) and (.bootstrap_owned_by_sley | index("parser_if_expression_dispatch_execution")) and (.bootstrap_owned_by_sley | index("parser_list_literal_expression_dispatch_execution")) and (.bootstrap_owned_by_sley | index("parser_map_literal_expression_dispatch_execution")) and (.bootstrap_owned_by_sley | index("parser_compound_index_expression_dispatch_execution"))'; then
  fail "parser call/member/index source classifier markers are missing"
fi

if ! self_hosting_status_field '(.bootstrap_owned_by_sley | index("parser_declaration_feature_classifier_task_execution")) and (.bootstrap_owned_by_sley | index("parser_ast_declaration_feature_dispatch_execution"))'; then
  fail "parser declaration source classifier markers are missing"
fi

if ! self_hosting_status_field '(.bootstrap_owned_by_sley | index("parser_declaration_dispatch_plan_task_execution")) and (.bootstrap_owned_by_sley | index("parser_ast_declaration_dispatch_plan_execution"))'; then
  fail "parser declaration dispatch plan markers are missing"
fi

if ! self_hosting_status_field '(.bootstrap_owned_by_sley | index("parser_declaration_source_pattern_plan_task_execution")) and (.bootstrap_owned_by_sley | index("parser_ast_declaration_source_pattern_execution"))'; then
  fail "parser declaration source pattern markers are missing"
fi

self_hosting_status_field '(.bootstrap_owned_by_sley | index("parser_statement_feature_classifier_task_execution"))'

self_hosting_status_field '(.bootstrap_owned_by_sley | index("parser_ast_statement_feature_dispatch_execution"))'

if ! self_hosting_status_field '(.bootstrap_owned_by_sley | index("parser_statement_dispatch_plan_task_execution")) and (.bootstrap_owned_by_sley | index("parser_ast_statement_dispatch_plan_execution"))'; then
  fail "parser statement dispatch plan markers are missing"
fi

if ! self_hosting_status_field '(.bootstrap_owned_by_sley | index("parser_statement_kind_plan_task_execution")) and (.bootstrap_owned_by_sley | index("parser_ast_statement_kind_plan_execution"))'; then
  fail "parser statement kind plan markers are missing"
fi

if ! self_hosting_status_field '(.bootstrap_owned_by_sley | index("parser_statement_source_dispatch_plan_task_execution")) and (.bootstrap_owned_by_sley | index("parser_ast_statement_source_dispatch_plan_execution"))'; then
  fail "parser statement source dispatch plan markers are missing"
fi

if ! self_hosting_status_field '(.bootstrap_owned_by_sley | index("parser_statement_source_pattern_plan_task_execution")) and (.bootstrap_owned_by_sley | index("parser_ast_statement_source_pattern_execution"))'; then
  fail "parser statement source pattern markers are missing"
fi

if ! self_hosting_status_field '(.bootstrap_owned_by_sley | index("parser_take_source_dispatch_plan_task_execution")) and (.bootstrap_owned_by_sley | index("parser_ast_take_source_dispatch_plan_execution"))'; then
  fail "parser take source dispatch plan markers are missing"
fi

if ! self_hosting_status_field '(.bootstrap_owned_by_sley | index("parser_take_source_pattern_plan_task_execution")) and (.bootstrap_owned_by_sley | index("parser_ast_take_source_pattern_execution"))'; then
  fail "parser take source pattern markers are missing"
fi

self_hosting_status_field '(.bootstrap_owned_by_sley | index("parser_binding_feature_classifier_task_execution"))'

self_hosting_status_field '(.bootstrap_owned_by_sley | index("parser_ast_binding_feature_dispatch_execution"))'

self_hosting_status_field '(.bootstrap_owned_by_sley | index("parser_for_statement_payload_task_execution"))'

self_hosting_status_field '(.bootstrap_owned_by_sley | index("parser_ast_for_payload_execution"))'

self_hosting_status_field '(.bootstrap_owned_by_sley | index("parser_condition_payload_task_execution"))'

self_hosting_status_field '(.bootstrap_owned_by_sley | index("parser_ast_condition_payload_execution"))'

self_hosting_status_field '(.bootstrap_owned_by_sley | index("parser_nested_body_depth_task_execution"))'

self_hosting_status_field '(.bootstrap_owned_by_sley | index("parser_ast_nested_body_payload_execution"))'

if ! awk '
  /^eval_parser_expression_dispatch_plan_text\(\)[ \t]*\{/ {in_fn=1}
  in_fn && /sley_eval_source_task_list_json "\$parser_file" expression_dispatch_plan/ {source_eval=1}
  in_fn && /^[ \t]*}/ {exit}
  END {exit source_eval ? 0 : 1}
' bin/sley; then
  fail "eval_parser_expression_dispatch_plan_text must execute expression_dispatch_plan through the shared Sley source list evaluator"
fi

if ! grep -Fq 'PARSER_EXPRESSION_DISPATCH_PLAN_TEXT="$(eval_parser_expression_dispatch_plan_text)"' bin/sley; then
  fail "parser expression dispatch plan source task dispatch is missing"
fi

if ! awk '
  /^eval_parser_declaration_dispatch_plan_text\(\)[ \t]*\{/ {in_fn=1}
  in_fn && /sley_eval_source_task_list_json "\$parser_file" declaration_dispatch_plan/ {source_eval=1}
  in_fn && /^[ \t]*}/ {exit}
  END {exit source_eval ? 0 : 1}
' bin/sley; then
  fail "eval_parser_declaration_dispatch_plan_text must execute declaration_dispatch_plan through the shared Sley source list evaluator"
fi

if ! grep -Fq 'PARSER_DECLARATION_DISPATCH_PLAN_TEXT="$(eval_parser_declaration_dispatch_plan_text)"' bin/sley; then
  fail "parser declaration dispatch plan source task dispatch is missing"
fi

if ! awk '
  /^eval_parser_statement_dispatch_plan_text\(\)[ \t]*\{/ {in_fn=1}
  in_fn && /sley_eval_source_task_list_json "\$parser_file" statement_dispatch_plan/ {source_eval=1}
  in_fn && /^[ \t]*}/ {exit}
  END {exit source_eval ? 0 : 1}
' bin/sley; then
  fail "eval_parser_statement_dispatch_plan_text must execute statement_dispatch_plan through the shared Sley source list evaluator"
fi

if ! grep -Fq 'PARSER_STATEMENT_DISPATCH_PLAN_TEXT="$(eval_parser_statement_dispatch_plan_text)"' bin/sley; then
  fail "parser statement dispatch plan source task dispatch is missing"
fi

if ! awk '
  /^eval_parser_statement_source_dispatch_plan_text\(\)[ \t]*\{/ {in_fn=1}
  in_fn && /sley_eval_source_task_list_json "\$parser_file" statement_source_dispatch_plan/ {source_eval=1}
  in_fn && /^[ \t]*}/ {exit}
  END {exit source_eval ? 0 : 1}
' bin/sley; then
  fail "eval_parser_statement_source_dispatch_plan_text must execute statement_source_dispatch_plan through the shared Sley source list evaluator"
fi

if ! grep -Fq 'PARSER_STATEMENT_SOURCE_DISPATCH_PLAN_TEXT="$(eval_parser_statement_source_dispatch_plan_text)"' bin/sley; then
  fail "parser statement source dispatch plan source task dispatch is missing"
fi

if ! awk '
  /^eval_parser_take_source_dispatch_plan_text\(\)[ \t]*\{/ {in_fn=1}
  in_fn && /sley_eval_source_task_list_json "\$parser_file" take_source_dispatch_plan/ {source_eval=1}
  in_fn && /^[ \t]*}/ {exit}
  END {exit source_eval ? 0 : 1}
' bin/sley; then
  fail "eval_parser_take_source_dispatch_plan_text must execute take_source_dispatch_plan through the shared Sley source list evaluator"
fi

if ! grep -Fq 'PARSER_TAKE_SOURCE_DISPATCH_PLAN_TEXT="$(eval_parser_take_source_dispatch_plan_text)"' bin/sley; then
  fail "parser take source dispatch plan source task dispatch is missing"
fi

if ! awk '
  /function expr_json\(expr, id, line, col/ {in_fn=1}
  in_fn && /expression_dispatch_count/ {plan=1}
  in_fn && /expr_candidate_json\(expression_dispatch_id\[dispatch_index\]/ {candidate=1}
  in_fn && /^[ \t]*}/ {exit}
  END {exit (plan && candidate) ? 0 : 1}
' bin/sley; then
  fail "expr_json must iterate Sley-owned parser expression dispatch candidates"
fi

if ! awk '
  /raw_expression_dispatch_count=split\(expression_dispatch_plan/ {in_parse=1}
  in_parse && /expression_dispatch_strategy\[expression_dispatch_count\]=expression_dispatch_parts\[3\]/ {strategy=1}
  in_parse && /expression_dispatch_match_value\[expression_dispatch_count\]=expression_dispatch_parts\[4\]/ {match_value=1}
  in_parse && /raw_declaration_dispatch_count=split/ {exit}
  END {exit (strategy && match_value) ? 0 : 1}
' bin/sley; then
  fail "parser expression dispatch must parse Sley-owned matcher strategy and patterns"
fi

if ! awk '
  /function expr_candidate_matches\(candidate_id, expr, strategy, match_value/ {in_fn=1}
  in_fn && /expr ~ match_value/ {pattern=1}
  in_fn && /expr == match_value/ {literal=1}
  in_fn && /strategy=="binary"/ {binary=1}
  in_fn && /strategy=="always"/ {always=1}
  in_fn && /^[ \t]*}/ {exit}
  END {exit (pattern && literal && binary && always) ? 0 : 1}
' bin/sley; then
  fail "parser expression matching must evaluate Sley-owned matcher patterns"
fi

if ! awk '
  /function expr_candidate_json\(candidate_id, expr, id, line, col, strategy, match_value/ {in_fn=1}
  in_fn && /expr_candidate_matches\(candidate_id, expr, strategy, match_value\)/ {matcher=1}
  in_fn && /^[ \t]*}/ {exit}
  END {exit matcher ? 0 : 1}
' bin/sley; then
  fail "parser expression candidate rendering must be guarded by Sley-owned matcher data"
fi

if grep -Fq 'if(expr ~ int_pattern){return' bin/sley; then
  fail "parser expression dispatch must not fall back to a host hard-coded expression ladder"
fi

if ! awk '
  /function declaration_candidate_id\(line, dispatch_index/ {in_fn=1}
  in_fn && /declaration_dispatch_count/ {plan=1}
  in_fn && /declaration_candidate_matches\(declaration_dispatch_id\[dispatch_index\]/ {candidate=1}
  in_fn && /^[ \t]*}/ {exit}
  END {exit (plan && candidate) ? 0 : 1}
' bin/sley; then
  fail "parser declarations must iterate Sley-owned declaration dispatch candidates"
fi

if ! awk '
  /raw_declaration_dispatch_count=split\(declaration_dispatch_plan/ {in_parse=1}
  in_parse && /declaration_dispatch_pattern\[declaration_dispatch_count\]=declaration_dispatch_parts\[3\]/ {pattern=1}
  in_parse && /raw_statement_dispatch_count=split/ {exit}
  END {exit pattern ? 0 : 1}
' bin/sley; then
  fail "parser declaration dispatch must parse Sley-owned declaration source patterns"
fi

if ! awk '
  /function declaration_candidate_matches\(candidate_id, line, pattern/ {in_fn=1}
  in_fn && /line ~ pattern/ {pattern=1}
  in_fn && /^[ \t]*}/ {exit}
  END {exit pattern ? 0 : 1}
' bin/sley; then
  fail "parser declaration matching must evaluate Sley-owned declaration source patterns"
fi

if awk '
  /function declaration_candidate_matches\(candidate_id, line/ {in_fn=1}
  in_fn && /candidate_id==[0-9]/ {hard_ladder=1}
  in_fn && /^[ \t]*}/ {exit}
  END {exit hard_ladder ? 0 : 1}
' bin/sley; then
  fail "parser declaration dispatch must not keep a host hard-coded declaration matcher ladder"
fi

if awk '
  /FNR==1 \{module="main"; file_seen\[FILENAME\]=1\}/ {in_ast_decl=1}
  in_ast_decl && /^[ \t]*\/\^\[ \\t\]\*module\[ \\t\]\+\// {hard_ladder=1}
  in_ast_decl && /^[ \t]*in_task[ \t]*\{/ {exit}
  END {exit hard_ladder ? 0 : 1}
' bin/sley; then
  fail "parser declaration dispatch must not fall back to a host hard-coded declaration pattern ladder"
fi

if ! awk '
  /function print_statement\(i, j, id, parts/ {in_fn=1}
  in_fn && /statement_dispatch_count/ {plan=1}
  in_fn && /statement_candidate_print\(statement_dispatch_id\[dispatch_index\]/ {candidate=1}
  in_fn && /^[ \t]*}/ {exit}
  END {exit (plan && candidate) ? 0 : 1}
' bin/sley; then
  fail "print_statement must iterate Sley-owned parser statement dispatch candidates"
fi

if ! awk '
  /raw_statement_dispatch_count=split\(statement_dispatch_plan/ {in_parse=1}
  in_parse && /statement_dispatch_match_kind\[statement_dispatch_count\]=statement_dispatch_parts\[3\]/ {match_kind=1}
  in_parse && /raw_statement_source_dispatch_count=split/ {exit}
  END {exit match_kind ? 0 : 1}
' bin/sley; then
  fail "parser statement dispatch must parse Sley-owned statement kind matches"
fi

if ! awk '
  /function statement_candidate_print\(candidate_id, i, j, id, parts, expected_kind/ {in_fn=1}
  in_fn && /parts\[1\] != expected_kind/ {expected=1}
  in_fn && /^[ \t]*}/ {exit}
  END {exit expected ? 0 : 1}
' bin/sley; then
  fail "parser statement rendering must be guarded by Sley-owned statement kind matches"
fi

if awk '
  /function statement_candidate_print\(candidate_id, i, j, id, parts/ {in_fn=1}
  in_fn && /&& parts\[1\]==/ {hard_ladder=1}
  in_fn && /^[ \t]*}/ {exit}
  END {exit hard_ladder ? 0 : 1}
' bin/sley; then
  fail "parser statement dispatch must not keep host hard-coded statement kind match guards"
fi

if grep -Fq 'else if(parts[1]==expr_statement_kind)' bin/sley; then
  fail "parser statement dispatch must not fall back to a host hard-coded statement ladder"
fi

if ! awk '
  /function statement_source_candidate_id\(line, depth, dispatch_index/ {in_fn=1}
  in_fn && /statement_source_dispatch_count/ {plan=1}
  in_fn && /statement_source_candidate_matches\(statement_source_dispatch_id\[dispatch_index\]/ {candidate=1}
  in_fn && /^[ \t]*}/ {exit}
  END {exit (plan && candidate) ? 0 : 1}
' bin/sley; then
  fail "parser statement source classification must iterate Sley-owned source dispatch candidates"
fi

if ! awk '
  /raw_statement_source_dispatch_count=split\(statement_source_dispatch_plan/ {in_parse=1}
  in_parse && /statement_source_dispatch_strategy\[statement_source_dispatch_count\]=statement_source_dispatch_parts\[3\]/ {strategy=1}
  in_parse && /statement_source_dispatch_match_value\[statement_source_dispatch_count\]=statement_source_dispatch_parts\[4\]/ {match_value=1}
  in_parse && /statement_source_dispatch_secondary_value\[statement_source_dispatch_count\]=statement_source_dispatch_parts\[5\]/ {secondary=1}
  in_parse && /raw_take_source_dispatch_count=split/ {exit}
  END {exit (strategy && match_value && secondary) ? 0 : 1}
' bin/sley; then
  fail "parser statement source dispatch must parse Sley-owned matcher strategy and patterns"
fi

if ! awk '
  /function statement_source_candidate_matches\(candidate_id, line, depth, strategy, match_value, secondary_value/ {in_fn=1}
  in_fn && /line ~ match_value/ {pattern=1}
  in_fn && /index\(line, match_value\) == 1/ {prefix=1}
  in_fn && /line ~ secondary_value/ {depth_pattern=1}
  in_fn && /^[ \t]*}/ {exit}
  END {exit (pattern && prefix && depth_pattern) ? 0 : 1}
' bin/sley; then
  fail "parser statement source matching must evaluate Sley-owned matcher patterns"
fi

if awk '
  /function statement_source_candidate_matches\(candidate_id, line, depth/ {in_fn=1}
  in_fn && /candidate_id==[0-9]/ {hard_ladder=1}
  in_fn && /call_expression_prefix/ {hard_ladder=1}
  in_fn && /^[ \t]*}/ {exit}
  END {exit hard_ladder ? 0 : 1}
' bin/sley; then
  fail "parser statement source dispatch must not keep a host hard-coded statement matcher ladder"
fi

if awk '
  /^[ \t]*in_task[ \t]*\{/ {in_task=1}
  in_task && /else if\(line ~ \/\^\(bind\|state\|tally\)/ {hard_ladder=1}
  in_task && /else if\(index\(line, call_expression_prefix\)/ {hard_ladder=1}
  in_task && /^[ \t]*depth \+=/ {exit}
  END {exit hard_ladder ? 0 : 1}
' bin/sley; then
  fail "parser statement source classification must not fall back to a host hard-coded line ladder"
fi

if ! awk '
  /function take_source_candidate_id\(source, dispatch_index/ {in_fn=1}
  in_fn && /take_source_dispatch_count/ {plan=1}
  in_fn && /take_source_candidate_matches\(take_source_dispatch_id\[dispatch_index\]/ {candidate=1}
  in_fn && /^[ \t]*}/ {exit}
  END {exit (plan && candidate) ? 0 : 1}
' bin/sley; then
  fail "parser take source classification must iterate Sley-owned take source dispatch candidates"
fi

if ! awk '
  /raw_take_source_dispatch_count=split\(take_source_dispatch_plan/ {in_parse=1}
  in_parse && /take_source_dispatch_strategy\[take_source_dispatch_count\]=take_source_dispatch_parts\[3\]/ {strategy=1}
  in_parse && /take_source_dispatch_match_value\[take_source_dispatch_count\]=take_source_dispatch_parts\[4\]/ {match_value=1}
  in_parse && /^[ \t]*}/ {exit}
  END {exit (strategy && match_value) ? 0 : 1}
' bin/sley; then
  fail "parser take source dispatch must parse Sley-owned matcher strategy and patterns"
fi

if ! awk '
  /function take_source_candidate_matches\(candidate_id, source, strategy, match_value/ {in_fn=1}
  in_fn && /source ~ match_value/ {pattern=1}
  in_fn && /source !~ match_value/ {not_pattern=1}
  in_fn && /^[ \t]*}/ {exit}
  END {exit (pattern && not_pattern) ? 0 : 1}
' bin/sley; then
  fail "parser take source matching must evaluate Sley-owned matcher patterns"
fi

if awk '
  /function take_source_candidate_matches\(candidate_id, source/ {in_fn=1}
  in_fn && /candidate_id==[0-9]/ {hard_ladder=1}
  in_fn && /^[ \t]*}/ {exit}
  END {exit hard_ladder ? 0 : 1}
' bin/sley; then
  fail "parser take source dispatch must not keep a host hard-coded take matcher ladder"
fi

if awk '
  /^[ \t]*in_task[ \t]*\{/ {in_task=1}
  in_task && /if\(t ~ \/\^gate/ {hard_ladder=1}
  in_task && /^[ \t]*name=t/ {exit}
  END {exit hard_ladder ? 0 : 1}
' bin/sley; then
  fail "parser take source classification must not fall back to a host hard-coded gate-take ladder"
fi

if ! awk '
  /^eval_parser_expression_classifiers_json\(\)[ \t]*\{/ {in_fn=1}
  in_fn && /sley_eval_source_task/ {source_eval=1}
  in_fn && /extract_sley_task_body/ {body_walk=1}
  in_fn && /^[ \t]*}/ {exit}
  END {exit (source_eval && !body_walk) ? 0 : 1}
' bin/sley; then
  fail "eval_parser_expression_classifiers_json must execute classify_expression through the shared Sley source task evaluator"
fi

if ! awk '
  /^eval_parser_expression_feature_classifiers_json\(\)[ \t]*\{/ {in_fn=1}
  in_fn && /sley_eval_source_task/ {source_eval=1}
  in_fn && /extract_sley_task_body/ {body_walk=1}
  in_fn && /^[ \t]*}/ {exit}
  END {exit (source_eval && !body_walk) ? 0 : 1}
' bin/sley; then
  fail "eval_parser_expression_feature_classifiers_json must execute classify_expression_features through the shared Sley source task evaluator"
fi

if ! awk '
  /^eval_parser_operator_feature_classifiers_json\(\)[ \t]*\{/ {in_fn=1}
  in_fn && /sley_eval_source_task/ {source_eval=1}
  in_fn && /extract_sley_task_body/ {body_walk=1}
  in_fn && /^[ \t]*}/ {exit}
  END {exit (source_eval && !body_walk) ? 0 : 1}
' bin/sley; then
  fail "eval_parser_operator_feature_classifiers_json must execute classify_expression_operator_features through the shared Sley source task evaluator"
fi

if ! awk '
  /^eval_parser_binary_operator_names_json\(\)[ \t]*\{/ {in_fn=1}
  in_fn && /sley_eval_source_task/ {source_eval=1}
  in_fn && /extract_sley_task_body/ {body_walk=1}
  in_fn && /^[ \t]*}/ {exit}
  END {exit (source_eval && !body_walk) ? 0 : 1}
' bin/sley; then
  fail "eval_parser_binary_operator_names_json must execute binary_operator_name through the shared Sley source task evaluator"
fi

if ! awk '
  /^eval_parser_expression_surface_classifiers_json\(\)[ \t]*\{/ {in_fn=1}
  in_fn && /sley_eval_source_task/ {source_eval=1}
  in_fn && /extract_sley_task_body/ {body_walk=1}
  in_fn && /^[ \t]*}/ {exit}
  END {exit (source_eval && !body_walk) ? 0 : 1}
' bin/sley; then
  fail "eval_parser_expression_surface_classifiers_json must execute classify_expression_surface_features through the shared Sley source task evaluator"
fi

if ! awk '
  /^eval_parser_declaration_feature_classifiers_json\(\)[ \t]*\{/ {in_fn=1}
  in_fn && /sley_eval_source_task/ {source_eval=1}
  in_fn && /extract_sley_task_body/ {body_walk=1}
  in_fn && /^[ \t]*}/ {exit}
  END {exit (source_eval && !body_walk) ? 0 : 1}
' bin/sley; then
  fail "eval_parser_declaration_feature_classifiers_json must execute classify_declaration_features through the shared Sley source task evaluator"
fi

if ! awk '
  /^eval_parser_statement_feature_classifiers_json\(\)[ \t]*\{/ {in_fn=1}
  in_fn && /sley_eval_source_task/ {source_eval=1}
  in_fn && /extract_sley_task_body/ {body_walk=1}
  in_fn && /^[ \t]*}/ {exit}
  END {exit (source_eval && !body_walk) ? 0 : 1}
' bin/sley; then
  fail "eval_parser_statement_feature_classifiers_json must execute classify_statement_features through the shared Sley source task evaluator"
fi

if ! awk '
  /^eval_parser_binding_feature_classifiers_json\(\)[ \t]*\{/ {in_fn=1}
  in_fn && /sley_eval_source_task/ {source_eval=1}
  in_fn && /extract_sley_task_body/ {body_walk=1}
  in_fn && /^[ \t]*}/ {exit}
  END {exit (source_eval && !body_walk) ? 0 : 1}
' bin/sley; then
  fail "eval_parser_binding_feature_classifiers_json must execute classify_binding_features through the shared Sley source task evaluator"
fi

bin/sley ast --json "$parser_feature_source" \
  | json_field '[.tasks[].body.statements[]?.expr.expr_kind] as $k | ($k | index("IntLiteral")) and ($k | index("StringLiteral")) and ($k | index("BoolLiteral")) and ($k | index("ListLiteral")) and ($k | index("Identifier")) and ($k | index("Call")) and ($k | index("Index")) and ($k | index("FieldAccess"))'

bin/sley ast --json "$parser_feature_source" \
  | json_field '(.tasks[0].body.statements[] | select(.expr.expr_kind == "Call" and .expr.callee.name == "noop" and .expr.args[0].name == "count")) and (.tasks[0].body.statements[] | select(.expr.expr_kind == "Index" and .expr.collection.name == "names" and .expr.index.value == 0)) and (.tasks[0].body.statements[] | select(.expr.expr_kind == "FieldAccess" and .expr.receiver.name == "profile" and .expr.field == "name"))'

bin/sley ast --json "$parser_feature_source" \
  | json_field '.module == "app.parser_features" and (.imports[] | select(.module == "app.parser_helpers" and .alias == "helpers")) and (.types[] | select(.name == "Profile" and .exported == true)) and (.effects[] | select(.name == "Audit")) and (.tasks[] | select(.name == "main"))'

bin/sley ast --json examples/constant_arithmetic_expression.sley \
  | json_field '.tasks[0].body.statements[0].expr.expr_kind == "Binary" and .tasks[0].body.statements[0].expr.op == "Add" and .tasks[0].body.statements[0].expr.left.value == 2 and .tasks[0].body.statements[0].expr.right.value == 3'

bin/sley ast --json examples/idempotent_boolean_expression.sley \
  | json_field '.tasks[0].body.statements[1].expr.expr_kind == "Binary" and .tasks[0].body.statements[1].expr.op == "And"'

bin/sley ast --json "$parser_statement_source" \
  | json_field '[.tasks[].body.statements[]?.kind] as $k | ($k | index("Binding")) and ($k | index("Set")) and ($k | index("Expr")) and ($k | index("For")) and ($k | index("While")) and ($k | index("If")) and ($k | index("Forge")) and ($k | index("Return"))'

bin/sley ast --json "$parser_binding_source" \
  | json_field '([.tasks[0].takes[]?.binding_kind] as $takes | ($takes | index("Gate")) and ($takes | index("Take"))) and ([.tasks[0].body.statements[]? | select(.kind == "Binding") | .binding_kind] as $bindings | ($bindings | index("Bind")) and ($bindings | index("State")) and ($bindings | index("Tally")))'

bin/sley ast --json "$parser_for_payload_source" \
  | json_field '.tasks[0].body.statements[] | select(.kind == "For" and .item == "score" and .collection.source == "values" and .collection.expr_kind == "Identifier" and .collection.name == "values")'

bin/sley ast --json "$parser_condition_source" \
  | json_field '(.tasks[0].body.statements[] | select(.kind == "While" and .condition.source == "index < 10")) and (.tasks[0].body.statements[] | select(.kind == "If" and .condition.source == "ready" and .condition.expr_kind == "Identifier"))'

bin/sley ast --json "$parser_block_body_source" \
  | json_field '(.tasks[0].body.statements[] | select(.kind == "For" and (.body.statements[] | select(.kind == "Set" and .name == "total" and .expr.source == "value")))) and (.tasks[0].body.statements[] | select(.kind == "While" and (.body.statements[] | select(.kind == "Set" and .name == "total" and .expr.source == "total + 1")))) and (.tasks[0].body.statements[] | select(.kind == "If" and (.then_block.statements[] | select(.kind == "Return" and .expr.source == "total")) and (.else_block.statements[] | select(.kind == "Return" and .expr.source == "0"))))'

self_hosting_status_field '(.bootstrap_owned_by_sley | index("parser_source_metadata_task_execution"))'

self_hosting_status_field '(.bootstrap_owned_by_sley | index("parser_fallback_removal_task_execution"))'

self_hosting_status_field '(.bootstrap_owned_by_sley | index("parser_call_prefix_fallback_removal_task_execution"))'

if grep -Fq 'extract_sley_string_task "$SELF_HOSTED_SOURCE_ROOT/loom/parser.sley"' bin/sley \
  || grep -Fq 'extract_sley_list_task_json "$SELF_HOSTED_SOURCE_ROOT/loom/parser.sley"' bin/sley; then
  fail "parser metadata must execute through Sley source tasks, not raw host extractors"
fi

if ! grep -Fq 'sley_source_task loom.parser' bin/sley; then
  fail "parser metadata source task dispatch is missing"
fi

if grep -Eq 'eval_parser_(id|message)_template [A-Za-z0-9_]+( [A-Za-z0-9_]+)? \|\| printf' bin/sley \
  || grep -Eq 'PARSER_[A-Z0-9_]+="\$\{PARSER_[A-Z0-9_]+:-' bin/sley; then
  fail "parser metadata and templates must not use host fallback literals"
fi

if grep -Fq '${PARSER_CALL_EXPRESSION_PREFIX:-call }' bin/sley; then
  fail "parser call-expression prefix must come from loom.parser without host fallback literals"
fi

self_hosting_status_field '(.bootstrap_owned_by_sley | index("semantic_source_count_parser_prefix_task_execution")) and (.bootstrap_owned_by_sley | index("bootstrap_smoke_parser_prefix_task_execution"))'

self_hosting_status_field '(.bootstrap_owned_by_sley | index("self_hosting_report_builder")) and (.bootstrap_owned_by_sley | index("self_hosting_report_builder_task_execution")) and ((.blockers | index("extend Sley-owned report builders across remaining command reports")) == null) and ((.blockers | index("replace shell JSON shaping with Sley-owned report builders")) == null)'

self_hosting_status_field '(.bootstrap_owned_by_sley | index("parser_declaration_id_task_execution")) and (.bootstrap_owned_by_sley | index("parser_take_id_task_execution"))'

self_hosting_status_field '(.bootstrap_owned_by_sley | index("parser_take_surface_id_task_execution"))'

self_hosting_status_field '(.bootstrap_owned_by_sley | index("parser_module_surface_id_task_execution"))'

self_hosting_status_field '(.bootstrap_owned_by_sley | index("parser_module_task_list_surface_id_task_execution"))'

self_hosting_status_field '(.bootstrap_owned_by_sley | index("parser_module_declaration_list_surface_id_task_execution"))'

self_hosting_status_field '(.bootstrap_owned_by_sley | index("parser_block_task_surface_id_task_execution"))'

self_hosting_status_field '(.bootstrap_owned_by_sley | index("parser_block_fallback_surface_id_task_execution"))'

self_hosting_status_field '(.bootstrap_owned_by_sley | index("parser_call_target_task_id_task_execution"))'

self_hosting_status_field '(.bootstrap_owned_by_sley | index("parser_call_argument_surface_id_task_execution"))'

self_hosting_status_field '(.bootstrap_owned_by_sley | index("parser_expression_side_surface_id_task_execution"))'

self_hosting_status_field '(.bootstrap_owned_by_sley | index("parser_task_statement_surface_id_task_execution"))'

self_hosting_status_field '(.bootstrap_owned_by_sley | index("parser_task_fallback_surface_id_task_execution"))'

self_hosting_status_field '(.bootstrap_owned_by_sley | index("parser_branch_statement_surface_id_task_execution"))'

self_hosting_status_field '(.bootstrap_owned_by_sley | index("parser_lint_statement_surface_id_task_execution"))'

self_hosting_status_field '(.bootstrap_owned_by_sley | index("parser_migrate_surface_id_task_execution"))'

self_hosting_status_field '(.bootstrap_owned_by_sley | index("parser_fix_migration_surface_id_task_execution"))'

self_hosting_status_field '(.bootstrap_owned_by_sley | index("migrate_qualify_call_parser_prefix_task_execution")) and (.bootstrap_owned_by_sley | index("fix_qualify_call_parser_prefix_task_execution"))'

self_hosting_status_field '(.bootstrap_owned_by_sley | index("symbol_graph_call_arg_parser_prefix_task_execution")) and (.bootstrap_owned_by_sley | index("plan_call_arg_parser_prefix_task_execution"))'

self_hosting_status_field '(.bootstrap_owned_by_sley | index("fix_update_call_sites_parser_prefix_task_execution")) and (.bootstrap_owned_by_sley | index("fix_remove_call_arg_parser_prefix_task_execution"))'

self_hosting_status_field '(.bootstrap_owned_by_sley | index("parser_fix_style_surface_id_task_execution"))'

self_hosting_status_field '(.bootstrap_owned_by_sley | index("parser_fix_empty_surface_id_task_execution"))'

self_hosting_status_field '(.bootstrap_owned_by_sley | index("parser_fix_unused_unreachable_surface_id_task_execution"))'

self_hosting_status_field '(.bootstrap_owned_by_sley | index("parser_fix_constant_control_surface_id_task_execution"))'

self_hosting_status_field '(.bootstrap_owned_by_sley | index("parser_fix_constant_scalar_surface_id_task_execution"))'

self_hosting_status_field '(.bootstrap_owned_by_sley | index("parser_fix_constant_derived_surface_id_task_execution"))'

self_hosting_status_field '(.bootstrap_owned_by_sley | index("parser_fix_algebra_boolean_surface_id_task_execution"))'

self_hosting_status_field '(.bootstrap_owned_by_sley | index("parser_fix_boolean_branch_surface_id_task_execution"))'

self_hosting_status_field '(.bootstrap_owned_by_sley | index("parser_expression_id_task_execution")) and (.bootstrap_owned_by_sley | index("parser_control_expression_id_task_execution"))'

self_hosting_status_field '(.bootstrap_owned_by_sley | index("diagnostics_report_builder")) and (.bootstrap_owned_by_sley | index("diagnostics_report_builder_task_execution")) and (.bootstrap_owned_by_sley | index("run_report_builder")) and (.bootstrap_owned_by_sley | index("run_report_builder_task_execution"))'

self_hosting_status_field '(.bootstrap_owned_by_sley | index("query_report_builder")) and (.bootstrap_owned_by_sley | index("query_report_builder_task_execution")) and (.bootstrap_owned_by_sley | index("lint_report_builder")) and (.bootstrap_owned_by_sley | index("lint_report_builder_task_execution")) and (.bootstrap_owned_by_sley | index("doctor_report_builder")) and (.bootstrap_owned_by_sley | index("doctor_report_builder_task_execution")) and (.bootstrap_owned_by_sley | index("verify_report_builder")) and (.bootstrap_owned_by_sley | index("verify_report_builder_task_execution"))'

self_hosting_status_field '(.bootstrap_owned_by_sley | index("parser_ast_program_report_builder")) and (.bootstrap_owned_by_sley | index("parser_ast_program_report_builder_task_execution"))'

self_hosting_status_field '(.bootstrap_owned_by_sley | index("parser_expression_statement_parser_prefix_task_execution"))'

self_hosting_status_field '(.bootstrap_owned_by_sley | index("parser_ast_node_report_builder")) and (.bootstrap_owned_by_sley | index("parser_ast_node_report_builder_task_execution")) and (.bootstrap_owned_by_sley | index("parser_ast_node_kind_task_execution")) and (.bootstrap_owned_by_sley | index("parser_ast_node_not_found_diagnostic"))'

self_hosting_status_field '(.bootstrap_owned_by_sley | index("parser_ast_node_parent_id_task_execution"))'

self_hosting_status_field '(.bootstrap_owned_by_sley | index("parser_ast_node_not_found_message"))'

self_hosting_status_field '(.bootstrap_owned_by_sley | index("parser_large_self_source_ast_execution"))'

bin/sley ast --json self-hosted/src/loom/checker.sley \
  | json_field '.schema == "sley.ast.program.v0" and (.tasks | length) >= 100 and ([.tasks[]?.name] | index("diagnostic_pass_descriptors"))'

self_hosting_status_field '(.bootstrap_owned_by_sley | index("parser_message_parser_prefix_task_execution"))'

self_hosting_status_field '(.bootstrap_owned_by_sley | index("symbol_graph_report_builder")) and (.bootstrap_owned_by_sley | index("symbol_graph_report_builder_task_execution")) and (.bootstrap_owned_by_sley | index("claim_verify_report_builder")) and (.bootstrap_owned_by_sley | index("claim_verify_report_builder_task_execution")) and (.bootstrap_owned_by_sley | index("migrate_report_builder")) and (.bootstrap_owned_by_sley | index("migrate_report_builder_task_execution")) and (.bootstrap_owned_by_sley | index("docgen_report_builder")) and (.bootstrap_owned_by_sley | index("docgen_report_builder_task_execution")) and (.bootstrap_owned_by_sley | index("sandbox_report_builder")) and (.bootstrap_owned_by_sley | index("sandbox_report_builder_task_execution")) and (.bootstrap_owned_by_sley | index("agent_bench_report_builder")) and (.bootstrap_owned_by_sley | index("agent_bench_report_builder_task_execution")) and (.bootstrap_owned_by_sley | index("deploy_report_builder")) and (.bootstrap_owned_by_sley | index("deploy_report_builder_task_execution"))'

self_hosting_status_field '(.bootstrap_owned_by_sley | index("conformance_report_builder")) and (.bootstrap_owned_by_sley | index("conformance_report_builder_task_execution")) and (.bootstrap_owned_by_sley | index("conformance_coverage_report_builder")) and (.bootstrap_owned_by_sley | index("conformance_coverage_report_builder_task_execution"))'

self_hosting_status_field '(.bootstrap_owned_by_sley | index("graft_outcome_report_builder")) and (.bootstrap_owned_by_sley | index("graft_outcome_report_builder_task_execution")) and (.bootstrap_owned_by_sley | index("ci_report_builder")) and (.bootstrap_owned_by_sley | index("ci_report_builder_task_execution")) and (.bootstrap_owned_by_sley | index("contract_inventory_report_builder")) and (.bootstrap_owned_by_sley | index("contract_inventory_report_builder_task_execution")) and (.bootstrap_owned_by_sley | index("contract_validate_report_builder")) and (.bootstrap_owned_by_sley | index("contract_validate_report_builder_task_execution")) and (.bootstrap_owned_by_sley | index("contract_fixture_check_report_builder")) and (.bootstrap_owned_by_sley | index("contract_fixture_check_report_builder_task_execution")) and (.bootstrap_owned_by_sley | index("deploy_artifact_check_report_builder")) and (.bootstrap_owned_by_sley | index("deploy_artifact_check_report_builder_task_execution")) and (.bootstrap_owned_by_sley | index("workbench_report_builder")) and (.bootstrap_owned_by_sley | index("workbench_report_builder_task_execution")) and (.bootstrap_owned_by_sley | index("zjx_tool_report_builder")) and (.bootstrap_owned_by_sley | index("zjx_tool_report_builder_task_execution"))'

self_hosting_status_field '(.bootstrap_owned_by_sley | index("reports_source_metadata_task_execution"))'

self_hosting_status_field '(.bootstrap_owned_by_sley | index("reports_fallback_removal_task_execution")) and (.bootstrap_owned_by_sley | index("bootstrap_status_fallback_removal_task_execution"))'

if grep -Fq 'extract_sley_string_task "$SELF_HOSTED_SOURCE_ROOT/loom/reports.sley"' bin/sley \
  || grep -Fq 'extract_sley_list_task_json "$SELF_HOSTED_SOURCE_ROOT/loom/reports.sley"' bin/sley; then
  fail "report metadata must execute through Sley source tasks, not raw host extractors"
fi

if grep -Eq '^(VERSION|SCHEMA_[A-Z0-9_]+|SELF_HOSTING_REPORT_SOURCE_ROOT)="\$\{[^}]+:-' bin/sley \
  || grep -Eq '^[[:space:]]*(DEFAULT_RULES_JSON|[A-Z0-9_]+_(REPORT_)?(FIELDS|BUILDER)_JSON)='\''\[' bin/sley \
  || grep -Eq 'if \[\[ "\$[A-Z0-9_]+_(REPORT_)?(FIELDS|BUILDER)_JSON" == "\[\]" \]\]' bin/sley; then
  fail "bootstrap/report metadata must come from Sley source without host fallback literals"
fi

if grep -Fq "owned_json='[\"implementation_version\"" bin/sley \
  || grep -Fq 'status="${status:-bootstrap}"' bin/sley \
  || grep -Fq 'strict="${strict:-false}"' bin/sley \
  || grep -Fq 'eval_bootstrap_list_count_task semantic_source_count || true' bin/sley \
  || grep -Fq 'find "$SELF_HOSTED_SOURCE_ROOT" -type f -name' bin/sley; then
  fail "self-hosting status metadata must come from loom.bootstrap without host fallback discovery"
fi

if ! grep -Fq 'sley_source_task loom.reports' bin/sley \
  || ! grep -Fq 'sley_source_list_task_json loom.reports' bin/sley; then
  fail "report metadata source task dispatch is missing"
fi

self_hosting_status_field '(.bootstrap_owned_by_sley | index("run_report_shape"))'

self_hosting_status_field '(.bootstrap_owned_by_sley | index("runtime_status_parser_prefix_task_execution"))'

self_hosting_status_field '(.bootstrap_owned_by_sley | index("generic_pure_task_evaluator_execution"))'

self_hosting_status_field '(.bootstrap_owned_by_sley | index("text_concat_runtime_task_execution"))'

self_hosting_status_field '(.bootstrap_owned_by_sley | index("bool_identity_runtime_task_execution"))'

self_hosting_status_field '(.bootstrap_owned_by_sley | index("generic_source_main_runtime_execution"))'

if ! self_hosting_status_field '(.bootstrap_owned_by_sley | index("generic_runtime_return_type_plan_task_execution")) and (.bootstrap_owned_by_sley | index("generic_runtime_return_type_dispatch_execution"))'; then
  fail "generic runtime return-type dispatch markers are missing"
fi

if ! grep -Fq 'RUNTIME_GENERIC_RETURN_PLAN_JSON="$(sley_source_list_task_json loom.runtime generic_runtime_return_type_plan' bin/sley; then
  fail "generic runtime return-type dispatch plan source task dispatch is missing"
fi

if ! awk '
  /^runtime_generic_return_descriptor_json\(\)[ \t]*\{/ {in_fn=1}
  in_fn && /RUNTIME_GENERIC_RETURN_PLAN_JSON/ {plan=1}
  in_fn && /^[ \t]*}/ {exit}
  END {exit plan ? 0 : 1}
' bin/sley; then
  fail "generic runtime return-type descriptor lookup must read the Sley-owned plan"
fi

if ! awk '
  /^eval_source_generic_main_return\(\)[ \t]*\{/ {in_fn=1}
  in_fn && /runtime_generic_return_descriptor_json/ {descriptor=1}
  in_fn && /runtime_generic_eval_value_task/ {value_task=1}
  in_fn && /^}/ {exit}
  END {exit (descriptor && value_task) ? 0 : 1}
' bin/sley; then
  fail "generic runtime main evaluation must dispatch return value normalization through the Sley-owned return-type plan"
fi

if awk '
  /^eval_source_generic_main_return\(\)[ \t]*\{/ {in_fn=1}
  in_fn && /^[ \t]*case[ \t]+"\$return_type"[ \t]+in/ {host_case=1}
  in_fn && /^}/ {exit}
  END {exit host_case ? 0 : 1}
' bin/sley; then
  fail "generic runtime main evaluation must not use a host hard-coded return-type case ladder"
fi

if ! self_hosting_status_field '(.bootstrap_owned_by_sley | index("generic_source_if_expression_runtime_execution"))'; then
  fail "generic source if-expression runtime marker is missing"
fi

if ! self_hosting_status_field '(.bootstrap_owned_by_sley | index("generic_source_state_set_runtime_execution"))'; then
  fail "generic source state/set runtime marker is missing"
fi

bin/sley run --json "$generic_source_main_source" \
  | json_field '.status == "passed" and .value.kind == "Text" and .value.value == "hi Ada"'

bin/sley run --json "$generic_state_set_source" \
  | json_field '.status == "passed" and .value.kind == "Text" and .value.value == "hi Ada"'

self_hosting_status_field '(.bootstrap_owned_by_sley | index("generic_if_assignment_runtime_execution"))'

bin/sley run --json "$generic_if_assignment_source" \
  | json_field '.status == "passed" and .value.kind == "Text" and .value.value == "ready"'

self_hosting_status_field '(.bootstrap_owned_by_sley | index("generic_result_err_runtime_execution"))'

bin/sley run --json "$generic_result_err_source" \
  | json_field '.status == "passed" and .value.kind == "Err" and .value.value.kind == "Text" and .value.value.value == "boom"'

self_hosting_status_field '(.bootstrap_owned_by_sley | index("generic_linear_tally_runtime_execution"))'

bin/sley run --json examples/mutable_binding_style.sley \
  | json_field '.status == "passed" and .value.kind == "Int" and .value.value == 21'

self_hosting_status_field '(.bootstrap_owned_by_sley | index("generic_while_loop_runtime_execution"))'

bin/sley run --json "$generic_while_source" \
  | json_field '.status == "passed" and .value.kind == "Text" and .value.value == "AdaLovelace"'

self_hosting_status_field '(.bootstrap_owned_by_sley | index("generic_each_loop_runtime_execution"))'

bin/sley run --json "$generic_each_source" \
  | json_field '.status == "passed" and .value.kind == "Text" and .value.value == "AdaLovelace"'

self_hosting_status_field '(.bootstrap_owned_by_sley | index("generic_for_loop_runtime_execution"))'

bin/sley run --json "$generic_for_source" \
  | json_field '.status == "passed" and .value.kind == "Text" and .value.value == "AdaLovelace"'

self_hosting_status_field '(.bootstrap_owned_by_sley | index("generic_runtime_scalar_task_evaluator_execution"))'

self_hosting_status_field '(.bootstrap_owned_by_sley | index("generic_runtime_collection_task_evaluator_execution"))'

self_hosting_status_field '(.bootstrap_owned_by_sley | index("generic_runtime_record_task_evaluator_execution"))'

self_hosting_status_field '(.bootstrap_owned_by_sley | index("generic_parser_checker_template_evaluator_execution"))'

self_hosting_status_field '(.bootstrap_owned_by_sley | index("generic_source_call_argument_evaluator_execution"))'

self_hosting_status_field '(.bootstrap_owned_by_sley | index("generic_source_bind_evaluator_execution"))'

self_hosting_status_field '(.bootstrap_owned_by_sley | index("generic_literal_list_index_runtime_execution"))'

self_hosting_status_field '(.bootstrap_owned_by_sley | index("generic_literal_map_index_runtime_execution"))'

self_hosting_status_field '(.bootstrap_owned_by_sley | index("generic_literal_record_field_runtime_execution"))'

self_hosting_status_field '(.bootstrap_owned_by_sley | index("generic_bound_collection_index_runtime_execution"))'

bin/sley run --json fixtures/corpus/accepted/collections_indexing.sley \
  | json_field '.status == "passed" and .value.kind == "Int" and .value.value == 8'

self_hosting_status_field '(.bootstrap_owned_by_sley | index("generic_record_argument_runtime_execution"))'

bin/sley run --json fixtures/corpus/accepted/records_and_calls.sley \
  | json_field '.status == "passed" and .value.kind == "Text" and .value.value == "Ada"'

if ! awk '
  /^sley_eval_source_call\(\)[ \t]*\{/ {in_fn=1}
  in_fn && /sley_eval_source_task_takes_json/ {takes=1}
  in_fn && /sley_eval_source_call_env_json/ {binds=1}
  in_fn && /^[ \t]*}/ {exit}
  END {exit (takes && binds) ? 0 : 1}
' bin/sley; then
  fail "sley_eval_source_call must bind call arguments to Sley take names before task evaluation"
fi

for source_bind_eval_fn in eval_bootstrap_smoke_task eval_bootstrap_list_count_task; do
  if ! awk -v fn="$source_bind_eval_fn" '
    $0 ~ "^" fn "\\(\\)[ \t]*\\{" {in_fn=1}
    in_fn && /sley_eval_source_task/ {found=1}
    in_fn && /^[ \t]*}/ {exit}
    END {exit found ? 0 : 1}
  ' bin/sley; then
    fail "$source_bind_eval_fn must execute through the shared Sley source task evaluator"
  fi
done

self_hosting_status_field '(.bootstrap_owned_by_sley | index("bootstrap_smoke_fallback_removal_task_execution"))'

if grep -Fq 'eval_bootstrap_smoke_task || printf' bin/sley; then
  fail "self-hosted runtime smoke must execute through loom.bootstrap without host fallback counts"
fi

if grep -Fq 'extract_sley_string_task "$ROOT_DIR/examples/hello.sley" main' bin/sley \
  || grep -Fq 'value="${value:-hello sley}"' bin/sley; then
  fail "FileRead runtime seeds must execute the source file task without host fallback literals"
fi

for source_task_eval_fn in \
  eval_parser_id_template \
  eval_parser_message_template \
  eval_checker_message_template \
  eval_checker_status_task \
  eval_lint_status_task \
  eval_runtime_status_task \
  eval_runtime_text_probe_task \
  eval_seeded_agent_deploy_value \
  eval_runtime_spend_prefixed_value_task \
  eval_runtime_list_len_value_task \
  eval_runtime_int_identity_value_task \
  eval_runtime_text_identity_value_task \
  eval_runtime_int_binary_value_task \
  eval_runtime_int_less_than_value_task \
  eval_runtime_int_equal_value_task \
  eval_runtime_int_greater_equal_value_task \
  eval_runtime_list_index_int_value_task \
  eval_runtime_list_index_text_value_task \
  eval_runtime_map_index_text_value_task \
  eval_runtime_map_index_int_value_task \
  eval_runtime_record_field_text_value_task \
  eval_runtime_bool_equal_value_task \
  eval_runtime_text_equal_value_task \
  eval_runtime_bool_and_value_task \
  eval_runtime_bool_or_value_task \
  eval_runtime_bool_not_value_task \
  eval_runtime_bool_if_value_task \
  eval_runtime_int_if_value_task \
  eval_runtime_text_if_value_task \
  eval_runtime_ok_int_value_task \
  eval_runtime_ok_text_value_task
do
  if ! awk -v fn="$source_task_eval_fn" '
    $0 ~ "^" fn "\\(\\)[ \t]*\\{" {in_fn=1}
    in_fn && /sley_eval_source_task/ {found=1}
    in_fn && /^[ \t]*}/ {exit}
    END {exit found ? 0 : 1}
  ' bin/sley; then
    fail "$source_task_eval_fn must execute through the shared Sley source task evaluator"
  fi
done

self_hosting_status_field '(.bootstrap_owned_by_sley | index("runtime_project_probe_parser_prefix_task_execution")) and (.bootstrap_owned_by_sley | index("runtime_project_binding_parser_prefix_task_execution"))'

self_hosting_status_field '(.bootstrap_owned_by_sley | index("runtime_project_probe_fallback_parser_prefix_task_execution")) and (.bootstrap_owned_by_sley | index("runtime_project_binding_fallback_parser_prefix_task_execution"))'

self_hosting_status_field '(.bootstrap_owned_by_sley | index("unit_runtime_task_execution"))'

self_hosting_status_field '(.bootstrap_owned_by_sley | index("unit_main_runtime_execution"))'

bin/sley run --json examples/declaration_hygiene.sley \
  | json_field '.status == "passed" and .value.kind == "Unit" and (.value | has("value") | not)'

self_hosting_status_field '(.bootstrap_owned_by_sley | index("main_take_runtime_diagnostic"))'

if bin/sley run --json examples/unused_take.sley > "$runtime_report"; then
  fail "main with ordinary runtime takes should not execute without arguments"
fi
jq -er '.schema == "sley.diagnostics.report.v0" and .status == "error" and .diagnostics[0].id == "RUNTIME_TAKE_REQUIRED" and .diagnostics[0].message == "`main` requires unsupported runtime take `value`"' "$runtime_report" >/dev/null

self_hosting_status_field '(.bootstrap_owned_by_sley | index("verify_report_shape"))'

self_hosting_status_field '(.bootstrap_owned_by_sley | index("graft_outcome_report_shape"))'

self_hosting_status_field '(.bootstrap_owned_by_sley | index("claim_verify_report_shape"))'

self_hosting_status_field '(.bootstrap_owned_by_sley | index("conformance_report_shape"))'

self_hosting_status_field '(.bootstrap_owned_by_sley | index("conformance_coverage_report_shape"))'

self_hosting_status_field '(.bootstrap_owned_by_sley | index("ci_report_shape"))'

self_hosting_status_field '(.bootstrap_owned_by_sley | index("contract_report_shapes"))'

self_hosting_status_field '(.bootstrap_owned_by_sley | index("deploy_report_shape"))'

self_hosting_status_field '(.bootstrap_owned_by_sley | index("deploy_artifact_check_report_shape"))'

self_hosting_status_field '(.bootstrap_owned_by_sley | index("migrate_report_shape"))'

self_hosting_status_field '(.bootstrap_owned_by_sley | index("docgen_report_shape"))'

self_hosting_status_field '(.bootstrap_owned_by_sley | index("docgen_capabilities_host_effect_needles_task_execution"))'

self_hosting_status_field '(.bootstrap_owned_by_sley | index("docgen_call_expression_prefix_task_execution"))'

self_hosting_status_field '(.bootstrap_owned_by_sley | index("docgen_call_tail_parser_prefix_task_execution"))'

self_hosting_status_field '(.bootstrap_owned_by_sley | index("workbench_report_shape"))'

self_hosting_status_field '(.bootstrap_owned_by_sley | index("workbench_call_expression_prefix_task_execution"))'

self_hosting_status_field '(.bootstrap_owned_by_sley | index("workbench_call_tail_parser_prefix_task_execution"))'

self_hosting_status_field '(.bootstrap_owned_by_sley | index("sandbox_report_shape"))'

self_hosting_status_field '(.bootstrap_owned_by_sley | index("agent_bench_report_shape"))'

self_hosting_status_field '(.bootstrap_owned_by_sley | index("zjx_tool_report_shape"))'

self_hosting_status_field '(.bootstrap_owned_by_sley | index("diagnostics_report_shape"))'

self_hosting_status_field '(.bootstrap_owned_by_sley | index("lint_status_parser_prefix_task_execution"))'

self_hosting_status_field '(.bootstrap_owned_by_sley | index("lint_report_shape"))'

self_hosting_status_field '(.bootstrap_owned_by_sley | index("lint_declared_effect_host_needles_task_execution"))'

self_hosting_status_field '(.bootstrap_owned_by_sley | index("lint_call_expression_prefix_task_execution"))'

self_hosting_status_field '(.bootstrap_owned_by_sley | index("query_report_shape"))'

self_hosting_status_field '(.bootstrap_owned_by_sley | index("symbol_graph_report_shape"))'

self_hosting_status_field '(.bootstrap_owned_by_sley | index("doctor_report_shape"))'

self_hosting_status_field '(.bootstrap_owned_by_sley | index("ok_int_runtime_task_execution"))'

self_hosting_status_field '(.bootstrap_owned_by_sley | index("ok_text_runtime_task_execution"))'

self_hosting_status_field '(.bootstrap_owned_by_sley | index("text_equal_runtime_task_execution"))'

self_hosting_status_field '(.bootstrap_owned_by_sley | index("result_flow_runtime_execution"))'

self_hosting_status_field '(.bootstrap_owned_by_sley | index("result_flow_parser_prefix_runtime_execution"))'

bin/sley run --json fixtures/corpus/accepted/result_flow.sley \
  | json_field '.status == "passed" and .value.kind == "Ok" and .value.value.kind == "Int" and .value.value.value == 42'

self_hosting_status_field '(.bootstrap_owned_by_sley | index("text_identity_runtime_task_execution"))'

self_hosting_status_field '(.bootstrap_owned_by_sley | index("host_call_text_runtime_execution"))'

self_hosting_status_field '(.bootstrap_owned_by_sley | index("runtime_host_default_texts_task_execution"))'

self_hosting_status_field '(.bootstrap_owned_by_sley | index("spend_prefix_runtime_task_execution"))'

self_hosting_status_field '(.bootstrap_owned_by_sley | index("direct_file_read_runtime_execution"))'

self_hosting_status_field '(.bootstrap_owned_by_sley | index("direct_file_read_parser_prefix_runtime_execution"))'

self_hosting_status_field '(.bootstrap_owned_by_sley | index("direct_file_read_result_runtime_execution"))'

self_hosting_status_field '(.bootstrap_owned_by_sley | index("runtime_file_read_source_task_execution"))'

self_hosting_status_field '(.bootstrap_owned_by_sley | index("seeded_host_result_source_runtime_execution"))'

self_hosting_status_field '(.bootstrap_owned_by_sley | index("database_row_source_runtime_execution"))'

self_hosting_status_field '(.bootstrap_owned_by_sley | index("seeded_host_text_source_runtime_execution"))'

self_hosting_status_field '(.bootstrap_owned_by_sley | index("gate_take_source_call_runtime_execution"))'

self_hosting_status_field '(.bootstrap_owned_by_sley | index("project_import_source_call_runtime_execution"))'

self_hosting_status_field '(.bootstrap_owned_by_sley | index("project_entry_source_runtime_execution"))'

if grep -Fq 'file="$(collect_files "$target" | head -n 1)"' bin/sley; then
  fail "source evaluators must use sley.toml entry files for project targets"
fi

for source_entry_fn in eval_source_seeded_host_result_main_return eval_source_seeded_host_text_main_return; do
  if awk -v fn="$source_entry_fn" '
    $0 ~ "^" fn "\\(\\)" {in_fn=1}
    in_fn && /collect_files "\$target" \| head -n 1/ {found=1}
    in_fn && /^}/ {in_fn=0}
    END {exit found ? 0 : 1}
  ' bin/sley; then
    fail "$source_entry_fn must use sley.toml entry files for project targets"
  fi
done

self_hosting_status_field '(.bootstrap_owned_by_sley | index("runtime_host_effect_fallback_removal_execution"))'

if grep -Eq 'elif runtime_project_ready_probe_matches "\$target"' bin/sley; then
  fail "project runtime calls must execute through source evaluation, not project-ready probe fallback"
fi

self_hosting_status_field '(.bootstrap_owned_by_sley | index("project_ready_runtime_fallback_removal_execution"))'

if grep -Eq 'elif runtime_source_has_host_effect "\$target" "(FileRead|FileWrite|DatabaseWrite|DatabaseRead|Network|Shell|ModelCall|SecretRead)"' bin/sley; then
  fail "runtime host effects must execute through source evaluation, not generic host-effect fallbacks"
fi

if grep -Eq 'elif .*runtime_agent_pipeline_probe_matches "\$target"|elif runtime_deploy_stage_probe_matches "\$target"|elif runtime_spend_authorize_probe_matches "\$target"' bin/sley; then
  fail "agent/deploy/spend runtime must execute through source evaluation, not probe-based fallbacks"
fi

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

self_hosting_status_field '(.bootstrap_owned_by_sley | index("list_index_runtime_task_execution"))'

self_hosting_status_field '(.bootstrap_owned_by_sley | index("list_index_text_runtime_task_execution"))'

self_hosting_status_field '(.bootstrap_owned_by_sley | index("list_index_expression_runtime_execution"))'

self_hosting_status_field '(.bootstrap_owned_by_sley | index("bound_list_index_sum_runtime_execution"))'

bin/sley run --json fixtures/corpus/accepted/type_alias_transparency.sley \
  | json_field '.status == "passed" and .value.kind == "Int" and .value.value == 42'

self_hosting_status_field '(.bootstrap_owned_by_sley | index("map_index_runtime_task_execution"))'

self_hosting_status_field '(.bootstrap_owned_by_sley | index("map_index_int_runtime_task_execution"))'

self_hosting_status_field '(.bootstrap_owned_by_sley | index("map_index_expression_runtime_execution"))'

self_hosting_status_field '(.bootstrap_owned_by_sley | index("collection_index_sum_runtime_execution"))'

bin/sley run --json fixtures/corpus/accepted/collections_indexing.sley \
  | json_field '.status == "passed" and .value.kind == "Int" and .value.value == 8'

self_hosting_status_field '(.bootstrap_owned_by_sley | index("record_field_runtime_task_execution"))'

self_hosting_status_field '(.bootstrap_owned_by_sley | index("record_field_expression_runtime_execution"))'

self_hosting_status_field '(.bootstrap_owned_by_sley | index("record_field_call_runtime_execution"))'

self_hosting_status_field '(.bootstrap_owned_by_sley | index("record_field_call_parser_prefix_runtime_execution"))'

bin/sley run --json fixtures/corpus/accepted/records_and_calls.sley \
  | json_field '.status == "passed" and .value.kind == "Text" and .value.value == "Ada"'

self_hosting_status_field '(.bootstrap_owned_by_sley | index("local_call_runtime_execution"))'

self_hosting_status_field '(.bootstrap_owned_by_sley | index("local_call_parser_prefix_runtime_execution"))'

bin/sley run --json examples/unused_private_task.sley \
  | json_field '.status == "passed" and .value.kind == "Int" and .value.value == 1'

bin/sley run --json examples/dead_private_tasks.sley \
  | json_field '.status == "passed" and .value.kind == "Int" and .value.value == 1'

self_hosting_status_field '(.bootstrap_owned_by_sley | index("zero_arg_project_call_runtime_execution"))'

self_hosting_status_field '(.bootstrap_owned_by_sley | index("zero_arg_project_call_parser_prefix_runtime_execution"))'

bin/sley run --json examples/unused_import_project \
  | json_field '.status == "passed" and .value.kind == "Int" and .value.value == 7'

bin/sley run --json examples/duplicate_import_project \
  | json_field '.status == "passed" and .value.kind == "Text" and .value.value == "ready"'

self_hosting_status_field '(.bootstrap_owned_by_sley | index("bool_comparison_runtime_task_execution"))'

self_hosting_status_field '(.bootstrap_owned_by_sley | index("bool_comparison_runtime_execution"))'

self_hosting_status_field '(.bootstrap_owned_by_sley | index("bool_and_runtime_task_execution"))'

self_hosting_status_field '(.bootstrap_owned_by_sley | index("bool_and_expression_runtime_execution"))'

self_hosting_status_field '(.bootstrap_owned_by_sley | index("bool_not_runtime_task_execution"))'

self_hosting_status_field '(.bootstrap_owned_by_sley | index("not_expression_runtime_execution"))'

self_hosting_status_field '(.bootstrap_owned_by_sley | index("double_not_expression_runtime_execution"))'

self_hosting_status_field '(.bootstrap_owned_by_sley | index("negated_comparison_runtime_execution"))'

self_hosting_status_field '(.bootstrap_owned_by_sley | index("int_arithmetic_runtime_task_execution"))'

if ! self_hosting_status_field '(.bootstrap_owned_by_sley | index("runtime_int_binary_operator_plan_task_execution")) and (.bootstrap_owned_by_sley | index("runtime_int_binary_operator_dispatch_execution"))'; then
  fail "runtime int binary operator plan markers are missing"
fi

if ! grep -Fq 'RUNTIME_INT_BINARY_OPERATOR_PLAN_JSON="$(sley_source_list_task_json loom.runtime int_binary_operator_plan' bin/sley; then
  fail "runtime int binary operator plan source task dispatch is missing"
fi

if ! grep -Fq 'runtime_int_binary_task_name()' bin/sley \
  || ! grep -Fq 'select(.op == $op) | .task' bin/sley \
  || ! grep -Fq 'eval_runtime_int_binary_operator_value_task "$op"' bin/sley; then
  fail "runtime int binary operator dispatch must use Sley-owned operator plan"
fi

if grep -Fq '[[ "$op" == "+" || "$op" == "*" ]]' bin/sley \
  || grep -Fq 'task_name="int_add_value"' bin/sley \
  || grep -Fq 'task_name="int_multiply_value"' bin/sley; then
  fail "runtime int binary operator dispatch must not keep host hard-coded operator map"
fi

self_hosting_status_field '(.bootstrap_owned_by_sley | index("constant_arithmetic_runtime_execution"))'

self_hosting_status_field '(.bootstrap_owned_by_sley | index("absorbing_arithmetic_runtime_execution"))'

self_hosting_status_field '(.bootstrap_owned_by_sley | index("bound_arithmetic_runtime_execution"))'

self_hosting_status_field '(.bootstrap_owned_by_sley | index("int_comparison_runtime_task_execution"))'

self_hosting_status_field '(.bootstrap_owned_by_sley | index("int_equal_runtime_task_execution"))'

self_hosting_status_field '(.bootstrap_owned_by_sley | index("int_greater_equal_runtime_task_execution"))'

self_hosting_status_field '(.bootstrap_owned_by_sley | index("constant_comparison_runtime_execution"))'

self_hosting_status_field '(.bootstrap_owned_by_sley | index("bool_if_runtime_task_execution"))'

self_hosting_status_field '(.bootstrap_owned_by_sley | index("bool_if_expression_runtime_execution"))'

self_hosting_status_field '(.bootstrap_owned_by_sley | index("bool_if_statement_runtime_execution"))'

self_hosting_status_field '(.bootstrap_owned_by_sley | index("text_if_runtime_task_execution"))'

self_hosting_status_field '(.bootstrap_owned_by_sley | index("compute_text_call_runtime_execution"))'

self_hosting_status_field '(.bootstrap_owned_by_sley | index("compute_text_call_parser_prefix_runtime_execution"))'

bin/sley run --json examples/compute.sley \
  | json_field '.status == "passed" and .value.kind == "Text" and .value.value == "excellent"'

self_hosting_status_field '(.bootstrap_owned_by_sley | index("int_if_runtime_task_execution"))'

self_hosting_status_field '(.bootstrap_owned_by_sley | index("int_if_expression_runtime_execution"))'

self_hosting_status_field '(.bootstrap_owned_by_sley | index("int_if_statement_runtime_execution"))'

self_hosting_status_field '(.bootstrap_owned_by_sley | index("literal_if_statement_runtime_execution"))'

self_hosting_status_field '(.bootstrap_owned_by_sley | index("comparison_if_statement_runtime_execution"))'

self_hosting_status_field '(.bootstrap_owned_by_sley | index("constant_false_while_runtime_execution"))'

self_hosting_status_field '(.bootstrap_owned_by_sley | index("while_list_sum_runtime_execution"))'

self_hosting_status_field '(.bootstrap_owned_by_sley | index("while_list_sum_parser_prefix_runtime_execution"))'

self_hosting_status_field '(.bootstrap_owned_by_sley | index("generic_runtime_block_support_task_execution"))'

self_hosting_status_field '(.bootstrap_owned_by_sley | index("generic_while_source_dispatch_runtime_execution"))'

self_hosting_status_field '(.bootstrap_owned_by_sley | index("runtime_dispatch_order_task_execution"))'

if ! self_hosting_status_field '(.bootstrap_owned_by_sley | index("runtime_dispatch_plan_task_execution")) and (.bootstrap_owned_by_sley | index("runtime_dispatch_id_map_task_execution")) and (.bootstrap_owned_by_sley | index("runtime_dispatch_function_map_task_execution")) and (.bootstrap_owned_by_sley | index("runtime_dispatch_function_execution"))'; then
  fail "runtime dispatch plan/id/function-map markers are missing"
fi

if grep -Fq 'elif simple_runtime="$(eval_source_unit_main_return "$target" 2>/dev/null)"' bin/sley; then
  fail "runtime dispatch order must come from loom.runtime, not a host hard-coded elif chain"
fi

if ! grep -Fq 'RUNTIME_DISPATCH_ORDER_JSON="$(sley_source_list_task_json loom.runtime runtime_dispatch_order)"' bin/sley; then
  fail "runtime dispatch order source task dispatch is missing"
fi

if ! grep -Fq 'RUNTIME_DISPATCH_PLAN_JSON="$(sley_source_list_task_json loom.runtime runtime_dispatch_plan' bin/sley \
  || ! grep -Fq 'evaluator_function:.[2]' bin/sley; then
  fail "runtime dispatch plan source task dispatch is missing"
fi

if ! awk '
  /^run_json\(\)[ \t]*\{/ {in_fn=1}
  in_fn && /runtime_dispatch_candidate_json/ {candidate=1}
  in_fn && /RUNTIME_DISPATCH_PLAN_JSON/ {plan=1}
  in_fn && /^[ \t]*}/ {exit}
  END {exit (candidate && plan) ? 0 : 1}
' bin/sley; then
  fail "run_json must iterate Sley-owned runtime dispatch candidates"
fi

if grep -Fq 'unit_main) eval_source_unit_main_return "$target" ;;' bin/sley \
  || grep -Fq 'generic_main) eval_source_generic_main_return "$target"' bin/sley; then
  fail "runtime dispatch candidate execution must use Sley-owned evaluator ids, not host string branches"
fi

if grep -Fq 'case "$evaluator_id" in' bin/sley; then
  fail "runtime dispatch candidate execution must not use a host hard-coded evaluator-id case ladder"
fi

if ! awk '
  /^runtime_dispatch_candidate_json\(\)[ \t]*\{/ {in_fn=1}
  in_fn && /runtime_dispatch_candidate_descriptor_json/ {descriptor=1}
  in_fn && /evaluator_function/ {source_function=1}
  in_fn && /declare -F "\$evaluator_fn"/ {declared=1}
  in_fn && /"\$evaluator_fn" "\$target" "\$http_text"/ {dynamic_call=1}
  in_fn && /^}/ {exit}
  END {exit (descriptor && source_function && declared && dynamic_call) ? 0 : 1}
' bin/sley; then
  fail "runtime dispatch candidate execution must call the Sley-owned evaluator function map"
fi

if grep -Fq 'line ~ /^(while|forge)([ \t{]|$)/' bin/sley; then
  fail "generic runtime block support must be driven by loom.runtime, not a host hard-coded while/forge block list"
fi

bin/sley run --json examples/collections.sley \
  | json_field '.status == "passed" and .value.kind == "Int" and .value.value == 10'

self_hosting_status_field '(.bootstrap_owned_by_sley | index("state_set_runtime_execution"))'

self_hosting_status_field '(.bootstrap_owned_by_sley | index("tally_set_runtime_execution"))'

self_hosting_status_field '(.bootstrap_owned_by_sley | index("each_sum_runtime_execution"))'

bin/sley run --json fixtures/corpus/accepted/mutable_sum.sley \
  | json_field '.status == "passed" and .value.kind == "Int" and .value.value == 10'

self_hosting_status_field '(.bootstrap_owned_by_sley | index("each_map_sum_runtime_execution"))'

bin/sley run --json examples/maps_for.sley \
  | json_field '.status == "passed" and .value.kind == "Int" and .value.value == 15'

self_hosting_status_field '(.bootstrap_owned_by_sley | index("project_call_runtime_execution"))'

self_hosting_status_field '(.bootstrap_owned_by_sley | index("project_call_parser_prefix_runtime_execution"))'

self_hosting_status_field '(.bootstrap_owned_by_sley | index("generic_unqualified_import_call_runtime_execution"))'

if ! awk '
  /^sley_eval_source_call\(\)[ \t]*\{/ {in_fn=1}
  in_fn && /resolve_sley_unqualified_imported_task_file/ {found=1}
  in_fn && /^[ \t]*}/ {exit}
  END {exit found ? 0 : 1}
' bin/sley; then
  fail "generic source call evaluator must resolve unqualified imported tasks before project-call host fallback"
fi

self_hosting_status_field '(.bootstrap_owned_by_sley | index("unqualified_project_call_runtime_execution"))'

bin/sley run --json examples/unqualified_import_call_project \
  | json_field '.status == "passed" and .value.kind == "Int" and .value.value == 42'

self_hosting_status_field '(.bootstrap_owned_by_sley | index("file_entry_project_call_runtime_execution"))'

bin/sley run --json examples/unqualified_import_call_project/src/app/main.sley \
  | json_field '.status == "passed" and .value.kind == "Int" and .value.value == 42'

self_hosting_status_field '(.bootstrap_owned_by_sley | index("file_entry_runtime_authority_execution"))'

self_hosting_status_field '(.bootstrap_owned_by_sley | index("bound_local_call_runtime_execution"))'

self_hosting_status_field '(.bootstrap_owned_by_sley | index("runtime_main_call_parser_prefix_task_execution"))'

bin/sley run --json fixtures/corpus/accepted/pure_main.sley \
  | json_field '.status == "passed" and .value.kind == "Int" and .value.value == 42'

self_hosting_status_field '(.bootstrap_owned_by_sley | index("int_identity_runtime_task_execution"))'

self_hosting_status_field '(.bootstrap_owned_by_sley | index("bound_int_return_runtime_execution"))'

bin/sley ast --json examples/hello.sley \
  | json_field '.schema == "sley.ast.program.v0" and .module == "app.hello" and .tasks[0].id == "task:app.hello.main" and .tasks[0].name == "main" and .tasks[0].body.statements[0].id == "block:task:app.hello.main:stmt:0" and .tasks[0].body.statements[0].expr.id == "block:task:app.hello.main:stmt:0:expr" and .tasks[0].body.statements[0].expr.expr_kind == "StringLiteral"'

bin/sley ast --json examples/declaration_hygiene.sley \
  | json_field '.types[0].id == "type:app.hygiene.Orphan" and .effects[0].id == "effect:app.hygiene.OrphanEffect"'

bin/sley ast --json examples/unused_take.sley \
  | json_field '.tasks[0].takes[0].id == "take:app.takes.main.value" and .tasks[0].takes[1].id == "take:app.takes.main.unused"'

bin/sley lint --json --rule unused_take examples/unused_take.sley \
  | json_field '.schema == "sley.lint.report.v0" and .findings[0].node == "take:task:app.takes.main:1:unused"'

bin/sley ast --json examples/constant_text_concatenation_expression.sley \
  | json_field '.schema == "sley.ast.program.v0" and .tasks[0].body.statements[0].expr.expr_kind == "Binary" and .tasks[0].body.statements[0].expr.op == "Add" and .tasks[0].body.statements[0].expr.source == "\"Sley \" + \"agents\""'

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
  | json_field '.tasks[0].body.statements[0].kind == "Expr" and .tasks[0].body.statements[0].expr.expr_kind == "Try" and .tasks[0].body.statements[0].expr.expr.expr_kind == "Call"'

bin/sley check --json examples/hello.sley \
  | json_field '.schema == "sley.diagnostics.report.v0" and .status == "ok"'

bin/sley check --json examples/hello.sley \
  | json_field 'keys == (["schema","status","diagnostics"] | sort)'

self_hosting_status_field '(.bootstrap_owned_by_sley | index("checker_status_fallback_removal_task_execution"))'

if grep -Eq 'CHECK_(OK|ERROR)_STATUS="\$\{CHECK_(OK|ERROR)_STATUS:-' bin/sley; then
  fail "checker status names must come from loom.checker without shell fallback literals"
fi

if grep -Fq 'eval_checker_status_task "$diagnostic_count" ||' bin/sley; then
  fail "checker status selection must not fall back to shell-side status logic"
fi

self_hosting_status_field '(.bootstrap_owned_by_sley | index("checker_diagnostic_id_fallback_removal_task_execution"))'

if grep -Eq 'DIAG_[A-Z0-9_]+="\$\{DIAG_[A-Z0-9_]+:-' bin/sley; then
  fail "checker diagnostic ids must come from loom.checker without shell fallback literals"
fi

self_hosting_status_field '(.bootstrap_owned_by_sley | index("checker_diagnostic_pass_order_task_execution"))'

if grep -Fq '[duplicate_effect_diags, duplicate_type_diags, duplicate_task_diags' bin/sley; then
  fail "checker diagnostic pass order must come from loom.checker"
fi

self_hosting_status_field '(.bootstrap_owned_by_sley | index("checker_diagnostic_pass_descriptor_task_execution")) and (.bootstrap_owned_by_sley | index("checker_unknown_reference_pass_task_execution"))'

if ! self_hosting_status_field '(.bootstrap_owned_by_sley | index("checker_diagnostic_pass_plan_task_execution")) and (.bootstrap_owned_by_sley | index("checker_executor_id_map_task_execution"))'; then
  fail "checker diagnostic pass plan/id-map markers are missing"
fi

if ! self_hosting_status_field '(.bootstrap_owned_by_sley | index("checker_executor_name_map_task_execution")) and (.bootstrap_owned_by_sley | index("checker_descriptor_executor_name_validation_execution"))'; then
  fail "checker diagnostic executor-name map markers are missing"
fi

self_hosting_status_field '(.bootstrap_owned_by_sley | index("checker_descriptor_value_map_task_execution"))'

if ! self_hosting_status_field '(.bootstrap_owned_by_sley | index("checker_executor_dispatch_map_task_execution"))'; then
  fail "checker executor dispatch map marker is missing"
fi

self_hosting_status_field '(.bootstrap_owned_by_sley | index("checker_gate_reference_pass_task_execution")) and (.bootstrap_owned_by_sley | index("checker_effect_authorization_pass_task_execution"))'

self_hosting_status_field '(.bootstrap_owned_by_sley | index("checker_duplicate_pass_family_task_execution"))'

self_hosting_status_field '(.bootstrap_owned_by_sley | index("checker_record_shape_pass_family_task_execution"))'

self_hosting_status_field '(.bootstrap_owned_by_sley | index("checker_collection_pass_family_task_execution"))'

self_hosting_status_field '(.bootstrap_owned_by_sley | index("checker_call_pass_family_task_execution"))'

self_hosting_status_field '(.bootstrap_owned_by_sley | index("checker_qualified_task_call_policy_task_execution"))'

self_hosting_status_field '(.bootstrap_owned_by_sley | index("checker_type_return_pass_family_task_execution"))'

self_hosting_status_field '(.bootstrap_owned_by_sley | index("checker_identifier_resolution_pass_task_execution")) and (.bootstrap_owned_by_sley | index("checker_question_result_pass_task_execution"))'

if grep -Fq '|legacy|' self-hosted/src/loom/checker.sley; then
  fail "checker diagnostic pass descriptors must not retain legacy executor rows"
fi

if grep -Eq 'def (return_type_diags|take_type_diags|unknown_effect_diags|gate_take_type_diags|gate_effect_diags|direct_effect_diags|transitive_effect_diags|duplicate_effect_diags|duplicate_type_diags|duplicate_task_diags|duplicate_take_diags|duplicate_map_key_diags|duplicate_record_field_diags|duplicate_record_literal_field_diags|record_field_missing_diags|record_field_unknown_diags|record_field_type_mismatch_diags|record_literal_non_record_type_diags|unknown_record_field_diags|list_element_type_diags|index_diags|map_key_type_diags|map_value_type_diags|unknown_task_diags|call_arity_diags|call_argument_type_diags|type_mismatch_diags|return_type_mismatch_diags|missing_return_diags|task_diags|question_requires_result_diags):' bin/sley; then
  fail "checker descriptor-backed diagnostics must run through Sley-owned pass descriptor engines"
fi

if grep -Fq 'if ($pass.diagnostic_id_task // "") == "unknown_identifier_id"' bin/sley \
  || grep -Fq 'if ($pass.message_task // "") == "unknown_identifier_message"' bin/sley; then
  fail "checker descriptor values must use lookup maps, not host if/elif chains"
fi

if ! grep -Fq '$diagnostic_id_task_values[($pass.diagnostic_id_task // "")] // ""' bin/sley \
  || ! grep -Fq '$message_template_task_values[($pass.message_task // "")] // ""' bin/sley; then
  fail "checker descriptor value lookup maps are missing"
fi

if ! grep -Fq 'CHECKER_DIAGNOSTIC_PASS_DESCRIPTORS_JSON="$(sley_source_list_task_json loom.checker diagnostic_pass_plan' bin/sley; then
  fail "checker diagnostic pass plan source task dispatch is missing"
fi

if ! grep -Fq 'executor:$parts[2]' bin/sley \
  || ! grep -Fq 'diagnostic_id_task:$parts[3]' bin/sley \
  || ! grep -Fq 'node_scope:$parts[7]' bin/sley; then
  fail "checker diagnostic pass plan must parse Sley-owned executor names"
fi

if grep -Fq 'executor_id:($executors | index' bin/sley; then
  fail "checker executor ids must come from loom.checker diagnostic_pass_plan"
fi

if grep -Fq '($pass.executor // "") == "identifier_resolution"' bin/sley \
  || grep -Fq '($pass.executor // "") == "unknown_reference"' bin/sley; then
  fail "checker executor dispatch must use Sley-owned executor ids, not host string branches"
fi

if ! grep -Fq 'CHECKER_DIAGNOSTIC_EXECUTORS_JSON="$(sley_source_list_task_json loom.checker diagnostic_executors)"' bin/sley \
  || ! grep -Fq 'descriptor_executor_id($pass)' bin/sley; then
  fail "checker executor dispatch source map is missing"
fi

if ! grep -Fq '$diagnostic_executors[$executor_id] == ($pass.executor // "")' bin/sley; then
  fail "checker executor ids must be validated against Sley-owned executor names"
fi

if grep -Fq 'elif ($callee | contains(".")) then true' bin/sley; then
  fail "checker qualified-task call policy must come from loom.checker"
fi

if ! grep -Fq 'CHECKER_ASSUME_QUALIFIED_TASK_CALLS_KNOWN="$(sley_source_task loom.checker assume_qualified_task_calls_known)"' bin/sley; then
  fail "checker qualified-task call policy source task dispatch is missing"
fi

if grep -Fq '$diagnostic_pass_order' bin/sley; then
  fail "checker dispatch must use Sley-owned diagnostic pass descriptors, not the old order list"
fi

if bin/sley check --json "$unknown_take_type_source" >/tmp/sley-rejected-take-type-check.json; then
  fail "rejected unknown take type source passed check"
fi
jq -er '.status == "error" and .diagnostics[0].id == "UNKNOWN_TYPE" and .diagnostics[0].message == "unknown type `MissingTakeType`"' /tmp/sley-rejected-take-type-check.json >/dev/null

if bin/sley check --json fixtures/corpus/rejected/unknown_effect.sley >/tmp/sley-rejected-effect-check.json; then
  fail "rejected unknown_effect.sley passed check"
fi
jq -er '.status == "error" and .diagnostics[0].id == "UNKNOWN_EFFECT" and .diagnostics[0].message == "unknown effect `MissingEffect`"' /tmp/sley-rejected-effect-check.json >/dev/null

if bin/sley check --json fixtures/corpus/rejected/gate_take_type_mismatch.sley >/tmp/sley-rejected-gate-take-check.json; then
  fail "rejected gate_take_type_mismatch.sley passed check"
fi
jq -er '.status == "error" and .diagnostics[0].id == "GATE_TAKE_TYPE_MISMATCH" and .diagnostics[0].message == "gate take must use Gate<Effect>, got `Int`"' /tmp/sley-rejected-gate-take-check.json >/dev/null

if bin/sley check --json fixtures/corpus/rejected/gate_effect_undeclared.sley >/tmp/sley-rejected-gate-effect-check.json; then
  fail "rejected gate_effect_undeclared.sley passed check"
fi
jq -er '.status == "error" and .diagnostics[0].id == "GATE_EFFECT_UNDECLARED" and .diagnostics[0].message == "gate effect undeclared `FileRead`"' /tmp/sley-rejected-gate-effect-check.json >/dev/null

if bin/sley check --json fixtures/corpus/rejected/authority/missing_file_read_effect.sley >/tmp/sley-rejected-direct-effect-check.json; then
  fail "rejected missing_file_read_effect.sley passed check"
fi
jq -er '.status == "error" and .diagnostics[0].id == "EFFECT_UNAUTHORIZED" and .diagnostics[0].message == "effect unauthorized `FileRead`"' /tmp/sley-rejected-direct-effect-check.json >/dev/null

if bin/sley check --json fixtures/corpus/rejected/authority/missing_transitive_spend_effect.sley >/tmp/sley-rejected-transitive-effect-check.json; then
  fail "rejected missing_transitive_spend_effect.sley passed check"
fi
jq -er '.status == "error" and .diagnostics[0].id == "EFFECT_UNAUTHORIZED" and .diagnostics[0].message == "effect unauthorized `Spend`"' /tmp/sley-rejected-transitive-effect-check.json >/dev/null

if bin/sley check --json fixtures/corpus/rejected/unknown_identifier.sley >/tmp/sley-rejected-check.json; then
  fail "rejected unknown_identifier.sley passed check"
fi
jq -er '.status == "error" and .diagnostics[0].id == "UNKNOWN_IDENTIFIER" and .diagnostics[0].message == "unknown identifier `missing`"' /tmp/sley-rejected-check.json >/dev/null

self_hosting_status_field '(.bootstrap_owned_by_sley | index("checker_unknown_message_fallback_removal_task_execution"))'

if grep -Eq 'eval_checker_message_template (unknown_identifier_message identifier_name|unknown_type_message type_name|unknown_task_message task_name) \|\| printf' bin/sley; then
  fail "checker unknown diagnostic message templates must come from loom.checker without printf fallbacks"
fi

if grep -Eq 'UNKNOWN_(IDENTIFIER|TYPE|TASK)_MESSAGE_(PREFIX|SUFFIX)="\$\{UNKNOWN_(IDENTIFIER|TYPE|TASK)_MESSAGE_(PREFIX|SUFFIX):-' bin/sley; then
  fail "checker unknown diagnostic message prefixes/suffixes must come from loom.checker without shell fallbacks"
fi

if grep -Eq 'if \[\[ -z "\$UNKNOWN_(IDENTIFIER|TYPE|TASK)_MESSAGE_TEMPLATE" \]\]' bin/sley; then
  fail "checker unknown diagnostic message templates must not use empty-template shell fallback branches"
fi

self_hosting_status_field '(.bootstrap_owned_by_sley | index("checker_identifier_input_fallback_removal_task_execution"))'

bin/sley check --json examples/unused_take.sley \
  | json_field '.status == "ok" and (.diagnostics | length) == 0'

bin/sley check --json fixtures/corpus/accepted/mutable_sum.sley \
  | json_field '.status == "ok" and (.diagnostics | length) == 0'

if grep -Fq "CHECKER_IDENTIFIER_INPUTS_JSON='[\"task_names\",\"take_names\",\"binding_names\"]'" bin/sley; then
  fail "checker identifier inputs must come from loom.checker without shell fallback JSON"
fi

if grep -Fq 'if [[ "$CHECKER_IDENTIFIER_INPUTS_JSON" == "[]" ]]' bin/sley; then
  fail "checker identifier inputs must not fall back when loom.checker extraction is empty"
fi

if bin/sley check --json fixtures/corpus/rejected/unknown_type.sley >/tmp/sley-rejected-type-check.json; then
  fail "rejected unknown_type.sley passed check"
fi
jq -er '.status == "error" and .diagnostics[0].id == "UNKNOWN_TYPE" and .diagnostics[0].message == "unknown type `MissingType`"' /tmp/sley-rejected-type-check.json >/dev/null

if bin/sley check --json fixtures/corpus/rejected/unknown_task.sley >/tmp/sley-rejected-task-check.json; then
  fail "rejected unknown_task.sley passed check"
fi
jq -er '.status == "error" and .diagnostics[0].id == "UNKNOWN_TASK" and .diagnostics[0].message == "unknown task `missing`" and .diagnostics[0].repair_hints[0].kind == "declare_or_import_task" and (.diagnostics[0].repair_hints[0].replacement | contains("take arg0: Int")) and (.diagnostics[0].repair_hints[0].replacement | contains("take arg1: Text"))' /tmp/sley-rejected-task-check.json >/dev/null

bin/sley check --json "$qualified_task_call_source" \
  | json_field '.status == "ok" and (.diagnostics | length) == 0'

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

self_hosting_status_field '(.bootstrap_owned_by_sley | index("checker_call_type_message_fallback_removal_task_execution"))'

if grep -Eq 'eval_checker_message_template (call_arity_mismatch_message task_name|call_argument_type_mismatch_message task_name|type_mismatch_message binding_name|return_type_mismatch_message task_name) \|\| printf' bin/sley; then
  fail "checker call/type diagnostic message templates must come from loom.checker without printf fallbacks"
fi

if grep -Eq '(CALL_ARITY_MISMATCH|CALL_ARGUMENT_TYPE_MISMATCH|TYPE_MISMATCH|RETURN_TYPE_MISMATCH)_MESSAGE_(PREFIX|SUFFIX)="\$\{(CALL_ARITY_MISMATCH|CALL_ARGUMENT_TYPE_MISMATCH|TYPE_MISMATCH|RETURN_TYPE_MISMATCH)_MESSAGE_(PREFIX|SUFFIX):-' bin/sley; then
  fail "checker call/type diagnostic message prefixes/suffixes must come from loom.checker without shell fallbacks"
fi

if grep -Eq 'if \[\[ -z "\$(CALL_ARITY_MISMATCH|CALL_ARGUMENT_TYPE_MISMATCH|TYPE_MISMATCH|RETURN_TYPE_MISMATCH)_MESSAGE_TEMPLATE" \]\]' bin/sley; then
  fail "checker call/type diagnostic message templates must not use empty-template shell fallback branches"
fi

if bin/sley check --json fixtures/corpus/rejected/missing_return.sley >/tmp/sley-rejected-missing-return-check.json; then
  fail "rejected missing_return.sley passed check"
fi
jq -er '.status == "error" and .diagnostics[0].id == "MISSING_RETURN" and .diagnostics[0].message == "missing return in task `corpus.rejected.main`" and [.diagnostics[0].repair_hints[].kind] == ["insert_return", "replace_task_body"]' /tmp/sley-rejected-missing-return-check.json >/dev/null

self_hosting_status_field '(.bootstrap_owned_by_sley | index("checker_repair_hint_fallback_removal_task_execution"))'

if grep -Eq 'CHECKER_(DECLARE_OR_IMPORT_TASK|INSPECT_RETURN_TYPE|INSERT_RETURN|REPLACE_TASK_BODY|REPLACE_EXPRESSION)_HINT_KIND="\$\{CHECKER_[A-Z_]+:-' bin/sley; then
  fail "checker repair hint kinds must come from loom.checker without shell fallback literals"
fi

self_hosting_status_field '(.bootstrap_owned_by_sley | index("checker_builtin_type_fallback_removal_task_execution"))'

self_hosting_status_field '(.bootstrap_owned_by_sley | index("checker_builtin_types_source_list_execution"))'

bin/sley check --json examples/result_flow.sley \
  | json_field '.status == "ok" and (.diagnostics | length) == 0'

if grep -Eq 'CHECKER_(INT|TEXT|BOOL|UNIT|RESULT|ERROR|GATE|LIST|MAP)_TYPE="\$\{CHECKER_[A-Z_]+:-' bin/sley; then
  fail "checker builtin type names must come from loom.checker without shell fallback literals"
fi

if grep -Fq 'eval_checker_builtin_types_json || printf' bin/sley; then
  fail "checker builtin type list must come from loom.checker without shell fallback JSON"
fi

if ! awk '
  /^eval_checker_builtin_types_json\(\)[ \t]*\{/ {in_fn=1}
  in_fn && /sley_eval_source_task_list_json/ {source_list=1}
  in_fn && /extract_sley_list_task_json/ {extract_list=1}
  in_fn && /^[ \t]*}/ {exit}
  END {exit (source_list && !extract_list) ? 0 : 1}
' bin/sley; then
  fail "checker builtin type list must execute through the shared Sley source list evaluator"
fi

self_hosting_status_field '(.bootstrap_owned_by_sley | index("checker_source_metadata_task_execution"))'

if grep -Fq 'extract_sley_string_task "$SELF_HOSTED_SOURCE_ROOT/loom/checker.sley"' bin/sley \
  || grep -Fq 'extract_sley_list_task_json "$SELF_HOSTED_SOURCE_ROOT/loom/checker.sley"' bin/sley; then
  fail "checker metadata must execute through Sley source tasks, not raw host extractors"
fi

if ! grep -Fq 'sley_source_task loom.checker' bin/sley \
  || ! grep -Fq 'sley_source_list_task_json loom.checker' bin/sley; then
  fail "checker metadata source task dispatch is missing"
fi

if bin/sley check --json fixtures/corpus/rejected/type_alias_mismatch.sley >/tmp/sley-rejected-type-alias-mismatch-check.json; then
  fail "rejected type_alias_mismatch.sley passed check"
fi
jq -er '.status == "error" and .diagnostics[0].id == "RETURN_TYPE_MISMATCH" and .diagnostics[0].message == "return type mismatch `corpus.rejected.type_alias_mismatch.main`"' /tmp/sley-rejected-type-alias-mismatch-check.json >/dev/null

if bin/sley check --json fixtures/corpus/rejected/duplicate_take.sley >/tmp/sley-rejected-duplicate-take-check.json; then
  fail "rejected duplicate_take.sley passed check"
fi
jq -er '.status == "error" and .diagnostics[0].id == "DUPLICATE_TAKE" and .diagnostics[0].message == "duplicate take `value`"' /tmp/sley-rejected-duplicate-take-check.json >/dev/null

if bin/sley check --json fixtures/corpus/rejected/duplicate_map_key.sley >/tmp/sley-rejected-duplicate-map-key-check.json; then
  fail "rejected duplicate_map_key.sley passed check"
fi
jq -er '.status == "error" and .diagnostics[0].id == "DUPLICATE_MAP_KEY" and .diagnostics[0].message == "duplicate map key `ada`"' /tmp/sley-rejected-duplicate-map-key-check.json >/dev/null

if bin/sley check --json fixtures/corpus/rejected/module_namespace_conflict.sley >/tmp/sley-rejected-module-namespace-check.json; then
  fail "rejected module_namespace_conflict.sley passed check"
fi
jq -er '.status == "error" and [.diagnostics[].id] == ["DUPLICATE_EFFECT", "DUPLICATE_TYPE", "DUPLICATE_TASK"] and .diagnostics[0].message == "duplicate effect `Audit`" and .diagnostics[1].message == "duplicate type `User`" and .diagnostics[2].message == "duplicate task `main`"' /tmp/sley-rejected-module-namespace-check.json >/dev/null

self_hosting_status_field '(.bootstrap_owned_by_sley | index("checker_duplicate_message_fallback_removal_task_execution"))'

if grep -Eq 'eval_checker_message_template (duplicate_take_message take_name|duplicate_map_key_message key_name|duplicate_field_message field_name|duplicate_record_literal_field_message field_name|duplicate_effect_message effect_name|duplicate_type_message type_name|duplicate_task_message task_name) \|\| printf' bin/sley; then
  fail "checker duplicate diagnostic message templates must come from loom.checker without printf fallbacks"
fi

if grep -Eq 'DUPLICATE_(TAKE|MAP_KEY|FIELD|RECORD_LITERAL_FIELD|EFFECT|TYPE|TASK)_MESSAGE_(PREFIX|SUFFIX)="\$\{DUPLICATE_(TAKE|MAP_KEY|FIELD|RECORD_LITERAL_FIELD|EFFECT|TYPE|TASK)_MESSAGE_(PREFIX|SUFFIX):-' bin/sley; then
  fail "checker duplicate diagnostic message prefixes/suffixes must come from loom.checker without shell fallbacks"
fi

if grep -Eq 'if \[\[ -z "\$DUPLICATE_(TAKE|MAP_KEY|FIELD|RECORD_LITERAL_FIELD|EFFECT|TYPE|TASK)_MESSAGE_TEMPLATE" \]\]' bin/sley; then
  fail "checker duplicate diagnostic message templates must not use empty-template shell fallback branches"
fi

if bin/sley check --json fixtures/corpus/rejected/unknown_effect.sley >/tmp/sley-rejected-unknown-effect-check.json; then
  fail "rejected unknown_effect.sley passed check"
fi
jq -er '.status == "error" and .diagnostics[0].id == "UNKNOWN_EFFECT" and .diagnostics[0].message == "unknown effect `MissingEffect`"' /tmp/sley-rejected-unknown-effect-check.json >/dev/null

self_hosting_status_field '(.bootstrap_owned_by_sley | index("checker_builtin_effect_fallback_removal_task_execution"))'

bin/sley check --json fixtures/corpus/accepted/authority/shell_run.sley \
  | json_field '.status == "ok" and (.diagnostics | length) == 0'

bin/sley check --json fixtures/corpus/accepted/authority/spend_authorize.sley \
  | json_field '.status == "ok" and (.diagnostics | length) == 0'

if grep -Fq "CHECKER_BUILTIN_EFFECTS_JSON='[\"DatabaseRead\",\"DatabaseWrite\",\"DbRead\",\"DbWrite\",\"Deploy\",\"FileRead\",\"FileWrite\",\"ModelCall\",\"Network\",\"SecretRead\",\"Shell\",\"Spend\"]'" bin/sley; then
  fail "checker builtin effects must come from loom.checker without shell fallback JSON"
fi

if grep -Fq 'if [[ "$CHECKER_BUILTIN_EFFECTS_JSON" == "[]" ]]' bin/sley; then
  fail "checker builtin effects must not fall back when loom.checker extraction is empty"
fi

self_hosting_status_field '(.bootstrap_owned_by_sley | index("checker_effect_alias_fallback_removal_task_execution"))'

bin/sley check --json fixtures/corpus/accepted/authority/database_aliases.sley \
  | json_field '.status == "ok" and (.diagnostics | length) == 0'

if bin/sley check --json fixtures/corpus/rejected/authority/missing_database_alias_write_effect.sley >/tmp/sley-rejected-missing-db-alias-write-effect-check.json; then
  fail "rejected missing_database_alias_write_effect.sley passed check"
fi
jq -er '.status == "error" and .diagnostics[0].id == "EFFECT_UNAUTHORIZED" and .diagnostics[0].message == "effect unauthorized `DatabaseWrite`"' /tmp/sley-rejected-missing-db-alias-write-effect-check.json >/dev/null

if grep -Fq "CHECKER_EFFECT_ALIASES_JSON='[\"DatabaseRead|DbRead\",\"DatabaseWrite|DbWrite\"]'" bin/sley; then
  fail "checker effect aliases must come from loom.checker without shell fallback JSON"
fi

if grep -Fq 'if [[ "$CHECKER_EFFECT_ALIASES_JSON" == "[]" ]]' bin/sley; then
  fail "checker effect aliases must not fall back when loom.checker extraction is empty"
fi

self_hosting_status_field '(.bootstrap_owned_by_sley | index("checker_host_effect_needle_fallback_removal_task_execution"))'

bin/sley check --json fixtures/corpus/accepted/agent_data_authority.sley \
  | json_field '.status == "ok" and (.diagnostics | length) == 0'

if bin/sley check --json fixtures/corpus/rejected/authority/missing_transitive_data_write_effect.sley >/tmp/sley-rejected-missing-transitive-data-write-effect-check.json; then
  fail "rejected missing_transitive_data_write_effect.sley passed check"
fi
jq -er '.status == "error" and .diagnostics[0].id == "EFFECT_UNAUTHORIZED" and .diagnostics[0].message == "effect unauthorized `DatabaseWrite`"' /tmp/sley-rejected-missing-transitive-data-write-effect-check.json >/dev/null

if grep -Fq "CHECKER_HOST_EFFECT_NEEDLES_JSON='[" bin/sley; then
  fail "checker host-effect needles must come from loom.checker without shell fallback JSON"
fi

if grep -Fq 'if [[ "$CHECKER_HOST_EFFECT_NEEDLES_JSON" == "[]" ]]' bin/sley; then
  fail "checker host-effect needles must not fall back when loom.checker extraction is empty"
fi

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

self_hosting_status_field '(.bootstrap_owned_by_sley | index("checker_effect_message_fallback_removal_task_execution"))'

self_hosting_status_field '(.bootstrap_owned_by_sley | index("checker_return_message_fallback_removal_task_execution"))'

self_hosting_status_field '(.bootstrap_owned_by_sley | index("checker_question_message_fallback_removal_task_execution"))'

if grep -Eq 'eval_checker_message_template (unknown_effect_message effect_name|gate_take_type_mismatch_message actual_type|gate_effect_undeclared_message effect_name|effect_unauthorized_message effect_name) \|\| printf' bin/sley; then
  fail "checker effect diagnostic message templates must come from loom.checker without printf fallbacks"
fi

if grep -Eq '(UNKNOWN_EFFECT|GATE_TAKE_TYPE_MISMATCH|GATE_EFFECT_UNDECLARED|EFFECT_UNAUTHORIZED)_MESSAGE_(PREFIX|SUFFIX)="\$\{[A-Z_]+_MESSAGE_(PREFIX|SUFFIX):-' bin/sley; then
  fail "checker effect diagnostic message prefixes/suffixes must come from loom.checker without shell fallbacks"
fi

if grep -Eq 'if \[\[ -z "\$(UNKNOWN_EFFECT|GATE_TAKE_TYPE_MISMATCH|GATE_EFFECT_UNDECLARED|EFFECT_UNAUTHORIZED)_MESSAGE_TEMPLATE" \]\]' bin/sley; then
  fail "checker effect diagnostic message templates must not use empty-template shell fallback branches"
fi

if grep -Eq 'eval_checker_message_template (missing_return_message task_name|question_requires_result_message task_name) \|\| printf' bin/sley; then
  fail "checker return/question diagnostic message templates must come from loom.checker without printf fallbacks"
fi

if grep -Eq '(MISSING_RETURN|QUESTION_REQUIRES_RESULT)_MESSAGE_(PREFIX|SUFFIX)="\$\{[A-Z_]+_MESSAGE_(PREFIX|SUFFIX):-' bin/sley; then
  fail "checker return/question diagnostic message prefixes/suffixes must come from loom.checker without shell fallbacks"
fi

if grep -Eq 'if \[\[ -z "\$(MISSING_RETURN|QUESTION_REQUIRES_RESULT)_MESSAGE_TEMPLATE" \]\]' bin/sley; then
  fail "checker return/question diagnostic message templates must not use empty-template shell fallback branches"
fi

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

if bin/sley check --json fixtures/corpus/rejected/list_element_type_mismatch.sley >/tmp/sley-rejected-list-element-type-check.json; then
  fail "rejected list_element_type_mismatch.sley passed check"
fi
jq -er '.status == "error" and .diagnostics[0].id == "LIST_ELEMENT_TYPE_MISMATCH" and .diagnostics[0].message == "list element type mismatch `Text`"' /tmp/sley-rejected-list-element-type-check.json >/dev/null

if bin/sley check --json fixtures/corpus/rejected/list_index_non_int.sley >/tmp/sley-rejected-list-index-non-int-check.json; then
  fail "rejected list_index_non_int.sley passed check"
fi
jq -er '.status == "error" and .diagnostics[0].id == "INDEX_NOT_INT" and .diagnostics[0].message == "list index must be Int, got `Text`"' /tmp/sley-rejected-list-index-non-int-check.json >/dev/null

if bin/sley check --json fixtures/corpus/rejected/map_index_non_text.sley >/tmp/sley-rejected-map-index-non-text-check.json; then
  fail "rejected map_index_non_text.sley passed check"
fi
jq -er '.status == "error" and .diagnostics[0].id == "INDEX_KEY_TYPE_MISMATCH" and .diagnostics[0].message == "map index key must be Text, got `Int`"' /tmp/sley-rejected-map-index-non-text-check.json >/dev/null

if bin/sley check --json fixtures/corpus/rejected/map_key_type_mismatch.sley >/tmp/sley-rejected-map-key-type-check.json; then
  fail "rejected map_key_type_mismatch.sley passed check"
fi
jq -er '.status == "error" and .diagnostics[0].id == "MAP_KEY_TYPE_MISMATCH" and .diagnostics[0].message == "map key type mismatch `Int`"' /tmp/sley-rejected-map-key-type-check.json >/dev/null

if bin/sley check --json fixtures/corpus/rejected/map_value_type_mismatch.sley >/tmp/sley-rejected-map-value-type-check.json; then
  fail "rejected map_value_type_mismatch.sley passed check"
fi
jq -er '.status == "error" and .diagnostics[0].id == "MAP_VALUE_TYPE_MISMATCH" and .diagnostics[0].message == "map value type mismatch `Text`"' /tmp/sley-rejected-map-value-type-check.json >/dev/null

self_hosting_status_field '(.bootstrap_owned_by_sley | index("checker_record_message_fallback_removal_task_execution"))'

self_hosting_status_field '(.bootstrap_owned_by_sley | index("checker_collection_message_fallback_removal_task_execution"))'

if grep -Eq 'eval_checker_message_template (record_field_missing_message field_name|record_field_unknown_message field_name|record_field_type_mismatch_message field_name|unknown_record_field_message field_name|record_literal_non_record_type_message type_name) \|\| printf' bin/sley; then
  fail "checker record diagnostic message templates must come from loom.checker without printf fallbacks"
fi

if grep -Eq '(RECORD_FIELD_MISSING|RECORD_FIELD_UNKNOWN|RECORD_FIELD_TYPE_MISMATCH|UNKNOWN_RECORD_FIELD|RECORD_LITERAL_NON_RECORD_TYPE)_MESSAGE_(PREFIX|SUFFIX)="\$\{[A-Z_]+_MESSAGE_(PREFIX|SUFFIX):-' bin/sley; then
  fail "checker record diagnostic message prefixes/suffixes must come from loom.checker without shell fallbacks"
fi

if grep -Eq 'if \[\[ -z "\$(RECORD_FIELD_MISSING|RECORD_FIELD_UNKNOWN|RECORD_FIELD_TYPE_MISMATCH|UNKNOWN_RECORD_FIELD|RECORD_LITERAL_NON_RECORD_TYPE)_MESSAGE_TEMPLATE" \]\]' bin/sley; then
  fail "checker record diagnostic message templates must not use empty-template shell fallback branches"
fi

if grep -Eq 'eval_checker_message_template (list_element_type_mismatch_message actual_type|index_not_int_message actual_type|index_key_type_mismatch_message actual_type|map_key_type_mismatch_message actual_type|map_value_type_mismatch_message actual_type) \|\| printf' bin/sley; then
  fail "checker collection diagnostic message templates must come from loom.checker without printf fallbacks"
fi

if grep -Eq '(LIST_ELEMENT_TYPE_MISMATCH|INDEX_NOT_INT|INDEX_KEY_TYPE_MISMATCH|MAP_KEY_TYPE_MISMATCH|MAP_VALUE_TYPE_MISMATCH)_MESSAGE_(PREFIX|SUFFIX)="\$\{[A-Z_]+_MESSAGE_(PREFIX|SUFFIX):-' bin/sley; then
  fail "checker collection diagnostic message prefixes/suffixes must come from loom.checker without shell fallbacks"
fi

if grep -Eq 'if \[\[ -z "\$(LIST_ELEMENT_TYPE_MISMATCH|INDEX_NOT_INT|INDEX_KEY_TYPE_MISMATCH|MAP_KEY_TYPE_MISMATCH|MAP_VALUE_TYPE_MISMATCH)_MESSAGE_TEMPLATE" \]\]' bin/sley; then
  fail "checker collection diagnostic message templates must not use empty-template shell fallback branches"
fi

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

self_hosting_status_field '(.bootstrap_owned_by_sley | index("lint_empty_finding_fallback_removal_task_execution"))'

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

if ! self_hosting_status_field '(.bootstrap_owned_by_sley | index("generic_literal_len_runtime_execution"))'; then
  fail "generic literal len runtime marker is missing"
fi

if ! grep -Fq 'if [[ "$expr" =~ ^len\(\[([^\]]*)\]\)$ ]]; then' bin/sley; then
  fail "generic source runtime must evaluate literal-list len expressions directly"
fi

bin/sley run --json examples/constant_arithmetic_expression.sley \
  | json_field '.schema == "sley.run.report.v0" and .value.kind == "Int" and .value.value == 5'

bin/sley run --json examples/absorbing_arithmetic_expression.sley \
  | json_field '.schema == "sley.run.report.v0" and .value.kind == "Int" and .value.value == 0'

bin/sley run --json "$generic_top_level_multiply_source" \
  | json_field '.schema == "sley.run.report.v0" and .value.kind == "Int" and .value.value == 5'

if ! self_hosting_status_field '(.bootstrap_owned_by_sley | index("generic_parenthesized_arithmetic_runtime_execution")) and (.bootstrap_owned_by_sley | index("generic_top_level_multiply_runtime_execution"))'; then
  fail "generic parenthesized arithmetic runtime marker is missing"
fi

if ! grep -Fq 'sley_split_top_level_token "$expr" "*"' bin/sley; then
  fail "generic source runtime must evaluate parenthesized multiplication directly"
fi

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

bin/sley run --json "$generic_top_level_comparison_source" \
  | json_field '.schema == "sley.run.report.v0" and .value.kind == "Bool" and .value.value == true'

if ! self_hosting_status_field '(.bootstrap_owned_by_sley | index("generic_top_level_comparison_runtime_execution"))'; then
  fail "generic top-level comparison runtime marker is missing"
fi

if ! grep -Fq 'sley_split_top_level_comparison "$expr"' bin/sley; then
  fail "generic source runtime must evaluate top-level comparison expressions directly"
fi

bin/sley run --json "$generic_balanced_parentheses_source" \
  | json_field '.schema == "sley.run.report.v0" and .value.kind == "Bool" and .value.value == true'

if ! self_hosting_status_field '(.bootstrap_owned_by_sley | index("generic_balanced_parentheses_runtime_execution"))'; then
  fail "generic balanced-parentheses runtime marker is missing"
fi

if ! grep -Fq 'if sley_expr_wrapped_by_parens "$expr"; then' bin/sley; then
  fail "generic source runtime must strip only balanced wrapping parentheses"
fi

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

bin/sley run --json "$generic_parenthesized_bool_and_source" \
  | json_field '.schema == "sley.run.report.v0" and .value.kind == "Bool" and .value.value == true'

if ! self_hosting_status_field '(.bootstrap_owned_by_sley | index("generic_parenthesized_bool_and_runtime_execution"))'; then
  fail "generic parenthesized boolean-and runtime marker is missing"
fi

if ! grep -Fq 'sley_split_top_level_token "$expr" "&&"' bin/sley; then
  fail "generic source runtime must evaluate boolean-and expressions directly"
fi

bin/sley run --json "$generic_bool_or_source" \
  | json_field '.schema == "sley.run.report.v0" and .value.kind == "Bool" and .value.value == true'

if ! self_hosting_status_field '(.bootstrap_owned_by_sley | index("bool_or_runtime_task_execution")) and (.bootstrap_owned_by_sley | index("generic_bool_or_runtime_execution"))'; then
  fail "generic boolean-or runtime markers are missing"
fi

if ! grep -Fq 'sley_split_top_level_token "$expr" "||"' bin/sley; then
  fail "generic source runtime must evaluate boolean-or expressions directly"
fi

bin/sley run --json examples/constant_not_expression.sley \
  | json_field '.schema == "sley.run.report.v0" and .value.kind == "Bool" and .value.value == false'

bin/sley run --json examples/double_negation_expression.sley \
  | json_field '.schema == "sley.run.report.v0" and .value.kind == "Bool" and .value.value == true'

bin/sley run --json examples/negated_comparison_expression.sley \
  | json_field '.schema == "sley.run.report.v0" and .value.kind == "Bool" and .value.value == false'

if ! self_hosting_status_field '(.bootstrap_owned_by_sley | index("generic_not_expression_runtime_execution")) and (.bootstrap_owned_by_sley | index("generic_parenthesized_bool_runtime_execution"))'; then
  fail "generic runtime negation/parenthesized boolean markers are missing"
fi

if ! grep -Fq 'if [[ "$expr" =~ ^!(.+)$ ]]; then' bin/sley \
  || ! grep -Fq 'if sley_expr_wrapped_by_parens "$expr"; then' bin/sley; then
  fail "generic source runtime must evaluate negation and parenthesized expressions directly"
fi

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

self_hosting_status_field '(.bootstrap_owned_by_sley | index("file_entry_agent_pipeline_runtime_execution"))'

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
