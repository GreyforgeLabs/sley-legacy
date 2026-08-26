#!/usr/bin/env bash
set -euo pipefail

ROOT_DIR="$(cd "$(dirname "$0")/.." && pwd)"
cd "$ROOT_DIR"
export PATH="$ROOT_DIR/bin:$PATH"

mode="${1:-all}"
case "$mode" in
  parser|checker|runtime|lint|all) ;;
  *)
    echo "usage: scripts/test-core-focus.sh [parser|checker|runtime|lint|all]" >&2
    exit 2
    ;;
esac

parser_smoke() {
  local report source
  report="$(mktemp)"
  source="$(mktemp)"
  trap 'rm -f "$report" "$source"' RETURN
  bin/sley ast --json examples/hello.sley >"$report"
  jq -e '.schema == "sley.ast.program.v0" and .module == "app.hello" and (.tasks | length) == 1' "$report" >/dev/null

  printf '%s\n' \
    'module app.multiline_record' \
    '' \
    'type User = {' \
    '  slot name: Text' \
    '  slot age: Int' \
    '}' \
    '' \
    'task main -> User {' \
    '  return User {' \
    '    name: "Ada",' \
    '    age: 37' \
    '  }' \
    '}' >"$source"
  bin/sley ast --json "$source" >"$report"
  jq -e '.tasks[0].body.statements[0].expr | .expr_kind == "RecordLiteral" and .type_name == "User" and ([.fields[].name] == ["name", "age"])' "$report" >/dev/null
  echo "parser focused smoke passed"
}

checker_smoke() {
  local report source fixture expected
  report="$(mktemp)"
  source="$(mktemp)"
  trap 'rm -f "$report" "$source"' RETURN

  bin/sley check --json examples/hello.sley >"$report"
  jq -e '.schema == "sley.diagnostics.report.v0" and .status == "ok" and (.diagnostics | length) == 0' "$report" >/dev/null

  bin/sley check --json examples/constant_if_statement.sley >"$report"
  jq -e '.schema == "sley.diagnostics.report.v0" and .status == "ok" and (.diagnostics | length) == 0' "$report" >/dev/null

  bin/sley check --json examples/constant_record_field_access_expression.sley >"$report"
  jq -e '.schema == "sley.diagnostics.report.v0" and .status == "ok" and (.diagnostics | length) == 0' "$report" >/dev/null

  printf '%s\n' \
    'module app.multiline_record' \
    '' \
    'type User = {' \
    '  slot name: Text' \
    '  slot age: Int' \
    '}' \
    '' \
    'task main -> User {' \
    '  return User {' \
    '    name: "Ada",' \
    '    age: 37' \
    '  }' \
    '}' >"$source"
  bin/sley check --json "$source" >"$report"
  jq -e '.schema == "sley.diagnostics.report.v0" and .status == "ok" and (.diagnostics | length) == 0' "$report" >/dev/null

  printf '%s\n' \
    'module app.focused_checker_error' \
    '' \
    'task main -> Int {' \
    '  return missing_value' \
    '}' >"$source"
  bin/sley check --json "$source" >"$report" 2>/dev/null || true
  jq -e '.schema == "sley.diagnostics.report.v0" and .status == "error" and (.diagnostics | length) >= 1' "$report" >/dev/null

  while IFS='|' read -r fixture expected; do
    bin/sley check --json "$fixture" >"$report" 2>/dev/null || true
    jq -e --arg expected "$expected" \
      '.schema == "sley.diagnostics.report.v0" and .status == "error" and ([.diagnostics[].id] | index($expected))' \
      "$report" >/dev/null
  done <<'EOF'
fixtures/corpus/rejected/unsupported_raw_expression.sley|UNSUPPORTED_RAW_EXPRESSION
fixtures/corpus/rejected/foreign_conditional_expression.sley|CONTROL_FLOW_EXPRESSION_BOUNDARY
fixtures/corpus/rejected/module_control_flow.sley|MODULE_CONTROL_FLOW_NOT_ALLOWED
fixtures/corpus/rejected/inline_task_parameter.sley|INLINE_TASK_PARAMETER_PLACEMENT
fixtures/checker/module_control_collision|MODULE_CONTROL_FLOW_NOT_ALLOWED
EOF
  echo "checker focused smoke passed"
}

runtime_smoke() {
  local report
  report="$(mktemp)"
  bin/sley run --json examples/hello.sley >"$report"
  jq -e '.schema == "sley.run.report.v0" and .status == "passed" and .value.kind == "Text" and .value.value == "hello sley"' "$report" >/dev/null
  bin/sley run --json examples/compute.sley >"$report"
  jq -e '.schema == "sley.run.report.v0" and .status == "passed" and .value.value == "excellent"' "$report" >/dev/null
  rm "$report"
  echo "runtime focused smoke passed"
}

lint_smoke() {
  local report
  report="$(mktemp)"
  bin/sley lint --json examples/empty_for_statement.sley >"$report"
  jq -e '.schema == "sley.lint.report.v0" and .status == "findings" and ([.findings[].id] | index("EMPTY_FOR_STATEMENT"))' "$report" >/dev/null
  rm "$report"
  echo "lint focused smoke passed"
}

if [[ "$mode" == "parser" || "$mode" == "all" ]]; then parser_smoke; fi
if [[ "$mode" == "checker" || "$mode" == "all" ]]; then checker_smoke; fi
if [[ "$mode" == "runtime" || "$mode" == "all" ]]; then runtime_smoke; fi
if [[ "$mode" == "lint" || "$mode" == "all" ]]; then lint_smoke; fi
