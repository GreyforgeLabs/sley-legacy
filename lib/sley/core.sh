# shellcheck shell=bash
# Shared syntax, report, validation, and compiler-core helpers.

usage() {
  cat <<'EOF'
Sley Loom self-hosting stage-1 bootstrap CLI

Usage:
  sley [--help|--version]
  sley <command> [--json] [options] <file-or-project>

Commands:
  parse, ast, check, explain, doctor, graph, graph-diff, query, lint, plan, verify, run
  adapter, worker, test, validate, release, reference-replay, operational-workflow, operational-evidence, change, machine, deploy, trace, seal, zjx, graft, fix, format, new, self-hosting-status
  claim-verify, arena

Release candidate:
  sley release build --output-dir dist --json
  sley release verify --output-dir dist --json

Reference replay:
  sley reference-replay siglum-numerology --siglum-root PATH --execute --json

Operational agent workflow:
  sley operational-workflow plan --controller PATH --json
  sley operational-workflow compare --controller PATH --raw-aggregate PATH --raw-evidence PATH --structural-aggregate PATH --structural-evidence PATH --approval-record PATH --execution-provenance PATH --json

Operational evidence decision:
  sley operational-evidence assemble --reference-replay PATH --agent-comparison PATH --controller PATH --raw-aggregate PATH --raw-evidence PATH --structural-aggregate PATH --structural-evidence PATH --approval-record PATH --execution-provenance PATH --json

Companion tools:
  git-sley-guard, sley-ci, sley-conformance, sley-contract, sley-corpus, sley-docgen, sley-lsp,
  sley-workbench, sley-agent-bench, sley-migrate, sley-sandbox-runner,
  sley-shadow, sley-zjx, sley-mcp-bridge
EOF
}

SLEY_MAX_SOURCE_BYTES="${SLEY_MAX_SOURCE_BYTES:-5242880}"
SLEY_MAX_JSON_BYTES="${SLEY_MAX_JSON_BYTES:-5242880}"
sley_require_bounded_positive_integer SLEY_MAX_SOURCE_BYTES "$SLEY_MAX_SOURCE_BYTES" 67108864
sley_require_bounded_positive_integer SLEY_MAX_JSON_BYTES "$SLEY_MAX_JSON_BYTES" 67108864

sley_reject_ambiguous_path() {
  local path="$1"
  case "$path" in
    *$'\n'*|*$'\r'*)
      echo "unsupported path containing a newline: $path" >&2
      exit 2
      ;;
  esac
}

sley_enforce_file_budget() {
  local path="$1" max_bytes="$2" size
  [[ -f "$path" ]] || return 0
  size="$(wc -c < "$path" | tr -d ' ')"
  if [[ "$size" =~ ^[0-9]+$ && "$size" -gt "$max_bytes" ]]; then
    echo "file exceeds Sley safety budget ($size > $max_bytes bytes): $path" >&2
    exit 2
  fi
}

sley_realpath_maybe() {
  local path="$1"
  realpath -m -- "$path"
}

sley_path_is_under() {
  local child="$1" parent="$2" child_abs parent_abs
  child_abs="$(sley_realpath_maybe "$child")"
  parent_abs="$(sley_realpath_maybe "$parent")"
  [[ "$child_abs" == "$parent_abs" || "$child_abs" == "$parent_abs/"* ]]
}

sley_validate_identifier() {
  local value="$1" label="${2:-identifier}"
  if [[ ! "$value" =~ ^[A-Za-z_][A-Za-z0-9_]*$ ]]; then
    echo "invalid Sley $label: $value" >&2
    exit 2
  fi
}

sley_validate_module_name() {
  local module="$1"
  if [[ ! "$module" =~ ^[A-Za-z_][A-Za-z0-9_]*(\.[A-Za-z_][A-Za-z0-9_]*)*$ ]]; then
    echo "invalid Sley module name: $module" >&2
    exit 2
  fi
}

sley_validate_project_name() {
  local name="$1"
  if [[ ! "$name" =~ ^[A-Za-z0-9_.-]+$ ]]; then
    echo "invalid Sley project name: $name" >&2
    exit 2
  fi
}

sley_module_path_for() {
  local module="$1"
  sley_validate_module_name "$module"
  printf '%s\n' "${module//.//}"
}

sley_write_new_file() {
  local path="$1"
  sley_reject_ambiguous_path "$path"
  if [[ -e "$path" || -L "$path" ]]; then
    echo "refusing to overwrite existing scaffold file: $path" >&2
    return 1
  fi
  mkdir -p "$(dirname -- "$path")"
  cat > "$path"
}

sley_append_trace_receipt() {
  local trace_path="$1" report="$2"
  [[ -n "$trace_path" ]] || return 0
  sley_reject_ambiguous_path "$trace_path"
  mkdir -p "$(dirname -- "$trace_path")"
  printf '%s\n' "$report" | jq -c '{schema:"sley.trace.receipt.v0", provenance:.provenance}' >> "$trace_path"
}

sley_assert_source_still_checks() {
  local source="$1" check
  check="$(check_json "$source" || true)"
  if [[ "$(printf '%s\n' "$check" | jq -r '.status? // ""')" == "error" ]]; then
    printf '%s\n' "$check" >&2
    return 1
  fi
}

sley_report_builder_json() {
  local builder_json="$1" values_json="$2"
  jq -n \
    --argjson type_plan "$REPORT_BUILDER_VALUE_TYPE_PLAN_JSON" \
    --slurpfile values_file <(printf '%s\n' "$values_json") \
    --slurpfile builder_file <(printf '%s\n' "$builder_json") \
    '
    ($values_file[0]) as $values |
    ($builder_file[0]) as $builder |
    def builder_slot($spec):
      ($spec | split("|")) as $parts |
      if ($parts | length) != 3 then
        error("invalid report builder slot: " + $spec)
      else
        ($parts[2]) as $raw_type |
        (($raw_type | startswith("Optional<")) and ($raw_type | endswith(">"))) as $optional |
        {
          field:$parts[0],
          source:$parts[1],
          optional:$optional,
          value_type:(if $optional then ($raw_type | sub("^Optional<"; "") | sub(">$"; "")) else $raw_type end)
        }
      end;
    def type_descriptor($value_type):
      ([$type_plan[]? | select(.value_type == $value_type)][0] // null);
    def source_lookup($source):
      ($source | split(".")) as $parts |
      if ($parts | length) != 2 then
        error("invalid report builder source: " + $source)
      else
        ($values[$parts[0]] // {}) as $scope |
        if ($scope | type) != "object" then
          {found:false, value:null}
        else
          {found:($scope | has($parts[1])), value:$scope[$parts[1]]}
        end
      end;
    def type_ok($value_type; $value):
      (type_descriptor($value_type)) as $plan |
      if $plan == null then false
      elif $plan.json_type == "any" then true
      elif ($value | type) != $plan.json_type then false
      elif (($plan.item_type // "") == "" or $plan.item_type == "any") then true
      elif ($value | type) == "array" then all($value[]; type == $plan.item_type)
      else false end;
    def optional_skip($slot; $found; $value):
      $slot.optional and
      (($found | not) or $value == null or
       ($slot.value_type == "Text" and ($value | type) == "string" and $value == ""));
    reduce ($builder[] | builder_slot(.)) as $slot
      ({};
       (source_lookup($slot.source)) as $source |
       if optional_skip($slot; $source.found; $source.value) then
         .
       elif (($source.found | not) or $source.value == null) then
         error("unknown report builder source: " + $slot.source)
       elif type_ok($slot.value_type; $source.value) then
         . + {($slot.field):$source.value}
       else
         error("report builder type mismatch for field: " + $slot.field)
       end)
    '
}

sley_report_builder_descriptor_json() {
  local namespace="$1"
  printf '%s\n' "$REPORT_BUILDER_REGISTRY_JSON" | jq -cer \
    --arg namespace "$namespace" \
    '.[] | select(.namespace == $namespace)'
}

sley_report_builder_source_json() {
  local namespace="$1"
  local descriptor builder_task
  descriptor="$(sley_report_builder_descriptor_json "$namespace")" || return 1
  builder_task="$(printf '%s\n' "$descriptor" | jq -er '.builder_task')" || return 1
  sley_source_list_task_json loom.reports "$builder_task"
}

sley_report_builder_for_namespace_json() {
  local namespace="$1" values_json="$2"
  local builder_json
  builder_json="$(sley_report_builder_source_json "$namespace")" || return 1
  sley_report_builder_json "$builder_json" "$values_json"
}

sley_report_builder_from_report_kind_json() {
  local namespace="$1" report_json="$2"
  local descriptor schema_name schema_value values_json
  descriptor="$(sley_report_builder_descriptor_json "$namespace")" || return 1
  schema_name="$(printf '%s\n' "$descriptor" | jq -er '.schema_task')" || return 1
  schema_value="$(sley_source_task loom.reports "$schema_name")" || return 1
  values_json="$(jq -n \
    --arg namespace "$namespace" \
    --arg schema_name "$schema_name" \
    --arg schema "$schema_value" \
    --slurpfile report_file <(printf '%s\n' "$report_json") '
    {reports:{($schema_name):$schema}} + {($namespace):$report_file[0]}
  ')"
  sley_report_builder_for_namespace_json "$namespace" "$values_json"
}

ast_program_report_from_json() {
  local report_json="$1"
  sley_report_builder_from_report_kind_json "ast_program" "$report_json"
}

ast_node_report_from_json() {
  local report_json="$1"
  sley_report_builder_from_report_kind_json "ast_node" "$report_json"
}

diagnostics_report_from_json() {
  local report_json="$1"
  sley_report_builder_from_report_kind_json "diagnostics" "$report_json"
}

explain_report_from_json() {
  local report_json="$1"
  sley_report_builder_from_report_kind_json "explain" "$report_json"
}

graft_outcome_report_from_json() {
  local report_json="$1"
  sley_report_builder_from_report_kind_json "graft_outcome" "$report_json"
}

ci_report_from_json() {
  local report_json="$1"
  sley_report_builder_from_report_kind_json "ci" "$report_json"
}

contract_report_from_json() {
  local report_json="$1" schema
  schema="$(printf '%s\n' "$report_json" | jq -r '.schema? // empty' 2>/dev/null || true)"
  case "$schema" in
    "$SCHEMA_CONTRACT_INVENTORY")
      sley_report_builder_from_report_kind_json "contract_inventory" "$report_json"
      ;;
    "$SCHEMA_CONTRACT_VALIDATE")
      sley_report_builder_from_report_kind_json "contract_validate" "$report_json"
      ;;
    "$SCHEMA_CONTRACT_FIXTURE_CHECK")
      sley_report_builder_from_report_kind_json "contract_fixture_check" "$report_json"
      ;;
    "$SCHEMA_DEPLOY_ARTIFACT_CHECK")
      sley_report_builder_from_report_kind_json "deploy_artifact_check" "$report_json"
      ;;
    *)
      printf '%s\n' "$report_json"
      ;;
  esac
}

workbench_report_from_json() {
  local report_json="$1"
  sley_report_builder_from_report_kind_json "workbench" "$report_json"
}

zjx_tool_report_from_json() {
  local report_json="$1"
  sley_report_builder_from_report_kind_json "zjx_tool" "$report_json"
}

graph_diff_report_from_json() {
  local report_json="$1"
  sley_report_builder_from_report_kind_json "graph_diff" "$report_json"
}

transaction_inspect_report_from_json() {
  local report_json="$1"
  sley_report_builder_from_report_kind_json "transaction_inspect" "$report_json"
}

change_plan_report_from_json() {
  local report_json="$1"
  sley_report_builder_from_report_kind_json "change_plan" "$report_json"
}

change_preview_report_from_json() {
  local report_json="$1"
  sley_report_builder_from_report_kind_json "change_preview" "$report_json"
}

change_approval_request_report_from_json() {
  local report_json="$1"
  sley_report_builder_from_report_kind_json "change_approval_request" "$report_json"
}

change_grant_report_from_json() {
  local report_json="$1"
  sley_report_builder_from_report_kind_json "change_grant" "$report_json"
}

change_apply_authorization_report_from_json() {
  local report_json="$1"
  sley_report_builder_from_report_kind_json "change_apply_authorization" "$report_json"
}

change_revocation_record_report_from_json() {
  local report_json="$1"
  sley_report_builder_from_report_kind_json "change_revocation_record" "$report_json"
}

change_recovery_report_from_json() {
  local report_json="$1"
  sley_report_builder_from_report_kind_json "change_recovery" "$report_json"
}

change_apply_report_from_json() {
  local report_json="$1"
  sley_report_builder_from_report_kind_json "change_apply" "$report_json"
}

change_rollback_report_from_json() {
  local report_json="$1"
  sley_report_builder_from_report_kind_json "change_rollback" "$report_json"
}

change_review_report_from_json() {
  local report_json="$1"
  sley_report_builder_from_report_kind_json "change_review" "$report_json"
}

change_transaction_seal_report_from_json() {
  local report_json="$1"
  sley_report_builder_from_report_kind_json "change_transaction_seal" "$report_json"
}

test_report_from_json() {
  local report_json="$1"
  sley_report_builder_from_report_kind_json "test" "$report_json"
}

review_packet_from_json() {
  local report_json="$1"
  sley_report_builder_from_report_kind_json "review_packet" "$report_json"
}

validation_report_from_json() {
  local report_json="$1"
  sley_report_builder_from_report_kind_json "validation" "$report_json"
}

sley_report_error_json() {
  local id="$1" message="$2" node="${3:-runtime}" hint_kind="${4:-}" hint_target="${5:-}" hint_replacement="${6:-}"
  local diagnostics values_json
  diagnostics="$(jq -n \
    --arg id "$id" \
    --arg message "$message" \
    --arg node "$node" \
    --arg hint_kind "$hint_kind" \
    --arg hint_target "$hint_target" \
    --arg hint_replacement "$hint_replacement" '
    [
      {
        id:$id,
        severity:"error",
        message:$message,
        node:$node
      }
      + (if $hint_kind == "" then {} else {
        repair_hints:[{kind:$hint_kind,target:$hint_target,replacement:$hint_replacement}]
      } end)
    ]')"
  values_json="$(jq -n \
    --arg schema "$SCHEMA_DIAGNOSTICS" \
    --arg status "error" \
    --argjson diagnostics "$diagnostics" '
    {
      reports:{diagnostics_schema:$schema},
      diagnostics:{status:$status, diagnostics:$diagnostics}
    }')"
  sley_report_builder_for_namespace_json "diagnostics" "$values_json"
}

collect_files() {
  local target="$1"
  if [[ -z "$target" ]]; then
    echo "missing file-or-project target" >&2
    exit 2
  fi
  sley_reject_ambiguous_path "$target"
  if [[ -f "$target" ]]; then
    sley_enforce_file_budget "$target" "$SLEY_MAX_SOURCE_BYTES"
    printf '%s\n' "$target"
    return
  fi
  if [[ -d "$target" ]]; then
    local file
    if [[ -f "$target/sley.toml" && -d "$target/src" ]]; then
      while IFS= read -r -d '' file; do
        sley_reject_ambiguous_path "$file"
        sley_enforce_file_budget "$file" "$SLEY_MAX_SOURCE_BYTES"
        printf '%s\n' "$file"
      done < <(find "$target/src" -type f -name '*.sley' -print0 | sort -z)
    else
      while IFS= read -r -d '' file; do
        sley_reject_ambiguous_path "$file"
        sley_enforce_file_budget "$file" "$SLEY_MAX_SOURCE_BYTES"
        printf '%s\n' "$file"
      done < <(find "$target" -type f -name '*.sley' -print0 | sort -z)
    fi
    return
  fi
  echo "target not found: $target" >&2
  exit 2
}

source_rows_json_for_target() {
  local target="$1" file module task_lines_json
  {
    while IFS= read -r file; do
      module="$(awk '/^[[:space:]]*module[[:space:]]+/ {print $2; found=1; exit} END {if (!found) print "main"}' "$file")"
      task_lines_json="$(awk -v task_pattern="$CHECKER_TASK_DECLARATION_PATTERN" '
        function brace_delta(line,    cursor, char, escaped, in_string, delta) {
          escaped = 0
          in_string = 0
          delta = 0
          for (cursor = 1; cursor <= length(line); cursor++) {
            char = substr(line, cursor, 1)
            if (escaped) {
              escaped = 0
              continue
            }
            if (in_string && char == "\\") {
              escaped = 1
              continue
            }
            if (char == "\"") {
              in_string = !in_string
              continue
            }
            if (!in_string && char == "#") break
            if (!in_string && char == "{") delta++
            if (!in_string && char == "}") delta--
          }
          return delta
        }
        {
          if (!in_task && $0 ~ task_pattern) {
            in_task = 1
            task_depth = 0
            task_opened = 0
          }
          if (in_task) {
            print NR
            line_delta = brace_delta($0)
            if (line_delta > 0) task_opened = 1
            task_depth += line_delta
            if (task_opened && task_depth <= 0) {
              in_task = 0
              task_depth = 0
              task_opened = 0
            }
          }
        }
      ' "$file" | jq -Rsc 'split("\n") | map(select(length > 0) | tonumber)')"
      jq -Rsc \
        --arg module "$module" \
        --arg file "$file" \
        --argjson task_lines "$task_lines_json" '
        split("\n")
        | to_entries
        | map(. as $entry | {
            module:$module,
            file:$file,
            line:($entry.key + 1),
            text:$entry.value,
            in_task:(($task_lines | index($entry.key + 1)) != null)
          })
      ' "$file"
    done < <(collect_files "$target")
  } | jq -cs 'add // []'
}

absolute_path() {
  local path="$1" dir base
  if [[ "$path" != /* ]]; then
    path="$PWD/$path"
  fi
  dir="$(cd "$(dirname "$path")" && pwd)" || return 1
  base="$(basename "$path")"
  printf '%s/%s\n' "$dir" "$base"
}

entry_module_for() {
  local target="$1" file
  if [[ -d "$target" && -f "$target/sley.toml" ]]; then
    file="$(sley_source_entry_file_for_target "$target")"
  else
    file="$(sley_source_entry_file_for_target "$target")"
  fi
  awk '/^[[:space:]]*module[[:space:]]+/ {print $2; found=1; exit} END {if (!found) print "main"}' "$file"
}

ast_json() {
  local target="$1" raw_ast
  mapfile -t files < <(collect_files "$target")
  if [[ "${#files[@]}" -eq 0 ]]; then
    echo "no .sley files found under: $target" >&2
    exit 2
  fi
  raw_ast="$(awk -v schema="$SCHEMA_AST_PROGRAM" \
    -v int_pattern="$PARSER_INT_PATTERN" \
    -v identifier_pattern="$PARSER_IDENTIFIER_PATTERN" \
    -v multiline_record_literal_open_pattern="$PARSER_MULTILINE_RECORD_LITERAL_OPEN_PATTERN" \
    -v int_kind="$PARSER_FEATURE_INT_KIND" \
    -v string_kind="$PARSER_FEATURE_STRING_KIND" \
    -v bool_kind="$PARSER_BOOL_KIND" \
    -v identifier_kind="$PARSER_FEATURE_IDENTIFIER_KIND" \
    -v list_kind="$PARSER_FEATURE_LIST_LITERAL_KIND" \
    -v binary_kind="$PARSER_FEATURE_BINARY_KIND" \
    -v unary_kind="$PARSER_FEATURE_UNARY_KIND" \
    -v call_kind="$PARSER_FEATURE_CALL_KIND" \
    -v field_access_kind="$PARSER_FEATURE_FIELD_ACCESS_KIND" \
    -v index_access_kind="$PARSER_FEATURE_INDEX_ACCESS_KIND" \
    -v map_literal_kind="$PARSER_FEATURE_MAP_LITERAL_KIND" \
    -v record_literal_kind="$PARSER_FEATURE_RECORD_LITERAL_KIND" \
    -v try_kind="$PARSER_FEATURE_TRY_KIND" \
    -v if_expr_kind="$PARSER_FEATURE_IF_EXPR_KIND" \
    -v expression_dispatch_plan="$PARSER_EXPRESSION_DISPATCH_PLAN_TEXT" \
    -v declaration_dispatch_plan="$PARSER_DECLARATION_DISPATCH_PLAN_TEXT" \
    -v statement_dispatch_plan="$PARSER_STATEMENT_DISPATCH_PLAN_TEXT" \
    -v statement_source_dispatch_plan="$PARSER_STATEMENT_SOURCE_DISPATCH_PLAN_TEXT" \
    -v take_source_dispatch_plan="$PARSER_TAKE_SOURCE_DISPATCH_PLAN_TEXT" \
    -v module_declaration_kind="$PARSER_FEATURE_MODULE_DECLARATION_KIND" \
    -v import_declaration_kind="$PARSER_FEATURE_IMPORT_DECLARATION_KIND" \
    -v type_declaration_kind="$PARSER_FEATURE_TYPE_DECLARATION_KIND" \
    -v effect_declaration_kind="$PARSER_FEATURE_EFFECT_DECLARATION_KIND" \
    -v task_declaration_kind="$PARSER_FEATURE_TASK_DECLARATION_KIND" \
    -v op_or="$PARSER_OP_OR" \
    -v op_and="$PARSER_OP_AND" \
    -v op_equal="$PARSER_OP_EQUAL" \
    -v op_not_equal="$PARSER_OP_NOT_EQUAL" \
    -v op_less="$PARSER_OP_LESS" \
    -v op_less_equal="$PARSER_OP_LESS_EQUAL" \
    -v op_greater="$PARSER_OP_GREATER" \
    -v op_greater_equal="$PARSER_OP_GREATER_EQUAL" \
    -v op_add="$PARSER_OP_ADD" \
    -v op_multiply="$PARSER_OP_MULTIPLY" \
    -v unary_op_not="$PARSER_UNARY_OP_NOT" \
    -v unary_op_negate="$PARSER_UNARY_OP_NEGATE" \
    -v raw_kind="$PARSER_RAW_KIND" \
    -v true_kind="$PARSER_FEATURE_TRUE_KIND" \
    -v false_kind="$PARSER_FEATURE_FALSE_KIND" \
    -v fallback_kind="$PARSER_FEATURE_FALLBACK_KIND" \
    -v task_id_template="$PARSER_TASK_ID_TEMPLATE" \
    -v import_id_template="$PARSER_IMPORT_ID_TEMPLATE" \
    -v type_id_template="$PARSER_TYPE_ID_TEMPLATE" \
    -v effect_id_template="$PARSER_EFFECT_ID_TEMPLATE" \
    -v take_id_template="$PARSER_TAKE_ID_TEMPLATE" \
    -v statement_id_template="$PARSER_STATEMENT_ID_TEMPLATE" \
    -v expression_id_template="$PARSER_EXPRESSION_ID_TEMPLATE" \
    -v expression_side_surface_id_template="$PARSER_EXPRESSION_SIDE_SURFACE_ID_TEMPLATE" \
    -v call_argument_surface_id_template="$PARSER_CALL_ARGUMENT_SURFACE_ID_TEMPLATE" \
    -v collection_expression_id_template="$PARSER_COLLECTION_EXPRESSION_ID_TEMPLATE" \
    -v condition_expression_id_template="$PARSER_CONDITION_EXPRESSION_ID_TEMPLATE" \
    -v for_item_name_template="$PARSER_FOR_ITEM_NAME_TEMPLATE" \
    -v for_collection_source_template="$PARSER_FOR_COLLECTION_SOURCE_TEMPLATE" \
    -v condition_source_template="$PARSER_CONDITION_SOURCE_TEMPLATE" \
    -v body_depth_offset="$PARSER_BODY_DEPTH_OFFSET" \
    -v call_expression_prefix="$PARSER_CALL_EXPRESSION_PREFIX" \
    -v return_statement_kind="$PARSER_RETURN_STATEMENT_KIND" \
    -v expr_statement_kind="$PARSER_EXPR_STATEMENT_KIND" \
    -v binding_statement_kind="$PARSER_BINDING_STATEMENT_KIND" \
    -v set_statement_kind="$PARSER_SET_STATEMENT_KIND" \
    -v for_statement_kind="$PARSER_FOR_STATEMENT_KIND" \
    -v while_statement_kind="$PARSER_WHILE_STATEMENT_KIND" \
    -v if_statement_kind="$PARSER_IF_STATEMENT_KIND" \
    -v forge_statement_kind="$PARSER_FORGE_STATEMENT_KIND" \
    -v return_feature_statement_kind="$PARSER_FEATURE_RETURN_STATEMENT_KIND" \
    -v call_feature_statement_kind="$PARSER_FEATURE_CALL_STATEMENT_KIND" \
    -v binding_feature_statement_kind="$PARSER_FEATURE_BINDING_STATEMENT_KIND" \
    -v set_feature_statement_kind="$PARSER_FEATURE_SET_STATEMENT_KIND" \
    -v for_feature_statement_kind="$PARSER_FEATURE_FOR_STATEMENT_KIND" \
    -v while_feature_statement_kind="$PARSER_FEATURE_WHILE_STATEMENT_KIND" \
    -v if_feature_statement_kind="$PARSER_FEATURE_IF_STATEMENT_KIND" \
    -v forge_feature_statement_kind="$PARSER_FEATURE_FORGE_STATEMENT_KIND" \
    -v fallback_feature_statement_kind="$PARSER_FEATURE_FALLBACK_STATEMENT_KIND" \
    -v take_binding_kind="$PARSER_TAKE_BINDING_KIND" \
    -v gate_binding_kind="$PARSER_GATE_BINDING_KIND" \
    -v state_binding_kind="$PARSER_STATE_BINDING_KIND" \
    -v tally_binding_kind="$PARSER_TALLY_BINDING_KIND" \
    -v bind_binding_kind="$PARSER_BIND_BINDING_KIND" \
    -v take_feature_binding_kind="$PARSER_FEATURE_TAKE_BINDING_KIND" \
    -v gate_feature_binding_kind="$PARSER_FEATURE_GATE_BINDING_KIND" \
    -v bind_feature_binding_kind="$PARSER_FEATURE_BIND_BINDING_KIND" \
    -v state_feature_binding_kind="$PARSER_FEATURE_STATE_BINDING_KIND" \
    -v tally_feature_binding_kind="$PARSER_FEATURE_TALLY_BINDING_KIND" '
    BEGIN {
      raw_expression_dispatch_count=split(expression_dispatch_plan, expression_dispatch_rows, "\n")
      expression_dispatch_count=0
      for(dispatch_index=1; dispatch_index<=raw_expression_dispatch_count; dispatch_index++){
        if(expression_dispatch_rows[dispatch_index]==""){continue}
        split(expression_dispatch_rows[dispatch_index], expression_dispatch_parts, "|")
        expression_dispatch_count++
        expression_dispatch_id[expression_dispatch_count]=expression_dispatch_parts[1]+0
        expression_dispatch_name[expression_dispatch_count]=expression_dispatch_parts[2]
        expression_dispatch_strategy[expression_dispatch_count]=expression_dispatch_parts[3]
        expression_dispatch_match_value[expression_dispatch_count]=expression_dispatch_parts[4]
      }
      raw_declaration_dispatch_count=split(declaration_dispatch_plan, declaration_dispatch_rows, "\n")
      declaration_dispatch_count=0
      for(dispatch_index=1; dispatch_index<=raw_declaration_dispatch_count; dispatch_index++){
        if(declaration_dispatch_rows[dispatch_index]==""){continue}
        split(declaration_dispatch_rows[dispatch_index], declaration_dispatch_parts, "|")
        declaration_dispatch_count++
        declaration_dispatch_id[declaration_dispatch_count]=declaration_dispatch_parts[1]+0
        declaration_dispatch_name[declaration_dispatch_count]=declaration_dispatch_parts[2]
        declaration_dispatch_pattern[declaration_dispatch_count]=declaration_dispatch_parts[3]
      }
      raw_statement_dispatch_count=split(statement_dispatch_plan, statement_dispatch_rows, "\n")
      statement_dispatch_count=0
      for(dispatch_index=1; dispatch_index<=raw_statement_dispatch_count; dispatch_index++){
        if(statement_dispatch_rows[dispatch_index]==""){continue}
        split(statement_dispatch_rows[dispatch_index], statement_dispatch_parts, "|")
        statement_dispatch_count++
        statement_dispatch_id[statement_dispatch_count]=statement_dispatch_parts[1]+0
        statement_dispatch_name[statement_dispatch_count]=statement_dispatch_parts[2]
        statement_dispatch_match_kind[statement_dispatch_count]=statement_dispatch_parts[3]
      }
      raw_statement_source_dispatch_count=split(statement_source_dispatch_plan, statement_source_dispatch_rows, "\n")
      statement_source_dispatch_count=0
      for(dispatch_index=1; dispatch_index<=raw_statement_source_dispatch_count; dispatch_index++){
        if(statement_source_dispatch_rows[dispatch_index]==""){continue}
        split(statement_source_dispatch_rows[dispatch_index], statement_source_dispatch_parts, "|")
        statement_source_dispatch_count++
        statement_source_dispatch_id[statement_source_dispatch_count]=statement_source_dispatch_parts[1]+0
        statement_source_dispatch_name[statement_source_dispatch_count]=statement_source_dispatch_parts[2]
        statement_source_dispatch_strategy[statement_source_dispatch_count]=statement_source_dispatch_parts[3]
        statement_source_dispatch_match_value[statement_source_dispatch_count]=statement_source_dispatch_parts[4]
        statement_source_dispatch_secondary_value[statement_source_dispatch_count]=statement_source_dispatch_parts[5]
      }
      raw_take_source_dispatch_count=split(take_source_dispatch_plan, take_source_dispatch_rows, "\n")
      take_source_dispatch_count=0
      for(dispatch_index=1; dispatch_index<=raw_take_source_dispatch_count; dispatch_index++){
        if(take_source_dispatch_rows[dispatch_index]==""){continue}
        split(take_source_dispatch_rows[dispatch_index], take_source_dispatch_parts, "|")
        take_source_dispatch_count++
        take_source_dispatch_id[take_source_dispatch_count]=take_source_dispatch_parts[1]+0
        take_source_dispatch_name[take_source_dispatch_count]=take_source_dispatch_parts[2]
        take_source_dispatch_strategy[take_source_dispatch_count]=take_source_dispatch_parts[3]
        take_source_dispatch_match_value[take_source_dispatch_count]=take_source_dispatch_parts[4]
      }
    }
    function trim(s){gsub(/^[ \t\r\n]+|[ \t\r\n]+$/, "", s); return s}
    function esc(s){gsub(/\\/, "\\\\", s); gsub(/"/, "\\\"", s); gsub(/\t/, "\\t", s); return s}
    function count_char(s, c, n, i){n=0; for(i=1;i<=length(s);i++){if(substr(s,i,1)==c)n++} return n}
    function type_json(s){s=trim(s); if(s=="")s="Unit"; return "{\"kind\":\"Named\",\"name\":\"" esc(s) "\"}"}
    function span_json(line, col){if(col<1)col=1; return "{\"line\":" line ",\"column\":" col "}"}
    function replace_all(source, needle, replacement, pos){while((pos=index(source, needle))>0){source=substr(source,1,pos-1) replacement substr(source,pos+length(needle))} return source}
    function instantiate_task_id(mod_name, task_label, id){id=task_id_template; id=replace_all(id, "{{module}}", mod_name); id=replace_all(id, "{{task}}", task_label); return id}
    function instantiate_import_id(mod_name, import_label, id){id=import_id_template; id=replace_all(id, "{{module}}", mod_name); id=replace_all(id, "{{import_module}}", import_label); return id}
    function instantiate_type_id(mod_name, type_label, id){id=type_id_template; id=replace_all(id, "{{module}}", mod_name); id=replace_all(id, "{{type}}", type_label); return id}
    function instantiate_effect_id(mod_name, effect_label, id){id=effect_id_template; id=replace_all(id, "{{module}}", mod_name); id=replace_all(id, "{{effect}}", effect_label); return id}
    function instantiate_take_id(mod_name, task_label, take_label, id){id=take_id_template; id=replace_all(id, "{{module}}", mod_name); id=replace_all(id, "{{task}}", task_label); id=replace_all(id, "{{take}}", take_label); return id}
    function instantiate_statement_id(mod_name, task_label, stmt_index, id){id=statement_id_template; id=replace_all(id, "{{module}}", mod_name); id=replace_all(id, "{{task}}", task_label); id=replace_all(id, "{{index}}", stmt_index); return id}
    function instantiate_expression_id(stmt_id, id){id=expression_id_template; id=replace_all(id, "{{statement}}", stmt_id); return id}
    function instantiate_expression_side_id(expr_id, side_name, id){id=expression_side_surface_id_template; id=replace_all(id, "{{expression}}", expr_id); id=replace_all(id, "{{side}}", side_name); return id}
    function instantiate_call_argument_id(expr_id, arg_index, id){id=call_argument_surface_id_template; id=replace_all(id, "{{expression}}", expr_id); id=replace_all(id, "{{index}}", arg_index); return id}
    function instantiate_collection_expression_id(stmt_id, id){id=collection_expression_id_template; id=replace_all(id, "{{statement}}", stmt_id); return id}
    function instantiate_condition_expression_id(stmt_id, id){id=condition_expression_id_template; id=replace_all(id, "{{statement}}", stmt_id); return id}
    function instantiate_for_item_name(item_name, value){value=for_item_name_template; value=replace_all(value, "{{item}}", item_name); return value}
    function instantiate_for_collection_source(collection_source, value){value=for_collection_source_template; value=replace_all(value, "{{collection}}", collection_source); return value}
    function instantiate_condition_source(condition_source, value){value=condition_source_template; value=replace_all(value, "{{condition}}", condition_source); return value}
    function top_level_operator(expr, op_list, ops, op_count, i, j, ch, op, in_string, paren_depth, bracket_depth, brace_depth){
      op_count=split(op_list, ops, " ")
      in_string=0; paren_depth=0; bracket_depth=0; brace_depth=0
      for(i=1;i<=length(expr);i++){
        ch=substr(expr,i,1)
        if(ch=="\"" && (i==1 || substr(expr,i-1,1)!="\\")){in_string=!in_string; continue}
        if(in_string){continue}
        if(ch=="("){paren_depth++; continue}
        if(ch==")"){if(paren_depth>0)paren_depth--; continue}
        if(ch=="["){bracket_depth++; continue}
        if(ch=="]"){if(bracket_depth>0)bracket_depth--; continue}
        if(ch=="{"){brace_depth++; continue}
        if(ch=="}"){if(brace_depth>0)brace_depth--; continue}
        if(paren_depth==0 && bracket_depth==0 && brace_depth==0){
          for(j=1;j<=op_count;j++){
            op=ops[j]
            if(op!="" && substr(expr,i,length(op))==op){return op}
          }
        }
      }
      return ""
    }
    function top_level_token_pos(expr, token, i, ch, in_string, paren_depth, bracket_depth, brace_depth){
      in_string=0; paren_depth=0; bracket_depth=0; brace_depth=0
      for(i=1;i<=length(expr);i++){
        ch=substr(expr,i,1)
        if(ch=="\"" && (i==1 || substr(expr,i-1,1)!="\\")){in_string=!in_string; continue}
        if(in_string){continue}
        if(ch=="("){paren_depth++; continue}
        if(ch==")"){if(paren_depth>0)paren_depth--; continue}
        if(ch=="["){bracket_depth++; continue}
        if(ch=="]"){if(bracket_depth>0)bracket_depth--; continue}
        if(ch=="{"){brace_depth++; continue}
        if(ch=="}"){if(brace_depth>0)brace_depth--; continue}
        if(paren_depth==0 && bracket_depth==0 && brace_depth==0 && substr(expr,i,length(token))==token){return i}
      }
      return 0
    }
    function binary_operator(expr, value) {
      if((value=top_level_operator(expr, "||"))!="") return value
      if((value=top_level_operator(expr, "&&"))!="") return value
      if((value=top_level_operator(expr, ">= <= == != > <"))!="") return value
      if((value=top_level_operator(expr, "+"))!="") return value
      if((value=top_level_operator(expr, "*"))!="") return value
      return ""
    }
    function binary_operator_name(op) {
      if(op=="||") return op_or
      if(op=="&&") return op_and
      if(op=="==") return op_equal
      if(op=="!=") return op_not_equal
      if(op=="<") return op_less
      if(op=="<=") return op_less_equal
      if(op==">") return op_greater
      if(op==">=") return op_greater_equal
      if(op=="+") return op_add
      if(op=="*") return op_multiply
      return raw_kind
    }
    function unary_operator_name(op) {
      if(op=="!") return unary_op_not
      if(op=="-") return unary_op_negate
      return raw_kind
    }
    function binary_operator_pos(expr, op){return top_level_token_pos(expr, op)}
    function binary_left_source(expr, op, pos){pos=binary_operator_pos(expr, op); if(pos < 1)return ""; return trim(substr(expr, 1, pos - 1))}
    function binary_right_source(expr, op, pos){pos=binary_operator_pos(expr, op); if(pos < 1)return ""; return trim(substr(expr, pos + length(op)))}
    function enclosure_closes_at_end(source, open_char, close_char, i, ch, in_string, depth){
      if(length(source) < 2 || substr(source,1,1) != open_char){return 0}
      in_string=0; depth=0
      for(i=1;i<=length(source);i++){
        ch=substr(source,i,1)
        if(ch=="\"" && (i==1 || substr(source,i-1,1)!="\\")){in_string=!in_string; continue}
        if(in_string){continue}
        if(ch==open_char){depth++; continue}
        if(ch==close_char){
          if(depth>0)depth--
          if(depth==0){return i==length(source)}
        }
      }
      return 0
    }
    function trailing_index_open(expr, i, ch, in_string, paren_depth, bracket_depth, brace_depth, candidate){
      in_string=0; paren_depth=0; bracket_depth=0; brace_depth=0; candidate=0
      for(i=1;i<=length(expr);i++){
        ch=substr(expr,i,1)
        if(ch=="\"" && (i==1 || substr(expr,i-1,1)!="\\")){in_string=!in_string; continue}
        if(in_string){continue}
        if(ch=="("){paren_depth++; continue}
        if(ch==")"){if(paren_depth>0)paren_depth--; continue}
        if(ch=="{"){brace_depth++; continue}
        if(ch=="}"){if(brace_depth>0)brace_depth--; continue}
        if(ch=="["){
          if(paren_depth==0 && brace_depth==0 && bracket_depth==0){candidate=i}
          bracket_depth++
          continue
        }
        if(ch=="]"){
          if(bracket_depth>0)bracket_depth--
          if(bracket_depth==0){
            if(i==length(expr)){return candidate}
            candidate=0
          }
        }
      }
      return 0
    }
    function is_call_expression(expr){return substr(expr, 1, length(call_expression_prefix)) == call_expression_prefix}
    function is_bare_call_expression(expr){return expr ~ /^[A-Za-z_][A-Za-z0-9_.]*[(].*[)]\??$/}
    function is_any_call_expression(expr){return is_call_expression(expr) || is_bare_call_expression(expr)}
    function is_field_access_expression(expr){return expr ~ /^[A-Za-z_][A-Za-z0-9_]*([.][A-Za-z_][A-Za-z0-9_]*)+$/}
    function is_index_access_expression(expr){return trailing_index_open(expr) > 1}
    function is_list_literal_expression(expr){return enclosure_closes_at_end(expr, "[", "]")}
    function map_literal_body_source(expr, source){source=trim(substr(expr, 4)); if(substr(source,1,1)!="{")return ""; return source}
    function is_map_literal_expression(expr, source){source=map_literal_body_source(expr); return source != "" && enclosure_closes_at_end(source, "{", "}")}
    function is_record_literal_expression(expr){return expr ~ /^[A-Za-z_][A-Za-z0-9_]*[ \t]+[{].*[}]$/}
    function is_try_expression(expr){return length(expr) > 1 && substr(expr, length(expr), 1) == "?"}
    function try_inner_source(expr){return trim(substr(expr, 1, length(expr) - 1))}
    function is_if_expression(expr){return expr ~ /^if[ \t]+.*[{].*[}][ \t]+else[ \t]+[{].*[}]$/}
    function if_expression_else_pos(expr){return top_level_token_pos(expr, " else ")}
    function if_expression_condition_source(expr, open_pos){open_pos=index(expr, "{"); if(open_pos < 1)return ""; return trim(substr(expr, 3, open_pos - 3))}
    function if_expression_then_source(expr, open_pos, else_pos){open_pos=index(expr, "{"); else_pos=if_expression_else_pos(expr); if(open_pos < 1 || else_pos < 1)return ""; return trim(substr(expr, open_pos + 1, else_pos - open_pos - 2))}
    function if_expression_else_source(expr, else_pos, rest, open_pos, close_pos){
      else_pos=if_expression_else_pos(expr)
      if(else_pos < 1)return ""
      rest=substr(expr, else_pos + length(" else "))
      open_pos=index(rest, "{")
      close_pos=length(rest)
      while(close_pos > 0 && substr(rest, close_pos, 1) != "}"){close_pos--}
      if(open_pos < 1 || close_pos <= open_pos){return ""}
      return trim(substr(rest, open_pos + 1, close_pos - open_pos - 1))
    }
    function is_parenthesized_expression(expr){return enclosure_closes_at_end(expr, "(", ")")}
    function parenthesized_inner_source(expr){return trim(substr(expr, 2, length(expr) - 2))}
    function is_unary_expression(expr){return length(expr) > 1 && (substr(expr,1,1) == "!" || (substr(expr,1,1) == "-" && expr !~ int_pattern))}
    function unary_operator_source(expr){return substr(expr,1,1)}
    function unary_operand_source(expr){return trim(substr(expr,2))}
    function call_expression_source(expr){
      if(is_call_expression(expr)){return substr(expr, length(call_expression_prefix) + 1)}
      return expr
    }
    function call_expression_target(expr, source, pos){
      source=call_expression_source(expr)
      pos=index(source, "(")
      if(pos > 0){source=substr(source, 1, pos - 1)}
      return trim(source)
    }
    function call_expression_args_source(expr, source, open_pos, close_pos){
      source=call_expression_source(expr)
      open_pos=index(source, "(")
      close_pos=length(source)
      while(close_pos > 0 && substr(source, close_pos, 1) != ")"){close_pos--}
      if(open_pos < 1 || close_pos <= open_pos){return ""}
      return trim(substr(source, open_pos + 1, close_pos - open_pos - 1))
    }
    function call_expression_args_json(expr, id, line, col, source, count, i, out, arg_source, start, ch, in_string, paren_depth, bracket_depth, brace_depth){
      source=call_expression_args_source(expr)
      if(source=="") return "[]"
      out="["
      count=0; start=1; in_string=0; paren_depth=0; bracket_depth=0; brace_depth=0
      for(i=1;i<=length(source);i++){
        ch=substr(source,i,1)
        if(ch=="\"" && (i==1 || substr(source,i-1,1)!="\\")){in_string=!in_string; continue}
        if(in_string){continue}
        if(ch=="("){paren_depth++; continue}
        if(ch==")"){if(paren_depth>0)paren_depth--; continue}
        if(ch=="["){bracket_depth++; continue}
        if(ch=="]"){if(bracket_depth>0)bracket_depth--; continue}
        if(ch=="{"){brace_depth++; continue}
        if(ch=="}"){if(brace_depth>0)brace_depth--; continue}
        if(ch=="," && paren_depth==0 && bracket_depth==0 && brace_depth==0){
          arg_source=trim(substr(source, start, i - start))
          if(arg_source!=""){
            if(count>0) out=out ","
            out=out expr_json(arg_source, instantiate_call_argument_id(id, count), line, col)
            count++
          }
          start=i + 1
        }
      }
      arg_source=trim(substr(source, start))
      if(arg_source!=""){
        if(count>0) out=out ","
        out=out expr_json(arg_source, instantiate_call_argument_id(id, count), line, col)
      }
      out=out "]"
      return out
    }
    function field_access_field(expr, parts, count){count=split(expr, parts, "."); return parts[count]}
    function field_access_target(expr, field, target){field=field_access_field(expr); target=expr; sub("[.]" field "$", "", target); return target}
    function index_access_target(expr, open_pos){open_pos=trailing_index_open(expr); return trim(substr(expr, 1, open_pos - 1))}
    function index_access_source(expr, open_pos, close_pos){
      open_pos=trailing_index_open(expr)
      close_pos=length(expr)
      while(close_pos > 0 && substr(expr, close_pos, 1) != "]"){close_pos--}
      if(open_pos < 1 || close_pos <= open_pos){return ""}
      return trim(substr(expr, open_pos + 1, close_pos - open_pos - 1))
    }
    function list_literal_items_source(expr){return trim(substr(expr, 2, length(expr) - 2))}
    function list_literal_items_json(expr, id, line, col, source, count, i, out, item_source, start, ch, in_string, paren_depth, bracket_depth, brace_depth){
      source=list_literal_items_source(expr)
      if(source=="") return "[]"
      out="["
      count=0; start=1; in_string=0; paren_depth=0; bracket_depth=0; brace_depth=0
      for(i=1;i<=length(source);i++){
        ch=substr(source,i,1)
        if(ch=="\"" && (i==1 || substr(source,i-1,1)!="\\")){in_string=!in_string; continue}
        if(in_string){continue}
        if(ch=="("){paren_depth++; continue}
        if(ch==")"){if(paren_depth>0)paren_depth--; continue}
        if(ch=="["){bracket_depth++; continue}
        if(ch=="]"){if(bracket_depth>0)bracket_depth--; continue}
        if(ch=="{"){brace_depth++; continue}
        if(ch=="}"){if(brace_depth>0)brace_depth--; continue}
        if(ch=="," && paren_depth==0 && bracket_depth==0 && brace_depth==0){
          item_source=trim(substr(source, start, i - start))
          if(item_source!=""){
            if(count>0) out=out ","
            out=out expr_json(item_source, instantiate_call_argument_id(id, count), line, col)
            count++
          }
          start=i + 1
        }
      }
      item_source=trim(substr(source, start))
      if(item_source!=""){
        if(count>0) out=out ","
        out=out expr_json(item_source, instantiate_call_argument_id(id, count), line, col)
      }
      out=out "]"
      return out
    }
    function map_literal_entries_source(expr, source){source=map_literal_body_source(expr); return trim(substr(source, 2, length(source) - 2))}
    function map_literal_entry_json(entry, id, entry_index, line, col, pos, key_source, value_source){
      pos=top_level_token_pos(entry, ":")
      if(pos < 1){return ""}
      key_source=trim(substr(entry, 1, pos - 1))
      value_source=trim(substr(entry, pos + 1))
      if(key_source=="" || value_source==""){return ""}
      return "{\"key\":" expr_json(key_source, instantiate_expression_side_id(instantiate_call_argument_id(id, entry_index), "key"), line, col) ",\"value\":" expr_json(value_source, instantiate_expression_side_id(instantiate_call_argument_id(id, entry_index), "value"), line, col) "}"
    }
    function map_literal_entries_json(expr, id, line, col, source, count, i, out, entry, entry_json, start, ch, in_string, paren_depth, bracket_depth, brace_depth){
      source=map_literal_entries_source(expr)
      if(source=="") return "[]"
      out="["
      count=0; start=1; in_string=0; paren_depth=0; bracket_depth=0; brace_depth=0
      for(i=1;i<=length(source);i++){
        ch=substr(source,i,1)
        if(ch=="\"" && (i==1 || substr(source,i-1,1)!="\\")){in_string=!in_string; continue}
        if(in_string){continue}
        if(ch=="("){paren_depth++; continue}
        if(ch==")"){if(paren_depth>0)paren_depth--; continue}
        if(ch=="["){bracket_depth++; continue}
        if(ch=="]"){if(bracket_depth>0)bracket_depth--; continue}
        if(ch=="{"){brace_depth++; continue}
        if(ch=="}"){if(brace_depth>0)brace_depth--; continue}
        if(ch=="," && paren_depth==0 && bracket_depth==0 && brace_depth==0){
          entry=trim(substr(source, start, i - start))
          entry_json=map_literal_entry_json(entry, id, count, line, col)
          if(entry_json!=""){
            if(count>0) out=out ","
            out=out entry_json
            count++
          }
          start=i + 1
        }
      }
      entry=trim(substr(source, start))
      entry_json=map_literal_entry_json(entry, id, count, line, col)
      if(entry_json!=""){
        if(count>0) out=out ","
        out=out entry_json
      }
      out=out "]"
      return out
    }
    function record_literal_type_name(expr, open_pos){open_pos=index(expr, "{"); if(open_pos < 1)return ""; return trim(substr(expr, 1, open_pos - 1))}
    function record_literal_fields_source(expr, open_pos, close_pos){
      open_pos=index(expr, "{")
      close_pos=length(expr)
      while(close_pos > 0 && substr(expr, close_pos, 1) != "}"){close_pos--}
      if(open_pos < 1 || close_pos <= open_pos){return ""}
      return trim(substr(expr, open_pos + 1, close_pos - open_pos - 1))
    }
    function record_literal_entry_json(entry, id, field_index, line, col, pos, name, value){
      pos=top_level_token_pos(entry, ":")
      if(pos < 1){return ""}
      name=trim(substr(entry, 1, pos - 1))
      value=trim(substr(entry, pos + 1))
      if(name==""){return ""}
      return "{\"name\":\"" esc(name) "\",\"expr\":" expr_json(value, instantiate_call_argument_id(id, field_index), line, col) "}"
    }
    function record_literal_fields_json(expr, id, line, col, source, count, i, out, entry, entry_json, start, ch, in_string, paren_depth, bracket_depth, brace_depth){
      source=record_literal_fields_source(expr)
      if(source=="") return "[]"
      out="["
      count=0; start=1; in_string=0; paren_depth=0; bracket_depth=0; brace_depth=0
      for(i=1;i<=length(source);i++){
        ch=substr(source,i,1)
        if(ch=="\"" && (i==1 || substr(source,i-1,1)!="\\")){in_string=!in_string; continue}
        if(in_string){continue}
        if(ch=="("){paren_depth++; continue}
        if(ch==")"){if(paren_depth>0)paren_depth--; continue}
        if(ch=="["){bracket_depth++; continue}
        if(ch=="]"){if(bracket_depth>0)bracket_depth--; continue}
        if(ch=="{"){brace_depth++; continue}
        if(ch=="}"){if(brace_depth>0)brace_depth--; continue}
        if(ch=="," && paren_depth==0 && bracket_depth==0 && brace_depth==0){
          entry=trim(substr(source, start, i - start))
          entry_json=record_literal_entry_json(entry, id, count, line, col)
          if(entry_json!=""){
            if(count>0) out=out ","
            out=out entry_json
            count++
          }
          start=i + 1
        }
      }
      entry=trim(substr(source, start))
      entry_json=record_literal_entry_json(entry, id, count, line, col)
      if(entry_json!=""){
        if(count>0) out=out ","
        out=out entry_json
      }
      out=out "]"
      return out
    }
    function expr_candidate_matches(candidate_id, expr, strategy, match_value){
      if(strategy=="pattern" && match_value!=""){return expr ~ match_value}
      if(strategy=="string"){return expr ~ /^"[^"]*"$/}
      if(strategy=="literal" && match_value!=""){return expr == match_value}
      if(strategy=="parenthesized"){return is_parenthesized_expression(expr)}
      if(strategy=="try"){return is_try_expression(expr)}
      if(strategy=="if"){return is_if_expression(expr)}
      if(strategy=="unary"){return is_unary_expression(expr)}
      if(strategy=="index"){return is_index_access_expression(expr)}
      if(strategy=="call"){return is_any_call_expression(expr)}
      if(strategy=="list"){return is_list_literal_expression(expr)}
      if(strategy=="map"){return is_map_literal_expression(expr)}
      if(strategy=="record"){return is_record_literal_expression(expr)}
      if(strategy=="binary"){return binary_operator(expr) != ""}
      if(strategy=="field"){return is_field_access_expression(expr)}
      if(strategy=="always"){return 1}
      return 0
    }
    function expr_candidate_json(candidate_id, expr, id, line, col, strategy, match_value, value, left_source, right_source, op_name, callee_source){
      if(!expr_candidate_matches(candidate_id, expr, strategy, match_value)){return ""}
      if(candidate_id==0){return "{\"id\":\"" id "\",\"source\":\"" esc(expr) "\",\"expr_kind\":\"" int_kind "\",\"value\":" expr ",\"span\":" span_json(line,col) "}"}
      if(candidate_id==1){value=expr; sub(/^"/,"",value); sub(/"$/,"",value); return "{\"id\":\"" id "\",\"source\":\"" esc(expr) "\",\"expr_kind\":\"" string_kind "\",\"value\":\"" esc(value) "\",\"span\":" span_json(line,col) "}"}
      if(candidate_id==2){return "{\"id\":\"" id "\",\"source\":\"" esc(expr) "\",\"expr_kind\":\"" true_kind "\",\"value\":true,\"span\":" span_json(line,col) "}"}
      if(candidate_id==3){return "{\"id\":\"" id "\",\"source\":\"" esc(expr) "\",\"expr_kind\":\"" false_kind "\",\"value\":false,\"span\":" span_json(line,col) "}"}
      if(candidate_id==4){return "{\"id\":\"" id "\",\"source\":\"[]\",\"expr_kind\":\"" list_kind "\",\"items\":[],\"span\":" span_json(line,col) "}"}
      if(candidate_id==5){return "{\"id\":\"" id "\",\"source\":\"" esc(expr) "\",\"expr_kind\":\"" identifier_kind "\",\"name\":\"" esc(expr) "\",\"span\":" span_json(line,col) "}"}
      if(candidate_id==6){return expr_json(parenthesized_inner_source(expr), id, line, col + 1)}
      if(candidate_id==7){return "{\"id\":\"" id "\",\"source\":\"" esc(expr) "\",\"expr_kind\":\"" try_kind "\",\"expr\":" expr_json(try_inner_source(expr), instantiate_expression_side_id(id, "try"), line, col) ",\"span\":" span_json(line,col) "}"}
      if(candidate_id==8){return "{\"id\":\"" id "\",\"source\":\"" esc(expr) "\",\"expr_kind\":\"" if_expr_kind "\",\"condition\":" expr_json(if_expression_condition_source(expr), instantiate_expression_side_id(id, "condition"), line, col + 3) ",\"then_branch\":" expr_json(if_expression_then_source(expr), instantiate_expression_side_id(id, "then"), line, col) ",\"else_branch\":" expr_json(if_expression_else_source(expr), instantiate_expression_side_id(id, "else"), line, col) ",\"span\":" span_json(line,col) "}"}
      if(candidate_id==9){return "{\"id\":\"" id "\",\"source\":\"" esc(expr) "\",\"expr_kind\":\"" unary_kind "\",\"op\":\"" esc(unary_operator_name(unary_operator_source(expr))) "\",\"expr\":" expr_json(unary_operand_source(expr), instantiate_expression_side_id(id, "operand"), line, col + 1) ",\"span\":" span_json(line,col) "}"}
      if(candidate_id==10){return "{\"id\":\"" id "\",\"source\":\"" esc(expr) "\",\"expr_kind\":\"" index_access_kind "\",\"collection\":" expr_json(index_access_target(expr), instantiate_expression_side_id(id, "collection"), line, col) ",\"index\":" expr_json(index_access_source(expr), instantiate_expression_side_id(id, "index"), line, col + length(index_access_target(expr)) + 2) ",\"span\":" span_json(line,col) "}"}
      if(candidate_id==11){callee_source=call_expression_target(expr); return "{\"id\":\"" id "\",\"source\":\"" esc(expr) "\",\"expr_kind\":\"" call_kind "\",\"callee\":" expr_json(callee_source, instantiate_expression_side_id(id, "callee"), line, col + (is_call_expression(expr) ? length(call_expression_prefix) : 0)) ",\"args\":" call_expression_args_json(expr, id, line, col) ",\"span\":" span_json(line,col) "}"}
      if(candidate_id==12){return "{\"id\":\"" id "\",\"source\":\"" esc(expr) "\",\"expr_kind\":\"" list_kind "\",\"items\":" list_literal_items_json(expr, id, line, col) ",\"span\":" span_json(line,col) "}"}
      if(candidate_id==13){return "{\"id\":\"" id "\",\"source\":\"" esc(expr) "\",\"expr_kind\":\"" map_literal_kind "\",\"entries\":" map_literal_entries_json(expr, id, line, col) ",\"span\":" span_json(line,col) "}"}
      if(candidate_id==14){return "{\"id\":\"" id "\",\"source\":\"" esc(expr) "\",\"expr_kind\":\"" record_literal_kind "\",\"type_name\":\"" esc(record_literal_type_name(expr)) "\",\"fields\":" record_literal_fields_json(expr, id, line, col) ",\"span\":" span_json(line,col) "}"}
      if(candidate_id==15){value=binary_operator(expr); if(value==""){return ""}; left_source=binary_left_source(expr, value); right_source=binary_right_source(expr, value); op_name=binary_operator_name(value); return "{\"id\":\"" id "\",\"source\":\"" esc(expr) "\",\"expr_kind\":\"" binary_kind "\",\"op\":\"" esc(op_name) "\",\"left\":" expr_json(left_source, instantiate_expression_side_id(id, "left"), line, col) ",\"right\":" expr_json(right_source, instantiate_expression_side_id(id, "right"), line, col + binary_operator_pos(expr, value) + length(value)) ",\"span\":" span_json(line,col) "}"}
      if(candidate_id==16){return "{\"id\":\"" id "\",\"source\":\"" esc(expr) "\",\"expr_kind\":\"" field_access_kind "\",\"receiver\":" expr_json(field_access_target(expr), instantiate_expression_side_id(id, "receiver"), line, col) ",\"field\":\"" esc(field_access_field(expr)) "\",\"span\":" span_json(line,col) "}"}
      if(candidate_id==17){return "{\"id\":\"" id "\",\"source\":\"" esc(expr) "\",\"expr_kind\":\"" fallback_kind "\",\"fallible\":" (expr ~ /\?$/ ? "true" : "false") ",\"span\":" span_json(line,col) "}"}
      return ""
    }
    function expr_json(expr, id, line, col, dispatch_index, rendered){
      expr=trim(expr)
      for(dispatch_index=1; dispatch_index<=expression_dispatch_count; dispatch_index++){
        rendered=expr_candidate_json(expression_dispatch_id[dispatch_index], expr, id, line, col, expression_dispatch_strategy[dispatch_index], expression_dispatch_match_value[dispatch_index])
        if(rendered!=""){return rendered}
      }
      return "{\"id\":\"" id "\",\"source\":\"" esc(expr) "\",\"expr_kind\":\"" fallback_kind "\",\"fallible\":" (expr ~ /\?$/ ? "true" : "false") ",\"span\":" span_json(line,col) "}"
    }
    function binding_kind(k){return k=="state" ? state_feature_binding_kind : (k=="tally" ? tally_feature_binding_kind : bind_feature_binding_kind)}
	    function add_stmt(kind, txt, line, col, id){
	      stmt_count[current]++
	      id=instantiate_statement_id(task_module[current], task_name[current], stmt_count[current]-1)
	      stmt_kind[current,stmt_count[current]]=kind
	      stmt_text[current,stmt_count[current]]=txt
	      stmt_line[current,stmt_count[current]]=line
	      stmt_col[current,stmt_count[current]]=col
	      stmt_id[current,stmt_count[current]]=id
	      stmt_depth[current,stmt_count[current]]=depth
	    }
	    function begin_multiline_record_stmt(kind, txt, line, col){
	      multiline_record_active=1
	      multiline_record_kind=kind
	      multiline_record_text=txt
	      multiline_record_line=line
	      multiline_record_col=col
	      multiline_record_depth=count_char(txt,"{")-count_char(txt,"}")
	      multiline_record_statement_depth=depth
	    }
	    function finish_multiline_record_stmt(id){
	      stmt_count[current]++
	      id=instantiate_statement_id(task_module[current], task_name[current], stmt_count[current]-1)
	      stmt_kind[current,stmt_count[current]]=multiline_record_kind
	      stmt_text[current,stmt_count[current]]=multiline_record_text
	      stmt_line[current,stmt_count[current]]=multiline_record_line
	      stmt_col[current,stmt_count[current]]=multiline_record_col
	      stmt_id[current,stmt_count[current]]=id
	      stmt_depth[current,stmt_count[current]]=multiline_record_statement_depth
	      multiline_record_active=0
	      multiline_record_kind=""
	      multiline_record_text=""
	      multiline_record_depth=0
	    }
	    function push_block(stmt_index, branch){
	      block_stack_count++
	      block_stack_task[block_stack_count]=current
	      block_stack_stmt[block_stack_count]=stmt_index
	      block_stack_branch[block_stack_count]=branch
	      block_stack_depth[block_stack_count]=depth + body_depth_offset
	    }
	    function close_active_block(line_number, opens_else, stmt_index, branch){
	      if(block_stack_count < 1){return}
	      if(block_stack_task[block_stack_count] != current){return}
	      if(block_stack_depth[block_stack_count] != depth){return}
	      stmt_index=block_stack_stmt[block_stack_count]
	      branch=block_stack_branch[block_stack_count]
	      if(opens_else && branch=="then"){
	        stmt_then_end_line[current,stmt_index]=line_number
	        stmt_else_start_line[current,stmt_index]=line_number
	        block_stack_branch[block_stack_count]="else"
	        return
	      }
	      if(branch=="then"){stmt_then_end_line[current,stmt_index]=line_number}
	      else if(branch=="else"){stmt_else_end_line[current,stmt_index]=line_number}
	      else{stmt_body_end_line[current,stmt_index]=line_number}
	      block_stack_count--
	    }
	    function print_block_statements(i, owner, start_line, end_line, target_depth, first, k){
	      first=1
	      if(end_line <= start_line){return}
	      for(k=1;k<=stmt_count[i];k++){
	        if(k != owner && stmt_line[i,k] > start_line && stmt_line[i,k] < end_line && stmt_depth[i,k] == target_depth){
	          if(!first)printf ","
	          print_statement(i,k)
	          first=0
	        }
	      }
	    }
	    function statement_candidate_print(candidate_id, i, j, id, parts, expected_kind, target_depth){
	      if(expected_kind=="" || parts[1] != expected_kind){return 0}
	      if(candidate_id==0){printf "{\"id\":\"%s\",\"kind\":\"%s\",\"expr\":%s,\"span\":%s}", id, return_statement_kind, expr_json(stmt_text[i,j], instantiate_expression_id(id), stmt_line[i,j], stmt_col[i,j]+7), span_json(stmt_line[i,j],stmt_col[i,j]); return 1}
	      if(candidate_id==1){printf "{\"id\":\"%s\",\"kind\":\"%s\",\"expr\":%s,\"span\":%s}", id, expr_statement_kind, expr_json(stmt_text[i,j], instantiate_expression_id(id), stmt_line[i,j], stmt_col[i,j]), span_json(stmt_line[i,j],stmt_col[i,j]); return 1}
	      if(candidate_id==2){printf "{\"id\":\"%s\",\"kind\":\"%s\",\"binding_kind\":\"%s\",\"name\":\"%s\",\"expr\":%s,\"span\":%s}", id, binding_statement_kind, parts[2], esc(parts[3]), expr_json(stmt_text[i,j], instantiate_expression_id(id), stmt_line[i,j], stmt_col[i,j]+length(parts[3])+4), span_json(stmt_line[i,j],stmt_col[i,j]); return 1}
	      if(candidate_id==3){printf "{\"id\":\"%s\",\"kind\":\"%s\",\"name\":\"%s\",\"expr\":%s,\"span\":%s}", id, set_statement_kind, esc(parts[2]), expr_json(stmt_text[i,j], instantiate_expression_id(id), stmt_line[i,j], stmt_col[i,j]+length(parts[2])+4), span_json(stmt_line[i,j],stmt_col[i,j]); return 1}
	      if(candidate_id==4){
	        target_depth=stmt_depth[i,j]+body_depth_offset
	        printf "{\"id\":\"%s\",\"kind\":\"%s\",\"item\":\"%s\",\"collection\":%s,\"body\":{\"statements\":[", id, for_statement_kind, esc(parts[2]), expr_json(stmt_text[i,j], instantiate_collection_expression_id(id), stmt_line[i,j], stmt_col[i,j]+4)
	        print_block_statements(i,j,stmt_line[i,j],stmt_body_end_line[i,j],target_depth)
	        printf "]},\"span\":%s}", span_json(stmt_line[i,j],stmt_col[i,j])
	        return 1
	      }
	      if(candidate_id==5){
	        target_depth=stmt_depth[i,j]+body_depth_offset
	        printf "{\"id\":\"%s\",\"kind\":\"%s\",\"condition\":%s,\"body\":{\"statements\":[", id, while_statement_kind, expr_json(stmt_text[i,j], instantiate_condition_expression_id(id), stmt_line[i,j], stmt_col[i,j]+6)
	        print_block_statements(i,j,stmt_line[i,j],stmt_body_end_line[i,j],target_depth)
	        printf "]},\"span\":%s}", span_json(stmt_line[i,j],stmt_col[i,j])
	        return 1
	      }
	      if(candidate_id==6){
	        target_depth=stmt_depth[i,j]+body_depth_offset
	        printf "{\"id\":\"%s\",\"kind\":\"%s\",\"condition\":%s,\"then_block\":{\"statements\":[", id, if_statement_kind, expr_json(stmt_text[i,j], instantiate_condition_expression_id(id), stmt_line[i,j], stmt_col[i,j]+3)
	        print_block_statements(i,j,stmt_line[i,j],stmt_then_end_line[i,j],target_depth)
	        printf "]},\"else_block\":{\"statements\":["
	        print_block_statements(i,j,stmt_else_start_line[i,j],stmt_else_end_line[i,j],target_depth)
	        printf "]},\"span\":%s}", span_json(stmt_line[i,j],stmt_col[i,j])
	        return 1
	      }
	      if(candidate_id==7){
	        target_depth=stmt_depth[i,j]+body_depth_offset
	        printf "{\"id\":\"%s\",\"kind\":\"%s\",\"body\":{\"statements\":[", id, forge_statement_kind
	        print_block_statements(i,j,stmt_line[i,j],stmt_body_end_line[i,j],target_depth)
	        printf "]},\"span\":%s}", span_json(stmt_line[i,j],stmt_col[i,j])
	        return 1
	      }
	      return 0
	    }
	    function print_statement(i, j, id, parts, dispatch_index){
	      split(stmt_kind[i,j], parts, /\|/); id=stmt_id[i,j]
	      for(dispatch_index=1; dispatch_index<=statement_dispatch_count; dispatch_index++){
	        if(statement_candidate_print(statement_dispatch_id[dispatch_index], i, j, id, parts, statement_dispatch_match_kind[dispatch_index])){return}
	      }
	    }
	    function declaration_candidate_matches(candidate_id, line, pattern){
	      if(pattern==""){return 0}
	      return line ~ pattern
	    }
	    function declaration_candidate_id(line, dispatch_index){
	      for(dispatch_index=1; dispatch_index<=declaration_dispatch_count; dispatch_index++){
	        if(declaration_candidate_matches(declaration_dispatch_id[dispatch_index], line, declaration_dispatch_pattern[dispatch_index])){return declaration_dispatch_id[dispatch_index]}
	      }
	      return -1
	    }
	    function statement_source_candidate_matches(candidate_id, line, depth, strategy, match_value, secondary_value){
	      if(strategy=="pattern" && match_value!=""){return line ~ match_value}
	      if(strategy=="prefix" && match_value!=""){return index(line, match_value) == 1}
	      if(strategy=="depth_pattern" && match_value!="" && secondary_value!=""){return depth==(match_value+0) && line ~ secondary_value}
	      return 0
	    }
	    function statement_source_candidate_id(line, depth, dispatch_index){
	      for(dispatch_index=1; dispatch_index<=statement_source_dispatch_count; dispatch_index++){
	        if(statement_source_candidate_matches(statement_source_dispatch_id[dispatch_index], line, depth, statement_source_dispatch_strategy[dispatch_index], statement_source_dispatch_match_value[dispatch_index], statement_source_dispatch_secondary_value[dispatch_index])){return statement_source_dispatch_id[dispatch_index]}
	      }
	      return -1
	    }
	    function take_source_candidate_matches(candidate_id, source, strategy, match_value){
	      if(strategy=="pattern" && match_value!=""){return source ~ match_value}
	      if(strategy=="not_pattern" && match_value!=""){return source !~ match_value}
	      return 0
	    }
	    function take_source_candidate_id(source, dispatch_index){
	      for(dispatch_index=1; dispatch_index<=take_source_dispatch_count; dispatch_index++){
	        if(take_source_candidate_matches(take_source_dispatch_id[dispatch_index], source, take_source_dispatch_strategy[dispatch_index], take_source_dispatch_match_value[dispatch_index])){return take_source_dispatch_id[dispatch_index]}
	      }
	      return -1
	    }
	    FNR==1 {module="main"; file_seen[FILENAME]=1}
	    !in_task {
	      declaration_id=declaration_candidate_id(trim($0))
	      if(declaration_id==0){
	        module=$2; if(entry=="") entry=module; modules[module]=1
	        next
	      }
	      if(declaration_id==1){
	        line=trim($0); sub(/^import[ \t]+/,"",line); alias="";
	        if(line ~ /[ \t]+as[ \t]+/){alias=line; sub(/^.*[ \t]+as[ \t]+/,"",alias); sub(/[ \t]+as[ \t]+.*$/,"",line)}
	        import_count++; import_owner[import_count]=module; import_module[import_count]=trim(line); import_alias[import_count]=trim(alias); import_line[import_count]=FNR
	        next
	      }
	      if(declaration_id==2){
	        line=trim($0); exported=(line ~ /^export[ \t]+/); sub(/^export[ \t]+/,"",line); sub(/^type[ \t]+/,"",line);
	        name=line; sub(/[ \t={].*$/,"",name); type_count++; type_module[type_count]=module; type_name[type_count]=name; type_exported[type_count]=exported; type_line[type_count]=FNR
	        next
	      }
	      if(declaration_id==3){
	        line=trim($0); exported=(line ~ /^export[ \t]+/); sub(/^export[ \t]+/,"",line); sub(/^effect[ \t]+/,"",line);
	        name=line; sub(/[ \t{].*$/,"",name); effect_count++; effect_module[effect_count]=module; effect_name[effect_count]=name; effect_exported[effect_count]=exported; effect_line[effect_count]=FNR
	        next
	      }
	      if(declaration_id==4){
	        line=trim($0); exported=(line ~ /^export[ \t]+/); sub(/^export[ \t]+/,"",line); sub(/^task[ \t]+/,"",line)
	        name=line; sub(/[ \t]*->.*/,"",name)
	        rest=line; sub(/^.*->[ \t]*/,"",rest); ret=rest; sub(/[ \t]+uses[ \t]+.*/,"",ret); sub(/[ \t]*\{.*/,"",ret)
	        effects=""; if(rest ~ /[ \t]+uses[ \t]+/){effects=rest; sub(/^.*[ \t]+uses[ \t]+/,"",effects); sub(/[ \t]*\{.*/,"",effects)}
	        task_count++; current=task_count; task_module[current]=module; task_name[current]=trim(name); task_return[current]=trim(ret); task_exported[current]=exported; task_effects[current]=trim(effects); task_line[current]=FNR
	        in_task=1; block_stack_count=0; depth=count_char($0,"{")-count_char($0,"}")
	        next
	      }
	    }
	    in_task {
	      raw=$0; line=trim(raw)
	      if(multiline_record_active){
	        multiline_record_text=multiline_record_text " " line
	        multiline_record_depth += count_char(raw,"{")-count_char(raw,"}")
	        depth += count_char(raw,"{")-count_char(raw,"}")
	        if(multiline_record_depth<=0){finish_multiline_record_stmt()}
	        if(depth<=0){in_task=0; current=0}
	        next
	      }
	      if(line ~ /^}[ \t]*else[ \t]*\{/){close_active_block(FNR, 1)}
	      else if(line ~ /^}/){close_active_block(FNR, 0)}
	      statement_source_id=statement_source_candidate_id(line, depth)
	      if(statement_source_id==0) {
	        take_count[current]++; t=line; sub(/^take[ \t]+/,"",t); kind=take_feature_binding_kind
	        take_source_id=take_source_candidate_id(t)
	        if(take_source_id==0){kind=gate_feature_binding_kind; sub(/^gate[ \t]+/,"",t)}
	        name=t; sub(/[ \t]*:.*/,"",name); typ=t; sub(/^.*:[ \t]*/,"",typ)
	        take_name[current,take_count[current]]=trim(name); take_kind[current,take_count[current]]=kind; take_type[current,take_count[current]]=trim(typ); take_line[current,take_count[current]]=FNR
	      } else if(statement_source_id==1) {
        kind=line; sub(/[ \t].*/,"",kind); rest=line; sub(/^[A-Za-z]+[ \t]+/,"",rest); name=rest; sub(/[ \t:=].*/,"",name); expr=rest; sub(/^.*=[ \t]*/,"",expr)
	        if(expr ~ multiline_record_literal_open_pattern){
	          begin_multiline_record_stmt(binding_feature_statement_kind "|" binding_kind(kind) "|" trim(name), expr, FNR, index(raw, kind))
	        } else {
	          add_stmt(binding_feature_statement_kind "|" binding_kind(kind) "|" trim(name), expr, FNR, index(raw, kind))
	        }
      } else if(statement_source_id==2) {
        rest=line; sub(/^set[ \t]+/,"",rest); name=rest; sub(/[ \t]*=.*/,"",name); expr=rest; sub(/^.*=[ \t]*/,"",expr)
        add_stmt(set_feature_statement_kind "|" trim(name), expr, FNR, index(raw, "set"))
      } else if(statement_source_id==3) {
	        expr=line; sub(/^return[ \t]+/,"",expr)
	        if(expr ~ multiline_record_literal_open_pattern){
	          begin_multiline_record_stmt(return_feature_statement_kind, expr, FNR, index(raw, "return"))
	        } else {
	          add_stmt(return_feature_statement_kind, expr, FNR, index(raw, "return"))
	        }
      } else if(statement_source_id==4) {
        add_stmt(call_feature_statement_kind, line, FNR, index(raw, call_expression_prefix))
      } else if(statement_source_id==5) {
        for_item="item"; for_collection="[]"; for_line=line
        if(for_line ~ /^for[ \t]+[A-Za-z_][A-Za-z0-9_]*[ \t]+in[ \t]+.*[ \t]*[{][ \t]*$/) {
          for_item=for_line; sub(/^for[ \t]+/,"",for_item); sub(/[ \t]+in[ \t]+.*$/,"",for_item)
          for_collection=for_line; sub(/^for[ \t]+[A-Za-z_][A-Za-z0-9_]*[ \t]+in[ \t]+/,"",for_collection); sub(/[ \t]*[{][ \t]*$/,"",for_collection)
	        }
	        add_stmt(for_feature_statement_kind "|" instantiate_for_item_name(trim(for_item)), instantiate_for_collection_source(trim(for_collection)), FNR, index(raw, "for"))
	        push_block(stmt_count[current], "body")
	      } else if(statement_source_id==6) {
	        condition=line; sub(/^while[ \t]+/,"",condition); sub(/[ \t]*[{][ \t]*$/,"",condition)
	        add_stmt(while_feature_statement_kind, instantiate_condition_source(trim(condition)), FNR, index(raw, "while"))
	        push_block(stmt_count[current], "body")
	      } else if(statement_source_id==7) {
	        condition=line; sub(/^if[ \t]+/,"",condition); sub(/[ \t]*[{][ \t]*$/,"",condition)
	        add_stmt(if_feature_statement_kind, instantiate_condition_source(trim(condition)), FNR, index(raw, "if"))
	        push_block(stmt_count[current], "then")
	      } else if(statement_source_id==8) {
	        add_stmt(forge_feature_statement_kind, line, FNR, index(raw, "forge"))
	        push_block(stmt_count[current], "body")
	      } else if(statement_source_id==9) {
	        add_stmt(fallback_feature_statement_kind, line, FNR, index(raw, line))
      }
      depth += count_char($0,"{")-count_char($0,"}")
      if(depth<=0){in_task=0; current=0}
      next
    }
    END {
      if(entry=="") entry=module
      printf "{\n  \"schema\": \"%s\",\n  \"module\": \"%s\",\n  \"imports\": [", schema, esc(entry)
      for(i=1;i<=import_count;i++){if(i>1)printf ","; import_id_value=instantiate_import_id(import_owner[i], import_module[i]); printf "\n    {\"id\":\"%s\",\"owner_module\":\"%s\",\"module\":\"%s\"", esc(import_id_value), esc(import_owner[i]), esc(import_module[i]); if(import_alias[i]!="")printf ",\"alias\":\"%s\"", esc(import_alias[i]); printf ",\"span\":%s}", span_json(import_line[i],1)}
      printf "\n  ],\n  \"types\": ["
      for(i=1;i<=type_count;i++){if(i>1)printf ","; type_id_value=instantiate_type_id(type_module[i], type_name[i]); printf "\n    {\"id\":\"%s\",\"module\":\"%s\",\"exported\":%s,\"name\":\"%s\",\"value\":%s,\"span\":%s}", esc(type_id_value), esc(type_module[i]), type_exported[i]?"true":"false", esc(type_name[i]), type_json("Unit"), span_json(type_line[i],1)}
      printf "\n  ],\n  \"effects\": ["
      for(i=1;i<=effect_count;i++){if(i>1)printf ","; effect_id_value=instantiate_effect_id(effect_module[i], effect_name[i]); printf "\n    {\"id\":\"%s\",\"module\":\"%s\",\"exported\":%s,\"name\":\"%s\",\"span\":%s}", esc(effect_id_value), esc(effect_module[i]), effect_exported[i]?"true":"false", esc(effect_name[i]), span_json(effect_line[i],1)}
      printf "\n  ],\n  \"tasks\": ["
      for(i=1;i<=task_count;i++){
        if(i>1)printf ","
        task_id_value=instantiate_task_id(task_module[i], task_name[i])
        printf "\n    {\"id\":\"%s\",\"module\":\"%s\",\"exported\":%s,\"name\":\"%s\",\"takes\":[", esc(task_id_value), esc(task_module[i]), task_exported[i]?"true":"false", esc(task_name[i])
        for(j=1;j<=take_count[i];j++){if(j>1)printf ","; take_id_value=instantiate_take_id(task_module[i], task_name[i], take_name[i,j]); printf "{\"id\":\"%s\",\"name\":\"%s\",\"binding_kind\":\"%s\",\"type\":%s,\"span\":%s}", esc(take_id_value), esc(take_name[i,j]), take_kind[i,j], type_json(take_type[i,j]), span_json(take_line[i,j],1)}
	        printf "],\"return_type\":%s,\"effects\":[", type_json(task_return[i])
	        n=split(task_effects[i], effs, /[ \t,]+/); first=1; for(e=1;e<=n;e++){if(effs[e]!=""){if(!first)printf ","; printf "\"%s\"", esc(effs[e]); first=0}}
	        printf "],\"body\":{\"statements\":["
	        for(j=1;j<=stmt_count[i];j++){
	          if(j>1)printf ","
	          print_statement(i,j)
	        }
        printf "]},\"span\":%s}", span_json(task_line[i],1)
      }
      printf "\n  ]\n}\n"
    }
  ' "${files[@]}")"
  ast_program_report_from_json "$raw_ast"
}

node_json() {
  local target="$1" node="$2" report_json report_schema
  report_json="$(ast_json "$target" | jq \
    --arg node "$node" \
    --arg schema "$SCHEMA_AST_NODE" \
    --arg diag_schema "$SCHEMA_DIAGNOSTICS" \
    --arg task_kind "$PARSER_AST_TASK_NODE_KIND" \
    --arg statement_kind "$PARSER_AST_STATEMENT_NODE_KIND" \
    --arg expression_kind "$PARSER_AST_EXPRESSION_NODE_KIND" \
    --arg task_id_template "$PARSER_TASK_ID_TEMPLATE" \
    --arg not_found_id "$PARSER_AST_NODE_NOT_FOUND_ID" \
    --arg not_found_message_template "$PARSER_AST_NODE_NOT_FOUND_MESSAGE_TEMPLATE" '
    def replace_all($needle; $replacement): split($needle) | join($replacement);
    def ast_task_id($module_name; $task_name):
      $task_id_template
      | replace_all("{{module}}"; $module_name)
      | replace_all("{{task}}"; $task_name);
    def ast_not_found_message($node_id): $not_found_message_template | gsub("::node_id::"; $node_id);
    def statements: [.tasks[] as $t | $t.body.statements[]? | . + {module:$t.module, task:$t.name}];
    (first(.tasks[]? | select(.id == $node)) // null) as $task_match |
    (first(statements[] | select(.id == $node or (.expr.id? == $node))) // null) as $stmt_match |
    if $task_match != null then
      {schema:$schema, target:$node, id:$node, node_kind:$task_kind, module:$task_match.module, value:$task_match}
    elif $stmt_match != null and ($stmt_match.expr.id? == $node) then
      {schema:$schema, target:$node, id:$node, node_kind:$expression_kind, module:$stmt_match.module, parent:$stmt_match.id, value:$stmt_match.expr}
    elif $stmt_match != null then
      {schema:$schema, target:$node, id:$node, node_kind:$statement_kind, module:$stmt_match.module, parent:ast_task_id($stmt_match.module; $stmt_match.task), value:($stmt_match | del(.module,.task))}
    else
      {schema:$diag_schema, status:"error", diagnostics:[{id:$not_found_id, severity:"error", message:ast_not_found_message($node), node:$node}]}
    end
  ')"
  report_schema="$(printf '%s\n' "$report_json" | jq -r '.schema? // ""')"
  if [[ "$report_schema" == "$SCHEMA_AST_NODE" ]]; then
    ast_node_report_from_json "$report_json"
  else
    diagnostics_report_from_json "$report_json"
  fi
}

type_fields_json() {
  local target="$1"
  local -a files=()
  mapfile -t files < <(collect_files "$target")
  if [[ "${#files[@]}" -eq 0 ]]; then
    printf '[]\n'
    return
  fi
  awk '
    function trim(s){gsub(/^[ \t\r\n]+|[ \t\r\n]+$/, "", s); return s}
    function esc(s){gsub(/\\/, "\\\\", s); gsub(/"/, "\\\"", s); gsub(/\t/, "\\t", s); return s}
    function emit_field(){
      if(count++ > 0) printf ","
      printf "{\"module\":\"%s\",\"type_name\":\"%s\",\"name\":\"%s\",\"type\":\"%s\"}", esc(module), esc(type_name), esc(field_name), esc(field_type)
    }
    BEGIN {printf "["}
    /^[ \t]*module[ \t]+/ {module=$2}
    /^[ \t]*(export[ \t]+)?type[ \t]+/ {
      line=trim($0)
      sub(/^export[ \t]+/,"",line)
      sub(/^type[ \t]+/,"",line)
      type_name=line
      sub(/[ \t={].*$/,"",type_name)
      in_type=($0 ~ /=[ \t]*\{/)
      next
    }
    in_type && /^[ \t]*slot[ \t]+/ {
      line=trim($0)
      sub(/^slot[ \t]+/,"",line)
      field_name=line
      sub(/[ \t]*:.*/,"",field_name)
      field_type=line
      sub(/^.*:[ \t]*/,"",field_type)
      field_type=trim(field_type)
      emit_field()
      next
    }
    in_type && /^[ \t]*\}/ {in_type=0; type_name=""}
    END {printf "]\n"}
  ' "${files[@]}"
}

query_json() {
  local target="$1" kind="${2:-all}" module_filter="${3:-}" exported_only="${4:-false}" type_fields report values_json
  type_fields="$(type_fields_json "$target")"
  report="$(ast_json "$target" | jq --arg kind "$kind" --arg module_filter "$module_filter" --argjson exported_only "$exported_only" --arg schema "$SCHEMA_QUERY" --arg diag_schema "$SCHEMA_DIAGNOSTICS" --arg call_expression_prefix "$PARSER_CALL_EXPRESSION_PREFIX" --argjson fields "$QUERY_REPORT_FIELDS_JSON" --argjson type_fields "$type_fields" '
    def call_callee:
      .source
      | if startswith($call_expression_prefix) then .[($call_expression_prefix | length):] else . end
      | sub("\\(.*$";"");
    def resolve($ast; $module; $callee):
      if ($callee | contains(".")) then
        ($callee | split(".")) as $parts |
        ($parts[0]) as $prefix |
        ([$ast.imports[]? | select(.owner_module == $module and .alias == $prefix) | .module][0]) as $imported |
        if $imported then ($imported + "." + ($parts[1:] | join("."))) else $callee end
      else $module + "." + $callee end;
    . as $ast |
    ($ast.tasks | map(.module) | unique) as $mods |
    (($module_filter != "") and (($mods | index($module_filter)) == null)) as $missing_module |
    if $missing_module then
      {schema:$diag_schema, status:"error", diagnostics:[{id:"QUERY_MODULE_FILTER_NOT_FOUND", severity:"error", message:("query module filter `" + $module_filter + "` matched no module"), node:$module_filter}]}
    else
    (if $module_filter == "" then $mods else [$module_filter] end) as $selected_modules |
    ([$ast.tasks[] as $t | $t.body.statements[]?.expr? | select((.source? // "") | startswith($call_expression_prefix)) |
      (call_callee) as $callee |
      {from:($t.module + "." + $t.name), from_module:$t.module, expr_id:.id, source:.source, callee:$callee, status:"resolved", target:resolve($ast; $t.module; $callee)}
    ]) as $all_calls |
    ([$ast.tasks[] as $task | select(($selected_modules | index($task.module)) != null) | select(($exported_only | not) or $task.exported) |
      $task |
      {
        id, name, module, qualified_name:(.module + "." + .name), exported,
        takes:(.takes | map({name, binding_kind, type:(.type.name // "Unit")})),
        return_type:(.return_type.name // "Unit"),
        effects,
        outbound_call_count:([.body.statements[]?.expr? | select((.source? // "") | startswith($call_expression_prefix))] | length),
        inbound_call_count:([$all_calls[] | select(.target == ($task.module + "." + $task.name))] | length)
      }]) as $task_reports |
    ([$ast.types[]? as $type | select(($selected_modules | index($type.module)) != null) | select(($exported_only | not) or $type.exported) |
      $type |
      {id,name,module,qualified_name:(.module + "." + .name),exported,value:(.value.name // "Unit"),fields:[$type_fields[] | select(.module == $type.module and .type_name == $type.name) | {name,type}]}
    ]) as $type_reports |
    ([$ast.effects[]? as $effect | select(($selected_modules | index($effect.module)) != null) | select(($exported_only | not) or $effect.exported) | $effect | {id,name,module,qualified_name:(.module + "." + .name),exported}]) as $effect_reports |
    ($all_calls | map(. as $call | select(($selected_modules | index($call.from_module)) != null))) as $call_reports |
      {
        ($fields[0] // "schema"):$schema,
        ($fields[1] // "kind"):$kind,
        ($fields[2] // "entry_module"):($ast.module // ($ast.tasks[0].module // "main")),
        ($fields[3] // "filters"):{module:(if $module_filter == "" then null else $module_filter end), exported_only:$exported_only},
        ($fields[4] // "modules"):($selected_modules | map(. as $m | {
          module:$m,
          imports:[$ast.imports[]? | select(.owner_module == $m) | {id, module} + (if .alias then {alias} else {} end)],
          types:[$ast.types[]? | select(.module == $m) | {name,id,exported}],
          effects:[$ast.effects[]? | select(.module == $m) | {name,id,exported}],
          tasks:[$ast.tasks[]? | select(.module == $m) | {name,id,exported}]
        })),
        ($fields[5] // "tasks"):(if $kind == "types" or $kind == "effects" or $kind == "calls" then [] else $task_reports end),
        ($fields[6] // "types"):(if $kind == "tasks" or $kind == "effects" or $kind == "calls" then [] else $type_reports end),
        ($fields[7] // "effects"):(if $kind == "tasks" or $kind == "types" or $kind == "calls" then [] else $effect_reports end),
        ($fields[8] // "calls"):(if $kind == "calls" or $kind == "all" then $call_reports else [] end)
      }
    end
  ')"
  if [[ "$(printf '%s\n' "$report" | jq -r '.schema? // ""')" == "$SCHEMA_DIAGNOSTICS" ]]; then
    printf '%s\n' "$report"
    return
  fi
  values_json="$(jq -n \
    --arg schema "$SCHEMA_QUERY" \
    --argjson report "$report" \
    '{
      reports:{query_schema:$schema},
      query:{
        kind:$report.kind,
        entry_module:$report.entry_module,
        filters:$report.filters,
        modules:$report.modules,
        tasks:$report.tasks,
        types:$report.types,
        effects:$report.effects,
        calls:$report.calls
      }
    }')"
  sley_report_builder_for_namespace_json "query" "$values_json"
}

check_json() {
  local target="$1" diagnostics diagnostic_count check_status builtin_types source_text values_json
  if [[ -f "$target" ]]; then
    source_text="$(<"$target")"
  else
    source_text=""
  fi
  builtin_types="$(eval_checker_builtin_types_json)"
  diagnostics="$(ast_json "$target" | jq \
    --arg unknown "$DIAG_UNKNOWN_IDENTIFIER" \
    --arg unsupported_raw_expression "$DIAG_UNSUPPORTED_RAW_EXPRESSION" \
    --arg control_flow_expression_boundary "$DIAG_CONTROL_FLOW_EXPRESSION_BOUNDARY" \
    --arg module_control_flow_not_allowed "$DIAG_MODULE_CONTROL_FLOW_NOT_ALLOWED" \
    --arg inline_task_parameter_placement "$DIAG_INLINE_TASK_PARAMETER_PLACEMENT" \
    --arg unknown_type "$DIAG_UNKNOWN_TYPE" \
    --arg unknown_task "$DIAG_UNKNOWN_TASK" \
    --arg call_arity "$DIAG_CALL_ARITY_MISMATCH" \
    --arg call_argument_type "$DIAG_CALL_ARGUMENT_TYPE_MISMATCH" \
    --arg type_mismatch "$DIAG_TYPE_MISMATCH" \
    --arg return_type_mismatch "$DIAG_RETURN_TYPE_MISMATCH" \
    --arg duplicate_take "$DIAG_DUPLICATE_TAKE" \
    --arg duplicate_map_key "$DIAG_DUPLICATE_MAP_KEY" \
    --arg duplicate_field "$DIAG_DUPLICATE_FIELD" \
    --arg duplicate_record_literal_field "$DIAG_DUPLICATE_RECORD_LITERAL_FIELD" \
    --arg record_field_missing "$DIAG_RECORD_FIELD_MISSING" \
    --arg record_field_unknown "$DIAG_RECORD_FIELD_UNKNOWN" \
    --arg record_field_type_mismatch "$DIAG_RECORD_FIELD_TYPE_MISMATCH" \
    --arg unknown_record_field "$DIAG_UNKNOWN_RECORD_FIELD" \
    --arg record_literal_non_record_type "$DIAG_RECORD_LITERAL_NON_RECORD_TYPE" \
    --arg list_element_type_mismatch "$DIAG_LIST_ELEMENT_TYPE_MISMATCH" \
    --arg index_not_int "$DIAG_INDEX_NOT_INT" \
    --arg index_key_type_mismatch "$DIAG_INDEX_KEY_TYPE_MISMATCH" \
    --arg map_key_type_mismatch "$DIAG_MAP_KEY_TYPE_MISMATCH" \
    --arg map_value_type_mismatch "$DIAG_MAP_VALUE_TYPE_MISMATCH" \
    --arg duplicate_effect "$DIAG_DUPLICATE_EFFECT" \
    --arg duplicate_type "$DIAG_DUPLICATE_TYPE" \
    --arg duplicate_task "$DIAG_DUPLICATE_TASK" \
    --arg unknown_effect "$DIAG_UNKNOWN_EFFECT" \
    --arg gate_take_type "$DIAG_GATE_TAKE_TYPE_MISMATCH" \
    --arg gate_effect "$DIAG_GATE_EFFECT_UNDECLARED" \
    --arg effect_unauthorized "$DIAG_EFFECT_UNAUTHORIZED" \
    --arg missing_return "$DIAG_MISSING_RETURN" \
    --arg question_requires_result "$DIAG_QUESTION_REQUIRES_RESULT" \
    --arg return_statement_kind "$PARSER_RETURN_STATEMENT_KIND" \
    --arg binding_statement_kind "$PARSER_BINDING_STATEMENT_KIND" \
    --arg gate_binding_kind "$PARSER_GATE_BINDING_KIND" \
    --arg identifier_expr_kind "$PARSER_IDENTIFIER_KIND" \
    --arg call_expression_prefix "$PARSER_CALL_EXPRESSION_PREFIX" \
    --arg unknown_prefix "$UNKNOWN_IDENTIFIER_MESSAGE_PREFIX" \
    --arg unknown_suffix "$UNKNOWN_IDENTIFIER_MESSAGE_SUFFIX" \
    --arg unknown_message_template "$UNKNOWN_IDENTIFIER_MESSAGE_TEMPLATE" \
    --arg unsupported_raw_expression_message_template "$UNSUPPORTED_RAW_EXPRESSION_MESSAGE_TEMPLATE" \
    --arg control_flow_expression_boundary_message_template "$CONTROL_FLOW_EXPRESSION_BOUNDARY_MESSAGE_TEMPLATE" \
    --arg module_control_flow_not_allowed_message_template "$MODULE_CONTROL_FLOW_NOT_ALLOWED_MESSAGE_TEMPLATE" \
    --arg inline_task_parameter_placement_message_template "$INLINE_TASK_PARAMETER_PLACEMENT_MESSAGE_TEMPLATE" \
    --arg unknown_type_prefix "$UNKNOWN_TYPE_MESSAGE_PREFIX" \
    --arg unknown_type_suffix "$UNKNOWN_TYPE_MESSAGE_SUFFIX" \
    --arg unknown_type_message_template "$UNKNOWN_TYPE_MESSAGE_TEMPLATE" \
    --arg unknown_task_prefix "$UNKNOWN_TASK_MESSAGE_PREFIX" \
    --arg unknown_task_suffix "$UNKNOWN_TASK_MESSAGE_SUFFIX" \
    --arg unknown_task_message_template "$UNKNOWN_TASK_MESSAGE_TEMPLATE" \
    --arg call_arity_prefix "$CALL_ARITY_MISMATCH_MESSAGE_PREFIX" \
    --arg call_arity_suffix "$CALL_ARITY_MISMATCH_MESSAGE_SUFFIX" \
    --arg call_arity_message_template "$CALL_ARITY_MISMATCH_MESSAGE_TEMPLATE" \
    --arg call_argument_type_prefix "$CALL_ARGUMENT_TYPE_MISMATCH_MESSAGE_PREFIX" \
    --arg call_argument_type_suffix "$CALL_ARGUMENT_TYPE_MISMATCH_MESSAGE_SUFFIX" \
    --arg call_argument_type_message_template "$CALL_ARGUMENT_TYPE_MISMATCH_MESSAGE_TEMPLATE" \
    --arg type_mismatch_prefix "$TYPE_MISMATCH_MESSAGE_PREFIX" \
    --arg type_mismatch_suffix "$TYPE_MISMATCH_MESSAGE_SUFFIX" \
    --arg type_mismatch_message_template "$TYPE_MISMATCH_MESSAGE_TEMPLATE" \
    --arg return_type_mismatch_prefix "$RETURN_TYPE_MISMATCH_MESSAGE_PREFIX" \
    --arg return_type_mismatch_suffix "$RETURN_TYPE_MISMATCH_MESSAGE_SUFFIX" \
    --arg return_type_mismatch_message_template "$RETURN_TYPE_MISMATCH_MESSAGE_TEMPLATE" \
    --arg duplicate_take_prefix "$DUPLICATE_TAKE_MESSAGE_PREFIX" \
    --arg duplicate_take_suffix "$DUPLICATE_TAKE_MESSAGE_SUFFIX" \
    --arg duplicate_take_message_template "$DUPLICATE_TAKE_MESSAGE_TEMPLATE" \
    --arg duplicate_map_key_prefix "$DUPLICATE_MAP_KEY_MESSAGE_PREFIX" \
    --arg duplicate_map_key_suffix "$DUPLICATE_MAP_KEY_MESSAGE_SUFFIX" \
    --arg duplicate_map_key_message_template "$DUPLICATE_MAP_KEY_MESSAGE_TEMPLATE" \
    --arg duplicate_field_prefix "$DUPLICATE_FIELD_MESSAGE_PREFIX" \
    --arg duplicate_field_suffix "$DUPLICATE_FIELD_MESSAGE_SUFFIX" \
    --arg duplicate_field_message_template "$DUPLICATE_FIELD_MESSAGE_TEMPLATE" \
    --arg duplicate_record_literal_field_prefix "$DUPLICATE_RECORD_LITERAL_FIELD_MESSAGE_PREFIX" \
    --arg duplicate_record_literal_field_suffix "$DUPLICATE_RECORD_LITERAL_FIELD_MESSAGE_SUFFIX" \
    --arg duplicate_record_literal_field_message_template "$DUPLICATE_RECORD_LITERAL_FIELD_MESSAGE_TEMPLATE" \
    --arg record_field_missing_prefix "$RECORD_FIELD_MISSING_MESSAGE_PREFIX" \
    --arg record_field_missing_suffix "$RECORD_FIELD_MISSING_MESSAGE_SUFFIX" \
    --arg record_field_missing_message_template "$RECORD_FIELD_MISSING_MESSAGE_TEMPLATE" \
    --arg record_field_unknown_prefix "$RECORD_FIELD_UNKNOWN_MESSAGE_PREFIX" \
    --arg record_field_unknown_suffix "$RECORD_FIELD_UNKNOWN_MESSAGE_SUFFIX" \
    --arg record_field_unknown_message_template "$RECORD_FIELD_UNKNOWN_MESSAGE_TEMPLATE" \
    --arg record_field_type_mismatch_prefix "$RECORD_FIELD_TYPE_MISMATCH_MESSAGE_PREFIX" \
    --arg record_field_type_mismatch_suffix "$RECORD_FIELD_TYPE_MISMATCH_MESSAGE_SUFFIX" \
    --arg record_field_type_mismatch_message_template "$RECORD_FIELD_TYPE_MISMATCH_MESSAGE_TEMPLATE" \
    --arg unknown_record_field_prefix "$UNKNOWN_RECORD_FIELD_MESSAGE_PREFIX" \
    --arg unknown_record_field_suffix "$UNKNOWN_RECORD_FIELD_MESSAGE_SUFFIX" \
    --arg unknown_record_field_message_template "$UNKNOWN_RECORD_FIELD_MESSAGE_TEMPLATE" \
    --arg record_literal_non_record_type_prefix "$RECORD_LITERAL_NON_RECORD_TYPE_MESSAGE_PREFIX" \
    --arg record_literal_non_record_type_suffix "$RECORD_LITERAL_NON_RECORD_TYPE_MESSAGE_SUFFIX" \
    --arg record_literal_non_record_type_message_template "$RECORD_LITERAL_NON_RECORD_TYPE_MESSAGE_TEMPLATE" \
    --arg list_element_type_mismatch_prefix "$LIST_ELEMENT_TYPE_MISMATCH_MESSAGE_PREFIX" \
    --arg list_element_type_mismatch_suffix "$LIST_ELEMENT_TYPE_MISMATCH_MESSAGE_SUFFIX" \
    --arg list_element_type_mismatch_message_template "$LIST_ELEMENT_TYPE_MISMATCH_MESSAGE_TEMPLATE" \
    --arg index_not_int_prefix "$INDEX_NOT_INT_MESSAGE_PREFIX" \
    --arg index_not_int_suffix "$INDEX_NOT_INT_MESSAGE_SUFFIX" \
    --arg index_not_int_message_template "$INDEX_NOT_INT_MESSAGE_TEMPLATE" \
    --arg index_key_type_mismatch_prefix "$INDEX_KEY_TYPE_MISMATCH_MESSAGE_PREFIX" \
    --arg index_key_type_mismatch_suffix "$INDEX_KEY_TYPE_MISMATCH_MESSAGE_SUFFIX" \
    --arg index_key_type_mismatch_message_template "$INDEX_KEY_TYPE_MISMATCH_MESSAGE_TEMPLATE" \
    --arg map_key_type_mismatch_prefix "$MAP_KEY_TYPE_MISMATCH_MESSAGE_PREFIX" \
    --arg map_key_type_mismatch_suffix "$MAP_KEY_TYPE_MISMATCH_MESSAGE_SUFFIX" \
    --arg map_key_type_mismatch_message_template "$MAP_KEY_TYPE_MISMATCH_MESSAGE_TEMPLATE" \
    --arg map_value_type_mismatch_prefix "$MAP_VALUE_TYPE_MISMATCH_MESSAGE_PREFIX" \
    --arg map_value_type_mismatch_suffix "$MAP_VALUE_TYPE_MISMATCH_MESSAGE_SUFFIX" \
    --arg map_value_type_mismatch_message_template "$MAP_VALUE_TYPE_MISMATCH_MESSAGE_TEMPLATE" \
    --arg duplicate_effect_prefix "$DUPLICATE_EFFECT_MESSAGE_PREFIX" \
    --arg duplicate_effect_suffix "$DUPLICATE_EFFECT_MESSAGE_SUFFIX" \
    --arg duplicate_effect_message_template "$DUPLICATE_EFFECT_MESSAGE_TEMPLATE" \
    --arg duplicate_type_prefix "$DUPLICATE_TYPE_MESSAGE_PREFIX" \
    --arg duplicate_type_suffix "$DUPLICATE_TYPE_MESSAGE_SUFFIX" \
    --arg duplicate_type_message_template "$DUPLICATE_TYPE_MESSAGE_TEMPLATE" \
    --arg duplicate_task_prefix "$DUPLICATE_TASK_MESSAGE_PREFIX" \
    --arg duplicate_task_suffix "$DUPLICATE_TASK_MESSAGE_SUFFIX" \
    --arg duplicate_task_message_template "$DUPLICATE_TASK_MESSAGE_TEMPLATE" \
    --arg unknown_effect_prefix "$UNKNOWN_EFFECT_MESSAGE_PREFIX" \
    --arg unknown_effect_suffix "$UNKNOWN_EFFECT_MESSAGE_SUFFIX" \
    --arg unknown_effect_message_template "$UNKNOWN_EFFECT_MESSAGE_TEMPLATE" \
    --arg gate_take_type_prefix "$GATE_TAKE_TYPE_MISMATCH_MESSAGE_PREFIX" \
    --arg gate_take_type_suffix "$GATE_TAKE_TYPE_MISMATCH_MESSAGE_SUFFIX" \
    --arg gate_take_type_message_template "$GATE_TAKE_TYPE_MISMATCH_MESSAGE_TEMPLATE" \
    --arg gate_effect_prefix "$GATE_EFFECT_UNDECLARED_MESSAGE_PREFIX" \
    --arg gate_effect_suffix "$GATE_EFFECT_UNDECLARED_MESSAGE_SUFFIX" \
    --arg gate_effect_message_template "$GATE_EFFECT_UNDECLARED_MESSAGE_TEMPLATE" \
    --arg effect_unauthorized_prefix "$EFFECT_UNAUTHORIZED_MESSAGE_PREFIX" \
    --arg effect_unauthorized_suffix "$EFFECT_UNAUTHORIZED_MESSAGE_SUFFIX" \
    --arg effect_unauthorized_message_template "$EFFECT_UNAUTHORIZED_MESSAGE_TEMPLATE" \
    --arg missing_return_prefix "$MISSING_RETURN_MESSAGE_PREFIX" \
    --arg missing_return_suffix "$MISSING_RETURN_MESSAGE_SUFFIX" \
    --arg missing_return_message_template "$MISSING_RETURN_MESSAGE_TEMPLATE" \
    --arg question_requires_result_prefix "$QUESTION_REQUIRES_RESULT_MESSAGE_PREFIX" \
    --arg question_requires_result_suffix "$QUESTION_REQUIRES_RESULT_MESSAGE_SUFFIX" \
    --arg question_requires_result_message_template "$QUESTION_REQUIRES_RESULT_MESSAGE_TEMPLATE" \
    --arg declare_or_import_task_hint_kind "$CHECKER_DECLARE_OR_IMPORT_TASK_HINT_KIND" \
    --arg inspect_return_type_hint_kind "$CHECKER_INSPECT_RETURN_TYPE_HINT_KIND" \
    --arg insert_return_hint_kind "$CHECKER_INSERT_RETURN_HINT_KIND" \
    --arg replace_task_body_hint_kind "$CHECKER_REPLACE_TASK_BODY_HINT_KIND" \
    --arg replace_expression_hint_kind "$CHECKER_REPLACE_EXPRESSION_HINT_KIND" \
    --arg rewrite_supported_expression_hint_kind "$CHECKER_REWRITE_SUPPORTED_EXPRESSION_HINT_KIND" \
    --arg rewrite_supported_expression_hint "$CHECKER_REWRITE_SUPPORTED_EXPRESSION_HINT" \
    --arg rewrite_conditional_hint_kind "$CHECKER_REWRITE_CONDITIONAL_HINT_KIND" \
    --arg rewrite_conditional_hint "$CHECKER_REWRITE_CONDITIONAL_HINT" \
    --arg move_control_flow_into_task_hint_kind "$CHECKER_MOVE_CONTROL_FLOW_INTO_TASK_HINT_KIND" \
    --arg move_control_flow_into_task_hint "$CHECKER_MOVE_CONTROL_FLOW_INTO_TASK_HINT" \
    --arg move_take_into_task_body_hint_kind "$CHECKER_MOVE_TAKE_INTO_TASK_BODY_HINT_KIND" \
    --arg move_take_into_task_body_hint "$CHECKER_MOVE_TAKE_INTO_TASK_BODY_HINT" \
    --arg raw_expr_kind "$PARSER_RAW_KIND" \
    --arg foreign_conditional_expression_pattern "$CHECKER_FOREIGN_CONDITIONAL_EXPRESSION_PATTERN" \
    --arg inline_task_parameter_pattern "$CHECKER_INLINE_TASK_PARAMETER_PATTERN" \
    --arg host_adapter_raw_recovery_pattern "$CHECKER_HOST_ADAPTER_RAW_RECOVERY_PATTERN" \
    --arg source_text "$source_text" \
    --arg checker_int_type "$CHECKER_INT_TYPE" \
    --arg checker_text_type "$CHECKER_TEXT_TYPE" \
    --arg checker_bool_type "$CHECKER_BOOL_TYPE" \
    --arg checker_unit_type "$CHECKER_UNIT_TYPE" \
    --arg checker_result_type "$CHECKER_RESULT_TYPE" \
    --arg checker_gate_type "$CHECKER_GATE_TYPE" \
    --arg checker_list_type "$CHECKER_LIST_TYPE" \
    --arg checker_map_type "$CHECKER_MAP_TYPE" \
    --argjson builtin_types "$builtin_types" \
    --argjson builtin_effects "$CHECKER_BUILTIN_EFFECTS_JSON" \
    --argjson effect_aliases "$CHECKER_EFFECT_ALIASES_JSON" \
    --argjson host_effect_needles "$CHECKER_HOST_EFFECT_NEEDLES_JSON" \
    --argjson module_control_flow_prefixes "$CHECKER_MODULE_CONTROL_FLOW_PREFIXES_JSON" \
    --argjson supported_raw_recovery_patterns "$CHECKER_SUPPORTED_RAW_RECOVERY_PATTERNS_JSON" \
    --slurpfile source_rows <(source_rows_json_for_target "$target") \
    --argjson identifier_inputs "$CHECKER_IDENTIFIER_INPUTS_JSON" \
    --argjson assume_qualified_task_calls_known "$CHECKER_ASSUME_QUALIFIED_TASK_CALLS_KNOWN" \
    --argjson diagnostic_executors "$CHECKER_DIAGNOSTIC_EXECUTORS_JSON" \
    --argjson diagnostic_pass_descriptors "$CHECKER_DIAGNOSTIC_PASS_DESCRIPTORS_JSON" '
    . as $root |
    ($source_rows[0] // []) as $source_rows |
    [$root.tasks[].name] as $all_task_names |
    [$root.tasks[] | .module + "." + .name] as $all_qualified_task_names |
    [$root.types[]?.name] as $declared_types |
    [$root.effects[]?.name] as $declared_effects |
    ($source_text | split("\n")) as $source_lines |
    def include_identifier_input($name):
      ($identifier_inputs | index($name)) != null;
    def base_type_name:
      (. // $checker_unit_type) | sub("<.*$"; "");
    def type_known($name):
      ($name | base_type_name) as $base |
      (($builtin_types | index($base)) != null) or (($declared_types | index($base)) != null);
    def effect_known($name):
      (($builtin_effects | index($name)) != null) or (($declared_effects | index($name)) != null);
    def table_left:
      split("|")[0] // "";
    def table_right:
      split("|")[1] // "";
    def effect_authorized($effects; $effect):
      (($effects | index($effect)) != null)
      or any($effect_aliases[]?; (table_left as $canonical | table_right as $alias | (($canonical == $effect) and (($effects | index($alias)) != null))));
    def source_line($line):
      if $line == null then "" else ($source_lines[$line - 1]) // "" end;
    def trim:
      gsub("^[[:space:]]+|[[:space:]]+$"; "");
    def expr_text:
      ((.source? // "") + " " + source_line(.span.line));
    def raw_recovery_supported($expr; $module):
      (($expr.source // "") | trim) as $source |
      any($supported_raw_recovery_patterns[]?; . as $pattern | $source | test($pattern))
      or (
        ($source | test($host_adapter_raw_recovery_pattern))
        and any(
          $source_rows[]? | select(.module == $module and .line == ($expr.span.line // -1)) | .text;
          . as $line | any($host_effect_needles[]?; table_right as $needle | $line | contains($needle))
        )
      );
    def required_host_effects:
      (expr_text) as $source |
      [$host_effect_needles[]? | (table_left as $effect | table_right as $needle | select($source | contains($needle)) | $effect)] | unique;
    def call_source_tail($source):
      if ($source | startswith($call_expression_prefix)) then $source[($call_expression_prefix | length):]
      else "" end;
    def call_callee:
      call_source_tail(.source // "") | sub("\\(.*$";"");
    def call_args:
      (call_source_tail(.source // "") | capture("^[^()]+\\((?<args>.*)\\)\\??$")? // {args:""}) as $parts |
      if (($parts.args | gsub("[[:space:]]"; "")) == "") then []
      else ($parts.args | split(",") | map(gsub("^[[:space:]]+|[[:space:]]+$"; ""))) end;
    def resolve_call($module; $callee):
      if ($callee | contains(".")) then
        ($callee | split(".")) as $parts |
        ($parts[0]) as $prefix |
        ([$root.imports[]? | select(.owner_module == $module and .alias == $prefix) | .module][0]) as $imported |
        if $imported then ($imported + "." + ($parts[1:] | join("."))) else $callee end
      else $module + "." + $callee end;
    def declared_call_task($module; $callee):
      (resolve_call($module; $callee)) as $resolved |
      $root.tasks[]? | select((.module + "." + .name) == $resolved or ((($callee | contains(".")) | not) and .name == $callee));
    def task_known($callee):
      if (($all_task_names | index($callee)) != null) then true
      elif (($all_qualified_task_names | index($callee)) != null) then true
      elif ($callee | contains(".")) then $assume_qualified_task_calls_known
      else false end;
    def literal_type:
      if test("^\".*\"$") then $checker_text_type
      elif test("^-?[0-9]+$") then $checker_int_type
      elif . == "true" or . == "false" then $checker_bool_type
      else "" end;
    def starter_expr($type_name):
      ($type_name // $checker_unit_type) as $name |
      ($name | base_type_name) as $base |
      if $base == $checker_int_type then "0"
      elif $base == $checker_text_type then "\"\""
      elif $base == $checker_bool_type then "false"
      elif $base == $checker_result_type then "Err(\"\")"
      else "()" end;
    def starter_return($type_name):
      "return " + starter_expr($type_name);
    def csv_items:
      if (trim == "") then [] else split(",") | map(trim | select(length > 0)) end;
    def render_arg($template; $name; $value):
      $template | split("::" + $name + "::") | join($value);
    def alias_target($type_name):
      ([
        range(0; ($source_lines | length)) as $i |
        ($source_lines[$i]) as $line |
        select($line | test("^[[:space:]]*(export[[:space:]]+)?type[[:space:]]+" + $type_name + "([[:space:]=]|$)")) |
        (($line | capture("=[[:space:]]*(?<target>.+)$")? // {target:null}) | .target | select(. != null) | trim | select(startswith("{") | not))
      ][0]) // $type_name;
    def normalize_type($type_name):
      ($type_name | trim) as $name |
      (alias_target($name)) as $target |
      ($target | trim) as $resolved |
      if ($resolved | test("^" + $checker_list_type + "<[^>]+>$")) then
        (($resolved | capture("^" + $checker_list_type + "<(?<element>[^>]+)>$")) | .element | trim) as $element |
        $checker_list_type + "<" + (alias_target($element) | trim) + ">"
      elif $resolved == $name then $name
      else
        (alias_target($resolved)) as $second |
        if $second == $resolved then $resolved else $second end
      end;
    def types_compatible($expected; $actual):
      (normalize_type($expected)) == (normalize_type($actual));
    def type_known_for_compare($name):
      (normalize_type($name)) as $normalized |
      type_known($name) and
      if ($normalized | test("^" + $checker_list_type + "<[^>]+>$")) then
        (($normalized | capture("^" + $checker_list_type + "<(?<element>[^>]+)>$")) | .element | trim | type_known(.))
      elif ($normalized | test("^" + $checker_map_type + "<[^,>]+,[^>]+>$")) then
        ($normalized | capture("^" + $checker_map_type + "<(?<key>[^,>]+),[[:space:]]*(?<value>[^>]+)>$")) as $map |
        (($map.key | trim | type_known(.)) and ($map.value | trim | type_known(.)))
      else type_known($normalized) end;
    def list_literal_type:
      (capture("^\\[(?<items>[^\\[\\]]*)\\]$")? // {items:null}) as $list |
      if $list.items == null then ""
      else
        ($list.items | csv_items | map(literal_type) | map(select(. != "")) | unique) as $types |
        if (($list.items | csv_items | length) > 0) and (($types | length) == 1) then $checker_list_type + "<" + $types[0] + ">" else "" end
      end;
    def indexed_list_literal_element_type:
      (capture("^\\[(?<items>[^\\[\\]]*)\\]\\[[^\\[\\]]+\\]$")? // {items:null}) as $list |
      if $list.items == null then ""
      else
        ($list.items | csv_items | map(literal_type) | map(select(. != "")) | unique) as $types |
        if (($list.items | csv_items | length) > 0) and (($types | length) == 1) then $types[0] else "" end
      end;
    def declared_binding_type($stmt):
      (source_line($stmt.span.line) | capture("^[[:space:]]*(bind|let|var|mut)[[:space:]]+[A-Za-z_][A-Za-z0-9_]*[[:space:]]*:[[:space:]]*(?<declared>[^=]+)=")? // {declared:null}) as $binding |
      if $binding.declared == null then null else ($binding.declared | trim) end;
    def binding_type($task; $name):
      ([$task.takes[]? | select(.name == $name) | .type.name][0]) as $take_type |
      if $take_type != null then $take_type
      else ([$task.body.statements[]? | select(.kind==$binding_statement_kind and .name == $name) | declared_binding_type(.) | select(. != null)][0]) // null end;
    def list_element_type($type_name):
      ((normalize_type($type_name)) | capture("^" + $checker_list_type + "<(?<element>[^>]+)>$")? // {element:null}) | .element | if . == null then null else trim end;
    def expression_static_type($task; $expr):
      ($expr.source? // "") as $source |
      ($source | literal_type) as $literal |
      if $literal != "" then $literal
      else
        ($source | indexed_list_literal_element_type) as $indexed_list_type |
        if $indexed_list_type != "" then $indexed_list_type
        else
          ($source | list_literal_type) as $list_type |
          if $list_type != "" then $list_type
          else
            (($source | capture("^(?<name>[A-Za-z_][A-Za-z0-9_]*)$")? // {name:null}) | .name) as $identifier |
            if $identifier != null then ((binding_type($task; $identifier)) // "")
            else
              (($source | capture("^(?<name>[A-Za-z_][A-Za-z0-9_]*)\\[[^\\[\\]]+\\]$")? // {name:null}) | .name) as $indexed |
              if $indexed != null then
                (binding_type($task; $indexed)) as $indexed_type |
                if $indexed_type == null then "" else (list_element_type($indexed_type) // "") end
              else "" end
            end
          end
        end
      end;
    def line_list_element_type:
      ((. | capture(":\\s*" + $checker_list_type + "<(?<element>[^>]+)>\\s*=")?) // {element:null}) | .element;
    def line_map_key_type:
      ((. | capture(":\\s*" + $checker_map_type + "<(?<key>[^,>]+),\\s*(?<value>[^>]+)>\\s*=")?) // {key:null}) | .key | if . == null then null else trim end;
    def line_map_value_type:
      ((. | capture(":\\s*" + $checker_map_type + "<(?<key>[^,>]+),\\s*(?<value>[^>]+)>\\s*=")?) // {value:null}) | .value | if . == null then null else trim end;
    def map_block_lines($start):
      if $start == null then []
      else
        reduce ($source_lines[$start:][]) as $line ({done:false, lines:[]};
          if .done then .
          elif ($line | test("^[[:space:]]*}")) then .done = true
          else .lines += [$line] end
        ) | .lines
      end;
    def map_entries($stmt):
      map_block_lines($stmt.span.line)
      | map(trim)
      | map(select(test(":")))
      | map((capture("^(?<key>[^:]+):[[:space:]]*(?<value>.*?)[,]?$")? // {key:"", value:""})
        | {key:(.key | trim), value:(.value | trim)});
    def unquote_key:
      gsub("^\"|\"$"; "");
    def record_type_fields:
      reduce range(0; ($source_lines | length)) as $i
        ({in_type:false, type_name:null, type_key:null, rows:[]};
          ($source_lines[$i]) as $line |
          if ($line | test("^[[:space:]]*(export[[:space:]]+)?type[[:space:]]+[A-Za-z_][A-Za-z0-9_]*[[:space:]]*=[[:space:]]*\\{")) then
            ($line | capture("^[[:space:]]*(export[[:space:]]+)?type[[:space:]]+(?<name>[A-Za-z_][A-Za-z0-9_]*)")) as $type_match |
            .in_type = true | .type_name = $type_match.name | .type_key = ($type_match.name + ":" + (($i + 1) | tostring))
          elif .in_type and ($line | test("^[[:space:]]*}")) then
            .in_type = false | .type_name = null | .type_key = null
          elif .in_type and ($line | test("^[[:space:]]*slot[[:space:]]+")) then
            ($line | capture("^[[:space:]]*slot[[:space:]]+(?<name>[A-Za-z_][A-Za-z0-9_]*)[[:space:]]*:[[:space:]]*(?<field_type>[A-Za-z_][A-Za-z0-9_]*(<[^>]+>)?)")?) as $field_match |
            if $field_match == null then .
            else .rows += [{type_name:.type_name, type_key:.type_key, name:$field_match.name, field_type:$field_match.field_type, line:($i + 1)}] end
          else . end
        ) | .rows;
    def record_fields_for($type_name):
      [record_type_fields[] | select(.type_name == $type_name)];
    def record_type_known($type_name):
      (record_fields_for($type_name) | length) > 0;
    def type_decl_kind($type_name):
      ([
        range(0; ($source_lines | length)) as $i |
        ($source_lines[$i]) as $line |
        select($line | test("^[[:space:]]*(export[[:space:]]+)?type[[:space:]]+" + $type_name + "([[:space:]=]|$)")) |
        if ($line | test("=[[:space:]]*\\{")) then "Record" else "Alias" end
      ][0]) // null;
    def record_literal_parts:
      ((.source? // "") | capture("^(?<type>[A-Za-z_][A-Za-z0-9_]*)[[:space:]]*\\{(?<body>[^}]*)\\}$")? // {type:null, body:null});
    def record_literal_type:
      (record_literal_parts) as $literal | $literal.type;
    def record_literal_entries:
      (record_literal_parts) as $literal |
      if $literal.body == null then []
      else
        $literal.body
        | split(",")
        | map(trim)
        | map(select(test(":")))
        | map((capture("^(?<key>[A-Za-z_][A-Za-z0-9_]*)[[:space:]]*:[[:space:]]*(?<value>.*)$")? // {key:"", value:""}) | select(.key != "") | .value = (.value | trim))
      end;
    def record_binding_type($task; $name):
      ([$task.takes[]? | select(.name == $name) | .type.name | select(record_type_known(.))][0]) as $take_type |
      if $take_type != null then $take_type
      else
        ([$task.body.statements[]? | select(.kind==$binding_statement_kind and .name == $name) | (.expr | record_literal_type) | select(. != null and record_type_known(.))][0]) // null
      end;
    def field_access:
      ((.source? // "") | capture("^(?<base>[A-Za-z_][A-Za-z0-9_]*)\\.(?<field>[A-Za-z_][A-Za-z0-9_]*)$")?) // {base:null, field:null};
    def binding_collection_kind($task; $name):
      ([$task.body.statements[]? | select(.kind==$binding_statement_kind and .name==$name) |
        (source_line(.span.line)) as $line |
        if ($line | test(":\\s*" + $checker_map_type + "<")) or ((.expr.source? // "") | test("^map[[:space:]]*\\{")) then $checker_map_type
        elif ($line | test(":\\s*" + $checker_list_type + "<")) or ((.expr.source? // "") | test("^\\[")) then $checker_list_type
        else empty end][0]) // null;
    def simple_index:
      ((.source? // "") | capture("^(?<name>[A-Za-z_][A-Za-z0-9_]*)\\[(?<index>[^\\[\\]]+)\\]$")?) // {name:null,index:null};
    def gate_effect_name:
      ((. | capture("^" + $checker_gate_type + "<(?<effect>[^>]+)>$")?) // {effect:null}) | .effect;
    ({
      unknown_identifier_id:$unknown,
      unsupported_raw_expression_id:$unsupported_raw_expression,
      control_flow_expression_boundary_id:$control_flow_expression_boundary,
      module_control_flow_not_allowed_id:$module_control_flow_not_allowed,
      inline_task_parameter_placement_id:$inline_task_parameter_placement,
      unknown_type_id:$unknown_type,
      unknown_task_id:$unknown_task,
      call_arity_mismatch_id:$call_arity,
      call_argument_type_mismatch_id:$call_argument_type,
      type_mismatch_id:$type_mismatch,
      return_type_mismatch_id:$return_type_mismatch,
      missing_return_id:$missing_return,
      unknown_effect_id:$unknown_effect,
      gate_take_type_mismatch_id:$gate_take_type,
      gate_effect_undeclared_id:$gate_effect,
      effect_unauthorized_id:$effect_unauthorized,
      question_requires_result_id:$question_requires_result,
      duplicate_effect_id:$duplicate_effect,
      duplicate_type_id:$duplicate_type,
      duplicate_task_id:$duplicate_task,
      duplicate_take_id:$duplicate_take,
      duplicate_map_key_id:$duplicate_map_key,
      duplicate_field_id:$duplicate_field,
      duplicate_record_literal_field_id:$duplicate_record_literal_field,
      record_field_missing_id:$record_field_missing,
      record_field_unknown_id:$record_field_unknown,
      record_field_type_mismatch_id:$record_field_type_mismatch,
      record_literal_non_record_type_id:$record_literal_non_record_type,
      unknown_record_field_id:$unknown_record_field,
      list_element_type_mismatch_id:$list_element_type_mismatch,
      index_not_int_id:$index_not_int,
      index_key_type_mismatch_id:$index_key_type_mismatch,
      map_key_type_mismatch_id:$map_key_type_mismatch,
      map_value_type_mismatch_id:$map_value_type_mismatch
    }) as $diagnostic_id_task_values |
    ({
      unknown_identifier_message:$unknown_message_template,
      unsupported_raw_expression_message:$unsupported_raw_expression_message_template,
      control_flow_expression_boundary_message:$control_flow_expression_boundary_message_template,
      module_control_flow_not_allowed_message:$module_control_flow_not_allowed_message_template,
      inline_task_parameter_placement_message:$inline_task_parameter_placement_message_template,
      unknown_type_message:$unknown_type_message_template,
      unknown_task_message:$unknown_task_message_template,
      call_arity_mismatch_message:$call_arity_message_template,
      call_argument_type_mismatch_message:$call_argument_type_message_template,
      type_mismatch_message:$type_mismatch_message_template,
      return_type_mismatch_message:$return_type_mismatch_message_template,
      missing_return_message:$missing_return_message_template,
      unknown_effect_message:$unknown_effect_message_template,
      gate_take_type_mismatch_message:$gate_take_type_message_template,
      gate_effect_undeclared_message:$gate_effect_message_template,
      effect_unauthorized_message:$effect_unauthorized_message_template,
      question_requires_result_message:$question_requires_result_message_template,
      duplicate_effect_message:$duplicate_effect_message_template,
      duplicate_type_message:$duplicate_type_message_template,
      duplicate_task_message:$duplicate_task_message_template,
      duplicate_take_message:$duplicate_take_message_template,
      duplicate_map_key_message:$duplicate_map_key_message_template,
      duplicate_field_message:$duplicate_field_message_template,
      duplicate_record_literal_field_message:$duplicate_record_literal_field_message_template,
      record_field_missing_message:$record_field_missing_message_template,
      record_field_unknown_message:$record_field_unknown_message_template,
      record_field_type_mismatch_message:$record_field_type_mismatch_message_template,
      record_literal_non_record_type_message:$record_literal_non_record_type_message_template,
      unknown_record_field_message:$unknown_record_field_message_template,
      list_element_type_mismatch_message:$list_element_type_mismatch_message_template,
      index_not_int_message:$index_not_int_message_template,
      index_key_type_mismatch_message:$index_key_type_mismatch_message_template,
      map_key_type_mismatch_message:$map_key_type_mismatch_message_template,
      map_value_type_mismatch_message:$map_value_type_mismatch_message_template
    }) as $message_template_task_values |
    def descriptor_diagnostic_id($pass):
      $diagnostic_id_task_values[($pass.diagnostic_id_task // "")] // "";
    def descriptor_message_template($pass):
      $message_template_task_values[($pass.message_task // "")] // "";
    ($diagnostic_executors | length) as $diagnostic_executor_count |
    def descriptor_executor_id($pass):
      ($pass.executor_id // -1) as $executor_id |
      if ($executor_id >= 0 and $executor_id < $diagnostic_executor_count and $diagnostic_executors[$executor_id] == ($pass.executor // "")) then $executor_id else -1 end;
    def identifier_resolution_diags($pass):
      if ($pass.subject // "") == "task_identifier" then
        $root.tasks[] as $t |
        ((
          (if include_identifier_input("take_names") then [$t.takes[].name] else [] end) +
          (if include_identifier_input("binding_names") then [$t.body.statements[]? | select(.kind==$binding_statement_kind) | .name] else [] end) +
          (if include_identifier_input("task_names") then $all_task_names else [] end)
        ) | unique) as $known |
        $t.body.statements[]? |
        . as $stmt |
        select(.expr.expr_kind? == $identifier_expr_kind) |
        select(($known | index($stmt.expr.name)) == null) |
        {id:descriptor_diagnostic_id($pass), severity:"error", message:render_arg(descriptor_message_template($pass); ($pass.message_arg // "identifier_name"); .expr.name), node:.expr.id, span:.expr.span}
      else empty end;
    def unknown_reference_diags($pass):
      if ($pass.subject // "") == "task_return_type" then
        $root.tasks[] as $t |
        ($t.return_type.name // $checker_unit_type) as $name |
        select(type_known($name) | not) |
        {id:descriptor_diagnostic_id($pass), severity:"error", message:render_arg(descriptor_message_template($pass); ($pass.message_arg // "type_name"); $name), node:$t.id, span:$t.span}
      elif ($pass.subject // "") == "take_type" then
        $root.tasks[] as $t |
        $t.takes[]? as $take |
        ($take.type.name // $checker_unit_type) as $name |
        select(type_known($name) | not) |
        {id:descriptor_diagnostic_id($pass), severity:"error", message:render_arg(descriptor_message_template($pass); ($pass.message_arg // "type_name"); $name), node:$take.id, span:$take.span}
      elif ($pass.subject // "") == "task_effect" then
        $root.tasks[] as $t |
        $t.effects[]? as $effect |
        select(effect_known($effect) | not) |
        {id:descriptor_diagnostic_id($pass), severity:"error", message:render_arg(descriptor_message_template($pass); ($pass.message_arg // "effect_name"); $effect), node:$t.id, span:$t.span}
      else empty end;
    def gate_reference_diags($pass):
      if ($pass.subject // "") == "gate_take_type" then
        $root.tasks[] as $t |
        $t.takes[]? as $take |
        select($take.binding_kind == $gate_binding_kind) |
        ($take.type.name // $checker_unit_type) as $name |
        select(($name | gate_effect_name) == null) |
        {id:descriptor_diagnostic_id($pass), severity:"error", message:render_arg(descriptor_message_template($pass); ($pass.message_arg // "actual_type"); $name), node:$take.id, span:$take.span}
      elif ($pass.subject // "") == "gate_effect_declared" then
        $root.tasks[] as $t |
        $t.takes[]? as $take |
        select($take.binding_kind == $gate_binding_kind) |
        (($take.type.name // $checker_unit_type) | gate_effect_name) as $effect |
        select($effect != null) |
        select(($t.effects | index($effect)) == null) |
        {id:descriptor_diagnostic_id($pass), severity:"error", message:render_arg(descriptor_message_template($pass); ($pass.message_arg // "effect_name"); $effect), node:$take.id, span:$take.span}
      else empty end;
    def duplicate_name_diags($pass):
      if ($pass.subject // "") == "module_effect_name" then
        ($root.effects | group_by(.module + "." + .name)[] | select(length > 1) | .[1:])[] as $effect |
        {id:descriptor_diagnostic_id($pass), severity:"error", message:render_arg(descriptor_message_template($pass); ($pass.message_arg // "effect_name"); $effect.name), node:$effect.id, span:$effect.span}
      elif ($pass.subject // "") == "module_type_name" then
        ($root.types | group_by(.module + "." + .name)[] | select(length > 1) | .[1:])[] as $type |
        {id:descriptor_diagnostic_id($pass), severity:"error", message:render_arg(descriptor_message_template($pass); ($pass.message_arg // "type_name"); $type.name), node:$type.id, span:$type.span}
      elif ($pass.subject // "") == "module_task_name" then
        ($root.tasks | group_by(.module + "." + .name)[] | select(length > 1) | .[1:])[] as $task |
        {id:descriptor_diagnostic_id($pass), severity:"error", message:render_arg(descriptor_message_template($pass); ($pass.message_arg // "task_name"); $task.name), node:$task.id, span:$task.span}
      elif ($pass.subject // "") == "task_take_name" then
        $root.tasks[] as $t |
        ($t.takes | group_by(.name)[] | select(length > 1) | .[1]) as $take |
        {id:descriptor_diagnostic_id($pass), severity:"error", message:render_arg(descriptor_message_template($pass); ($pass.message_arg // "take_name"); $take.name), node:$take.id, span:$take.span}
      elif ($pass.subject // "") == "map_entry_key" then
        $root.tasks[] as $t |
        $t.body.statements[]? |
        select(.kind==$binding_statement_kind) |
        . as $stmt |
        (map_entries($stmt) | group_by(.key)[] | select(length > 1) | .[1].key | unquote_key) as $key |
        {id:descriptor_diagnostic_id($pass), severity:"error", message:render_arg(descriptor_message_template($pass); ($pass.message_arg // "key_name"); $key), node:$stmt.expr.id, span:$stmt.expr.span}
      elif ($pass.subject // "") == "record_type_field_name" then
        (record_type_fields | group_by(.type_key + "." + .name)[] | select(length > 1) | .[1]) as $field |
        ([$root.types[]? | select(.name == $field.type_name)][0]) as $type_node |
        {id:descriptor_diagnostic_id($pass), severity:"error", message:render_arg(descriptor_message_template($pass); ($pass.message_arg // "field_name"); $field.name), node:($type_node.id // ("type:" + $field.type_name)), span:{line:$field.line,column:1}}
      elif ($pass.subject // "") == "record_literal_field_name" then
        $root.tasks[] as $t |
        $t.body.statements[]?.expr? as $expr |
        ($expr | record_literal_entries | group_by(.key)[] | select(length > 1) | .[1].key) as $key |
        {id:descriptor_diagnostic_id($pass), severity:"error", message:render_arg(descriptor_message_template($pass); ($pass.message_arg // "field_name"); $key), node:$expr.id, span:$expr.span}
      else empty end;
    def record_shape_diags($pass):
      if ($pass.subject // "") == "record_literal_missing_field" then
        $root.tasks[] as $t |
        $t.body.statements[]? |
        select(.kind==$binding_statement_kind) |
        . as $stmt |
        ($stmt.expr | record_literal_type) as $type_name |
        select($type_name != null and record_type_known($type_name)) |
        ($stmt.expr | record_literal_entries) as $entries |
        ($entries | map(.key)) as $actual_fields |
        (record_fields_for($type_name))[] as $field |
        select(($actual_fields | index($field.name)) == null) |
        {id:descriptor_diagnostic_id($pass), severity:"error", message:render_arg(descriptor_message_template($pass); ($pass.message_arg // "field_name"); $field.name), node:$stmt.expr.id, span:$stmt.expr.span}
      elif ($pass.subject // "") == "record_literal_unknown_field" then
        $root.tasks[] as $t |
        $t.body.statements[]? |
        select(.kind==$binding_statement_kind) |
        . as $stmt |
        ($stmt.expr | record_literal_type) as $type_name |
        select($type_name != null and record_type_known($type_name)) |
        (record_fields_for($type_name) | map(.name)) as $known_fields |
        ($stmt.expr | record_literal_entries)[] as $entry |
        select(($known_fields | index($entry.key)) == null) |
        {id:descriptor_diagnostic_id($pass), severity:"error", message:render_arg(descriptor_message_template($pass); ($pass.message_arg // "field_name"); $entry.key), node:$stmt.expr.id, span:$stmt.expr.span}
      elif ($pass.subject // "") == "record_literal_field_type" then
        $root.tasks[] as $t |
        $t.body.statements[]? |
        select(.kind==$binding_statement_kind) |
        . as $stmt |
        ($stmt.expr | record_literal_type) as $type_name |
        select($type_name != null and record_type_known($type_name)) |
        (record_fields_for($type_name)) as $fields |
        ($stmt.expr | record_literal_entries)[] as $entry |
        ([$fields[] | select(.name == $entry.key)][0]) as $field |
        select($field != null) |
        ($entry.value | literal_type) as $actual |
        ($field.field_type | base_type_name) as $expected |
        select($actual != "" and $expected != "" and $actual != $expected) |
        {id:descriptor_diagnostic_id($pass), severity:"error", message:render_arg(descriptor_message_template($pass); ($pass.message_arg // "field_name"); $entry.key), node:$stmt.expr.id, span:$stmt.expr.span}
      elif ($pass.subject // "") == "record_literal_non_record_type" then
        $root.tasks[] as $t |
        $t.body.statements[]? |
        select(.kind==$binding_statement_kind) |
        . as $stmt |
        ($stmt.expr | record_literal_type) as $type_name |
        select($type_name != null) |
        select(type_decl_kind($type_name) == "Alias") |
        {id:descriptor_diagnostic_id($pass), severity:"error", message:render_arg(descriptor_message_template($pass); ($pass.message_arg // "type_name"); $type_name), node:$stmt.expr.id, span:$stmt.expr.span}
      elif ($pass.subject // "") == "record_access_unknown_field" then
        $root.tasks[] as $t |
        $t.body.statements[]?.expr? as $expr |
        ($expr | field_access) as $access |
        select($access.base != null) |
        (record_binding_type($t; $access.base)) as $type_name |
        select($type_name != null) |
        (record_fields_for($type_name) | map(.name)) as $known_fields |
        select(($known_fields | index($access.field)) == null) |
        {id:descriptor_diagnostic_id($pass), severity:"error", message:render_arg(descriptor_message_template($pass); ($pass.message_arg // "field_name"); $access.field), node:$expr.id, span:$expr.span}
      else empty end;
    def call_shape_diags($pass):
      if ($pass.subject // "") == "unknown_task_call" then
        $root.tasks[] as $t |
        $t.body.statements[]?.expr? |
        select((.source? // "") | startswith($call_expression_prefix)) |
        . as $expr |
        (call_callee) as $callee |
        select(task_known($callee) | not) |
        ($expr | call_args) as $args |
        def arg_type($arg):
          ($arg | literal_type) as $literal |
          if $literal != "" then $literal
          else
            ([$t.takes[]? | select(.name == $arg) | .type.name][0])
            // ([$t.body.statements[]? | select(.kind==$binding_statement_kind and .name == $arg) | expression_static_type($t; .expr)][0])
            // $checker_unit_type
          end;
        {id:descriptor_diagnostic_id($pass), severity:"error", message:render_arg(descriptor_message_template($pass); ($pass.message_arg // "task_name"); $callee), node:$expr.id, span:$expr.span,
          repair_hints:[{
            kind:$declare_or_import_task_hint_kind,
            target:$expr.id,
            replacement:("task " + $callee + " -> " + $checker_unit_type + " {\n" + ($args | to_entries | map("  take arg" + (.key|tostring) + ": " + arg_type(.value)) | join("\n")) + (if ($args | length) > 0 then "\n" else "" end) + "}")
          }]
        }
      elif ($pass.subject // "") == "call_arity" then
        $root.tasks[] as $t |
        $t.body.statements[]?.expr? |
        select((.source? // "") | startswith($call_expression_prefix)) |
        . as $expr |
        (call_callee) as $callee |
        ([declared_call_task($t.module; $callee)][0]) as $target |
        select($target != null) |
        ($target.takes | map(select(.binding_kind != $gate_binding_kind))) as $explicit_takes |
        ($expr | call_args) as $args |
        select(($args | length) != ($explicit_takes | length)) |
        {id:descriptor_diagnostic_id($pass), severity:"error", message:render_arg(descriptor_message_template($pass); ($pass.message_arg // "task_name"); $callee), node:$expr.id, span:$expr.span}
      elif ($pass.subject // "") == "call_argument_type" then
        $root.tasks[] as $t |
        $t.body.statements[]?.expr? |
        select((.source? // "") | startswith($call_expression_prefix)) |
        . as $expr |
        (call_callee) as $callee |
        ([declared_call_task($t.module; $callee)][0]) as $target |
        select($target != null) |
        ($target.takes | map(select(.binding_kind != $gate_binding_kind))) as $explicit_takes |
        ($expr | call_args) as $args |
        select(($args | length) == ($explicit_takes | length)) |
        $args | to_entries[] as $arg |
        ($explicit_takes[$arg.key].type.name // $checker_unit_type) as $expected |
        ($arg.value | literal_type) as $actual |
        select($actual != "" and $expected != $actual) |
        {id:descriptor_diagnostic_id($pass), severity:"error", message:render_arg(descriptor_message_template($pass); ($pass.message_arg // "task_name"); $callee), node:$expr.id, span:$expr.span}
      else empty end;
    def type_return_diags($pass):
      if ($pass.subject // "") == "binding_type" then
        $root.tasks[] as $t |
        $t.body.statements[]? |
        select(.kind==$binding_statement_kind) |
        . as $stmt |
        (declared_binding_type($stmt)) as $expected |
        select($expected != null) |
        select(type_known_for_compare($expected)) |
        (expression_static_type($t; $stmt.expr)) as $actual |
        select(type_known_for_compare($actual)) |
        select($actual != "" and (types_compatible($expected; $actual) | not)) |
        {id:descriptor_diagnostic_id($pass), severity:"error", message:render_arg(descriptor_message_template($pass); ($pass.message_arg // "binding_name"); $stmt.name), node:$stmt.id, span:$stmt.span}
      elif ($pass.subject // "") == "return_type" then
        $root.tasks[] as $t |
        ($t.return_type.name // $checker_unit_type) as $expected |
        select(type_known_for_compare($expected)) |
        $t.body.statements[]? |
        select(.kind == $return_statement_kind) |
        . as $stmt |
        (expression_static_type($t; $stmt.expr)) as $actual |
        select(type_known_for_compare($actual)) |
        select($actual != "" and (types_compatible($expected; $actual) | not)) |
        (starter_expr(normalize_type($expected))) as $replacement_expr |
        {id:descriptor_diagnostic_id($pass), severity:"error", message:render_arg(descriptor_message_template($pass); ($pass.message_arg // "task_name"); ($t.module + "." + $t.name)), node:$stmt.expr.id, span:$stmt.expr.span,
          repair_hints:[
            {kind:$inspect_return_type_hint_kind, target:$t.id, replacement:$expected},
            {kind:$replace_task_body_hint_kind, target:$t.id, replacement:("return " + $replacement_expr)},
            {kind:$replace_expression_hint_kind, target:$stmt.expr.id, replacement:("{\"op\":\"ReplaceExpression\",\"payload\":{\"source\":\"" + $replacement_expr + "\"},\"target\":\"" + $stmt.expr.id + "\"}")}
          ]
        }
      elif ($pass.subject // "") == "missing_return" then
        $root.tasks[] as $t |
        ($t.return_type.name // $checker_unit_type) as $name |
        select($name != $checker_unit_type) |
        select(([$t.body.statements[]? | select(.kind == $return_statement_kind)] | length) == 0) |
        {id:descriptor_diagnostic_id($pass), severity:"error", message:render_arg(descriptor_message_template($pass); ($pass.message_arg // "task_name"); ($t.module + "." + $t.name)), node:$t.id, span:$t.span,
          repair_hints:[
            {kind:$insert_return_hint_kind, target:$t.id, replacement:starter_return($name)},
            {kind:$replace_task_body_hint_kind, target:$t.id, replacement:starter_return($name)}
          ]
        }
      else empty end;
    def collection_shape_diags($pass):
      if ($pass.subject // "") == "list_literal_element_type" then
        $root.tasks[] as $t |
        $t.body.statements[]? |
        select(.kind==$binding_statement_kind) |
        . as $stmt |
        (source_line($stmt.span.line) | line_list_element_type) as $expected |
        select($expected != null) |
        (($stmt.expr.source? // "") | capture("^\\[(?<items>.*)\\]$")? // {items:""}) as $list |
        ($list.items | csv_items)[] as $item |
        ($item | literal_type) as $actual |
        select($actual != "" and $actual != $expected) |
        {id:descriptor_diagnostic_id($pass), severity:"error", message:render_arg(descriptor_message_template($pass); ($pass.message_arg // "actual_type"); $actual), node:$stmt.expr.id, span:$stmt.expr.span}
      elif ($pass.subject // "") == "list_index_type" then
        $root.tasks[] as $t |
        $t.body.statements[]?.expr? |
        . as $expr |
        ($expr | simple_index) as $idx |
        select($idx.name != null) |
        (binding_collection_kind($t; $idx.name)) as $kind |
        select($kind == $checker_list_type) |
        ($idx.index | literal_type) as $actual |
        select($actual != "" and $actual != $checker_int_type) |
        {id:descriptor_diagnostic_id($pass), severity:"error", message:render_arg(descriptor_message_template($pass); ($pass.message_arg // "actual_type"); $actual), node:$expr.id, span:$expr.span}
      elif ($pass.subject // "") == "map_index_key_type" then
        $root.tasks[] as $t |
        $t.body.statements[]?.expr? |
        . as $expr |
        ($expr | simple_index) as $idx |
        select($idx.name != null) |
        (binding_collection_kind($t; $idx.name)) as $kind |
        select($kind == $checker_map_type) |
        ($idx.index | literal_type) as $actual |
        select($actual != "" and $actual != $checker_text_type) |
        {id:descriptor_diagnostic_id($pass), severity:"error", message:render_arg(descriptor_message_template($pass); ($pass.message_arg // "actual_type"); $actual), node:$expr.id, span:$expr.span}
      elif ($pass.subject // "") == "map_literal_key_type" then
        $root.tasks[] as $t |
        $t.body.statements[]? |
        select(.kind==$binding_statement_kind) |
        . as $stmt |
        (source_line($stmt.span.line) | line_map_key_type) as $expected |
        select($expected != null) |
        (map_entries($stmt))[] as $entry |
        ($entry.key | literal_type) as $actual |
        select($actual != "" and $actual != $expected) |
        {id:descriptor_diagnostic_id($pass), severity:"error", message:render_arg(descriptor_message_template($pass); ($pass.message_arg // "actual_type"); $actual), node:$stmt.expr.id, span:$stmt.expr.span}
      elif ($pass.subject // "") == "map_literal_value_type" then
        $root.tasks[] as $t |
        $t.body.statements[]? |
        select(.kind==$binding_statement_kind) |
        . as $stmt |
        (map_entries($stmt)) as $entries |
        select(($entries | length) > 0) |
        (source_line($stmt.span.line) | line_map_value_type) as $declared_expected |
        ((if $declared_expected != null then $declared_expected else ([$entries[].value | literal_type | select(. != "")][0] // null) end)) as $expected |
        select($expected != null) |
        $entries[] as $entry |
        ($entry.value | literal_type) as $actual |
        select($actual != "" and $actual != $expected) |
        {id:descriptor_diagnostic_id($pass), severity:"error", message:render_arg(descriptor_message_template($pass); ($pass.message_arg // "actual_type"); $actual), node:$stmt.expr.id, span:$stmt.expr.span}
      else empty end;
    def question_result_diags($pass):
      if ($pass.subject // "") == "fallible_question_result" then
        $root.tasks[] as $t |
        ($t.return_type.name // $checker_unit_type | base_type_name) as $return_base |
        select($return_base != $checker_result_type) |
        $t.body.statements[]?.expr? |
        select((.fallible? == true) or (.expr_kind? == "Try")) |
        {id:descriptor_diagnostic_id($pass), severity:"error", message:render_arg(descriptor_message_template($pass); ($pass.message_arg // "task_name"); ($t.module + "." + $t.name)), node:.id, span:.span}
      else empty end;
    def effect_authorization_diags($pass):
      if ($pass.subject // "") == "direct_host_effect" then
        $root.tasks[] as $t |
        $t.body.statements[]?.expr? |
        . as $expr |
        ($expr | required_host_effects)[] as $effect |
        select(effect_authorized($t.effects; $effect) | not) |
        {id:descriptor_diagnostic_id($pass), severity:"error", message:render_arg(descriptor_message_template($pass); ($pass.message_arg // "effect_name"); $effect), node:$expr.id, span:$expr.span}
      elif ($pass.subject // "") == "transitive_task_effect" then
        $root.tasks[] as $t |
        $t.body.statements[]?.expr? |
        select((.source? // "") | startswith($call_expression_prefix)) |
        . as $expr |
        (call_callee) as $callee |
        ([declared_call_task($t.module; $callee)][0]) as $target |
        select($target != null) |
        $target.effects[]? as $effect |
        select(effect_authorized($t.effects; $effect) | not) |
        {id:descriptor_diagnostic_id($pass), severity:"error", message:render_arg(descriptor_message_template($pass); ($pass.message_arg // "effect_name"); $effect), node:$expr.id, span:$expr.span}
      else empty end;
    def syntax_context_diags($pass):
      if ($pass.subject // "") == "foreign_conditional_expression" then
        $root.tasks[]? as $task
        | $task.body
        | ..
        | objects
        | select(.expr_kind? == $raw_expr_kind)
        | . as $expr
        | (($expr.source // "") | trim) as $source
        | select(raw_recovery_supported($expr; $task.module) | not)
        | select($source | test($foreign_conditional_expression_pattern))
        | {
            id:descriptor_diagnostic_id($pass),
            severity:"error",
            message:render_arg(descriptor_message_template($pass); ($pass.message_arg // "expression_text"); $source),
            node:$expr.id,
            span:$expr.span,
            repair_hints:[{kind:$rewrite_conditional_hint_kind, target:$expr.id, replacement:$rewrite_conditional_hint}]
          }
      elif ($pass.subject // "") == "executable_raw" then
        $root.tasks[]? as $task
        | $task.body
        | ..
        | objects
        | select(.expr_kind? == $raw_expr_kind)
        | . as $expr
        | (($expr.source // "") | trim) as $source
        | select(raw_recovery_supported($expr; $task.module) | not)
        | select(($source | test($foreign_conditional_expression_pattern)) | not)
        | {
            id:descriptor_diagnostic_id($pass),
            severity:"error",
            message:render_arg(descriptor_message_template($pass); ($pass.message_arg // "expression_text"); $source),
            node:$expr.id,
            span:$expr.span,
            repair_hints:[{kind:$rewrite_supported_expression_hint_kind, target:$expr.id, replacement:$rewrite_supported_expression_hint}]
          }
      elif ($pass.subject // "") == "module_control_flow" then
        $source_rows[]? as $row
        | (($row.text // "") | trim) as $source
        | select(any($module_control_flow_prefixes[]?; . as $prefix | ($source == $prefix) or ($source | startswith($prefix + " ")) or ($source | startswith($prefix + "{"))))
        | select(($row.in_task // false) | not)
        | ("module:" + $row.module) as $node
        | {
            id:descriptor_diagnostic_id($pass),
            severity:"error",
            message:render_arg(descriptor_message_template($pass); ($pass.message_arg // "statement_text"); $source),
            node:$node,
            span:{line:$row.line, column:1},
            repair_hints:[{kind:$move_control_flow_into_task_hint_kind, target:$node, replacement:$move_control_flow_into_task_hint}]
          }
      elif ($pass.subject // "") == "inline_task_parameter" then
        $root.tasks[]? as $task
        | select(($task.name // "") | test($inline_task_parameter_pattern))
        | {
            id:descriptor_diagnostic_id($pass),
            severity:"error",
            message:render_arg(descriptor_message_template($pass); ($pass.message_arg // "task_name"); $task.name),
            node:$task.id,
            span:$task.span,
            repair_hints:[{kind:$move_take_into_task_body_hint_kind, target:$task.id, replacement:$move_take_into_task_body_hint}]
          }
      else empty end;
    def diagnostic_pass($pass):
      (descriptor_executor_id($pass)) as $executor |
      if $executor == 0 then identifier_resolution_diags($pass)
      elif $executor == 1 then unknown_reference_diags($pass)
      elif $executor == 2 then gate_reference_diags($pass)
      elif $executor == 3 then effect_authorization_diags($pass)
      elif $executor == 4 then duplicate_name_diags($pass)
      elif $executor == 5 then record_shape_diags($pass)
      elif $executor == 6 then collection_shape_diags($pass)
      elif $executor == 7 then call_shape_diags($pass)
      elif $executor == 8 then type_return_diags($pass)
      elif $executor == 9 then question_result_diags($pass)
      elif $executor == 10 then syntax_context_diags($pass)
      else empty end;
    [$diagnostic_pass_descriptors[]? | diagnostic_pass(.)]
  ')"
  diagnostic_count="$(printf '%s\n' "$diagnostics" | jq 'length')"
  check_status="$(eval_checker_status_task "$diagnostic_count")"
  [[ -n "$check_status" ]]
  values_json="$(jq -n \
    --arg schema "$SCHEMA_DIAGNOSTICS" \
    --arg status "$check_status" \
    --argjson diagnostics "$diagnostics" \
    '{
      reports:{diagnostics_schema:$schema},
      diagnostics:{status:$status, diagnostics:$diagnostics}
    }')"
  sley_report_builder_for_namespace_json "diagnostics" "$values_json"
}

lint_json() {
  local target="$1" entry findings finding_count lint_status raw_sources_json report values_json
  entry="$(entry_module_for "$target")"
  findings="$(mktemp)"
  : > "$findings"
  while IFS= read -r file; do
    local module
    module="$(awk '/^[[:space:]]*module[[:space:]]+/ {print $2; found=1; exit} END {if(!found) print "main"}' "$file")"
    if ! grep -Eq '^[[:space:]]*module[[:space:]]+' "$file"; then
      printf '%s\n' '{"id":"MISSING_MODULE_DECLARATION","rule":"missing_module_declaration","severity":"warning","message":"source relies on the implicit `main` module; declare an explicit module for stable project graph ids","node":"program","module":"main","hint":"add `module app.name` at the top of the file before deployable or project code"}' >> "$findings"
    fi
    if grep -Eq '^[[:space:]]*for[[:space:]].*in[[:space:]]+\[\][[:space:]]*\{' "$file"; then
      local empty_for_node
      empty_for_node="$(parser_task_statement_surface_value "$module" main 1)"
      printf '%s\n' "{\"id\":\"$LINT_EMPTY_FOR\",\"rule\":\"$LINT_EMPTY_FOR_RULE\",\"severity\":\"warning\",\"message\":\"task \`$module.main\` $LINT_EMPTY_FOR_MESSAGE\",\"node\":\"$empty_for_node\",\"module\":\"$module\",\"hint\":\"$LINT_EMPTY_FOR_HINT\"}" >> "$findings"
    fi
    if awk '/^[[:space:]]*forge[[:space:]]*\{/ {getline; if ($0 ~ /^[[:space:]]*\}/) found=1} END{exit found?0:1}' "$file"; then
      local empty_forge_node
      empty_forge_node="$(parser_task_statement_surface_value "$module" main 1)"
      printf '%s\n' "{\"id\":\"$LINT_EMPTY_FORGE\",\"rule\":\"$LINT_EMPTY_FORGE_RULE\",\"severity\":\"warning\",\"message\":\"task \`$module.main\` $LINT_EMPTY_FORGE_MESSAGE\",\"node\":\"$empty_forge_node\",\"module\":\"$module\",\"hint\":\"$LINT_EMPTY_FORGE_HINT\"}" >> "$findings"
    fi
    awk -v module="$module" -v parser_task_id_template="$PARSER_TASK_ID_TEMPLATE" -v parser_task_statement_surface_id_template="$PARSER_TASK_STATEMENT_SURFACE_ID_TEMPLATE" -v if_id="$LINT_EMPTY_IF" -v if_rule="$LINT_EMPTY_IF_RULE" -v if_message="$LINT_EMPTY_IF_MESSAGE" -v if_hint="$LINT_EMPTY_IF_HINT" -v while_id="$LINT_EMPTY_WHILE" -v while_rule="$LINT_EMPTY_WHILE_RULE" -v while_message="$LINT_EMPTY_WHILE_MESSAGE" -v while_hint="$LINT_EMPTY_WHILE_HINT" '
      function trim(s){gsub(/^[ \t\r\n]+|[ \t\r\n]+$/, "", s); return s}
      function esc(s){gsub(/\\/, "\\\\", s); gsub(/"/, "\\\"", s); return s}
      function count_char(s, c, n, i){n=0; for(i=1;i<=length(s);i++){if(substr(s,i,1)==c)n++} return n}
      function parser_task_id(module_name, task_name, s){s=parser_task_id_template; gsub(/\{\{module\}\}/, module_name, s); gsub(/\{\{task\}\}/, task_name, s); return s}
      function parser_statement_node(module_name, task_name, stmt_index, s){s=parser_task_statement_surface_id_template; gsub(/\{\{task_id\}\}/, parser_task_id(module_name, task_name), s); gsub(/\{\{index\}\}/, stmt_index, s); return s}
      function finding(id, rule, message, node, hint) {
        printf "{\"id\":\"%s\",\"rule\":\"%s\",\"severity\":\"warning\",\"message\":\"task `%s.%s` %s\",\"node\":\"%s\",\"module\":\"%s\",\"hint\":\"%s\"}\n", esc(id), esc(rule), esc(module), esc(task), esc(message), esc(node), esc(module), esc(hint)
      }
      /^[ \t]*(export[ \t]+)?task[ \t]+/ {
        line=trim($0); sub(/^export[ \t]+/,"",line); sub(/^task[ \t]+/,"",line)
        task=line; sub(/[ \t]*->.*/,"",task)
        in_task=1; depth=count_char($0,"{")-count_char($0,"}"); stmt=0; pending_if=""; pending_if_else=""; pending_while=""; next
      }
      in_task {
        raw=$0; line=trim(raw)
        if(pending_while != "") {
          if(line ~ /^}[ \t]*$/) finding(while_id, while_rule, while_message, parser_statement_node(module, task, pending_while), while_hint)
          if(line != "") pending_while=""
        }
        if(pending_if_else != "") {
          if(line ~ /^}[ \t]*$/) finding(if_id, if_rule, if_message, parser_statement_node(module, task, pending_if_else), if_hint)
          if(line != "") pending_if_else=""
        }
        if(pending_if != "") {
          if(line ~ /^}[ \t]*else[ \t]*\{[ \t]*$/) {
            pending_if_else=pending_if
          } else if(line ~ /^}[ \t]*$/) {
            finding(if_id, if_rule, if_message, parser_statement_node(module, task, pending_if), if_hint)
          }
          if(line != "") pending_if=""
        }
        if(depth == 1 && line ~ /^while[ \t].*\{[ \t]*$/) {
          pending_while=stmt
        } else if(depth == 1 && line ~ /^if[ \t].*\{[ \t]*$/) {
          pending_if=stmt
        }
        if(line != "" && line !~ /^take[ \t]+/ && line !~ /^}/ && depth == 1) {
          stmt++
        }
        depth += count_char(raw,"{")-count_char(raw,"}")
        if(depth<=0) in_task=0
      }
    ' "$file" >> "$findings"
    awk -v module="$module" -v parser_task_id_template="$PARSER_TASK_ID_TEMPLATE" -v parser_task_statement_surface_id_template="$PARSER_TASK_STATEMENT_SURFACE_ID_TEMPLATE" -v id="$LINT_EMPTY_ELSE" -v rule="$LINT_EMPTY_ELSE_RULE" -v message="$LINT_EMPTY_ELSE_MESSAGE" -v hint="$LINT_EMPTY_ELSE_HINT" '
      function trim(s){gsub(/^[ \t\r\n]+|[ \t\r\n]+$/, "", s); return s}
      function esc(s){gsub(/\\/, "\\\\", s); gsub(/"/, "\\\"", s); return s}
      function count_char(s, c, n, i){n=0; for(i=1;i<=length(s);i++){if(substr(s,i,1)==c)n++} return n}
      function parser_task_id(module_name, task_name, s){s=parser_task_id_template; gsub(/\{\{module\}\}/, module_name, s); gsub(/\{\{task\}\}/, task_name, s); return s}
      function parser_statement_node(module_name, task_name, stmt_index, s){s=parser_task_statement_surface_id_template; gsub(/\{\{task_id\}\}/, parser_task_id(module_name, task_name), s); gsub(/\{\{index\}\}/, stmt_index, s); return s}
      /^[ \t]*(export[ \t]+)?task[ \t]+/ {
        line=trim($0); sub(/^export[ \t]+/,"",line); sub(/^task[ \t]+/,"",line)
        task=line; sub(/[ \t]*->.*/,"",task)
        in_task=1; depth=count_char($0,"{")-count_char($0,"}"); stmt=0; if_stmt=""; if_then_has=0; next
      }
      in_task {
        raw=$0; line=trim(raw)
        if(pending_else != "" && line ~ /^[ \t]*}[ \t]*$/) {
          node=parser_statement_node(module, task, pending_else)
          printf "{\"id\":\"%s\",\"rule\":\"%s\",\"severity\":\"warning\",\"message\":\"task `%s.%s` %s\",\"node\":\"%s\",\"module\":\"%s\",\"hint\":\"%s\"}\n", esc(id), esc(rule), esc(module), esc(task), esc(message), esc(node), esc(module), esc(hint)
          pending_else=""
        } else if(pending_else != "" && line != "") {
          pending_else=""
        }
        if(depth == 1 && line ~ /^if[ \t].*\{[ \t]*$/) {
          if_stmt=stmt
          if_then_has=0
        }
        if(if_stmt != "" && depth == 2 && line != "" && line !~ /^}/ && line !~ /^else[ \t]*\{/) {
          if_then_has=1
        }
        if(line ~ /^}[ \t]*else[ \t]*\{[ \t]*$/ || line ~ /^else[ \t]*\{[ \t]*$/) {
          if(if_then_has) {
            pending_else=if_stmt
          } else {
            pending_else=""
          }
        }
        if(line != "" && line !~ /^take[ \t]+/ && line !~ /^}/ && depth == 1) {
          stmt++
        }
        depth += count_char(raw,"{")-count_char(raw,"}")
        if(depth<=0) in_task=0
      }
    ' "$file" >> "$findings"
    awk -v module="$module" -v parser_task_id_template="$PARSER_TASK_ID_TEMPLATE" -v parser_task_statement_surface_id_template="$PARSER_TASK_STATEMENT_SURFACE_ID_TEMPLATE" '
      function trim(s){gsub(/^[ \t\r\n]+|[ \t\r\n]+$/, "", s); return s}
      function esc(s){gsub(/\\/, "\\\\", s); gsub(/"/, "\\\"", s); return s}
      function count_char(s, c, n, i){n=0; for(i=1;i<=length(s);i++){if(substr(s,i,1)==c)n++} return n}
      function parser_task_id(module_name, task_name, s){s=parser_task_id_template; gsub(/\{\{module\}\}/, module_name, s); gsub(/\{\{task\}\}/, task_name, s); return s}
      function parser_statement_node(module_name, task_name, stmt_index, s){s=parser_task_statement_surface_id_template; gsub(/\{\{task_id\}\}/, parser_task_id(module_name, task_name), s); gsub(/\{\{index\}\}/, stmt_index, s); return s}
      function condition_from(line, cond){
        cond=trim(line)
        sub(/^if[ \t]+/,"",cond)
        sub(/[ \t]*\{[ \t]*$/,"",cond)
        return cond
      }
      function replacement_for_boolean_if(cond, then_expr, else_expr) {
        if(then_expr == "true" && else_expr == "false") return cond
        if(then_expr == "false" && else_expr == "true") return "!" cond
        return ""
      }
      function emit(rule_id, rule, node, message, hint, replacement, kind){
        printf "{\"id\":\"%s\",\"rule\":\"%s\",\"severity\":\"warning\",\"message\":\"%s\",\"node\":\"%s\",\"module\":\"%s\",\"hint\":\"%s\",\"replacement\":\"%s\",\"repair_kind\":\"%s\"}\n", rule_id, rule, esc(message), esc(node), esc(module), esc(hint), esc(replacement), esc(kind)
      }
      /^[ \t]*(export[ \t]+)?task[ \t]+/ {
        line=trim($0); sub(/^export[ \t]+/,"",line); sub(/^task[ \t]+/,"",line)
        task=line; sub(/[ \t]*->.*/,"",task)
        in_task=1; depth=count_char($0,"{")-count_char($0,"}"); stmt=0; if_stmt=""; if_cond=""; then_return=""; else_return=""; in_then=0; in_else=0; next
      }
      in_task {
        raw=$0; line=trim(raw)
        if(depth == 1 && line ~ /^if[ \t].*\{[ \t]*$/) {
          if_stmt=stmt
          if_cond=condition_from(line)
          then_return=""
          else_return=""
          in_then=1
          in_else=0
        } else if(if_stmt != "" && in_then && depth == 2 && line ~ /^return[ \t]+/) {
          then_return=line
          sub(/^return[ \t]+/,"",then_return)
          then_return=trim(then_return)
        } else if(if_stmt != "" && line ~ /^}[ \t]*else[ \t]*\{[ \t]*$/) {
          in_then=0
          in_else=1
        } else if(if_stmt != "" && in_else && depth == 2 && line ~ /^return[ \t]+/) {
          else_return=line
          sub(/^return[ \t]+/,"",else_return)
          else_return=trim(else_return)
        } else if(if_stmt != "" && in_else && line ~ /^}[ \t]*$/) {
          node=parser_statement_node(module, task, if_stmt)
          bool_replacement=replacement_for_boolean_if(if_cond, then_return, else_return)
          if(bool_replacement != "") {
            emit("REDUNDANT_BOOLEAN_IF_STATEMENT", "redundant_boolean_if_statement", node, "statement-level if returns Boolean literals from a Boolean condition", "replace the if statement with the condition or its negation", "return " bool_replacement, "simplify_redundant_boolean_if_statement")
          } else if(then_return != "" && then_return == else_return) {
            emit("SAME_BRANCH_IF_STATEMENT", "same_branch_if_statement", node, "if statement has identical return branches", "replace the if statement with the shared return", "return " then_return, "simplify_same_branch_if_statement")
          }
          if_stmt=""
          in_else=0
        }
        if(line != "" && line !~ /^take[ \t]+/ && line !~ /^}/ && depth == 1) {
          stmt++
        }
        depth += count_char(raw,"{")-count_char(raw,"}")
        if(depth<=0) in_task=0
      }
    ' "$file" >> "$findings"
    awk -v module="$module" -v parser_task_id_template="$PARSER_TASK_ID_TEMPLATE" -v parser_task_statement_surface_id_template="$PARSER_TASK_STATEMENT_SURFACE_ID_TEMPLATE" -v parser_expression_id_template="$PARSER_EXPRESSION_ID_TEMPLATE" '
      function trim(s){gsub(/^[ \t\r\n]+|[ \t\r\n]+$/, "", s); return s}
      function esc(s){gsub(/\\/, "\\\\", s); gsub(/"/, "\\\"", s); return s}
      function count_char(s, c, n, i){n=0; for(i=1;i<=length(s);i++){if(substr(s,i,1)==c)n++} return n}
      function parser_task_id(module_name, task_name, s){s=parser_task_id_template; gsub(/\{\{module\}\}/, module_name, s); gsub(/\{\{task\}\}/, task_name, s); return s}
      function parser_statement_node(module_name, task_name, stmt_index, s){s=parser_task_statement_surface_id_template; gsub(/\{\{task_id\}\}/, parser_task_id(module_name, task_name), s); gsub(/\{\{index\}\}/, stmt_index, s); return s}
      function parser_expression_node(module_name, task_name, stmt_index, s){s=parser_expression_id_template; gsub(/\{\{statement\}\}/, parser_statement_node(module_name, task_name, stmt_index), s); return s}
      /^[ \t]*(export[ \t]+)?task[ \t]+/ {
        line=trim($0); sub(/^export[ \t]+/,"",line); sub(/^task[ \t]+/,"",line)
        task=line; sub(/[ \t]*->.*/,"",task)
        in_task=1; depth=count_char($0,"{")-count_char($0,"}"); stmt=0; next
      }
      in_task {
        raw=$0; line=trim(raw)
        if(line == "" || line ~ /^take[ \t]+/) {
          depth += count_char(raw,"{")-count_char(raw,"}")
          if(depth<=0) in_task=0
          next
        }
        if(line !~ /^[ \t]*}/ && line ~ /try_[A-Za-z0-9_]*\(/ && line !~ /\?$/ && line !~ /^(bind|return)[ \t]+/) {
          node=parser_expression_node(module, task, stmt)
          replacement=line "?"
          printf "{\"id\":\"UNCHECKED_RESULT\",\"rule\":\"unchecked_result\",\"severity\":\"warning\",\"message\":\"task `%s.%s` discards Result in an expression statement\",\"node\":\"%s\",\"module\":\"%s\",\"hint\":\"use `?` to propagate failure, return the Result, or bind it for explicit handling\",\"replacement\":\"%s\"}\n", esc(module), esc(task), esc(node), esc(module), esc(replacement)
        }
        if(line !~ /^[ \t]*}/) stmt++
        depth += count_char(raw,"{")-count_char(raw,"}")
        if(depth<=0) in_task=0
      }
    ' "$file" >> "$findings"
  done < <(collect_files "$target")
  raw_sources_json="$(while IFS= read -r file; do jq -Rs . < "$file"; done < <(collect_files "$target") | jq -s .)"
  ast_json "$target" | jq -c '
    .imports
    | group_by(.owner_module + ":" + .module)[]
    | select(length > 1)
    | .[1:][]
    | {
      id:"DUPLICATE_IMPORT",
      rule:"duplicate_import",
      severity:"warning",
      message:("duplicate import `" + .module + "`"),
      node:.id,
      module:.owner_module,
      hint:"delete the duplicate import"
    }
  ' >> "$findings"
  ast_json "$target" | jq -c --argjson raw_sources "$raw_sources_json" '
    . as $root |
    def raw_type_used($type):
      any($raw_sources[] | split("\n")[]?;
        ((test("^[[:space:]]*(export[[:space:]]+)?type[[:space:]]+" + $type.name + "([[:space:]=]|$)") | not)
         and test("(^|[^A-Za-z0-9_])" + $type.name + "([^A-Za-z0-9_]|$)")));
    def type_used($type):
      any($root.tasks[]?; ((.return_type.name // "Unit") == $type.name) or any(.takes[]?; ((.type.name // "Unit") == $type.name))) or
      any($root.types[]?; ((.value.name // "Unit") == $type.name)) or
      raw_type_used($type);
    def effect_used($effect):
      any($root.tasks[]?; any(.effects[]?; . == $effect.name));
    ($root.effects[]? | select((.exported | not) and (effect_used(.) | not)) |
      {
        id:"UNUSED_PRIVATE_EFFECT",
        rule:"unused_private_effect",
        severity:"warning",
        message:("private effect `" + .module + "." + .name + "` is not used"),
        node:.id,
        module:.module,
        hint:"delete it or use it in a task"
      }),
    ($root.types[]? | select((.exported | not) and (type_used(.) | not)) |
      {
        id:"UNUSED_PRIVATE_TYPE",
        rule:"unused_private_type",
        severity:"warning",
        message:("private type `" + .module + "." + .name + "` is not used"),
        node:.id,
        module:.module,
        hint:"delete it or use it in a signature"
      })
  ' >> "$findings"
  ast_json "$target" | jq -c --arg call_expression_prefix "$PARSER_CALL_EXPRESSION_PREFIX" '
    . as $root |
    ([.. | objects | .source? // empty | select(type == "string")] | unique) as $sources |
    def call_source_tail($source):
      if ($source | startswith($call_expression_prefix)) then $source[($call_expression_prefix | length):]
      else "" end;
    def import_prefix($import):
      ($import.alias // ($import.module | split(".")[-1]));
    def import_task_names($import):
      [$root.tasks[]? | select(.module == $import.module and (.exported // false)) | .name];
    def import_used($import):
      (import_prefix($import)) as $prefix |
      (import_task_names($import)) as $task_names |
      any($sources[]; test("(^|[^A-Za-z0-9_])" + $prefix + "\\.")) or
      any($sources[]; . as $source |
        (call_source_tail($source) | capture("^(?<callee>[A-Za-z_][A-Za-z0-9_]*)\\(")? // null) as $call |
        ($call != null and (($task_names | index($call.callee)) != null))
      );
    .imports[]? as $import |
    select(import_used($import) | not) |
    {
      id:"UNUSED_IMPORT",
      rule:"unused_import",
      severity:"warning",
      message:("import `" + $import.module + "` is not used"),
      node:$import.id,
      module:$import.owner_module,
      hint:"delete the import or use it"
    }
  ' >> "$findings"
  ast_json "$target" | jq -c --argjson raw_sources "$raw_sources_json" --argjson host_effect_needles "$CHECKER_HOST_EFFECT_NEEDLES_JSON" --argjson effect_aliases "$CHECKER_EFFECT_ALIASES_JSON" '
    . as $root |
    (([.. | objects | .source? // empty | select(type == "string")] + $raw_sources) | unique) as $sources |
    def table_left:
      split("|")[0] // "";
    def table_right:
      split("|")[1] // "";
    def canonical_effect($effect):
      ([$effect_aliases[]? | select(table_right == $effect) | table_left][0] // $effect);
    def effect_used($effect):
      (canonical_effect($effect)) as $canonical |
      if any($host_effect_needles[]?; table_left == $canonical) then
        any($host_effect_needles[]?; (table_left == $canonical) and (table_right as $needle | any($sources[]; contains($needle))))
      else true end;
    $root.tasks[]? as $task |
    $task.effects | to_entries[] |
    select(effect_used(.value) | not) |
    {
      id:"UNUSED_DECLARED_EFFECT",
      rule:"unused_declared_effect",
      severity:"warning",
      message:("declared effect `" + .value + "` is not required by the task body"),
      node:("effect-use:" + $task.id + ":" + (.key|tostring) + ":" + .value),
      module:$task.module,
      hint:"remove the effect from the task signature"
    }
  ' >> "$findings"
  ast_json "$target" | jq -c --arg take_surface_id_template "$PARSER_TAKE_SURFACE_ID_TEMPLATE" '
    . as $root |
    def replace_all($needle; $replacement): split($needle) | join($replacement);
    def take_surface_id($task_id; $index; $take_name):
      $take_surface_id_template
      | replace_all("{{task_id}}"; $task_id)
      | replace_all("{{index}}"; $index)
      | replace_all("{{take}}"; $take_name);
    def source_uses($task; $name):
      any([$task.body.statements[]? | .. | objects | .source? // empty | select(type == "string")][]; test("(^|[^A-Za-z0-9_])" + $name + "([^A-Za-z0-9_]|$)"));
    $root.tasks[]? as $task |
    $task.takes | to_entries[] |
    select(source_uses($task; .value.name) | not) |
    {
      id:"UNUSED_TAKE",
      rule:"unused_take",
      severity:"warning",
      message:("take `" + .value.name + "` is not used"),
      node:take_surface_id($task.id; (.key|tostring); .value.name),
      module:$task.module,
      hint:"remove the take or use it in the task body"
    }
  ' >> "$findings"
  ast_json "$target" | jq -c --arg call_expression_prefix "$PARSER_CALL_EXPRESSION_PREFIX" '
    . as $root |
    def call_source_tail($source):
      if ($source | startswith($call_expression_prefix)) then $source[($call_expression_prefix | length):]
      else "" end;
    def task_key($t): ($t.module + "." + $t.name);
    def call_callee($expr):
      (call_source_tail($expr.source // "") | sub("\\(.*$";""));
    def call_target($module; $callee):
      if ($callee | contains(".")) then $callee else ($module + "." + $callee) end;
    def called_targets($task):
      [$task.body.statements[]?.expr? | select((.source? // "") | startswith($call_expression_prefix)) | call_target($task.module; call_callee(.))];
    def step($seen):
      reduce ($root.tasks[]? as $candidate | select(($seen | index(task_key($candidate))) != null) | called_targets($candidate)[]) as $call
        ($seen; if (. | index($call)) == null then . + [$call] else . end);
    def closure($seed):
      reduce range(0; (($root.tasks | length) + 1)) as $i ($seed; step(.));
    def inbound_count($target):
      [$root.tasks[]? | called_targets(.)[] | select(. == $target)] | length;
    ([$root.tasks[]? | select(.name == "main") | task_key(.)]) as $roots |
    (closure($roots)) as $reachable |
    $root.tasks[]? as $task |
    (task_key($task)) as $key |
    select(($task.exported | not) and $task.name != "main" and (($reachable | index($key)) == null) and (inbound_count($key) > 0)) |
    {
      id:"UNREACHABLE_PRIVATE_TASK",
      rule:"unreachable_private_task",
      severity:"warning",
      message:("private task `" + $key + "` is unreachable from main"),
      node:$task.id,
      module:$task.module,
      hint:"delete it or call it from a reachable task"
    }
  ' >> "$findings"
  ast_json "$target" | jq -c '
    ([.. | objects | .source? // empty | select(type == "string") |
      capture("^(call[[:space:]]+)?(?<callee>[A-Za-z_][A-Za-z0-9_.]*)\\(")? |
      .callee |
      split(".")[-1]] | unique) as $called |
    .tasks[]? as $task |
    select(($task.exported | not) and $task.name != "main" and (($called | index($task.name)) == null)) |
    {
      id:"UNUSED_PRIVATE_TASK",
      rule:"unused_private_task",
      severity:"warning",
      message:("private task `" + $task.module + "." + $task.name + "` is not called by any checked task"),
      node:$task.id,
      module:$task.module,
      hint:"call it, export it, or delete it"
    }
  ' >> "$findings"
  ast_json "$target" | jq -c '
    .tasks[]? as $task |
    $task.body.statements[]?.expr? as $expr |
    ($expr.source // "") as $source |
    select($source | test("fs\\.read_text\\(")) |
    {
      id:"RAW_HOST_ADAPTER",
      rule:"raw_host_adapter",
      severity:"warning",
      message:"host call `fs.read_text` uses a legacy diagnostic-failing adapter; prefer fallible `fs.try_read_text`",
      node:$expr.id,
      module:$task.module,
      hint:"replace `fs.read_text` with `fs.try_read_text` and handle the Result with `?` inside a Result-returning task",
      replacement:(($source | sub("fs\\.read_text"; "fs.try_read_text")) + "?")
    }
  ' >> "$findings"
  ast_json "$target" | jq -c --arg call_expression_prefix "$PARSER_CALL_EXPRESSION_PREFIX" '
    . as $root |
    def call_source_tail($source):
      if ($source | startswith($call_expression_prefix)) then $source[($call_expression_prefix | length):]
      else "" end;
    def call_callee:
      call_source_tail(.) | sub("\\(.*$";"");
    def resolve($module; $callee):
      if ($callee | contains(".")) then
        ($callee | split(".")) as $parts |
        ($parts[0]) as $prefix |
        ([$root.imports[]? | select(.owner_module == $module and ((.alias? // "") == $prefix)) | .module][0]) as $imported |
        if $imported then ($imported + "." + ($parts[1:] | join("."))) else $callee end
      else $module + "." + $callee end;
    def call_returns_result($module; $source):
      if ($source | startswith($call_expression_prefix)) then
        ($source | call_callee) as $callee |
        (resolve($module; $callee)) as $target |
        any($root.tasks[]?; ((.module + "." + .name) == $target and ((.return_type.name // "") | startswith("Result"))))
      else false end;
    def result_source($task; $expr):
      (($expr.source // "") | test("try_[A-Za-z0-9_]+\\(")) or call_returns_result($task.module; ($expr.source // ""));
    def source_uses_except($task; $stmt_id; $name):
      any([$task.body.statements[]? | select(.id != $stmt_id) | .. | objects | .source? // empty | select(type == "string")][]; test("(^|[^A-Za-z0-9_])" + $name + "([^A-Za-z0-9_]|$)"));
    $root.tasks[]? as $task |
    $task.body.statements[]? as $stmt |
    ($stmt.expr? // null) as $expr |
    select(($expr | type) == "object") |
    ($expr.source // "") as $source |
    if ($stmt.kind == "ExprStatement" and result_source($task; $expr) and (($source | endswith("?")) | not)) then
      {
        id:"UNCHECKED_RESULT",
        rule:"unchecked_result",
        severity:"warning",
        message:("task `" + $task.module + "." + $task.name + "` discards Result in an expression statement"),
        node:$expr.id,
        module:$task.module,
        hint:"use `?` to propagate failure, return the Result, or bind it for explicit handling",
        replacement:($source + "?")
      }
    elif ($stmt.kind == "Binding" and result_source($task; $expr) and (($source | endswith("?")) | not) and (source_uses_except($task; $stmt.id; $stmt.name) | not)) then
      {
        id:"UNCHECKED_RESULT_BINDING",
        rule:"unchecked_result_binding",
        severity:"warning",
        message:("task `" + $task.module + "." + $task.name + "` binds Result from `" + $source + "` to `" + $stmt.name + "` but never reads it"),
        node:$stmt.id,
        module:$task.module,
        hint:"use `?` to propagate failure, return the Result, or read the binding for explicit handling",
        replacement:($source + "?")
      }
    else empty end
  ' >> "$findings"
  ast_json "$target" | jq -c '
    . as $root |
    def source_uses_except($task; $stmt_id; $name):
      any([$task.body.statements[]? | select(.id != $stmt_id) | .. | objects | .source? // empty | select(type == "string")][]; test("(^|[^A-Za-z0-9_])" + $name + "([^A-Za-z0-9_]|$)"));
    $root.tasks[]? as $task |
    $task.body.statements[]? as $stmt |
    ($stmt.expr? // null) as $expr |
    select($stmt.kind == "Binding" and ($stmt.binding_kind // "") == "Bind" and (($expr | type) == "object")) |
    ($expr.source // "") as $source |
    select(($source | endswith("?")) and (source_uses_except($task; $stmt.id; $stmt.name) | not)) |
    {
      id:"UNUSED_EFFECTFUL_BINDING",
      rule:"unused_effectful_binding",
      severity:"warning",
      message:("binding `" + $stmt.name + "` is not read after the effectful expression is evaluated"),
      node:$stmt.id,
      module:$task.module,
      hint:"drop the binding name or read it later in the task",
      replacement:$source
    }
  ' >> "$findings"
  while IFS= read -r file; do
    local module
    module="$(awk '/^[[:space:]]*module[[:space:]]+/ {print $2; found=1; exit} END {if(!found) print "main"}' "$file")"
    awk -v module="$module" -v parser_task_id_template="$PARSER_TASK_ID_TEMPLATE" -v parser_task_statement_surface_id_template="$PARSER_TASK_STATEMENT_SURFACE_ID_TEMPLATE" '
      function trim(s){gsub(/^[ \t\r\n]+|[ \t\r\n]+$/, "", s); return s}
      function esc(s){gsub(/\\/, "\\\\", s); gsub(/"/, "\\\"", s); return s}
      function count_char(s, c, n, i){n=0; for(i=1;i<=length(s);i++){if(substr(s,i,1)==c)n++} return n}
      function parser_task_id(module_name, task_name, s){s=parser_task_id_template; gsub(/\{\{module\}\}/, module_name, s); gsub(/\{\{task\}\}/, task_name, s); return s}
      function parser_statement_node(module_name, task_name, stmt_index, s){s=parser_task_statement_surface_id_template; gsub(/\{\{task_id\}\}/, parser_task_id(module_name, task_name), s); gsub(/\{\{index\}\}/, stmt_index, s); return s}
      /^[ \t]*(export[ \t]+)?task[ \t]+/ {
        line=trim($0); sub(/^export[ \t]+/,"",line); sub(/^task[ \t]+/,"",line)
        task=line; sub(/[ \t]*->.*/,"",task)
        in_task=1; depth=count_char($0,"{")-count_char($0,"}"); stmt=0; terminated=0; next
      }
      in_task {
        raw=$0; line=trim(raw)
        if(line == "" || line ~ /^take[ \t]+/) {
          depth += count_char(raw,"{")-count_char(raw,"}")
          if(depth<=0) in_task=0
          next
        }
        if(line !~ /^[ \t]*}/ && depth == 1) {
          if(terminated) {
            node=parser_statement_node(module, task, stmt)
            printf "{\"id\":\"UNREACHABLE_STATEMENT\",\"rule\":\"unreachable_statement\",\"severity\":\"warning\",\"message\":\"statement after return in task `%s.%s` is unreachable\",\"node\":\"%s\",\"module\":\"%s\",\"hint\":\"delete the unreachable statement\"}\n", esc(module), esc(task), esc(node), esc(module)
          }
          if(line ~ /^return([ \t]|$)/) terminated=1
          stmt++
        }
        depth += count_char(raw,"{")-count_char(raw,"}")
        if(depth<=0) in_task=0
      }
    ' "$file" >> "$findings"
  done < <(collect_files "$target")
  ast_json "$target" | jq -c --arg call_expression_prefix "$PARSER_CALL_EXPRESSION_PREFIX" '
    . as $root |
    def call_source_tail($source):
      if ($source | startswith($call_expression_prefix)) then $source[($call_expression_prefix | length):]
      else "" end;
    def import_prefix($import):
      ($import.alias // ($import.module | split(".")[-1]));
    def call_callee($source):
      ((call_source_tail($source) | capture("^(?<callee>[A-Za-z_][A-Za-z0-9_]*)\\(")? // {callee:""}).callee);
    $root.tasks[]? as $task |
    $task.body.statements[]?.expr? as $expr |
    ($expr.source // "") as $source |
    select($source | startswith($call_expression_prefix)) |
    (call_callee($source)) as $callee |
    select($callee != "") |
    $root.imports[]? as $import |
    select($import.owner_module == $task.module) |
    select(any($root.tasks[]?; .module == $import.module and .name == $callee and (.exported // false))) |
    (import_prefix($import)) as $prefix |
    {
      id:"UNQUALIFIED_IMPORTED_CALL",
      rule:"unqualified_imported_call",
      severity:"warning",
      message:("call `" + $callee + "` resolves to an imported task but is not qualified"),
      node:$expr.id,
      module:$task.module,
      hint:"qualify the call through the import alias",
      replacement:($call_expression_prefix + $prefix + "." + call_source_tail($source))
    }
  ' >> "$findings"
  ast_json "$target" | jq -c --arg call_expression_prefix "$PARSER_CALL_EXPRESSION_PREFIX" '
    . as $root |
    def call_source_tail($source):
      if ($source | startswith($call_expression_prefix)) then $source[($call_expression_prefix | length):]
      else "" end;
    def call_callee($source):
      ((call_source_tail($source) | capture("^(?<callee>[A-Za-z_][A-Za-z0-9_.]*)\\(")? // {callee:""}).callee);
    def resolve($module; $callee):
      if ($callee | contains(".")) then
        ($callee | split(".")) as $parts |
        ($parts[0]) as $prefix |
        ([$root.imports[]? | select(.owner_module == $module and ((.alias? // "") == $prefix)) | .module][0]) as $imported |
        if $imported then ($imported + "." + ($parts[1:] | join("."))) else $callee end
      else $module + "." + $callee end;
    def call_returns_result($module; $source):
      if ($source | startswith($call_expression_prefix)) then
        (call_callee($source)) as $callee |
        (resolve($module; $callee)) as $target |
        any($root.tasks[]?; ((.module + "." + .name) == $target and ((.return_type.name // "") | startswith("Result"))))
      else false end;
    def result_source($task; $expr):
      (($expr.source // "") | test("try_[A-Za-z0-9_]+\\(")) or call_returns_result($task.module; ($expr.source // ""));
    def source_uses_except($task; $stmt_id; $name):
      any([$task.body.statements[]? | select(.id != $stmt_id) | .. | objects | .source? // empty | select(type == "string")][]; test("(^|[^A-Za-z0-9_])" + $name + "([^A-Za-z0-9_]|$)"));
    $root.tasks[]? as $task |
    $task.body.statements[]? as $stmt |
    ($stmt.expr? // null) as $expr |
    select($stmt.kind == "Binding" and ($stmt.binding_kind // "") == "Bind" and $expr != null) |
    ($expr.source // "") as $source |
    select((source_uses_except($task; $stmt.id; $stmt.name) | not) and (($source | endswith("?")) | not) and (result_source($task; $expr) | not)) |
    {
      id:"UNUSED_PURE_BINDING",
      rule:"unused_pure_binding",
      severity:"warning",
      message:("pure binding `" + $stmt.name + "` is not used"),
      node:$stmt.id,
      module:$task.module,
      hint:"delete the binding or use it later in the task"
    }
  ' >> "$findings"
  while IFS= read -r file; do
    local module
    module="$(awk '/^[[:space:]]*module[[:space:]]+/ {print $2; found=1; exit} END {if(!found) print "main"}' "$file")"
    awk -v module="$module" -v parser_task_id_template="$PARSER_TASK_ID_TEMPLATE" -v parser_task_statement_surface_id_template="$PARSER_TASK_STATEMENT_SURFACE_ID_TEMPLATE" '
      function trim(s){gsub(/^[ \t\r\n]+|[ \t\r\n]+$/, "", s); return s}
      function esc(s){gsub(/\\/, "\\\\", s); gsub(/"/, "\\\"", s); return s}
      function count_char(s, c, n, i){n=0; for(i=1;i<=length(s);i++){if(substr(s,i,1)==c)n++} return n}
      function parser_task_id(module_name, task_name, s){s=parser_task_id_template; gsub(/\{\{module\}\}/, module_name, s); gsub(/\{\{task\}\}/, task_name, s); return s}
      function parser_statement_node(module_name, task_name, stmt_index, s){s=parser_task_statement_surface_id_template; gsub(/\{\{task_id\}\}/, parser_task_id(module_name, task_name), s); gsub(/\{\{index\}\}/, stmt_index, s); return s}
      /^[ \t]*(export[ \t]+)?task[ \t]+/ {
        line=trim($0); sub(/^export[ \t]+/,"",line); sub(/^task[ \t]+/,"",line)
        task=line; sub(/[ \t]*->.*/,"",task)
        in_task=1; depth=count_char($0,"{")-count_char($0,"}"); stmt=0; next
      }
      in_task {
        raw=$0; line=trim(raw)
        if(line == "" || line ~ /^take[ \t]+/) {
          depth += count_char(raw,"{")-count_char(raw,"}")
          if(depth<=0) in_task=0
          next
        }
        if(line !~ /^[ \t]*}/ && depth == 1 && line !~ /^(bind|state|tally|set|return|call|for|while|if|forge)([ \t{]|$)/ && line !~ /\?$/) {
          node=parser_statement_node(module, task, stmt)
          printf "{\"id\":\"UNUSED_PURE_EXPRESSION_STATEMENT\",\"rule\":\"unused_pure_expression_statement\",\"severity\":\"warning\",\"message\":\"pure expression statement has no effect\",\"node\":\"%s\",\"module\":\"%s\",\"hint\":\"delete the expression or bind/return its value\"}\n", esc(node), esc(module)
        }
        if(line !~ /^[ \t]*}/) stmt++
        depth += count_char(raw,"{")-count_char(raw,"}")
        if(depth<=0) in_task=0
      }
    ' "$file" >> "$findings"
  done < <(collect_files "$target")
  ast_json "$target" | jq -c '
    . as $root |
    def set_targets($task):
      [$task.body.statements[]? | select(.kind == "Set") | .name];
    $root.tasks[]? as $task |
    (set_targets($task)) as $sets |
    $task.body.statements[]? as $stmt |
    ($stmt.expr? // null) as $expr |
    select($stmt.kind == "Binding" and ($stmt.binding_kind // "") == "State" and $expr != null) |
    select(($sets | index($stmt.name)) == null) |
    {
      id:"MUTABLE_BINDING_NEVER_SET",
      rule:"mutable_binding_never_set",
      severity:"warning",
      message:("mutable binding `" + $stmt.name + "` is never assigned after initialization"),
      node:$stmt.id,
      module:$task.module,
      hint:"use an immutable bind when the value is never changed",
      replacement:("bind " + $stmt.name + " = " + ($expr.source // "0"))
    }
  ' >> "$findings"
  ast_json "$target" | jq -c '
    .tasks[]? as $task |
    $task.body.statements[]? as $stmt |
    select($stmt.kind == "Set") |
    select(($stmt.expr.source // "") == $stmt.name) |
    {
      id:"SELF_ASSIGNMENT_STATEMENT",
      rule:"self_assignment_statement",
      severity:"warning",
      message:("set `" + $stmt.name + "` assigns the variable to itself"),
      node:$stmt.id,
      module:$task.module,
      hint:"delete the self-assignment"
    }
  ' >> "$findings"
  ast_json "$target" | jq -c '
    def name_used($source; $name):
      ($source // "") | test("(^|[^A-Za-z0-9_])" + $name + "([^A-Za-z0-9_]|$)");
    def overwritten_before_read($tail; $name):
      reduce $tail[] as $stmt ({done:false, hit:false};
        if .done then .
        elif ($stmt.kind == "Set" and $stmt.name == $name and ((name_used($stmt.expr.source; $name)) | not)) then {done:true, hit:true}
        elif name_used(($stmt.expr.source? // ""); $name) then {done:true, hit:false}
        else . end
      ) | .hit;
    .tasks[]? as $task |
    ($task.body.statements // []) as $stmts |
    $stmts | to_entries[] as $entry |
    ($entry.value) as $stmt |
    select($stmt.kind == "Set") |
    select(($stmt.expr.source // "") != $stmt.name) |
    select(overwritten_before_read($stmts[($entry.key + 1):]; $stmt.name)) |
    {
      id:"OVERWRITTEN_SET_STATEMENT",
      rule:"overwritten_set_statement",
      severity:"warning",
      message:("set `" + $stmt.name + "` is overwritten before it is read"),
      node:$stmt.id,
      module:$task.module,
      hint:"delete the overwritten assignment"
    }
  ' >> "$findings"
  ast_json "$target" | jq -c '
    def name_used($source; $name):
      ($source // "") | test("(^|[^A-Za-z0-9_])" + $name + "([^A-Za-z0-9_]|$)");
    def has_later_set($tail; $name):
      any($tail[]?; .kind == "Set" and .name == $name);
    .tasks[]? as $task |
    ($task.body.statements // []) as $stmts |
    $stmts | to_entries[] as $entry |
    select($entry.key > 0) |
    ($stmts[$entry.key - 1]) as $prev |
    ($entry.value) as $stmt |
    select($prev.kind == "Binding" and ($prev.binding_kind // "") == "State") |
    select(($prev.expr.source // "") | test("^-?[0-9]+$")) |
    select($stmt.kind == "Set" and $stmt.name == $prev.name) |
    select((name_used(($stmt.expr.source // ""); $stmt.name)) | not) |
    ($stmts[($entry.key + 1):]) as $tail |
    {
      id:"REDUNDANT_INITIAL_SET_STATEMENT",
      rule:"redundant_initial_set_statement",
      severity:"warning",
      message:("initial value for `" + $stmt.name + "` is overwritten before it is read"),
      node:$stmt.id,
      module:$task.module,
      hint:"fold the set value into the initializer",
      binding_node:$prev.id,
      binding_expr_node:$prev.expr.id,
      replacement:($stmt.expr.source // "0"),
      replacement_statement:((if has_later_set($tail; $stmt.name) then "state " else "bind " end) + $stmt.name + " = " + ($stmt.expr.source // "0")),
      repair_kind:(if has_later_set($tail; $stmt.name) then "fold_redundant_initial_set_into_binding" else "convert_redundant_initial_set_to_bind" end)
    }
  ' >> "$findings"
  ast_json "$target" | jq -c '
    .tasks[]? as $task |
    $task.body.statements[]?.expr? as $expr |
    ($expr.source // "") as $source |
    (($source | capture("^if[[:space:]]+(?<cond>true|false)[[:space:]]*\\{[[:space:]]*(?<then>[^}]*)[[:space:]]*\\}[[:space:]]*else[[:space:]]*\\{[[:space:]]*(?<else>[^}]*)[[:space:]]*\\}$")? // null)) as $m |
    select($m != null) |
    {
      id:"CONSTANT_IF_EXPRESSION",
      rule:"constant_if_expression",
      severity:"warning",
      message:"if expression has a constant condition",
      node:$expr.id,
      module:$task.module,
      hint:"replace the if expression with the selected branch",
      replacement:((if $m.cond == "true" then $m.then else $m.else end) | gsub("^[[:space:]]+|[[:space:]]+$"; ""))
    }
  ' >> "$findings"
  ast_json "$target" | jq -c '
    def comparison_result($left; $op; $right):
      if $op == "<" then $left < $right
      elif $op == "<=" then $left <= $right
      elif $op == ">" then $left > $right
      elif $op == ">=" then $left >= $right
      elif $op == "==" then $left == $right
      elif $op == "!=" then $left != $right
      else false end;
    .tasks[]? as $task |
    $task.body.statements[]?.expr? as $expr |
    ($expr.source // "") as $source |
    (($source | capture("^(?<left>-?[0-9]+)[[:space:]]*(?<op>==|!=|<=|>=|<|>)[[:space:]]*(?<right>-?[0-9]+)$")? // null)) as $m |
    select($m != null) |
    {
      id:"CONSTANT_COMPARISON_EXPRESSION",
      rule:"constant_comparison_expression",
      severity:"warning",
      message:"comparison expression is constant",
      node:$expr.id,
      module:$task.module,
      hint:"replace the comparison with its Boolean result",
      replacement:(comparison_result(($m.left | tonumber); $m.op; ($m.right | tonumber)) | tostring)
    }
  ' >> "$findings"
  ast_json "$target" | jq -c '
    def bool_value($source): $source == "true";
    def comparison_result($left; $op; $right):
      if $op == "==" then $left == $right
      elif $op == "!=" then $left != $right
      else false end;
    .tasks[]? as $task |
    $task.body.statements[]?.expr? as $expr |
    ($expr.source // "") as $source |
    (($source | capture("^(?<left>true|false)[[:space:]]*(?<op>==|!=)[[:space:]]*(?<right>true|false)$")? // null)) as $m |
    select($m != null) |
    {
      id:"CONSTANT_BOOLEAN_COMPARISON_EXPRESSION",
      rule:"constant_boolean_comparison_expression",
      severity:"warning",
      message:"Boolean comparison expression is constant",
      node:$expr.id,
      module:$task.module,
      hint:"replace the comparison with its Boolean result",
      replacement:(comparison_result(bool_value($m.left); $m.op; bool_value($m.right)) | tostring)
    }
  ' >> "$findings"
  ast_json "$target" | jq -c '
    def arithmetic_result($left; $op; $right):
      if $op == "+" then $left + $right
      elif $op == "-" then $left - $right
      elif $op == "*" then $left * $right
      else null end;
    .tasks[]? as $task |
    $task.body.statements[]?.expr? as $expr |
    ($expr.source // "") as $source |
    (($source | capture("^(?<left>-?[0-9]+)[[:space:]]*(?<op>\\+|-|\\*)[[:space:]]*(?<right>-?[0-9]+)$")? // null)) as $m |
    select($m != null) |
    (arithmetic_result(($m.left | tonumber); $m.op; ($m.right | tonumber))) as $result |
    select($result != null) |
    {
      id:"CONSTANT_ARITHMETIC_EXPRESSION",
      rule:"constant_arithmetic_expression",
      severity:"warning",
      message:"arithmetic expression is constant",
      node:$expr.id,
      module:$task.module,
      hint:"replace the arithmetic expression with its constant value",
      replacement:($result | tostring)
    }
  ' >> "$findings"
  ast_json "$target" | jq -c '
    .tasks[]? as $task |
    $task.body.statements[]?.expr? as $expr |
    ($expr.source // "") as $source |
    select($source | test("(^|[[:space:](])0[[:space:]]*\\*|\\*[[:space:]]*0([[:space:])]|$)")) |
    {
      id:"ABSORBING_ARITHMETIC_EXPRESSION",
      rule:"absorbing_arithmetic_expression",
      severity:"warning",
      message:"arithmetic expression is absorbed by zero",
      node:$expr.id,
      module:$task.module,
      hint:"replace the multiplication by zero with 0",
      replacement:"0"
    }
  ' >> "$findings"
  ast_json "$target" | jq -c '
    .tasks[]? as $task |
    $task.body.statements[]?.expr? as $expr |
    ($expr.source // "") as $source |
    (($source | capture("^\"(?<left>[^\"]*)\"[[:space:]]*\\+[[:space:]]*\"(?<right>[^\"]*)\"$")? // null)) as $m |
    select($m != null) |
    {
      id:"CONSTANT_TEXT_CONCATENATION_EXPRESSION",
      rule:"constant_text_concatenation_expression",
      severity:"warning",
      message:"text concatenation expression is constant",
      node:$expr.id,
      module:$task.module,
      hint:"replace the text concatenation with the combined literal",
      replacement:(($m.left + $m.right) | @json)
    }
  ' >> "$findings"
  ast_json "$target" | jq -c '
    def trim: gsub("^[[:space:]]+|[[:space:]]+$"; "");
    .tasks[]? as $task |
    $task.body.statements[]?.expr? as $expr |
    ($expr.source // "") as $source |
    (($source | capture("^\\[(?<items>[^\\]]*)\\]\\[(?<index>[0-9]+)\\]$")? // null)) as $m |
    select($m != null) |
    (($m.items | split(",") | map(trim))[$m.index | tonumber] // null) as $replacement |
    select($replacement != null and $replacement != "") |
    {
      id:"CONSTANT_LIST_INDEX_EXPRESSION",
      rule:"constant_list_index_expression",
      severity:"warning",
      message:"list index expression is constant",
      node:$expr.id,
      module:$task.module,
      hint:"replace the list index with the selected literal",
      replacement:$replacement
    }
  ' >> "$findings"
  ast_json "$target" | jq -c '
    def trim: gsub("^[[:space:]]+|[[:space:]]+$"; "");
    def map_pair:
      (trim | capture("^\"(?<key>[^\"]+)\"[[:space:]]*:[[:space:]]*(?<value>.+)$")? // null);
    .tasks[]? as $task |
    $task.body.statements[]?.expr? as $expr |
    ($expr.source // "") as $source |
    (($source | capture("^map[[:space:]]*\\{[[:space:]]*(?<pairs>.*)[[:space:]]*\\}\\[\"(?<key>[^\"]+)\"\\]$")? // null)) as $m |
    select($m != null) |
    ([$m.pairs | split(",")[] | map_pair | select(. != null and .key == $m.key) | .value][0] // null) as $replacement |
    select($replacement != null and $replacement != "") |
    {
      id:"CONSTANT_MAP_INDEX_EXPRESSION",
      rule:"constant_map_index_expression",
      severity:"warning",
      message:"map index expression is constant",
      node:$expr.id,
      module:$task.module,
      hint:"replace the map index with the selected literal",
      replacement:($replacement | trim)
    }
  ' >> "$findings"
  ast_json "$target" | jq -c '
    def trim: gsub("^[[:space:]]+|[[:space:]]+$"; "");
    def record_field_pair:
      (trim | capture("^(?<field>[A-Za-z_][A-Za-z0-9_]*)[[:space:]]*:[[:space:]]*(?<value>.+)$")? // null);
    .tasks[]? as $task |
    $task.body.statements[]?.expr? as $expr |
    ($expr.source // "") as $source |
    (($source | capture("^[A-Za-z_][A-Za-z0-9_]*[[:space:]]*\\{[[:space:]]*(?<fields>.*)[[:space:]]*\\}\\.(?<field>[A-Za-z_][A-Za-z0-9_]*)$")? // null)) as $m |
    select($m != null) |
    ([$m.fields | split(",")[] | record_field_pair | select(. != null and .field == $m.field) | .value][0] // null) as $replacement |
    select($replacement != null and $replacement != "") |
    {
      id:"CONSTANT_RECORD_FIELD_ACCESS_EXPRESSION",
      rule:"constant_record_field_access_expression",
      severity:"warning",
      message:"record field access expression is constant",
      node:$expr.id,
      module:$task.module,
      hint:"replace the record field access with the selected literal",
      replacement:($replacement | trim)
    }
  ' >> "$findings"
  ast_json "$target" | jq -c '
    def trim: gsub("^[[:space:]]+|[[:space:]]+$"; "");
    .tasks[]? as $task |
    $task.body.statements[]?.expr? as $expr |
    ($expr.source // "") as $source |
    (($source | capture("^len\\(\\[(?<items>.*)\\]\\)$")? // null)) as $m |
    select($m != null) |
    (if ($m.items | trim) == "" then 0 else ($m.items | split(",") | length) end) as $count |
    {
      id:"CONSTANT_LEN_EXPRESSION",
      rule:"constant_len_expression",
      severity:"warning",
      message:"len expression is constant",
      node:$expr.id,
      module:$task.module,
      hint:"replace len with the collection length",
      replacement:($count | tostring)
    }
  ' >> "$findings"
  ast_json "$target" | jq -c '
    .tasks[]? as $task |
    $task.body.statements[]?.expr? as $expr |
    ($expr.source // "") as $source |
    (($source | capture("^![[:space:]]*(?<value>true|false)$")? // null)) as $m |
    select($m != null) |
    {
      id:"CONSTANT_NOT_EXPRESSION",
      rule:"constant_not_expression",
      severity:"warning",
      message:"not expression is constant",
      node:$expr.id,
      module:$task.module,
      hint:"replace not with its Boolean result",
      replacement:(if $m.value == "true" then "false" else "true" end)
    }
  ' >> "$findings"
  ast_json "$target" | jq -c '
    .tasks[]? as $task |
    $task.body.statements[]?.expr? as $expr |
    ($expr.source // "") as $source |
    (($source | capture("^(?<left>[A-Za-z_][A-Za-z0-9_]*|\"\"|true|false)[[:space:]]*(?<op>\\+|\\*|&&|\\|\\|)[[:space:]]*(?<right>[A-Za-z_][A-Za-z0-9_]*|\"\"|true|false|0|1)$")? // null)) as $m |
    select($m != null) |
    (if ($m.op == "+" and $m.right == "0") then $m.left
     elif ($m.op == "+" and $m.left == "0") then $m.right
     elif ($m.op == "*" and $m.right == "1") then $m.left
     elif ($m.op == "*" and $m.left == "1") then $m.right
     elif ($m.op == "+" and $m.left == "\"\"") then $m.right
     elif ($m.op == "+" and $m.right == "\"\"") then $m.left
     elif ($m.op == "&&" and $m.right == "true") then $m.left
     elif ($m.op == "&&" and $m.left == "true") then $m.right
     elif ($m.op == "||" and $m.right == "false") then $m.left
     elif ($m.op == "||" and $m.left == "false") then $m.right
     else null end) as $replacement |
    select($replacement != null) |
    {
      id:"IDENTITY_BINARY_EXPRESSION",
      rule:"identity_binary_expression",
      severity:"warning",
      message:"binary expression has an identity operand",
      node:$expr.id,
      module:$task.module,
      hint:"replace the expression with the non-identity side",
      replacement:$replacement
    }
  ' >> "$findings"
  ast_json "$target" | jq -c '
    .tasks[]? as $task |
    $task.body.statements[]?.expr? as $expr |
    ($expr.source // "") as $source |
    (($source | capture("^(?<name>[A-Za-z_][A-Za-z0-9_]*)[[:space:]]*(?<op>==|!=)[[:space:]]*(?<value>true|false)$")? // null)) as $m |
    select($m != null) |
    (if (($m.op == "==" and $m.value == "true") or ($m.op == "!=" and $m.value == "false")) then $m.name else ("!" + $m.name) end) as $replacement |
    {
      id:"REDUNDANT_BOOLEAN_COMPARISON",
      rule:"redundant_boolean_comparison",
      severity:"warning",
      message:"Boolean comparison can be simplified",
      node:$expr.id,
      module:$task.module,
      hint:"replace the comparison with the Boolean expression",
      replacement:$replacement
    }
  ' >> "$findings"
  ast_json "$target" | jq -c '
    .tasks[]? as $task |
    $task.body.statements[]?.expr? as $expr |
    ($expr.source // "") as $source |
    (($source | capture("^(?<left>[A-Za-z_][A-Za-z0-9_]*|true|false)[[:space:]]*(?<op>&&|\\|\\|)[[:space:]]*(?<right>[A-Za-z_][A-Za-z0-9_]*|true|false)$")? // null)) as $m |
    select($m != null) |
    (if ($m.op == "&&" and ($m.left == "false" or $m.right == "false")) then "false"
     elif ($m.op == "||" and ($m.left == "true" or $m.right == "true")) then "true"
     else null end) as $replacement |
    select($replacement != null) |
    {
      id:"ABSORBING_BOOLEAN_EXPRESSION",
      rule:"absorbing_boolean_expression",
      severity:"warning",
      message:"Boolean expression is absorbed by a literal operand",
      node:$expr.id,
      module:$task.module,
      hint:"replace the expression with the absorbing Boolean literal",
      replacement:$replacement
    }
  ' >> "$findings"
  ast_json "$target" | jq -c '
    .tasks[]? as $task |
    $task.body.statements[]?.expr? as $expr |
    ($expr.source // "") as $source |
    (($source | capture("^(?<left>[A-Za-z_][A-Za-z0-9_]*)[[:space:]]*(?<op>&&|\\|\\|)[[:space:]]*(?<right>[A-Za-z_][A-Za-z0-9_]*)$")? // null)) as $m |
    select($m != null and $m.left == $m.right) |
    {
      id:"IDEMPOTENT_BOOLEAN_EXPRESSION",
      rule:"idempotent_boolean_expression",
      severity:"warning",
      message:"Boolean expression repeats the same operand",
      node:$expr.id,
      module:$task.module,
      hint:"replace the idempotent Boolean expression with one operand",
      replacement:$m.left
    }
  ' >> "$findings"
  ast_json "$target" | jq -c '
    def self_result($op):
      if $op == "==" or $op == "<=" or $op == ">=" then "true"
      else "false" end;
    .tasks[]? as $task |
    $task.body.statements[]?.expr? as $expr |
    ($expr.source // "") as $source |
    (($source | capture("^(?<left>[A-Za-z_][A-Za-z0-9_]*|-?[0-9]+)[[:space:]]*(?<op>==|!=|<=|>=|<|>)[[:space:]]*(?<right>[A-Za-z_][A-Za-z0-9_]*|-?[0-9]+)$")? // null)) as $m |
    select($m != null and $m.left == $m.right) |
    {
      id:"SELF_COMPARISON_EXPRESSION",
      rule:"self_comparison_expression",
      severity:"warning",
      message:"comparison checks a value against itself",
      node:$expr.id,
      module:$task.module,
      hint:"replace the self-comparison with its Boolean result",
      replacement:self_result($m.op)
    }
  ' >> "$findings"
  ast_json "$target" | jq -c '
    .tasks[]? as $task |
    $task.body.statements[]?.expr? as $expr |
    ($expr.source // "") as $source |
    (($source | capture("^!![[:space:]]*(?<inner>[A-Za-z_][A-Za-z0-9_]*)$")? // null)) as $m |
    select($m != null) |
    {
      id:"DOUBLE_NEGATION_EXPRESSION",
      rule:"double_negation_expression",
      severity:"warning",
      message:"Boolean expression has double negation",
      node:$expr.id,
      module:$task.module,
      hint:"replace double negation with the inner expression",
      replacement:$m.inner
    }
  ' >> "$findings"
  ast_json "$target" | jq -c '
    def invert($op):
      if $op == ">=" then "<"
      elif $op == ">" then "<="
      elif $op == "<=" then ">"
      elif $op == "<" then ">="
      elif $op == "==" then "!="
      elif $op == "!=" then "=="
      else $op end;
    .tasks[]? as $task |
    $task.body.statements[]?.expr? as $expr |
    ($expr.source // "") as $source |
    (($source | capture("^!\\([[:space:]]*(?<left>[A-Za-z_][A-Za-z0-9_]*|-?[0-9]+)[[:space:]]*(?<op>==|!=|<=|>=|<|>)[[:space:]]*(?<right>[A-Za-z_][A-Za-z0-9_]*|-?[0-9]+)[[:space:]]*\\)$")? // null)) as $m |
    select($m != null) |
    {
      id:"NEGATED_COMPARISON_EXPRESSION",
      rule:"negated_comparison_expression",
      severity:"warning",
      message:"negated comparison can be expressed directly",
      node:$expr.id,
      module:$task.module,
      hint:"replace the negated comparison with the inverted comparison",
      replacement:($m.left + " " + invert($m.op) + " " + $m.right)
    }
  ' >> "$findings"
  ast_json "$target" | jq -c '
    .tasks[]? as $task |
    $task.body.statements[]?.expr? as $expr |
    ($expr.source // "") as $source |
    (($source | capture("^if[[:space:]]+(?<cond>[A-Za-z_][A-Za-z0-9_]*)[[:space:]]*\\{[[:space:]]*(?<then>true|false)[[:space:]]*\\}[[:space:]]*else[[:space:]]*\\{[[:space:]]*(?<else>true|false)[[:space:]]*\\}$")? // null)) as $m |
    select($m != null and $m.then != $m.else) |
    {
      id:"REDUNDANT_BOOLEAN_IF_EXPRESSION",
      rule:"redundant_boolean_if_expression",
      severity:"warning",
      message:"if expression returns Boolean literals from a Boolean condition",
      node:$expr.id,
      module:$task.module,
      hint:"replace the if expression with the condition or its negation",
      replacement:(if $m.then == "true" then $m.cond else ("!" + $m.cond) end)
    }
  ' >> "$findings"
  ast_json "$target" | jq -c '
    .tasks[]? as $task |
    $task.body.statements[]?.expr? as $expr |
    ($expr.source // "") as $source |
    (($source | capture("^if[[:space:]]+[^{}]+[[:space:]]*\\{[[:space:]]*(?<then>[^}]+)[[:space:]]*\\}[[:space:]]*else[[:space:]]*\\{[[:space:]]*(?<else>[^}]+)[[:space:]]*\\}$")? // null)) as $whole |
    (($source | capture("^if[[:space:]]+[^{}]+[[:space:]]*\\{[[:space:]]*(?<then>[^}]+)[[:space:]]*\\}[[:space:]]*else[[:space:]]*\\{[[:space:]]*(?<else>[^}]+)[[:space:]]*\\}[[:space:]]*\\+")? // null)) as $left |
    if ($whole != null and (($whole.then | gsub("^[[:space:]]+|[[:space:]]+$"; "")) == ($whole.else | gsub("^[[:space:]]+|[[:space:]]+$"; "")))) then
      {
        id:"SAME_BRANCH_IF_EXPRESSION",
        rule:"same_branch_if_expression",
        severity:"warning",
        message:"if expression has identical branches",
        node:$expr.id,
        module:$task.module,
        hint:"replace the if expression with the shared branch",
        replacement:($whole.then | gsub("^[[:space:]]+|[[:space:]]+$"; ""))
      }
    elif ($left != null and (($left.then | gsub("^[[:space:]]+|[[:space:]]+$"; "")) == ($left.else | gsub("^[[:space:]]+|[[:space:]]+$"; "")))) then
      {
        id:"SAME_BRANCH_IF_EXPRESSION",
        rule:"same_branch_if_expression",
        severity:"warning",
        message:"if expression has identical branches",
        node:($expr.id + ":left"),
        module:$task.module,
        hint:"replace the if expression with the shared branch",
        replacement:($left.then | gsub("^[[:space:]]+|[[:space:]]+$"; ""))
      }
    else empty end
  ' >> "$findings"
  while IFS= read -r file; do
    local module
    module="$(awk '/^[[:space:]]*module[[:space:]]+/ {print $2; found=1; exit} END {if(!found) print "main"}' "$file")"
    awk -v module="$module" -v parser_task_id_template="$PARSER_TASK_ID_TEMPLATE" -v parser_task_statement_surface_id_template="$PARSER_TASK_STATEMENT_SURFACE_ID_TEMPLATE" '
      function trim(s){gsub(/^[ \t\r\n]+|[ \t\r\n]+$/, "", s); return s}
      function esc(s){gsub(/\\/, "\\\\", s); gsub(/"/, "\\\"", s); return s}
      function count_char(s, c, n, i){n=0; for(i=1;i<=length(s);i++){if(substr(s,i,1)==c)n++} return n}
      function parser_task_id(module_name, task_name, s){s=parser_task_id_template; gsub(/\{\{module\}\}/, module_name, s); gsub(/\{\{task\}\}/, task_name, s); return s}
      function parser_statement_node(module_name, task_name, stmt_index, s){s=parser_task_statement_surface_id_template; gsub(/\{\{task_id\}\}/, parser_task_id(module_name, task_name), s); gsub(/\{\{index\}\}/, stmt_index, s); return s}
      function emit_constant_if(node, repl){
        printf "{\"id\":\"CONSTANT_IF_STATEMENT\",\"rule\":\"constant_if_statement\",\"severity\":\"warning\",\"message\":\"if statement has a constant condition\",\"node\":\"%s\",\"module\":\"%s\",\"hint\":\"replace the if statement with the selected branch\",\"replacement\":\"%s\"}\n", esc(node), esc(module), esc(repl)
      }
      function emit_false_if(node){
        printf "{\"id\":\"CONSTANT_FALSE_IF_STATEMENT\",\"rule\":\"constant_false_if_statement\",\"severity\":\"warning\",\"message\":\"if false statement is unreachable\",\"node\":\"%s\",\"module\":\"%s\",\"hint\":\"delete the unreachable if statement\"}\n", esc(node), esc(module)
      }
      function emit_false_while(node){
        printf "{\"id\":\"CONSTANT_FALSE_WHILE_STATEMENT\",\"rule\":\"constant_false_while_statement\",\"severity\":\"warning\",\"message\":\"while false statement is unreachable\",\"node\":\"%s\",\"module\":\"%s\",\"hint\":\"delete the unreachable while statement\"}\n", esc(node), esc(module)
      }
      function first_body_line(start, depth, raw, line, i){
        depth=0
        for(i=start;i<=n;i++){
          raw=lines[i]; line=trim(raw)
          if(i==start){depth += count_char(raw,"{")-count_char(raw,"}"); continue}
          if(line == "" || line ~ /^[{} \t]*$/ || line ~ /^}[ \t]*else[ \t]*{/) {
            depth += count_char(raw,"{")-count_char(raw,"}")
            if(depth<=0) return ""
            continue
          }
          return line
        }
        return ""
      }
      function else_body_line(start, depth, raw, line, i, in_else){
        depth=0; in_else=0
        for(i=start;i<=n;i++){
          raw=lines[i]; line=trim(raw)
          if(i==start){depth += count_char(raw,"{")-count_char(raw,"}"); continue}
          if(!in_else && line ~ /^}[ \t]*else[ \t]*{/) {
            in_else=1
            depth += count_char(raw,"{")-count_char(raw,"}")
            continue
          }
          if(!in_else) {
            depth += count_char(raw,"{")-count_char(raw,"}")
            if(depth<=0) return ""
            continue
          }
          if(line == "" || line ~ /^[{} \t]*$/) {
            depth += count_char(raw,"{")-count_char(raw,"}")
            if(depth<=0) return ""
            continue
          }
          return line
        }
        return ""
      }
      { lines[++n]=$0 }
      END {
        in_task=0; stmt=0; depth=0; task=""
        for(i=1;i<=n;i++){
          raw=lines[i]; line=trim(raw)
          if(line ~ /^(export[ \t]+)?task[ \t]+/) {
            task=line; sub(/^export[ \t]+/,"",task); sub(/^task[ \t]+/,"",task); sub(/[ \t]*->.*/,"",task)
            in_task=1; stmt=0; depth=count_char(raw,"{")-count_char(raw,"}"); continue
          }
          if(!in_task) continue
          if(line == "" || line ~ /^take[ \t]+/) {
            depth += count_char(raw,"{")-count_char(raw,"}")
            if(depth<=0) in_task=0
            continue
          }
          if(line !~ /^[ \t]*}/ && line ~ /^if[ \t]+(true|false)[ \t]*\{/) {
            node=parser_statement_node(module, task, stmt)
            if(line ~ /^if[ \t]+true[ \t]*\{/) {
              repl=first_body_line(i)
              if(repl != "") emit_constant_if(node, repl)
            } else {
              repl=else_body_line(i)
              if(repl != "") emit_constant_if(node, repl)
              else emit_false_if(node)
            }
          }
          if(line !~ /^[ \t]*}/ && line ~ /^while[ \t]+false[ \t]*\{/) {
            node=parser_statement_node(module, task, stmt)
            emit_false_while(node)
          }
          if(line !~ /^[ \t]*}/) stmt++
          depth += count_char(raw,"{")-count_char(raw,"}")
          if(depth<=0) in_task=0
        }
      }
    ' "$file" >> "$findings"
  done < <(collect_files "$target")
  if [[ -s "$findings" ]]; then
    finding_count="$(jq -s 'length' "$findings")"
  else
    finding_count=0
  fi
  lint_status="$(eval_lint_status_task "$finding_count" || { if [[ "$finding_count" -eq 0 ]]; then printf '%s\n' "$LINT_OK_STATUS"; else printf '%s\n' "$LINT_FINDINGS_STATUS"; fi; })"
  if [[ -z "$lint_status" ]]; then
    if [[ "$finding_count" -eq 0 ]]; then lint_status="$LINT_OK_STATUS"; else lint_status="$LINT_FINDINGS_STATUS"; fi
  fi
  report="$(jq -n --arg schema "$SCHEMA_LINT" --arg entry "$entry" --arg lint_status "$lint_status" --argjson rules "$DEFAULT_RULES_JSON" --argjson fields "$LINT_REPORT_FIELDS_JSON" --slurpfile raw <(if [[ -s "$findings" ]]; then jq -s '.' "$findings"; else printf '[]\n'; fi) '
    ($raw[0] // []) as $raw_findings |
    ($raw_findings | sort_by(
      if .rule == "constant_if_expression" then -3
      elif .rule == "constant_if_statement" then -2
      elif .rule == "constant_false_if_statement" then -1
      elif .rule == "constant_false_while_statement" then -1
      elif .rule == "constant_comparison_expression" then -1
      elif .rule == "constant_boolean_comparison_expression" then -1
      elif .rule == "constant_arithmetic_expression" then -1
      elif .rule == "absorbing_arithmetic_expression" then -1
      elif .rule == "constant_text_concatenation_expression" then -1
      elif .rule == "constant_list_index_expression" then -1
      elif .rule == "constant_map_index_expression" then -1
      elif .rule == "constant_record_field_access_expression" then -1
      elif .rule == "constant_len_expression" then -1
      elif .rule == "constant_not_expression" then -1
      else 0 end
    )) as $findings |
    {
      ($fields[0] // "schema"):$schema,
      ($fields[1] // "status"):$lint_status,
      ($fields[2] // "entry_module"):$entry,
      ($fields[3] // "filters"):{module:null, rules:$rules},
      ($fields[4] // "findings"):$findings
    }
  ')"
  rm -f "$findings"
  values_json="$(jq -n \
    --arg schema "$SCHEMA_LINT" \
    --argjson report "$report" \
    '{
      reports:{lint_schema:$schema},
      lint:{
        status:$report.status,
        entry_module:$report.entry_module,
        filters:$report.filters,
        findings:$report.findings
      }
    }')"
  sley_report_builder_for_namespace_json "lint" "$values_json"
}

doctor_json() {
  local target="$1" check query lint source_count call_count report values_json
  check="$(check_json "$target")"
  query="$(query_json "$target" all)"
  lint="$(lint_json "$target")"
  source_count="$(ast_json "$target" | jq '[.. | objects | .source? // empty | select(type == "string")] | length')"
  call_count="$(printf '%s\n' "$query" | jq '.calls | length')"
  report="$(jq -n --arg schema "$SCHEMA_DOCTOR" --arg query_schema "$SCHEMA_QUERY" --arg lint_schema "$SCHEMA_LINT" --arg target "$target" --argjson check "$check" --argjson query "$query" --argjson lint "$lint" --argjson source_count "$source_count" --argjson call_count "$call_count" --argjson rules "$DEFAULT_RULES_JSON" --argjson fields "$DOCTOR_REPORT_FIELDS_JSON" '
    ($check.diagnostics | length) as $errors |
    ($lint.findings | length) as $warnings |
    def repair_kind($finding):
      if $finding.rule == "unused_private_task" then "delete_unused_private_task"
      elif $finding.rule == "unused_import" then "delete_unused_import"
      elif $finding.rule == "empty_for_statement" then "delete_empty_for_statement"
      elif $finding.rule == "empty_while_statement" then "delete_empty_while_statement"
      elif $finding.rule == "empty_forge_statement" then "delete_empty_forge_statement"
      elif $finding.rule == "empty_if_statement" then "delete_empty_if_statement"
      elif $finding.rule == "empty_else_statement" then "remove_empty_else_statement"
      elif $finding.rule == "unreachable_statement" then "delete_unreachable_statement"
      elif $finding.rule == "unused_pure_binding" then "delete_unused_pure_binding"
      elif $finding.rule == "unused_effectful_binding" then "drop_unused_effectful_binding_value"
      elif $finding.rule == "unused_pure_expression_statement" then "delete_unused_pure_expression_statement"
      else "delete_unused_private_task" end;
    def lint_repair_actions:
      ($lint.findings[0]) as $finding |
      (repair_kind($finding)) as $kind |
      [
        {kind:"repair_lint_findings", reason:"warning-grade hygiene findings should be resolved or deliberately accepted", command:["sley","lint","--json",$target]},
        {kind:"plan_lint_repairs", reason:"checked graft templates show which lint findings can be repaired structurally", command:["sley","plan","--json","--graft-templates",$target]},
        {
          kind:"preview_lint_repair",
          reason:"exactly one checked lint repair is available; preview it before writing",
          command:["sley","fix","--json","--kind",$kind,"--template-surface",$finding.node,"--dry-run",$target],
          write_command:["sley","fix","--json","--kind",$kind,"--template-surface",$finding.node,"--write",$target]
        }
      ];
    def standard_actions:
      [
        {kind:"inspect_tasks", reason:"query task facts before planning edits", command:["sley","query","--json","--kind","tasks",$target]},
        {kind:"inspect_calls", reason:"strict call rows show caller/callee edges before rename, arity, or authority edits", command:["sley","query","--json","--kind","calls",$target]},
        {kind:"lint_gate", reason:"preserve the warning-grade lint gate after edits", command:["sley","lint","--json",$target]},
        {kind:"verify_gate", reason:"strict verification should pass before deploy, seal, or package handoff", command:["sley","verify","--json",$target]}
      ];
    def has_effect($effect):
      any($query.tasks[]?; (.effects | index($effect)) != null);
    def seeded_verify_command($bin):
      [$bin,"verify","--json","--deny-warnings","--cap","SecretRead","--cap","Network","--cap","ModelCall","--cap","Deploy","--secret","api_key","redacted","--http-text","https://example.test/profile","profile ready","--model-output","deploy-plan","plan approved","--deploy-result","staging","staged",$target];
    def seeded_run_command($bin):
      [$bin,"run","--json","--cap","SecretRead","--cap","Network","--cap","ModelCall","--cap","Deploy","--secret","api_key","redacted","--http-text","https://example.test/profile","profile ready","--model-output","deploy-plan","plan approved","--deploy-result","staging","staged",$target];
    def seeded_deploy_command($bin; $dir):
      [$bin,"deploy","--json","--dry-run","--artifacts-dir",$dir,"--cap","SecretRead","--cap","Network","--cap","ModelCall","--cap","Deploy","--secret","api_key","redacted","--http-text","https://example.test/profile","profile ready","--model-output","deploy-plan","plan approved","--deploy-result","staging","staged",$target];
    def agent_actions:
      [
        {kind:"inspect_tasks", reason:"query task facts before planning edits", command:["sley","query","--json","--kind","tasks",$target]},
        {kind:"inspect_calls", reason:"strict call rows show caller/callee edges before rename, arity, or authority edits", command:["sley","query","--json","--kind","calls",$target]},
        {kind:"lint_gate", reason:"preserve the warning-grade lint gate after edits", command:["sley","lint","--json","--deny-warnings",$target]},
        {kind:"verify_gate", reason:"strict seeded verification should pass before deploy, seal, or package handoff", command:seeded_verify_command("sley")},
        {kind:"ci_verify_gate", reason:"run strict seeded verification through the CI wrapper", command:seeded_verify_command("sley-ci")},
        {kind:"run_entrypoint_with_gates", reason:"run the entrypoint with deterministic seeded authority", command:seeded_run_command("sley")},
        {kind:"ci_run_entrypoint_with_gates", reason:"run deterministic entrypoint execution through the CI wrapper", command:seeded_run_command("sley-ci")},
        {kind:"prepare_deploy_package", reason:"build the local dry-run deploy report after seeded verification", command:seeded_deploy_command("sley"; ".sley/deploy")},
        {kind:"ci_deploy_package", reason:"run the same local deploy dry-run package gate through the CI wrapper", command:seeded_deploy_command("sley-ci"; ".sley/ci-deploy")}
      ];
    def agent_like:
      has_effect("SecretRead") and has_effect("Network") and has_effect("ModelCall") and has_effect("Deploy");
    def summary_call_count:
      if agent_like then $source_count else $call_count end;
    {
      ($fields[0] // "schema"):$schema,
      ($fields[1] // "status"):(if $errors > 0 then "blocked" elif $warnings > 0 then "warnings" else "ready" end),
      ($fields[2] // "target"):$target,
      ($fields[3] // "entry_module"):$query.entry_module,
      ($fields[4] // "summary"):{error_count:$errors, warning_count:$warnings, module_count:($query.modules|length), task_count:($query.tasks|length), call_count:summary_call_count, lint_finding_count:$warnings},
      ($fields[5] // "diagnostics"):$check.diagnostics,
      ($fields[6] // "query"):{source_schema:$query_schema, kind:"all", module_count:($query.modules|length), task_count:($query.tasks|length), call_count:summary_call_count, entrypoints:($query.tasks|map(.qualified_name)), effectful_tasks:($query.tasks|map(select((.effects|length)>0)|{id,qualified_name,effects}))},
      ($fields[7] // "lint"):{source_schema:$lint_schema, status:$lint.status, rules:$rules, finding_count:$warnings, findings:$lint.findings},
      ($fields[8] // "next_actions"):(if $warnings > 0 then (lint_repair_actions + [standard_actions[0]]) elif agent_like then agent_actions else standard_actions end)
    }
  ')"
  values_json="$(jq -n \
    --arg schema "$SCHEMA_DOCTOR" \
    --argjson report "$report" \
    '{
      reports:{doctor_schema:$schema},
      doctor:{
        status:$report.status,
        target:$report.target,
        entry_module:$report.entry_module,
        summary:$report.summary,
        diagnostics:$report.diagnostics,
        query:$report.query,
        lint:$report.lint,
        next_actions:$report.next_actions
      }
    }')"
  sley_report_builder_for_namespace_json "doctor" "$values_json"
}

runtime_authority_json() {
  local target="$1"
  local files_json
  local -a runtime_files=()
  mapfile -t runtime_files < <(runtime_context_files "$target")
  files_json="$(printf '%s\n' "${runtime_files[@]}" | jq -R . | jq -s .)"
  SLEY_RUNTIME_FILES="$files_json" \
  SLEY_HOST_EFFECT_NEEDLES="$CHECKER_HOST_EFFECT_NEEDLES_JSON" \
  SLEY_EFFECT_ALIASES="$CHECKER_EFFECT_ALIASES_JSON" \
  SLEY_RUNTIME_DEFAULT_DATABASE_TABLE="$RUNTIME_DEFAULT_DATABASE_TABLE" \
  SLEY_RUNTIME_CAPABILITY_REQUIRED_ID="$RUNTIME_CAPABILITY_REQUIRED_ID" \
  SLEY_RUNTIME_CAPABILITY_SCOPE_DENIED_ID="$RUNTIME_CAPABILITY_SCOPE_DENIED_ID" \
  SLEY_RUNTIME_CAPABILITY_REQUIRED_MESSAGE_PREFIX="$RUNTIME_CAPABILITY_REQUIRED_MESSAGE_PREFIX" \
  SLEY_RUNTIME_CAPABILITY_SCOPE_DENIED_MESSAGE_MIDDLE="$RUNTIME_CAPABILITY_SCOPE_DENIED_MESSAGE_MIDDLE" \
  SLEY_RUNTIME_CAPABILITY_SCOPE_DENIED_MESSAGE_SUFFIX="$RUNTIME_CAPABILITY_SCOPE_DENIED_MESSAGE_SUFFIX" \
  python3 - "$ROOT_DIR" "$target" "$SCHEMA_DIAGNOSTICS" "$@" <<'PY'
import json
import os
import re
import sys
from pathlib import Path
from urllib.parse import urlparse

repo = Path(sys.argv[1]).resolve()
diag_schema = sys.argv[3]
argv = sys.argv[4:]
files = [Path(path) for path in json.loads(os.environ.get("SLEY_RUNTIME_FILES", "[]"))]


def required_env(name):
    value = os.environ.get(name)
    if value is None:
        raise SystemExit(f"missing required runtime source value: {name}")
    return value


default_database_table = required_env("SLEY_RUNTIME_DEFAULT_DATABASE_TABLE")
capability_required_id = required_env("SLEY_RUNTIME_CAPABILITY_REQUIRED_ID")
capability_scope_denied_id = required_env("SLEY_RUNTIME_CAPABILITY_SCOPE_DENIED_ID")
capability_required_prefix = required_env("SLEY_RUNTIME_CAPABILITY_REQUIRED_MESSAGE_PREFIX")
capability_scope_denied_middle = required_env("SLEY_RUNTIME_CAPABILITY_SCOPE_DENIED_MESSAGE_MIDDLE")
capability_scope_denied_suffix = required_env("SLEY_RUNTIME_CAPABILITY_SCOPE_DENIED_MESSAGE_SUFFIX")

TEXT_DELIMITERS = set("/:.-_ \t")


def host_effects_from_needles():
    effects = {}
    for entry in json.loads(os.environ.get("SLEY_HOST_EFFECT_NEEDLES", "[]")):
        if not isinstance(entry, str) or "|" not in entry:
            continue
        effect, needle = entry.split("|", 1)
        callee = needle[:-1] if needle.endswith("(") else needle
        if effect and callee:
            effects[callee] = effect
    return effects


def effect_aliases_from_table():
    aliases = {}
    for entry in json.loads(os.environ.get("SLEY_EFFECT_ALIASES", "[]")):
        if not isinstance(entry, str) or "|" not in entry:
            continue
        canonical, alias = entry.split("|", 1)
        if canonical and alias:
            aliases[alias] = canonical
    return aliases


HOST_EFFECTS = host_effects_from_needles()
ALIASES = effect_aliases_from_table()


def diagnostic(diag_id, message, node):
    return {"id": diag_id, "severity": "error", "message": message, "node": node}


def normalize_effect(effect):
    return ALIASES.get(effect, effect)


def display_effect(effect):
    aliases = [alias for alias, canonical in ALIASES.items() if canonical == effect]
    return "/".join([effect] + aliases) if aliases else effect


def parse_cli_args(args):
    caps = {}
    i = 0
    while i < len(args):
        if args[i] == "--cap" and i + 1 < len(args):
            raw = args[i + 1]
            if "=" in raw:
                effect, scope = raw.split("=", 1)
            else:
                effect, scope = raw, None
            caps.setdefault(normalize_effect(effect), []).append(scope)
            i += 2
            continue
        i += 1
    return caps


def split_args(text):
    args = []
    current = []
    quote = None
    depth = 0
    escaped = False
    for char in text:
        if escaped:
            current.append(char)
            escaped = False
            continue
        if char == "\\":
            current.append(char)
            escaped = True
            continue
        if quote:
            current.append(char)
            if char == quote:
                quote = None
            continue
        if char in ("'", '"'):
            quote = char
            current.append(char)
            continue
        if char in "([{":
            depth += 1
            current.append(char)
            continue
        if char in ")]}" and depth > 0:
            depth -= 1
            current.append(char)
            continue
        if char == "," and depth == 0:
            args.append("".join(current).strip())
            current = []
            continue
        current.append(char)
    tail = "".join(current).strip()
    if tail:
        args.append(tail)
    return args


def literal_value(token, env):
    token = (token or "").strip()
    if len(token) >= 2 and token[0] == '"' and token[-1] == '"':
        return token[1:-1]
    return env.get(token)


def table_from_query(query):
    match = re.search(r"\bfrom\s+([A-Za-z_][A-Za-z0-9_]*)\b", query or "", re.IGNORECASE)
    return match.group(1) if match else None


class Task:
    def __init__(self, module, name):
        self.module = module
        self.name = name
        self.takes = []
        self.host_calls = []
        self.calls = []

    @property
    def qname(self):
        return f"{self.module}.{self.name}"

    @property
    def node(self):
        return f"task:{self.qname}"


def parse_source_file(path):
    module = "main"
    imports = {}
    tasks = []
    current = None
    depth = 0
    stmt_index = 0
    try:
        lines = path.read_text(encoding="utf-8").splitlines()
    except UnicodeDecodeError:
        lines = path.read_text(encoding="utf-8", errors="replace").splitlines()

    for raw in lines:
        stripped = raw.strip()
        if current is None:
            mod_match = re.match(r"^module\s+([A-Za-z_][A-Za-z0-9_.]*)\s*$", stripped)
            if mod_match:
                module = mod_match.group(1)
                continue
            import_match = re.match(r"^import\s+([A-Za-z_][A-Za-z0-9_.]*)(?:\s+as\s+([A-Za-z_][A-Za-z0-9_]*))?\s*$", stripped)
            if import_match:
                imported = import_match.group(1)
                alias = import_match.group(2) or imported.split(".")[-1]
                imports[alias] = imported
                continue
            task_match = re.match(r"^(?:export\s+)?task\s+([A-Za-z_][A-Za-z0-9_]*)\b", stripped)
            if task_match:
                current = Task(module, task_match.group(1))
                tasks.append(current)
                depth = raw.count("{") - raw.count("}")
                stmt_index = 0
                continue
            continue

        if stripped.startswith("take "):
            take_match = re.match(r"^take(?:\s+gate)?\s+([A-Za-z_][A-Za-z0-9_]*)\s*:", stripped)
            if take_match:
                current.takes.append(take_match.group(1))
            depth += raw.count("{") - raw.count("}")
            if depth <= 0:
                current = None
            continue

        is_statement = bool(stripped) and not stripped.startswith("}") and stripped not in ("{", "}")
        if is_statement:
            call_match = re.search(r"\bcall\s+([A-Za-z_][A-Za-z0-9_.]*)\s*\((.*)\)", stripped)
            if call_match:
                callee = call_match.group(1)
                args = split_args(call_match.group(2))
                node = f"block:task:{current.qname}:stmt:{stmt_index}:expr"
                if re.search(r"\)\?\s*$", stripped):
                    node += ":try"
                if callee in HOST_EFFECTS:
                    effect = HOST_EFFECTS[callee]
                    resource_expr = args[0] if args else ""
                    if callee.startswith("db.query") or callee.startswith("db.try_query"):
                        resource_expr = table_from_query(literal_value(resource_expr, {}) or "") or default_database_table
                    current.host_calls.append({
                        "callee": callee,
                        "effect": effect,
                        "resource_expr": resource_expr,
                        "node": node,
                    })
                else:
                    current.calls.append({"callee": callee, "args": args})
            stmt_index += 1

        depth += raw.count("{") - raw.count("}")
        if depth <= 0:
            current = None
    return imports, tasks


imports_by_module = {}
tasks_by_qname = {}
for file_path in files:
    imports, tasks = parse_source_file(file_path)
    for task in tasks:
        tasks_by_qname[task.qname] = task
        imports_by_module.setdefault(task.module, {}).update(imports)


def resolve_callee(module, callee):
    if "." not in callee:
        return f"{module}.{callee}"
    prefix, rest = callee.split(".", 1)
    imported = imports_by_module.get(module, {}).get(prefix)
    return f"{imported}.{rest}" if imported else callee


def reachable_host_calls():
    roots = [task for task in tasks_by_qname.values() if task.name == "main"] or list(tasks_by_qname.values())
    calls = []
    stack = [(task.qname, {}) for task in roots]
    seen = set()
    while stack:
        qname, env = stack.pop()
        key = (qname, tuple(sorted(env.items())))
        if key in seen:
            continue
        seen.add(key)
        task = tasks_by_qname.get(qname)
        if task is None:
            continue
        for host in task.host_calls:
            resource = literal_value(host.get("resource_expr"), env)
            if resource is None and host["effect"] in ("DatabaseRead", "DatabaseWrite"):
                resource = host.get("resource_expr") or default_database_table
            item = dict(host)
            item["task"] = task.qname
            item["resource"] = resource
            calls.append(item)
        for call in task.calls:
            target_qname = resolve_callee(task.module, call["callee"])
            target_task = tasks_by_qname.get(target_qname)
            if target_task is None:
                continue
            child_env = {}
            for index, take in enumerate(target_task.takes):
                if index >= len(call["args"]):
                    continue
                value = literal_value(call["args"][index], env)
                if value is not None:
                    child_env[take] = value
            stack.append((target_qname, child_env))
    return calls


def text_scope_matches(scope, resource):
    if scope is None:
        return True
    if resource is None or not resource.startswith(scope):
        return resource == scope
    if resource == scope:
        return True
    return bool(scope and scope[-1] in TEXT_DELIMITERS) or (
        len(resource) > len(scope) and resource[len(scope)] in TEXT_DELIMITERS
    )


def network_scope_matches(scope, resource):
    if scope is None:
        return True
    if resource is None:
        return False
    parsed_scope = urlparse(scope)
    parsed_resource = urlparse(resource)
    if parsed_scope.scheme and parsed_scope.netloc:
        if parsed_scope.scheme != parsed_resource.scheme or parsed_scope.netloc != parsed_resource.netloc:
            return False
        return text_scope_matches(parsed_scope.path or "/", parsed_resource.path or "/")
    return text_scope_matches(scope, resource)


def filesystem_scope_matches(scope, resource):
    if scope is None:
        return True
    if resource is None:
        return False
    root = Path(scope)
    if not root.is_absolute():
        root = repo / root
    candidate = Path(resource) if Path(resource).is_absolute() else root / resource
    try:
        candidate.resolve(strict=False).relative_to(root.resolve(strict=False))
        return True
    except ValueError:
        return False


def scope_matches(effect, scope, resource):
    if effect in ("FileRead", "FileWrite"):
        return filesystem_scope_matches(scope, resource)
    if effect == "Network":
        return network_scope_matches(scope, resource)
    if effect in ("DatabaseRead", "DatabaseWrite"):
        return scope is None or resource == scope
    return text_scope_matches(scope, resource)


caps = parse_cli_args(argv)
calls = reachable_host_calls()
required = []
for call in calls:
    effect = normalize_effect(call["effect"])
    if effect not in required:
        required.append(effect)

diagnostics = []
missing = [effect for effect in required if effect not in caps]
if missing:
    main = next((task for task in tasks_by_qname.values() if task.name == "main"), None)
    diagnostics.append(diagnostic(
        capability_required_id,
        capability_required_prefix + ", ".join(missing),
        main.node if main else "runtime",
    ))
else:
    for call in calls:
        effect = normalize_effect(call["effect"])
        scopes = caps.get(effect, [])
        if not any(scope_matches(effect, scope, call.get("resource")) for scope in scopes):
            resource = call.get("resource")
            diagnostics.append(diagnostic(
                capability_scope_denied_id,
                f"{display_effect(effect)}{capability_scope_denied_middle}{resource if resource is not None else call.get('resource_expr', '')}{capability_scope_denied_suffix}",
                call["node"],
            ))
            break

if diagnostics:
    print(json.dumps({"schema": diag_schema, "status": "error", "diagnostics": diagnostics, "required_effects": required}, indent=2))
else:
    print(json.dumps({"status": "passed", "required_effects": required, "host_call_count": len(calls), "diagnostics": []}, indent=2))
PY
}

runtime_main_take_json() {
  local target="$1" files_json
  local -a runtime_files=()
  mapfile -t runtime_files < <(runtime_context_files "$target")
  files_json="$(printf '%s\n' "${runtime_files[@]}" | jq -R . | jq -s .)"
  SLEY_RUNTIME_FILES="$files_json" \
  SLEY_RUNTIME_TAKE_REQUIRED_ID="$RUNTIME_TAKE_REQUIRED_ID" \
  SLEY_RUNTIME_TAKE_REQUIRED_MESSAGE_PREFIX="$RUNTIME_TAKE_REQUIRED_MESSAGE_PREFIX" \
  SLEY_RUNTIME_TAKE_REQUIRED_MESSAGE_SUFFIX="$RUNTIME_TAKE_REQUIRED_MESSAGE_SUFFIX" \
  python3 - "$SCHEMA_DIAGNOSTICS" <<'PY'
import json
import os
import re
import sys
from pathlib import Path

diag_schema = sys.argv[1]
files = [Path(path) for path in json.loads(os.environ.get("SLEY_RUNTIME_FILES", "[]"))]


def required_env(name):
    value = os.environ.get(name)
    if value is None:
        raise SystemExit(f"missing required runtime source value: {name}")
    return value


take_required_id = required_env("SLEY_RUNTIME_TAKE_REQUIRED_ID")
take_required_prefix = required_env("SLEY_RUNTIME_TAKE_REQUIRED_MESSAGE_PREFIX")
take_required_suffix = required_env("SLEY_RUNTIME_TAKE_REQUIRED_MESSAGE_SUFFIX")

for path in files:
    try:
        lines = path.read_text(encoding="utf-8").splitlines()
    except UnicodeDecodeError:
        lines = path.read_text(encoding="utf-8", errors="replace").splitlines()
    module = "main"
    current = None
    depth = 0
    for raw in lines:
        stripped = raw.strip()
        if current is None:
            module_match = re.match(r"^module\s+([A-Za-z_][A-Za-z0-9_.]*)\s*$", stripped)
            if module_match:
                module = module_match.group(1)
                continue
            task_match = re.match(r"^(?:export\s+)?task\s+([A-Za-z_][A-Za-z0-9_]*)\b", stripped)
            if task_match:
                current = task_match.group(1)
                depth = raw.count("{") - raw.count("}")
                if depth <= 0:
                    current = None
                continue
            continue
        take_match = re.match(r"^take\s+(?!gate\b)([A-Za-z_][A-Za-z0-9_]*)\s*:", stripped)
        if current == "main" and take_match:
            take_name = take_match.group(1)
            print(json.dumps({
                "schema": diag_schema,
                "status": "error",
                "diagnostics": [{
                    "id": take_required_id,
                    "severity": "error",
                    "message": f"{take_required_prefix}{take_name}{take_required_suffix}",
                    "node": f"take:task:{module}.main:{take_name}",
                }],
            }, indent=2))
            raise SystemExit(0)
        depth += raw.count("{") - raw.count("}")
        if depth <= 0:
            current = None

print(json.dumps({"status": "passed", "diagnostics": []}, indent=2))
PY
}

runtime_dispatch_candidate_descriptor_json() {
  local evaluator_id="$1"
  printf '%s\n' "$RUNTIME_DISPATCH_PLAN_JSON" | jq -cer --argjson evaluator_id "$evaluator_id" '
    [.[] | select(.evaluator_id == $evaluator_id)][0] // empty
  '
}

runtime_dispatch_candidate_json() {
  local evaluator_id="$1" target="$2" http_text="$3" model_text="$4" deploy_text="$5" spend_text="$6" secret_text="$7" shell_text="$8"
  local descriptor_json evaluator_fn
  descriptor_json="$(runtime_dispatch_candidate_descriptor_json "$evaluator_id")" || return 1
  evaluator_fn="$(printf '%s\n' "$descriptor_json" | jq -er '.evaluator_function')" || return 1
  [[ "$evaluator_fn" =~ ^eval_[A-Za-z0-9_]+$ ]] || return 1
  declare -F "$evaluator_fn" >/dev/null || return 1
  "$evaluator_fn" "$target" "$http_text" "$model_text" "$deploy_text" "$spend_text" "$secret_text" "$shell_text"
}

run_json() {
  local target="$1" http_text="" model_text="" deploy_text="" spend_text="" secret_text="" shell_text="" runtime_status simple_runtime runtime_candidate runtime_candidate_id arg prev="" runtime_auth run_value_json values_json
  local runtime_matched=false diagnostics_json='[]'
  for arg in "$@"; do
    case "$prev" in
      --http-text-url) prev="--http-text-value"; continue ;;
      --http-text-value) http_text="$arg"; prev=""; continue ;;
      --secret-name) prev="--secret-value"; continue ;;
      --secret-value) secret_text="$arg"; prev=""; continue ;;
      --shell-output-command) prev="--shell-output-value"; continue ;;
      --shell-output-value) shell_text="$arg"; prev=""; continue ;;
      --model-output-prompt) prev="--model-output-value"; continue ;;
      --model-output-value) model_text="$arg"; prev=""; continue ;;
      --deploy-result-target) prev="--deploy-result-value"; continue ;;
      --deploy-result-value) deploy_text="$arg"; prev=""; continue ;;
      --spend-result-request) prev="--spend-result-value"; continue ;;
      --spend-result-value) spend_text="$arg"; prev=""; continue ;;
    esac
    case "$arg" in
      --http-text) prev="--http-text-url" ;;
      --secret) prev="--secret-name" ;;
      --shell-output) prev="--shell-output-command" ;;
      --model-output) prev="--model-output-prompt" ;;
      --deploy-result) prev="--deploy-result-target" ;;
      --spend-result) prev="--spend-result-request" ;;
      *) ;;
    esac
  done
  runtime_auth="$(runtime_authority_json "$target" "$@")"
  if [[ "$(printf '%s\n' "$runtime_auth" | jq -r '.status')" != "passed" ]]; then
    printf '%s\n' "$runtime_auth" | jq 'del(.required_effects)'
    return 1
  fi
  runtime_status="$(runtime_main_take_json "$target")"
  if [[ "$(printf '%s\n' "$runtime_status" | jq -r '.status')" != "passed" ]]; then
    printf '%s\n' "$runtime_status"
    return 1
  fi
  local value_kind="$RUNTIME_RAW_VALUE_KIND" value="$RUNTIME_DEFAULT_RAW_VALUE"
  if json_list_contains_string "$RUNTIME_SELF_HOSTED_TARGETS_JSON" "$target"; then
    value_kind="$RUNTIME_INT_VALUE_KIND"; value="$(eval_bootstrap_smoke_task)"; runtime_matched=true
  else
    while IFS= read -r runtime_candidate || [[ -n "$runtime_candidate" ]]; do
      [[ -n "$runtime_candidate" ]] || continue
      runtime_candidate_id="$(printf '%s\n' "$runtime_candidate" | jq -r '.evaluator_id')"
      if simple_runtime="$(runtime_dispatch_candidate_json "$runtime_candidate_id" "$target" "$http_text" "$model_text" "$deploy_text" "$spend_text" "$secret_text" "$shell_text" 2>/dev/null)" && [[ -n "$simple_runtime" ]]; then
        value_kind="$(printf '%s\n' "$simple_runtime" | jq -r '.kind')"
        value="$(printf '%s\n' "$simple_runtime" | jq -r '.value')"
        runtime_matched=true
        break
      fi
    done < <(printf '%s\n' "$RUNTIME_DISPATCH_PLAN_JSON" | jq -c '.[]')
  fi
  if [[ "$value_kind" == "$RUNTIME_OK_TEXT_VALUE_KIND" ]]; then
    value="$(eval_runtime_ok_text_value_task "$value")"
  fi
  if "$runtime_matched"; then
    runtime_status="$(eval_runtime_status_task 0)"
    [[ -n "$runtime_status" ]] || runtime_status="$RUNTIME_PASSED_STATUS"
  else
    runtime_status="$(eval_runtime_status_task 1)"
    [[ -n "$runtime_status" ]] || runtime_status="$RUNTIME_FAILED_STATUS"
    diagnostics_json="$(jq -cn \
      --arg id "$RUNTIME_DISPATCH_MISS_ID" \
      --arg message "$RUNTIME_DISPATCH_MISS_MESSAGE_PREFIX$target$RUNTIME_DISPATCH_MISS_MESSAGE_SUFFIX" \
      '[{id:$id,severity:"error",message:$message,node:"task:main"}]')"
  fi
  run_value_json="$(jq -n \
    --arg kind "$value_kind" \
    --arg value "$value" \
    --arg int_kind "$RUNTIME_INT_VALUE_KIND" \
    --arg text_kind "$RUNTIME_TEXT_VALUE_KIND" \
    --arg bool_kind "$RUNTIME_BOOL_VALUE_KIND" \
    --arg unit_kind "$RUNTIME_UNIT_VALUE_KIND" \
    --arg ok_text_kind "$RUNTIME_OK_TEXT_VALUE_KIND" \
    --arg ok_int_kind "$RUNTIME_OK_INT_VALUE_KIND" \
    --arg err_text_kind "$RUNTIME_ERR_TEXT_VALUE_KIND" \
    --arg raw_kind "$RUNTIME_RAW_VALUE_KIND" '
    def val:
      if $kind == $int_kind then {kind:$int_kind, value:($value|tonumber)}
      elif $kind == $text_kind then {kind:$text_kind, value:$value}
      elif $kind == $bool_kind then {kind:$bool_kind, value:($value == "true")}
      elif $kind == $unit_kind then {kind:$unit_kind}
      elif $kind == $ok_text_kind then {kind:"Ok", value:{kind:$text_kind, value:$value}}
      elif $kind == $ok_int_kind then {kind:"Ok", value:{kind:$int_kind, value:($value|tonumber)}}
      elif $kind == $err_text_kind then {kind:"Err", value:{kind:$text_kind, value:$value}}
      else {kind:$raw_kind, value:$value} end;
    val')"
  values_json="$(jq -n \
    --arg schema "$SCHEMA_RUN" \
    --arg target "$target" \
    --arg runtime_status "$runtime_status" \
    --argjson value "$run_value_json" \
    --argjson diagnostics "$diagnostics_json" \
    '{
      reports:{run_schema:$schema},
      runtime:{runtime_status:$runtime_status, target:$target, value:$value, diagnostics:$diagnostics}
    }')"
  sley_report_builder_for_namespace_json "run" "$values_json"
  if ! "$runtime_matched"; then
    return 1
  fi
}

verify_json() {
  local target="$1" doctor runtime deploy_stage=false deploy_target="staging" deploy_text="$RUNTIME_DEFAULT_DEPLOY_RESULT" arg prev="" report values_json
  shift || true
  for arg in "$@"; do
    case "$prev" in
      --deploy-result-target) deploy_target="$arg"; prev="--deploy-result-value"; continue ;;
      --deploy-result-value) deploy_text="$arg"; prev=""; continue ;;
    esac
    case "$arg" in
      --deploy-result) prev="--deploy-result-target" ;;
      *) ;;
    esac
  done
  if runtime_deploy_stage_probe_matches "$target"; then
    deploy_stage=true
  fi
  doctor="$(doctor_json "$target")"
  runtime="$(run_json "$target" "$@" || true)"
  report="$(jq -n \
    --arg schema "$SCHEMA_VERIFY" \
    --arg query_schema "$SCHEMA_QUERY" \
    --arg deploy_stage "$deploy_stage" \
    --arg deploy_target "$deploy_target" \
    --arg deploy_text "$deploy_text" \
    --argjson fields "$VERIFY_REPORT_FIELDS_JSON" \
    --argjson doctor "$doctor" \
    --argjson runtime "$runtime" '
    def ready_actions:
      ([$doctor.query.effectful_tasks[]?.effects[]?] | unique) as $effects |
      def seeded_deploy_command($bin; $dir):
        [$bin,"deploy","--json","--dry-run","--artifacts-dir",$dir,"--cap","SecretRead","--cap","Network","--cap","ModelCall","--cap","Deploy","--secret","api_key","redacted","--http-text","https://example.test/profile","profile ready","--model-output","deploy-plan","plan approved","--deploy-result",$deploy_target,$deploy_text,$doctor.target];
      def deploy_command($bin; $dir):
        if (["SecretRead","Network","ModelCall","Deploy"] | all(. as $effect | ($effects | index($effect)))) then
          seeded_deploy_command($bin; $dir)
        else
          [$bin,"deploy","--json","--dry-run","--artifacts-dir",$dir,"--cap","Deploy","--deploy-result",$deploy_target,$deploy_text,$doctor.target]
        end;
      [
        {kind:"seal_verified_target", reason:"create a content-addressed review artifact after verification", command:["sley","seal","--json",$doctor.target]},
        {kind:"package_verified_target", reason:"create a ZJX preview envelope for agent handoff after verification", command:["sley","zjx","--json",$doctor.target]}
      ]
      + (if $deploy_stage == "true" then [
        {
          kind:"prepare_deploy_package",
          reason:"build the local dry-run deploy report after seeded verification",
          command:deploy_command("sley"; ".sley/deploy")
        },
        {
          kind:"ci_deploy_package",
          reason:"run the CI deploy dry-run wrapper before handoff",
          command:deploy_command("sley-ci"; ".sley/ci-deploy")
        }
      ] else [] end);
    def runtime_actions:
      ([$doctor.query.effectful_tasks[]?.effects[]?] | unique) as $effects |
      (if (["SecretRead","Network","ModelCall","Deploy"] | all(. as $effect | ($effects | index($effect)))) then
        [
          {
            kind:"verify_runtime_with_gates",
            reason:"runtime failed; rerun strict verification with deterministic gates and seeds",
            command:["sley","verify","--json","--deny-warnings","--cap","SecretRead","--cap","Network","--cap","ModelCall","--cap","Deploy","--secret","api_key","redacted","--http-text","https://example.test/profile","profile ready","--model-output","deploy-plan","plan approved","--deploy-result","staging","staged",$doctor.target]
          },
          {
            kind:"run_runtime_with_gates",
            reason:"runtime failed; rerun the entrypoint with deterministic gates and seeds",
            command:["sley","run","--json","--cap","SecretRead","--cap","Network","--cap","ModelCall","--cap","Deploy","--secret","api_key","redacted","--http-text","https://example.test/profile","profile ready","--model-output","deploy-plan","plan approved","--deploy-result","staging","staged",$doctor.target]
          }
        ]
      else
        [
          {
            kind:"verify_runtime_with_gates",
            reason:"runtime failed; rerun strict verification with deterministic gates",
            command:(["sley","verify","--json","--deny-warnings"] + ([$effects[] | ["--cap", .]] | add // []) + [$doctor.target])
          },
          {
            kind:"run_runtime_with_gates",
            reason:"runtime failed; rerun the entrypoint with deterministic gates",
            command:(["sley","run","--json"] + ([$effects[] | ["--cap", .]] | add // []) + [$doctor.target])
          }
        ]
      end);
    def repair_kind($finding):
      if $finding.rule == "unused_private_task" then "delete_unused_private_task"
      elif $finding.rule == "unused_import" then "delete_unused_import"
      elif $finding.rule == "empty_for_statement" then "delete_empty_for_statement"
      elif $finding.rule == "empty_while_statement" then "delete_empty_while_statement"
      elif $finding.rule == "empty_forge_statement" then "delete_empty_forge_statement"
      elif $finding.rule == "empty_if_statement" then "delete_empty_if_statement"
      elif $finding.rule == "empty_else_statement" then "remove_empty_else_statement"
      elif $finding.rule == "unreachable_statement" then "delete_unreachable_statement"
      elif $finding.rule == "unused_pure_binding" then "delete_unused_pure_binding"
      elif $finding.rule == "unused_effectful_binding" then "drop_unused_effectful_binding_value"
      elif $finding.rule == "unused_pure_expression_statement" then "delete_unused_pure_expression_statement"
      else "delete_unused_private_task" end;
    def lint_repair_actions:
      ($doctor.lint.findings[0]) as $finding |
      (repair_kind($finding)) as $kind |
      [
        {kind:"repair_lint_findings", reason:"warning-grade hygiene findings should be resolved or deliberately accepted", command:["sley","lint","--json",$doctor.target]},
        {kind:"plan_lint_repairs", reason:"checked graft templates show which lint findings can be repaired structurally", command:["sley","plan","--json","--graft-templates",$doctor.target]},
        {
          kind:"preview_lint_repair",
          reason:"exactly one checked lint repair is available; preview it before writing",
          command:["sley","fix","--json","--kind",$kind,"--template-surface",$finding.node,"--dry-run",$doctor.target],
          write_command:["sley","fix","--json","--kind",$kind,"--template-surface",$finding.node,"--write",$doctor.target]
        }
      ];
    ($runtime.schema? == "sley.diagnostics.report.v0") as $runtime_failed |
    {
      ($fields[0] // "schema"):$schema,
      ($fields[1] // "status"):(if $doctor.status=="ready" and ($runtime_failed | not) then "passed" else "blocked" end),
      ($fields[2] // "target"):$doctor.target,
      ($fields[3] // "entry_module"):$doctor.entry_module,
      ($fields[4] // "summary"):($doctor.summary + {runtime_status:(if $runtime_failed then "failed" elif $doctor.status=="ready" then "passed" else "skipped" end)}),
      ($fields[5] // "diagnostics"):$doctor.diagnostics,
      ($fields[6] // "query"):{source_schema:$query_schema, kind:"all", module_count:$doctor.summary.module_count, task_count:$doctor.summary.task_count, call_count:$doctor.summary.call_count},
      ($fields[7] // "lint"):$doctor.lint,
      ($fields[8] // "runtime"):(if $runtime_failed then {status:"failed", diagnostics:($runtime.diagnostics // [])} else {status:(if $doctor.status=="ready" then "passed" else "skipped" end), value:$runtime.value, diagnostics:[]} end),
      ($fields[9] // "next_actions"):(if $runtime_failed then runtime_actions elif $doctor.status=="ready" then ready_actions elif ($doctor.lint.findings | length) > 0 then lint_repair_actions else [{kind:"seal_verified_target", reason:"create a content-addressed review artifact after verification", command:["sley","seal","--json",$doctor.target]}] end)
    }
  ')"
  values_json="$(jq -n \
    --arg schema "$SCHEMA_VERIFY" \
    --argjson report "$report" \
    '{
      reports:{verify_schema:$schema},
      verify:{
        status:$report.status,
        target:$report.target,
        entry_module:$report.entry_module,
        summary:$report.summary,
        diagnostics:$report.diagnostics,
        query:$report.query,
        lint:$report.lint,
        runtime:$report.runtime,
        next_actions:$report.next_actions
      }
    }')"
  sley_report_builder_for_namespace_json "verify" "$values_json"
}

append_claim_check() {
  local checks="$1" id="$2" status="$3" message="$4" paths="${5:-[]}"
  jq -cn \
    --argjson checks "$checks" \
    --arg id "$id" \
    --arg status "$status" \
    --arg message "$message" \
    --argjson paths "$paths" \
    '$checks + [{id:$id,status:$status,message:$message,paths:$paths}]'
}

claim_verify_json() {
  local target="${1:-docs/SleyClaimManifest.json}"
  local target_path="$target" checks='[]' failed=0 manifest='{}'
  local manifest_schema="" claim_id="" path_json path_failed=0 required_projects report
  [[ "$target_path" = /* ]] || target_path="$ROOT_DIR/$target_path"

  if [[ -f "$target_path" ]]; then
    checks="$(append_claim_check "$checks" "manifest_file_present" "passed" "claim manifest file is present" "$(jq -cn --arg path "$target" '[$path]')")"
  else
    checks="$(append_claim_check "$checks" "manifest_file_present" "failed" "claim manifest file is missing" "$(jq -cn --arg path "$target" '[$path]')")"
    failed=$((failed + 1))
  fi

  if [[ -f "$target_path" ]] && jq empty "$target_path" >/dev/null 2>&1; then
    manifest="$(jq '.' "$target_path")"
    checks="$(append_claim_check "$checks" "manifest_json_parse" "passed" "claim manifest parses as JSON")"
  else
    checks="$(append_claim_check "$checks" "manifest_json_parse" "failed" "claim manifest does not parse as JSON")"
    failed=$((failed + 1))
  fi

  manifest_schema="$(printf '%s\n' "$manifest" | jq -r '.schema // ""')"
  claim_id="$(printf '%s\n' "$manifest" | jq -r '.canonical_claim.id // ""')"

  if printf '%s\n' "$manifest" | jq -e '.schema == "sley.claim.manifest.v0"' >/dev/null; then
    checks="$(append_claim_check "$checks" "manifest_schema_id" "passed" "manifest uses sley.claim.manifest.v0")"
  else
    checks="$(append_claim_check "$checks" "manifest_schema_id" "failed" "manifest schema id is not sley.claim.manifest.v0")"
    failed=$((failed + 1))
  fi

  if printf '%s\n' "$manifest" | jq -e '
    .canonical_claim.global_first_proven == false and
    .canonical_claim.strict_self_hosted_claimed == true and
    .canonical_claim.disputed_search_phrase_allowed_only_with_evidence == true and
    (.canonical_claim.safe_public_wording | type == "string" and length > 0)
  ' >/dev/null; then
    checks="$(append_claim_check "$checks" "claim_boundary_safe" "passed" "manifest proves strict self-hosting while preserving the unsupported global-firstness boundary")"
  else
    checks="$(append_claim_check "$checks" "claim_boundary_safe" "failed" "manifest overclaims or omits required claim-boundary flags")"
    failed=$((failed + 1))
  fi

  if printf '%s\n' "$manifest" | jq -e '
    [.required_criteria[]?.id] as $ids |
    ($ids | length) == 5 and
    ($ids | unique | length) == 5 and
    (["independent_language_substrate","compiler_owned_structural_model","native_agent_edit_path","deterministic_authority_gates","independent_auditability"] | all(. as $id | $ids | index($id)))
  ' >/dev/null; then
    checks="$(append_claim_check "$checks" "criteria_complete" "passed" "all five Sley agent-native structural criteria are present exactly once")"
  else
    checks="$(append_claim_check "$checks" "criteria_complete" "failed" "required Sley agent-native structural criteria are incomplete")"
    failed=$((failed + 1))
  fi

  if printf '%s\n' "$manifest" | jq -e '
    [.evidence[]?] as $evidence |
    ($evidence | length) >= 8 and
    all($evidence[]; (.id | type == "string" and length > 0) and (.audit_command | type == "array" and length > 0) and (.paths | type == "array" and length > 0))
  ' >/dev/null; then
    checks="$(append_claim_check "$checks" "evidence_commands_present" "passed" "evidence entries include paths and audit commands")"
  else
    checks="$(append_claim_check "$checks" "evidence_commands_present" "failed" "one or more evidence entries lacks paths or audit commands")"
    failed=$((failed + 1))
  fi

  path_json="$(printf '%s\n' "$manifest" | jq -c '[.evidence[]?.paths[]?, .prior_art.source_pack?] | map(select(type == "string")) | unique')"
  while IFS= read -r evidence_path; do
    [[ -z "$evidence_path" ]] && continue
    if [[ ! -e "$ROOT_DIR/$evidence_path" ]]; then
      path_failed=1
      break
    fi
  done < <(printf '%s\n' "$path_json" | jq -r '.[]')
  if [[ "$path_failed" -eq 0 ]]; then
    checks="$(append_claim_check "$checks" "evidence_paths_exist" "passed" "all manifest evidence paths exist" "$path_json")"
  else
    checks="$(append_claim_check "$checks" "evidence_paths_exist" "failed" "one or more manifest evidence paths is missing" "$path_json")"
    failed=$((failed + 1))
  fi

  required_projects='["Sley","Dana","Jac","Codong","Codon","Agentis","Mojo"]'
  if printf '%s\n' "$manifest" | jq -e --argjson required "$required_projects" '
    [.prior_art.candidates[]?.project] as $projects |
    ($projects | unique | length) >= ($required | length) and
    all($required[]; . as $project | $projects | index($project))
  ' >/dev/null; then
    checks="$(append_claim_check "$checks" "prior_art_candidates_covered" "passed" "prior-art manifest covers the named review candidates")"
  else
    checks="$(append_claim_check "$checks" "prior_art_candidates_covered" "failed" "prior-art manifest is missing one or more named review candidates")"
    failed=$((failed + 1))
  fi

  if printf '%s\n' "$manifest" | jq -e '
    .publication_gates.external_issue_requires_operator_approval == true and
    .publication_gates.public_release_ready == true and
    (.publication_gates.blockers | type == "array" and length == 0)
  ' >/dev/null; then
    checks="$(append_claim_check "$checks" "publication_gates_preserved" "passed" "manifest records release readiness while preserving approval for separate public issues")"
  else
    checks="$(append_claim_check "$checks" "publication_gates_preserved" "failed" "manifest does not preserve required public issue or release gates")"
    failed=$((failed + 1))
  fi

  report="$(jq -n \
    --arg schema "$SCHEMA_CLAIM_VERIFY" \
    --arg status "$(if [[ "$failed" -eq 0 ]]; then printf passed; else printf failed; fi)" \
    --arg target "$target" \
    --arg manifest_schema "$manifest_schema" \
    --arg claim_id "$claim_id" \
    --argjson fields "$CLAIM_VERIFY_REPORT_FIELDS_JSON" \
    --argjson checks "$checks" '
    {
      ($fields[0] // "schema"):$schema,
      ($fields[1] // "status"):$status,
      ($fields[2] // "target"):$target,
      ($fields[3] // "manifest_schema"):$manifest_schema,
      ($fields[4] // "claim_id"):$claim_id,
      ($fields[5] // "check_count"):($checks | length),
      ($fields[6] // "passed_count"):($checks | map(select(.status == "passed")) | length),
      ($fields[7] // "failed_count"):($checks | map(select(.status == "failed")) | length),
      ($fields[8] // "checks"):$checks,
      ($fields[9] // "issues"):($checks | map(select(.status == "failed") | {code:.id,message:.message}))
    }
  ')"
  sley_report_builder_from_report_kind_json "claim_verify" "$report"
  [[ "$failed" -eq 0 ]]
}

contract_jsonschema() {
  SLEY_CONTRACT_INVENTORY_SCHEMA="$SCHEMA_CONTRACT_INVENTORY" \
  SLEY_CONTRACT_INVENTORY_FIELDS="$CONTRACT_INVENTORY_FIELDS_JSON" \
  SLEY_CONTRACT_VALIDATE_SCHEMA="$SCHEMA_CONTRACT_VALIDATE" \
  SLEY_CONTRACT_VALIDATE_FIELDS="$CONTRACT_VALIDATE_FIELDS_JSON" \
  SLEY_CONTRACT_FIXTURE_CHECK_SCHEMA="$SCHEMA_CONTRACT_FIXTURE_CHECK" \
  SLEY_CONTRACT_FIXTURE_CHECK_FIELDS="$CONTRACT_FIXTURE_CHECK_FIELDS_JSON" \
  SLEY_DEPLOY_ARTIFACTS_SCHEMA="$SCHEMA_DEPLOY_ARTIFACTS" \
  SLEY_DEPLOY_ARTIFACT_CHECK_SCHEMA="$SCHEMA_DEPLOY_ARTIFACT_CHECK" \
  SLEY_DEPLOY_ARTIFACT_CHECK_FIELDS="$DEPLOY_ARTIFACT_CHECK_FIELDS_JSON" \
  python3 - "$@" <<'PY'
import argparse
import hashlib
import json
import os
from pathlib import Path
import sys
import warnings

VALIDATION_LEVEL = "json_schema_draft_2020_12"


def issue(code, message):
    return {"code": code, "message": message}


def env_list(name, default):
    try:
        value = json.loads(os.environ.get(name, "[]"))
    except json.JSONDecodeError:
        value = []
    if not isinstance(value, list) or not all(isinstance(item, str) and item for item in value):
        return default
    return value


CONTRACT_INVENTORY_SCHEMA = os.environ.get("SLEY_CONTRACT_INVENTORY_SCHEMA") or "sley.contract.inventory.v0"
CONTRACT_INVENTORY_FIELDS = env_list(
    "SLEY_CONTRACT_INVENTORY_FIELDS",
    ["schema", "status", "schema_dir", "schema_count", "schemas"],
)
CONTRACT_VALIDATE_SCHEMA = os.environ.get("SLEY_CONTRACT_VALIDATE_SCHEMA") or "sley.contract.validate.v0"
CONTRACT_VALIDATE_FIELDS = env_list(
    "SLEY_CONTRACT_VALIDATE_FIELDS",
    ["schema", "status", "validation_level", "requested_schema", "report_path", "schema_dir", "report_schema", "issues"],
)
CONTRACT_FIXTURE_CHECK_SCHEMA = os.environ.get("SLEY_CONTRACT_FIXTURE_CHECK_SCHEMA") or "sley.contract.fixture_check.v0"
CONTRACT_FIXTURE_CHECK_FIELDS = env_list(
    "SLEY_CONTRACT_FIXTURE_CHECK_FIELDS",
    ["schema", "status", "validation_level", "schema_dir", "fixtures_dir", "fixture_count", "passed_count", "failed_count", "fixtures"],
)
DEPLOY_ARTIFACTS_SCHEMA = os.environ.get("SLEY_DEPLOY_ARTIFACTS_SCHEMA") or "sley.deploy.artifacts.v0"
DEPLOY_ARTIFACT_CHECK_SCHEMA = os.environ.get("SLEY_DEPLOY_ARTIFACT_CHECK_SCHEMA") or "sley.deploy.artifact_check.v0"
DEPLOY_ARTIFACT_CHECK_FIELDS = env_list(
    "SLEY_DEPLOY_ARTIFACT_CHECK_FIELDS",
    ["schema", "status", "validation_level", "artifacts_dir", "manifest_path", "manifest_schema", "summary", "files", "issues"],
)

DEPLOY_ARTIFACT_ROLES = {
    "report": "sley.deploy.report.v0",
    "seal": "sley.trace.seal.v0",
    "package": "sley.zjx.envelope.v0",
}
EMPTY_SHA256 = "sha256:" + ("0" * 64)
MAX_JSON_BYTES = int(os.environ.get("SLEY_MAX_JSON_BYTES", "5242880"))
MAX_SCHEMA_COUNT = int(os.environ.get("SLEY_MAX_SCHEMA_COUNT", "256"))


def ordered_report(fields, values):
    report = {}
    for field in fields:
        if field in values:
            report[field] = values[field]
    for key, value in values.items():
        if key not in report:
            report[key] = value
    return report


def load_json(path):
    if "\n" in str(path) or "\r" in str(path):
        return None, [issue("ambiguous_path", f"{path} contains a newline")]
    try:
        if path.exists() and path.is_file() and path.stat().st_size > MAX_JSON_BYTES:
            return None, [issue("json_file_too_large", f"{path} exceeds {MAX_JSON_BYTES} bytes")]
    except OSError as exc:
        return None, [issue("file_stat_failed", f"{path}: {exc}")]
    try:
        with path.open("r", encoding="utf-8") as handle:
            return json.load(handle), []
    except FileNotFoundError:
        return None, [issue("file_missing", f"{path} does not exist")]
    except json.JSONDecodeError as exc:
        return None, [issue("invalid_json", f"{path}: {exc}")]


def digest_file(path):
    digest = hashlib.sha256()
    with path.open("rb") as handle:
        for chunk in iter(lambda: handle.read(1024 * 1024), b""):
            digest.update(chunk)
    return "sha256:" + digest.hexdigest()


def resolve_artifact_path(artifacts_dir, declared_path):
    declared = Path(declared_path)
    if declared.is_absolute():
        return declared
    candidates = [
        Path.cwd() / declared,
        artifacts_dir / declared,
        artifacts_dir / declared.name,
    ]
    for candidate in candidates:
        if candidate.exists():
            return candidate
    return candidates[0]


def load_schema_inventory(schema_dir):
    store = {}
    issues = []
    inventory = []
    if not schema_dir.exists():
        return store, [issue("schema_dir_missing", f"{schema_dir} does not exist")], inventory
    schema_paths = sorted(schema_dir.glob("*.schema.json"))
    if len(schema_paths) > MAX_SCHEMA_COUNT:
        return store, [issue("schema_count_limit_exceeded", f"{schema_dir} has more than {MAX_SCHEMA_COUNT} schema files")], inventory
    for schema_path in schema_paths:
        schema, schema_issues = load_json(schema_path)
        if schema_issues:
            issues.extend(schema_issues)
            continue
        if not isinstance(schema, dict):
            issues.append(issue("schema_not_object", f"{schema_path} is not a JSON object"))
            continue
        schema_id = schema.get("$id")
        if not isinstance(schema_id, str) or not schema_id:
            issues.append(issue("schema_id_missing", f"{schema_path} has no $id"))
            continue
        store[schema_id] = schema
        inventory.append({
            "id": schema_id,
            "path": str(schema_path),
            "title": schema.get("title", schema_id),
            "draft": schema.get("$schema", ""),
            "root_type": schema.get("type", ""),
        })
    return store, issues, inventory


def iter_refs(value):
    if isinstance(value, dict):
        ref = value.get("$ref")
        if isinstance(ref, str):
            yield ref
        for child in value.values():
            yield from iter_refs(child)
    elif isinstance(value, list):
        for child in value:
            yield from iter_refs(child)


def validate_ref_closure(schema_id, store):
    known = set(store)
    issues = []
    to_visit = [schema_id]
    visited = set()
    while to_visit:
        current_id = to_visit.pop()
        if current_id in visited:
            continue
        visited.add(current_id)
        schema = store.get(current_id)
        if schema is None:
            issues.append(issue("schema_not_found", f"{current_id} was not found in schema inventory"))
            continue
        for ref in iter_refs(schema):
            if ref.startswith("#"):
                continue
            if "://" in ref or ref.startswith("/") or ref.startswith("."):
                issues.append(issue("external_schema_ref_denied", f"{current_id} references disallowed schema ref {ref!r}"))
                continue
            ref_id = ref.split("#", 1)[0]
            if ref_id not in known:
                issues.append(issue("schema_ref_not_found", f"{current_id} references unknown schema {ref!r}"))
                continue
            to_visit.append(ref_id)
    return issues


def pointer(parts):
    parts = list(parts)
    if not parts:
        return "#"
    return "/" + "/".join(str(part).replace("~", "~0").replace("/", "~1") for part in parts)


def validate_instance(schema_id, instance, schema_dir):
    with warnings.catch_warnings():
        warnings.simplefilter("ignore", DeprecationWarning)
        try:
            from jsonschema import Draft202012Validator, RefResolver
        except Exception as exc:
            return [issue("jsonschema_unavailable", f"Python package jsonschema is required for draft-2020-12 validation: {exc}")]
    store, schema_issues, _ = load_schema_inventory(schema_dir)
    if schema_issues:
        return schema_issues
    schema = store.get(schema_id)
    if schema is None:
        return [issue("schema_not_found", f"{schema_id} was not found in {schema_dir}")]
    ref_issues = validate_ref_closure(schema_id, store)
    if ref_issues:
        return ref_issues
    resolver = RefResolver.from_schema(schema, store=store)
    validator = Draft202012Validator(schema, resolver=resolver)
    return [
        issue("json_schema_violation", f"{pointer(error.path)}: {error.message}")
        for error in sorted(validator.iter_errors(instance), key=lambda item: list(item.path))
    ]


def command_inventory(args):
    _, issues, inventory = load_schema_inventory(Path(args.schema_dir))
    if issues:
        for item in issues:
            print(f"{item['code']}: {item['message']}", file=sys.stderr)
        return 1
    print(json.dumps(ordered_report(CONTRACT_INVENTORY_FIELDS, {
        "schema": CONTRACT_INVENTORY_SCHEMA,
        "status": "passed",
        "schema_dir": args.schema_dir,
        "schema_count": len(inventory),
        "schemas": inventory,
    }), indent=2))
    return 0


def command_validate(args):
    instance, load_issues = load_json(Path(args.report_path))
    report_schema = ""
    issues = list(load_issues)
    if isinstance(instance, dict):
        value = instance.get("schema")
        report_schema = value if isinstance(value, str) else ""
    if instance is not None and not issues:
        issues.extend(validate_instance(args.requested_schema, instance, Path(args.schema_dir)))
    status = "passed" if not issues else "failed"
    print(json.dumps(ordered_report(CONTRACT_VALIDATE_FIELDS, {
        "schema": CONTRACT_VALIDATE_SCHEMA,
        "status": status,
        "validation_level": VALIDATION_LEVEL,
        "requested_schema": args.requested_schema,
        "report_path": args.report_path,
        "schema_dir": args.schema_dir,
        "report_schema": report_schema,
        "issues": issues,
    }), indent=2))
    return 0 if status == "passed" else 1


def command_check_fixtures(args):
    fixtures_dir = Path(args.fixtures_dir)
    fixtures = []
    if not fixtures_dir.exists():
        issues = [issue("fixtures_dir_missing", f"{fixtures_dir} does not exist")]
        fixtures.append({"path": args.fixtures_dir, "status": "failed", "issues": issues})
    else:
        for fixture_path in sorted(fixtures_dir.rglob("*.json")):
            instance, load_issues = load_json(fixture_path)
            schema_id = ""
            issues = list(load_issues)
            if isinstance(instance, dict):
                value = instance.get("schema")
                schema_id = value if isinstance(value, str) else ""
                if not schema_id:
                    issues.append(issue("schema_field_missing", f"{fixture_path} has no top-level schema"))
            elif instance is not None:
                issues.append(issue("fixture_not_object", f"{fixture_path} is not a JSON object"))
            if instance is not None and schema_id:
                issues.extend(validate_instance(schema_id, instance, Path(args.schema_dir)))
            fixture = {"path": str(fixture_path), "status": "passed" if not issues else "failed"}
            if schema_id:
                fixture["schema_id"] = schema_id
            fixture["issues"] = issues
            fixtures.append(fixture)
    failed_count = sum(1 for fixture in fixtures if fixture["status"] == "failed")
    print(json.dumps(ordered_report(CONTRACT_FIXTURE_CHECK_FIELDS, {
        "schema": CONTRACT_FIXTURE_CHECK_SCHEMA,
        "status": "passed" if failed_count == 0 else "failed",
        "validation_level": VALIDATION_LEVEL,
        "schema_dir": args.schema_dir,
        "fixtures_dir": args.fixtures_dir,
        "fixture_count": len(fixtures),
        "passed_count": len(fixtures) - failed_count,
        "failed_count": failed_count,
        "fixtures": fixtures,
    }), indent=2))
    return 0 if failed_count == 0 else 1


def valid_digest(value):
    return (
        isinstance(value, str)
        and value.startswith("sha256:")
        and len(value) == len("sha256:") + 64
        and all(char in "0123456789abcdef" for char in value[len("sha256:"):])
    )


def command_inspect_deploy_artifacts(args):
    artifacts_dir = Path(args.artifacts_dir)
    manifest_path = artifacts_dir / "manifest.json"
    manifest, manifest_issues = load_json(manifest_path)
    issues = list(manifest_issues)
    files = []
    manifest_schema = ""

    if isinstance(manifest, dict):
        value = manifest.get("schema")
        manifest_schema = value if isinstance(value, str) else ""
        if manifest_schema != DEPLOY_ARTIFACTS_SCHEMA:
            issues.append(issue("manifest_schema_mismatch", f"{manifest_path} does not use {DEPLOY_ARTIFACTS_SCHEMA}"))
        else:
            issues.extend(validate_instance(DEPLOY_ARTIFACTS_SCHEMA, manifest, Path(args.schema_dir)))
    elif manifest is not None:
        issues.append(issue("manifest_not_object", f"{manifest_path} is not a JSON object"))

    manifest_files = manifest.get("files", {}) if isinstance(manifest, dict) else {}
    if isinstance(manifest_files, dict):
        for role, default_schema in DEPLOY_ARTIFACT_ROLES.items():
            entry = manifest_files.get(role, {})
            file_issues = []
            if not isinstance(entry, dict):
                entry = {}
                file_issues.append(issue("manifest_file_entry_missing", f"manifest has no object for {role}"))

            declared_path = entry.get("path")
            if not isinstance(declared_path, str) or not declared_path:
                declared_path = str(artifacts_dir / {
                    "report": "deploy-report.json",
                    "seal": "seal.json",
                    "package": "zjx-envelope.json",
                }[role])
                file_issues.append(issue("manifest_file_path_missing", f"manifest file entry for {role} has no path"))

            expected_schema = entry.get("schema")
            if expected_schema not in DEPLOY_ARTIFACT_ROLES.values():
                expected_schema = default_schema
                file_issues.append(issue("manifest_file_schema_missing", f"manifest file entry for {role} has no recognized schema"))

            expected_digest = entry.get("digest")
            if not valid_digest(expected_digest):
                expected_digest = EMPTY_SHA256
                file_issues.append(issue("manifest_file_digest_missing", f"manifest file entry for {role} has no valid digest"))

            actual_path = resolve_artifact_path(artifacts_dir, declared_path)
            file_report = {
                "role": role,
                "status": "passed",
                "path": str(actual_path),
                "manifest_path": declared_path,
                "expected_schema": expected_schema,
                "expected_digest": expected_digest,
                "issues": file_issues,
            }

            if not actual_path.exists():
                file_issues.append(issue("artifact_file_missing", f"{declared_path} does not exist"))
            else:
                actual_digest = digest_file(actual_path)
                file_report["actual_digest"] = actual_digest
                if actual_digest != expected_digest:
                    file_issues.append(issue("artifact_digest_mismatch", f"{declared_path} digest does not match manifest"))
                actual_doc, load_issues = load_json(actual_path)
                file_issues.extend(load_issues)
                if isinstance(actual_doc, dict):
                    raw_schema = actual_doc.get("schema")
                    if not isinstance(raw_schema, str):
                        raw_schema = actual_doc.get("source_schema")
                    if raw_schema in DEPLOY_ARTIFACT_ROLES.values():
                        file_report["actual_schema"] = raw_schema
                    if raw_schema != expected_schema:
                        file_issues.append(issue("artifact_schema_mismatch", f"{declared_path} schema does not match manifest"))
                elif actual_doc is not None:
                    file_issues.append(issue("artifact_not_object", f"{declared_path} is not a JSON object"))

            file_report["status"] = "passed" if not file_issues else "failed"
            files.append(file_report)

    for file_report in files:
        for item in file_report["issues"]:
            issues.append(issue(f"{file_report['role']}_{item['code']}", item["message"]))

    failed_count = sum(1 for file_report in files if file_report["status"] == "failed")
    status = "passed" if not issues and failed_count == 0 else "failed"
    values = {
        "schema": DEPLOY_ARTIFACT_CHECK_SCHEMA,
        "status": status,
        "validation_level": VALIDATION_LEVEL,
        "artifacts_dir": args.artifacts_dir,
        "manifest_path": str(manifest_path),
        "summary": {
            "file_count": len(files),
            "passed_count": len(files) - failed_count,
            "failed_count": failed_count,
            "issue_count": len(issues),
        },
        "files": files,
        "issues": issues,
    }
    if manifest_schema == DEPLOY_ARTIFACTS_SCHEMA:
        values["manifest_schema"] = manifest_schema
    print(json.dumps(ordered_report(DEPLOY_ARTIFACT_CHECK_FIELDS, values), indent=2))
    return 0 if status == "passed" else 1


parser = argparse.ArgumentParser()
subparsers = parser.add_subparsers(dest="command", required=True)
inventory = subparsers.add_parser("inventory")
inventory.add_argument("--schema-dir", required=True)
inventory.set_defaults(func=command_inventory)
validate = subparsers.add_parser("validate")
validate.add_argument("--schema-dir", required=True)
validate.add_argument("--schema", dest="requested_schema", required=True)
validate.add_argument("report_path")
validate.set_defaults(func=command_validate)
check = subparsers.add_parser("check-fixtures")
check.add_argument("--schema-dir", required=True)
check.add_argument("fixtures_dir")
check.set_defaults(func=command_check_fixtures)
inspect = subparsers.add_parser("inspect-deploy-artifacts")
inspect.add_argument("--schema-dir", required=True)
inspect.add_argument("artifacts_dir")
inspect.set_defaults(func=command_inspect_deploy_artifacts)
args = parser.parse_args()
raise SystemExit(args.func(args))
PY
}
