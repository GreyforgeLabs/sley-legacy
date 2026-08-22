#!/usr/bin/env bash
set -euo pipefail

ROOT_DIR="$(cd "$(dirname "$0")/.." && pwd)"
SLEY="$ROOT_DIR/bin/sley"
WORK_DIR="$(mktemp -d "$ROOT_DIR/.sley-graph-diff-test.XXXXXXXX")"
trap 'rm -rf -- "$WORK_DIR"' EXIT

fail() {
  printf 'Sley graph-diff test failed: %s\n' "$*" >&2
  exit 1
}

write_case() {
  local path="$1" body="$2"
  printf '%s\n' "$body" > "$path"
}

"$SLEY" graph-diff --json \
  --base "$ROOT_DIR/examples/hello.sley" \
  --ours "$ROOT_DIR/examples/hello.sley" \
  --theirs "$ROOT_DIR/examples/hello.sley" > "$WORK_DIR/noop.json"
jq -e '
  .schema == "sley.graph_diff.report.v0" and
  .status == "passed" and
  .mode == "report_only" and
  .merge_permitted == false and
  .summary == {node_change_count:0,call_change_count:0,conflict_count:0,unsupported_count:0} and
  .changes == [] and .conflicts == [] and
  ([.projection_checks[].status] | all(. == "passed"))
' "$WORK_DIR/noop.json" >/dev/null || fail "no-op report mismatch"

write_case "$WORK_DIR/base.sley" $'module app.merge\n\ntask main -> Int {\n  return 1\n}'
write_case "$WORK_DIR/ours.sley" $'module app.merge\n\ntask main -> Int {\n  return 2\n}'
write_case "$WORK_DIR/theirs.sley" $'module app.merge\n\ntask main -> Int {\n  return 3\n}'
if "$SLEY" graph-diff --json --base "$WORK_DIR/base.sley" --ours "$WORK_DIR/ours.sley" --theirs "$WORK_DIR/theirs.sley" > "$WORK_DIR/same-node.json"; then
  fail "same-node conflict returned success"
fi
jq -e '
  .status == "conflicted" and .merge_permitted == false and
  .summary.conflict_count == 1 and .conflicts[0].class == "same_node_edit" and
  .conflicts[0].identity == "task:app.merge.main" and
  ([.changes[].ordering_key] == ([.changes[].ordering_key] | sort))
' "$WORK_DIR/same-node.json" >/dev/null || fail "same-node conflict mismatch"

write_case "$WORK_DIR/compact-base.sley" $'module app.compact\n\ntask main -> Int { return 1 }'
write_case "$WORK_DIR/compact-ours.sley" $'module app.compact\n\ntask main -> Int { return 2 }'
write_case "$WORK_DIR/compact-theirs.sley" $'module app.compact\n\ntask main -> Int { return 3 }'
if "$SLEY" graph-diff --json --base "$WORK_DIR/compact-base.sley" --ours "$WORK_DIR/compact-ours.sley" --theirs "$WORK_DIR/compact-theirs.sley" > "$WORK_DIR/compact.json"; then
  fail "unrepresented compact-body edits returned success"
fi
jq -e '.status == "conflicted" and ([.conflicts[].class] | index("projection_drift")) != null' "$WORK_DIR/compact.json" >/dev/null || fail "compact-body projection drift mismatch"

write_case "$WORK_DIR/rename-base.sley" $'module app.rename\n\ntask old_name -> Int {\n  return 1\n}'
write_case "$WORK_DIR/rename-ours.sley" $'module app.rename\n\ntask ours_name -> Int {\n  return 1\n}'
write_case "$WORK_DIR/rename-theirs.sley" $'module app.rename\n\ntask theirs_name -> Int {\n  return 1\n}'
if "$SLEY" graph-diff --json --base "$WORK_DIR/rename-base.sley" --ours "$WORK_DIR/rename-ours.sley" --theirs "$WORK_DIR/rename-theirs.sley" > "$WORK_DIR/rename.json"; then
  fail "ambiguous rename replacements returned success"
fi
jq -e '.status == "conflicted" and ([.conflicts[].class] | index("rename_delete")) != null' "$WORK_DIR/rename.json" >/dev/null || fail "rename/delete conflict mismatch"

write_case "$WORK_DIR/effect-base.sley" $'module app.effects\n\ntask main -> Int uses FileRead {\n  return 1\n}'
write_case "$WORK_DIR/effect-ours.sley" $'module app.effects\n\ntask main -> Int uses FileRead, Network {\n  return 1\n}'
write_case "$WORK_DIR/effect-theirs.sley" $'module app.effects\n\ntask main -> Int uses FileRead, Shell {\n  return 1\n}'
if "$SLEY" graph-diff --json --base "$WORK_DIR/effect-base.sley" --ours "$WORK_DIR/effect-ours.sley" --theirs "$WORK_DIR/effect-theirs.sley" > "$WORK_DIR/effect.json"; then
  fail "effect conflict returned success"
fi
jq -e '.status == "conflicted" and ([.conflicts[].class] | index("effect_change")) != null' "$WORK_DIR/effect.json" >/dev/null || fail "effect conflict mismatch"

write_case "$WORK_DIR/gate-base.sley" $'module app.gates\n\ntask main -> Int uses FileRead {\n  take gate access: Gate<FileRead>\n  return 1\n}'
write_case "$WORK_DIR/gate-ours.sley" $'module app.gates\n\ntask main -> Int uses FileRead {\n  take gate access: Gate<Network>\n  return 1\n}'
write_case "$WORK_DIR/gate-theirs.sley" $'module app.gates\n\ntask main -> Int uses FileRead {\n  take gate access: Gate<Shell>\n  return 1\n}'
if "$SLEY" graph-diff --json --base "$WORK_DIR/gate-base.sley" --ours "$WORK_DIR/gate-ours.sley" --theirs "$WORK_DIR/gate-theirs.sley" > "$WORK_DIR/gate.json"; then
  fail "gate conflict returned success"
fi
jq -e '.status == "conflicted" and ([.conflicts[].class] | index("gate_change")) != null' "$WORK_DIR/gate.json" >/dev/null || fail "gate conflict mismatch"

write_case "$WORK_DIR/call-base.sley" $'module app.calls\n\ntask first -> Int { return 1 }\ntask second -> Int { return 2 }\ntask third -> Int { return 3 }\ntask main -> Int {\n  return call first()\n}'
write_case "$WORK_DIR/call-ours.sley" $'module app.calls\n\ntask first -> Int { return 1 }\ntask second -> Int { return 2 }\ntask third -> Int { return 3 }\ntask main -> Int {\n  return call second()\n}'
write_case "$WORK_DIR/call-theirs.sley" $'module app.calls\n\ntask first -> Int { return 1 }\ntask second -> Int { return 2 }\ntask third -> Int { return 3 }\ntask main -> Int {\n  return call third()\n}'
if "$SLEY" graph-diff --json --base "$WORK_DIR/call-base.sley" --ours "$WORK_DIR/call-ours.sley" --theirs "$WORK_DIR/call-theirs.sley" > "$WORK_DIR/call.json"; then
  fail "call-site conflict returned success"
fi
jq -e '.status == "conflicted" and ([.conflicts[].class] | index("call_site_rewrite")) != null' "$WORK_DIR/call.json" >/dev/null || fail "call-site conflict mismatch"

for report in "$WORK_DIR"/{noop,same-node,compact,rename,effect,gate,call}.json; do
  "$ROOT_DIR/bin/sley-contract" validate --schema sley.graph_diff.report.v0 "$report" --schemas "$ROOT_DIR/docs/schemas" --json \
    | jq -e '.status == "passed"' >/dev/null || fail "schema validation failed for $(basename "$report")"
done

printf 'Sley graph-diff tests passed.\n'
