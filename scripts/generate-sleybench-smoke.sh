#!/usr/bin/env bash
set -euo pipefail

repo_root="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
destination="${1:-$repo_root/fixtures/sleybench/smoke-v0}"
[[ ! -e "$destination" ]] || {
  printf 'refusing to replace existing smoke fixtures: %s\n' "$destination" >&2
  exit 1
}

build_root="$(mktemp -d)"
trap 'rm -rf -- "$build_root"' EXIT
suite_root="$build_root/smoke-v0"
mkdir -p "$suite_root/cases"

sha256_file() {
  printf 'sha256:%s\n' "$(sha256sum "$1" | awk '{print $1}')"
}

tree_digest() {
  local root="$1"
  (
    cd "$root"
    find . -type f -print0 | sort -z | while IFS= read -r -d '' file; do
      printf '%s\0' "${file#./}"
      sha256sum "$file" | awk '{print $1}'
    done
  ) | sha256sum | awk '{print "sha256:" $1}'
}

training_digest() {
  local prompt="$1" workspace="$2"
  { sha256_file "$prompt"; tree_digest "$workspace"; } \
    | sha256sum | awk '{print "sha256:" $1}'
}

check_ok='[{"name":"check","argv":["sley","check","--json","main.sley"],"expected_exit":0,"expected_schema":"sley.diagnostics.report.v0","assertions":[{"pointer":"/status","value":"ok"}]}]'
ast_ok='[{"name":"ast","argv":["sley","ast","--json","main.sley"],"expected_exit":0,"expected_schema":"sley.ast.program.v0","assertions":[]}]'
empty_steps='[]'

jq -n '{schema:"sley.agent_bench.case_manifest.v0",suite_id:"sleybench-smoke-v0",tier:"smoke",version:"0.1",cases:[]}' \
  > "$suite_root/case-manifest.json"

append_case() {
  local id="$1" family="$2" tools="$3" preflight="$4" oracle="$5" tags="$6"
  local max_files="$7" max_lines="$8" owned="$9"
  local case_root="$suite_root/cases/$id" digest tmp
  digest="$(training_digest "$case_root/prompt.md" "$case_root/workspace")"
  tmp="$suite_root/case-manifest.tmp"
  jq \
    --arg id "$id" --arg family "$family" --arg digest "$digest" \
    --arg prompt "cases/$id/prompt.md" \
    --arg workspace "cases/$id/workspace" \
    --arg candidate "cases/$id/candidate" \
    --argjson tools "$tools" --argjson preflight "$preflight" \
    --argjson oracle "$oracle" --argjson tags "$tags" \
    --argjson max_files "$max_files" --argjson max_lines "$max_lines" \
    --argjson owned "$owned" '
      .cases += [{
        id:$id,
        family:$family,
        visibility:"public",
        sley_version:"1.2",
        prompt_path:$prompt,
        workspace_path:$workspace,
        candidate_path:$candidate,
        owned_paths:$owned,
        allowed_context:["SLEY_AI.md"],
        allowed_tools:$tools,
        limits:{wall_seconds:120,tool_calls:8,output_bytes:1048576,processes:256,workspace_bytes:1048576},
        policy:{kind:"fixture_candidate"},
        preflight:$preflight,
        oracle:{commands:$oracle},
        score_tags:$tags,
        minimality:{max_changed_files:$max_files,max_changed_lines:$max_lines},
        training_exclusion_id:$digest
      }]
    ' "$suite_root/case-manifest.json" > "$tmp"
  mv "$tmp" "$suite_root/case-manifest.json"
}

single_case() {
  local id="$1" family="$2" prompt="$3" initial="$4" candidate="$5"
  local tools="$6" preflight="$7" oracle="$8" tags="$9"
  local max_files="${10}" max_lines="${11}"
  local root="$suite_root/cases/$id"
  mkdir -p "$root/workspace" "$root/candidate"
  printf '%s\n' "$prompt" > "$root/prompt.md"
  printf '%s\n' "$(printf '%s' "$initial")" > "$root/workspace/main.sley"
  if [[ -n "$candidate" ]]; then
    printf '%s\n' "$(printf '%s' "$candidate")" > "$root/candidate/main.sley"
  fi
  append_case "$id" "$family" "$tools" "$preflight" "$oracle" "$tags" \
    "$max_files" "$max_lines" '["main.sley"]'
}

run_oracle() {
  local kind="$1" value="$2"
  jq -cn --arg kind "$kind" --argjson value "$value" '[
    {name:"check",argv:["sley","check","--json","main.sley"],expected_exit:0,expected_schema:"sley.diagnostics.report.v0",assertions:[{pointer:"/status",value:"ok"}]},
    {name:"run",argv:["sley","run","--json","main.sley"],expected_exit:0,expected_schema:"sley.run.report.v0",assertions:[{pointer:"/status",value:"passed"},{pointer:"/value/kind",value:$kind},{pointer:"/value/value",value:$value}]}
  ]'
}

repair_preflight() {
  jq -cn --arg id "$1" '[{name:"diagnose",argv:["sley","check","--json","main.sley"],expected_exit:1,expected_schema:"sley.diagnostics.report.v0",assertions:[{pointer:"/status",value:"error"},{pointer:"/diagnostics/0/id",value:$id}]}]'
}

# F1: syntax and compilation.
single_case sleybench-smoke-f1-001 syntax_compilation \
  'Complete a minimal Int-returning module that passes check and exposes an AST.' \
  $'module bench.f1_001\n' \
  $'module bench.f1_001\n\ntask main -> Int {\n  return 7\n}' \
  '["check","ast"]' "$empty_steps" "$(jq -cn --argjson a "$check_ok" --argjson b "$ast_ok" '$a + $b')" \
  '["parse","compile","pass"]' 1 8
single_case sleybench-smoke-f1-002 syntax_compilation \
  'Add a typed helper with one take and call it from main.' \
  $'module bench.f1_002\n' \
  $'module bench.f1_002\n\ntask double -> Int {\n  take value: Int\n  return value * 2\n}\n\ntask main -> Int {\n  return call double(4)\n}' \
  '["check","ast"]' "$empty_steps" "$(jq -cn --argjson a "$check_ok" --argjson b "$ast_ok" '$a + $b')" \
  '["parse","compile","pass"]' 1 12
single_case sleybench-smoke-f1-003 syntax_compilation \
  'Create a Bool task using a binding and equality expression.' \
  $'module bench.f1_003\n' \
  $'module bench.f1_003\n\ntask main -> Bool {\n  bind ready = true\n  return ready == true\n}' \
  '["check","ast"]' "$empty_steps" "$(jq -cn --argjson a "$check_ok" --argjson b "$ast_ok" '$a + $b')" \
  '["parse","compile","pass"]' 1 9
single_case sleybench-smoke-f1-004 syntax_compilation \
  'Create a Text task with a deterministic concatenation.' \
  $'module bench.f1_004\n' \
  $'module bench.f1_004\n\ntask main -> Text {\n  return "Sley" + "Bench"\n}' \
  '["check","ast"]' "$empty_steps" "$(jq -cn --argjson a "$check_ok" --argjson b "$ast_ok" '$a + $b')" \
  '["parse","compile","pass"]' 1 8

# F2: semantic implementation.
single_case sleybench-smoke-f2-001 semantic_implementation 'Implement main so it returns 42 by calling a typed double task.' \
  $'module bench.f2_001\n' \
  $'module bench.f2_001\n\ntask double -> Int {\n  take value: Int\n  return value * 2\n}\n\ntask main -> Int {\n  return call double(21)\n}' \
  '["check","run"]' "$empty_steps" "$(run_oracle Int 42)" '["compile","pass","semantic"]' 1 12
single_case sleybench-smoke-f2-002 semantic_implementation 'Implement the stated true branch so main deterministically returns 9.' \
  $'module bench.f2_002\n' \
  $'module bench.f2_002\n\ntask main -> Int {\n  if true {\n    return 9\n  } else {\n    return 0\n  }\n}' \
  '["check","run"]' "$empty_steps" "$(run_oracle Int 9)" '["compile","pass","semantic"]' 1 12
single_case sleybench-smoke-f2-003 semantic_implementation 'Implement main so the exact Text result is SleyBench.' \
  $'module bench.f2_003\n' \
  $'module bench.f2_003\n\ntask main -> Text {\n  return "Sley" + "Bench"\n}' \
  '["check","run"]' "$empty_steps" "$(run_oracle Text '"SleyBench"')" '["compile","pass","semantic"]' 1 8
single_case sleybench-smoke-f2-004 semantic_implementation 'Implement main so a Boolean conjunction evaluates to true.' \
  $'module bench.f2_004\n' \
  $'module bench.f2_004\n\ntask main -> Bool {\n  return true && true\n}' \
  '["check","run"]' "$empty_steps" "$(run_oracle Bool true)" '["compile","pass","semantic"]' 1 7

# F3: diagnosis and minimal repair.
single_case sleybench-smoke-f3-001 diagnosis_repair 'Diagnose the missing return and apply the smallest repair.' \
  $'module bench.f3_001\n\ntask main -> Int {\n  bind value = 41\n}' \
  $'module bench.f3_001\n\ntask main -> Int {\n  bind value = 41\n  return value\n}' \
  '["check","run"]' "$(repair_preflight MISSING_RETURN)" "$(run_oracle Int 41)" '["compile","pass","repair"]' 1 2
single_case sleybench-smoke-f3-002 diagnosis_repair 'Diagnose the unknown identifier and replace it with the intended literal.' \
  $'module bench.f3_002\n\ntask main -> Int {\n  return missing\n}' \
  $'module bench.f3_002\n\ntask main -> Int {\n  return 5\n}' \
  '["check","run"]' "$(repair_preflight UNKNOWN_IDENTIFIER)" "$(run_oracle Int 5)" '["compile","pass","repair"]' 1 2
single_case sleybench-smoke-f3-003 diagnosis_repair 'Diagnose the unknown return type and repair it to Int.' \
  $'module bench.f3_003\n\ntask main -> MissingType {\n  return 3\n}' \
  $'module bench.f3_003\n\ntask main -> Int {\n  return 3\n}' \
  '["check","run"]' "$(repair_preflight UNKNOWN_TYPE)" "$(run_oracle Int 3)" '["compile","pass","repair"]' 1 2
single_case sleybench-smoke-f3-004 diagnosis_repair 'Diagnose the call arity mismatch and supply the required argument.' \
  $'module bench.f3_004\n\ntask helper -> Int {\n  take value: Int\n  return value\n}\n\ntask main -> Int {\n  return call helper()\n}' \
  $'module bench.f3_004\n\ntask helper -> Int {\n  take value: Int\n  return value\n}\n\ntask main -> Int {\n  return call helper(6)\n}' \
  '["check","run"]' "$(repair_preflight CALL_ARITY_MISMATCH)" "$(run_oracle Int 6)" '["compile","pass","repair"]' 1 2

# F4: translation with explicit semantic oracles.
single_case sleybench-smoke-f4-001 translation_preservation 'Translate the pseudocode `return (6 * 7)` into Sley.' \
  $'module bench.f4_001\n' $'module bench.f4_001\n\ntask main -> Int {\n  return 6 * 7\n}' \
  '["check","run"]' "$empty_steps" "$(run_oracle Int 42)" '["compile","pass","semantic"]' 1 7
single_case sleybench-smoke-f4-002 translation_preservation 'Translate the expression `"forge" + "node"` while preserving its result.' \
  $'module bench.f4_002\n' $'module bench.f4_002\n\ntask main -> Text {\n  return "forge" + "node"\n}' \
  '["check","run"]' "$empty_steps" "$(run_oracle Text '"forgenode"')" '["compile","pass","semantic"]' 1 7
single_case sleybench-smoke-f4-003 translation_preservation 'Translate `1 + 1 == 2` into a Sley Bool task.' \
  $'module bench.f4_003\n' $'module bench.f4_003\n\ntask main -> Bool {\n  return 1 + 1 == 2\n}' \
  '["check","run"]' "$empty_steps" "$(run_oracle Bool true)" '["compile","pass","semantic"]' 1 7
single_case sleybench-smoke-f4-004 translation_preservation 'Translate an if/else that returns 12 when its fixed condition is true.' \
  $'module bench.f4_004\n' $'module bench.f4_004\n\ntask main -> Int {\n  if true {\n    return 12\n  } else {\n    return 4\n  }\n}' \
  '["check","run"]' "$empty_steps" "$(run_oracle Int 12)" '["compile","pass","semantic"]' 1 12

# F5: structural tool selection and schema assertions; no candidate edit.
for number in 001 002 003 004; do
  id="sleybench-smoke-f5-$number"
  source=$'module bench.f5_'"$number"$'\n\ntask helper -> Int {\n  return 2\n}\n\ntask main -> Int {\n  return call helper()\n}'
  root="$suite_root/cases/$id"
  mkdir -p "$root/workspace" "$root/candidate"
  printf 'Use the requested structural report and preserve the workspace unchanged.\n' > "$root/prompt.md"
  printf '%s\n' "$source" > "$root/workspace/main.sley"
  case "$number" in
    001) tools='["ast"]'; oracle='[{"name":"ast","argv":["sley","ast","--json","main.sley"],"expected_exit":0,"expected_schema":"sley.ast.program.v0","assertions":[]}]' ;;
    002) tools='["query"]'; oracle='[{"name":"query","argv":["sley","query","--json","--kind","tasks","main.sley"],"expected_exit":0,"expected_schema":"sley.query.report.v0","assertions":[{"pointer":"/tasks/0/name","value":"helper"}]}]' ;;
    003) tools='["graph"]'; oracle='[{"name":"graph","argv":["sley","graph","--json","main.sley"],"expected_exit":0,"expected_schema":"sley.symbol_graph.v0","assertions":[]}]' ;;
    004) tools='["plan"]'; oracle='[{"name":"plan","argv":["sley","plan","--json","--graft-templates","main.sley"],"expected_exit":0,"expected_schema":"sley.edit_plan.report.v0","assertions":[{"pointer":"/status","value":"ready"}]}]' ;;
  esac
  append_case "$id" structural_tool_use "$tools" "$empty_steps" "$oracle" '["pass","structural"]' 0 0 '["main.sley"]'
done

project_case() {
  local id="$1" prompt="$2" initial_main="$3" candidate_main="$4" expected="$5" max_files="$6" max_lines="$7"
  local root="$suite_root/cases/$id"
  mkdir -p "$root/workspace/src/app" "$root/candidate/src/app"
  printf '%s\n' "$prompt" > "$root/prompt.md"
  printf '%s\n' $'[project]\nname = "sleybench-smoke"\nroot = "src"\nentry = "app.main"' > "$root/workspace/sley.toml"
  printf '%s\n' "$initial_main" > "$root/workspace/src/app/main.sley"
  if [[ -n "$candidate_main" ]]; then
    printf '%s\n' "$candidate_main" > "$root/candidate/src/app/main.sley"
  fi
  oracle="$(jq -cn --argjson expected "$expected" '[
    {name:"project-check",argv:["sley","check","--json","."],expected_exit:0,expected_schema:"sley.diagnostics.report.v0",assertions:[{pointer:"/status",value:"ok"}]},
    {name:"project-run",argv:["sley","run","--json","."],expected_exit:0,expected_schema:"sley.run.report.v0",assertions:[{pointer:"/status",value:"passed"},{pointer:"/value/kind",value:"Int"},{pointer:"/value/value",value:$expected}]},
    {name:"project-query",argv:["sley","query","--json","--kind","tasks","."],expected_exit:0,expected_schema:"sley.query.report.v0",assertions:[{pointer:"/tasks/0/name",value:"main"}]}
  ]')"
  append_case "$id" repository_patch_integration '["check","run","query"]' "$empty_steps" "$oracle" \
    '["compile","pass","repository_patch"]' "$max_files" "$max_lines" '["src/app/main.sley"]'
}

project_case sleybench-smoke-f6-001 'Change only the project entry task so it returns 2.' \
  $'module app.main\n\ntask main -> Int {\n  return 1\n}' \
  $'module app.main\n\ntask main -> Int {\n  return 2\n}' 2 1 2
project_case sleybench-smoke-f6-002 'Change only the project entry task so it returns the specified arithmetic result.' \
  $'module app.main\n\ntask main -> Int {\n  return 3\n}' \
  $'module app.main\n\ntask main -> Int {\n  return 3 * 3\n}' 9 1 2
project_case sleybench-smoke-f6-003 'Inspect the project. It already returns 8, so make no edit.' \
  $'module app.main\n\ntask main -> Int {\n  return 8\n}' '' 8 0 0
project_case sleybench-smoke-f6-004 'Repair the project entry task without changing project configuration.' \
  $'module app.main\n\ntask main -> Int {\n  return 1\n}' \
  $'module app.main\n\ntask main -> Int {\n  return 4 + 6\n}' 10 1 2

# Git cannot retain empty candidate directories. No-op cases are represented by
# zero minimality limits and an absent candidate path in a clean checkout.
find "$suite_root/cases" -depth -type d -empty -delete

mkdir -p "$suite_root/malformed"
jq '.cases = [.cases[0]] | .unexpected = true' "$suite_root/case-manifest.json" \
  > "$suite_root/malformed/case-manifest-unknown-field.json"
jq '.cases = [.cases[0]] | .cases[0].limits.wall_seconds = 0' "$suite_root/case-manifest.json" \
  > "$suite_root/malformed/case-manifest-zero-limit.json"

case_digest="$(sha256_file "$suite_root/case-manifest.json")"
bootstrap_digest="$(sha256_file "$repo_root/SLEY_AI.md")"
jq -n --arg case_digest "$case_digest" --arg bootstrap_digest "$bootstrap_digest" '{
  schema:"sley.agent_bench.run_manifest.v0",
  run_id:"sleybench-smoke-v0-deterministic",
  suite_id:"sleybench-smoke-v0",
  case_manifest_path:"case-manifest.json",
  case_manifest_digest:$case_digest,
  mode:"K4",
  adapter:{kind:"deterministic_fixture",id:"deterministic-fixture-v0",trusted_fixtures_only:true},
  bootstrap:{path:"SLEY_AI.md",digest:$bootstrap_digest},
  tool_allowlist:["ast","check","graph","plan","query","run"],
  retain_workspaces:false
}' > "$suite_root/run-manifest.json"

mkdir -p "$(dirname "$destination")"
mv "$suite_root" "$destination"
printf 'generated %s with %s cases\n' "$destination" "$(jq '.cases | length' "$destination/case-manifest.json")"
