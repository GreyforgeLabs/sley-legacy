#!/usr/bin/env bash
set -euo pipefail

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
SLEY="$ROOT_DIR/bin/sley"
BASE="$ROOT_DIR/fixtures/contracts/adapter_manifest_local_replay.json"
PINNED_RECORD="$ROOT_DIR/fixtures/contracts/adapter_replay_agent_pipeline.json"
ZERO_DIGEST="sha256:0000000000000000000000000000000000000000000000000000000000000000"
WORKDIR="$(mktemp -d "$ROOT_DIR/.sley-adapter-test.XXXXXX")"
OUTSIDE_SOURCE="$(mktemp /tmp/sley-adapter-outside.XXXXXX.sley)"
trap 'rm -rf "$WORKDIR"; rm -f "$OUTSIDE_SOURCE"' EXIT

fail() {
  echo "adapter replay test failed: $*" >&2
  exit 1
}

manifest_case() {
  local name="$1" filter="$2" target="${3:-$ROOT_DIR/examples/agent_deploy_pipeline.sley}" stage output manifest_digest
  stage="$WORKDIR/$name.stage.json"
  output="$WORKDIR/$name.json"
  jq --arg target "$target" --arg zero "$ZERO_DIGEST" \
    ".target = \$target | $filter | .manifest_digest = \$zero" "$BASE" > "$stage"
  manifest_digest="$(python3 - "$stage" <<'PY'
import hashlib
import json
import sys

with open(sys.argv[1], encoding="utf-8") as handle:
    manifest = json.load(handle)
payload = json.dumps(manifest, sort_keys=True, separators=(",", ":"), ensure_ascii=False).encode("utf-8")
print("sha256:" + hashlib.sha256(payload).hexdigest())
PY
)"
  jq --arg digest "$manifest_digest" '.manifest_digest = $digest' "$stage" > "$output"
  printf '%s\n' "$output"
}

expect_status_code() {
  local manifest="$1" expected_status="$2" expected_code="$3" report="$4"
  if "$SLEY" adapter replay --json "$manifest" > "$report" 2>/dev/null; then
    [[ "$expected_status" == "passed" ]] || fail "expected $expected_status for $manifest"
  else
    [[ "$expected_status" != "passed" ]] || fail "expected pass for $manifest"
  fi
  jq -e --arg status "$expected_status" --arg code "$expected_code" '
    .schema == "sley.adapter.report.v0" and
    .status == $status and
    (if $code == "" then (.issues | length) == 0 else .issues[0].code == $code end) and
    .isolation.os_isolation_enforced == false and
    .isolation.owner == "S12-502"
  ' "$report" >/dev/null || fail "unexpected report for $manifest"
}

source_before="$(sha256sum "$ROOT_DIR/examples/agent_deploy_pipeline.sley" | awk '{print $1}')"
index_before="$(git -C "$ROOT_DIR" write-tree)"

happy_report="$WORKDIR/happy-report.json"
expect_status_code "$BASE" passed "" "$happy_report"
jq -e '
  .replay.matched == true and
  .record.status == "passed" and
  .record.terminal_code == "ADAPTER_REPLAY_VERIFIED" and
  .record.redaction.seed_families[2].family == "secrets" and
  .record.redaction.seed_families[2].values_included == false and
  ([.record.events[].phase] == [
    "opened", "manifest_validated", "authority_bound", "seed_bound",
    "runtime_started", "adapter_called", "adapter_returned",
    "runtime_finished", "replay_verified"
  ])
' "$happy_report" >/dev/null || fail "happy replay did not preserve the deterministic lifecycle"
if rg -q 'redacted|profile ready|plan approved|staged' "$happy_report"; then
  fail "replay report leaked a seeded value"
fi
if rg -q 'str\(seed\["text"\]\)' "$ROOT_DIR/scripts/sley-adapter-replay.sh"; then
  fail "adapter host passes raw seeded text to a child process argument"
fi

record_report="$WORKDIR/record-report.json"
"$SLEY" adapter replay --json --record "$PINNED_RECORD" "$BASE" > "$record_report"
jq -e '.status == "passed" and .replay.matched == true' "$record_report" >/dev/null \
  || fail "pinned replay record did not match"

missing_capability="$(manifest_case missing-capability 'del(.capabilities[] | select(.effect == "Deploy"))')"
expect_status_code "$missing_capability" blocked ADAPTER_AUTHORITY_DENIED "$WORKDIR/missing-capability-report.json"

scope_denial="$(manifest_case scope-denial '(.capabilities[] | select(.effect == "Deploy") | .scope) = "production"')"
expect_status_code "$scope_denial" blocked ADAPTER_SCOPE_DENIED "$WORKDIR/scope-denial-report.json"

missing_seed="$(manifest_case missing-seed '.seeds.deploy_results = []')"
expect_status_code "$missing_seed" blocked ADAPTER_SEED_NOT_FOUND "$WORKDIR/missing-seed-report.json"

replay_mismatch="$(manifest_case replay-mismatch '.replay.record_digest = "sha256:ffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffff"')"
expect_status_code "$replay_mismatch" failed ADAPTER_REPLAY_MISMATCH "$WORKDIR/replay-mismatch-report.json"

cancellation="$(manifest_case cancellation '.cancellation.requested_at_phase = "before_adapter_call"')"
expect_status_code "$cancellation" cancelled ADAPTER_CANCELLED "$WORKDIR/cancellation-report.json"
jq -e '.record.events[-1].phase == "cancellation_observed" and .record.events[-1].code == "ADAPTER_CANCELLED"' \
  "$WORKDIR/cancellation-report.json" >/dev/null || fail "cancellation was not observed at the declared boundary"

budget="$(manifest_case budget '.bounds.max_input_bytes = 1')"
expect_status_code "$budget" blocked ADAPTER_BUDGET_EXCEEDED "$WORKDIR/budget-report.json"

live_provider="$(manifest_case live-provider '.adapter.live_execution = true')"
expect_status_code "$live_provider" blocked ADAPTER_LIVE_PROVIDER_DENIED "$WORKDIR/live-provider-report.json"

printf 'task main -> Text { return "outside" }\n' > "$OUTSIDE_SOURCE"
mkdir "$WORKDIR/symlink-target"
ln -s "$OUTSIDE_SOURCE" "$WORKDIR/symlink-target/outside.sley"
nested_symlink="$(manifest_case nested-symlink '.' "$WORKDIR/symlink-target")"
expect_status_code "$nested_symlink" blocked ADAPTER_SCOPE_DENIED "$WORKDIR/nested-symlink-report.json"
jq -e '.issues[0].message | contains("contains a symlink")' "$WORKDIR/nested-symlink-report.json" >/dev/null \
  || fail "nested target symlink did not fail at the confinement boundary"

tampered_manifest="$WORKDIR/tampered-manifest.json"
jq '.replay.replay_key = "replay:tampered"' "$BASE" > "$tampered_manifest"
expect_status_code "$tampered_manifest" failed ADAPTER_MANIFEST_INVALID "$WORKDIR/tampered-manifest-report.json"

tampered_record="$WORKDIR/tampered-record.json"
jq '.terminal_code = "ADAPTER_REPLAY_TAMPERED"' "$PINNED_RECORD" > "$tampered_record"
if "$SLEY" adapter replay --json --record "$tampered_record" "$BASE" > "$WORKDIR/tampered-record-report.json" 2>/dev/null; then
  fail "tampered replay record unexpectedly passed"
fi
jq -e '.status == "failed" and .issues[0].code == "ADAPTER_REPLAY_MISMATCH"' \
  "$WORKDIR/tampered-record-report.json" >/dev/null || fail "tampered replay record was not rejected"

"$ROOT_DIR/bin/sley-contract" validate --schema sley.adapter.report.v0 "$happy_report" --schemas "$ROOT_DIR/docs/schemas" --json \
  | jq -e '.status == "passed"' >/dev/null || fail "happy adapter report did not validate"
"$ROOT_DIR/bin/sley-contract" validate --schema sley.adapter.replay.v0 "$PINNED_RECORD" --schemas "$ROOT_DIR/docs/schemas" --json \
  | jq -e '.status == "passed"' >/dev/null || fail "pinned replay record did not validate"

source_after="$(sha256sum "$ROOT_DIR/examples/agent_deploy_pipeline.sley" | awk '{print $1}')"
index_after="$(git -C "$ROOT_DIR" write-tree)"
[[ "$source_before" == "$source_after" ]] || fail "adapter replay mutated the target source"
[[ "$index_before" == "$index_after" ]] || fail "adapter replay mutated the Git index"

echo "local adapter manifest, authority denial, redaction, budget, cancellation, and deterministic replay contracts passed"
