#!/usr/bin/env bash
set -euo pipefail

repo="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
fixture="$repo/fixtures/machine/package"
machine=("$repo/bin/sley" machine --source "$fixture" --package-id machine-fixture --package-version v1)
base='{"protocolVersion":"sley.machine.invoke.v0","requestId":"fixture-1","operation":"invoke","packageId":"machine-fixture","packageVersion":"v1","task":"machine.fixture.summarize","input":{"values":[2,3,5],"label":"ready"},"limits":{"requestLineBytes":4096,"responseBytes":262144,"batch":500,"evaluationSteps":250000,"callDepth":128,"collectionLength":4096,"timeoutMs":2000}}'

check_report="$("$repo/bin/sley" check --json "$fixture")"
jq -e '.status == "ok" and (.diagnostics | length) == 0' <<<"$check_report" >/dev/null

first="$(printf '%s\n' "$base" | "${machine[@]}")"
second="$(printf '%s\n' "$base" | "${machine[@]}")"
[[ "$first" == "$second" ]]
jq -e '
  .schema == "sley.machine.response.v0"
  and .status == "ok"
  and .result == {label:"ready",meta:{count:3,first:2},tags:["machine","ready"],total:10}
  and .counters.calls == 1
  and .counters.maxCallDepth == 1
  and .counters.outputBytes > 0
' <<<"$first" >/dev/null
python3 - "$repo/docs/schemas/sley.machine.response.v0.schema.json" "$first" <<'PY'
import json
import sys
import jsonschema

with open(sys.argv[1], encoding="utf-8") as handle:
    schema = json.load(handle)
jsonschema.validate(json.loads(sys.argv[2]), schema)
PY

batch="$(printf '%s\n%s\n' "$base" "${base/fixture-1/fixture-2}" | "${machine[@]}")"
[[ "$(wc -l <<<"$batch")" -eq 2 ]]
jq -s -e '.[0].counters.batchIndex == 1 and .[1].counters.batchIndex == 2' <<<"$batch" >/dev/null

intrinsic_request="$(jq -c '.requestId = "intrinsic-1" | .task = "machine.fixture.intrinsic_probe" | .input = {}' <<<"$base")"
intrinsic="$(printf '%s\n' "$intrinsic_request" | "${machine[@]}")"
jq -e '.status == "ok" and .result == {label:"STRASSE",meta:{count:3,first:2},tags:["1990","9","2","1"],total:4}' <<<"$intrinsic" >/dev/null

set +e
malformed="$(printf '{bad json}\n' | "${machine[@]}" 2>/dev/null)"
malformed_rc=$?
wrong_pin_request="$(jq -c '.packageId = "machine-other"' <<<"$base")"
wrong_pin="$(printf '%s\n' "$wrong_pin_request" | "${machine[@]}" 2>/dev/null)"
wrong_pin_rc=$?
step_request="$(jq -c '.limits.evaluationSteps = 1' <<<"$base")"
step_limited="$(printf '%s\n' "$step_request" | "${machine[@]}" 2>/dev/null)"
step_limited_rc=$?
effect_request='{"protocolVersion":"sley.machine.invoke.v0","requestId":"effect-1","operation":"invoke","packageId":"machine-fixture","packageVersion":"v1","task":"machine.limits.effectful","input":{}}'
effectful="$(printf '%s\n' "$effect_request" | "${machine[@]}" 2>/dev/null)"
effectful_rc=$?
set -e

[[ "$malformed_rc" -eq 64 ]]
jq -e '.status == "error" and .error.code == "SLEY_MACHINE_JSON_PARSE_ERROR"' <<<"$malformed" >/dev/null
[[ "$wrong_pin_rc" -eq 72 ]]
jq -e '.status == "error" and .error.code == "SLEY_MACHINE_PIN_MISMATCH"' <<<"$wrong_pin" >/dev/null
[[ "$step_limited_rc" -eq 71 ]]
jq -e '.status == "error" and .error.code == "SLEY_MACHINE_STEP_LIMIT_EXCEEDED"' <<<"$step_limited" >/dev/null
[[ "$effectful_rc" -eq 70 ]]
jq -e '.status == "error" and .error.code == "SLEY_MACHINE_EFFECTFUL_TASK"' <<<"$effectful" >/dev/null

printf 'machine tests passed\n'
