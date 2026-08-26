#!/usr/bin/env bash
set -euo pipefail

ROOT_DIR="$(cd "$(dirname "$0")/.." && pwd)"
CONTRACT="$ROOT_DIR/bin/sley-contract"
SCHEMA_DIR="$ROOT_DIR/docs/schemas"
REGISTRY="$ROOT_DIR/docs/v1.2/CONTRACT_STABILITY.json"
WORK_DIR="$(mktemp -d)"
trap 'rm -rf "$WORK_DIR"' EXIT

fail() {
  echo "contract compatibility failed: $*" >&2
  exit 1
}

validate() {
  "$CONTRACT" validate --schema "$1" "$2" --schemas "$SCHEMA_DIR" --json >/dev/null
}

expect_failure() {
  if validate "$1" "$2" 2>/dev/null; then
    fail "$2 unexpectedly validated as $1"
  fi
}

validate sley.contract.stability_registry.v1 "$REGISTRY"
"$CONTRACT" inventory --schema-dir "$SCHEMA_DIR" --json >"$WORK_DIR/inventory.json"

jq -e '([.entries[].id] | length) == ([.entries[].id] | unique | length)' "$REGISTRY" >/dev/null \
  || fail "registry contains duplicate schema IDs"

while IFS= read -r id; do
  jq -e --arg id "$id" '.schemas | any(.id == $id)' "$WORK_DIR/inventory.json" >/dev/null \
    || fail "registered schema does not exist: $id"
done < <(jq -r '.entries[].id' "$REGISTRY")

while IFS= read -r dependency; do
  jq -e --arg id "$dependency" '.schemas | any(.id == $id)' "$WORK_DIR/inventory.json" >/dev/null \
    || fail "registered dependency does not exist: $dependency"
done < <(jq -r '.entries[].dependencies[]?' "$REGISTRY")

for id in \
  sley.contract.stability_registry.v1 \
  sley.diagnostics.report.v1 \
  sley.symbol_graph.slice.v1 \
  sley.verify.report.v1 \
  sley.machine.response.v1; do
  jq -e --arg id "$id" '.entries | any(.id == $id and .class == "stable")' "$REGISTRY" >/dev/null \
    || fail "required stable root is not stable: $id"
done

while IFS= read -r prefix; do
  stem="${prefix%\*}"
  if jq -e --arg stem "$stem" '.schemas | any(.id | startswith($stem))' "$WORK_DIR/inventory.json" >/dev/null; then
    fail "reserved namespace already has a registered schema: $prefix"
  fi
done < <(jq -r '.reserved_namespaces[].prefix' "$REGISTRY")

compatibility_pair() {
  local v0_schema="$1"
  local v1_schema="$2"
  local v0_fixture="$3"
  local v1_fixture="$4"
  local required_field="$5"
  local name="${v1_schema//./_}"

  jq --arg schema "$v1_schema" '.schema = $schema' "$v0_fixture" >"$WORK_DIR/$name-from-v0.json"
  validate "$v1_schema" "$WORK_DIR/$name-from-v0.json"

  jq --arg schema "$v0_schema" '.schema = $schema' "$v1_fixture" >"$WORK_DIR/$name-to-v0.json"
  validate "$v0_schema" "$WORK_DIR/$name-to-v0.json"

  expect_failure "$v1_schema" "$v0_fixture"
  expect_failure "$v0_schema" "$v1_fixture"

  jq --arg field "$required_field" 'del(.[$field])' "$WORK_DIR/$name-from-v0.json" >"$WORK_DIR/$name-missing-required.json"
  expect_failure "$v1_schema" "$WORK_DIR/$name-missing-required.json"

  jq '.unexpected_w3_field = true' "$WORK_DIR/$name-from-v0.json" >"$WORK_DIR/$name-unknown-field.json"
  expect_failure "$v1_schema" "$WORK_DIR/$name-unknown-field.json"
}

compatibility_pair \
  sley.diagnostics.report.v0 sley.diagnostics.report.v1 \
  "$ROOT_DIR/fixtures/contracts/diagnostic_report_unknown_identifier.json" \
  "$ROOT_DIR/fixtures/contracts/diagnostic_report_v1_unknown_identifier.json" \
  status
compatibility_pair \
  sley.symbol_graph.slice.v0 sley.symbol_graph.slice.v1 \
  "$ROOT_DIR/fixtures/contracts/graph_slice_minimal_task.json" \
  "$ROOT_DIR/fixtures/contracts/graph_slice_v1_minimal_task.json" \
  target
compatibility_pair \
  sley.verify.report.v0 sley.verify.report.v1 \
  "$ROOT_DIR/fixtures/contracts/verify_project_ready.json" \
  "$ROOT_DIR/fixtures/contracts/verify_v1_project_ready.json" \
  status
compatibility_pair \
  sley.machine.response.v0 sley.machine.response.v1 \
  "$ROOT_DIR/fixtures/contracts/machine_response_ok.json" \
  "$ROOT_DIR/fixtures/contracts/machine_response_v1_ok.json" \
  status

echo "contract stability and v0/v1 compatibility passed"
