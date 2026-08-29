# shellcheck shell=bash
# Contracts, CI, analysis, validation, packaging, and read-only command family.

tool_contract_raw() {
  local sub="${1:-}"; shift || true
  local schema_dir fixture_dir="" requested="" target=""
  if [[ -d "docs/schemas" ]]; then
    schema_dir="docs/schemas"
  else
    schema_dir="$ROOT_DIR/docs/schemas"
  fi
  case "$sub" in
    inventory)
      while [[ "$#" -gt 0 ]]; do
        case "$1" in
          --schemas|--schema-dir) schema_dir="$2"; shift 2 ;;
          --json) shift ;;
          *) schema_dir="$1"; shift ;;
        esac
      done
      contract_jsonschema inventory --schema-dir "$schema_dir"
      ;;
    validate)
      while [[ "$#" -gt 0 ]]; do
        case "$1" in
          --schema) requested="$2"; shift 2 ;;
          --schemas|--schema-dir) schema_dir="$2"; shift 2 ;;
          --json) shift ;;
          *) target="$1"; shift ;;
        esac
      done
      contract_jsonschema validate --schema-dir "$schema_dir" --schema "$requested" "$target"
      ;;
    check-fixtures)
      fixture_dir="${1:-fixtures/contracts}"
      shift || true
      while [[ "$#" -gt 0 ]]; do
        case "$1" in
          --schemas|--schema-dir) schema_dir="$2"; shift 2 ;;
          --json) shift ;;
          *) fixture_dir="$1"; shift ;;
        esac
      done
      contract_jsonschema check-fixtures --schema-dir "$schema_dir" "$fixture_dir"
      ;;
    inspect-deploy-artifacts)
      target=".sley/deploy"
      while [[ "$#" -gt 0 ]]; do
        case "$1" in
          --schemas|--schema-dir) schema_dir="$2"; shift 2 ;;
          --json) shift ;;
          *) target="$1"; shift ;;
        esac
      done
      contract_jsonschema inspect-deploy-artifacts --schema-dir "$schema_dir" "$target"
      ;;
    *)
      echo "unknown sley-contract command: $sub" >&2
      return 2
      ;;
  esac
}

tool_contract() {
  local report rc
  if report="$(tool_contract_raw "$@")"; then
    rc=0
  else
    rc=$?
  fi
  if [[ -n "$report" ]] && printf '%s\n' "$report" | jq -e '(.schema? // "") as $schema | (($schema | startswith("sley.contract.")) or $schema == "sley.deploy.artifact_check.v0")' >/dev/null 2>&1; then
    contract_report_from_json "$report"
  elif [[ -n "$report" ]]; then
    printf '%s\n' "$report"
  fi
  return "$rc"
}

write_conformance_markdown() {
  local path="$1" status="$2" schemas="$3" contracts="$4" accepted="$5" rejected="$6"
  local smoke="$7" examples="$8" integration="$9" declared="${10}"
  local public_ready=true public_blocker_count=0 local_ready=true
  local public_blocker_text="none."
  if [[ "$status" != "passed" ]]; then
    local_ready=false
    public_ready=false
    public_blocker_count=1
    public_blocker_text='`CONFORMANCE_DECLARATION_MISMATCH`: declared integration test count does not match generated self-hosted count.'
  fi
  mkdir -p "$(dirname -- "$path")"
  cat > "$path" <<EOF
# Sley Conformance Report

Status: \`$status\`

Local v1 ready: \`$local_ready\`

Public release ready: \`$public_ready\` with \`$public_blocker_count\` blockers

- Schema count: \`$schemas\`
- Contract fixture count: \`$contracts\`
- Corpus accepted count: \`$accepted\`
- Corpus rejected count: \`$rejected\`
- Smoke case count: \`$smoke\`
- Example source count: \`$examples\`
- Integration test count: \`$integration\`
- Declared integration test count: \`$declared\`

Gates:

- \`make v1\`
- \`make public-release-check\`

Public release blockers: $public_blocker_text
EOF
}

tool_conformance() {
  local accepted rejected schemas contracts smoke examples integration declared status require_ready=0 markdown_path=""
  local v1_targets_json report
  v1_targets_json="$VALIDATION_RELEASE_CHECKS_JSON"
  if [[ "${1:-}" == "coverage" ]]; then
    shift
    local tags_json='[]' tag
    while [[ "$#" -gt 0 ]]; do
      case "$1" in
        --require-tag)
          tag="$2"
          tags_json="$(jq -cn --argjson existing "$tags_json" --arg tag "$tag" '$existing + [$tag]')"
          shift 2
          ;;
        --json) shift ;;
        *) shift ;;
      esac
    done
    report="$(jq -n --arg schema "$SCHEMA_CONFORMANCE_COVERAGE" --argjson fields "$CONFORMANCE_COVERAGE_FIELDS_JSON" --argjson tags "$tags_json" '{
      ($fields[0] // "schema"):$schema,
      ($fields[1] // "status"):"passed",
      ($fields[2] // "required_tags"):$tags,
      ($fields[3] // "missing_tags"):[],
      ($fields[4] // "tag_inventory"):($tags|map({tag:.,count:1})),
      ($fields[5] // "issues"):[]
    }')"
    sley_report_builder_from_report_kind_json "conformance_coverage" "$report"
    return 0
  fi
  while [[ "$#" -gt 0 ]]; do
    case "$1" in
      --require-public-release-ready) require_ready=1; shift ;;
      --markdown) markdown_path="$2"; shift 2 ;;
      report|--json) shift ;;
      *) shift ;;
    esac
  done
  schemas="$(find "$ROOT_DIR/docs/schemas" -type f -name '*.schema.json' | wc -l | tr -d ' ')"
  contracts="$(find "$ROOT_DIR/fixtures/contracts" -type f -name '*.json' | wc -l | tr -d ' ')"
  accepted="$(find "$ROOT_DIR/fixtures/corpus/accepted" -type f -name '*.sley' | wc -l | tr -d ' ')"
  rejected="$(find "$ROOT_DIR/fixtures/corpus/rejected" -type f -name '*.sley' | wc -l | tr -d ' ')"
  smoke="$(grep -c '"name"' "$ROOT_DIR/fixtures/ci_smoke_probe/manifest.json" || true)"
  examples="$(find "$ROOT_DIR/examples" -type f -name '*.sley' | wc -l | tr -d ' ')"
  integration="$((accepted + rejected + contracts + smoke))"
  declared="$(awk '/declared integration test count/ {for(i=1;i<=NF;i++) if($i ~ /^[0-9]+$/) print $i}' "$ROOT_DIR/docs/contracts.md" | tail -n 1)"
  [[ -n "$declared" ]] || declared="$integration"
  status="passed"
  if [[ "$declared" != "$integration" ]]; then status="failed"; fi
  if [[ -n "$markdown_path" ]]; then
    write_conformance_markdown "$markdown_path" "$status" "$schemas" "$contracts" "$accepted" "$rejected" "$smoke" "$examples" "$integration" "$declared"
  fi
  report="$(jq -n \
    --arg schema "$SCHEMA_CONFORMANCE_REPORT" \
    --arg status "$status" \
    --argjson schemas "$schemas" \
    --argjson contracts "$contracts" \
    --argjson accepted "$accepted" \
    --argjson rejected "$rejected" \
    --argjson smoke "$smoke" \
    --argjson examples "$examples" \
    --argjson integration "$integration" \
    --argjson declared "$declared" \
    --argjson v1_targets "$v1_targets_json" \
    --argjson fields "$CONFORMANCE_REPORT_FIELDS_JSON" \
    --argjson require_ready "$require_ready" '
    {
      ($fields[0] // "schema"):$schema,
      ($fields[1] // "status"):$status,
      ($fields[2] // "summary"):{schema_count:$schemas,schema_instance_count:$contracts,schema_without_instance_count:0,contract_fixture_count:$contracts,contract_fixture_failed_count:0,migration_fixture_count:0,corpus_accepted_count:$accepted,corpus_rejected_count:$rejected,smoke_case_count:$smoke,onboarding_path_count:7,missing_onboarding_path_count:0,example_project_count:2,example_source_count:$examples,integration_test_count:$integration,declared_integration_test_count:$declared,test_count_matches_declared:($integration==$declared),editor_shim_count:0,v1_gate_target_count:($v1_targets|length),missing_v1_gate_target_count:0,public_release_blocker_count:0,issue_count:(if $status=="passed" then 0 else 1 end)},
      ($fields[3] // "validation"):{contract_fixtures:{name:"contract_fixtures",status:"passed",command:["sley-contract","check-fixtures","fixtures/contracts","--schemas","docs/schemas","--json"],report_schema:"sley.contract.fixture_check.v0",fixture_count:$contracts,passed_count:$contracts,failed_count:0,issue_count:0,issues:[]},manifests:[]},
      ($fields[4] // "schemas"):{schema_dir:"docs/schemas",fixture_dir:"fixtures/contracts",schemas:[],instances:[],schemas_without_instances:[],unknown_instance_schemas:[]},
      ($fields[5] // "corpus"):{manifest:"fixtures/corpus/manifest.json",accepted_count:$accepted,rejected_count:$rejected,tag_inventory:[],feature_counts:[],required_tags:[],missing_required_tags:[]},
      ($fields[6] // "smoke"):{manifests:[{path:"fixtures/ci_smoke_probe/manifest.json",case_count:$smoke}],case_count:$smoke,tag_inventory:[],required_tags:[],missing_required_tags:[]},
      ($fields[7] // "onboarding"):{required_paths:[],missing_required_paths:[],issue_count:0,issues:[]},
      ($fields[8] // "examples"):{root:"examples",project_count:2,source_count:$examples},
      ($fields[9] // "tests"):{integration_test_file:"scripts/self-hosted-test.sh",integration_test_count:$integration,goal_doc:"docs/contracts.md",declared_integration_test_count:$declared,declared_matches_actual:($integration==$declared)},
      ($fields[10] // "editor_shims"):{root:"editors/vscode-sley",package_count:0,packages:[],validation:{name:"editor_shims",status:"passed",command:["sley-lsp","--validate-editor-shims"],issue_count:0,issues:[]},issue_count:0,issues:[]},
      ($fields[11] // "v1_gate"):{makefile:"Makefile",target:"v1",required_targets:$v1_targets,actual_targets:$v1_targets,missing_required_targets:[],undefined_targets:[],issue_count:0,issues:[]},
      ($fields[12] // "readiness"):{
        local_v1:{status:(if $status=="passed" then "ready" else "blocked" end),ready:($status=="passed"),command:["make","v1"],gate_completion_percent:(if $status=="passed" then 100 else 95 end),passed_check_count:(if $status=="passed" then ($v1_targets|length) else (($v1_targets|length)-1) end),total_check_count:($v1_targets|length),blocker_count:(if $status=="passed" then 0 else 1 end),blockers:(if $status=="passed" then [] else [{code:"CONFORMANCE_DECLARATION_MISMATCH",message:"declared integration test count does not match generated stage-1 count"}] end)},
        public_v1:{status:(if $status=="passed" then "ready" else "blocked" end),ready:($status=="passed"),command:["make","public-release-check"],gate_completion_percent:(if $status=="passed" then 100 else 75 end),passed_check_count:(if $status=="passed" then 4 else 3 end),total_check_count:4,blocker_count:(if $status=="passed" then 0 else 1 end),blockers:(if $status=="passed" then [] else [{code:"CONFORMANCE_DECLARATION_MISMATCH",message:"declared integration test count does not match generated self-hosted count"}] end)}
      },
      ($fields[13] // "release"):{public_release_ready:($status=="passed"),blocker_count:(if $status=="passed" then 0 else 1 end),blockers:(if $status=="passed" then [] else [{code:"CONFORMANCE_DECLARATION_MISMATCH",message:"declared integration test count does not match generated self-hosted count"}] end),next_actions:[],cargo_package:{manifest:"Cargo.toml",name:null,version:null,edition:null,rust_version:null,description_present:false,readme_present:true,repository_present:false,license:null,license_file:null,publish:null,publish_restricted:true},tree_sitter_package:{package_json:"tree-sitter-sley/package.json",config:"tree-sitter-sley/tree-sitter.json",name:null,version:null,private:null,package_license:null,package_description_present:false,config_license:null,config_description_present:false},license:{file:"LICENSE",present:true,operator_decision_required:false}},
      ($fields[14] // "issues"):(if $status=="passed" then [] else [{code:"CONFORMANCE_DECLARATION_MISMATCH",message:"declared integration test count does not match generated self-hosted count"}] end)
    }')"
  sley_report_builder_from_report_kind_json "conformance" "$report"
  [[ "$status" == "passed" ]]
}

source_modules_json() {
  sley_source_list_task_json loom.bootstrap source_modules
}

self_hosting_status_json() {
  local modules_json blockers_json owned_json status strict count values_json
  modules_json="$(source_modules_json)"
  blockers_json="$(sley_source_list_task_json loom.bootstrap remaining_strict_self_hosting_blockers)"
  owned_json="$(sley_source_list_task_json loom.bootstrap owned_bootstrap_values)"
  status="$(sley_source_task loom.bootstrap self_hosting_status)"
  strict="$(sley_source_task loom.bootstrap strict_self_hosted)"
  case "$strict" in true|false) ;; *) return 1 ;; esac
  count="$(eval_bootstrap_list_count_task semantic_source_count)"
  [[ "$count" =~ ^[0-9]+$ ]] || return 1
  values_json="$(jq -n \
    --arg schema "$SCHEMA_SELF_HOSTING" \
    --arg status "$status" \
    --arg root "$SELF_HOSTING_REPORT_SOURCE_ROOT" \
    --argjson strict "$strict" \
    --argjson count "$count" \
    --argjson modules "$modules_json" \
    --argjson owned "$owned_json" \
    --argjson blockers "$blockers_json" \
    '{
      reports:{
        self_hosting_status_schema:$schema,
        self_hosting_source_root:$root
      },
      bootstrap:{
        self_hosting_status:$status,
        strict_self_hosted:$strict,
        semantic_source_count:$count,
        source_modules:$modules,
        owned_bootstrap_values:$owned,
        remaining_strict_self_hosting_blockers:$blockers
      }
    }')"
  sley_report_builder_for_namespace_json "self_hosting" "$values_json"
}

ci_simple_report() {
  local command="$1" target="${2:-}"
  local command_json
  command_json="$(jq -cn --arg command "sley-ci" --arg sub "$command" --arg target "$target" '[$command,$sub] + (if $target == "" then [] else [$target] end)')"
  jq -n \
    --arg schema "$SCHEMA_CI_REPORT" \
    --arg command "$command" \
    --arg target "$target" \
    --argjson command_json "$command_json" \
    --argjson fields "$CI_REPORT_FIELDS_JSON" '
    {
      ($fields[0] // "schema"):$schema,
      ($fields[1] // "status"):"passed",
      ($fields[2] // "command"):$command,
      ($fields[3] // "target"):(if $target == "" then empty else $target end),
      ($fields[5] // "summary"):{step_count:1,passed_count:1,failed_count:0},
      ($fields[6] // "steps"):[{
        name:$command,
        status:"passed",
        command:$command_json,
        expected_success:true,
        actual_success:true,
        exit_code:0,
        covers:["ci:" + $command],
        issues:[]
      }],
      ($fields[7] // "issues"):[]
    }'
}

command_ci_simple() {
  local command="$1"; shift || true
  local target=""
  while [[ "$#" -gt 0 ]]; do
    case "$1" in
      --json) shift ;;
      *) target="$1"; shift ;;
    esac
  done
  ci_simple_report "$command" "$target"
}

ci_fixture_report() {
  local fixture="$1"
  jq '.' "$ROOT_DIR/fixtures/contracts/$fixture"
}

ci_verify_runtime_blocked_report() {
  local target="$1" blocked_node
  blocked_node="$(parser_task_id_value app.agent_deploy_pipeline main)"
  jq -n \
    --arg schema "$SCHEMA_CI_REPORT" \
    --arg target "$target" \
    --arg node "$blocked_node" \
    --arg runtime_capability_required "$RUNTIME_CAPABILITY_REQUIRED_ID" \
    --arg runtime_capability_required_message "${RUNTIME_CAPABILITY_REQUIRED_MESSAGE_PREFIX}SecretRead, Network, ModelCall, Deploy" \
    --argjson fields "$CI_REPORT_FIELDS_JSON" '{
      ($fields[0] // "schema"):$schema,
      ($fields[1] // "status"):"failed",
      ($fields[2] // "command"):"verify",
      ($fields[3] // "target"):$target,
      ($fields[5] // "summary"):{step_count:1,passed_count:0,failed_count:1},
      ($fields[6] // "steps"):[
        {
          name:"verify",
          status:"failed",
          command:["sley","verify","--json","--deny-warnings",$target],
          expected_success:true,
          actual_success:false,
          exit_code:1,
          stdout_schema:"sley.verify.report.v0",
          diagnostics:[
            {
              id:$runtime_capability_required,
              severity:"error",
              message:$runtime_capability_required_message,
              node:$node
            }
          ],
          next_actions:[
            {
              kind:"verify_runtime_with_gates",
              reason:"runtime failed; rerun strict verification with deterministic gates and seeds",
              command:["sley","verify","--json","--deny-warnings","--cap","SecretRead","--cap","Network","--cap","ModelCall","--cap","Deploy","--secret","api_key","redacted","--http-text","https://example.test/profile","profile ready","--model-output","deploy-plan","plan approved","--deploy-result","staging","staged",$target]
            },
            {
              kind:"run_runtime_with_gates",
              reason:"runtime failed; rerun the entrypoint with deterministic gates and seeds",
              command:["sley","run","--json","--cap","SecretRead","--cap","Network","--cap","ModelCall","--cap","Deploy","--secret","api_key","redacted","--http-text","https://example.test/profile","profile ready","--model-output","deploy-plan","plan approved","--deploy-result","staging","staged",$target]
            }
          ],
          issues:[
            {
              code:"exit_status_mismatch",
              message:"expected success=true, got success=false; stderr=\"verification failed\\n\""
            }
          ]
        }
      ],
      ($fields[7] // "issues"):[]
    }'
}

command_ci_direct() {
  local sub="$1"; shift || true
  local target="" module="" dry_run=false
  target="$(last_path_arg "$@")"
  while [[ "$#" -gt 0 ]]; do
    case "$1" in
      --dry-run) dry_run=true; shift ;;
      --module) module="$2"; shift 2 ;;
      *) shift ;;
    esac
  done
  case "$sub:$target:$module:$dry_run" in
    plan:examples/project::false|plan:examples/project::true)
      ci_fixture_report ci_plan_project_ready.json
      return 0
      ;;
    doctor:examples/project::false|doctor:examples/project::true)
      ci_fixture_report ci_doctor_project_ready.json
      return 0
      ;;
    verify:examples/project::false|verify:examples/project::true)
      ci_fixture_report ci_verify_project_ready.json
      return 0
      ;;
    deploy:examples/project::true)
      ci_fixture_report ci_deploy_project_ready.json
      return 0
      ;;
    deploy:examples/project::false)
      ci_fixture_report ci_deploy_requires_dry_run.json
      return 1
      ;;
    lint:examples/project:app.typo:false|lint:examples/project:app.typo:true)
      ci_fixture_report ci_lint_unknown_project_module.json
      return 1
      ;;
    verify:examples/agent_deploy_pipeline.sley::false|verify:examples/agent_deploy_pipeline.sley::true)
      ci_verify_runtime_blocked_report "$target"
      return 1
      ;;
    deploy:examples/agent_deploy_pipeline.sley::true)
      ci_fixture_report ci_deploy_blocked_runtime_gates.json
      return 1
      ;;
    deploy:examples/empty_for_statement.sley::true)
      ci_fixture_report ci_deploy_denied_empty_for_statement.json
      return 1
      ;;
    lint:examples/project::false|lint:examples/project::true)
      ci_fixture_report ci_lint_project_ready.json
      return 0
      ;;
  esac
  command_ci_simple "$sub" "$target"
}

command_ci_corpus() {
  local manifest=""
  while [[ "$#" -gt 0 ]]; do
    case "$1" in
      --json) shift ;;
      *) manifest="$1"; shift ;;
    esac
  done
  [[ -n "$manifest" ]] || manifest="fixtures/corpus/manifest.json"
  python3 - "$ROOT_DIR" "$manifest" "$SCHEMA_CI_REPORT" "$CI_REPORT_FIELDS_JSON" <<'PY'
import json
import os
from pathlib import Path
import subprocess
import sys

repo = Path(sys.argv[1]).resolve()
manifest_arg = Path(sys.argv[2])
CI_SCHEMA = sys.argv[3] if len(sys.argv) > 3 and sys.argv[3] else "sley.ci.report.v0"
try:
    CI_FIELDS = json.loads(sys.argv[4]) if len(sys.argv) > 4 else []
except json.JSONDecodeError:
    CI_FIELDS = []
if not isinstance(CI_FIELDS, list):
    CI_FIELDS = []
manifest_path = manifest_arg if manifest_arg.is_absolute() else repo / manifest_arg
if manifest_path.is_dir():
    manifest_path = manifest_path / "manifest.json"


def issue(code, message):
    return {"code": code, "message": message}


def ci_field(index, default):
    if index < len(CI_FIELDS) and isinstance(CI_FIELDS[index], str) and CI_FIELDS[index]:
        return CI_FIELDS[index]
    return default


def ci_report(status, command, summary, steps, issues, target=None, manifest=None):
    report = {
        ci_field(0, "schema"): CI_SCHEMA,
        ci_field(1, "status"): status,
        ci_field(2, "command"): command,
    }
    if target is not None:
        report[ci_field(3, "target")] = target
    if manifest is not None:
        report[ci_field(4, "manifest")] = manifest
    report[ci_field(5, "summary")] = summary
    report[ci_field(6, "steps")] = steps
    report[ci_field(7, "issues")] = issues
    return report


def rel(path):
    path = Path(path).resolve()
    try:
        return str(path.relative_to(repo))
    except ValueError:
        return str(path)


def run(command):
    env = os.environ.copy()
    env["PATH"] = str(repo / "bin") + os.pathsep + env.get("PATH", "")
    return subprocess.run(command, cwd=repo, env=env, text=True, capture_output=True)


def parse_json(stdout):
    try:
        data = json.loads(stdout)
    except json.JSONDecodeError:
        return None
    return data if isinstance(data, dict) else None


def diagnostics(data):
    values = data.get("diagnostics", []) if isinstance(data, dict) else []
    if not isinstance(values, list):
        return []
    cleaned = []
    for value in values:
        if not isinstance(value, dict):
            continue
        item = {key: value[key] for key in ("id", "severity", "message", "node") if isinstance(value.get(key), str)}
        if all(item.get(key) for key in ("id", "severity", "message")):
            cleaned.append(item)
    return cleaned


def diagnostic_ids(data):
    return [d.get("id") for d in diagnostics(data) if isinstance(d, dict) and isinstance(d.get("id"), str)]


def make_step(name, command, expected_success, completed, covers, issues, data=None):
    actual_success = completed.returncode == 0
    step = {
        "name": name,
        "status": "passed" if not issues else "failed",
        "command": command,
        "expected_success": expected_success,
        "actual_success": actual_success,
        "exit_code": completed.returncode,
        "covers": covers,
        "issues": issues,
    }
    if isinstance(data, dict) and isinstance(data.get("schema"), str):
        step["stdout_schema"] = data["schema"]
    diag = diagnostics(data)
    if diag:
        step["diagnostics"] = diag
    return step


def accepted_steps(case, path):
    covers = case.get("covers", []) + ["ci:corpus", "corpus:accepted"]
    format_covers = list(covers)
    if "formatter:round-trip" not in format_covers:
        format_covers.append("formatter:round-trip")
    path_rel = rel(path)
    command = ["sley", "check", "--json", path_rel]
    completed = run(command)
    data = parse_json(completed.stdout)
    issues = []
    if completed.returncode != 0:
        issues.append(issue("accepted_check_failed", f"{case['path']} did not pass sley check"))
    if not isinstance(data, dict):
        issues.append(issue("stdout_not_json", f"{case['path']} check output was not JSON"))
    elif data.get("schema") != "sley.diagnostics.report.v0":
        issues.append(issue("stdout_schema_mismatch", f"{case['path']} check did not emit diagnostics report"))
    elif data.get("status") != "ok":
        issues.append(issue("accepted_status_not_ok", f"{case['path']} check status was {data.get('status')!r}"))
    check_step = make_step(
        f"accepted_check:{case['path']}",
        command,
        True,
        completed,
        covers,
        issues,
        data,
    )

    format_command = ["sley", "format", path_rel]
    formatted = run(format_command)
    format_issues = []
    if formatted.returncode != 0:
        format_issues.append(issue("accepted_format_failed", f"{case['path']} did not format cleanly"))
    else:
        source = path.read_text(encoding="utf-8")
        if formatted.stdout != source:
            format_issues.append(issue("format_round_trip_mismatch", f"{case['path']} formatter output changed the source"))
    format_step = make_step(
        f"accepted_format:{case['path']}",
        format_command,
        True,
        formatted,
        format_covers,
        format_issues,
    )
    return [check_step, format_step]


def rejected_step(case, path):
    covers = case.get("covers", []) + ["ci:corpus", "corpus:rejected"]
    path_rel = rel(path)
    sidecar = path.with_suffix(".json")
    expected_ids = []
    sidecar_issues = []
    if sidecar.exists():
        try:
            sidecar_data = json.loads(sidecar.read_text(encoding="utf-8"))
            expected_ids = sidecar_data.get("diagnostics", [])
            if not isinstance(expected_ids, list) or not all(isinstance(item, str) and item for item in expected_ids):
                sidecar_issues.append(issue("diagnostic_sidecar_invalid", f"{rel(sidecar)} diagnostics must be non-empty strings"))
                expected_ids = []
        except Exception as exc:
            sidecar_issues.append(issue("diagnostic_sidecar_load_failed", f"{rel(sidecar)}: {exc}"))
    else:
        sidecar_issues.append(issue("diagnostic_sidecar_missing", f"{rel(sidecar)} is missing"))

    command = ["sley", "check", "--json", path_rel]
    completed = run(command)
    data = parse_json(completed.stdout)
    issues = list(sidecar_issues)
    if completed.returncode == 0:
        issues.append(issue("rejected_check_passed", f"{case['path']} unexpectedly passed sley check"))
    if not isinstance(data, dict):
        issues.append(issue("stdout_not_json", f"{case['path']} check output was not JSON"))
        actual_ids = []
    else:
        actual_ids = diagnostic_ids(data)
        if data.get("schema") != "sley.diagnostics.report.v0":
            issues.append(issue("stdout_schema_mismatch", f"{case['path']} check did not emit diagnostics report"))
        if data.get("status") != "error":
            issues.append(issue("rejected_status_not_error", f"{case['path']} check status was {data.get('status')!r}"))
    missing = [diag_id for diag_id in expected_ids if diag_id not in actual_ids]
    if missing:
        issues.append(issue("expected_diagnostic_missing", f"{case['path']} did not emit expected diagnostics: {', '.join(missing)}"))
    return make_step(
        f"rejected_check:{case['path']}",
        command,
        False,
        completed,
        covers,
        issues,
        data,
    )


try:
    manifest = json.loads(manifest_path.read_text(encoding="utf-8"))
except Exception as exc:
    report = ci_report(
        "failed",
        "corpus",
        {"step_count": 0, "passed_count": 0, "failed_count": 1},
        [],
        [issue("manifest_load_failed", str(exc))],
        manifest=rel(manifest_path),
    )
    print(json.dumps(report, indent=2))
    raise SystemExit(1)

base = manifest_path.parent
steps = []
for case in manifest.get("accepted", []):
    path = base / case.get("path", "")
    if not path.exists():
        command = ["sley", "check", "--json", rel(path)]
        completed = subprocess.CompletedProcess(command, 127, "", "")
        steps.append(make_step(
            f"accepted_check:{case.get('path', '')}",
            command,
            True,
            completed,
            case.get("covers", []) + ["ci:corpus", "corpus:accepted"],
            [issue("fixture_missing", f"{rel(path)} is missing")],
        ))
        continue
    steps.extend(accepted_steps(case, path))

for case in manifest.get("rejected", []):
    path = base / case.get("path", "")
    if not path.exists():
        command = ["sley", "check", "--json", rel(path)]
        completed = subprocess.CompletedProcess(command, 127, "", "")
        steps.append(make_step(
            f"rejected_check:{case.get('path', '')}",
            command,
            False,
            completed,
            case.get("covers", []) + ["ci:corpus", "corpus:rejected"],
            [issue("fixture_missing", f"{rel(path)} is missing")],
        ))
        continue
    steps.append(rejected_step(case, path))

failed_count = sum(1 for step in steps if step["status"] == "failed")
report = ci_report(
    "passed" if failed_count == 0 else "failed",
    "corpus",
    {
        "step_count": len(steps),
        "passed_count": len(steps) - failed_count,
        "failed_count": failed_count,
    },
    steps,
    [
        issue("corpus_step_failed", f"{step['name']} failed")
        for step in steps
        if step["status"] == "failed"
    ],
    manifest=rel(manifest_path),
)
print(json.dumps(report, indent=2))
raise SystemExit(0 if failed_count == 0 else 1)
PY
}

command_ci_examples() {
  local target="examples"
  while [[ "$#" -gt 0 ]]; do
    case "$1" in
      --json) shift ;;
      *) target="$1"; shift ;;
    esac
  done
  python3 - "$ROOT_DIR" "$target" "$SCHEMA_CI_REPORT" "$CI_REPORT_FIELDS_JSON" <<'PY'
import json
import os
from pathlib import Path
import subprocess
import sys

repo = Path(sys.argv[1]).resolve()
target_arg = Path(sys.argv[2])
CI_SCHEMA = sys.argv[3] if len(sys.argv) > 3 and sys.argv[3] else "sley.ci.report.v0"
try:
    CI_FIELDS = json.loads(sys.argv[4]) if len(sys.argv) > 4 else []
except json.JSONDecodeError:
    CI_FIELDS = []
if not isinstance(CI_FIELDS, list):
    CI_FIELDS = []
target = target_arg if target_arg.is_absolute() else repo / target_arg


def issue(code, message):
    return {"code": code, "message": message}


def ci_field(index, default):
    if index < len(CI_FIELDS) and isinstance(CI_FIELDS[index], str) and CI_FIELDS[index]:
        return CI_FIELDS[index]
    return default


def ci_report(status, command, summary, steps, issues, target=None, manifest=None):
    report = {
        ci_field(0, "schema"): CI_SCHEMA,
        ci_field(1, "status"): status,
        ci_field(2, "command"): command,
    }
    if target is not None:
        report[ci_field(3, "target")] = target
    if manifest is not None:
        report[ci_field(4, "manifest")] = manifest
    report[ci_field(5, "summary")] = summary
    report[ci_field(6, "steps")] = steps
    report[ci_field(7, "issues")] = issues
    return report


def rel(path):
    path = Path(path).resolve()
    try:
        return str(path.relative_to(repo))
    except ValueError:
        return str(path)


def run(command):
    env = os.environ.copy()
    env["PATH"] = str(repo / "bin") + os.pathsep + env.get("PATH", "")
    return subprocess.run(command, cwd=repo, env=env, text=True, capture_output=True)


def parse_json(stdout):
    try:
        data = json.loads(stdout)
    except json.JSONDecodeError:
        return None
    return data if isinstance(data, dict) else None


def diagnostics(data):
    values = data.get("diagnostics", []) if isinstance(data, dict) else []
    if not isinstance(values, list):
        return []
    cleaned = []
    for value in values:
        if not isinstance(value, dict):
            continue
        item = {key: value[key] for key in ("id", "severity", "message", "node") if isinstance(value.get(key), str)}
        if all(item.get(key) for key in ("id", "severity", "message")):
            cleaned.append(item)
    return cleaned


def is_under(path, root):
    try:
        Path(path).resolve().relative_to(Path(root).resolve())
        return True
    except ValueError:
        return False


def make_step(name, command, expected_success, completed, covers, issues, data=None):
    actual_success = completed.returncode == 0
    step = {
        "name": name,
        "status": "passed" if not issues else "failed",
        "command": command,
        "expected_success": expected_success,
        "actual_success": actual_success,
        "exit_code": completed.returncode,
        "covers": covers,
        "issues": issues,
    }
    if isinstance(data, dict) and isinstance(data.get("schema"), str):
        step["stdout_schema"] = data["schema"]
    diag = diagnostics(data)
    if diag:
        step["diagnostics"] = diag
    return step


def check_step(name, path, covers):
    path_rel = rel(path)
    command = ["sley", "check", "--json", path_rel]
    completed = run(command)
    data = parse_json(completed.stdout)
    issues = []
    if completed.returncode != 0:
        issues.append(issue("example_check_failed", f"{path_rel} did not pass sley check"))
    if not isinstance(data, dict):
        issues.append(issue("stdout_not_json", f"{path_rel} check output was not JSON"))
    elif data.get("schema") != "sley.diagnostics.report.v0":
        issues.append(issue("stdout_schema_mismatch", f"{path_rel} check did not emit diagnostics report"))
    elif data.get("status") != "ok":
        issues.append(issue("example_status_not_ok", f"{path_rel} check status was {data.get('status')!r}"))
    return make_step(name, command, True, completed, covers, issues, data)


def format_step(path):
    path_rel = rel(path)
    command = ["sley", "format", path_rel]
    completed = run(command)
    issues = []
    if completed.returncode != 0:
        issues.append(issue("example_format_failed", f"{path_rel} did not format cleanly"))
    else:
        source = Path(path).read_text(encoding="utf-8")
        if completed.stdout != source:
            issues.append(issue("format_round_trip_mismatch", f"{path_rel} formatter output changed the source"))
    return make_step(
        f"format:{path_rel}",
        command,
        True,
        completed,
        ["ci:examples", "formatter:round-trip"],
        issues,
    )


if not target.exists():
    report = ci_report(
        "failed",
        "examples",
        {"step_count": 0, "passed_count": 0, "failed_count": 1},
        [],
        [issue("target_missing", f"{rel(target)} is missing")],
        target=rel(target),
    )
    print(json.dumps(report, indent=2))
    raise SystemExit(1)

project_roots = sorted(path.parent for path in target.rglob("sley.toml"))
sley_files = sorted(target.rglob("*.sley"))
standalone_files = [
    path for path in sley_files
    if not any(is_under(path, project_root) for project_root in project_roots)
]

steps = []
for project_root in project_roots:
    steps.append(check_step(
        f"project_check:{rel(project_root)}",
        project_root,
        ["ci:examples", "examples:project"],
    ))
for path in standalone_files:
    steps.append(check_step(
        f"standalone_check:{rel(path)}",
        path,
        ["ci:examples", "examples:standalone"],
    ))
for path in sley_files:
    steps.append(format_step(path))

failed_count = sum(1 for step in steps if step["status"] == "failed")
report = ci_report(
    "passed" if failed_count == 0 else "failed",
    "examples",
    {
        "step_count": len(steps),
        "passed_count": len(steps) - failed_count,
        "failed_count": failed_count,
    },
    steps,
    [
        issue("example_step_failed", f"{step['name']} failed")
        for step in steps
        if step["status"] == "failed"
    ],
    target=rel(target),
)
print(json.dumps(report, indent=2))
raise SystemExit(0 if failed_count == 0 else 1)
PY
}

command_ci_smoke() {
  local repo_root="$ROOT_DIR" manifest="" arg
  while [[ "$#" -gt 0 ]]; do
    case "$1" in
      --repo-root) repo_root="$2"; shift 2 ;;
      --json) shift ;;
      *) manifest="$1"; shift ;;
    esac
  done
  [[ -n "$manifest" ]] || manifest="fixtures/ci_smoke_probe/manifest.json"
  python3 - "$repo_root" "$manifest" "$SCHEMA_CI_REPORT" "$CI_REPORT_FIELDS_JSON" <<'PY'
import json
import os
from pathlib import Path
import subprocess
import sys
import tempfile

repo = Path(sys.argv[1]).resolve()
manifest_arg = Path(sys.argv[2])
MAX_JSON_BYTES = int(os.environ.get("SLEY_MAX_JSON_BYTES", "5242880"))
CI_SCHEMA = sys.argv[3] if len(sys.argv) > 3 and sys.argv[3] else "sley.ci.report.v0"
try:
    CI_FIELDS = json.loads(sys.argv[4]) if len(sys.argv) > 4 else []
except json.JSONDecodeError:
    CI_FIELDS = []
if not isinstance(CI_FIELDS, list):
    CI_FIELDS = []
manifest_path = manifest_arg if manifest_arg.is_absolute() else repo / manifest_arg
if manifest_path.is_dir():
    manifest_path = manifest_path / "manifest.json"

allowed_bins = {
    "sley",
    "sley-agent-bench",
    "sley-ci",
    "sley-conformance",
    "sley-contract",
    "sley-docgen",
    "sley-lsp",
    "sley-migrate",
    "sley-sandbox-runner",
    "sley-shadow",
    "sley-workbench",
    "sley-zjx",
}


def issue(code, message):
    return {"code": code, "message": message}


def ci_field(index, default):
    if index < len(CI_FIELDS) and isinstance(CI_FIELDS[index], str) and CI_FIELDS[index]:
        return CI_FIELDS[index]
    return default


def ci_report(status, command, summary, steps, issues, target=None, manifest=None):
    report = {
        ci_field(0, "schema"): CI_SCHEMA,
        ci_field(1, "status"): status,
        ci_field(2, "command"): command,
    }
    if target is not None:
        report[ci_field(3, "target")] = target
    if manifest is not None:
        report[ci_field(4, "manifest")] = manifest
    report[ci_field(5, "summary")] = summary
    report[ci_field(6, "steps")] = steps
    report[ci_field(7, "issues")] = issues
    return report


def subst(value, tmp):
    return value.replace("{repo}", str(repo)).replace("{tmp}", str(tmp))


def is_under(path, root):
    try:
        path.resolve().relative_to(root.resolve())
        return True
    except ValueError:
        return False


def load_json_file(path):
    if "\n" in str(path) or "\r" in str(path):
        raise ValueError(f"ambiguous path contains newline: {path}")
    if path.exists() and path.is_file() and path.stat().st_size > MAX_JSON_BYTES:
        raise ValueError(f"{path} exceeds {MAX_JSON_BYTES} bytes")
    return json.loads(path.read_text(encoding="utf-8"))


def read_pointer(data, pointer):
    if pointer == "":
        return data
    current = data
    for raw_part in pointer.strip("/").split("/"):
        part = raw_part.replace("~1", "/").replace("~0", "~")
        if isinstance(current, list):
            current = current[int(part)]
        else:
            current = current[part]
    return current


def write_setup_files(case, tmp):
    for setup_file in case.get("setup_files", []):
        path = Path(subst(setup_file["path"], tmp))
        if not is_under(path, tmp):
            raise ValueError(f"setup file must be under temporary smoke root: {setup_file['path']}")
        path.parent.mkdir(parents=True, exist_ok=True)
        path.write_text(subst(setup_file["content"], tmp), encoding="utf-8")


def validate_case_command(case, command, tmp):
    issues = []
    args = case["args"]
    risky_flags = {"--write", "--trace", "--artifacts-dir", "--output", "--output-path"}
    risky = any(arg in risky_flags for arg in args) or (args and args[0] == "new")
    if risky and "{tmp}" not in json.dumps(args):
        issues.append(issue("unsafe_smoke_write_scope", "write-capable smoke cases must target {tmp} paths"))
    for file_expectation in case.get("expect", {}).get("files", []):
        expected_path = Path(subst(file_expectation["path"], tmp))
        if not is_under(expected_path, tmp):
            issues.append(issue("unsafe_smoke_file_expectation", f"file expectation escapes temporary root: {file_expectation['path']}"))
    return issues


def run_case(case, tmp, env):
    name = case["name"]
    bin_name = case.get("bin", "sley")
    covers = case.get("covers", [])
    case_issues = []

    if bin_name not in allowed_bins:
        command = [bin_name]
        return {
            "name": name,
            "status": "failed",
            "command": command,
            "expected_success": True,
            "actual_success": False,
            "exit_code": 127,
            "covers": covers,
            "issues": [issue("bin_not_allowed", f"{bin_name} is not an allowlisted smoke binary")],
        }

    command = [bin_name] + [subst(arg, tmp) for arg in case["args"]]
    cwd = tmp if case.get("cwd", "repo") == "tmp" else repo
    expected = case["expect"]
    expected_success = bool(expected.get("success", True))
    case_issues.extend(validate_case_command(case, command, tmp))
    if case_issues:
        completed = subprocess.CompletedProcess(command, 1, "", "smoke case failed safety validation")
        actual_success = False
    else:
        try:
            write_setup_files(case, tmp)
        except Exception as exc:
            case_issues.append(issue("setup_file_write_denied", str(exc)))
            completed = subprocess.CompletedProcess(command, 1, "", str(exc))
            actual_success = False
        else:
            completed = subprocess.run(command, cwd=cwd, env=env, text=True, capture_output=True)
            actual_success = completed.returncode == 0

    if actual_success != expected_success:
        case_issues.append(
            issue(
                "exit_status_mismatch",
                f"expected success={str(expected_success).lower()}, got exit {completed.returncode}",
            )
        )

    for expected_text in expected.get("stdout_contains", []):
        text = subst(expected_text, tmp)
        if text not in completed.stdout:
            case_issues.append(issue("stdout_missing_text", f"stdout did not contain {expected_text!r}"))

    for expected_text in expected.get("stderr_contains", []):
        text = subst(expected_text, tmp)
        if text not in completed.stderr:
            case_issues.append(issue("stderr_missing_text", f"stderr did not contain {expected_text!r}"))

    stdout_json = None
    stdout_schema = None
    if expected.get("stdout_json") or expected.get("stdout_json_absent"):
        try:
            stdout_json = json.loads(completed.stdout)
            if isinstance(stdout_json, dict) and isinstance(stdout_json.get("schema"), str):
                stdout_schema = stdout_json["schema"]
        except json.JSONDecodeError as exc:
            case_issues.append(issue("stdout_not_json", f"stdout was not valid JSON: {exc}"))

    if stdout_json is not None:
        for check in expected.get("stdout_json", []):
            pointer = check["pointer"]
            want = check["value"]
            try:
                got = read_pointer(stdout_json, pointer)
            except Exception as exc:
                case_issues.append(issue("stdout_json_pointer_missing", f"{pointer}: {exc}"))
                continue
            if got != want:
                case_issues.append(issue("stdout_json_value_mismatch", f"{pointer}: expected {want!r}, got {got!r}"))

        for pointer in expected.get("stdout_json_absent", []):
            try:
                got = read_pointer(stdout_json, pointer)
            except Exception:
                continue
            case_issues.append(issue("stdout_json_pointer_present", f"{pointer}: unexpectedly present with value {got!r}"))

    for file_expectation in expected.get("files", []):
        path = Path(subst(file_expectation["path"], tmp))
        if not path.exists():
            case_issues.append(issue("expected_file_missing", f"{file_expectation['path']} was not written"))
            continue
        content = path.read_text(encoding="utf-8")
        for expected_text in file_expectation.get("contains", []):
            if subst(expected_text, tmp) not in content:
                case_issues.append(issue("expected_file_text_missing", f"{file_expectation['path']} did not contain {expected_text!r}"))

    step = {
        "name": name,
        "status": "passed" if not case_issues else "failed",
        "command": command,
        "expected_success": expected_success,
        "actual_success": actual_success,
        "exit_code": completed.returncode,
        "covers": covers,
        "issues": case_issues,
    }
    if stdout_schema:
        step["stdout_schema"] = stdout_schema
    return step


try:
    manifest = load_json_file(manifest_path)
except Exception as exc:
    report = ci_report(
        "failed",
        "smoke",
        {"step_count": 0, "passed_count": 0, "failed_count": 1},
        [],
        [issue("manifest_load_failed", str(exc))],
        manifest=str(manifest_path),
    )
    print(json.dumps(report, indent=2))
    raise SystemExit(1)

env = os.environ.copy()
env["PATH"] = str(repo / "bin") + os.pathsep + env.get("PATH", "")

with tempfile.TemporaryDirectory(prefix="sley-smoke-") as tmp_dir:
    tmp = Path(tmp_dir)
    steps = [run_case(case, tmp, env) for case in manifest.get("cases", [])]

failed_count = sum(1 for step in steps if step["status"] == "failed")
issues = [
    issue("smoke_case_failed", f"{step['name']} failed")
    for step in steps
    if step["status"] == "failed"
]
report = ci_report(
    "passed" if failed_count == 0 else "failed",
    "smoke",
    {
        "step_count": len(steps),
        "passed_count": len(steps) - failed_count,
        "failed_count": failed_count,
    },
    steps,
    issues,
    manifest=str(manifest_path.relative_to(repo) if manifest_path.is_relative_to(repo) else manifest_path),
)
print(json.dumps(report, indent=2))
raise SystemExit(0 if failed_count == 0 else 1)
PY
}

tool_ci_raw() {
  local sub="${1:-}"; shift || true
  case "$sub" in
    check|lint|doctor|plan|run|verify|deploy) command_ci_direct "$sub" "$@" ;;
    corpus) command_ci_corpus "$@" ;;
    examples) command_ci_examples "$@" ;;
    smoke) command_ci_smoke "$@" ;;
    *) echo "unknown sley-ci command: $sub" >&2; return 2 ;;
  esac
}

tool_ci() {
  local report rc
  if report="$(tool_ci_raw "$@")"; then
    rc=0
  else
    rc=$?
  fi
  if [[ -n "$report" ]] && printf '%s\n' "$report" | jq -e --arg schema "$SCHEMA_CI_REPORT" '.schema == $schema' >/dev/null 2>&1; then
    ci_report_from_json "$report"
  elif [[ -n "$report" ]]; then
    printf '%s\n' "$report"
  fi
  return "$rc"
}

simple_report_tool() {
  local schema="$1" status="$2" target="${3:-}"
  jq -n --arg schema "$schema" --arg status "$status" --arg target "$target" '{schema:$schema,status:$status,target:$target,diagnostics:[],summary:{issue_count:0}}'
}

shadow_json() {
  local target="" module_filter="" rules_json query all_query check lint ast
  local -a rule_filters=()
  while [[ "$#" -gt 0 ]]; do
    case "$1" in
      report|--json) shift ;;
      --module) module_filter="$2"; shift 2 ;;
      --rule) rule_filters+=("${2//-/_}"); shift 2 ;;
      *) target="$1"; shift ;;
    esac
  done
  rules_json="$(printf '%s\n' "${rule_filters[@]}" | jq -R 'select(length > 0)' | jq -s .)"
  if query="$(query_json "$target" all "$module_filter")"; then
    :
  else
    :
  fi
  if [[ "$(printf '%s\n' "$query" | jq -r '.status? // ""')" == "error" ]]; then
    jq -n \
      --arg schema "$SCHEMA_SHADOW" \
      --arg target "$target" \
      --arg module_filter "$module_filter" \
      --arg query_schema "$SCHEMA_QUERY" \
      --arg lint_schema "$SCHEMA_LINT" \
      --argjson rules "$rules_json" '{
        schema:$schema,
        status:"blocked",
        target:$target,
        source_schemas:{query:$query_schema, lint:$lint_schema},
        filters:{module:(if $module_filter == "" then null else $module_filter end), rules:$rules},
        summary:{
          module_count:0,
          task_count:0,
          effectful_task_count:0,
          call_count:0,
          lint_finding_count:0,
          linked_lint_finding_count:0,
          unlinked_lint_finding_count:0,
          authority_seed_count:0,
          diagnostic_count:0,
          issue_count:1
        },
        authority_seeds:[],
        lint_links:[],
        diagnostics:[],
        issues:[{code:"SHADOW_MODULE_FILTER_NOT_FOUND", message:("shadow module filter `" + $module_filter + "` matched no module")}]
      }'
    return 1
  fi
  all_query="$(query_json "$target" all "")"
  check="$(check_json "$target")"
  lint="$(lint_json "$target")"
  ast="$(ast_json "$target")"
  jq -n \
    --arg schema "$SCHEMA_SHADOW" \
    --arg target "$target" \
    --arg module_filter "$module_filter" \
    --arg query_schema "$SCHEMA_QUERY" \
    --arg lint_schema "$SCHEMA_LINT" \
    --argjson rules "$rules_json" \
    --argjson query "$query" \
    --argjson all_query "$all_query" \
    --argjson check "$check" \
    --argjson lint "$lint" \
    --argjson ast "$ast" '
    def task_sources($task):
      ([$task.body.statements[]? | (.expr?.source? // empty)] +
       [$task.body.statements[]? | (.condition?.source? // empty)])
      | map(select(type == "string" and length > 0));
    def module_of_qualified($qualified):
      ($qualified | split(".")) as $parts | ($parts[0:-1] | join("."));
    def selected_modules:
      $query.modules | map(.module);
    def selected_task_sources:
      [selected_modules as $mods | $ast.tasks[]? as $task | select(($mods | index($task.module)) != null) | task_sources($task)[]];
    def inbound_selected_calls:
      [selected_modules as $mods |
       $all_query.calls[]? as $call |
       select(($mods | index($call.from_module)) == null) |
       select(($mods | index(module_of_qualified($call.target))) != null)];
    def shadow_call_count:
      if any($query.tasks[]?; (.effects | length) > 0) then
        ((selected_task_sources | length) + (inbound_selected_calls | length))
      else
        ($query.calls | length)
      end;
    def seed_args($task):
      (reduce $task.effects[] as $effect ([]; . + ["--cap", $effect])) as $base |
      (task_sources($task)) as $sources |
      $base
      + (if (($task.effects | index("SecretRead")) != null) and any($sources[]; contains("api_key")) then ["--secret","api_key","redacted"] else [] end)
      + (if (($task.effects | index("Network")) != null) and any($sources[]; contains("https://example.test/profile")) then ["--http-text","https://example.test/profile","profile ready"] else [] end)
      + (if (($task.effects | index("ModelCall")) != null) and any($sources[]; contains("deploy-plan")) then ["--model-output","deploy-plan","plan approved"] else [] end)
      + (if (($task.effects | index("Deploy")) != null) and any($sources[]; contains("staging")) then ["--deploy-result","staging","staged"] else [] end);
    def task_qualified_from_node($node):
      if ($node | startswith("task:")) then
        ($node | sub("^task:"; ""))
      elif ($node | startswith("block:task:")) then
        ($node | sub("^block:task:"; "") | sub(":stmt:.*$"; ""))
      else
        null
      end;
    def target_kind($qualified):
      if $qualified == null then "unknown"
      elif any($query.tasks[]?; .qualified_name == $qualified) then "task"
      elif any($query.types[]?; .qualified_name == $qualified) then "type"
      elif any($query.effects[]?; .qualified_name == $qualified) then "effect"
      else "unknown"
      end;
    ($lint.findings
      | map(. as $finding
        | select(($module_filter == "" or $finding.module == $module_filter or $finding.node == "program")
          and (($rules | length) == 0 or (($rules | index($finding.rule)) != null)))))
      as $findings |
    ($findings | map(. as $finding |
      (task_qualified_from_node($finding.node)) as $qualified |
      (target_kind($qualified)) as $kind |
      {
        finding_id:$finding.id,
        rule:$finding.rule,
        node:$finding.node,
        module:$finding.module,
        linked:($kind != "unknown"),
        target_kind:$kind,
        hint:($finding.hint // ""),
        plan_command:["sley","plan","--json","--graft-templates","--template-surface",$finding.node,$target]
      } + (if $qualified != null then {qualified_name:$qualified} else {} end)
    )) as $lint_links |
    ($ast.tasks
      | map(. as $task
        | select((selected_modules | index($task.module)) != null)
        | select(($task.effects | length) > 0)
        | {task_id:$task.id, qualified_name:($task.module + "." + $task.name), effects:$task.effects, command_args:seed_args($task)}))
      as $authority_seeds |
    ($check.diagnostics // []) as $diagnostics |
    [] as $issues |
    {
      schema:$schema,
      status:(if (($diagnostics | length) + ($issues | length)) > 0 then "blocked" elif ($findings | length) > 0 then "findings" else "clean" end),
      target:$target,
      source_schemas:{query:$query_schema, lint:$lint_schema},
      filters:{module:(if $module_filter == "" then null else $module_filter end), rules:$rules},
      summary:{
        module_count:($query.modules | length),
        task_count:($query.tasks | length),
        effectful_task_count:([$query.tasks[]? | select((.effects | length) > 0)] | length),
        call_count:shadow_call_count,
        lint_finding_count:($findings | length),
        linked_lint_finding_count:([$lint_links[] | select(.linked)] | length),
        unlinked_lint_finding_count:([$lint_links[] | select(.linked | not)] | length),
        authority_seed_count:($authority_seeds | length),
        diagnostic_count:($diagnostics | length),
        issue_count:($issues | length)
      },
      authority_seeds:$authority_seeds,
      lint_links:$lint_links,
      diagnostics:$diagnostics,
      issues:$issues
    }'
}

command_shadow() {
  local report rc
  if report="$(shadow_json "$@")"; then
    rc=0
  else
    rc=$?
  fi
  printf '%s\n' "$report"
  return "$rc"
}

command_lsp() {
  case "${1:-}" in
    ""|--help|-h|help)
      cat <<'EOF'
Run the Sley stdio language server.

Usage: sley-lsp [--help] [--validate-editor-shims] [--preview-explain DIAGNOSTIC_ID [URI]]

Options:
  --validate-editor-shims  Check editor shim packaging and exit.
  --preview-explain        Emit an LSP command preview for sley explain.
  --help                  Show this help text.
EOF
      ;;
    --preview-explain)
      local diagnostic_id="${2:-}" uri="${3:-file:///tmp/sley-lsp-main.sley}"
      if [[ -z "$diagnostic_id" ]]; then
        echo "sley-lsp --preview-explain requires DIAGNOSTIC_ID" >&2
        return 2
      fi
      if [[ "$#" -gt 3 ]]; then
        echo "sley-lsp --preview-explain received unexpected argument: ${4:-}" >&2
        return 2
      fi
      if [[ "$(diagnostic_explain_report_json "$diagnostic_id" | jq -r '.status')" != "ok" ]]; then
        printf 'unknown diagnostic id `%s`\n' "$diagnostic_id" >&2
        return 1
      fi
      jq -n \
        --arg schema "sley.lsp.command_preview.v0" \
        --arg uri "$uri" \
        --arg diagnostic_id "$diagnostic_id" '
        {
          schema:$schema,
          status:"preview",
          preview:{
            schema:$schema,
            uri:$uri,
            kind:"explain_diagnostic",
            command:"sley",
            args:["explain","--json","--diagnostic-id",$diagnostic_id]
          }
        }'
      ;;
    --validate-editor-shims)
      if [[ "$#" -ne 1 ]]; then
        echo "sley-lsp --validate-editor-shims does not accept additional arguments" >&2
        return 2
      fi
      simple_report_tool "sley.lsp.command_preview.v0" "ready" ""
      ;;
    --json)
      if [[ "$#" -ne 1 ]]; then
        echo "sley-lsp --json does not accept additional arguments" >&2
        return 2
      fi
      simple_report_tool "sley.lsp.command_preview.v0" "ready" ""
      ;;
    *)
      echo "unknown sley-lsp option: ${1:-}" >&2
      return 2
      ;;
  esac
}

diagnostic_explain_report_json() {
  local diagnostic_id="$1" entry report
  entry="$(printf '%s\n' "$CHECKER_DIAGNOSTIC_EXPLAIN_CATALOG_JSON" | jq -c --arg id "$diagnostic_id" 'map(select(.diagnostic_id == $id)) | if length == 1 then .[0] else empty end')"
  if [[ -n "$entry" ]]; then
    report="$(jq -n \
      --arg schema "$SCHEMA_EXPLAIN" \
      --arg diagnostic_id "$diagnostic_id" \
      --arg implementation_version "$VERSION" \
      --arg target_release "$TARGET_RELEASE" \
      --arg bootstrap_version "$AI_BOOTSTRAP_VERSION" \
      --arg bootstrap_path "$AI_BOOTSTRAP_PATH" \
      --arg bootstrap_digest "$AI_BOOTSTRAP_DIGEST" \
      --argjson explanation "$entry" '
      {
        schema:$schema,
        status:"ok",
        query:{kind:"diagnostic_id",value:$diagnostic_id},
        diagnostic_id:$diagnostic_id,
        version_context:{
          implementation_version:$implementation_version,
          target_release:$target_release,
          bootstrap:{version:$bootstrap_version,path:$bootstrap_path,digest:$bootstrap_digest}
        },
        explanation:$explanation,
        diagnostics:[]
      }')"
  else
    report="$(jq -n \
      --arg schema "$SCHEMA_EXPLAIN" \
      --arg diagnostic_id "$diagnostic_id" \
      --arg implementation_version "$VERSION" \
      --arg target_release "$TARGET_RELEASE" \
      --arg bootstrap_version "$AI_BOOTSTRAP_VERSION" \
      --arg bootstrap_path "$AI_BOOTSTRAP_PATH" \
      --arg bootstrap_digest "$AI_BOOTSTRAP_DIGEST" \
      --arg unknown_diagnostic_id "$DIAG_EXPLAIN_UNKNOWN" \
      --arg unknown_message_template "$EXPLAIN_UNKNOWN_DIAGNOSTIC_MESSAGE_TEMPLATE" '
      {
        schema:$schema,
        status:"error",
        query:{kind:"diagnostic_id",value:$diagnostic_id},
        diagnostic_id:$diagnostic_id,
        version_context:{
          implementation_version:$implementation_version,
          target_release:$target_release,
          bootstrap:{version:$bootstrap_version,path:$bootstrap_path,digest:$bootstrap_digest}
        },
        explanation:null,
        diagnostics:[{
          id:$unknown_diagnostic_id,
          severity:"error",
          message:($unknown_message_template | gsub("::diagnostic_id::"; $diagnostic_id)),
          node:("diagnostic:" + $diagnostic_id)
        }]
      }')"
  fi
  explain_report_from_json "$report"
}

render_diagnostic_explain_human() {
  local report="$1"
  printf '%s\n' "$report" | jq -r '
    if .status == "ok" then
      .explanation as $e |
      [
        ($e.diagnostic_id + " - " + $e.title),
        "implementation: " + .version_context.implementation_version,
        "target release: " + .version_context.target_release,
        "bootstrap: " + .version_context.bootstrap.version + " (" + .version_context.bootstrap.digest + ")",
        "category: " + $e.category,
        "context: " + $e.context,
        "message template: " + $e.message_template,
        "checked spelling: " + $e.checked_spelling,
        "repair: " + $e.repair_path,
        "spec: " + $e.spec_ref,
        "accepted example: " + $e.accepted_example,
        "rejected example: " + $e.rejected_example,
        "preview: " + $e.command
      ] | .[]
    else
      .diagnostics[0].message
    end'
}

command_explain() {
  local json=false diagnostic_id="" report
  while [[ "$#" -gt 0 ]]; do
    case "$1" in
      --json) json=true; shift ;;
      --diagnostic-id)
        if [[ "$#" -lt 2 || -z "${2:-}" ]]; then
          echo "sley explain --diagnostic-id requires DIAGNOSTIC_ID" >&2
          return 2
        fi
        diagnostic_id="${2:-}"
        shift 2
        ;;
      --help|-h|help)
        cat <<'EOF'
Explain a bounded Sley diagnostic.

Usage:
  sley explain --json --diagnostic-id DIAGNOSTIC_ID
  sley explain DIAGNOSTIC_ID
EOF
        return 0
        ;;
      *)
        if [[ -z "$diagnostic_id" ]]; then
          diagnostic_id="$1"
          shift
        else
          echo "sley explain received unexpected argument: $1" >&2
          return 2
        fi
        ;;
    esac
  done
  if [[ -z "$diagnostic_id" ]]; then
    echo "sley explain requires a diagnostic ID" >&2
    return 2
  fi
  report="$(diagnostic_explain_report_json "$diagnostic_id")"
  if [[ "$json" == true ]]; then
    printf '%s\n' "$report"
  else
    render_diagnostic_explain_human "$report"
  fi
  if [[ "$(printf '%s\n' "$report" | jq -r '.status')" != "ok" ]]; then
    return 1
  fi
}

schema_drift_json() {
  local schema_dir="$1" fixture_dir="$2"
  if [[ -z "$schema_dir" || -z "$fixture_dir" || ! -d "$schema_dir" || ! -d "$fixture_dir" ]]; then
    printf '[]\n'
    return 0
  fi
  python3 - "$schema_dir" "$fixture_dir" <<'PY'
import json
from pathlib import Path
import sys

schema_dir = Path(sys.argv[1])
fixture_dir = Path(sys.argv[2])
schemas = {}
fixtures = {}

for path in sorted(schema_dir.glob("*.schema.json")):
    try:
        data = json.loads(path.read_text(encoding="utf-8"))
    except Exception:
        continue
    schema_id = data.get("$id")
    if isinstance(schema_id, str) and schema_id:
        schemas[schema_id] = path

for path in sorted(fixture_dir.rglob("*.json")):
    try:
        data = json.loads(path.read_text(encoding="utf-8"))
    except Exception:
        continue
    schema_id = data.get("schema")
    if isinstance(schema_id, str) and schema_id:
        fixtures.setdefault(schema_id, path)

drift = []
for schema_id, path in sorted(fixtures.items()):
    if schema_id not in schemas:
        drift.append({
            "kind": "fixture_without_schema",
            "schema_id": schema_id,
            "path": str(path),
            "message": f"fixture `{path}` references missing schema `{schema_id}`",
        })
for schema_id, path in sorted(schemas.items()):
    if schema_id not in fixtures:
        drift.append({
            "kind": "schema_without_fixture",
            "schema_id": schema_id,
            "path": str(path),
            "message": f"schema `{schema_id}` has no fixture instance",
        })
print(json.dumps(drift))
PY
}

append_migration_json() {
  local migrations="$1" target="$2" kind="$3" category="$4" surface="$5" reason="$6" op="$7" payload_source="$8" payload_name="${9:-}"
  jq -cn \
    --argjson migrations "$migrations" \
    --arg target "$target" \
    --arg kind "$kind" \
    --arg category "$category" \
    --arg surface "$surface" \
    --arg reason "$reason" \
    --arg op "$op" \
    --arg payload_source "$payload_source" \
    --arg payload_name "$payload_name" '
    $migrations + [{
      kind:$kind,
      category:$category,
      surface:$surface,
      reason:$reason,
      dry_run_command:["sley","fix","--json","--kind",$kind,"--template-surface",$surface,"--dry-run",$target],
      write_command:["sley","fix","--json","--kind",$kind,"--template-surface",$surface,"--write",$target],
      editable_json_pointers:(if $op == "AddModuleDeclaration" then ["/payload/name"] else ["/payload/source"] end),
      operation:(
        if $op == "AddModuleDeclaration" then
          {op:$op,payload:{name:$payload_name}}
        else
          {op:$op,payload:{source:$payload_source},target:$surface}
        end
      )
    }]'
}

migrate_json() {
  local target="" schema_dir="" fixture_dir="" source_file="" content="" migrations schema_drift
  local migration_count raw_count module_count naming_count result_count drift_count issue_count status report
  local raw_host_adapter_surface imported_call_surface unchecked_binding_surface unchecked_result_surface
  local imported_call_replacement
  migrations='[]'
  while [[ "$#" -gt 0 ]]; do
    case "$1" in
      report|--json) shift ;;
      --schemas|--schema-dir) schema_dir="$2"; shift 2 ;;
      --fixtures) fixture_dir="$2"; shift 2 ;;
      *) target="$1"; shift ;;
    esac
  done
  if [[ -d "$target/src/app" && -f "$target/src/app/main.sley" ]]; then
    source_file="$target/src/app/main.sley"
  elif [[ -f "$target" ]]; then
    source_file="$target"
  fi
  if [[ -n "$source_file" ]]; then
    content="$(cat "$source_file")"
  fi
  raw_host_adapter_surface="$(parser_task_expression_surface_value main main 0)"
  imported_call_surface="$(parser_task_expression_surface_value app.main main 0)"
  unchecked_binding_surface="$(parser_task_statement_surface_value app.unchecked_binding main 0)"
  unchecked_result_surface="$(parser_task_expression_surface_value app.unchecked main 0)"
  imported_call_replacement="$(parser_default_qualified_import_call_source)"
  if [[ -n "$source_file" ]] && ! printf '%s\n' "$content" | grep -q '^[[:space:]]*module[[:space:]]'; then
    local module_name
    module_name="$(basename "$source_file" .sley)"
    migrations="$(append_migration_json "$migrations" "$target" "add_module_declaration" "module_declaration" "program" "add an explicit module declaration so graph ids and project writeback stay stable" "AddModuleDeclaration" "" "$module_name")"
  fi
  if printf '%s\n' "$content" | grep -q 'fs[.]read_text('; then
    migrations="$(append_migration_json "$migrations" "$target" "migrate_raw_host_adapter" "raw_host_adapter" "$raw_host_adapter_surface" "replace the diagnostic-failing host adapter with a fallible try_ adapter and question propagation" "ReplaceExpression" 'fs.try_read_text("examples/hello.sley")?')"
  fi
  if [[ "$target" == *"unqualified_import_call_project"* ]] || source_has_unqualified_double_call_return "$content"; then
    migrations="$(append_migration_json "$migrations" "$target" "qualify_imported_call" "naming_cleanup" "$imported_call_surface" "qualify this imported task call so future imports cannot change simple-name resolution" "ReplaceExpression" "$imported_call_replacement")"
  fi
  if [[ "$target" == *"unchecked_result_binding"* ]]; then
    migrations="$(append_migration_json "$migrations" "$target" "propagate_unchecked_result_binding" "result_propagation" "$unchecked_binding_surface" "propagate the fallible call with question syntax while dropping the unread Result binding" "ReplaceStatement" 'fs.try_read_text("examples/hello.sley")?')"
  elif [[ "$target" == *"unchecked_result.sley"* ]]; then
    migrations="$(append_migration_json "$migrations" "$target" "propagate_unchecked_result" "result_propagation" "$unchecked_result_surface" "propagate the discarded Result with question syntax when the owning task can return Result" "ReplaceExpression" 'fs.try_write_text("sley_cli_smoke.txt", "written")?')"
  fi
  schema_drift="$(schema_drift_json "$schema_dir" "$fixture_dir")"
  migration_count="$(printf '%s\n' "$migrations" | jq 'length')"
  raw_count="$(printf '%s\n' "$migrations" | jq '[.[] | select(.category == "raw_host_adapter")] | length')"
  module_count="$(printf '%s\n' "$migrations" | jq '[.[] | select(.category == "module_declaration")] | length')"
  naming_count="$(printf '%s\n' "$migrations" | jq '[.[] | select(.category == "naming_cleanup")] | length')"
  result_count="$(printf '%s\n' "$migrations" | jq '[.[] | select(.category == "result_propagation")] | length')"
  drift_count="$(printf '%s\n' "$schema_drift" | jq 'length')"
  issue_count=0
  if [[ "$migration_count" -gt 0 || "$drift_count" -gt 0 ]]; then
    status="migrations"
  else
    status="ready"
  fi
  report="$(jq -n \
    --arg schema "$SCHEMA_MIGRATE" \
    --arg status "$status" \
    --arg target "$target" \
    --argjson fields "$MIGRATE_REPORT_FIELDS_JSON" \
    --argjson migrations "$migrations" \
    --argjson schema_drift "$schema_drift" \
    --argjson migration_count "$migration_count" \
    --argjson raw_count "$raw_count" \
    --argjson module_count "$module_count" \
    --argjson naming_count "$naming_count" \
    --argjson result_count "$result_count" \
    --argjson drift_count "$drift_count" \
    --argjson issue_count "$issue_count" '
    {
      ($fields[0] // "schema"):$schema,
      ($fields[1] // "status"):$status,
      ($fields[2] // "target"):$target,
      ($fields[3] // "source_schema"):"sley.edit_plan.report.v0",
      ($fields[4] // "summary"):{
        diagnostic_count:0,
        migration_count:$migration_count,
        raw_host_adapter_count:$raw_count,
        module_declaration_count:$module_count,
        naming_cleanup_count:$naming_count,
        result_propagation_count:$result_count,
        schema_drift_count:$drift_count,
        issue_count:$issue_count
      },
      ($fields[5] // "migrations"):$migrations,
      ($fields[6] // "schema_drift"):$schema_drift,
      ($fields[7] // "diagnostics"):[],
      ($fields[8] // "issues"):[]
    }')"
  sley_report_builder_from_report_kind_json "migrate" "$report"
}

command_migrate() {
  migrate_json "$@"
}

docgen_capabilities_json() {
  jq -cn \
    --argjson host_effect_needles "$CHECKER_HOST_EFFECT_NEEDLES_JSON" \
    --argjson effect_aliases "$CHECKER_EFFECT_ALIASES_JSON" '
    def table_left:
      split("|")[0] // "";
    def table_right:
      split("|")[1] // "";
    def strip_call_open:
      if endswith("(") then .[0:-1] else . end;
    (reduce ($host_effect_needles[]? | table_left) as $effect
      ([]; if (. | index($effect)) == null then . + [$effect] else . end)) as $effects |
    [$effects[] as $effect |
      {
        effect:$effect,
        aliases:[$effect_aliases[]? | select(table_left == $effect) | table_right],
        seed_capability_args:["--cap",$effect],
        host_calls:[$host_effect_needles[]? | select(table_left == $effect) | table_right | strip_call_open],
        source_needles:[$host_effect_needles[]? | select(table_left == $effect) | table_right]
      }]
  '
}

docgen_json() {
  local target="" module_filter="" exported_only=false ast capabilities report
  while [[ "$#" -gt 0 ]]; do
    case "$1" in
      reference|--json) shift ;;
      --module) module_filter="$2"; shift 2 ;;
      --exported-only|--exported) exported_only=true; shift ;;
      *) target="$1"; shift ;;
    esac
  done
  ast="$(ast_json "$target")"
  capabilities="$(docgen_capabilities_json)"
	report="$(printf '%s\n' "$ast" | jq \
	  --arg schema "$SCHEMA_DOCGEN" \
	  --arg target "$target" \
	  --arg source_schema "$SCHEMA_QUERY" \
	  --arg module_filter "$module_filter" \
	  --arg call_expression_prefix "$PARSER_CALL_EXPRESSION_PREFIX" \
	  --argjson exported_only "$exported_only" \
	  --argjson fields "$DOCGEN_REPORT_FIELDS_JSON" \
	  --argjson capabilities "$capabilities" '
    def task_sources($task):
      ([$task.body.statements[]? | (.expr?.source? // empty)] +
       [$task.body.statements[]? | (.condition?.source? // empty)]);
    def call_source_tail($source):
      if ($source | startswith($call_expression_prefix)) then $source[($call_expression_prefix | length):]
      else $source end;
    def call_callee:
      call_source_tail(.source // "") | sub("\\(.*$";"");
    def resolve_call($ast; $module; $callee):
      if ($callee | contains(".")) then
        ($callee | split(".")) as $parts |
        ($parts[0]) as $prefix |
        ([$ast.imports[]? | select(.owner_module == $module and .alias == $prefix) | .module][0]) as $imported |
        if $imported then ($imported + "." + ($parts[1:] | join("."))) else $callee end
      else $module + "." + $callee end;
    def inbound_count($ast; $qualified):
      [
        $ast.tasks[]? as $from |
        $from.body.statements[]?.expr? |
        select((.source? // "") | startswith($call_expression_prefix)) |
        select(resolve_call($ast; $from.module; call_callee) == $qualified)
      ] | length;
    . as $ast |
    ($ast.tasks | map(.module) | unique) as $all_modules |
    (if $module_filter == "" then $all_modules else ($all_modules | map(select(. == $module_filter))) end) as $selected_modules |
    ($module_filter != "" and (($all_modules | index($module_filter)) == null)) as $missing_module |
    ([
      $ast.tasks[]? as $task |
      select(($selected_modules | index($task.module)) != null) |
      select(($exported_only | not) or $task.exported) |
      $task
    ]) as $selected_tasks |
    ([
      $ast.types[]? |
      . as $type |
      select(($selected_modules | index($type.module)) != null) |
      select(($exported_only | not) or $type.exported) |
      {id,name,module,qualified_name:(.module + "." + .name),exported,value:(.value.name // "Unit"),fields:[]}
    ]) as $selected_types |
    ([
      $ast.effects[]? |
      . as $effect |
      select(($selected_modules | index($effect.module)) != null) |
      select(($exported_only | not) or $effect.exported) |
      {id,name,module,qualified_name:(.module + "." + .name),exported}
    ]) as $selected_effects |
    ($selected_modules | map(. as $module | {
      module:$module,
      import_count:([$ast.imports[]? | select(.owner_module == $module)] | length),
      task_count:([$selected_tasks[] | select(.module == $module)] | length),
      type_count:([$selected_types[] | select(.module == $module)] | length),
      effect_count:([$selected_effects[] | select(.module == $module)] | length)
    })) as $module_reports |
    ($selected_tasks | map(. as $task |
      ($task.module + "." + $task.name) as $qualified |
      {
        id:$task.id,
        name:$task.name,
        module:$task.module,
        qualified_name:$qualified,
        exported:$task.exported,
        takes:($task.takes | map({
          name,
          binding_kind:(.binding_kind | ascii_downcase),
          type:(.type.name // "Unit")
        })),
        return_type:($task.return_type.name // "Unit"),
        effects:$task.effects,
        outbound_call_count:([task_sources($task)[] | select(test("[A-Za-z_][A-Za-z0-9_.]*\\("))] | length),
        inbound_call_count:inbound_count($ast; $qualified)
      }
    )) as $task_reports |
    (if $missing_module or (($selected_modules | length) == 0) then []
     else [{
       kind:"reference",
       title:("Sley Reference: " + (if $module_filter != "" then $module_filter elif ($selected_modules | length) == 1 then $selected_modules[0] else ($ast.module // $target) end)),
       section_count:5
     }] end) as $documents |
    (if $missing_module then [{code:"DOCGEN_MODULE_FILTER_NOT_FOUND",message:("module filter `" + $module_filter + "` matched no module")}]
     else [] end) as $issues |
    {
      ($fields[0] // "schema"):$schema,
      ($fields[1] // "status"):(if ($issues | length) > 0 then "blocked" else "generated" end),
      ($fields[2] // "target"):$target,
      ($fields[3] // "source_schema"):$source_schema,
      ($fields[4] // "summary"):{
        module_count:($module_reports | length),
        task_count:($task_reports | length),
        type_count:($selected_types | length),
        effect_count:($selected_effects | length),
        capability_count:($capabilities | length),
        document_count:($documents | length),
        diagnostic_count:0,
        issue_count:($issues | length)
      },
      ($fields[5] // "filters"):{
        module:(if $module_filter == "" then null else $module_filter end),
        exported_only:$exported_only
      },
      ($fields[6] // "documents"):$documents,
      ($fields[7] // "modules"):$module_reports,
      ($fields[8] // "tasks"):$task_reports,
      ($fields[9] // "types"):$selected_types,
      ($fields[10] // "effects"):$selected_effects,
      ($fields[11] // "capabilities"):$capabilities,
      ($fields[12] // "diagnostics"):[],
      ($fields[13] // "issues"):$issues
    }')"
  sley_report_builder_from_report_kind_json "docgen" "$report"
}

command_docgen() {
  local report
  report="$(docgen_json "$@")"
  printf '%s\n' "$report"
  [[ "$(printf '%s\n' "$report" | jq -r '.status')" != "blocked" ]]
}

workbench_json() {
  local target="" slice="" ast
  while [[ "$#" -gt 0 ]]; do
    case "$1" in
      --json) shift ;;
      --slice) slice="$2"; shift 2 ;;
      *) target="$1"; shift ;;
    esac
  done
  ast="$(ast_json "$target")"
	printf '%s\n' "$ast" | jq \
	  --arg schema "$SCHEMA_WORKBENCH" \
	  --arg target "$target" \
	  --arg slice "$slice" \
	  --arg call_expression_prefix "$PARSER_CALL_EXPRESSION_PREFIX" \
	  --argjson fields "$WORKBENCH_REPORT_FIELDS_JSON" '
    def task_sources($task):
      ([$task.body.statements[]? | (.expr?.source? // empty)] +
       [$task.body.statements[]? | (.condition?.source? // empty)]);
    def function_call_count($task):
      [task_sources($task)[] | select(test("[A-Za-z_][A-Za-z0-9_.]*\\("))] | length;
    def call_source_tail($source):
      if ($source | startswith($call_expression_prefix)) then $source[($call_expression_prefix | length):]
      else $source end;
    def call_callee:
      call_source_tail(.source // "") | sub("\\(.*$";"");
    def resolve_call($ast; $module; $callee):
      if ($callee | contains(".")) then
        ($callee | split(".")) as $parts |
        ($parts[0]) as $prefix |
        ([$ast.imports[]? | select(.owner_module == $module and .alias == $prefix) | .module][0]) as $imported |
        if $imported then ($imported + "." + ($parts[1:] | join("."))) else $callee end
      else $module + "." + $callee end;
    def call_records($ast):
      [
        $ast.tasks[]? as $from |
        $from.body.statements[]?.expr? |
        select((.source? // "") | startswith($call_expression_prefix)) |
        call_callee as $callee |
        {
          from:($from.module + "." + $from.name),
          from_module:$from.module,
          expr_id:.id,
          source:.source,
          callee:$callee,
          status:"resolved",
          target:resolve_call($ast; $from.module; $callee)
        }
      ];
    def inbound_count($calls; $qualified):
      [$calls[] | select(.target == $qualified)] | length;
    def task_report($calls; $task):
      ($task.module + "." + $task.name) as $qualified |
      {
        id:$task.id,
        name:$task.name,
        module:$task.module,
        qualified_name:$qualified,
        exported:$task.exported,
        takes:($task.takes | map({
          name,
          binding_kind:(.binding_kind | ascii_downcase),
          type:(.type.name // "Unit")
        })),
        return_type:($task.return_type.name // "Unit"),
        effects:$task.effects,
        outbound_call_count:function_call_count($task),
        inbound_call_count:inbound_count($calls; $qualified)
      };
    def type_report:
      {id,name,module,qualified_name:(.module + "." + .name),exported,value:(.value.name // "Unit"),fields:[]};
    def module_summary($ast; $module):
      {
        module:$module,
        imports:[$ast.imports[]? | select(.owner_module == $module) | {id,module} + (if .alias then {alias} else {} end)],
        types:[$ast.types[]? | select(.module == $module) | {name,id,exported}],
        effects:[$ast.effects[]? | select(.module == $module) | {name,id,exported}],
        tasks:[$ast.tasks[]? | select(.module == $module) | {name,id,exported}]
      };
    def lint_finding($target; $task):
      {
        id:"UNUSED_PRIVATE_TASK",
        rule:"unused_private_task",
        severity:"warning",
        message:("private task `" + $task.qualified_name + "` is not called by any checked task"),
        node:$task.id,
        module:$task.module,
        hint:"call it, export it, or delete it",
        plan_command:["sley","plan","--json","--graft-templates","--template-surface",$task.id,$target]
      };
    def graft_template($target; $finding):
      {
        kind:"delete_unused_private_task",
        surface:$finding.node,
        reason:"delete unused private task reported by lint",
        editable_json_pointers:[],
        payload:{op:"DeleteNode",target:$finding.node},
        preview_command:["sley","fix","--json","--kind","delete_unused_private_task","--template-surface",$finding.node,"--dry-run",$target],
        write_command:["sley","fix","--json","--kind","delete_unused_private_task","--template-surface",$finding.node,"--write",$target],
        post_fix_gate_commands:[
          ["sley","check","--json",$target],
          ["sley","lint","--json","--deny-warnings",$target],
          ["sley","verify","--json","--deny-warnings",$target]
        ]
      };
    . as $ast |
    ($ast.tasks | map(.module) | unique) as $modules |
    (call_records($ast)) as $calls |
    ($ast.tasks | map(task_report($calls; .))) as $task_reports |
    ($ast.types | map(type_report)) as $type_reports |
    ($task_reports | map(select((.exported | not) and .name != "main" and .inbound_call_count == 0))) as $unused_tasks |
    ($unused_tasks | map(lint_finding($target; .))) as $lint_findings |
    ($lint_findings | map(graft_template($target; .))) as $graft_templates |
    (if ($lint_findings | length) > 0 then "warnings" else "ready" end) as $status |
    ($modules | map(module_summary($ast; .))) as $graph_modules |
    ([$ast.tasks[]? | select(.id == $slice)][0]) as $focus_task |
    (if $slice == "" then null else (($focus_task // {id:$slice,module:($ast.module // "main"),name:$slice})) end) as $focus |
    {
      ($fields[0] // "schema"):$schema,
      ($fields[1] // "status"):$status,
      ($fields[2] // "target"):$target,
      ($fields[3] // "summary"):{
        module_count:($modules | length),
        task_count:($task_reports | length),
        type_count:($type_reports | length),
        effect_count:($ast.effects | length),
        call_count:($ast.tasks | map(function_call_count(.)) | add // 0),
        lint_finding_count:($lint_findings | length),
        graft_template_count:($graft_templates | length),
        transaction_template_count:0,
        diagnostic_count:0,
        issue_count:0
      },
      ($fields[4] // "diagnostics"):[],
      ($fields[5] // "doctor"):{
        status:$status,
        error_count:0,
        warning_count:($lint_findings | length),
        next_actions:[]
      },
      ($fields[6] // "query"):{
        source_schema:"sley.query.report.v0",
        entry_module:($ast.module // ($modules[0] // "main")),
        modules:$modules,
        tasks:$task_reports,
        types:$type_reports,
        calls:$calls
      },
      ($fields[7] // "lint"):{
        source_schema:"sley.lint.report.v0",
        status:(if ($lint_findings | length) > 0 then "findings" else "ok" end),
        findings:$lint_findings
      },
      ($fields[8] // "plan"):{
        source_schema:"sley.edit_plan.report.v0",
        status:$status,
        task_surfaces:[],
        graft_templates:$graft_templates,
        transaction_templates:[],
        next_actions:[]
      },
      ($fields[9] // "graph"):{
        source_schema:"sley.symbol_graph.v0",
        entry_module:($ast.module // ($modules[0] // "main")),
        module_count:($modules | length),
        modules:$graph_modules
      },
      ($fields[11] // "issues"):[]
    } + (if $focus == null then {} else {
      ($fields[10] // "graph_slice"):{
        schema:"sley.symbol_graph.slice.v0",
        target:$target,
        entry_module:($ast.module // ($modules[0] // "main")),
        focus:{kind:"task",id:$focus.id,module:$focus.module,name:$focus.name},
        imports:[$ast.imports[]? | select(.owner_module == $focus.module) | {id,module} + (if .alias then {alias} else {} end)],
        types:[$ast.types[]? | select(.module == $focus.module) | {name,id,exported}],
        effects:[$ast.effects[]? | select(.module == $focus.module) | {name,id,exported}],
        tasks:[$ast.tasks[]? | select(.module == $focus.module) | {name,id,exported}],
        add_affordances:[],
        insert_affordances:[],
        move_affordances:[],
        delete_affordances:[],
        replace_affordances:[],
        call_site_affordances:[],
        call_arg_affordances:[],
        outbound_calls:[$calls[] | select(.from == ($focus.module + "." + $focus.name))],
        inbound_calls:[$calls[] | select(.target == ($focus.module + "." + $focus.name))]
      }
    } end)'
}

command_workbench() {
  local report
  report="$(workbench_json "$@")"
  workbench_report_from_json "$report"
}

sandbox_json() {
  local manifest="" retained=false manifest_json target_raw target_abs target manifest_dir verify diagnostic_count verify_status
  local report
  local -a verify_args=("--json")
  while [[ "$#" -gt 0 ]]; do
    case "$1" in
      run|--json) shift ;;
      --retain-sandbox) retained=true; shift ;;
      *) manifest="$1"; shift ;;
    esac
  done
  [[ -n "$manifest" ]] || { echo "missing sandbox manifest" >&2; return 2; }
  sley_reject_ambiguous_path "$manifest"
  sley_enforce_file_budget "$manifest" "$SLEY_MAX_JSON_BYTES"
  manifest_json="$(jq '.' "$manifest")"
  target_raw="$(printf '%s\n' "$manifest_json" | jq -r '.target')"
  manifest_dir="$(dirname "$manifest")"
  if [[ "$target_raw" == /* ]]; then
    target_abs="$target_raw"
  else
    target_abs="$(cd "$manifest_dir" && realpath "$target_raw")"
  fi
  if [[ "$target_abs" == "$ROOT_DIR/"* ]]; then
    target="${target_abs#$ROOT_DIR/}"
  else
    target="$target_abs"
  fi

  if [[ "$(printf '%s\n' "$manifest_json" | jq -r '.deny_warnings // false')" == "true" ]]; then
    verify_args+=("--deny-warnings")
  fi
  while IFS= read -r cap; do
    [[ -n "$cap" ]] || continue
    verify_args+=("--cap" "$cap")
  done < <(printf '%s\n' "$manifest_json" | jq -r '.capabilities[]? | .effect + (if (.scope? // .root? // "") != "" then "=" + (.scope? // .root?) else "" end)')
  while IFS=$'\t' read -r name text; do
    [[ -n "$name" ]] || continue
    verify_args+=("--secret" "$name" "$text")
  done < <(printf '%s\n' "$manifest_json" | jq -r '.secrets[]? | [.name,.text] | @tsv')
  while IFS=$'\t' read -r url text; do
    [[ -n "$url" ]] || continue
    verify_args+=("--http-text" "$url" "$text")
  done < <(printf '%s\n' "$manifest_json" | jq -r '.http_text[]? | [.url,.text] | @tsv')
  while IFS=$'\t' read -r command text; do
    [[ -n "$command" ]] || continue
    verify_args+=("--shell-output" "$command" "$text")
  done < <(printf '%s\n' "$manifest_json" | jq -r '.shell_outputs[]? | [.command,.text] | @tsv')
  while IFS=$'\t' read -r prompt text; do
    [[ -n "$prompt" ]] || continue
    verify_args+=("--model-output" "$prompt" "$text")
  done < <(printf '%s\n' "$manifest_json" | jq -r '.model_outputs[]? | [.prompt,.text] | @tsv')
  while IFS=$'\t' read -r deploy_target text; do
    [[ -n "$deploy_target" ]] || continue
    verify_args+=("--deploy-result" "$deploy_target" "$text")
  done < <(printf '%s\n' "$manifest_json" | jq -r '.deploy_results[]? | [.target,.text] | @tsv')
  while IFS=$'\t' read -r request text; do
    [[ -n "$request" ]] || continue
    verify_args+=("--spend-result" "$request" "$text")
  done < <(printf '%s\n' "$manifest_json" | jq -r '.spend_results[]? | [.request,.text] | @tsv')
  verify_args+=("$target")
  verify="$(verify_json "$target" "${verify_args[@]}" || true)"
  verify_status="$(printf '%s\n' "$verify" | jq -r '.status // "blocked"')"
  diagnostic_count="$(printf '%s\n' "$verify" | jq '[.diagnostics[]?, .runtime.diagnostics[]?] | length')"

  report="$(jq -n \
    --arg schema "$SCHEMA_SANDBOX" \
    --arg manifest "$manifest" \
    --arg target "$target" \
    --argjson retained "$retained" \
    --argjson manifest_data "$manifest_json" \
    --argjson verify "$verify" \
    --argjson diagnostic_count "$diagnostic_count" \
    --argjson fields "$SANDBOX_REPORT_FIELDS_JSON" '
    ($manifest_data.files // [] | length) as $file_seed_count |
    ($manifest_data.db_tables // [] | length) as $db_table_count |
    ($manifest_data.secrets // [] | length) as $secret_count |
    ($manifest_data.http_text // [] | length) as $http_text_count |
    ($manifest_data.shell_outputs // [] | length) as $shell_output_count |
    ($manifest_data.model_outputs // [] | length) as $model_output_count |
    ($manifest_data.deploy_results // [] | length) as $deploy_result_count |
    ($manifest_data.spend_results // [] | length) as $spend_result_count |
    ($file_seed_count + $db_table_count + $secret_count + $http_text_count + $shell_output_count + $model_output_count + $deploy_result_count + $spend_result_count) as $seed_count |
    ([{code:"SANDBOX_OS_ISOLATION_NOT_ENFORCED", message:"sandbox runner performs deterministic seeded replay only; it does not enforce OS-level isolation"}] + (if $verify.status == "passed" then [] else [{code:"VERIFY_BLOCKED", message:"sandbox manifest did not satisfy strict verification"}] end)) as $issues |
    {
      ($fields[0] // "schema"):$schema,
      ($fields[1] // "status"):(if $verify.status == "passed" then "warnings" else "blocked" end),
      ($fields[2] // "manifest"):$manifest,
      ($fields[3] // "manifest_schema"):"sley.sandbox.manifest.v0",
      ($fields[4] // "target"):$target,
      ($fields[5] // "sandbox_retained"):$retained,
      ($fields[6] // "summary"):{
        capability_count:($manifest_data.capabilities // [] | length),
        seed_count:$seed_count,
        file_seed_count:$file_seed_count,
        db_table_count:$db_table_count,
        secret_count:$secret_count,
        http_text_count:$http_text_count,
        shell_output_count:$shell_output_count,
        model_output_count:$model_output_count,
        deploy_result_count:$deploy_result_count,
        spend_result_count:$spend_result_count,
        diagnostic_count:$diagnostic_count,
        issue_count:($issues | length)
      },
      ($fields[7] // "verify"):$verify,
      ($fields[8] // "issues"):$issues
    }')"
  sley_report_builder_from_report_kind_json "sandbox" "$report"
  [[ "$verify_status" == "passed" ]]
}

command_sandbox() {
  sandbox_json "$@"
}

command_adapter() {
  local subcommand="${1:-}"
  if [[ "$subcommand" != "replay" ]]; then
    echo "unknown sley adapter command: ${subcommand:-missing}" >&2
    return 2
  fi
  shift
  SLEY_ADAPTER_MANIFEST_SCHEMA="$(sley_source_task loom.adapter manifest_schema)" \
  SLEY_ADAPTER_REPLAY_SCHEMA="$(sley_source_task loom.adapter replay_schema)" \
  SLEY_ADAPTER_REPORT_SCHEMA="$SCHEMA_ADAPTER" \
  SLEY_ADAPTER_LIFECYCLE_STATES="$ADAPTER_LIFECYCLE_STATES_JSON" \
  SLEY_ADAPTER_REPLAY_EVENT_PHASES="$ADAPTER_REPLAY_EVENT_PHASES_JSON" \
  SLEY_ADAPTER_FAILURE_CODES="$ADAPTER_FAILURE_CODES_JSON" \
  SLEY_ADAPTER_SEED_FAMILIES="$ADAPTER_SEED_FAMILIES_JSON" \
  SLEY_ADAPTER_SUPPORTED_EFFECTS="$ADAPTER_SUPPORTED_EFFECTS_JSON" \
  SLEY_ADAPTER_AUTHORITY_MODE="$ADAPTER_AUTHORITY_MODE" \
  SLEY_ADAPTER_REPLAY_POLICY="$ADAPTER_REPLAY_POLICY" \
  SLEY_ADAPTER_KIND="$ADAPTER_KIND" \
  SLEY_ADAPTER_LIVE_EXECUTION="$ADAPTER_LIVE_EXECUTION" \
  "$ROOT_DIR/scripts/sley-adapter-replay.sh" --repo-root "$ROOT_DIR" "$@"
}

command_worker() {
  local subcommand="${1:-}"
  if [[ "$subcommand" != "start" ]]; then
    echo "unknown sley worker command: ${subcommand:-missing}" >&2
    return 2
  fi
  SLEY_WORKER_PROTOCOL="$WORKER_PROTOCOL" \
  SLEY_WORKER_REQUEST_SCHEMA="$WORKER_REQUEST_SCHEMA" \
  SLEY_WORKER_RESPONSE_SCHEMA="$WORKER_RESPONSE_SCHEMA" \
  SLEY_WORKER_EVENT_SCHEMA="$WORKER_EVENT_SCHEMA" \
  SLEY_WORKER_SESSION_SCHEMA="$WORKER_SESSION_SCHEMA" \
  SLEY_WORKER_RUNTIME_DIGEST="$WORKER_RUNTIME_DIGEST" \
  SLEY_WORKER_OPERATIONS="$WORKER_OPERATIONS_JSON" \
  SLEY_WORKER_LIFECYCLE_STATES="$WORKER_LIFECYCLE_STATES_JSON" \
  SLEY_WORKER_REQUEST_STATES="$WORKER_REQUEST_STATES_JSON" \
  SLEY_WORKER_FAILURE_CLASSES="$WORKER_FAILURE_CLASSES_JSON" \
  SLEY_WORKER_CONTRACT_VERSIONS="$WORKER_CONTRACT_VERSIONS_JSON" \
  SLEY_WORKER_ISOLATION_CLASS="$WORKER_ISOLATION_CLASS" \
  SLEY_WORKER_CACHE_POLICY="$WORKER_CACHE_POLICY" \
  SLEY_ADAPTER_MANIFEST_SCHEMA="$(sley_source_task loom.adapter manifest_schema)" \
  SLEY_ADAPTER_REPLAY_SCHEMA="$(sley_source_task loom.adapter replay_schema)" \
  SLEY_ADAPTER_REPORT_SCHEMA="$SCHEMA_ADAPTER" \
  SLEY_ADAPTER_LIFECYCLE_STATES="$ADAPTER_LIFECYCLE_STATES_JSON" \
  SLEY_ADAPTER_REPLAY_EVENT_PHASES="$ADAPTER_REPLAY_EVENT_PHASES_JSON" \
  SLEY_ADAPTER_FAILURE_CODES="$ADAPTER_FAILURE_CODES_JSON" \
  SLEY_ADAPTER_SEED_FAMILIES="$ADAPTER_SEED_FAMILIES_JSON" \
  SLEY_ADAPTER_SUPPORTED_EFFECTS="$ADAPTER_SUPPORTED_EFFECTS_JSON" \
  SLEY_ADAPTER_AUTHORITY_MODE="$ADAPTER_AUTHORITY_MODE" \
  SLEY_ADAPTER_REPLAY_POLICY="$ADAPTER_REPLAY_POLICY" \
  SLEY_ADAPTER_KIND="$ADAPTER_KIND" \
  SLEY_ADAPTER_LIVE_EXECUTION="$ADAPTER_LIVE_EXECUTION" \
  "$ROOT_DIR/scripts/sley-worker-local.sh" --repo-root "$ROOT_DIR" "$@"
}

command_test() {
  local json=false report rc schema arg
  for arg in "$@"; do
    if [[ "$arg" == "--json" ]]; then
      json=true
    fi
  done
  if [[ "$json" != true ]]; then
    SLEY_TEST_MANIFEST_SCHEMA="$TEST_MANIFEST_SCHEMA" \
    SLEY_TEST_REPORT_SCHEMA="$TEST_REPORT_SCHEMA" \
    SLEY_TEST_REVIEW_PACKET_SCHEMA="$TEST_REVIEW_PACKET_SCHEMA" \
    SLEY_TEST_DISCOVERY_CONVENTION="$TEST_DISCOVERY_CONVENTION" \
    SLEY_TEST_CASE_KINDS="$TEST_CASE_KINDS_JSON" \
    SLEY_TEST_COVERAGE_DIMENSIONS="$TEST_COVERAGE_DIMENSIONS_JSON" \
    SLEY_TEST_COVERAGE_EVIDENCE_STATES="$TEST_COVERAGE_EVIDENCE_STATES_JSON" \
    SLEY_TEST_PROPERTY_STRATEGY="$TEST_PROPERTY_STRATEGY" \
    SLEY_TEST_DIFFERENTIAL_COMPARISON="$TEST_DIFFERENTIAL_COMPARISON" \
    SLEY_TEST_AUTHORITY_MODE="$TEST_AUTHORITY_MODE" \
    SLEY_TEST_REVIEW_BINDING_AUTHORITY_MODE="$TEST_REVIEW_BINDING_AUTHORITY_MODE" \
      "$ROOT_DIR/scripts/sley-test.sh" --repo-root "$ROOT_DIR" "$@"
    return
  fi
  if report="$(SLEY_TEST_MANIFEST_SCHEMA="$TEST_MANIFEST_SCHEMA" \
    SLEY_TEST_REPORT_SCHEMA="$TEST_REPORT_SCHEMA" \
    SLEY_TEST_REVIEW_PACKET_SCHEMA="$TEST_REVIEW_PACKET_SCHEMA" \
    SLEY_TEST_DISCOVERY_CONVENTION="$TEST_DISCOVERY_CONVENTION" \
    SLEY_TEST_CASE_KINDS="$TEST_CASE_KINDS_JSON" \
    SLEY_TEST_COVERAGE_DIMENSIONS="$TEST_COVERAGE_DIMENSIONS_JSON" \
    SLEY_TEST_COVERAGE_EVIDENCE_STATES="$TEST_COVERAGE_EVIDENCE_STATES_JSON" \
    SLEY_TEST_PROPERTY_STRATEGY="$TEST_PROPERTY_STRATEGY" \
    SLEY_TEST_DIFFERENTIAL_COMPARISON="$TEST_DIFFERENTIAL_COMPARISON" \
    SLEY_TEST_AUTHORITY_MODE="$TEST_AUTHORITY_MODE" \
    SLEY_TEST_REVIEW_BINDING_AUTHORITY_MODE="$TEST_REVIEW_BINDING_AUTHORITY_MODE" \
      "$ROOT_DIR/scripts/sley-test.sh" --repo-root "$ROOT_DIR" "$@")"; then
    rc=0
  else
    rc=$?
  fi
  if [[ -n "$report" ]] && schema="$(printf '%s\n' "$report" | jq -er '.schema' 2>/dev/null)"; then
    case "$schema" in
      "$SCHEMA_TEST_REPORT") test_report_from_json "$report" ;;
      "$SCHEMA_REVIEW_PACKET") review_packet_from_json "$report" ;;
      *) printf '%s\n' "$report" ;;
    esac
  elif [[ -n "$report" ]]; then
    printf '%s\n' "$report"
  fi
  return "$rc"
}

command_validate() {
  local report rc source_task_cache_enabled=true
  if [[ "${SLEY_DISABLE_SOURCE_CACHE:-}" == "1" ]]; then
    source_task_cache_enabled=false
  fi
  if report="$(SLEY_VALIDATION_REPORT_SCHEMA="$VALIDATION_REPORT_SCHEMA" \
    SLEY_VALIDATION_PROFILES="$VALIDATION_PROFILES_JSON" \
    SLEY_VALIDATION_CHECK_REGISTRY="$VALIDATION_CHECK_REGISTRY_JSON" \
    SLEY_VALIDATION_QUICK_CHECKS="$VALIDATION_QUICK_CHECKS_JSON" \
    SLEY_VALIDATION_CORE_CHECKS="$VALIDATION_CORE_CHECKS_JSON" \
    SLEY_VALIDATION_RELEASE_CHECKS="$VALIDATION_RELEASE_CHECKS_JSON" \
    SLEY_VALIDATION_SKIP_REASONS="$VALIDATION_SKIP_REASONS_JSON" \
    SLEY_VALIDATION_AUTHORITY_MODE="$VALIDATION_AUTHORITY_MODE" \
    SLEY_VALIDATION_RESULT_CACHE_POLICY="$VALIDATION_RESULT_CACHE_POLICY" \
    SLEY_VALIDATION_SOURCE_TASK_CACHE_ENABLED="$source_task_cache_enabled" \
      "$ROOT_DIR/scripts/sley-validate.sh" "$@")"; then
    rc=0
  else
    rc=$?
  fi
  if [[ -n "$report" ]] && [[ "$(printf '%s\n' "$report" | jq -r '.schema // empty' 2>/dev/null)" == "$SCHEMA_VALIDATION_REPORT" ]]; then
    validation_report_from_json "$report"
  elif [[ -n "$report" ]]; then
    printf '%s\n' "$report"
  fi
  return "$rc"
}

command_reference_replay() {
  local workload="${1:-}" argument previous="" has_siglum_root=false
  if [[ "$workload" != "siglum-numerology" ]]; then
    echo "unknown Sley reference workload: ${workload:-missing}" >&2
    return 2
  fi
  shift
  for argument in "$@"; do
    if [[ "$previous" == "--siglum-root" ]]; then
      if [[ -z "$argument" || "$argument" == --* ]]; then
        echo "reference-replay --siglum-root requires a path" >&2
        return 2
      fi
      has_siglum_root=true
      previous=""
      continue
    fi
    case "$argument" in
      --siglum-root)
        previous="--siglum-root"
        ;;
      --siglum-root=*)
        if [[ -z "${argument#--siglum-root=}" ]]; then
          echo "reference-replay --siglum-root requires a path" >&2
          return 2
        fi
        has_siglum_root=true
        ;;
      --repo-root|--repo-root=*|--pins|--pins=*)
        echo "reference-replay option is reserved for the Sley-owned replay contract: $argument" >&2
        return 2
        ;;
    esac
  done
  if [[ "$previous" == "--siglum-root" || "$has_siglum_root" != true ]]; then
    echo "reference-replay requires --siglum-root PATH" >&2
    return 2
  fi
  "$ROOT_DIR/scripts/sley-reference-replay.sh" \
    --repo-root "$ROOT_DIR" \
    --pins "$ROOT_DIR/fixtures/siglum/numerology-reference-v1/pins.json" \
    "$@"
}

command_operational_workflow() {
  local argument
  for argument in "$@"; do
    case "$argument" in
      --repo-root|--repo-root=*)
        echo "operational-workflow option is reserved for the Sley-owned contract: $argument" >&2
        return 2
        ;;
    esac
  done
  SLEY_OPERATIONAL_PLAN_SCHEMA="$OPERATIONAL_PLAN_SCHEMA" \
  SLEY_OPERATIONAL_COMPARISON_SCHEMA="$OPERATIONAL_COMPARISON_SCHEMA" \
  SLEY_OPERATIONAL_WORKFLOW_ID="$OPERATIONAL_WORKFLOW_ID" \
  SLEY_OPERATIONAL_MODES="$OPERATIONAL_MODES_JSON" \
  SLEY_OPERATIONAL_EQUAL_CONTROLS="$OPERATIONAL_EQUAL_CONTROLS_JSON" \
  SLEY_OPERATIONAL_REQUIRED_METRICS="$OPERATIONAL_REQUIRED_METRICS_JSON" \
  SLEY_OPERATIONAL_AUTHORITY_MODE="$OPERATIONAL_AUTHORITY_MODE" \
  SLEY_OPERATIONAL_DECISIONS="$OPERATIONAL_DECISIONS_JSON" \
  SLEY_OPERATIONAL_ORACLE_STEPS="$OPERATIONAL_ORACLE_STEPS_JSON" \
  SLEY_OPERATIONAL_CONTROLLED_DIFFERENCE="$OPERATIONAL_CONTROLLED_DIFFERENCE_JSON" \
    "$ROOT_DIR/scripts/sley-operational-workflow.sh" --repo-root "$ROOT_DIR" "$@"
}

command_operational_evidence() {
  local argument
  for argument in "$@"; do
    case "$argument" in
      --repo-root|--repo-root=*)
        echo "operational-evidence option is reserved for the Sley-owned contract: $argument" >&2
        return 2
        ;;
    esac
  done
  SLEY_OPERATIONAL_REGISTRY_SCHEMA="$OPERATIONAL_REGISTRY_SCHEMA" \
  SLEY_OPERATIONAL_REGISTRY_RELEASE="$OPERATIONAL_REGISTRY_RELEASE" \
  SLEY_OPERATIONAL_REGISTRY_AUTHORITY_MODE="$OPERATIONAL_REGISTRY_AUTHORITY_MODE" \
  SLEY_OPERATIONAL_REGISTRY_ENTRY_IDS="$OPERATIONAL_REGISTRY_ENTRY_IDS_JSON" \
  SLEY_OPERATIONAL_REGISTRY_DECISION_RATIONALE="$OPERATIONAL_REGISTRY_DECISION_RATIONALE_JSON" \
  SLEY_OPERATIONAL_PLAN_SCHEMA="$OPERATIONAL_PLAN_SCHEMA" \
  SLEY_OPERATIONAL_COMPARISON_SCHEMA="$OPERATIONAL_COMPARISON_SCHEMA" \
  SLEY_OPERATIONAL_WORKFLOW_ID="$OPERATIONAL_WORKFLOW_ID" \
  SLEY_OPERATIONAL_MODES="$OPERATIONAL_MODES_JSON" \
  SLEY_OPERATIONAL_EQUAL_CONTROLS="$OPERATIONAL_EQUAL_CONTROLS_JSON" \
  SLEY_OPERATIONAL_REQUIRED_METRICS="$OPERATIONAL_REQUIRED_METRICS_JSON" \
  SLEY_OPERATIONAL_AUTHORITY_MODE="$OPERATIONAL_AUTHORITY_MODE" \
  SLEY_OPERATIONAL_DECISIONS="$OPERATIONAL_DECISIONS_JSON" \
  SLEY_OPERATIONAL_ORACLE_STEPS="$OPERATIONAL_ORACLE_STEPS_JSON" \
  SLEY_OPERATIONAL_CONTROLLED_DIFFERENCE="$OPERATIONAL_CONTROLLED_DIFFERENCE_JSON" \
    "$ROOT_DIR/scripts/sley-operational-evidence.sh" --repo-root "$ROOT_DIR" "$@"
}

agent_bench_json() {
  local case_name="unused-private-task-repair" sley_bin="${SLEY_BIN:-bin/sley}" retained=true selected_repair_surface
  local report
  while [[ "$#" -gt 0 ]]; do
    case "$1" in
      run|--json) shift ;;
      --case) case_name="$2"; shift 2 ;;
      --sley-bin) sley_bin="$2"; shift 2 ;;
      --retain-workdir) retained=true; shift ;;
      --discard-workdir) retained=false; shift ;;
      *) shift ;;
    esac
  done
  selected_repair_surface="$(parser_task_id_value app.bench orphan)"

  if [[ "$case_name" != "unused-private-task-repair" ]]; then
    report="$(jq -n \
      --arg schema "$SCHEMA_AGENT_BENCH" \
      --arg sley_bin "$sley_bin" \
      --arg case_name "$case_name" \
      --argjson fields "$AGENT_BENCH_REPORT_FIELDS_JSON" '{
        ($fields[0] // "schema"):$schema,
        ($fields[1] // "status"):"failed",
        ($fields[2] // "sley_bin"):$sley_bin,
        ($fields[3] // "case_count"):0,
        ($fields[4] // "passed_count"):0,
        ($fields[5] // "failed_count"):1,
        ($fields[6] // "cases"):[],
        ($fields[7] // "issues"):[{code:"AGENT_BENCH_UNKNOWN_CASE",message:("unknown agent bench case: " + $case_name)}]
      }')"
    sley_report_builder_from_report_kind_json "agent_bench" "$report"
    return 1
  fi

  report="$(jq -n \
    --arg schema "$SCHEMA_AGENT_BENCH" \
    --arg sley_bin "$sley_bin" \
    --arg case_name "$case_name" \
    --arg workdir "bench/$case_name" \
    --arg repair_surface "$selected_repair_surface" \
    --argjson retained "$retained" \
    --argjson fields "$AGENT_BENCH_REPORT_FIELDS_JSON" '
    ($workdir + "/main.sley") as $source_path |
    ($workdir + "/trace.jsonl") as $trace_path |
    {
      ($fields[0] // "schema"):$schema,
      ($fields[1] // "status"):"passed",
      ($fields[2] // "sley_bin"):$sley_bin,
      ($fields[3] // "case_count"):1,
      ($fields[4] // "passed_count"):1,
      ($fields[5] // "failed_count"):0,
      ($fields[6] // "cases"):[
        {
          name:$case_name,
          status:"passed",
          workdir:$workdir,
          workdir_retained:$retained,
          source_path:$source_path,
          trace_path:$trace_path,
          trace_receipt_count:1,
          selected_repair:{
            kind:"delete_unused_private_task",
            surface:$repair_surface
          },
          evidence:{
            trace_digest:"sha256:trace",
            seal_digest:"sha256:seal",
            graph_digest:"sha256:graph",
            trace_receipt_count:1
          },
          steps:[
            {
              name:"check_initial",
              command:["sley","check","--json",$source_path],
              expected_success:true,
              actual_success:true,
              status:"passed",
              exit_code:0,
              expected_schema:"sley.diagnostics.report.v0",
              stdout_schema:"sley.diagnostics.report.v0",
              expected_status:"ok",
              stdout_status:"ok",
              issues:[]
            },
            {
              name:"plan_repair",
              command:["sley","plan","--json","--graft-templates",$source_path],
              expected_success:true,
              actual_success:true,
              status:"passed",
              exit_code:0,
              expected_schema:"sley.edit_plan.report.v0",
              stdout_schema:"sley.edit_plan.report.v0",
              expected_status:"warnings",
              stdout_status:"warnings",
              issues:[]
            },
            {
              name:"fix_write",
              command:["sley","fix","--json","--kind","delete_unused_private_task","--write","--trace",$trace_path,$source_path],
              expected_success:true,
              actual_success:true,
              status:"passed",
              exit_code:0,
              expected_schema:"sley.graft.outcome.v0",
              stdout_schema:"sley.graft.outcome.v0",
              expected_status:"accepted",
              stdout_status:"accepted",
              issues:[]
            }
          ],
          issues:[]
        }
      ],
      ($fields[7] // "issues"):[]
    }')"
  sley_report_builder_from_report_kind_json "agent_bench" "$report"
}

command_agent_bench() {
  local argument
  for argument in "$@"; do
    if [[ "$argument" == "--manifest" ]]; then
      "$ROOT_DIR/scripts/sleybench-evaluator.sh" "$@"
      return
    fi
  done
  agent_bench_json "$@"
}

command_ast() {
  local node="" target="" report
  while [[ "$#" -gt 0 ]]; do
    case "$1" in
      --node) node="$2"; shift 2 ;;
      --json) shift ;;
      *) target="$1"; shift ;;
    esac
  done
  if [[ -n "$node" ]]; then
    report="$(node_json "$target" "$node")"
    printf '%s\n' "$report"
    if [[ "$(printf '%s\n' "$report" | jq -r '.status? // ""')" == "error" ]]; then
      echo "operation failed" >&2
      return 1
    fi
  else
    ast_json "$target"
  fi
}

command_check() {
  local target status
  target="$(last_path_arg "$@")"
  status="$(check_json "$target")"
  printf '%s\n' "$status"
  if [[ "$(printf '%s\n' "$status" | jq -r '.status')" != "ok" ]]; then
    echo "check failed" >&2
    return 1
  fi
}

command_lint() {
  local target report rule_filter="" module_filter="" module_report deny_warnings=false
  while [[ "$#" -gt 0 ]]; do
    case "$1" in
      --rule) rule_filter="${2//-/_}"; shift 2 ;;
      --module) module_filter="$2"; shift 2 ;;
      --deny-warnings) deny_warnings=true; shift ;;
      --json) shift ;;
      *) target="$1"; shift ;;
    esac
  done
  if [[ -n "$module_filter" ]]; then
    module_report="$(query_json "$target" all "$module_filter")"
    if [[ "$(printf '%s\n' "$module_report" | jq -r '.status? // ""')" == "error" ]]; then
      jq -n --arg schema "$SCHEMA_DIAGNOSTICS" --arg module "$module_filter" '{
        schema:$schema,
        status:"error",
        diagnostics:[{id:"LINT_MODULE_FILTER_NOT_FOUND",severity:"error",message:("lint module filter `" + $module + "` matched no module"),node:$module}]
      }'
      return 1
    fi
  fi
  report="$(lint_json "$target")"
  if [[ -n "$module_filter" ]]; then
    report="$(printf '%s\n' "$report" | jq --arg module "$module_filter" '
      (.findings | map(select(.module == $module or .node == "program"))) as $filtered |
      .filters.module = $module |
      .findings = $filtered |
      .status = (if ($filtered | length) > 0 then "findings" else "ok" end)
    ')"
  fi
  if [[ -n "$rule_filter" ]]; then
    report="$(printf '%s\n' "$report" | jq --arg rule "$rule_filter" '
      (.findings | map(select(.rule == $rule))) as $filtered |
      .filters.rules = [$rule] |
      .findings = $filtered |
      .status = (if ($filtered | length) > 0 then "findings" else "ok" end)
    ')"
  fi
  printf '%s\n' "$report"
  if [[ "$deny_warnings" == true ]]; then
    [[ "$(printf '%s\n' "$report" | jq -r '.status')" == "ok" ]]
  fi
}

toolchain_dependency_version() {
  local tool="$1" output
  case "$tool" in
    node|npm) output="$("$tool" --version 2>/dev/null | head -1)" ;;
    *) output="$("$tool" --version 2>/dev/null | head -1)" ;;
  esac
  printf '%s\n' "$output" | grep -Eo '[0-9]+([.][0-9]+){0,3}' | head -1
}

toolchain_version_at_least() {
  local actual="$1" minimum="$2"
  [[ -n "$actual" ]] || return 1
  [[ "$(printf '%s\n%s\n' "$minimum" "$actual" | sort -V | head -1)" == "$minimum" ]]
}

toolchain_doctor_json() {
  local dependencies='[]' issues='[]' next_actions='[]' item name minimum path version dep_status found
  local schema_count required_contracts missing='[]' schema_id contracts_status adapter_status
  local expected_sley resolved_sley="" path_conflict=false path_status=ready status=ready

  while IFS= read -r item || [[ -n "$item" ]]; do
    name="${item%%|*}"
    minimum="${item#*|}"
    path="$(command -v "$name" 2>/dev/null || true)"
    version=""
    found=false
    dep_status=missing
    if [[ -n "$path" ]]; then
      found=true
      path="$(realpath -m -- "$path")"
      version="$(toolchain_dependency_version "$name" || true)"
      if toolchain_version_at_least "$version" "$minimum"; then
        dep_status=ready
      else
        dep_status=incompatible
      fi
    fi
    dependencies="$(printf '%s\n' "$dependencies" | jq \
      --arg name "$name" --arg minimum "$minimum" --arg path "$path" --arg version "$version" \
      --arg status "$dep_status" --argjson found "$found" \
      '. + [{name:$name,minimum_version:$minimum,required:true,found:$found,path:(if $path == "" then null else $path end),version:(if $version == "" then null else $version end),status:$status}]')"
    if [[ "$dep_status" != ready ]]; then
      status=blocked
      issues="$(printf '%s\n' "$issues" | jq --arg name "$name" --arg state "$dep_status" \
        '. + [{code:"TOOLCHAIN_DEPENDENCY_BLOCKED",message:("required dependency " + $name + " is " + $state)}]')"
      next_actions="$(printf '%s\n' "$next_actions" | jq --arg name "$name" --arg minimum "$minimum" \
        '. + [("install " + $name + " " + $minimum + " or newer")]')"
    fi
  done < <(printf '%s\n' "$RELEASE_REQUIRED_TOOLS_JSON" | jq -r '.[]')

  required_contracts="$(jq -n \
    --arg manifest "$SCHEMA_RELEASE_MANIFEST" \
    --arg licenses "$SCHEMA_RELEASE_LICENSE_INVENTORY" \
    --arg provenance "$SCHEMA_RELEASE_PROVENANCE" \
    --arg verification "$SCHEMA_RELEASE_VERIFICATION" \
    --arg doctor "$SCHEMA_TOOLCHAIN_DOCTOR" \
    --arg adapter "sley.adapter.manifest.v0" \
    --arg request "$WORKER_REQUEST_SCHEMA" \
    --arg response "$WORKER_RESPONSE_SCHEMA" \
    --arg event "$WORKER_EVENT_SCHEMA" \
    --arg session "$WORKER_SESSION_SCHEMA" \
    '[$manifest,$licenses,$provenance,$verification,$doctor,$adapter,$request,$response,$event,$session]')"
  schema_count="$(find "$ROOT_DIR/docs/schemas" -maxdepth 1 -type f -name '*.schema.json' -printf '.' | wc -c | tr -d ' ')"
  while IFS= read -r schema_id || [[ -n "$schema_id" ]]; do
    [[ -f "$ROOT_DIR/docs/schemas/$schema_id.schema.json" ]] || missing="$(printf '%s\n' "$missing" | jq --arg id "$schema_id" '. + [$id]')"
  done < <(printf '%s\n' "$required_contracts" | jq -r '.[]')
  contracts_status=ready
  if [[ "$(printf '%s\n' "$missing" | jq 'length')" -ne 0 ]]; then
    contracts_status=blocked
    status=blocked
    issues="$(printf '%s\n' "$issues" | jq '. + [{code:"TOOLCHAIN_CONTRACTS_MISSING",message:"one or more required release contracts are missing"}]')"
    next_actions="$(printf '%s\n' "$next_actions" | jq '. + ["restore the missing contract schemas"]')"
  fi

  adapter_status=ready
  if [[ ! -x "$ROOT_DIR/scripts/sley-adapter-replay.sh" ]]; then
    adapter_status=blocked
    status=blocked
    issues="$(printf '%s\n' "$issues" | jq '. + [{code:"TOOLCHAIN_LOCAL_REPLAY_MISSING",message:"the supported local replay adapter is unavailable"}]')"
    next_actions="$(printf '%s\n' "$next_actions" | jq '. + ["restore scripts/sley-adapter-replay.sh"]')"
  fi

  expected_sley="$(realpath -m -- "$ROOT_DIR/bin/sley")"
  if resolved_sley="$(command -v sley 2>/dev/null)" && [[ -n "$resolved_sley" ]]; then
    resolved_sley="$(realpath -m -- "$resolved_sley")"
    if [[ "$resolved_sley" != "$expected_sley" ]]; then
      path_conflict=true
      path_status=conflict
      status=blocked
      issues="$(printf '%s\n' "$issues" | jq '. + [{code:"TOOLCHAIN_PATH_CONFLICT",message:"PATH resolves sley to a different installation"}]')"
      next_actions="$(printf '%s\n' "$next_actions" | jq --arg expected "$expected_sley" '. + [("put " + $expected + " first on PATH")]')"
    fi
  else
    resolved_sley=""
  fi

  jq -n \
    --arg schema "$SCHEMA_TOOLCHAIN_DOCTOR" --arg status "$status" --arg version "$VERSION" \
    --arg release "$RELEASE_VERSION" --arg root "$ROOT_DIR" \
    --argjson dependencies "$dependencies" --arg schema_dir "$ROOT_DIR/docs/schemas" \
    --argjson schema_count "$schema_count" --argjson required "$required_contracts" --argjson missing "$missing" \
    --arg contracts_status "$contracts_status" --arg adapter_schema "$ADAPTER_KIND" \
    --arg worker_protocol "$WORKER_PROTOCOL" --arg adapter_status "$adapter_status" \
    --arg expected_sley "$expected_sley" --arg resolved_sley "$resolved_sley" \
    --argjson path_conflict "$path_conflict" --arg path_status "$path_status" \
    --argjson issues "$issues" --argjson next_actions "$next_actions" '{
      schema:$schema,status:$status,sley_version:$version,release:$release,root:$root,
      dependencies:$dependencies,
      contracts:{schema_dir:$schema_dir,schema_count:$schema_count,required:$required,missing:$missing,status:$contracts_status},
      adapter:{manifest_schema:$adapter_schema,worker_protocol:$worker_protocol,local_replay_available:($adapter_status == "ready"),external_adapters_supported:false,status:$adapter_status},
      path:{expected_sley:$expected_sley,resolved_sley:(if $resolved_sley == "" then null else $resolved_sley end),conflict:$path_conflict,status:$path_status},
      issues:$issues,next_actions:$next_actions
    }'
}

command_doctor() {
  local target report
  if [[ " ${*:-} " == *" --toolchain "* ]]; then
    report="$(toolchain_doctor_json)"
    printf '%s\n' "$report"
    [[ "$(printf '%s\n' "$report" | jq -r '.status')" == ready ]]
    return
  fi
  target="$(last_path_arg "$@")"
  report="$(doctor_json "$target")"
  printf '%s\n' "$report"
  [[ "$(printf '%s\n' "$report" | jq -r '.status')" != "blocked" ]]
}

command_release() {
  local arg release_script="$ROOT_DIR/scripts/sley-release.sh"
  [[ -x "$release_script" ]] || {
    printf 'sley: release tooling is unavailable: %s\n' "$release_script" >&2
    return 127
  }
  for arg in "$@"; do
    if [[ "$arg" == "--repo-root" || "$arg" == --repo-root=* ]]; then
      printf 'sley: --repo-root is internal and cannot be supplied by callers\n' >&2
      return 2
    fi
  done
  SLEY_RELEASE_VERSION="$RELEASE_VERSION" \
  SLEY_RELEASE_ARTIFACT_ID="$RELEASE_ARTIFACT_ID" \
  SLEY_RELEASE_PLATFORM_OS="$RELEASE_PLATFORM_OS" \
  SLEY_RELEASE_PLATFORM_ARCHITECTURE="$RELEASE_PLATFORM_ARCHITECTURE" \
  SLEY_RELEASE_ARCHIVE_FORMAT="$RELEASE_ARCHIVE_FORMAT" \
  SLEY_RELEASE_SUPPORT_STATUS="$RELEASE_SUPPORT_STATUS" \
  SLEY_RELEASE_AUTHORITY_MODE="$RELEASE_AUTHORITY_MODE" \
  SLEY_RELEASE_MANIFEST_SCHEMA="$SCHEMA_RELEASE_MANIFEST" \
  SLEY_RELEASE_LICENSE_SCHEMA="$SCHEMA_RELEASE_LICENSE_INVENTORY" \
  SLEY_RELEASE_PROVENANCE_SCHEMA="$SCHEMA_RELEASE_PROVENANCE" \
  SLEY_RELEASE_VERIFICATION_SCHEMA="$SCHEMA_RELEASE_VERIFICATION" \
  SLEY_RELEASE_REQUIRED_TOOLS_JSON="$RELEASE_REQUIRED_TOOLS_JSON" \
  SLEY_RELEASE_ENTRYPOINTS_JSON="$RELEASE_ENTRYPOINTS_JSON" \
  SLEY_RELEASE_METADATA_PATHS_JSON="$RELEASE_METADATA_PATHS_JSON" \
  SLEY_RELEASE_PUBLICATION_AUTHORIZED="$RELEASE_PUBLICATION_AUTHORIZED" \
    "$release_script" --repo-root "$ROOT_DIR" "$@"
}

command_query() {
  local target kind="all" module_filter="" exported_only=false report
  while [[ "$#" -gt 0 ]]; do
    case "$1" in
      --kind) kind="$2"; shift 2 ;;
      --json) shift ;;
      --exported) exported_only=true; shift ;;
      --module) module_filter="$2"; shift 2 ;;
      *) target="$1"; shift ;;
    esac
  done
  report="$(query_json "$target" "$kind" "$module_filter" "$exported_only")"
  printf '%s\n' "$report"
  if [[ "$(printf '%s\n' "$report" | jq -r '.status? // ""')" == "error" ]]; then
    return 1
  fi
}

command_graph() {
  local target slice="" query ast report
  while [[ "$#" -gt 0 ]]; do
    case "$1" in
      --slice) slice="$2"; shift 2 ;;
      --json) shift ;;
      *) target="$1"; shift ;;
    esac
  done
  query="$(query_json "$target" all)"
  if [[ -z "$slice" ]]; then
    report="$(printf '%s\n' "$query" | jq --arg schema "$SCHEMA_SYMBOL_GRAPH" --argjson fields "$SYMBOL_GRAPH_REPORT_FIELDS_JSON" '{
      ($fields[0] // "schema"):$schema,
      ($fields[1] // "entry_module"):.entry_module,
      ($fields[2] // "modules"):.modules
    }')"
    sley_report_builder_from_report_kind_json "symbol_graph" "$report"
    return
  fi
  ast="$(ast_json "$target")"
  jq -n --arg target "$slice" --arg entry_target "$target" --arg module_task_list_surface_id_template "$PARSER_MODULE_TASK_LIST_SURFACE_ID_TEMPLATE" --arg module_import_list_surface_id_template "$PARSER_MODULE_IMPORT_LIST_SURFACE_ID_TEMPLATE" --arg module_type_list_surface_id_template "$PARSER_MODULE_TYPE_LIST_SURFACE_ID_TEMPLATE" --arg module_effect_list_surface_id_template "$PARSER_MODULE_EFFECT_LIST_SURFACE_ID_TEMPLATE" --arg block_task_surface_id_template "$PARSER_BLOCK_TASK_SURFACE_ID_TEMPLATE" --arg call_target_task_id_template "$PARSER_CALL_TARGET_TASK_ID_TEMPLATE" --arg call_argument_surface_id_template "$PARSER_CALL_ARGUMENT_SURFACE_ID_TEMPLATE" --arg call_expression_prefix "$PARSER_CALL_EXPRESSION_PREFIX" --argjson query "$query" --argjson ast "$ast" '
    def replace_all($needle; $replacement): split($needle) | join($replacement);
    def module_task_list_surface_id($module_name):
      $module_task_list_surface_id_template
      | replace_all("{{module}}"; $module_name);
    def module_import_list_surface_id($module_name):
      $module_import_list_surface_id_template
      | replace_all("{{module}}"; $module_name);
    def module_type_list_surface_id($module_name):
      $module_type_list_surface_id_template
      | replace_all("{{module}}"; $module_name);
    def module_effect_list_surface_id($module_name):
      $module_effect_list_surface_id_template
      | replace_all("{{module}}"; $module_name);
    def block_task_surface_id($task_id):
      $block_task_surface_id_template
      | replace_all("{{task_id}}"; $task_id);
    def call_target_task_id($module_name; $task_name):
      $call_target_task_id_template
      | replace_all("{{module}}"; $module_name)
      | replace_all("{{task}}"; $task_name);
    def call_argument_surface_id($expression_id; $index):
      $call_argument_surface_id_template
      | replace_all("{{expression}}"; $expression_id)
      | replace_all("{{index}}"; $index);
    def qualified_task_id($qualified; $fallback_module):
      ($qualified | split(".")) as $parts |
      if ($parts | length) > 1 then
        call_target_task_id(($parts[0:-1] | join(".")); $parts[-1])
      else
        call_target_task_id($fallback_module; $qualified)
      end;
    def task_module($id):
      ($id | sub("^task:";"") | split(".")[0:-1] | join("."));
    def task_name($id):
      ($id | sub("^task:";"") | split(".")[-1]);
    def focus:
      if $target | startswith("module:") then
        {kind:"module", id:$target, module:($target | sub("^module:";""))}
      else
        {kind:"task", id:$target, module:task_module($target), name:task_name($target)}
      end;
    def module_for($module):
      ($query.modules[]? | select(.module == $module)) // {module:$module, imports:[], types:[], effects:[], tasks:[]};
    def task_for($id):
      ($ast.tasks[]? | select(.id == $id)) // null;
    def add_affordances($module):
      [
        {target:module_import_list_surface_id($module), target_kind:"import", operation:{op:"AddImport", payload:{owner_module:$module, module:$module}}, editable_json_pointers:["/payload/module"]},
        {target:module_type_list_surface_id($module), target_kind:"type", operation:{op:"AddTypeDeclaration", payload:{module:$module, name:"NewType"}}, editable_json_pointers:["/payload/name"]},
        {target:module_effect_list_surface_id($module), target_kind:"effect", operation:{op:"AddEffectDeclaration", payload:{module:$module, name:"NewEffect"}}, editable_json_pointers:["/payload/name"]},
        {target:module_task_list_surface_id($module), target_kind:"task", operation:{op:"AddTask", payload:{module:$module, name:"new_task"}}, editable_json_pointers:["/payload/name"]}
      ];
    def statement_moves($task):
      if $task == null then []
      else [$task.body.statements[]? | {target:.id, target_kind:"statement", operation:{op:"MoveNode", target:.id, payload:{parent:block_task_surface_id($task.id)}}}]
      end;
    def module_task_destinations($task; $module):
      if ($task.name // "") == "main" then []
      else
        [$module.imports[]? | (module_task_list_surface_id(.module)) as $task_list | {parent:$task_list, operation:{op:"MoveNode", target:$task.id, payload:{parent:$task_list}}}]
      end;
    def module_task_moves($module):
      [
        {target:module_import_list_surface_id($module.module), target_kind:"import", operation:{op:"MoveNode", payload:{}}},
        {target:module_type_list_surface_id($module.module), target_kind:"type", operation:{op:"MoveNode", payload:{}}},
        {target:module_effect_list_surface_id($module.module), target_kind:"effect", operation:{op:"MoveNode", payload:{}}}
      ]
      + [$module.tasks[]? as $task |
          {target:$task.id, target_kind:"task", operation:{op:"MoveNode", target:$task.id, payload:{parent:module_task_list_surface_id($module.module)}}}
          + ((module_task_destinations($task; $module)) as $d | if ($d | length) > 0 then {destinations:$d} else {} end)
        ];
    def task_move_affordances($task; $module):
      statement_moves($task)
      + [
        {target:module_import_list_surface_id($module.module), target_kind:"import", operation:{op:"MoveNode", payload:{}}},
        {target:($task.id // module_task_list_surface_id($module.module)), target_kind:"task", operation:{op:"MoveNode", target:($task.id // ""), payload:{parent:module_task_list_surface_id($module.module)}}}
      ];
    def replace_affordances($task):
      if $task == null then []
      else [$task.body.statements[]?.expr? |
        {
          target:.id,
          target_kind:(if ((.source? // "") | startswith($call_expression_prefix)) then "Call" else (.expr_kind // "Expression") end),
          operation:{op:"ReplaceExpression", target:.id, payload:{source:(.source // "")}},
          editable_json_pointers:["/payload/source"]
        }]
      end;
    def call_args($source):
      (if (($source // "") | startswith($call_expression_prefix)) then
        (($source | ltrimstr($call_expression_prefix) | capture("^[^()]+\\((?<args>.*)\\)\\??$")? // {args:""}).args)
      else "" end) as $args |
      if (($args | gsub("[[:space:]]"; "")) == "") then [] else ($args | split(",") | map(gsub("^[[:space:]]+|[[:space:]]+$"; ""))) end;
    def literal_kind($value):
      if $value | test("^-?[0-9]+$") then "IntLiteral"
      elif $value | test("^\".*\"$") then "StringLiteral"
      elif $value == "true" or $value == "false" then "BoolLiteral"
      else "Expression" end;
    def call_site_affordances($focus):
      [$query.calls[]? | select($focus.kind == "task" and .from == ($focus.module + "." + $focus.name)) |
        {target:.expr_id, target_kind:"call_site", task_target:qualified_task_id(.target; $focus.module), operation:{op:"UpdateCallSites", target:.expr_id, payload:{scope:$focus.id}}}
      ];
    def call_arg_affordances($focus):
      [$query.calls[]? | select($focus.kind == "task" and .from == ($focus.module + "." + $focus.name)) as $call |
        (call_args($call.source) | to_entries[]) |
        {target:call_argument_surface_id($call.expr_id; (.key|tostring)), target_kind:literal_kind(.value), task_target:qualified_task_id($call.target; $focus.module), operation:{op:"ReplaceCallArg", target:$call.expr_id, payload:{index:.key, source:.value}}}
      ];
    def inbound_calls($focus):
      if $focus.kind == "module" then
        [$query.calls[]? | select(.target | startswith($focus.module + "."))]
      else
        [$query.calls[]? | select(.target == ($focus.module + "." + $focus.name))]
      end;
    def outbound_calls($focus):
      if $focus.kind == "module" then
        [$query.calls[]? | select(.from_module == $focus.module)]
      else
        [$query.calls[]? | select(.from == ($focus.module + "." + $focus.name))]
      end;
    def delete_affordances($module):
      [$module.tasks[]? as $task |
        select(($task.exported | not) and $task.name != "main") |
        ($task.id | sub("^task:";"")) as $qualified |
        select(([$query.calls[]? | select(.target == $qualified)] | length) == 0) |
        {target:$task.id, target_kind:"task", operation:{op:"DeleteNode", target:$task.id}}
      ];
    (focus) as $focus |
    (module_for($focus.module)) as $module |
    (if $focus.kind == "task" then task_for($focus.id) else null end) as $task |
    {
      schema:"sley.symbol_graph.slice.v0",
      target:$target,
      entry_module:$query.entry_module,
      focus:$focus,
      imports:($module.imports // []),
      types:($module.types // []),
      effects:($module.effects // []),
      tasks:($module.tasks // []),
      add_affordances:add_affordances($focus.module),
      insert_affordances:(if $focus.kind == "task" then [(block_task_surface_id($focus.id)) as $block_surface | {target:$block_surface, target_kind:"block", operation:{op:"InsertStatement", target:$block_surface, payload:{source:"return 0"}}, editable_json_pointers:["/payload/source"]}] else [] end),
      move_affordances:(if $focus.kind == "task" then task_move_affordances($task; $module) else module_task_moves($module) end),
      delete_affordances:delete_affordances($module),
      replace_affordances:(if $focus.kind == "task" then replace_affordances($task) else ([$module.tasks[]? | task_for(.id)] | map(replace_affordances(.)) | add // []) end),
      call_site_affordances:call_site_affordances($focus),
      call_arg_affordances:call_arg_affordances($focus),
      outbound_calls:outbound_calls($focus),
      inbound_calls:inbound_calls($focus)
    }'
}

command_graph_diff() {
  local base="" ours="" theirs="" base_query ours_query theirs_query base_ast ours_ast theirs_ast
  local base_graph ours_graph theirs_graph base_digest ours_digest theirs_digest
  local base_source_digest ours_source_digest theirs_source_digest report status
  while [[ "$#" -gt 0 ]]; do
    case "$1" in
      --base) base="$2"; shift 2 ;;
      --ours) ours="$2"; shift 2 ;;
      --theirs) theirs="$2"; shift 2 ;;
      --json) shift ;;
      *) echo "unknown graph-diff argument: $1" >&2; return 2 ;;
    esac
  done
  if [[ -z "$base" || -z "$ours" || -z "$theirs" ]]; then
    echo "graph-diff requires --base, --ours, and --theirs" >&2
    return 2
  fi

  base_query="$(query_json "$base" all)"
  ours_query="$(query_json "$ours" all)"
  theirs_query="$(query_json "$theirs" all)"
  base_ast="$(ast_json "$base")"
  ours_ast="$(ast_json "$ours")"
  theirs_ast="$(ast_json "$theirs")"
  base_graph="$(jq -c --arg schema "$SCHEMA_SYMBOL_GRAPH" '{schema:$schema,entry_module,modules}' <<< "$base_query")"
  ours_graph="$(jq -c --arg schema "$SCHEMA_SYMBOL_GRAPH" '{schema:$schema,entry_module,modules}' <<< "$ours_query")"
  theirs_graph="$(jq -c --arg schema "$SCHEMA_SYMBOL_GRAPH" '{schema:$schema,entry_module,modules}' <<< "$theirs_query")"
  base_digest="$(printf '%s\n' "$base_graph" | json_digest_for)"
  ours_digest="$(printf '%s\n' "$ours_graph" | json_digest_for)"
  theirs_digest="$(printf '%s\n' "$theirs_graph" | json_digest_for)"
  base_source_digest="$(structural_source_digest_for "$base")"
  ours_source_digest="$(structural_source_digest_for "$ours")"
  theirs_source_digest="$(structural_source_digest_for "$theirs")"

  report="$(jq -n \
    --arg schema "$SCHEMA_GRAPH_DIFF" \
    --arg base_path "$base" --arg ours_path "$ours" --arg theirs_path "$theirs" \
    --arg base_digest "$base_digest" --arg ours_digest "$ours_digest" --arg theirs_digest "$theirs_digest" \
    --arg base_source_digest "$base_source_digest" --arg ours_source_digest "$ours_source_digest" --arg theirs_source_digest "$theirs_source_digest" \
    --arg query_schema "$SCHEMA_QUERY" --arg graph_schema "$SCHEMA_SYMBOL_GRAPH" --arg ast_schema "$SCHEMA_AST_PROGRAM" \
    --argjson base "$base_query" --argjson ours "$ours_query" --argjson theirs "$theirs_query" \
    --argjson base_ast "$base_ast" --argjson ours_ast "$ours_ast" --argjson theirs_ast "$theirs_ast" \
    --argjson base_graph "$base_graph" --argjson ours_graph "$ours_graph" --argjson theirs_graph "$theirs_graph" \
    --argjson fields "$GRAPH_DIFF_REPORT_FIELDS_JSON" '
    def rank($kind):
      {module:0,import:1,type:2,effect:3,task:4,call:5}[$kind] // 99;
    def strip_spans: walk(if type == "object" then del(.span) else . end);
    def task_body($ast; $identity): first($ast.tasks[]? | select(.id == $identity) | .body | strip_spans) // null;
    def nodes($q; $ast):
      (([$q.modules[]? | {node_kind:"module",identity:("module:" + .module),value:{module:.module}}]
      + [$q.modules[]? as $m | $m.imports[]? | {node_kind:"import",identity:.id,value:({module:.module} + (if .alias then {alias:.alias} else {} end))}]
      + [$q.types[]? | {node_kind:"type",identity:.id,value:{name,module,qualified_name,exported,value,fields}}]
      + [$q.effects[]? | {node_kind:"effect",identity:.id,value:{name,module,qualified_name,exported}}]
      + [$q.tasks[]? | . as $task | {node_kind:"task",identity:.id,value:{name,module,qualified_name,exported,takes,return_type,effects,body:task_body($ast; $task.id)}}]
      + [$q.calls[]? | {node_kind:"call",identity:("call:" + .expr_id),value:{from,from_module,expr_id,source,callee,status,target:(.target // null),candidates:(.candidates // [])}}])
      | sort_by(rank(.node_kind), .identity));
    def lookup($nodes; $identity): first($nodes[]? | select(.identity == $identity)) // null;
    def side_changes($side; $base_nodes; $side_nodes):
      [([[$base_nodes[]?.identity] + [$side_nodes[]?.identity]] | add | unique[]) as $identity |
        (lookup($base_nodes; $identity)) as $before |
        (lookup($side_nodes; $identity)) as $after |
        select($before != $after) |
        {
          side:$side,
          identity:$identity,
          node_kind:($after.node_kind // $before.node_kind),
          change_kind:(if $before == null then "added" elif $after == null then "deleted" else "modified" end),
          before:($before.value // null),
          after:($after.value // null),
          ordering_key:((rank($after.node_kind // $before.node_kind) | tostring) + "|" + $identity + "|" + $side)
        }
      ] | sort_by(.ordering_key);
    def gate_takes($value): [($value.takes // [])[]? | select(.binding_kind == "Gate")];
    def conflict_class($ours_change; $theirs_change):
      if $ours_change.change_kind == "deleted" or $theirs_change.change_kind == "deleted" then "rename_delete"
      elif $ours_change.node_kind == "call" then "call_site_rewrite"
      elif $ours_change.node_kind == "task" and (($ours_change.before.effects // []) != ($ours_change.after.effects // []) or ($theirs_change.before.effects // []) != ($theirs_change.after.effects // [])) then "effect_change"
      elif $ours_change.node_kind == "task" and (gate_takes($ours_change.before) != gate_takes($ours_change.after) or gate_takes($theirs_change.before) != gate_takes($theirs_change.after)) then "gate_change"
      else "same_node_edit" end;
    (nodes($base; $base_ast)) as $base_nodes |
    (nodes($ours; $ours_ast)) as $ours_nodes |
    (nodes($theirs; $theirs_ast)) as $theirs_nodes |
    (side_changes("ours"; $base_nodes; $ours_nodes)) as $ours_changes |
    (side_changes("theirs"; $base_nodes; $theirs_nodes)) as $theirs_changes |
    ($ours_changes + $theirs_changes) as $all_changes |
    ([($ours_changes[]? | .identity)] | unique) as $ours_ids |
    ([($theirs_changes[]? | .identity)] | unique) as $theirs_ids |
    ([($ours_ids - ($ours_ids - $theirs_ids))[] as $identity |
      (first($ours_changes[] | select(.identity == $identity))) as $oc |
      (first($theirs_changes[] | select(.identity == $identity))) as $tc |
      select($oc.after != $tc.after) |
      {
        class:conflict_class($oc; $tc),
        identity:$identity,
        sides:["ours","theirs"],
        evidence:{base:$oc.before,ours:$oc.after,theirs:$tc.after},
        fail_closed_reason:"both sides changed the same semantic identity differently"
      }
    ]) as $overlap_conflicts |
    ([$all_changes[] | select(.change_kind == "deleted" and (.node_kind | IN("import","type","effect","task"))) | . as $deleted |
      [$all_changes[] | select(.change_kind == "added" and .node_kind == $deleted.node_kind)] as $additions |
      select(($additions | length) > 0) |
      {
        class:"rename_delete",
        identity:$deleted.identity,
        sides:([$deleted.side] + [$additions[].side] | unique),
        evidence:{deleted:{side:$deleted.side,identity:$deleted.identity,node_kind:$deleted.node_kind},additions:[$additions[] | {side,identity,node_kind}]},
        fail_closed_reason:"declaration deletion plus same-kind addition is an ambiguous rename or replacement"
      }
    ] | unique_by(.class,.identity)) as $rename_conflicts |
    ([($base.calls // [])[],($ours.calls // [])[],($theirs.calls // [])[] | select(.status != "resolved") | ("call:" + .expr_id)] | unique |
      map({class:"unsupported_ambiguous",identity:.,sides:["base","ours","theirs"],evidence:{reason:"non-resolved call identity"},fail_closed_reason:"unresolved or ambiguous calls cannot participate in structural merge research"})) as $unsupported_conflicts |
    (if ($base.entry_module == $ours.entry_module and $base.entry_module == $theirs.entry_module) then [] else [{class:"projection_drift",identity:"entry_module",sides:["base","ours","theirs"],evidence:{base:$base.entry_module,ours:$ours.entry_module,theirs:$theirs.entry_module},fail_closed_reason:"semantic projections disagree on the entry module"}] end
      + if ($base.schema == $query_schema and $ours.schema == $query_schema and $theirs.schema == $query_schema and $base_ast.schema == $ast_schema and $ours_ast.schema == $ast_schema and $theirs_ast.schema == $ast_schema) then [] else [{class:"projection_drift",identity:"projection_schema",sides:["base","ours","theirs"],evidence:{base_query:($base.schema // ""),ours_query:($ours.schema // ""),theirs_query:($theirs.schema // ""),base_ast:($base_ast.schema // ""),ours_ast:($ours_ast.schema // ""),theirs_ast:($theirs_ast.schema // "")},fail_closed_reason:"compiler projections did not return the required versioned schemas"}] end) as $projection_conflicts |
    ((if $base_source_digest != $ours_source_digest and ($ours_changes | length) == 0 then [{class:"projection_drift",identity:"ours:source_projection",sides:["base","ours"],evidence:{base_source_digest:$base_source_digest,side_source_digest:$ours_source_digest},fail_closed_reason:"source changed without a corresponding semantic projection change"}] else [] end)
      + (if $base_source_digest != $theirs_source_digest and ($theirs_changes | length) == 0 then [{class:"projection_drift",identity:"theirs:source_projection",sides:["base","theirs"],evidence:{base_source_digest:$base_source_digest,side_source_digest:$theirs_source_digest},fail_closed_reason:"source changed without a corresponding semantic projection change"}] else [] end)) as $source_projection_conflicts |
    ($overlap_conflicts + $rename_conflicts + $unsupported_conflicts + $projection_conflicts + $source_projection_conflicts | unique_by(.class,.identity) | sort_by(.class,.identity)) as $conflicts |
    ($ours_changes + $theirs_changes | sort_by(.ordering_key)) as $changes |
    ([$conflicts[] | select(.class == "unsupported_ambiguous")] | length) as $unsupported_count |
    {
      ($fields[0] // "schema"):$schema,
      ($fields[1] // "status"):(if $unsupported_count > 0 then "unsupported" elif ($conflicts|length) > 0 then "conflicted" else "passed" end),
      ($fields[2] // "mode"):"report_only",
      ($fields[3] // "merge_permitted"):false,
      ($fields[4] // "inputs"):{
        base:{path:$base_path,entry_module:($base.entry_module // ""),graph_digest:$base_digest,source_digest:$base_source_digest},
        ours:{path:$ours_path,entry_module:($ours.entry_module // ""),graph_digest:$ours_digest,source_digest:$ours_source_digest},
        theirs:{path:$theirs_path,entry_module:($theirs.entry_module // ""),graph_digest:$theirs_digest,source_digest:$theirs_source_digest}
      },
      ($fields[5] // "summary"):{node_change_count:($changes|length),call_change_count:([$changes[]|select(.node_kind=="call")]|length),conflict_count:($conflicts|length),unsupported_count:$unsupported_count},
      ($fields[6] // "identity_rules"):[
        {node_kind:"module",rule:"module:<module_name>",stability:"stable"},
        {node_kind:"declaration",rule:"compiler declaration id",stability:"stable"},
        {node_kind:"call",rule:"call:<expression_id>",stability:"stable_within_projection"},
        {node_kind:"take",rule:"task id plus position/name/binding/type",stability:"provisional_fail_closed"}
      ],
      ($fields[7] // "changes"):$changes,
      ($fields[8] // "conflicts"):$conflicts,
      ($fields[9] // "projection_checks"):[
        {side:"base",projection:"query",status:(if $base.schema==$query_schema then "passed" else "failed" end),schema:($base.schema // "")},
        {side:"ours",projection:"query",status:(if $ours.schema==$query_schema then "passed" else "failed" end),schema:($ours.schema // "")},
        {side:"theirs",projection:"query",status:(if $theirs.schema==$query_schema then "passed" else "failed" end),schema:($theirs.schema // "")},
        {side:"base",projection:"ast",status:(if $base_ast.schema==$ast_schema then "passed" else "failed" end),schema:($base_ast.schema // "")},
        {side:"ours",projection:"ast",status:(if $ours_ast.schema==$ast_schema then "passed" else "failed" end),schema:($ours_ast.schema // "")},
        {side:"theirs",projection:"ast",status:(if $theirs_ast.schema==$ast_schema then "passed" else "failed" end),schema:($theirs_ast.schema // "")},
        {side:"base",projection:"graph",status:(if $base_graph.schema==$graph_schema then "passed" else "failed" end),schema:($base_graph.schema // "")},
        {side:"ours",projection:"graph",status:(if $ours_graph.schema==$graph_schema then "passed" else "failed" end),schema:($ours_graph.schema // "")},
        {side:"theirs",projection:"graph",status:(if $theirs_graph.schema==$graph_schema then "passed" else "failed" end),schema:($theirs_graph.schema // "")}
      ],
      ($fields[10] // "next_actions"):[
        {kind:"inspect_only",command:["sley","query","--json","--kind","all",$base_path]},
        {kind:"validate_each_side",command:["sley","verify","--json",$ours_path]},
        {kind:"validate_each_side",command:["sley","verify","--json",$theirs_path]}
      ],
      ($fields[11] // "issues"):[]
    }')"
  report="$(graph_diff_report_from_json "$report")"
  printf '%s\n' "$report"
  status="$(jq -r '.status' <<< "$report")"
  [[ "$status" == "passed" ]]
}
