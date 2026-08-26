#!/usr/bin/env bash
set -euo pipefail

ROOT_DIR="$(cd "$(dirname "$0")/.." && pwd)"
SCHEMA_DIR="$ROOT_DIR/docs/schemas"
WORK_DIR="$(mktemp -d)"
trap 'rm -rf "$WORK_DIR"' EXIT
mkdir -p "$WORK_DIR/bin"

fail() {
  echo "Sley validation test failed: $*" >&2
  exit 1
}

cat >"$WORK_DIR/bin/make" <<'EOF'
#!/usr/bin/env bash
set -euo pipefail
printf '%s\n' "$*" >>"$SLEY_FAKE_MAKE_LOG"
if [[ "${SLEY_FAKE_MAKE_BYTES:-0}" -gt 0 ]]; then
  head -c "$SLEY_FAKE_MAKE_BYTES" </dev/zero
fi
if [[ "${SLEY_FAKE_MAKE_FAIL:-0}" == "1" ]]; then
  echo "synthetic validation failure" >&2
  exit 17
fi
EOF
chmod +x "$WORK_DIR/bin/make"
export SLEY_FAKE_MAKE_LOG="$WORK_DIR/make.log"

run_validate() {
  PATH="$WORK_DIR/bin:$PATH" "$ROOT_DIR/bin/sley" validate --json "$@"
}

changed_report="$WORK_DIR/changed.json"
run_validate --changed --changed-file self-hosted/src/loom/parser.sley --explain . >"$changed_report"
"$ROOT_DIR/bin/sley-contract" validate --schema sley.validation.report.v1 "$changed_report" --schemas "$SCHEMA_DIR" --json \
  | jq -e '.status == "passed"' >/dev/null
jq -e '
  .status == "passed" and
  .profile == "changed" and
  .detected_changes == ["self-hosted/src/loom/parser.sley"] and
  (.selected_checks | index("parser-smoke")) != null and
  (.selected_checks | index("arena")) == null and
  (.skipped_checks[] | select(.id == "arena" and .reason == "unchanged_subsystem")) and
  (.selection_reasons[] | select(.id == "parser-smoke" and .reason == "selected_by_changed_path_classifier")) and
  .release_gate.executed == false and
  .cache.validation_result_used == false and
  .steps[0].evidence_embedded == false
' "$changed_report" >/dev/null || fail "changed report lost selection, skip, cache, or evidence truth"
if grep -Eq '(^| )arena($| )' "$SLEY_FAKE_MAKE_LOG"; then
  fail "narrow changed validation invoked Arena"
fi

for profile in quick core release; do
  : >"$SLEY_FAKE_MAKE_LOG"
  report="$WORK_DIR/$profile.json"
  run_validate --profile "$profile" . >"$report"
  "$ROOT_DIR/bin/sley-contract" validate --schema sley.validation.report.v1 "$report" --schemas "$SCHEMA_DIR" --json \
    | jq -e '.status == "passed"' >/dev/null
  jq -e --arg profile "$profile" '.profile == $profile and .status == "passed"' "$report" >/dev/null \
    || fail "$profile report did not pass"
done
jq -e '.release_gate.required and .release_gate.executed and .steps[0].command[-1] == "v1"' "$WORK_DIR/release.json" >/dev/null \
  || fail "release profile did not preserve make v1 authority"

make_profile_checks() {
  local target="$1"
  awk -v target="$target" '$1 == target ":" {for (i=2; i<=NF; i++) print $i}' "$ROOT_DIR/Makefile" | jq -Rsc 'split("\n")[:-1]'
}
for profile in quick core; do
  expected="$(make_profile_checks "$profile")"
  jq -e --argjson expected "$expected" '.selected_checks == $expected' "$WORK_DIR/$profile.json" >/dev/null \
    || fail "$profile Sley registry drifted from Make prerequisites"
done
expected_release="$(make_profile_checks v1)"
jq -e --argjson expected "$expected_release" '.selected_checks == $expected' "$WORK_DIR/release.json" >/dev/null \
  || fail "release Sley registry drifted from Make v1 prerequisites"

set +e
SLEY_FAKE_MAKE_FAIL=1 SLEY_FAKE_MAKE_BYTES=9000000 run_validate --profile quick . >"$WORK_DIR/failed.json"
failure_rc=$?
set -e
[[ "$failure_rc" -eq 17 ]] || fail "failed validation returned $failure_rc instead of executor status 17"
"$ROOT_DIR/bin/sley-contract" validate --schema sley.validation.report.v1 "$WORK_DIR/failed.json" --schemas "$SCHEMA_DIR" --json \
  | jq -e '.status == "passed"' >/dev/null
jq -e '
  .status == "failed" and .outcome == "failed" and
  .steps[0].exit_code == 17 and .issues[0].code == "SLEY_VALIDATION_CHECK_FAILED" and
  .steps[0].evidence_bytes == 8388608 and
  .steps[0].evidence_total_bytes > .steps[0].evidence_bytes and
  .steps[0].evidence_truncated == true
' "$WORK_DIR/failed.json" >/dev/null || fail "failure report was not auditable"

if run_validate --profile quick --changed . >"$WORK_DIR/invalid.out" 2>"$WORK_DIR/invalid.err"; then
  fail "mixed --profile and --changed was accepted"
fi
grep -Fq -- '--changed and --profile are mutually exclusive' "$WORK_DIR/invalid.err" \
  || fail "mixed profile error was not explicit"

if run_validate --profile changed . >"$WORK_DIR/profile-changed.out" 2>"$WORK_DIR/profile-changed.err"; then
  fail "--profile changed was accepted"
fi
grep -Fq -- 'use --changed instead of --profile changed' "$WORK_DIR/profile-changed.err" \
  || fail "changed profile spelling error was not explicit"

if run_validate --profile quick --changed-file README.md . >"$WORK_DIR/changed-file.out" 2>"$WORK_DIR/changed-file.err"; then
  fail "--changed-file without --changed was accepted"
fi
grep -Fq -- '--changed-file requires --changed' "$WORK_DIR/changed-file.err" \
  || fail "orphan changed-file error was not explicit"

echo "Sley validation profile and report tests passed"
