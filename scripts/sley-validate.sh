#!/usr/bin/env bash
set -euo pipefail

ROOT_DIR="$(cd "$(dirname "$0")/.." && pwd -P)"
profile=""
changed=0
explain=0
target=""
declare -a changed_files=()

usage() {
  cat <<'EOF'
Usage:
  sley validate --profile quick|core|release [--explain] [--json] .
  sley validate --changed [--changed-file PATH ...] [--explain] [--json] .

Validation is repository-local. The report is always emitted as JSON.
EOF
}

fail_usage() {
  echo "$1" >&2
  usage >&2
  exit 2
}

while (($#)); do
  case "$1" in
    --profile)
      [[ $# -ge 2 ]] || fail_usage "--profile requires a value"
      [[ -z "$profile" ]] || fail_usage "--profile may be provided once"
      profile="$2"
      shift 2
      ;;
    --changed)
      changed=1
      shift
      ;;
    --changed-file)
      [[ $# -ge 2 ]] || fail_usage "--changed-file requires a path"
      changed_files+=("$2")
      shift 2
      ;;
    --explain)
      explain=1
      shift
      ;;
    --json)
      shift
      ;;
    --help|-h)
      usage
      exit 0
      ;;
    --*)
      fail_usage "unknown option: $1"
      ;;
    *)
      [[ -z "$target" ]] || fail_usage "validation accepts one repository target"
      target="$1"
      shift
      ;;
  esac
done

if ((changed)) && [[ -n "$profile" ]]; then
  fail_usage "--changed and --profile are mutually exclusive"
fi
if ((!changed)) && ((${#changed_files[@]})); then
  fail_usage "--changed-file requires --changed"
fi
if ((!changed)) && [[ "$profile" == "changed" ]]; then
  fail_usage "use --changed instead of --profile changed"
fi
if ((changed)); then
  profile="changed"
fi
case "$profile" in
  quick|changed|core|release) ;;
  "") fail_usage "validation requires --profile or --changed" ;;
  *) fail_usage "unknown validation profile: $profile" ;;
esac
target="${target:-.}"
target_root="$(realpath -e "$target" 2>/dev/null || true)"
[[ -d "$target_root" ]] || fail_usage "validation target must be an existing directory"
[[ "$target_root" == "$ROOT_DIR" ]] || fail_usage "Sley v1 validation is restricted to its repository root"

for path in "${changed_files[@]}"; do
  case "$path" in
    ""|/*|..|../*|*/../*|*/..|*$'\n'*|*$'\r'*) fail_usage "changed paths must be repository-relative and confined: $path" ;;
  esac
  resolved="$(realpath -m "$ROOT_DIR/$path")"
  [[ "$resolved" == "$ROOT_DIR"/* ]] || fail_usage "changed path escapes repository root: $path"
done

required_env=(
  SLEY_VALIDATION_REPORT_SCHEMA SLEY_VALIDATION_PROFILES SLEY_VALIDATION_CHECK_REGISTRY
  SLEY_VALIDATION_QUICK_CHECKS SLEY_VALIDATION_CORE_CHECKS
  SLEY_VALIDATION_RELEASE_CHECKS SLEY_VALIDATION_AUTHORITY_MODE
  SLEY_VALIDATION_RESULT_CACHE_POLICY SLEY_VALIDATION_SOURCE_TASK_CACHE_ENABLED
)
for name in "${required_env[@]}"; do
  [[ -n "${!name:-}" ]] || { echo "missing validation contract value: $name" >&2; exit 1; }
done
jq -e --arg profile "$profile" '.[] | select(.id == $profile)' <<<"$SLEY_VALIDATION_PROFILES" >/dev/null \
  || fail_usage "profile is absent from the Sley validation registry: $profile"
profile_executor="$(jq -r --arg profile "$profile" '.[] | select(.id == $profile) | .executor' <<<"$SLEY_VALIDATION_PROFILES")"
case "$profile" in
  quick) expected_executor="make quick" ;;
  changed) expected_executor="scripts/check-changed.sh" ;;
  core) expected_executor="make core" ;;
  release) expected_executor="make v1" ;;
esac
[[ "$profile_executor" == "$expected_executor" ]] \
  || { echo "validation profile executor drift: $profile" >&2; exit 1; }

plan_args=(--plan-json)
if ((${#changed_files[@]})); then
  plan_args+=(--files "${changed_files[@]}")
fi
changed_plan="$($ROOT_DIR/scripts/check-changed.sh "${plan_args[@]}")"
printf '%s\n' "$changed_plan" | jq -e '.schema == "sley.validation.changed_plan.v1"' >/dev/null \
  || { echo "changed validation planner returned an invalid report" >&2; exit 1; }

case "$profile" in
  quick) selected_checks="$SLEY_VALIDATION_QUICK_CHECKS" ;;
  core) selected_checks="$SLEY_VALIDATION_CORE_CHECKS" ;;
  release) selected_checks="$SLEY_VALIDATION_RELEASE_CHECKS" ;;
  changed) selected_checks="$(printf '%s\n' "$changed_plan" | jq -c '.selected_targets')" ;;
esac

all_checks="$(printf '%s\n' "$SLEY_VALIDATION_CHECK_REGISTRY" | jq -c 'map(.id)')"
selected_subsystems="$(jq -n \
  --argjson registry "$SLEY_VALIDATION_CHECK_REGISTRY" \
  --argjson selected "$selected_checks" '
    [$registry[] | select(.id as $id | $selected | index($id)) | .subsystem] | unique
  ')"
if [[ "$profile" == "changed" ]]; then
  selected_subsystems="$(printf '%s\n' "$changed_plan" | jq -c '.selected_subsystems')"
fi

changed_count="$(printf '%s\n' "$changed_plan" | jq '.changed_files | length')"
skipped_reason="profile_scope"
if [[ "$profile" == "changed" ]]; then
  if [[ "$changed_count" -eq 0 ]]; then
    skipped_reason="no_detected_changes"
  else
    skipped_reason="unchanged_subsystem"
  fi
fi
skipped_checks="$(jq -n \
  --arg profile "$profile" \
  --arg reason "$skipped_reason" \
  --argjson all "$all_checks" \
  --argjson selected "$selected_checks" '
    [$all[] | select(. as $id | $selected | index($id) | not) |
      {id:., reason:(if $profile == "release" and . == "machine-test" then "not_part_of_release_gate" else $reason end)}]
  ')"
selection_reasons='[]'
if ((explain)); then
  selection_reasons="$(jq -n --arg profile "$profile" --argjson selected "$selected_checks" '
    [$selected[] | {id:., reason:(if $profile == "changed" then "selected_by_changed_path_classifier" else "selected_by_" + $profile + "_profile" end)}]
  ')"
fi

release_required="$(printf '%s\n' "$changed_plan" | jq -r '.release_gate.required')"
release_reason="$(printf '%s\n' "$changed_plan" | jq -r '.release_gate.reason')"
release_executed=false
if [[ "$profile" == "release" ]]; then
  release_required=true
  release_executed=true
  release_reason="executed because the release profile is the authoritative full V1 gate"
fi

case "$profile" in
  quick) command_argv=(make --no-print-directory quick) ;;
  core) command_argv=(make --no-print-directory core) ;;
  release) command_argv=(make --no-print-directory v1) ;;
  changed)
    command_argv=("$ROOT_DIR/scripts/check-changed.sh")
    if ((${#changed_files[@]})); then
      command_argv+=(--files "${changed_files[@]}")
    fi
    ;;
esac

command_json="$(printf '%s\n' "${command_argv[@]}" | jq -Rsc 'split("\n")[:-1]')"
started_ms="$(date +%s%3N)"
evidence_file="$(mktemp "${TMPDIR:-/tmp}/sley-validation.XXXXXX")"
evidence_meta_file="${evidence_file}.meta"
max_evidence_bytes=8388608
trap 'rm -f "$evidence_file" "$evidence_meta_file"' EXIT
set +e
(cd "$ROOT_DIR" && "${command_argv[@]}" 2>&1) | python3 -c '
import pathlib
import sys

output = pathlib.Path(sys.argv[1])
metadata = pathlib.Path(sys.argv[2])
limit = int(sys.argv[3])
total = 0
retained = 0
with output.open("wb") as stream:
    while True:
        chunk = sys.stdin.buffer.read(65536)
        if not chunk:
            break
        total += len(chunk)
        if retained < limit:
            kept = chunk[:limit - retained]
            stream.write(kept)
            retained += len(kept)
metadata.write_text(f"{total}\n{str(total > limit).lower()}\n", encoding="utf-8")
' "$evidence_file" "$evidence_meta_file" "$max_evidence_bytes"
pipeline_status=("${PIPESTATUS[@]}")
set -e
command_rc="${pipeline_status[0]}"
collector_rc="${pipeline_status[1]}"
if [[ "$collector_rc" -ne 0 && "$command_rc" -eq 0 ]]; then
  command_rc="$collector_rc"
fi
if [[ "$command_rc" -eq 0 ]]; then
  status="passed"
  outcome="passed"
  issues='[]'
else
  status="failed"
  outcome="failed"
fi
ended_ms="$(date +%s%3N)"
wall_time_ms=$((ended_ms - started_ms))
evidence_bytes="$(wc -c <"$evidence_file" | tr -d ' ')"
if [[ -f "$evidence_meta_file" ]]; then
  evidence_total_bytes="$(sed -n '1p' "$evidence_meta_file")"
  evidence_truncated="$(sed -n '2p' "$evidence_meta_file")"
else
  evidence_total_bytes="$evidence_bytes"
  evidence_truncated=false
fi
evidence_digest="sha256:$(sha256sum "$evidence_file" | awk '{print $1}')"
if [[ "$status" == "failed" ]]; then
  issues="$(jq -n --arg digest "$evidence_digest" --argjson exit_code "$command_rc" '[{
    code:"SLEY_VALIDATION_CHECK_FAILED",
    message:("validation executor exited with status " + ($exit_code | tostring)),
    evidence_digest:$digest
  }]')"
fi

step="$(jq -n \
  --arg id "$profile" \
  --arg status "$status" \
  --arg digest "$evidence_digest" \
  --argjson command "$command_json" \
  --argjson exit_code "$command_rc" \
  --argjson wall_time_ms "$wall_time_ms" \
  --argjson evidence_bytes "$evidence_bytes" \
  --argjson evidence_total_bytes "$evidence_total_bytes" \
  --argjson evidence_truncated "$evidence_truncated" '{
    id:$id,
    command:$command,
    status:$status,
    exit_code:$exit_code,
    wall_time_ms:$wall_time_ms,
    evidence_digest:$digest,
    evidence_bytes:$evidence_bytes,
    evidence_total_bytes:$evidence_total_bytes,
    evidence_truncated:$evidence_truncated,
    evidence_embedded:false
  }')"

jq -n \
  --arg schema "$SLEY_VALIDATION_REPORT_SCHEMA" \
  --arg status "$status" \
  --arg profile "$profile" \
  --arg target "." \
  --arg authority_mode "$SLEY_VALIDATION_AUTHORITY_MODE" \
  --arg cache_policy "$SLEY_VALIDATION_RESULT_CACHE_POLICY" \
  --argjson source_task_cache_enabled "$SLEY_VALIDATION_SOURCE_TASK_CACHE_ENABLED" \
  --arg release_reason "$release_reason" \
  --arg outcome "$outcome" \
  --argjson changed_plan "$changed_plan" \
  --argjson selected_subsystems "$selected_subsystems" \
  --argjson selected_checks "$selected_checks" \
  --argjson skipped_checks "$skipped_checks" \
  --argjson selection_reasons "$selection_reasons" \
  --argjson release_required "$release_required" \
  --argjson release_executed "$release_executed" \
  --argjson wall_time_ms "$wall_time_ms" \
  --argjson step "$step" \
  --argjson issues "$issues" '{
    schema:$schema,
    status:$status,
    profile:$profile,
    target:$target,
    detected_changes:$changed_plan.changed_files,
    detected_subsystems:$changed_plan.selected_subsystems,
    selected_subsystems:$selected_subsystems,
    selected_checks:$selected_checks,
    skipped_checks:$skipped_checks,
    selection_reasons:$selection_reasons,
    release_gate:{
      required:$release_required,
      executed:$release_executed,
      authoritative_command:["make","v1"],
      reason:$release_reason
    },
    cache:{
      validation_result_used:false,
      validation_result_policy:$cache_policy,
      source_task_cache_enabled:$source_task_cache_enabled,
      source_task_cache_scope:"contract_loading_only"
    },
    authority:{
      mode:$authority_mode,
      repository_mutation:false,
      provider_calls:false,
      network_calls:false,
      deploy:false,
      spend:false
    },
    wall_time_ms:$wall_time_ms,
    outcome:$outcome,
    steps:[$step],
    issues:$issues
  }'
exit "$command_rc"
