#!/usr/bin/env bash
set -euo pipefail

repo_root="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
schema_root="$repo_root/docs/schemas"
case_schema="$schema_root/sley.agent_bench.case_manifest.v0.schema.json"
run_schema="$schema_root/sley.agent_bench.run_manifest.v0.schema.json"
event_schema="$schema_root/sley.agent_bench.event.v0.schema.json"
result_schema="$schema_root/sley.agent_bench.case_result.v0.schema.json"
aggregate_schema="$schema_root/sley.agent_bench.aggregate.v0.schema.json"
empty_tree_digest="sha256:e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855"

case_manifest=""
run_manifest=""
sley_bin="$repo_root/bin/sley"
output_dir=""
only_case=""

die() {
  printf 'sley-agent-bench: %s\n' "$1" >&2
  exit 1
}

usage() {
  cat <<'EOF'
Usage:
  sley-agent-bench run --json --manifest <cases.json> --run-manifest <run.json>
    [--sley-bin <path>] [--output-dir <new-directory>] [--case <case-id>]

The v0 evaluator accepts only the deterministic-fixture adapter and trusted,
repository-owned smoke fixtures. It does not call a model or provider.
EOF
}

while [[ "$#" -gt 0 ]]; do
  case "$1" in
    run|--json) shift ;;
    --manifest) case_manifest="${2:-}"; shift 2 ;;
    --run-manifest) run_manifest="${2:-}"; shift 2 ;;
    --sley-bin) sley_bin="${2:-}"; shift 2 ;;
    --output-dir) output_dir="${2:-}"; shift 2 ;;
    --case) only_case="${2:-}"; shift 2 ;;
    --help|-h) usage; exit 0 ;;
    *) die "unknown evaluator option: $1" ;;
  esac
done

[[ -n "$case_manifest" ]] || die "--manifest is required"
[[ -n "$run_manifest" ]] || die "--run-manifest is required"
[[ -x "$sley_bin" ]] || die "Sley binary is not executable: $sley_bin"

case_manifest="$(realpath "$case_manifest")"
run_manifest="$(realpath "$run_manifest")"
sley_bin="$(realpath "$sley_bin")"
[[ -f "$case_manifest" ]] || die "case manifest is not a file"
[[ -f "$run_manifest" ]] || die "run manifest is not a file"

"$sley_bin" sley-contract validate \
  --schema sley.agent_bench.case_manifest.v0 "$case_manifest" \
  --schemas "$schema_root" --json >/dev/null \
  || die "case manifest failed schema validation"
"$sley_bin" sley-contract validate \
  --schema sley.agent_bench.run_manifest.v0 "$run_manifest" \
  --schemas "$schema_root" --json >/dev/null \
  || die "run manifest failed schema validation"

sha256_file() {
  printf 'sha256:%s\n' "$(sha256sum "$1" | awk '{print $1}')"
}

tree_digest() {
  local root="$1"
  (
    cd "$root"
    find . -type f ! -path './.git/*' -print0 \
      | sort -z \
      | while IFS= read -r -d '' file; do
          printf '%s\0' "${file#./}"
          sha256sum "$file" | awk '{print $1}'
        done
  ) | sha256sum | awk '{print "sha256:" $1}'
}

training_digest() {
  local prompt="$1" workspace="$2"
  {
    sha256_file "$prompt"
    tree_digest "$workspace"
  } | sha256sum | awk '{print "sha256:" $1}'
}

safe_relative_path() {
  local path="$1" part
  [[ "$path" != /* && -n "$path" ]] || return 1
  IFS='/' read -r -a parts <<< "$path"
  for part in "${parts[@]}"; do
    [[ -n "$part" && "$part" != "." && "$part" != ".." ]] || return 1
  done
}

resolve_inside() {
  local base="$1" relative="$2" resolved
  safe_relative_path "$relative" || return 1
  resolved="$(realpath "$base/$relative")" || return 1
  [[ "$resolved" == "$base"/* ]] || return 1
  printf '%s\n' "$resolved"
}

case_manifest_digest="$(sha256_file "$case_manifest")"
run_manifest_digest="$(sha256_file "$run_manifest")"
declared_case_digest="$(jq -r '.case_manifest_digest' "$run_manifest")"
[[ "$declared_case_digest" == "$case_manifest_digest" ]] \
  || die "run manifest case digest does not match the case manifest"

case_manifest_dir="$(dirname "$case_manifest")"
declared_manifest_path="$(jq -r '.case_manifest_path' "$run_manifest")"
resolved_declared_manifest="$(resolve_inside "$case_manifest_dir" "$declared_manifest_path")" \
  || die "run manifest case path escapes its manifest directory"
[[ "$resolved_declared_manifest" == "$case_manifest" ]] \
  || die "run manifest case path does not resolve to --manifest"

suite_id="$(jq -r '.suite_id' "$case_manifest")"
run_suite_id="$(jq -r '.suite_id' "$run_manifest")"
[[ "$suite_id" == "$run_suite_id" ]] || die "suite IDs do not match"
tier="$(jq -r '.tier' "$case_manifest")"
run_id="$(jq -r '.run_id' "$run_manifest")"
mode="$(jq -r '.mode' "$run_manifest")"
adapter_id="$(jq -r '.adapter.id' "$run_manifest")"
retain_workspaces="$(jq -r '.retain_workspaces' "$run_manifest")"

bootstrap_path="$(jq -r '.bootstrap.path' "$run_manifest")"
bootstrap_abs="$(resolve_inside "$repo_root" "$bootstrap_path")" \
  || die "bootstrap path escapes the repository"
bootstrap_digest="$(sha256_file "$bootstrap_abs")"
[[ "$bootstrap_digest" == "$(jq -r '.bootstrap.digest' "$run_manifest")" ]] \
  || die "bootstrap digest does not match"

sley_implementation_version="$($sley_bin --version | sed -n 's/^sley //p' | head -n 1)"
[[ -n "$sley_implementation_version" ]] || die "could not resolve Sley version"
sley_version="$(sed -E 's/^([0-9]+\.[0-9]+)(\.[0-9]+)?$/\1/' <<< "$sley_implementation_version")"
[[ "$sley_version" =~ ^[0-9]+\.[0-9]+$ ]] || die "could not resolve Sley compatibility version"

if [[ "$tier" == "smoke" ]]; then
  jq -e '
    (.cases | length) == 24 and
    ([.cases[].id] | unique | length) == 24 and
    ([.cases[].family] | unique | length) == 6 and
    ([.cases | group_by(.family)[] | length] | all(. == 4)) and
    all(.cases[]; .visibility == "public")
  ' "$case_manifest" >/dev/null || die "smoke manifest must contain four public cases per family"
fi

if [[ -n "$output_dir" ]]; then
  [[ ! -e "$output_dir" ]] || die "--output-dir must not already exist"
  mkdir -p "$output_dir"
  run_root="$(realpath "$output_dir")"
  temporary_run_root=false
else
  [[ "$retain_workspaces" == "false" ]] \
    || die "retained workspaces require an explicit --output-dir"
  run_root="$(mktemp -d)"
  temporary_run_root=true
fi

cleanup() {
  if [[ "$temporary_run_root" == "true" && -d "$run_root" ]]; then
    rm -rf -- "$run_root"
  fi
}
trap cleanup EXIT

mkdir -p "$run_root/workspaces" "$run_root/results" "$run_root/steps" \
  "$run_root/evidence" "$run_root/cache" "$run_root/tmp"
baseline_process_count="$(ps -eLf -u "$(id -u)" --no-headers | wc -l)"
events_path="$run_root/events.jsonl"
results_jsonl="$run_root/case-results.jsonl"
: > "$events_path"
: > "$results_jsonl"

event_sequence=0
emit_event() {
  local kind="$1" case_id="$2" payload="$3"
  event_sequence=$((event_sequence + 1))
  jq -cn \
    --arg schema "sley.agent_bench.event.v0" \
    --arg run_id "$run_id" \
    --argjson sequence "$event_sequence" \
    --arg kind "$kind" \
    --arg case_id "$case_id" \
    --argjson payload "$payload" '
      {
        schema:$schema,
        run_id:$run_id,
        sequence:$sequence,
        kind:$kind,
        case_id:(if $case_id == "" then null else $case_id end),
        payload:$payload
      }
    ' >> "$events_path"
}

emit_event run_started "" "$(jq -cn --arg suite "$suite_id" --arg mode "$mode" --arg adapter "$adapter_id" '{suite_id:$suite,mode:$mode,adapter_id:$adapter}')"

path_is_owned() {
  local path="$1" owned
  while IFS= read -r owned; do
    if [[ "$owned" == */ ]]; then
      [[ "$path" == "$owned"* ]] && return 0
    elif [[ "$path" == "$owned" ]]; then
      return 0
    fi
  done
  return 1
}

pointer_assertion_count() {
  local output_file="$1" assertions_json="$2"
  jq -n --slurpfile document "$output_file" --argjson assertions "$assertions_json" '
    def pointer_path($pointer):
      if $pointer == "" then []
      else
        $pointer
        | ltrimstr("/")
        | split("/")
        | map(gsub("~1"; "/") | gsub("~0"; "~") | if test("^(0|[1-9][0-9]*)$") then tonumber else . end)
      end;
    ($document[0] // null) as $doc
    | [$assertions[] | . as $assertion | (($doc | getpath(pointer_path($assertion.pointer))) == $assertion.value)]
    | map(select(. == true))
    | length
  '
}

run_step() {
  local case_json="$1" phase="$2" command_json="$3" workspace="$4" case_step_dir="$5"
  local step_name expected_exit expected_schema assertions_json assertion_count assertion_passed_count
  local tool output_file stderr_file actual_exit output_bytes stdout_schema step_status issues_json
  local wall_seconds max_output processes tool_call_limit file_blocks
  local elapsed_ms remaining_ms remaining_seconds
  local logical_target actual_target redacted_output output_text
  local command_pid process_count process_limit_hit
  local -a logical_argv exec_argv

  step_name="$(jq -r '.name' <<< "$command_json")"
  expected_exit="$(jq -r '.expected_exit' <<< "$command_json")"
  expected_schema="$(jq -r '.expected_schema // ""' <<< "$command_json")"
  assertions_json="$(jq -c '.assertions' <<< "$command_json")"
  assertion_count="$(jq 'length' <<< "$assertions_json")"
  mapfile -d '' -t logical_argv < <(jq -j '.argv[] + "\u0000"' <<< "$command_json")
  tool="${logical_argv[1]}"
  wall_seconds="$(jq -r '.limits.wall_seconds' <<< "$case_json")"
  max_output="$(jq -r '.limits.output_bytes' <<< "$case_json")"
  processes="$(jq -r '.limits.processes' <<< "$case_json")"
  tool_call_limit="$(jq -r '.limits.tool_calls' <<< "$case_json")"
  elapsed_ms=$(( $(date +%s%3N) - case_started_ms ))
  remaining_ms=$((wall_seconds * 1000 - elapsed_ms))
  remaining_seconds=$(( (remaining_ms + 999) / 1000 ))

  case_tool_calls=$((case_tool_calls + 1))
  case_compiler_cycles=$((case_compiler_cycles + 1))
  output_file="$case_step_dir/${phase}-${case_tool_calls}-${step_name}.stdout"
  stderr_file="$case_step_dir/${phase}-${case_tool_calls}-${step_name}.stderr"
  issues_json='[]'
  actual_exit=125
  process_limit_hit=false

  if (( remaining_ms <= 0 )); then
    issues_json='[{"code":"SLEYBENCH_WALL_LIMIT","message":"case exhausted its wall-time ceiling before the step"}]'
    : > "$output_file"
    : > "$stderr_file"
  elif (( case_tool_calls > tool_call_limit )); then
    issues_json='[{"code":"SLEYBENCH_TOOL_LIMIT","message":"case exceeded its tool-call ceiling"}]'
    : > "$output_file"
    : > "$stderr_file"
  elif ! jq -e --arg tool "$tool" '.allowed_tools | index($tool) != null' <<< "$case_json" >/dev/null; then
    issues_json='[{"code":"SLEYBENCH_TOOL_NOT_ALLOWED","message":"manifest requested a tool outside the case allowlist"}]'
    case_invalid_tool_call=true
    : > "$output_file"
    : > "$stderr_file"
  elif ! jq -e --arg tool "$tool" '.tool_allowlist | index($tool) != null' "$run_manifest" >/dev/null; then
    issues_json='[{"code":"SLEYBENCH_RUN_TOOL_NOT_ALLOWED","message":"manifest requested a tool outside the run allowlist"}]'
    case_invalid_tool_call=true
    : > "$output_file"
    : > "$stderr_file"
  else
    exec_argv=("$sley_bin" "${logical_argv[@]:1}")
    logical_target="${logical_argv[${#logical_argv[@]} - 1]}"
    if [[ "$tool" == "run" ]]; then
      actual_target="$(realpath "$workspace/$logical_target")" \
        || die "$case_id run target could not be resolved"
      [[ "$actual_target" == "$workspace" || "$actual_target" == "$workspace"/* ]] \
        || die "$case_id run target escapes its workspace"
      exec_argv[${#exec_argv[@]} - 1]="$actual_target"
    fi
    file_blocks=$(( (max_output + 1023) / 1024 ))
    set +e
    (
      cd "$workspace"
      ulimit -f "$file_blocks"
      # RLIMIT_NPROC counts every process owned by the current user, not just
      # this case. Treat the manifest value as the case's additional budget.
      ulimit -u "$((baseline_process_count + processes))" 2>/dev/null || true
      exec setsid timeout --signal=KILL "$remaining_seconds" \
        env -i PATH="$repo_root/bin:/usr/bin:/bin" LANG=C LC_ALL=C \
        SLEY_SOURCE_CACHE_DIR="$run_root/cache" TMPDIR="$run_root/tmp" \
        "${exec_argv[@]}"
    ) >"$output_file" 2>"$stderr_file" &
    command_pid=$!
    while kill -0 "$command_pid" 2>/dev/null; do
      process_count="$(ps -eo pid=,ppid= | awk -v root="$command_pid" '
        { pid[NR]=$1; parent[NR]=$2 }
        END {
          descendant[root]=1
          changed=1
          while (changed) {
            changed=0
            for (cursor=1; cursor<=NR; cursor++) {
              if (descendant[parent[cursor]] && !descendant[pid[cursor]]) {
                descendant[pid[cursor]]=1
                changed=1
              }
            }
          }
          count=0
          for (item in descendant) if (descendant[item]) count++
          print count
        }
      ')"
      if (( process_count > processes )); then
        process_limit_hit=true
        kill -KILL -- "-$command_pid" 2>/dev/null || true
        break
      fi
      sleep 0.1
    done
    wait "$command_pid" 2>/dev/null
    actual_exit=$?
    set -e
    for redacted_output in "$output_file" "$stderr_file"; do
      output_text="$(<"$redacted_output")"
      printf '%s' "${output_text//"$workspace"/<workspace>}" > "$redacted_output"
    done
    if [[ "$tool" == "run" ]] && jq -e '.schema == "sley.run.report.v0"' "$output_file" >/dev/null 2>&1; then
      jq --arg target "$logical_target" '.target = $target' "$output_file" > "$output_file.tmp"
      mv "$output_file.tmp" "$output_file"
    fi
    if [[ "$process_limit_hit" == true ]]; then
      issues_json='[{"code":"SLEYBENCH_PROCESS_LIMIT","message":"case command exceeded its process-tree ceiling"}]'
    fi
  fi

  output_bytes=$(( $(wc -c < "$output_file") + $(wc -c < "$stderr_file") ))
  stdout_schema=null
  assertion_passed_count=0
  if jq -e . "$output_file" >/dev/null 2>&1; then
    stdout_schema="$(jq -r '.schema // empty' "$output_file")"
    assertion_passed_count="$(pointer_assertion_count "$output_file" "$assertions_json")"
  fi

  step_status=passed
  if [[ "$actual_exit" -ne "$expected_exit" ]]; then
    step_status=failed
    issues_json="$(jq -c --argjson issues "$issues_json" --arg message "expected exit $expected_exit, got $actual_exit" '$issues + [{code:"SLEYBENCH_EXIT_MISMATCH",message:$message}]' <<< '{}')"
  fi
  if [[ "$process_limit_hit" != true && ( "$actual_exit" -eq 124 || "$actual_exit" -eq 137 ) ]]; then
    step_status=failed
    issues_json="$(jq -c --argjson issues "$issues_json" '$issues + [{code:"SLEYBENCH_WALL_LIMIT",message:"case command exceeded its remaining wall-time ceiling"}]' <<< '{}')"
  fi
  if [[ "$actual_exit" -eq 153 ]]; then
    step_status=failed
    issues_json="$(jq -c --argjson issues "$issues_json" '$issues + [{code:"SLEYBENCH_OUTPUT_LIMIT",message:"case command exceeded its file-output ceiling"}]' <<< '{}')"
  fi
  if [[ "$process_limit_hit" != true && "$actual_exit" -ne "$expected_exit" ]] \
    && grep -Eqi 'resource temporarily unavailable|cannot fork|fork.*failed' "$stderr_file"; then
    step_status=failed
    issues_json="$(jq -c --argjson issues "$issues_json" '$issues + [{code:"SLEYBENCH_PROCESS_LIMIT",message:"case command exhausted its additional process ceiling"}]' <<< '{}')"
  fi
  if (( output_bytes > max_output )); then
    step_status=failed
    issues_json="$(jq -c --argjson issues "$issues_json" '$issues + [{code:"SLEYBENCH_OUTPUT_LIMIT",message:"step exceeded its output-byte ceiling"}]' <<< '{}')"
  fi
  if [[ -n "$expected_schema" && "$stdout_schema" != "$expected_schema" ]]; then
    step_status=failed
    issues_json="$(jq -c --argjson issues "$issues_json" --arg expected "$expected_schema" --arg actual "$stdout_schema" '$issues + [{code:"SLEYBENCH_SCHEMA_MISMATCH",message:("expected schema " + $expected + ", got " + $actual)}]' <<< '{}')"
  fi
  if [[ "$assertion_passed_count" -ne "$assertion_count" ]]; then
    step_status=failed
    issues_json="$(jq -c --argjson issues "$issues_json" '$issues + [{code:"SLEYBENCH_ASSERTION_FAILED",message:"one or more JSON-pointer assertions failed"}]' <<< '{}')"
  fi

  step_json="$(jq -cn \
    --arg name "$step_name" \
    --arg phase "$phase" \
    --argjson argv "$(printf '%s\n' "${logical_argv[@]}" | jq -R . | jq -s .)" \
    --argjson expected_exit "$expected_exit" \
    --argjson actual_exit "$actual_exit" \
    --arg status "$step_status" \
    --arg stdout_schema "$stdout_schema" \
    --argjson assertion_count "$assertion_count" \
    --argjson assertion_passed_count "$assertion_passed_count" \
    --argjson output_bytes "$output_bytes" \
    --arg stdout_digest "$(sha256_file "$output_file")" \
    --arg stderr_digest "$(sha256_file "$stderr_file")" \
    --argjson issues "$issues_json" '
      {
        name:$name,
        phase:$phase,
        argv:$argv,
        expected_exit:$expected_exit,
        actual_exit:$actual_exit,
        status:$status,
        stdout_schema:(if $stdout_schema == "null" or $stdout_schema == "" then null else $stdout_schema end),
        assertion_count:$assertion_count,
        assertion_passed_count:$assertion_passed_count,
        output_bytes:$output_bytes,
        stdout_digest:$stdout_digest,
        stderr_digest:$stderr_digest,
        issues:$issues
      }
    ')"
  printf '%s\n' "$step_json" >> "$case_steps_path"
  emit_event "${phase}_completed" "$case_id" "$(jq -cn --arg name "$step_name" --arg status "$step_status" --arg digest "$(sha256_file "$output_file")" '{step:$name,status:$status,stdout_digest:$digest}')"
  [[ "$step_status" == "passed" ]]
}

if [[ -n "$only_case" ]]; then
  jq -e --arg id "$only_case" 'any(.cases[]; .id == $id)' "$case_manifest" >/dev/null \
    || die "requested case is not present in the manifest: $only_case"
  mapfile -t case_indices < <(jq -r --arg id "$only_case" '.cases | to_entries[] | select(.value.id == $id) | .key' "$case_manifest")
else
  mapfile -t case_indices < <(jq -r '.cases | keys[]' "$case_manifest")
fi
case_count="${#case_indices[@]}"
for case_index in "${case_indices[@]}"; do
  case_json="$(jq -c --argjson index "$case_index" '.cases[$index]' "$case_manifest")"
  case_id="$(jq -r '.id' <<< "$case_json")"
  family="$(jq -r '.family' <<< "$case_json")"
  case_version="$(jq -r '.sley_version' <<< "$case_json")"
  [[ "$case_version" == "$sley_version" ]] || die "$case_id pins Sley $case_version, evaluator is $sley_version"

  prompt_rel="$(jq -r '.prompt_path' <<< "$case_json")"
  workspace_rel="$(jq -r '.workspace_path' <<< "$case_json")"
  candidate_rel="$(jq -r '.candidate_path' <<< "$case_json")"
  prompt_source="$(resolve_inside "$case_manifest_dir" "$prompt_rel")" || die "$case_id prompt escapes the suite"
  workspace_source="$(resolve_inside "$case_manifest_dir" "$workspace_rel")" || die "$case_id workspace escapes the suite"
  candidate_source="$(resolve_inside "$case_manifest_dir" "$candidate_rel")" || die "$case_id candidate escapes the suite"
  declared_candidate_digest="$(jq -r '.candidate_digest // empty' <<< "$case_json")"
  if [[ ! -e "$candidate_source" \
    && "$(jq -r '.minimality.max_changed_files' <<< "$case_json")" == "0" \
    && "$(jq -r '.minimality.max_changed_lines' <<< "$case_json")" == "0" \
    && ( "$tier" != "baseline" || "$declared_candidate_digest" == "$empty_tree_digest" ) ]]; then
    candidate_source="$run_root/empty-candidates/$case_id"
    mkdir -p "$candidate_source"
  fi
  [[ -f "$prompt_source" && -d "$workspace_source" && -d "$candidate_source" ]] \
    || die "$case_id fixture paths are incomplete or do not declare an empty no-op candidate"
  if find "$workspace_source" "$candidate_source" -type l -print -quit | grep -q .; then
    die "$case_id fixture contains a symlink"
  fi

  computed_training_id="$(training_digest "$prompt_source" "$workspace_source")"
  declared_training_id="$(jq -r '.training_exclusion_id' <<< "$case_json")"
  [[ "$computed_training_id" == "$declared_training_id" ]] \
    || die "$case_id training exclusion digest does not match its prompt/workspace"

  case_workspace="$run_root/workspaces/$case_id"
  mkdir -p "$case_workspace"
  cp -a "$workspace_source/." "$case_workspace/"
  initial_tree_digest="$(tree_digest "$case_workspace")"
  candidate_digest="$(tree_digest "$candidate_source")"
  if [[ "$tier" == "baseline" ]]; then
    [[ "$candidate_digest" == "$declared_candidate_digest" ]] \
      || die "$case_id candidate digest does not match its pinned baseline solution"
  fi
  prompt_digest="$(sha256_file "$prompt_source")"
  mkdir -p "$run_root/evidence/$case_id/candidate"
  cp "$prompt_source" "$run_root/evidence/$case_id/prompt.md"
  cp -a "$candidate_source/." "$run_root/evidence/$case_id/candidate/"
  case_started_ms="$(date +%s%3N)"
  emit_event case_started "$case_id" "$(jq -cn --arg family "$family" --arg input "$initial_tree_digest" '{family:$family,initial_tree_digest:$input}')"

  git -C "$case_workspace" init -q
  git -C "$case_workspace" config user.name SleyBench
  git -C "$case_workspace" config user.email sleybench@invalid.example
  git -C "$case_workspace" add --all
  git -C "$case_workspace" commit -qm baseline
  clean_before=true
  [[ -z "$(git -C "$case_workspace" status --porcelain=v1)" ]] || clean_before=false

  case_step_dir="$run_root/steps/$case_id"
  mkdir -p "$case_step_dir"
  case_steps_path="$case_step_dir/steps.jsonl"
  : > "$case_steps_path"
  case_tool_calls=0
  case_compiler_cycles=0
  case_invalid_tool_call=false
  preflight_passed=true
  oracle_passed=true

  while IFS= read -r command_json; do
    if ! run_step "$case_json" preflight "$command_json" "$case_workspace" "$case_step_dir"; then
      preflight_passed=false
    fi
  done < <(jq -c '.preflight[]' <<< "$case_json")

  cp -a "$candidate_source/." "$case_workspace/"
  emit_event candidate_applied "$case_id" "$(jq -cn --arg adapter "$adapter_id" --arg digest "$candidate_digest" '{adapter_id:$adapter,candidate_digest:$digest}')"

  workspace_bytes="$(du -sb "$case_workspace" | awk '{print $1}')"
  workspace_limit="$(jq -r '.limits.workspace_bytes' <<< "$case_json")"
  workspace_within_limit=true
  (( workspace_bytes <= workspace_limit )) || workspace_within_limit=false

  if [[ "$workspace_within_limit" == true ]]; then
    while IFS= read -r command_json; do
      if ! run_step "$case_json" oracle "$command_json" "$case_workspace" "$case_step_dir"; then
        oracle_passed=false
      fi
    done < <(jq -c '.oracle.commands[]' <<< "$case_json")
  else
    oracle_passed=false
  fi

  mapfile -t owned_paths < <(jq -r '.owned_paths[]' <<< "$case_json")
  mapfile -d '' -t changed_entries < <(git -C "$case_workspace" status --porcelain=v1 -z)
  changed_paths=()
  scope_precise=true
  for changed_entry in "${changed_entries[@]}"; do
    changed_path="${changed_entry:3}"
    changed_paths+=("$changed_path")
    if ! printf '%s\n' "${owned_paths[@]}" | path_is_owned "$changed_path"; then
      scope_precise=false
    fi
  done
  changed_files="${#changed_paths[@]}"
  changed_lines="$(git -C "$case_workspace" diff --numstat HEAD -- \
    | awk '{added += ($1 == "-" ? 0 : $1); removed += ($2 == "-" ? 0 : $2)} END {print added + removed + 0}')"
  for changed_path in "${changed_paths[@]}"; do
    if [[ "$(git -C "$case_workspace" status --porcelain=v1 -- "$changed_path")" == '?? '* && -f "$case_workspace/$changed_path" ]]; then
      changed_lines=$((changed_lines + $(wc -l < "$case_workspace/$changed_path")))
    fi
  done

  max_changed_files="$(jq -r '.minimality.max_changed_files' <<< "$case_json")"
  max_changed_lines="$(jq -r '.minimality.max_changed_lines' <<< "$case_json")"
  repair_minimal=true
  (( changed_files <= max_changed_files && changed_lines <= max_changed_lines )) || repair_minimal=false
  final_tree_digest="$(tree_digest "$case_workspace")"
  step_evidence_digest="$(sha256_file "$case_steps_path")"
  case_finished_ms="$(date +%s%3N)"
  case_wall_ms=$((case_finished_ms - case_started_ms))
  wall_limit_ms=$(( $(jq -r '.limits.wall_seconds' <<< "$case_json") * 1000 ))
  wall_within_limit=true
  (( case_wall_ms <= wall_limit_ms )) || wall_within_limit=false

  steps_json="$(jq -s . "$case_steps_path")"
  prompt_accepted=true
  [[ -s "$prompt_source" ]] || prompt_accepted=false
  parse_passed="$(jq -e 'any(.[]; .phase == "oracle" and (.argv[1] == "ast" or .argv[1] == "check") and .status == "passed" and .actual_exit == 0)' <<< "$steps_json" >/dev/null && printf true || printf false)"
  check_passed="$(jq -e 'any(.[]; .phase == "oracle" and .argv[1] == "check" and .status == "passed" and .actual_exit == 0)' <<< "$steps_json" >/dev/null && printf true || printf false)"
  expected_diagnosis_identified=true
  if jq -e '.score_tags | index("repair") != null' <<< "$case_json" >/dev/null; then
    expected_diagnosis_identified="$preflight_passed"
  fi
  semantic_output_matched=true
  if jq -e '.score_tags | index("semantic") != null' <<< "$case_json" >/dev/null; then
    semantic_output_matched="$oracle_passed"
  fi
  required_evidence_produced=true
  [[ -s "$case_steps_path" ]] || required_evidence_produced=false

  case_status=passed
  if [[ "$prompt_accepted" != true || "$preflight_passed" != true || "$oracle_passed" != true \
    || "$scope_precise" != true || "$repair_minimal" != true \
    || "$required_evidence_produced" != true || "$workspace_within_limit" != true \
    || "$wall_within_limit" != true ]]; then
    case_status=failed
  fi

  case_issues='[]'
  [[ "$workspace_within_limit" == true ]] \
    || case_issues="$(jq -c '. + [{code:"SLEYBENCH_WORKSPACE_LIMIT",message:"workspace exceeded its byte ceiling"}]' <<< "$case_issues")"
  [[ "$scope_precise" == true ]] \
    || case_issues="$(jq -c '. + [{code:"SLEYBENCH_SCOPE_VIOLATION",message:"candidate changed a path outside case ownership"}]' <<< "$case_issues")"
  [[ "$repair_minimal" == true ]] \
    || case_issues="$(jq -c '. + [{code:"SLEYBENCH_MINIMALITY_LIMIT",message:"candidate exceeded the case change budget"}]' <<< "$case_issues")"
  [[ "$wall_within_limit" == true ]] \
    || case_issues="$(jq -c '. + [{code:"SLEYBENCH_WALL_LIMIT",message:"case exceeded its wall-time ceiling"}]' <<< "$case_issues")"
  [[ "$preflight_passed" == true && "$oracle_passed" == true ]] \
    || case_issues="$(jq -c '. + [{code:"SLEYBENCH_STEP_FAILED",message:"one or more preflight or oracle steps failed"}]' <<< "$case_issues")"

  score_tags="$(jq -c '.score_tags' <<< "$case_json")"
  case_result="$(jq -cn \
    --arg schema "sley.agent_bench.case_result.v0" \
    --arg run_id "$run_id" \
    --arg case_id "$case_id" \
    --arg family "$family" \
    --arg mode "$mode" \
    --arg adapter_id "$adapter_id" \
    --arg status "$case_status" \
    --argjson score_tags "$score_tags" \
    --argjson prompt_accepted "$prompt_accepted" \
    --argjson parse_passed "$parse_passed" \
    --argjson check_passed "$check_passed" \
    --argjson oracle_passed "$oracle_passed" \
    --argjson semantic_output_matched "$semantic_output_matched" \
    --argjson expected_diagnosis_identified "$expected_diagnosis_identified" \
    --argjson repair_minimal "$repair_minimal" \
    --argjson scope_precise "$scope_precise" \
    --argjson required_evidence_produced "$required_evidence_produced" \
    --argjson tool_calls "$case_tool_calls" \
    --argjson compiler_cycles "$case_compiler_cycles" \
    --argjson wall_ms "$case_wall_ms" \
    --argjson changed_files "$changed_files" \
    --argjson changed_lines "$changed_lines" \
    --argjson invalid_tool_call "$case_invalid_tool_call" \
    --argjson retained "$retain_workspaces" \
    --argjson clean_before "$clean_before" \
    --arg training_exclusion_id "$declared_training_id" \
    --arg initial_tree_digest "$initial_tree_digest" \
    --arg candidate_digest "$candidate_digest" \
    --arg final_tree_digest "$final_tree_digest" \
    --arg prompt_digest "$prompt_digest" \
    --arg step_evidence_digest "$step_evidence_digest" \
    --argjson steps "$steps_json" \
    --argjson issues "$case_issues" '
      {
        schema:$schema,
        run_id:$run_id,
        case_id:$case_id,
        family:$family,
        mode:$mode,
        adapter_id:$adapter_id,
        status:$status,
        score_tags:$score_tags,
        scores:{
          prompt_accepted:$prompt_accepted,
          parse_passed:$parse_passed,
          check_passed:$check_passed,
          oracle_tests_passed:$oracle_passed,
          semantic_output_matched:$semantic_output_matched,
          expected_diagnosis_identified:$expected_diagnosis_identified,
          repair_minimal:$repair_minimal,
          scope_precise:$scope_precise,
          required_evidence_produced:$required_evidence_produced
        },
        measurements:{
          tool_calls:$tool_calls,
          compiler_cycles:$compiler_cycles,
          input_tokens:0,
          output_tokens:0,
          wall_ms:$wall_ms,
          estimated_cost_usd:0,
          changed_files:$changed_files,
          changed_lines:$changed_lines,
          syntax_hallucination:false,
          stdlib_hallucination:false,
          invalid_tool_call:$invalid_tool_call,
          model_measurement_applicable:false
        },
        workspace:{
          retained:$retained,
          clean_before:$clean_before,
          path_exposed:false,
          trusted_fixture:true,
          os_isolation:false
        },
        evidence:{
          training_exclusion_id:$training_exclusion_id,
          initial_tree_digest:$initial_tree_digest,
          candidate_digest:$candidate_digest,
          final_tree_digest:$final_tree_digest,
          prompt_digest:$prompt_digest,
          step_evidence_digest:$step_evidence_digest
        },
        steps:$steps,
        issues:$issues
      }
    ')"
  case_result_path="$run_root/results/$case_id.json"
  printf '%s\n' "$case_result" > "$case_result_path"
  "$sley_bin" sley-contract validate \
    --schema sley.agent_bench.case_result.v0 "$case_result_path" \
    --schemas "$schema_root" --json >/dev/null \
    || die "$case_id result failed schema validation"
  printf '%s\n' "$case_result" >> "$results_jsonl"
  emit_event case_completed "$case_id" "$(jq -cn --arg status "$case_status" --arg digest "$(sha256_file "$case_result_path")" '{status:$status,result_digest:$digest}')"

  if [[ "$retain_workspaces" == "false" ]]; then
    rm -rf -- "$case_workspace"
  fi
done

passed_count="$(jq -s '[.[] | select(.status == "passed")] | length' "$results_jsonl")"
failed_count=$((case_count - passed_count))
run_status=passed
(( failed_count == 0 )) || run_status=failed
emit_event run_completed "" "$(jq -cn --arg status "$run_status" --argjson cases "$case_count" --argjson failed "$failed_count" '{status:$status,case_count:$cases,failed_count:$failed}')"

python3 - "$event_schema" "$events_path" <<'PY'
import json
import sys
from jsonschema import Draft202012Validator

schema_path, events_path = sys.argv[1:]
with open(schema_path, encoding="utf-8") as handle:
    schema = json.load(handle)
validator = Draft202012Validator(schema)
with open(events_path, encoding="utf-8") as handle:
    for line_number, line in enumerate(handle, 1):
        event = json.loads(line)
        errors = list(validator.iter_errors(event))
        if errors:
            raise SystemExit(f"event {line_number} failed schema validation: {errors[0].message}")
PY

event_stream_digest="$(sha256_file "$events_path")"
case_results_digest="$(sha256_file "$results_jsonl")"
family_counts="$(jq -s '
  {
    syntax_compilation:([.[] | select(.family == "syntax_compilation")] | length),
    semantic_implementation:([.[] | select(.family == "semantic_implementation")] | length),
    diagnosis_repair:([.[] | select(.family == "diagnosis_repair")] | length),
    translation_preservation:([.[] | select(.family == "translation_preservation")] | length),
    structural_tool_use:([.[] | select(.family == "structural_tool_use")] | length),
    repository_patch_integration:([.[] | select(.family == "repository_patch_integration")] | length)
  }
' "$results_jsonl")"

aggregate="$(jq -s \
  --arg schema "sley.agent_bench.aggregate.v0" \
  --arg status "$run_status" \
  --arg run_id "$run_id" \
  --arg suite_id "$suite_id" \
  --arg tier "$tier" \
  --arg mode "$mode" \
  --arg adapter_id "$adapter_id" \
  --arg sley_version "$sley_version" \
  --argjson passed_count "$passed_count" \
  --argjson failed_count "$failed_count" \
  --argjson family_counts "$family_counts" \
  --arg case_manifest_digest "$case_manifest_digest" \
  --arg run_manifest_digest "$run_manifest_digest" \
  --arg bootstrap_digest "$bootstrap_digest" \
  --arg event_stream_digest "$event_stream_digest" \
  --arg case_results_digest "$case_results_digest" '
    def rate_for($tag; $field):
      [.[] | select(.score_tags | index($tag) != null)] as $cases
      | ($cases | length) as $denominator
      | ([$cases[] | select(getpath($field) == true)] | length) as $numerator
      | if $denominator == 0 then
          {numerator:0,denominator:0,rate:null,wilson_95_low:null,wilson_95_high:null,applicable:false}
        else
          ($numerator / $denominator) as $p
          | 1.959963984540054 as $z
          | ($z * $z) as $z2
          | (1 + $z2 / $denominator) as $scale
          | (($p + $z2 / (2 * $denominator)) / $scale) as $center
          | ($z * (((($p * (1 - $p)) + $z2 / (4 * $denominator)) / $denominator) | sqrt) / $scale) as $margin
          | {
              numerator:$numerator,
              denominator:$denominator,
              rate:$p,
              wilson_95_low:([0, ($center - $margin)] | max),
              wilson_95_high:([1, ($center + $margin)] | min),
              applicable:true
            }
        end;
    def model_rate($field):
      [.[] | select(.measurements.model_measurement_applicable == true)] as $cases
      | ($cases | length) as $denominator
      | ([$cases[] | select(getpath($field) == true)] | length) as $numerator
      | if $denominator == 0 then
          {numerator:0,denominator:0,rate:null,wilson_95_low:null,wilson_95_high:null,applicable:false}
        else
          ($numerator / $denominator) as $p
          | 1.959963984540054 as $z
          | ($z * $z) as $z2
          | (1 + $z2 / $denominator) as $scale
          | (($p + $z2 / (2 * $denominator)) / $scale) as $center
          | ($z * (((($p * (1 - $p)) + $z2 / (4 * $denominator)) / $denominator) | sqrt) / $scale) as $margin
          | {
              numerator:$numerator,
              denominator:$denominator,
              rate:$p,
              wilson_95_low:([0, ($center - $margin)] | max),
              wilson_95_high:([1, ($center + $margin)] | min),
              applicable:true
            }
        end;
    {
      schema:$schema,
      status:$status,
      run_id:$run_id,
      suite_id:$suite_id,
      tier:$tier,
      mode:$mode,
      adapter:{kind:"deterministic_fixture",id:$adapter_id,model_measurement_applicable:false},
      sley_version:$sley_version,
      counts:{
        case_count:length,
        passed_count:$passed_count,
        failed_count:$failed_count,
        family_counts:$family_counts
      },
      rates:{
        parse_at_1:rate_for("parse"; ["scores","parse_passed"]),
        compile_at_1:rate_for("compile"; ["scores","check_passed"]),
        pass_at_1:rate_for("pass"; ["scores","oracle_tests_passed"]),
        repair_at_1:rate_for("repair"; ["scores","expected_diagnosis_identified"]),
        repository_patch_acceptance:rate_for("repository_patch"; ["scores","oracle_tests_passed"]),
        regression_free_success:rate_for("pass"; ["scores","scope_precise"]),
        syntax_hallucination:model_rate(["measurements","syntax_hallucination"]),
        stdlib_hallucination:model_rate(["measurements","stdlib_hallucination"]),
        invalid_tool_call:model_rate(["measurements","invalid_tool_call"])
      },
      resources:{
        tool_calls_total:(map(.measurements.tool_calls) | add // 0),
        compiler_cycles_total:(map(.measurements.compiler_cycles) | add // 0),
        wall_ms_total:(map(.measurements.wall_ms) | add // 0),
        input_tokens_total:(map(.measurements.input_tokens) | add // 0),
        output_tokens_total:(map(.measurements.output_tokens) | add // 0),
        estimated_cost_usd_total:(map(.measurements.estimated_cost_usd) | add // 0)
      },
      cases:.,
      evidence:{
        case_manifest_digest:$case_manifest_digest,
        run_manifest_digest:$run_manifest_digest,
        bootstrap_digest:$bootstrap_digest,
        event_stream_digest:$event_stream_digest,
        case_results_digest:$case_results_digest
      },
      issues:[]
    }
  ' "$results_jsonl")"

aggregate_path="$run_root/aggregate.json"
printf '%s\n' "$aggregate" > "$aggregate_path"
"$sley_bin" sley-contract validate \
  --schema sley.agent_bench.aggregate.v0 "$aggregate_path" \
  --schemas "$schema_root" --json >/dev/null \
  || die "aggregate failed schema validation"

printf '%s\n' "$aggregate"
[[ "$run_status" == "passed" ]]
