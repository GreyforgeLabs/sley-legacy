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

bash -n bin/sley
./scripts/check-self-hosted-code.sh

bin/sley --help >/dev/null
bin/sley --version | grep -q 'self-hosting-stage2-source'

bin/sley self-hosting-status --json \
  | json_field '.schema == "sley.self_hosting.status.v0" and .status == "bootstrap" and .strict_self_hosted == false and .semantic_source_count >= 6 and (.bootstrap_owned_by_sley | index("implementation_version")) and (.bootstrap_owned_by_sley | index("implementation_stage")) and (.bootstrap_owned_by_sley | index("self_hosting_status")) and (.bootstrap_owned_by_sley | index("strict_self_hosted")) and (.bootstrap_owned_by_sley | index("default_lint_rules")) and (.bootstrap_owned_by_sley | index("core_report_schema_ids")) and (.bootstrap_owned_by_sley | index("diagnostic_ids")) and (.bootstrap_owned_by_sley | index("runtime_seed_values")) and (.bootstrap_owned_by_sley | index("parser_expression_classifiers")) and (.bootstrap_owned_by_sley | index("parser_statement_and_binding_kinds")) and (.bootstrap_owned_by_sley | index("bootstrap_smoke_runtime")) and (.bootstrap_owned_by_sley | index("checker_diagnostic_status")) and (.bootstrap_owned_by_sley | index("lint_finding_messages")) and (.bootstrap_owned_by_sley | index("runtime_report_status_and_dispatch"))'

bin/sley ast --json examples/hello.sley \
  | json_field '.schema == "sley.ast.program.v0" and .module == "app.hello" and .tasks[0].name == "main" and .tasks[0].body.statements[0].expr.expr_kind == "StringLiteral"'

bin/sley ast --json --node block:task:app.hello.main:stmt:0:expr examples/hello.sley \
  | json_field '.schema == "sley.ast.node.v0" and .node_kind == "expression"'

bin/sley check --json examples/hello.sley \
  | json_field '.schema == "sley.diagnostics.report.v0" and .status == "ok"'

if bin/sley check --json fixtures/corpus/rejected/unknown_identifier.sley >/tmp/sley-rejected-check.json; then
  fail "rejected unknown_identifier.sley passed check"
fi
jq -er '.status == "error" and .diagnostics[0].id == "UNKNOWN_IDENTIFIER"' /tmp/sley-rejected-check.json >/dev/null

bin/sley query --json --kind calls examples/project \
  | json_field '.schema == "sley.query.report.v0" and .calls[0].target == "app.math.double"'

bin/sley doctor --json examples/project \
  | json_field '.schema == "sley.doctor.report.v0" and .status == "ready" and .summary.task_count == 2'

bin/sley lint --json examples/empty_for_statement.sley \
  | json_field '.schema == "sley.lint.report.v0" and .status == "findings" and .findings[0].id == "EMPTY_FOR_STATEMENT" and (.filters.rules | index("empty_for_statement"))'

bin/sley run --json examples/hello.sley \
  | json_field '.schema == "sley.run.report.v0" and .value.kind == "Text" and .value.value == "hello sley"'

bin/sley run --json self-hosted \
  | json_field '.schema == "sley.run.report.v0" and .value.kind == "Int" and .value.value == 51'

bin/sley run --json --cap SecretRead --secret api_key redacted --cap Network --http-text https://example.test/profile "profile ready" --cap ModelCall --model-output deploy-plan "plan approved" --cap Deploy --deploy-result staging staged fixtures/corpus/accepted/agent_deploy_pipeline.sley \
  | json_field '.schema == "sley.run.report.v0" and .value.kind == "Ok" and .value.value.value == "profile ready | plan approved | staged"'

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
