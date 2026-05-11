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
  | json_field '.schema == "sley.self_hosting.status.v0" and .status == "bootstrap" and .strict_self_hosted == false and .source_root == "self-hosted/src" and .semantic_source_count >= 6 and (.source_modules | index("loom.bootstrap")) and (.bootstrap_owned_by_sley | index("implementation_version")) and (.bootstrap_owned_by_sley | index("implementation_stage")) and (.bootstrap_owned_by_sley | index("self_hosting_status")) and (.bootstrap_owned_by_sley | index("strict_self_hosted")) and (.bootstrap_owned_by_sley | index("source_modules")) and (.bootstrap_owned_by_sley | index("default_lint_rules")) and (.bootstrap_owned_by_sley | index("core_report_schema_ids")) and (.bootstrap_owned_by_sley | index("diagnostic_ids")) and (.bootstrap_owned_by_sley | index("runtime_seed_values")) and (.bootstrap_owned_by_sley | index("parser_expression_classifiers")) and (.bootstrap_owned_by_sley | index("parser_statement_and_binding_kinds")) and (.bootstrap_owned_by_sley | index("bootstrap_smoke_runtime")) and (.bootstrap_owned_by_sley | index("bootstrap_smoke_task_execution")) and (.bootstrap_owned_by_sley | index("checker_diagnostic_status")) and (.bootstrap_owned_by_sley | index("checker_status_task_execution")) and (.bootstrap_owned_by_sley | index("checker_builtin_type_task_execution")) and (.bootstrap_owned_by_sley | index("checker_unknown_type_diagnostics")) and (.bootstrap_owned_by_sley | index("checker_unknown_task_diagnostics")) and (.bootstrap_owned_by_sley | index("checker_identifier_resolution_inputs")) and (.bootstrap_owned_by_sley | index("lint_status_task_execution")) and (.bootstrap_owned_by_sley | index("lint_finding_messages")) and (.bootstrap_owned_by_sley | index("runtime_status_task_execution")) and (.bootstrap_owned_by_sley | index("pure_literal_runtime_execution")) and (.bootstrap_owned_by_sley | index("bool_literal_runtime_execution")) and (.bootstrap_owned_by_sley | index("seeded_agent_deploy_task_execution")) and (.bootstrap_owned_by_sley | index("project_ready_task_execution")) and (.bootstrap_owned_by_sley | index("ast_runtime_probe_dispatch")) and (.bootstrap_owned_by_sley | index("self_hosting_report_shape")) and (.bootstrap_owned_by_sley | index("runtime_report_status_and_dispatch"))'

bin/sley self-hosting-status --json \
  | json_field '(.bootstrap_owned_by_sley | index("run_report_shape"))'

bin/sley self-hosting-status --json \
  | json_field '(.bootstrap_owned_by_sley | index("diagnostics_report_shape"))'

bin/sley self-hosting-status --json \
  | json_field '(.bootstrap_owned_by_sley | index("lint_report_shape"))'

bin/sley self-hosting-status --json \
  | json_field '(.bootstrap_owned_by_sley | index("query_report_shape"))'

bin/sley self-hosting-status --json \
  | json_field '(.bootstrap_owned_by_sley | index("doctor_report_shape"))'

bin/sley ast --json examples/hello.sley \
  | json_field '.schema == "sley.ast.program.v0" and .module == "app.hello" and .tasks[0].name == "main" and .tasks[0].body.statements[0].expr.expr_kind == "StringLiteral"'

bin/sley ast --json --node block:task:app.hello.main:stmt:0:expr examples/hello.sley \
  | json_field '.schema == "sley.ast.node.v0" and .node_kind == "expression"'

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

bin/sley query --json --kind calls examples/project \
  | json_field '.schema == "sley.query.report.v0" and .calls[0].target == "app.math.double"'

bin/sley query --json --kind calls examples/project \
  | json_field 'keys == (["schema","kind","entry_module","filters","modules","tasks","types","effects","calls"] | sort)'

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

bin/sley-contract inventory --json \
  | json_field '.schema == "sley.contract.inventory.v0" and .status == "passed" and .schema_count > 0'

bin/sley-contract check-fixtures fixtures/contracts --schemas docs/schemas --json \
  | json_field '.schema == "sley.contract.fixture_check.v0" and .status == "passed" and .failed_count == 0'

bin/sley-conformance report --json \
  | json_field '.schema == "sley.conformance.report.v0" and .status == "passed" and .summary.test_count_matches_declared == true'

echo "Self-hosting bootstrap tests passed."
