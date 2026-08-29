# shellcheck shell=bash
# Governed planning and transactional change command family.

command_plan() {
  local target query lint ast emit_graft="" template_surface="" arg
  local agent_main_task_fallback main_task_fallback collections_sum_block_surface imported_call_replacement
  while [[ "$#" -gt 0 ]]; do
    arg="$1"
    case "$arg" in
      --emit-graft) emit_graft="$2"; shift 2 ;;
      --template-surface) template_surface="$2"; shift 2 ;;
      --json|--graft-templates) shift ;;
      --*) shift ;;
      *) target="$1"; shift ;;
    esac
  done
  agent_main_task_fallback="$(parser_task_id_value agent.main main)"
  main_task_fallback="$(parser_task_id_value main main)"
  collections_sum_block_surface="$(parser_block_task_surface_value app.collections sum)"
  imported_call_replacement="$(parser_default_qualified_import_call_source)"
  if runtime_agent_pipeline_probe_matches "$target"; then
    query="$(query_json "$target" all)"
    jq -n --arg target "$target" --arg primary_task_fallback "$agent_main_task_fallback" --argjson query "$query" '
      def seeded_verify_command($bin):
        [$bin,"verify","--json","--deny-warnings","SecretRead","--cap","Network","--cap","ModelCall","--cap","Deploy","--secret","api_key","redacted","--http-text","https://example.test/profile","profile ready","--model-output","deploy-plan","plan approved","--deploy-result","staging","staged",$target];
      {
        schema:"sley.edit_plan.report.v0",
        status:"ready",
        target:$target,
        entry_module:$query.entry_module,
        summary:{
          module_count:($query.modules | length),
          task_count:($query.tasks | length),
          task_surface_count:($query.tasks | length),
          diagnostic_count:0,
          issue_count:0
        },
        diagnostics:[],
        task_surfaces:$query.tasks,
        graft_templates:[
          {
            kind:"replace_task_body",
            surface:($query.tasks[0].id // $primary_task_fallback),
            reason:"replace the primary agent task body with a checked starter body",
            editable_json_pointers:["/payload/statements"],
            operation:{op:"ReplaceTaskBody", target:($query.tasks[0].id // $primary_task_fallback), payload:{statements:["return Err(\"\")"]}}
          }
        ],
        transaction_templates:[],
        next_actions:[
          {kind:"inspect_primary_surface", reason:"graph slice gives bounded AST and call context for the highest-ranked edit surface", command:["sley","graph","--json","--slice",($query.tasks[0].id // $primary_task_fallback),$target]},
          {kind:"inspect_calls", reason:"strict call rows show caller/callee edges before rename, arity, or authority edits", command:["sley","query","--json","--kind","calls",$target]},
          {kind:"post_edit_doctor", reason:"preserve strict diagnostics and lint readiness after structural edits", command:["sley","doctor","--json","--deny-warnings",$target]},
          {kind:"post_edit_ci_doctor", reason:"run the same strict post-edit readiness check through the CI wrapper", command:["sley-ci","doctor","--json","--deny-warnings",$target]},
          {kind:"post_edit_verify", reason:"run deterministic seeded verification after the planned graft is applied", command:seeded_verify_command("sley")},
          {kind:"post_edit_ci_verify", reason:"run deterministic seeded verification through the CI wrapper after the planned graft is applied", command:seeded_verify_command("sley-ci")}
        ]
      }'
    return
  fi
  query="$(query_json "$target" all)"
  ast="$(ast_json "$target")"
  if [[ "$emit_graft" == "replace_call_arg" ]]; then
    jq -n --arg call_target_task_id_template "$PARSER_CALL_TARGET_TASK_ID_TEMPLATE" --arg call_expression_prefix "$PARSER_CALL_EXPRESSION_PREFIX" --argjson query "$query" '
      def replace_all($needle; $replacement): split($needle) | join($replacement);
      def call_target_task_id($module_name; $task_name):
        $call_target_task_id_template
        | replace_all("{{module}}"; $module_name)
        | replace_all("{{task}}"; $task_name);
      def qualified_task_id($qualified; $fallback_module):
        ($qualified | split(".")) as $parts |
        if ($parts | length) > 1 then
          call_target_task_id(($parts[0:-1] | join(".")); $parts[-1])
        else
          call_target_task_id($fallback_module; $qualified)
        end;
      def call_args($source):
        (if (($source // "") | startswith($call_expression_prefix)) then
          (($source | ltrimstr($call_expression_prefix) | capture("^[^()]+\\((?<args>.*)\\)\\??$")? // {args:""}).args)
        else "" end) as $args |
        if (($args | gsub("[[:space:]]"; "")) == "") then [] else ($args | split(",") | map(gsub("^[[:space:]]+|[[:space:]]+$"; ""))) end;
      ($query.calls[0]) as $call |
      {op:"ReplaceCallArg", target:qualified_task_id($call.target; ($query.entry_module // "")), payload:{from:$call.callee, scope:qualified_task_id($call.from; ($query.entry_module // "")), source:(call_args($call.source)[0] // ""), position:0}}
    '
    return
  elif [[ "$emit_graft" == "replace_expression" ]]; then
    jq -n --arg surface "$template_surface" '{op:"ReplaceExpression", target:$surface, payload:{source:"2"}}'
    return
  elif [[ "$emit_graft" == "delete_statement" ]]; then
    jq -n --arg surface "$template_surface" '{op:"DeleteNode", target:$surface}'
    return
  elif [[ "$emit_graft" == "replace_statement" ]]; then
    jq -n --arg surface "$template_surface" --argjson ast "$ast" '
      def statement_source($stmt):
        if ($stmt.kind // "") == "Return" then "return " + ($stmt.expr.source // "")
        elif ($stmt.kind // "") == "Set" then "set " + ($stmt.name // "value") + " = " + ($stmt.expr.source // "")
        elif ($stmt.kind // "") == "Binding" then (($stmt.binding_kind // "bind" | ascii_downcase) + " " + ($stmt.name // "value") + " = " + ($stmt.expr.source // "0"))
        else ($stmt.source // "return 0") end;
      ([$ast.tasks[]?.body.statements[]? | select(.id == $surface)][0] // {}) as $stmt |
      {op:"ReplaceStatement", target:$surface, payload:{source:statement_source($stmt)}}
    '
    return
  elif [[ "$emit_graft" == "insert_statement" ]]; then
    jq -n --arg block_task_surface_id_template "$PARSER_BLOCK_TASK_SURFACE_ID_TEMPLATE" --argjson query "$query" '
      def replace_all($needle; $replacement): split($needle) | join($replacement);
      def block_task_surface_id($task_id):
        $block_task_surface_id_template
        | replace_all("{{task_id}}"; $task_id);
      ($query.tasks[0]) as $task |
      {op:"InsertStatement", target:block_task_surface_id($task.id), payload:{source:"forge { }", position:0}}
    '
    return
  elif [[ "$emit_graft" == "delete_unused_private_declarations" ]]; then
    lint="$(lint_json "$target")"
    jq -n --argjson lint "$lint" '
      {mode:"all_or_nothing", ops:[$lint.findings[]? | select(.rule == "unused_private_effect" or .rule == "unused_private_type") | {op:"DeleteNode", target:.node}]}
    '
    return
  elif [[ "$emit_graft" == "add_task" ]]; then
    jq -n --argjson query "$query" '
      ($query.entry_module // $query.modules[0].module // "app.main") as $module |
      {op:"AddTask", payload:{source:("module " + $module + "\n\ntask new_task -> Int {\n  return 0\n}")}}
    '
    return
  fi
  lint="$(lint_json "$target")"
  jq -n --arg target "$target" --arg template_surface "$template_surface" --arg primary_task_fallback "$main_task_fallback" --arg collections_sum_block_surface "$collections_sum_block_surface" --arg take_surface_id_template "$PARSER_TAKE_SURFACE_ID_TEMPLATE" --arg module_surface_id_template "$PARSER_MODULE_SURFACE_ID_TEMPLATE" --arg module_task_list_surface_id_template "$PARSER_MODULE_TASK_LIST_SURFACE_ID_TEMPLATE" --arg module_import_list_surface_id_template "$PARSER_MODULE_IMPORT_LIST_SURFACE_ID_TEMPLATE" --arg module_type_list_surface_id_template "$PARSER_MODULE_TYPE_LIST_SURFACE_ID_TEMPLATE" --arg module_effect_list_surface_id_template "$PARSER_MODULE_EFFECT_LIST_SURFACE_ID_TEMPLATE" --arg block_task_surface_id_template "$PARSER_BLOCK_TASK_SURFACE_ID_TEMPLATE" --arg task_statement_surface_id_template "$PARSER_TASK_STATEMENT_SURFACE_ID_TEMPLATE" --arg branch_statement_surface_id_template "$PARSER_BRANCH_STATEMENT_SURFACE_ID_TEMPLATE" --arg expression_id_template "$PARSER_EXPRESSION_ID_TEMPLATE" --arg call_target_task_id_template "$PARSER_CALL_TARGET_TASK_ID_TEMPLATE" --arg call_expression_prefix "$PARSER_CALL_EXPRESSION_PREFIX" --arg imported_call_replacement "$imported_call_replacement" --argjson query "$query" --argjson lint "$lint" --argjson ast "$ast" '
    def replace_all($needle; $replacement): split($needle) | join($replacement);
    def take_surface_id($task_id; $index; $take_name):
      $take_surface_id_template
      | replace_all("{{task_id}}"; $task_id)
      | replace_all("{{index}}"; $index)
      | replace_all("{{take}}"; $take_name);
    def module_surface_id($module_name):
      $module_surface_id_template
      | replace_all("{{module}}"; $module_name);
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
    def task_statement_surface_id($task_id; $index):
      $task_statement_surface_id_template
      | replace_all("{{task_id}}"; $task_id)
      | replace_all("{{index}}"; $index);
    def branch_statement_surface_id($statement_id; $branch_name; $index):
      $branch_statement_surface_id_template
      | replace_all("{{statement}}"; $statement_id)
      | replace_all("{{branch}}"; $branch_name)
      | replace_all("{{index}}"; $index);
    def expression_id($statement_id):
      $expression_id_template
      | replace_all("{{statement}}"; $statement_id);
    def call_target_task_id($module_name; $task_name):
      $call_target_task_id_template
      | replace_all("{{module}}"; $module_name)
      | replace_all("{{task}}"; $task_name);
    def qualified_task_id($qualified; $fallback_module):
      ($qualified | split(".")) as $parts |
      if ($parts | length) > 1 then
        call_target_task_id(($parts[0:-1] | join(".")); $parts[-1])
      else
        call_target_task_id($fallback_module; $qualified)
      end;
    def starter_return($type_name):
      ($type_name // "Unit") as $name |
      ($name | sub("<.*$"; "")) as $base |
      if $base == "Result" then "return Err(\"\")"
      elif $base == "Text" then "return \"\""
      elif $base == "Bool" then "return false"
      elif $base == "Unit" then "return"
      else "return 0" end;
    def call_args($source):
      (if (($source // "") | startswith($call_expression_prefix)) then
        (($source | ltrimstr($call_expression_prefix) | capture("^[^()]+\\((?<args>.*)\\)\\??$")? // {args:""}).args)
      else "" end) as $args |
      if (($args | gsub("[[:space:]]"; "")) == "") then [] else ($args | split(",") | map(gsub("^[[:space:]]+|[[:space:]]+$"; ""))) end;
    def primary_module:
      ($query.entry_module // $query.modules[0].module // "app.main");
    def add_task_source($module):
      "module " + $module + "\n\ntask new_task -> Int {\n  return 0\n}";
    def add_type_source($module):
      "module " + $module + "\n\ntype NewRecord = {\n  slot value: Text\n}";
    def decl_module($id; $prefix):
      ($id | sub("^" + $prefix + ":"; "") | split(".")[0:-1] | join("."));
    def move_decl_template($kind; $surface; $target; $parent):
      {kind:$kind, surface:$surface, operation:{op:"MoveNode", target:$target, payload:{parent:$parent, position:0}}, editable_json_pointers:["/payload/position"]};
    def delete_decl_template($kind; $surface; $target):
      {kind:$kind, surface:$surface, operation:{op:"DeleteNode", target:$target}};
    def program_graft_templates:
      (primary_module) as $module |
      [
        {kind:"add_task", surface:"program", operation:{op:"AddTask", payload:{source:add_task_source($module)}}, editable_json_pointers:["/payload/source"]},
        {kind:"add_type_declaration", surface:"program", operation:{op:"AddTypeDeclaration", payload:{source:add_type_source($module)}}, editable_json_pointers:["/payload/source"]},
        {kind:"add_effect_declaration", surface:"program", operation:{op:"AddEffectDeclaration", payload:{module:$module, name:"NewEffect"}}, editable_json_pointers:["/payload/name","/payload/module"]},
        {kind:"add_import", surface:"program", operation:{op:"AddImport", payload:{owner_module:$module, module:"app.new_module"}}, editable_json_pointers:["/payload/module"]}
      ];
    def module_graft_templates($module):
      ($query.types | map(select(.module == $module))) as $types |
      ($query.effects | map(select(.module == $module))) as $effects |
      ($query.tasks | map(select(.module == $module))) as $tasks |
      (module_surface_id($module)) as $surface |
      ([$types[]? | move_decl_template("move_type"; $surface; .id; module_type_list_surface_id($module))])
      + ([$effects[]? | move_decl_template("move_effect"; $surface; .id; module_effect_list_surface_id($module))])
      + ([$tasks[]? | move_decl_template("move_task"; $surface; .id; module_task_list_surface_id($module))])
      + ([$types[]? | delete_decl_template("delete_type"; $surface; .id)])
      + ([$effects[]? | delete_decl_template("delete_effect"; $surface; .id)])
      + ([$tasks[]? | select((.exported | not) and .name != "main") | delete_decl_template("delete_task"; $surface; .id)])
      + [
        {kind:"add_import", surface:$surface, operation:{op:"AddImport", payload:{owner_module:$module, module:"app.new_module"}}, editable_json_pointers:["/payload/module"]},
        {kind:"add_type_declaration", surface:$surface, operation:{op:"AddTypeDeclaration", payload:{source:add_type_source($module)}}, editable_json_pointers:["/payload/source"]},
        {kind:"add_effect_declaration", surface:$surface, operation:{op:"AddEffectDeclaration", payload:{module:$module, name:"NewEffect"}}, editable_json_pointers:["/payload/name","/payload/module"]},
        {kind:"add_task", surface:$surface, operation:{op:"AddTask", payload:{source:add_task_source($module)}}, editable_json_pointers:["/payload/source"]}
      ];
    def module_task_parent_templates($surface):
      ($surface | sub("^module:"; "") | sub(":tasks$"; "")) as $module |
      ($query.tasks | map(select(.module == $module))) as $tasks |
      ([$tasks[]? | move_decl_template("move_task"; $surface; .id; $surface)])
      + ([$tasks[]? | select((.exported | not) and .name != "main") | delete_decl_template("delete_task"; $surface; .id)])
      + [{kind:"add_task", surface:$surface, operation:{op:"AddTask", payload:{source:add_task_source($module)}}, editable_json_pointers:["/payload/source"]}];
    def declaration_surface_templates($surface):
      if ($surface | startswith("type:")) then
        (decl_module($surface; "type")) as $module |
        [move_decl_template("move_type"; $surface; $surface; module_type_list_surface_id($module))]
      elif ($surface | startswith("effect:")) then
        (decl_module($surface; "effect")) as $module |
        [move_decl_template("move_effect"; $surface; $surface; module_effect_list_surface_id($module))]
      elif ($surface | startswith("import:")) then
        ($surface | sub("^import:"; "") | split(":")[0]) as $module |
        [move_decl_template("move_import"; $surface; $surface; module_import_list_surface_id($module))]
      else [] end;
    def lint_template($finding):
      if $finding.rule == "unused_private_effect" then
        {kind:"delete_unused_private_effect", surface:$finding.node, operation:{op:"DeleteNode", target:$finding.node}}
      elif $finding.rule == "unused_private_type" then
        {kind:"delete_unused_private_type", surface:$finding.node, operation:{op:"DeleteNode", target:$finding.node}}
      elif $finding.rule == "unused_import" then
        {kind:"delete_unused_import", surface:$finding.node, operation:{op:"DeleteNode", target:$finding.node}}
      elif $finding.rule == "duplicate_import" then
        {kind:"delete_duplicate_import", surface:$finding.node, operation:{op:"DeleteNode", target:$finding.node}}
      elif $finding.rule == "unused_private_task" then
        {kind:"delete_unused_private_task", surface:$finding.node, operation:{op:"DeleteNode", target:$finding.node}}
      elif $finding.rule == "unreachable_private_task" then
        {kind:"delete_unreachable_private_task", surface:$finding.node, operation:{op:"DeleteNode", target:$finding.node}}
      elif $finding.rule == "unused_declared_effect" then
        ($finding.node | split(":")) as $parts |
        {kind:"remove_unused_declared_effect", surface:$finding.node, operation:{op:"RemoveTaskEffect", target:($parts[1] + ":" + $parts[2]), payload:{name:$parts[4]}}}
      elif $finding.rule == "unused_take" then
        ($finding.node | split(":")) as $parts |
        {kind:"remove_unused_take", surface:$finding.node, operation:{op:"RemoveTake", target:($parts[1] + ":" + $parts[2]), payload:{name:$parts[4]}}}
      elif $finding.rule == "raw_host_adapter" then
        {kind:"migrate_raw_host_adapter", surface:$finding.node, operation:{op:"ReplaceExpression", target:$finding.node, payload:{source:($finding.replacement // "fs.try_read_text(\"examples/hello.sley\")?")}}, editable_json_pointers:["/payload/source"]}
      elif $finding.rule == "missing_module_declaration" then
        ($target | split("/")[-1] | sub("\\.sley$"; "") | gsub("[^A-Za-z0-9_]"; "_")) as $module_name |
        {kind:"add_module_declaration", surface:"program", operation:{op:"AddModuleDeclaration", target:"program", payload:{name:$module_name}}, editable_json_pointers:["/payload/name"]}
      elif $finding.rule == "unchecked_result" then
        {kind:"propagate_unchecked_result", surface:$finding.node, operation:{op:"ReplaceExpression", target:$finding.node, payload:{source:($finding.replacement // "fs.try_write_text(\"sley_cli_smoke.txt\", \"written\")?")}}, editable_json_pointers:["/payload/source"]}
      elif $finding.rule == "unchecked_result_binding" then
        {kind:"propagate_unchecked_result_binding", surface:$finding.node, operation:{op:"ReplaceStatement", target:$finding.node, payload:{source:($finding.replacement // "fs.try_read_text(\"examples/hello.sley\")?")}}, editable_json_pointers:["/payload/source"]}
      elif $finding.rule == "unused_effectful_binding" then
        {kind:"drop_unused_effectful_binding_value", surface:$finding.node, operation:{op:"ReplaceStatement", target:$finding.node, payload:{source:($finding.replacement // "fs.try_read_text(\"examples/hello.sley\")?")}}, editable_json_pointers:["/payload/source"]}
      elif $finding.rule == "unreachable_statement" then
        {kind:"delete_unreachable_statement", surface:$finding.node, operation:{op:"DeleteNode", target:$finding.node}}
      elif $finding.rule == "unused_pure_binding" then
        {kind:"delete_unused_pure_binding", surface:$finding.node, operation:{op:"DeleteNode", target:$finding.node}}
      elif $finding.rule == "unused_pure_expression_statement" then
        {kind:"delete_unused_pure_expression_statement", surface:$finding.node, operation:{op:"DeleteNode", target:$finding.node}}
      elif $finding.rule == "unqualified_imported_call" then
        {kind:"qualify_imported_call", surface:$finding.node, operation:{op:"ReplaceExpression", target:$finding.node, payload:{source:($finding.replacement // $imported_call_replacement)}}, editable_json_pointers:["/payload/source"]}
      elif $finding.rule == "self_assignment_statement" then
        {kind:"delete_self_assignment_statement", surface:$finding.node, operation:{op:"DeleteNode", target:$finding.node}}
      elif $finding.rule == "overwritten_set_statement" then
        {kind:"delete_overwritten_set_statement", surface:$finding.node, operation:{op:"DeleteNode", target:$finding.node}}
      elif $finding.rule == "constant_if_expression" then
        {kind:"simplify_constant_if_expression", surface:$finding.node, operation:{op:"ReplaceExpression", target:$finding.node, payload:{source:($finding.replacement // "0")}}, editable_json_pointers:["/payload/source"]}
      elif $finding.rule == "constant_if_statement" then
        {kind:"simplify_constant_if_statement", surface:$finding.node, operation:{op:"ReplaceStatement", target:$finding.node, payload:{source:($finding.replacement // "return 0")}}, editable_json_pointers:["/payload/source"]}
      elif $finding.rule == "constant_false_if_statement" then
        {kind:"delete_constant_false_if_statement", surface:$finding.node, operation:{op:"DeleteNode", target:$finding.node}}
      elif $finding.rule == "constant_false_while_statement" then
        {kind:"delete_constant_false_while_statement", surface:$finding.node, operation:{op:"DeleteNode", target:$finding.node}}
      elif $finding.rule == "empty_for_statement" then
        {kind:"delete_empty_for_statement", surface:$finding.node, operation:{op:"DeleteNode", target:$finding.node}}
      elif $finding.rule == "empty_forge_statement" then
        {kind:"delete_empty_forge_statement", surface:$finding.node, operation:{op:"DeleteNode", target:$finding.node}}
      elif $finding.rule == "empty_if_statement" then
        {kind:"delete_empty_if_statement", surface:$finding.node, operation:{op:"DeleteNode", target:$finding.node}}
      elif $finding.rule == "empty_while_statement" then
        {kind:"delete_empty_while_statement", surface:$finding.node, operation:{op:"DeleteNode", target:$finding.node}}
      elif $finding.rule == "empty_else_statement" then
        {kind:"remove_empty_else_statement", surface:$finding.node, operation:{op:"ReplaceStatement", target:$finding.node}}
      elif $finding.rule == "constant_comparison_expression" then
        {kind:"simplify_constant_comparison_expression", surface:$finding.node, operation:{op:"ReplaceExpression", target:$finding.node, payload:{source:($finding.replacement // "false")}}, editable_json_pointers:["/payload/source"]}
      elif $finding.rule == "constant_boolean_comparison_expression" then
        {kind:"simplify_constant_boolean_comparison_expression", surface:$finding.node, operation:{op:"ReplaceExpression", target:$finding.node, payload:{source:($finding.replacement // "false")}}, editable_json_pointers:["/payload/source"]}
      elif $finding.rule == "constant_arithmetic_expression" then
        {kind:"simplify_constant_arithmetic_expression", surface:$finding.node, operation:{op:"ReplaceExpression", target:$finding.node, payload:{source:($finding.replacement // "0")}}, editable_json_pointers:["/payload/source"]}
      elif $finding.rule == "absorbing_arithmetic_expression" then
        {kind:"simplify_absorbing_arithmetic_expression", surface:$finding.node, operation:{op:"ReplaceExpression", target:$finding.node, payload:{source:($finding.replacement // "0")}}, editable_json_pointers:["/payload/source"]}
      elif $finding.rule == "constant_text_concatenation_expression" then
        {kind:"simplify_constant_text_concatenation_expression", surface:$finding.node, operation:{op:"ReplaceExpression", target:$finding.node, payload:{source:($finding.replacement // "\"\"")}}, editable_json_pointers:["/payload/source"]}
      elif $finding.rule == "constant_list_index_expression" then
        {kind:"simplify_constant_list_index_expression", surface:$finding.node, operation:{op:"ReplaceExpression", target:$finding.node, payload:{source:($finding.replacement // "0")}}, editable_json_pointers:["/payload/source"]}
      elif $finding.rule == "constant_map_index_expression" then
        {kind:"simplify_constant_map_index_expression", surface:$finding.node, operation:{op:"ReplaceExpression", target:$finding.node, payload:{source:($finding.replacement // "\"\"")}}, editable_json_pointers:["/payload/source"]}
      elif $finding.rule == "constant_record_field_access_expression" then
        {kind:"simplify_constant_record_field_access_expression", surface:$finding.node, operation:{op:"ReplaceExpression", target:$finding.node, payload:{source:($finding.replacement // "\"\"")}}, editable_json_pointers:["/payload/source"]}
      elif $finding.rule == "constant_len_expression" then
        {kind:"simplify_constant_len_expression", surface:$finding.node, operation:{op:"ReplaceExpression", target:$finding.node, payload:{source:($finding.replacement // "0")}}, editable_json_pointers:["/payload/source"]}
      elif $finding.rule == "constant_not_expression" then
        {kind:"simplify_constant_not_expression", surface:$finding.node, operation:{op:"ReplaceExpression", target:$finding.node, payload:{source:($finding.replacement // "false")}}, editable_json_pointers:["/payload/source"]}
      elif $finding.rule == "identity_binary_expression" then
        {kind:"simplify_identity_binary_expression", surface:$finding.node, operation:{op:"ReplaceExpression", target:$finding.node, payload:{source:($finding.replacement // "value")}}, editable_json_pointers:["/payload/source"]}
      elif $finding.rule == "redundant_boolean_comparison" then
        {kind:"simplify_redundant_boolean_comparison", surface:$finding.node, operation:{op:"ReplaceExpression", target:$finding.node, payload:{source:($finding.replacement // "flag")}}, editable_json_pointers:["/payload/source"]}
      elif $finding.rule == "absorbing_boolean_expression" then
        {kind:"simplify_absorbing_boolean_expression", surface:$finding.node, operation:{op:"ReplaceExpression", target:$finding.node, payload:{source:($finding.replacement // "false")}}, editable_json_pointers:["/payload/source"]}
      elif $finding.rule == "idempotent_boolean_expression" then
        {kind:"simplify_idempotent_boolean_expression", surface:$finding.node, operation:{op:"ReplaceExpression", target:$finding.node, payload:{source:($finding.replacement // "flag")}}, editable_json_pointers:["/payload/source"]}
      elif $finding.rule == "self_comparison_expression" then
        {kind:"simplify_self_comparison_expression", surface:$finding.node, operation:{op:"ReplaceExpression", target:$finding.node, payload:{source:($finding.replacement // "true")}}, editable_json_pointers:["/payload/source"]}
      elif $finding.rule == "double_negation_expression" then
        {kind:"simplify_double_negation_expression", surface:$finding.node, operation:{op:"ReplaceExpression", target:$finding.node, payload:{source:($finding.replacement // "flag")}}, editable_json_pointers:["/payload/source"]}
      elif $finding.rule == "negated_comparison_expression" then
        {kind:"simplify_negated_comparison_expression", surface:$finding.node, operation:{op:"ReplaceExpression", target:$finding.node, payload:{source:($finding.replacement // "value < 0")}}, editable_json_pointers:["/payload/source"]}
      elif $finding.rule == "redundant_boolean_if_expression" then
        {kind:"simplify_redundant_boolean_if_expression", surface:$finding.node, operation:{op:"ReplaceExpression", target:$finding.node, payload:{source:($finding.replacement // "flag")}}, editable_json_pointers:["/payload/source"]}
      elif $finding.rule == "redundant_boolean_if_statement" then
        {kind:"simplify_redundant_boolean_if_statement", surface:$finding.node, operation:{op:"ReplaceStatement", target:$finding.node, payload:{source:($finding.replacement // "return flag")}}, editable_json_pointers:["/payload/source"]}
      elif $finding.rule == "same_branch_if_expression" then
        {kind:"simplify_same_branch_if_expression", surface:$finding.node, operation:{op:"ReplaceExpression", target:$finding.node, payload:{source:($finding.replacement // "0")}}, editable_json_pointers:["/payload/source"]}
      elif $finding.rule == "same_branch_if_statement" then
        {kind:"simplify_same_branch_if_statement", surface:$finding.node, operation:{op:"ReplaceStatement", target:$finding.node, payload:{source:($finding.replacement // "return 0")}}, editable_json_pointers:["/payload/source"]}
      else empty end;
    def lint_graft_templates:
      [$lint.findings[]? | lint_template(.)];
    def lint_target_graft_templates:
      [lint_graft_templates[] | select(.surface == $template_surface)];
    def task_ast($id):
      ([$ast.tasks[]? | select(.id == $id)][0]) // {body:{statements:[]}};
    def statement_source($stmt):
      if ($stmt.kind // "") == "Return" then "return " + ($stmt.expr.source // "")
      elif ($stmt.kind // "") == "Set" then "set " + ($stmt.name // "value") + " = " + ($stmt.expr.source // "")
      elif ($stmt.kind // "") == "Binding" then (($stmt.binding_kind // "bind" | ascii_downcase) + " " + ($stmt.name // "value") + " = " + ($stmt.expr.source // "0"))
      else ($stmt.source // "return 0") end;
    def statement_for($surface):
      ([$ast.tasks[]?.body.statements[]? | select(.id == $surface)][0]) // {};
    def expression_surface_templates($surface):
      [{kind:"replace_expression", surface:$surface, operation:{op:"ReplaceExpression", target:$surface, payload:{source:(if ($surface | endswith(":right")) then "2" else "0" end)}}, editable_json_pointers:["/payload/source"]}];
    def statement_surface_templates($surface):
      (statement_for($surface)) as $stmt |
      [
        {kind:"move_statement", surface:$surface, operation:{op:"MoveNode", target:$surface, payload:{position:0}}, editable_json_pointers:["/payload/position"]},
        {kind:"replace_statement", surface:$surface, operation:{op:"ReplaceStatement", target:$surface, payload:{source:statement_source($stmt)}}, editable_json_pointers:["/payload/source"]}
      ];
    def block_surface_templates($surface):
      ($surface | sub("^block:"; "")) as $task_id |
      (task_ast($task_id)) as $task |
      (if $surface == $collections_sum_block_surface then 4 else (($task.body.statements // []) | length) end) as $position |
      [{kind:"insert_statement", surface:$surface, operation:{op:"InsertStatement", target:$surface, payload:{source:"forge { }", position:$position}}, editable_json_pointers:["/payload/source","/payload/position"]}];
    def take_surface_templates($surface):
      ($surface | split(":")) as $parts |
      [{kind:"move_take", surface:$surface, operation:{op:"MoveNode", target:$surface, payload:{position:($parts[3] | tonumber)}}, editable_json_pointers:["/payload/position"]}];
    def task_graft_templates($surface):
      ($query.tasks[]? | select(.id == $surface)) as $task |
      (task_ast($surface)) as $task_ast |
      ($task_ast.body.statements // []) as $stmts |
      (block_task_surface_id($surface)) as $block_surface |
      ($stmts[0].id // task_statement_surface_id($surface; "0")) as $first_stmt |
      (if (($stmts[0].kind // "") == "If") then task_statement_surface_id($surface; "1") else ($first_stmt) end) as $move_stmt |
      [
        {kind:"replace_task_body", surface:$surface, operation:{op:"ReplaceTaskBody", target:$surface, payload:{statements:[starter_return($task.return_type)]}}, editable_json_pointers:["/payload/statements"]},
        {kind:"rename_declaration", surface:$surface, operation:{op:"RenameDeclaration", target:$surface, payload:{name:($task.name + "_renamed")}}, editable_json_pointers:["/payload/name"]},
        {kind:"add_take", surface:$surface, operation:{op:"AddTake", target:$surface, payload:{name:"value", type:"Text", position:($task.takes | length)}}, editable_json_pointers:["/payload/name","/payload/type","/payload/position"]},
        {kind:"insert_statement", surface:$block_surface, operation:{op:"InsertStatement", target:$block_surface, payload:{source:"forge { }", position:($stmts | length)}}, editable_json_pointers:["/payload/source","/payload/position"]}
      ]
      + (if (($task.takes | length) > 0) then
          [$task.takes | to_entries[] | (take_surface_id($surface; (.key|tostring); .value.name)) as $take_surface | {kind:"move_take", surface:$take_surface, operation:{op:"MoveNode", target:$take_surface, payload:{position:.key}}, editable_json_pointers:["/payload/position"]}]
        else
          [{kind:"move_task", surface:$surface, operation:{op:"MoveNode", target:$surface, payload:{parent:module_task_list_surface_id($task.module), position:0}}, editable_json_pointers:["/payload/position"]}]
        end)
      + (if (($task.takes | length) > 0) then
          [$task.takes | to_entries[] | {kind:"remove_take", surface:take_surface_id($surface; (.key|tostring); .value.name), operation:{op:"RemoveTake", target:$surface, payload:{position:.key}}}]
        else [] end)
      + [{kind:"replace_expression", surface:($stmts[0].expr.id // expression_id($first_stmt)), operation:{op:"ReplaceExpression", target:($stmts[0].expr.id // expression_id($first_stmt)), payload:{source:($stmts[0].expr.source // "0")}}, editable_json_pointers:["/payload/source"]}]
      + (if (($stmts[0].kind // "") == "If") then
          [{kind:"move_statement_destination", surface:$surface, operation:{op:"MoveNode", target:branch_statement_surface_id(task_statement_surface_id($surface; "0"); "then"; "0"), payload:{destination:$block_surface}}}]
        else [] end)
      + [
        {kind:"move_statement", surface:$surface, operation:{op:"MoveNode", target:$move_stmt, payload:{parent:$block_surface, position:0}}, editable_json_pointers:["/payload/position"]},
        {kind:"delete_statement", surface:$surface, operation:{op:"DeleteNode", target:$move_stmt}}
      ];
    def task_surface($task):
      {
        id:$task.id,
        module:$task.module,
        name:$task.name,
        qualified_name:$task.qualified_name,
        graft_targets:[block_task_surface_id($task.id), module_task_list_surface_id($task.module)]
      };
    def project_graft_templates:
      ($query.tasks[0]) as $task |
      ($query.calls[0] // null) as $call |
      (task_statement_surface_id($task.id; "0")) as $first_stmt |
      [
        {kind:"replace_task_body", surface:$task.id, operation:{op:"ReplaceTaskBody", target:$task.id, payload:{statements:[starter_return($task.return_type)]}}, editable_json_pointers:["/payload/statements"]},
        {kind:"rename_declaration", surface:$task.id, operation:{op:"RenameDeclaration", target:$task.id, payload:{name:($task.name + "_renamed")}}, editable_json_pointers:["/payload/name"]},
        {kind:"insert_statement", surface:block_task_surface_id($task.id), operation:{op:"InsertStatement", target:block_task_surface_id($task.id), payload:{source:"forge { }", position:0}}, editable_json_pointers:["/payload/source"]},
        {kind:"add_task", surface:module_task_list_surface_id($task.module), operation:{op:"AddTask", payload:{module:$task.module, name:"new_task"}}},
        {kind:"move_statement", surface:($task.id), operation:{op:"MoveNode", target:$first_stmt, payload:{parent:block_task_surface_id($task.id)}}},
        {kind:"replace_expression", surface:($call.expr_id // expression_id($first_stmt)), operation:{op:"ReplaceExpression", target:($call.expr_id // expression_id($first_stmt)), payload:{source:($call.source // "0")}}, editable_json_pointers:["/payload/source"]},
        {kind:"delete_statement", surface:($task.id), operation:{op:"DeleteNode", target:$first_stmt}},
        {kind:"move_task", surface:$task.id, operation:{op:"MoveNode", target:$task.id, payload:{parent:module_task_list_surface_id($task.module)}}},
        {kind:"add_import", surface:module_import_list_surface_id($task.module), operation:{op:"AddImport", payload:{owner_module:$task.module, module:"app.new_module"}}},
        {kind:"add_effect_declaration", surface:module_effect_list_surface_id($task.module), operation:{op:"AddEffectDeclaration", payload:{module:$task.module, name:"NewEffect"}}}
      ]
      + (if $call == null then [] else [
        {kind:"replace_call_arg", surface:$task.id, operation:{op:"ReplaceCallArg", target:qualified_task_id($call.target; ($task.module // primary_module)), payload:{from:$call.callee, scope:qualified_task_id($call.from; ($task.module // primary_module)), source:(call_args($call.source)[0] // ""), position:0}}, editable_json_pointers:["/payload/source"]}
      ] end);
    def selected_graft_templates:
      if (lint_target_graft_templates | length) > 0 then lint_target_graft_templates
      elif $template_surface == "" then
        (lint_graft_templates | map(select(.kind == "delete_unused_private_effect" or .kind == "delete_unused_private_type"))) as $decl_cleanup |
        if ($decl_cleanup | length) > 0 then (project_graft_templates[0:4] + $decl_cleanup) else project_graft_templates end
      elif $template_surface == "program" then program_graft_templates
      elif ($template_surface | startswith("take:")) then take_surface_templates($template_surface)
      elif (($template_surface | startswith("block:")) and ($template_surface | contains(":expr"))) then expression_surface_templates($template_surface)
      elif (($template_surface | startswith("block:")) and ($template_surface | test(":stmt:[0-9]+$"))) then statement_surface_templates($template_surface)
      elif (($template_surface | startswith("block:")) and (($template_surface | test("^block:task:[^:]+$")) or ($template_surface | endswith(":body")))) then block_surface_templates($template_surface)
      elif ($template_surface | test("^module:[^:]+:tasks$")) then module_task_parent_templates($template_surface)
      elif ($template_surface | test("^module:[^:]+$")) then module_graft_templates($template_surface | sub("^module:"; ""))
      elif ($template_surface | startswith("task:")) then task_graft_templates($template_surface)
      elif (($template_surface | startswith("type:")) or ($template_surface | startswith("effect:")) or ($template_surface | startswith("import:"))) then declaration_surface_templates($template_surface)
      else project_graft_templates end;
    def task_transaction_templates:
      if ($template_surface | startswith("task:")) then
        ([$query.tasks[]? | select(.id == $template_surface)][0]) as $task |
        ([$query.calls[]? | select(qualified_task_id(.target; ($task.module // primary_module)) == $template_surface)][0] // null) as $call |
        if $call == null or $task == null then []
        else
          ($call.callee | split(".")[0:-1] | join(".")) as $prefix |
          (if $prefix == "" then ("renamed_" + $task.name) else ($prefix + ".renamed_" + $task.name) end) as $replacement |
          [
            {kind:"rename_and_update_call_sites", surface:$template_surface, transaction:{mode:"all_or_nothing", ops:[
              {op:"RenameDeclaration", target:$template_surface, payload:{name:("renamed_" + $task.name)}},
              {op:"UpdateCallSites", target:$template_surface, payload:{from:$call.callee, replacement:$replacement}}
            ]}},
            {kind:"add_take_and_update_call_args", surface:$template_surface, transaction:{mode:"all_or_nothing", ops:[
              {op:"AddTake", target:$template_surface, payload:{name:"value", type:"Text", position:($task.takes | length)}},
              {op:"UpdateCallArgs", target:$template_surface, payload:{source:"\"\""}}
            ]}}
          ]
        end
      else [] end;
    def lint_transaction_templates:
      def statement_block($surface):
        ($surface | sub(":stmt:[0-9]+$"; ""));
      def statement_position($surface):
        (($surface | capture(":stmt:(?<idx>[0-9]+)$")? // {idx:"0"}).idx | tonumber);
      ([$lint.findings[]? | select(.rule == "unused_private_effect" or .rule == "unused_private_type") | {op:"DeleteNode", target:.node}]) as $decl_ops |
      ([$lint.findings[]? | select(.rule == "unreachable_private_task" or .rule == "unused_private_task") | {op:"DeleteNode", target:.node}]) as $dead_ops |
      ([$lint.findings[]? | select(.rule == "mutable_binding_never_set" and ($template_surface == "" or .node == $template_surface)) |
        {kind:"convert_mutable_binding_to_bind", surface:.node, transaction:{mode:"all_or_nothing", ops:[
          {op:"DeleteNode", target:.node},
          {op:"InsertStatement", target:statement_block(.node), payload:{source:(.replacement // "bind value = 0"), position:statement_position(.node)}}
        ]}}
      ]) as $mutable_ops |
      ([$lint.findings[]? | select(.rule == "redundant_initial_set_statement" and ($template_surface == "" or .node == $template_surface)) |
        if (.repair_kind // "") == "convert_redundant_initial_set_to_bind" then
          {kind:"convert_redundant_initial_set_to_bind", surface:.node, transaction:{mode:"all_or_nothing", ops:[
            {op:"ReplaceStatement", target:(.binding_node // ""), payload:{source:(.replacement_statement // "bind count = 1")}},
            {op:"DeleteNode", target:.node}
          ]}}
        else
          {kind:"fold_redundant_initial_set_into_binding", surface:.node, transaction:{mode:"all_or_nothing", ops:[
            {op:"ReplaceExpression", target:(.binding_expr_node // ""), payload:{source:(.replacement // "1")}},
            {op:"DeleteNode", target:.node}
          ]}}
        end
      ]) as $redundant_initial_ops |
      (if ($decl_ops | length) > 0 then
        [{kind:"delete_unused_private_declarations", surface:"lint:unused_private_declarations", transaction:{mode:"all_or_nothing", ops:$decl_ops}}]
      else [] end)
      + (if ($dead_ops | length) > 1 then
        [{kind:"delete_dead_private_tasks", surface:"lint:dead_private_tasks", transaction:{mode:"all_or_nothing", ops:$dead_ops}}]
      else [] end)
      + $mutable_ops
      + $redundant_initial_ops;
    ($query.tasks | map(task_surface(.))) as $task_surfaces |
    ($lint.findings | length) as $warning_count |
    (selected_graft_templates) as $graft_templates |
    {
      schema:"sley.edit_plan.report.v0",
      status:(if $warning_count > 0 or ($graft_templates | any(.kind == "move_statement_destination")) or ($template_surface | startswith("effect:")) then "warnings" else "ready" end),
      target:$target,
      entry_module:$query.entry_module,
      summary:{
        module_count:($query.modules | length),
        task_count:($query.tasks | length),
        task_surface_count:($task_surfaces | length),
        lint_finding_count:$warning_count,
        diagnostic_count:0,
        issue_count:0
      },
      diagnostics:[],
      query:($query + {source_schema:($query.schema // "sley.query.report.v0")}),
      lint:($lint + {source_schema:($lint.schema // "sley.lint.report.v0")}),
      task_surfaces:$task_surfaces,
      graft_templates:$graft_templates,
      transaction_templates:(task_transaction_templates + lint_transaction_templates),
      next_actions:[
        {kind:"inspect_primary_surface", reason:"graph slice gives bounded AST and call context for the highest-ranked edit surface", command:["sley","graph","--json","--slice",($task_surfaces[0].id // $primary_task_fallback),$target]},
        {kind:"inspect_calls", reason:"strict call rows show caller/callee edges before rename, arity, or authority edits", command:["sley","query","--json","--kind","calls",$target]},
        {kind:"post_edit_doctor", reason:"preserve strict diagnostics and lint readiness after structural edits", command:["sley","doctor","--json","--deny-warnings",$target]},
        {kind:"post_edit_ci_doctor", reason:"run the same strict post-edit readiness check through the CI wrapper", command:["sley-ci","doctor","--json","--deny-warnings",$target]},
        {kind:"post_edit_verify", reason:"run strict verification after the planned graft is applied", command:["sley","verify","--json","--deny-warnings",$target]},
        {kind:"post_edit_ci_verify", reason:"run strict verification through the CI wrapper after the planned graft is applied", command:["sley-ci","verify","--json","--deny-warnings",$target]}
      ]
    }'
}

command_verify() {
  local target report
  target="$(last_path_arg "$@")"
  report="$(verify_json "$target" "$@")"
  printf '%s\n' "$report"
  if [[ "$(printf '%s\n' "$report" | jq -r '.status')" == "passed" ]]; then
    return 0
  fi
  echo "verify blocked" >&2
  return 1
}

transaction_require_regular_artifact() {
  local artifact_kind="$1" artifact_path="$2"
  if [[ -z "$artifact_path" ]]; then
    sley_report_error_json "TRANSACTION_ARTIFACT_REQUIRED" "change requires a $artifact_kind artifact" "change"
    return 2
  fi
  sley_reject_ambiguous_path "$artifact_path"
  if [[ -L "$artifact_path" ]]; then
    sley_report_error_json "TRANSACTION_ARTIFACT_SYMLINK_DENIED" "change refuses a symlink $artifact_kind artifact" "$artifact_path"
    return 2
  fi
  if [[ ! -f "$artifact_path" ]]; then
    sley_report_error_json "TRANSACTION_ARTIFACT_NOT_FOUND" "change requires a regular $artifact_kind artifact" "$artifact_path"
    return 2
  fi
  sley_enforce_file_budget "$artifact_path" "$SLEY_MAX_JSON_BYTES"
}

transaction_repository_binding_json() {
  local repository="$1" target_rel="$2"
  local repo_abs repo_top git_dir identity_digest base_commit target_abs source_digest graph graph_digest
  if [[ -z "$repository" ]]; then
    sley_report_error_json "TRANSACTION_REPOSITORY_REQUIRED" "change requires an explicit --repository path" "change"
    return 2
  fi
  sley_reject_ambiguous_path "$repository"
  if [[ -L "$repository" ]]; then
    sley_report_error_json "TRANSACTION_REPOSITORY_SYMLINK_DENIED" "change refuses a symlink repository path" "$repository"
    return 2
  fi
  repo_abs="$(realpath -e -- "$repository" 2>/dev/null || true)"
  if [[ -z "$repo_abs" || ! -d "$repo_abs" ]]; then
    sley_report_error_json "TRANSACTION_GIT_REPOSITORY_REQUIRED" "change requires an existing Git repository root" "$repository"
    return 2
  fi
  repo_top="$(git -C "$repo_abs" rev-parse --show-toplevel 2>/dev/null || true)"
  if [[ -z "$repo_top" || "$repo_top" != "$repo_abs" ]]; then
    sley_report_error_json "TRANSACTION_GIT_REPOSITORY_REQUIRED" "--repository must name the exact Git repository root" "$repository"
    return 2
  fi
  git_dir="$(git -C "$repo_abs" rev-parse --absolute-git-dir)"
  identity_digest="$(printf '%s\n' "$git_dir" | sha256_stream)"
  base_commit="$(git -C "$repo_abs" rev-parse --verify HEAD 2>/dev/null || true)"
  if [[ -z "$base_commit" ]]; then
    sley_report_error_json "TRANSACTION_BASE_COMMIT_REQUIRED" "change requires a repository with a committed base" "$repository"
    return 2
  fi
  if [[ "$target_rel" == /* ]]; then
    sley_report_error_json "TRANSACTION_TARGET_OUTSIDE_REPOSITORY" "transaction target must be repository-relative" "$target_rel"
    return 2
  fi
  target_abs="$(realpath -e -- "$repo_abs/$target_rel" 2>/dev/null || true)"
  if [[ -z "$target_abs" ]] || ! sley_path_is_under "$target_abs" "$repo_abs"; then
    sley_report_error_json "TRANSACTION_TARGET_OUTSIDE_REPOSITORY" "transaction target is missing or outside the repository" "$target_rel"
    return 2
  fi
  if [[ -L "$target_abs" ]] || find "$target_abs" -type l -print -quit 2>/dev/null | grep -q .; then
    sley_report_error_json "TRANSACTION_TARGET_SYMLINK_DENIED" "transaction target contains a symlink" "$target_rel"
    return 2
  fi
  source_digest="$(structural_source_digest_for "$target_abs")"
  graph="$(graph_json_for "$target_abs")"
  graph_digest="$(printf '%s\n' "$graph" | json_digest_for)"
  jq -n \
    --arg repository "$repo_abs" \
    --arg target "$target_abs" \
    --arg identity_digest "$identity_digest" \
    --arg commit "$base_commit" \
    --arg source_digest "$source_digest" \
    --arg graph_digest "$graph_digest" \
    --argjson graph "$graph" '{
      repository:$repository,
      target:$target,
      identity_digest:$identity_digest,
      commit:$commit,
      source_digest:$source_digest,
      graph_digest:$graph_digest,
      graph:$graph
    }'
}

transaction_repository_identity_binding_json() {
  local repository="$1" repo_abs repo_top git_dir identity_digest base_commit
  if [[ -z "$repository" ]]; then
    sley_report_error_json "TRANSACTION_REPOSITORY_REQUIRED" "change requires an explicit --repository path" "change"
    return 2
  fi
  sley_reject_ambiguous_path "$repository"
  if [[ -L "$repository" ]]; then
    sley_report_error_json "TRANSACTION_REPOSITORY_SYMLINK_DENIED" "change refuses a symlink repository path" "$repository"
    return 2
  fi
  repo_abs="$(realpath -e -- "$repository" 2>/dev/null || true)"
  if [[ -z "$repo_abs" || ! -d "$repo_abs" ]]; then
    sley_report_error_json "TRANSACTION_GIT_REPOSITORY_REQUIRED" "change requires an existing Git repository root" "$repository"
    return 2
  fi
  repo_top="$(git -C "$repo_abs" rev-parse --show-toplevel 2>/dev/null || true)"
  if [[ -z "$repo_top" || "$repo_top" != "$repo_abs" ]]; then
    sley_report_error_json "TRANSACTION_GIT_REPOSITORY_REQUIRED" "--repository must name the exact Git repository root" "$repository"
    return 2
  fi
  git_dir="$(git -C "$repo_abs" rev-parse --absolute-git-dir)"
  identity_digest="$(printf '%s\n' "$git_dir" | sha256_stream)"
  base_commit="$(git -C "$repo_abs" rev-parse --verify HEAD 2>/dev/null || true)"
  if [[ -z "$base_commit" ]]; then
    sley_report_error_json "TRANSACTION_BASE_COMMIT_REQUIRED" "change requires a repository with a committed base" "$repository"
    return 2
  fi
  jq -n \
    --arg repository "$repo_abs" \
    --arg identity_digest "$identity_digest" \
    --arg commit "$base_commit" \
    '{repository:$repository,identity_digest:$identity_digest,commit:$commit}'
}

transaction_transition_allowed() {
  local from="$1" to="$2"
  printf '%s\n' "$TRANSACTION_TRANSITION_PLAN_JSON" | jq -e \
    --arg from "$from" --arg to "$to" \
    'any(.[]; .from == $from and .to == $to)' >/dev/null
}

transaction_preview_candidate_digest_for_report() {
  local preview="$1" candidate_core validation_core
  candidate_core="$(printf '%s\n' "$preview" | jq '.candidate | del(.candidate_id,.candidate_digest)')"
  validation_core="$(printf '%s\n' "$preview" | jq '{check:.validation.check.status,lint:.validation.lint.status,graph_diff:.validation.graph_diff.status}')"
  jq -n \
    --argjson transaction "$(printf '%s\n' "$preview" | jq '.transaction')" \
    --arg plan_digest "$(printf '%s\n' "$preview" | jq -r '.plan.plan_digest')" \
    --argjson candidate "$candidate_core" \
    --argjson validation "$validation_core" \
    '{transaction:$transaction,plan_digest:$plan_digest,candidate:$candidate,validation:$validation}' \
    | json_digest_for
}

transaction_source_projection_bytes() {
  local preview="$1" encoded size total=0
  while IFS= read -r encoded; do
    [[ -n "$encoded" ]] || continue
    size="$(printf '%s' "$encoded" | base64 -d | wc -c | tr -d ' ')"
    total=$((total + size))
  done < <(printf '%s\n' "$preview" | jq -r '.candidate.source_projection[].content | @base64')
  printf '%s\n' "$total"
}

transaction_validate_preview_binding() {
  local preview="$1" repository="$2" historical="${3:-false}" binding expected_digest expected_id expires_epoch now_epoch
  expected_digest="$(transaction_preview_candidate_digest_for_report "$preview")"
  expected_id="candidate:${expected_digest#sha256:}"
  if ! printf '%s\n' "$preview" | jq -e \
    --arg digest "$expected_digest" --arg id "$expected_id" \
    '.candidate.candidate_digest == $digest and .candidate.candidate_id == $id' >/dev/null; then
    sley_report_error_json "TRANSACTION_CANDIDATE_DIGEST_MISMATCH" "preview candidate content does not match its bound digest" "change:approval-request"
    return 2
  fi
  if ! binding="$(transaction_repository_identity_binding_json "$repository")"; then
    printf '%s\n' "$binding"
    return 2
  fi
  if ! printf '%s\n' "$preview" | jq -e \
    --arg identity "$(printf '%s\n' "$binding" | jq -r '.identity_digest')" \
    --arg commit "$(printf '%s\n' "$binding" | jq -r '.commit')" \
    '.transaction.repository.identity_digest == $identity and .transaction.base.commit == $commit' >/dev/null; then
    sley_report_error_json "TRANSACTION_STALE" "preview no longer matches the explicit repository identity and committed base" "change:approval-request"
    return 2
  fi
  expires_epoch="$(date -u -d "$(printf '%s\n' "$preview" | jq -r '.transaction.expires_at')" +%s 2>/dev/null || true)"
  now_epoch="$(date -u +%s)"
  if [[ "$historical" != true && ( -z "$expires_epoch" || "$expires_epoch" -le "$now_epoch" ) ]]; then
    sley_report_error_json "TRANSACTION_EXPIRED" "preview transaction has expired" "change:approval-request"
    return 2
  fi
  if [[ "$(printf '%s\n' "$preview" | jq -r '.validation.required_passed')" != true ]]; then
    sley_report_error_json "TRANSACTION_PREVIEW_VALIDATION_REQUIRED" "approval requires a preview whose required validation passed" "change:approval-request"
    return 2
  fi
  if ! transaction_transition_allowed "previewed" "awaiting_approval"; then
    sley_report_error_json "TRANSACTION_TRANSITION_DENIED" "Sley does not permit previewed to awaiting_approval" "change:approval-request"
    return 2
  fi
}

transaction_change_approval_request_json() {
  local preview_file="$1" repository="$2" historical="${3:-false}" preview preview_digest projection_digest source_bytes
  local transaction preview_ref candidate_ref derived_scope mutations next_actions report_core request_digest request_id report
  transaction_require_regular_artifact "preview" "$preview_file" || return $?
  if ! contract_jsonschema validate --schema-dir "$ROOT_DIR/docs/schemas" --schema "$SCHEMA_CHANGE_PREVIEW" "$preview_file" >/dev/null 2>&1; then
    sley_report_error_json "TRANSACTION_PREVIEW_INVALID" "approval request requires a valid change preview artifact" "$preview_file"
    return 2
  fi
  preview="$(jq -cS . "$preview_file")"
  transaction_validate_preview_binding "$preview" "$repository" "$historical" || return $?
  preview_digest="$(printf '%s\n' "$preview" | json_digest_for)"
  projection_digest="$(printf '%s\n' "$preview" | jq '.candidate.source_projection' | json_digest_for)"
  source_bytes="$(transaction_source_projection_bytes "$preview")"
  transaction="$(printf '%s\n' "$preview" | jq --arg schema "$SCHEMA_CHANGE_APPROVAL_REQUEST" '
    .transaction
    | .state="awaiting_approval"
    | .history=["opened","inspected","planned","previewed","awaiting_approval"]
    | .compiler.contract_versions.approval_request=$schema')"
  preview_ref="$(printf '%s\n' "$preview" | jq --arg digest "$preview_digest" '{
    schema,preview_digest:$digest,transaction_id:.transaction.transaction_id,
    plan_digest:.plan.plan_digest,operation_digest:.plan.operation_digest,
    candidate_id:.candidate.candidate_id,candidate_digest:.candidate.candidate_digest,
    base:.transaction.base
  }')"
  candidate_ref="$(printf '%s\n' "$preview" | jq --arg projection_digest "$projection_digest" '{
    candidate_digest:.candidate.candidate_digest,
    candidate_source_digest:.candidate.candidate_source_digest,
    candidate_graph_digest:.candidate.candidate_graph_digest,
    source_projection_digest:$projection_digest,
    owned_paths:(.candidate.owned_paths | unique | sort),
    validation:{profile:.validation.profile,required_passed:.validation.required_passed}
  }')"
  derived_scope="$(printf '%s\n' "$preview" | jq \
    --arg scope_kind "$(printf '%s\n' "$AUTHORITY_LOCAL_GRANT_RESOURCE_SCOPE_KINDS_JSON" | jq -r '.[0]')" \
    --argjson max_source_bytes "$source_bytes" '{
      operation_classes:([.plan.operations[].op] | unique | sort),
      affected_nodes:(.plan.affected_nodes | unique | sort),
      allowed_paths:(.candidate.owned_paths | unique | sort),
      effect_changes:(.diffs.effects | sort_by(.identity,.change_kind)),
      authority_changes:(.diffs.authority | sort_by(.identity,.change_kind)),
      review_requirements:([.review_requirements[].kind] | unique | sort),
      resource_scopes:[{kind:$scope_kind,paths:(.candidate.owned_paths | unique | sort)}],
      max_source_bytes:$max_source_bytes,
      max_changed_files:(.candidate.owned_paths | length)
    }')"
  mutations="$(printf '%s\n' "$TRANSACTION_MUTATION_BOUNDARIES_JSON" | jq 'reduce .[] as $name ({}; .[$name] = false)')"
  next_actions="$(jq -n '[{
    kind:"approve_exact_candidate",
    reason:"supply every operator assertion and acknowledge every compiler-derived review requirement",
    command:["sley","change","approve","--json","--request","<approval-request.json>","--preview","<preview.json>","--repository","<repository>"]
  }]')"
  report_core="$(jq -n \
    --arg schema "$SCHEMA_CHANGE_APPROVAL_REQUEST" \
    --argjson transaction "$transaction" \
    --argjson preview_ref "$preview_ref" \
    --argjson candidate_ref "$candidate_ref" \
    --argjson derived_scope "$derived_scope" \
    --argjson required_operator_fields "$AUTHORITY_LOCAL_GRANT_OPERATOR_FIELDS_JSON" \
    --argjson mutations "$mutations" \
    --argjson next_actions "$next_actions" '{
      schema:$schema,status:"awaiting_approval",transaction:$transaction,
      preview_ref:$preview_ref,candidate_ref:$candidate_ref,derived_scope:$derived_scope,
      required_operator_fields:$required_operator_fields,mutations:$mutations,
      next_actions:$next_actions,issues:[]
    }')"
  request_digest="$(printf '%s\n' "$report_core" | json_digest_for)"
  request_id="approval-request:${request_digest#sha256:}"
  report="$(printf '%s\n' "$report_core" | jq --arg id "$request_id" --arg digest "$request_digest" '{schema,status,request_id:$id,request_digest:$digest} + del(.schema,.status)')"
  change_approval_request_report_from_json "$report"
}

transaction_operation_document_json() {
  local operation_file="$1" allowed_json="$TRANSACTION_PREVIEW_OPERATION_PLAN_JSON"
  if ! jq -e --argjson allowed "$allowed_json" '
    def exact_keys($allowed): ((keys_unsorted - $allowed) | length) == 0;
    def valid_payload:
      if .op == "RenameDeclaration" then
        (.payload | type == "object" and exact_keys(["name"]) and (.name | type == "string" and test("^[A-Za-z_][A-Za-z0-9_]*$")))
      elif .op == "UpdateCallSites" then
        (.payload | type == "object" and exact_keys(["from","replacement","scope"])
          and all([.from,.replacement][]; type == "string" and test("^[A-Za-z_][A-Za-z0-9_.]*$"))
          and (.scope | type == "string" and test("^(module|task):[A-Za-z_][A-Za-z0-9_.]*$")))
      elif .op == "AddTake" then
        (.payload | type == "object" and exact_keys(["name","type","position"])
          and (.name | type == "string" and test("^[A-Za-z_][A-Za-z0-9_]*$"))
          and (.type | type == "string" and length > 0)
          and .position == 0)
      elif .op == "UpdateCallArgs" then
        (.payload | type == "object" and exact_keys(["from","source","position","scope"])
          and (.from | type == "string" and test("^[A-Za-z_][A-Za-z0-9_.]*$"))
          and (.source | type == "string" and length > 0 and length <= 4096)
          and .position == 0
          and (.scope | type == "string" and test("^(module|task):[A-Za-z_][A-Za-z0-9_.]*$")))
      else false end;
    type == "object"
    and exact_keys(["transaction","actor","mode","ops"])
    and (.transaction | type == "string" and length > 0)
    and ((.actor? // "") | type == "string")
    and .mode == "all_or_nothing"
    and (.ops | type == "array" and length > 0 and length <= 32)
    and all(.ops[];
      type == "object"
      and exact_keys(["op","target","payload"])
      and (.op as $op | any($allowed[]; .op == $op))
      and (.target | type == "string" and test("^task:[A-Za-z_][A-Za-z0-9_.]*$"))
      and valid_payload)
  ' "$operation_file" >/dev/null 2>&1; then
    sley_report_error_json "TRANSACTION_OPERATION_INVALID" "change operation artifact must be a strict supported all-or-nothing transaction" "$operation_file"
    return 2
  fi
  jq -cS '{transaction,actor:(.actor // ""),mode,ops}' "$operation_file"
}

transaction_module_file_map_json() {
  local target="$1" repository="$2" source_file module relative
  while IFS= read -r source_file; do
    module="$(awk '/^[[:space:]]*module[[:space:]]+/ {print $2; found=1; exit} END {if(!found) print "main"}' "$source_file")"
    relative="$(realpath --relative-to="$repository" -- "$source_file")"
    jq -cn --arg key "$module" --arg value "$relative" '{key:$key,value:$value}'
  done < <(collect_files "$target") | jq -s 'from_entries'
}

command_change_plan() {
  local inspection_file="" operation_file="" repository="" requested_outcome="" validation_profile="compiler_preview_v0" arg
  local -a assumptions=() non_goals=()
  while [[ "$#" -gt 0 ]]; do
    arg="$1"
    case "$arg" in
      --json) shift ;;
      --inspection|--operation|--repository|--requested-outcome|--assumption|--non-goal|--validation-profile)
        if [[ "$#" -lt 2 ]]; then
          sley_report_error_json "TRANSACTION_ARGUMENT_REQUIRED" "$arg requires a value" "change:plan"
          return 2
        fi
        case "$arg" in
          --inspection) inspection_file="$2" ;;
          --operation) operation_file="$2" ;;
          --repository) repository="$2" ;;
          --requested-outcome) requested_outcome="$2" ;;
          --assumption) assumptions+=("$2") ;;
          --non-goal) non_goals+=("$2") ;;
          --validation-profile) validation_profile="$2" ;;
        esac
        shift 2
        ;;
      --*)
        sley_report_error_json "TRANSACTION_ARGUMENT_UNKNOWN" "unknown change plan option: $arg" "change:plan"
        return 2
        ;;
      *)
        sley_report_error_json "TRANSACTION_ARGUMENT_UNKNOWN" "change plan does not accept positional arguments" "change:plan"
        return 2
        ;;
    esac
  done
  if [[ -z "$requested_outcome" || "${#assumptions[@]}" -eq 0 || "${#non_goals[@]}" -eq 0 ]]; then
    sley_report_error_json "TRANSACTION_PLAN_RATIONALE_REQUIRED" "change plan requires --requested-outcome, at least one --assumption, and at least one --non-goal" "change:plan"
    return 2
  fi
  if ! printf '%s\n' "$TRANSACTION_PLAN_VALIDATION_PROFILES_JSON" | jq -e --arg profile "$validation_profile" 'index($profile) != null' >/dev/null; then
    sley_report_error_json "TRANSACTION_VALIDATION_PROFILE_UNSUPPORTED" "unsupported transaction validation profile: $validation_profile" "change:plan"
    return 2
  fi
  transaction_require_regular_artifact "inspection" "$inspection_file" || return $?
  transaction_require_regular_artifact "operation" "$operation_file" || return $?
  if ! contract_jsonschema validate --schema-dir "$ROOT_DIR/docs/schemas" --schema "$SCHEMA_TRANSACTION_INSPECT" "$inspection_file" >/dev/null 2>&1; then
    sley_report_error_json "TRANSACTION_INSPECTION_INVALID" "change plan requires a valid transaction inspection artifact" "$inspection_file"
    return 2
  fi
  local inspection operation_json target_rel binding expires_at expires_epoch now_epoch
  local expected_identity expected_commit expected_source expected_graph
  inspection="$(jq -cS . "$inspection_file")"
  if ! operation_json="$(transaction_operation_document_json "$operation_file")"; then
    printf '%s\n' "$operation_json"
    return 2
  fi
  target_rel="$(printf '%s\n' "$inspection" | jq -r '.inspection.target')"
  if ! binding="$(transaction_repository_binding_json "$repository" "$target_rel")"; then
    printf '%s\n' "$binding"
    return 2
  fi
  expected_identity="$(printf '%s\n' "$inspection" | jq -r '.transaction.repository.identity_digest')"
  expected_commit="$(printf '%s\n' "$inspection" | jq -r '.transaction.base.commit')"
  expected_source="$(printf '%s\n' "$inspection" | jq -r '.transaction.base.source_digest')"
  expected_graph="$(printf '%s\n' "$inspection" | jq -r '.transaction.base.graph_digest')"
  if ! printf '%s\n' "$binding" | jq -e \
    --arg identity "$expected_identity" --arg commit "$expected_commit" \
    --arg source "$expected_source" --arg graph "$expected_graph" \
    '.identity_digest == $identity and .commit == $commit and .source_digest == $source and .graph_digest == $graph' >/dev/null; then
    sley_report_error_json "TRANSACTION_STALE" "inspection no longer matches the explicit repository base" "$target_rel"
    return 2
  fi
  expires_at="$(printf '%s\n' "$inspection" | jq -r '.transaction.expires_at')"
  expires_epoch="$(date -u -d "$expires_at" +%s 2>/dev/null || true)"
  now_epoch="$(date -u +%s)"
  if [[ -z "$expires_epoch" || "$expires_epoch" -le "$now_epoch" ]]; then
    sley_report_error_json "TRANSACTION_EXPIRED" "inspection transaction has expired" "$target_rel"
    return 2
  fi

  local target_abs repo_abs base_query module_files affected_nodes affected_modules affected_files missing_modules
  local assumptions_json non_goals_json operation_digest expected_changes preconditions inspection_ref transaction plan_core plan_digest plan mutations next_actions report
  target_abs="$(printf '%s\n' "$binding" | jq -r '.target')"
  repo_abs="$(printf '%s\n' "$binding" | jq -r '.repository')"
  base_query="$(query_json "$target_abs" all)"
  module_files="$(transaction_module_file_map_json "$target_abs" "$repo_abs")"
  affected_nodes="$(printf '%s\n' "$operation_json" | jq '[.ops[] | .target, .payload.scope?] | map(select(. != null)) | unique | sort')"
  affected_modules="$(printf '%s\n' "$operation_json" | jq '
    def target_module: sub("^task:";"") | split(".") | .[0:-1] | join(".");
    def scoped_module:
      if startswith("module:") then sub("^module:";"")
      else sub("^task:";"") | split(".") | .[0:-1] | join(".") end;
    [.ops[] | (.target | target_module), (.payload.scope? // empty | scoped_module)] | unique | sort
  ')"
  missing_modules="$(jq -n --argjson modules "$affected_modules" --argjson files "$module_files" '$modules | map(select(($files[.] // "") == ""))')"
  if [[ "$(printf '%s\n' "$missing_modules" | jq 'length')" -gt 0 ]]; then
    sley_report_error_json "TRANSACTION_OPERATION_TARGET_UNKNOWN" "change operation targets a module outside the inspected source" "change:plan"
    return 2
  fi
  affected_files="$(jq -n --argjson modules "$affected_modules" --argjson files "$module_files" '$modules | map($files[.]) | unique | sort')"
  assumptions_json="$(printf '%s\n' "${assumptions[@]}" | jq -Rsc 'split("\n") | map(select(length > 0)) | unique')"
  non_goals_json="$(printf '%s\n' "${non_goals[@]}" | jq -Rsc 'split("\n") | map(select(length > 0)) | unique')"
  operation_digest="$(printf '%s\n' "$operation_json" | json_digest_for)"
  expected_changes="$(printf '%s\n' "$operation_json" | jq '
    def expectation($text): {source:"compiler_derived",description:$text};
    {
      call_sites:[.ops[] | select(.op == "RenameDeclaration" or .op == "UpdateCallSites" or .op == "UpdateCallArgs") | expectation(.op + " affects " + .target)],
      types:[.ops[] | select(.op == "RenameDeclaration" or .op == "AddTake") | expectation(.op + " affects " + .target)],
      effects:[],
      authority:[]
    }
  ')"
  preconditions="$(printf '%s\n' "$inspection" | jq '[
    "repository identity matches inspection",
    "base commit matches inspection",
    "working-tree source digest matches inspection",
    "symbol graph digest matches inspection",
    "transaction expiry remains valid",
    "operations execute in declared order and all-or-nothing"
  ]')"
  inspection_ref="$(printf '%s\n' "$inspection" | jq '{schema,transaction_id:.transaction.transaction_id,context_digest:.inspection.context_digest,target:.inspection.target}')"
  transaction="$(printf '%s\n' "$inspection" | jq \
    --arg plan_schema "$SCHEMA_CHANGE_PLAN" \
    '.transaction
      | .state = "planned"
      | .history = ["opened","inspected","planned"]
      | .repository.root = "."
      | .compiler.contract_versions.plan = $plan_schema')"
  plan_core="$(jq -n \
    --arg operation_digest "$operation_digest" \
    --arg requested_outcome "$requested_outcome" \
    --arg profile "$validation_profile" \
    --argjson assumptions "$assumptions_json" \
    --argjson preconditions "$preconditions" \
    --argjson affected_nodes "$affected_nodes" \
    --argjson affected_files "$affected_files" \
    --argjson operations "$(printf '%s\n' "$operation_json" | jq '.ops')" \
    --argjson expected_changes "$expected_changes" \
    --argjson non_goals "$non_goals_json" '{
      operation_digest:$operation_digest,
      requested_outcome:$requested_outcome,
      assumptions:$assumptions,
      preconditions:$preconditions,
      affected_nodes:$affected_nodes,
      affected_files:$affected_files,
      operations:$operations,
      expected_changes:$expected_changes,
      validation_profile:{id:$profile,required_checks:["check","lint","graph_diff"]},
      selected_tests:["compiler_check","compiler_lint","structural_graph_diff"],
      non_goals:$non_goals,
      fact_sources:{
        compiler:["transaction inspection","checked query","symbol graph","operation target mapping"],
        operator:["requested outcome","assumptions","non-goals","operation artifact"],
        model:[]
      }
    }')"
  plan_digest="$(jq -n --argjson transaction "$transaction" --argjson inspection "$inspection_ref" --argjson plan "$plan_core" '{transaction_id:$transaction.transaction_id,base:$transaction.base,inspection:$inspection,plan:$plan}' | json_digest_for)"
  plan="$(printf '%s\n' "$plan_core" | jq --arg plan_digest "$plan_digest" '{plan_digest:$plan_digest} + .')"
  mutations="$(printf '%s\n' "$TRANSACTION_MUTATION_BOUNDARIES_JSON" | jq 'reduce .[] as $name ({}; .[$name] = false)')"
  next_actions="$(jq -n '[{kind:"preview_candidate",reason:"materialize the exact plan only inside an ephemeral candidate copy",command:["sley","change","preview","--json","--repository",".","--plan","<plan.json>"]}]')"
  report="$(jq -n \
    --arg schema "$SCHEMA_CHANGE_PLAN" \
    --argjson transaction "$transaction" \
    --argjson inspection "$inspection_ref" \
    --argjson plan "$plan" \
    --argjson mutations "$mutations" \
    --argjson next_actions "$next_actions" '{
      schema:$schema,status:"planned",transaction:$transaction,inspection:$inspection,
      plan:$plan,mutations:$mutations,next_actions:$next_actions,issues:[]
    }')"
  change_plan_report_from_json "$report"
}

transaction_scope_file() {
  local source="$1" scope="$2" scope_rest scope_module
  scope_rest="${scope#*:}"
  if [[ "$scope" == task:* ]]; then
    scope_module="${scope_rest%.*}"
  else
    scope_module="$scope_rest"
  fi
  sley_task_file_for_module "$source" "$scope_module"
}

transaction_rewrite_call_site_names() {
  local source="$1" from="$2" replacement="$3" scope="$4" file tmp_file scope_task=""
  file="$(transaction_scope_file "$source" "$scope")" || return 1
  [[ "$scope" == task:* ]] && scope_task="${scope##*.}"
  tmp_file="$(mktemp)"
  awk -v from="$from" -v replacement="$replacement" -v scope_task="$scope_task" -v call_prefix="$PARSER_CALL_EXPRESSION_PREFIX" '
    function count_char(s, c, n, i){n=0; for(i=1;i<=length(s);i++){if(substr(s,i,1)==c)n++} return n}
    {
      line=$0
      if(scope_task != "" && !in_scope && line ~ "^[[:space:]]*(export[[:space:]]+)?task[[:space:]]+" scope_task "([[:space:]]|$)") {
        in_scope=1
        depth=0
      }
      active=(scope_task == "" || in_scope)
      if(active) {
        needle=call_prefix from "("
        repl=call_prefix replacement "("
        while((pos=index(line,needle)) > 0) {
          line=substr(line,1,pos-1) repl substr(line,pos+length(needle))
        }
      }
      print line
      if(scope_task != "" && in_scope) {
        depth += count_char($0,"{")-count_char($0,"}")
        if(depth<=0) in_scope=0
      }
    }
  ' "$file" > "$tmp_file"
  mv "$tmp_file" "$file"
}

transaction_add_call_arg() {
  local source="$1" from="$2" argument_source="$3" scope="$4" file tmp_file scope_task=""
  file="$(transaction_scope_file "$source" "$scope")" || return 1
  [[ "$scope" == task:* ]] && scope_task="${scope##*.}"
  tmp_file="$(mktemp)"
  awk -v from="$from" -v argument_source="$argument_source" -v scope_task="$scope_task" -v call_prefix="$PARSER_CALL_EXPRESSION_PREFIX" '
    function count_char(s, c, n, i){n=0; for(i=1;i<=length(s);i++){if(substr(s,i,1)==c)n++} return n}
    {
      line=$0
      if(scope_task != "" && !in_scope && line ~ "^[[:space:]]*(export[[:space:]]+)?task[[:space:]]+" scope_task "([[:space:]]|$)") {
        in_scope=1
        depth=0
      }
      active=(scope_task == "" || in_scope)
      if(active) {
        needle=call_prefix from "("
        remaining=line
        rewritten=""
        while((pos=index(remaining,needle)) > 0) {
          tail=substr(remaining,pos+length(needle))
          separator=(substr(tail,1,1) == ")" ? "" : ", ")
          rewritten=rewritten substr(remaining,1,pos-1) needle argument_source separator
          remaining=tail
        }
        line=rewritten remaining
      }
      print line
      if(scope_task != "" && in_scope) {
        depth += count_char($0,"{")-count_char($0,"}")
        if(depth<=0) in_scope=0
      }
    }
  ' "$file" > "$tmp_file"
  mv "$tmp_file" "$file"
}

transaction_preview_rename_declaration() {
  local source="$1" operation="$2" task_rest task_module task_name new_name
  task_rest="$(printf '%s\n' "$operation" | jq -r '.target | sub("^task:";"")')"
  task_module="${task_rest%.*}"
  task_name="${task_rest##*.}"
  new_name="$(printf '%s\n' "$operation" | jq -r '.payload.name')"
  sley_rename_task_declaration "$source" "$task_module" "$task_name" "$new_name"
}

transaction_preview_update_call_sites() {
  local source="$1" operation="$2"
  transaction_rewrite_call_site_names "$source" \
    "$(printf '%s\n' "$operation" | jq -r '.payload.from')" \
    "$(printf '%s\n' "$operation" | jq -r '.payload.replacement')" \
    "$(printf '%s\n' "$operation" | jq -r '.payload.scope')"
}

transaction_preview_add_take() {
  local source="$1" operation="$2"
  sley_add_take_to_task "$source" \
    "$(printf '%s\n' "$operation" | jq -r '.target')" \
    "$(printf '%s\n' "$operation" | jq -r '.payload.name')" \
    "$(printf '%s\n' "$operation" | jq -r '.payload.type')"
}

transaction_preview_update_call_args() {
  local source="$1" operation="$2"
  transaction_add_call_arg "$source" \
    "$(printf '%s\n' "$operation" | jq -r '.payload.from')" \
    "$(printf '%s\n' "$operation" | jq -r '.payload.source')" \
    "$(printf '%s\n' "$operation" | jq -r '.payload.scope')"
}

transaction_apply_preview_operation() {
  local source="$1" operation="$2" op executor
  op="$(printf '%s\n' "$operation" | jq -r '.op')"
  executor="$(printf '%s\n' "$TRANSACTION_PREVIEW_OPERATION_PLAN_JSON" | jq -r --arg op "$op" '.[] | select(.op == $op) | .executor' | head -n 1)"
  if [[ -z "$executor" ]] || ! declare -F "$executor" >/dev/null; then
    sley_report_error_json "TRANSACTION_OPERATION_UNSUPPORTED" "preview has no Sley-owned executor for operation: $op" "change:preview"
    return 2
  fi
  "$executor" "$source" "$operation"
}

transaction_source_projection_json() {
  local candidate="$1" target_rel="$2" file inner relative digest
  while IFS= read -r file; do
    if [[ -f "$candidate" ]]; then
      relative="$target_rel"
    else
      inner="$(realpath --relative-to="$candidate" -- "$file")"
      relative="${target_rel%/}/$inner"
    fi
    digest="$(sha256sum "$file" | awk '{print "sha256:" $1}')"
    jq -cn --arg path "$relative" --arg digest "$digest" --rawfile content "$file" '{path:$path,digest:$digest,content:$content}'
  done < <(collect_files "$candidate") | jq -s 'sort_by(.path)'
}

transaction_source_diff_json() {
  local base="$1" candidate="$2" target_rel="$3" base_file inner candidate_file relative diff_text="" piece
  local -a changed=()
  while IFS= read -r base_file; do
    if [[ -f "$base" ]]; then
      candidate_file="$candidate"
      relative="$target_rel"
    else
      inner="$(realpath --relative-to="$base" -- "$base_file")"
      candidate_file="$candidate/$inner"
      relative="${target_rel%/}/$inner"
    fi
    if [[ ! -f "$candidate_file" ]] || ! cmp -s "$base_file" "$candidate_file"; then
      changed+=("$relative")
      piece="$(diff -u --label "a/$relative" --label "b/$relative" "$base_file" "$candidate_file" 2>/dev/null || true)"
      diff_text+="$piece"$'\n'
    fi
  done < <(collect_files "$base")
  if [[ "${#changed[@]}" -eq 0 ]]; then
    sley_report_error_json "TRANSACTION_CANDIDATE_UNCHANGED" "preview operations produced no candidate source change" "change:preview"
    return 2
  fi
  printf '%s\n' "${changed[@]}" | jq -Rsc --arg text "${diff_text%$'\n'}" '{format:"unified",changed_files:(split("\n") | map(select(length > 0)) | unique | sort),text:$text}'
}

command_change_preview() (
  set +e
  local plan_file="" repository="" arg candidate_parent="" candidate=""
  trap 'sley_cleanup_temp_dir "${candidate_parent:-}"' EXIT
  while [[ "$#" -gt 0 ]]; do
    arg="$1"
    case "$arg" in
      --json) shift ;;
      --plan|--repository)
        if [[ "$#" -lt 2 ]]; then
          sley_report_error_json "TRANSACTION_ARGUMENT_REQUIRED" "$arg requires a value" "change:preview"
          return 2
        fi
        [[ "$arg" == "--plan" ]] && plan_file="$2" || repository="$2"
        shift 2
        ;;
      --*) sley_report_error_json "TRANSACTION_ARGUMENT_UNKNOWN" "unknown change preview option: $arg" "change:preview"; return 2 ;;
      *) sley_report_error_json "TRANSACTION_ARGUMENT_UNKNOWN" "change preview does not accept positional arguments" "change:preview"; return 2 ;;
    esac
  done
  transaction_require_regular_artifact "plan" "$plan_file" || return $?
  if ! contract_jsonschema validate --schema-dir "$ROOT_DIR/docs/schemas" --schema "$SCHEMA_CHANGE_PLAN" "$plan_file" >/dev/null 2>&1; then
    sley_report_error_json "TRANSACTION_PLAN_INVALID" "change preview requires a valid transaction plan artifact" "$plan_file"
    return 2
  fi
  local plan target_rel binding expected_plan_digest actual_plan_digest expires_at expires_epoch now_epoch
  plan="$(jq -cS . "$plan_file")"
  expected_plan_digest="$(printf '%s\n' "$plan" | jq -r '.plan.plan_digest')"
  actual_plan_digest="$(printf '%s\n' "$plan" | jq -cS '{transaction_id:.transaction.transaction_id,base:.transaction.base,inspection:.inspection,plan:(.plan | del(.plan_digest))}' | json_digest_for)"
  if [[ "$expected_plan_digest" != "$actual_plan_digest" ]]; then
    sley_report_error_json "TRANSACTION_PLAN_DIGEST_MISMATCH" "change plan content does not match its bound digest" "$plan_file"
    return 2
  fi
  target_rel="$(printf '%s\n' "$plan" | jq -r '.inspection.target')"
  if ! binding="$(transaction_repository_binding_json "$repository" "$target_rel")"; then
    printf '%s\n' "$binding"
    return 2
  fi
  if ! printf '%s\n' "$binding" | jq -e \
    --arg identity "$(printf '%s\n' "$plan" | jq -r '.transaction.repository.identity_digest')" \
    --arg commit "$(printf '%s\n' "$plan" | jq -r '.transaction.base.commit')" \
    --arg source "$(printf '%s\n' "$plan" | jq -r '.transaction.base.source_digest')" \
    --arg graph "$(printf '%s\n' "$plan" | jq -r '.transaction.base.graph_digest')" \
    '.identity_digest == $identity and .commit == $commit and .source_digest == $source and .graph_digest == $graph' >/dev/null; then
    sley_report_error_json "TRANSACTION_STALE" "plan no longer matches the explicit repository base" "$target_rel"
    return 2
  fi
  expires_at="$(printf '%s\n' "$plan" | jq -r '.transaction.expires_at')"
  expires_epoch="$(date -u -d "$expires_at" +%s 2>/dev/null || true)"
  now_epoch="$(date -u +%s)"
  if [[ -z "$expires_epoch" || "$expires_epoch" -le "$now_epoch" ]]; then
    sley_report_error_json "TRANSACTION_EXPIRED" "plan transaction has expired" "$target_rel"
    return 2
  fi

  local base_target repo_abs operation before_digest after_digest changed_json allowed_changed
  base_target="$(printf '%s\n' "$binding" | jq -r '.target')"
  repo_abs="$(printf '%s\n' "$binding" | jq -r '.repository')"
  candidate_parent="$(mktemp -d)"
  if [[ -f "$base_target" ]]; then
    candidate="$candidate_parent/candidate.sley"
    if ! cp -a -- "$base_target" "$candidate" 2>/dev/null; then
      sley_report_error_json "TRANSACTION_CANDIDATE_COPY_FAILED" "preview could not create an isolated candidate file" "change:preview"
      return 2
    fi
  else
    candidate="$candidate_parent/candidate"
    mkdir -p "$candidate"
    if ! cp -a -- "$base_target/." "$candidate/" 2>/dev/null; then
      sley_report_error_json "TRANSACTION_CANDIDATE_COPY_FAILED" "preview could not create an isolated candidate project" "change:preview"
      return 2
    fi
  fi
  if find "$candidate" -type l -print -quit | grep -q .; then
    sley_report_error_json "TRANSACTION_CANDIDATE_SYMLINK_DENIED" "candidate copy unexpectedly contains a symlink" "change:preview"
    return 2
  fi
  while IFS= read -r operation; do
    before_digest="$(structural_source_digest_for "$candidate")"
    transaction_apply_preview_operation "$candidate" "$operation" || return $?
    after_digest="$(structural_source_digest_for "$candidate")"
    if [[ "$before_digest" == "$after_digest" ]]; then
      sley_report_error_json "TRANSACTION_OPERATION_NO_CHANGE" "preview operation did not change the candidate: $(printf '%s\n' "$operation" | jq -r '.op')" "change:preview"
      return 2
    fi
  done < <(printf '%s\n' "$plan" | jq -c '.plan.operations[]')

  local source_diff changed_files affected_files outside_paths
  if ! source_diff="$(transaction_source_diff_json "$base_target" "$candidate" "$target_rel")"; then
    printf '%s\n' "$source_diff"
    return 2
  fi
  changed_files="$(printf '%s\n' "$source_diff" | jq '.changed_files')"
  affected_files="$(printf '%s\n' "$plan" | jq '.plan.affected_files')"
  outside_paths="$(jq -n --argjson changed "$changed_files" --argjson allowed "$affected_files" '$changed - $allowed')"
  if [[ "$(printf '%s\n' "$outside_paths" | jq 'length')" -ne 0 ]]; then
    sley_report_error_json "TRANSACTION_OWNED_PATH_VIOLATION" "candidate changed source outside the planned affected files" "change:preview"
    return 2
  fi

  local check lint graph_diff base_query candidate_query candidate_source_digest candidate_graph candidate_graph_digest
  local source_projection graph_status check_status lint_status check_count lint_count graph_issue_count required_passed
  check="$(check_json "$candidate")"
  lint="$(lint_json "$candidate")"
  graph_diff="$(command_graph_diff --json --base "$base_target" --ours "$base_target" --theirs "$candidate" || true)"
  graph_diff="$(printf '%s\n' "$graph_diff" | jq '
    .inputs.base.path="base" | .inputs.ours.path="base" | .inputs.theirs.path="candidate"
    | .next_actions |= map(if .command then .command[-1]="candidate" else . end)')"
  base_query="$(query_json "$base_target" all)"
  candidate_query="$(query_json "$candidate" all)"
  candidate_source_digest="$(structural_source_digest_for "$candidate")"
  candidate_graph="$(graph_json_for "$candidate")"
  candidate_graph_digest="$(printf '%s\n' "$candidate_graph" | json_digest_for)"
  source_projection="$(transaction_source_projection_json "$candidate" "$target_rel")"
  check_status="$(printf '%s\n' "$check" | jq -r --arg ok "$CHECK_OK_STATUS" 'if .status == $ok then "passed" else "failed" end')"
  lint_status="$(printf '%s\n' "$lint" | jq -r --arg ok "$LINT_OK_STATUS" 'if .status == $ok then "passed" else "review_required" end')"
  graph_status="$(printf '%s\n' "$graph_diff" | jq -r 'if .status == "passed" then "passed" elif .status == "conflicted" then "review_required" else "failed" end')"
  check_count="$(printf '%s\n' "$check" | jq '.diagnostics | length')"
  lint_count="$(printf '%s\n' "$lint" | jq '.findings | length')"
  graph_issue_count="$(printf '%s\n' "$graph_diff" | jq '(.conflicts | length) + ([.projection_checks[]? | select(.status != "passed")] | length)')"
  required_passed=false
  [[ "$check_status" == passed && "$graph_status" != failed ]] && required_passed=true

  local semantic_changes type_changes call_changes effect_changes authority_changes transaction candidate_core candidate_digest candidate_id
  semantic_changes="$(printf '%s\n' "$graph_diff" | jq '[.changes[]? | select(.side == "theirs") | {identity,change_kind,before,after}]')"
  type_changes="$(printf '%s\n' "$semantic_changes" | jq '[.[] | select(.identity | startswith("task:"))]')"
  call_changes="$(printf '%s\n' "$semantic_changes" | jq '[.[] | select(.identity | startswith("call:"))]')"
  effect_changes="$(printf '%s\n' "$type_changes" | jq '[.[] | select((.before.effects // []) != (.after.effects // []))]')"
  authority_changes="$(printf '%s\n' "$type_changes" | jq '[.[] | select(
    (.before.effects // []) != (.after.effects // []) or
    ([.before.takes[]? | select(.binding_kind == "Gate")] != [.after.takes[]? | select(.binding_kind == "Gate")]))]')"
  transaction="$(printf '%s\n' "$plan" | jq --arg preview_schema "$SCHEMA_CHANGE_PREVIEW" '
    .transaction | .state="previewed" | .history=["opened","inspected","planned","previewed"]
    | .compiler.contract_versions.preview=$preview_schema')"
  candidate_core="$(jq -n \
    --arg workspace_kind "$(printf '%s\n' "$TRANSACTION_CANDIDATE_WORKSPACE_KINDS_JSON" | jq -r '.[0]')" \
    --arg base_source_digest "$(printf '%s\n' "$plan" | jq -r '.transaction.base.source_digest')" \
    --arg candidate_source_digest "$candidate_source_digest" \
    --arg candidate_graph_digest "$candidate_graph_digest" \
    --arg operation_digest "$(printf '%s\n' "$plan" | jq -r '.plan.operation_digest')" \
    --argjson owned_paths "$changed_files" \
    --argjson source_projection "$source_projection" '{
      workspace_kind:$workspace_kind,materialized:false,base_source_digest:$base_source_digest,
      candidate_source_digest:$candidate_source_digest,candidate_graph_digest:$candidate_graph_digest,
      operation_digest:$operation_digest,owned_paths:$owned_paths,source_projection:$source_projection
    }')"
  candidate_digest="$(jq -n --argjson transaction "$transaction" --arg plan_digest "$expected_plan_digest" --argjson candidate "$candidate_core" --argjson validation "$(jq -n --arg check "$check_status" --arg lint "$lint_status" --arg graph "$graph_status" '{check:$check,lint:$lint,graph_diff:$graph}')" '{transaction:$transaction,plan_digest:$plan_digest,candidate:$candidate,validation:$validation}' | json_digest_for)"
  candidate_id="candidate:${candidate_digest#sha256:}"

  local focused_tests validation reviews mutations next_actions report
  focused_tests="$(jq -n \
    --arg check_status "$check_status" --arg check_schema "$(printf '%s\n' "$check" | jq -r '.schema')" --argjson check_count "$check_count" \
    --arg lint_status "$lint_status" --arg lint_schema "$(printf '%s\n' "$lint" | jq -r '.schema')" --argjson lint_count "$lint_count" \
    --arg graph_status "$graph_status" --arg graph_schema "$(printf '%s\n' "$graph_diff" | jq -r '.schema')" --argjson graph_count "$graph_issue_count" '[
      {kind:"compiler_check",status:$check_status,schema:$check_schema,issue_count:$check_count},
      {kind:"compiler_lint",status:$lint_status,schema:$lint_schema,issue_count:$lint_count},
      {kind:"structural_graph_diff",status:$graph_status,schema:$graph_schema,issue_count:$graph_count}
    ]')"
  validation="$(jq -n --argjson required_passed "$required_passed" \
    --arg check_status "$check_status" --arg check_schema "$(printf '%s\n' "$check" | jq -r '.schema')" --argjson check_count "$check_count" \
    --arg lint_status "$lint_status" --arg lint_schema "$(printf '%s\n' "$lint" | jq -r '.schema')" --argjson lint_count "$lint_count" \
    --arg graph_status "$graph_status" --arg graph_schema "$(printf '%s\n' "$graph_diff" | jq -r '.schema')" --argjson graph_count "$graph_issue_count" '{
      profile:"compiler_preview_v0",required_passed:$required_passed,
      check:{status:$check_status,schema:$check_schema,issue_count:$check_count},
      lint:{status:$lint_status,schema:$lint_schema,issue_count:$lint_count},
      graph_diff:{status:$graph_status,schema:$graph_schema,issue_count:$graph_count}
    }')"
  reviews="$(printf '%s\n' "$TRANSACTION_PREVIEW_REVIEW_REQUIREMENT_KINDS_JSON" | jq '[.[] | {
    kind:.,blocking_for_apply:true,
    reason:(if . == "human_source_review" then "review the complete candidate source projection and unified diff"
      elif . == "graph_diff_review" then "review structural graph changes and rename ambiguity"
      elif . == "lint_review" then "review compiler lint findings even when non-fatal"
      else "review effect and gate authority changes before any approval" end)
  }]')"
  mutations="$(printf '%s\n' "$TRANSACTION_MUTATION_BOUNDARIES_JSON" | jq 'reduce .[] as $name ({}; .[$name] = false)')"
  next_actions='[{"kind":"review_candidate","reason":"review the immutable candidate evidence before a later approval protocol exists"},{"kind":"reinspect_if_base_changes","reason":"discard this preview and inspect again if the repository base changes"}]'
  report="$(jq -n \
    --arg schema "$SCHEMA_CHANGE_PREVIEW" --argjson transaction "$transaction" --argjson plan "$(printf '%s\n' "$plan" | jq '.plan')" \
    --arg candidate_id "$candidate_id" --arg candidate_digest "$candidate_digest" --argjson candidate "$candidate_core" \
    --argjson source "$source_diff" --argjson graph "$graph_diff" --argjson types "$type_changes" --argjson calls "$call_changes" \
    --argjson effects "$effect_changes" --argjson authority "$authority_changes" --argjson diagnostics "$(printf '%s\n' "$check" | jq '.diagnostics')" \
    --argjson focused_tests "$focused_tests" --argjson validation "$validation" --argjson reviews "$reviews" \
    --argjson mutations "$mutations" --argjson next_actions "$next_actions" '{
      schema:$schema,status:"previewed",transaction:$transaction,plan:$plan,
      candidate:({candidate_id:$candidate_id,candidate_digest:$candidate_digest} + $candidate),
      diffs:{source:$source,graph:$graph,types:$types,calls:$calls,effects:$effects,authority:$authority},
      diagnostics:$diagnostics,focused_tests:$focused_tests,validation:$validation,
      review_requirements:$reviews,mutations:$mutations,next_actions:$next_actions,issues:[]
    }')"
  change_preview_report_from_json "$report"
  [[ "$required_passed" == true ]]
)

command_change_approval_request() {
  local preview_file="" repository="" arg
  while [[ "$#" -gt 0 ]]; do
    arg="$1"
    case "$arg" in
      --json) shift ;;
      --preview|--repository)
        if [[ "$#" -lt 2 ]]; then
          sley_report_error_json "TRANSACTION_ARGUMENT_REQUIRED" "$arg requires a value" "change:approval-request"
          return 2
        fi
        [[ "$arg" == "--preview" ]] && preview_file="$2" || repository="$2"
        shift 2
        ;;
      --*)
        sley_report_error_json "TRANSACTION_ARGUMENT_UNKNOWN" "unknown change approval-request option: $arg" "change:approval-request"
        return 2
        ;;
      *)
        sley_report_error_json "TRANSACTION_ARGUMENT_UNKNOWN" "change approval-request does not accept positional arguments" "change:approval-request"
        return 2
        ;;
    esac
  done
  transaction_change_approval_request_json "$preview_file" "$repository"
}

command_change_approve() {
  local request_file="" preview_file="" repository="" issuer="" principal="" audience="" purpose="" nonce=""
  local created_at="" expires_at="" revocation_state="" max_wall_clock_seconds="" signature_ref="" greynucleus_proof_ref=""
  local confirm_exact_local_grant=false arg
  local -a acknowledgements=()
  while [[ "$#" -gt 0 ]]; do
    arg="$1"
    case "$arg" in
      --json) shift ;;
      --confirm-exact-local-grant) confirm_exact_local_grant=true; shift ;;
      --request|--preview|--repository|--issuer|--principal|--audience|--purpose|--nonce|--created-at|--expires-at|--ack-review|--revocation-state|--max-wall-clock-seconds|--signature-ref|--greynucleus-proof-ref)
        if [[ "$#" -lt 2 ]]; then
          sley_report_error_json "TRANSACTION_ARGUMENT_REQUIRED" "$arg requires a value" "change:approve"
          return 2
        fi
        case "$arg" in
          --request) request_file="$2" ;;
          --preview) preview_file="$2" ;;
          --repository) repository="$2" ;;
          --issuer) issuer="$2" ;;
          --principal) principal="$2" ;;
          --audience) audience="$2" ;;
          --purpose) purpose="$2" ;;
          --nonce) nonce="$2" ;;
          --created-at) created_at="$2" ;;
          --expires-at) expires_at="$2" ;;
          --ack-review) acknowledgements+=("$2") ;;
          --revocation-state) revocation_state="$2" ;;
          --max-wall-clock-seconds) max_wall_clock_seconds="$2" ;;
          --signature-ref) signature_ref="$2" ;;
          --greynucleus-proof-ref) greynucleus_proof_ref="$2" ;;
        esac
        shift 2
        ;;
      --*)
        sley_report_error_json "TRANSACTION_ARGUMENT_UNKNOWN" "unknown change approve option: $arg" "change:approve"
        return 2
        ;;
      *)
        sley_report_error_json "TRANSACTION_ARGUMENT_UNKNOWN" "change approve does not accept positional arguments" "change:approve"
        return 2
        ;;
    esac
  done

  transaction_require_regular_artifact "approval request" "$request_file" || return $?
  transaction_require_regular_artifact "preview" "$preview_file" || return $?
  if ! contract_jsonschema validate --schema-dir "$ROOT_DIR/docs/schemas" --schema "$SCHEMA_CHANGE_APPROVAL_REQUEST" "$request_file" >/dev/null 2>&1; then
    sley_report_error_json "TRANSACTION_APPROVAL_REQUEST_INVALID" "approve requires a valid approval request artifact" "$request_file"
    return 2
  fi
  local request expected_request expected_request_output expected_request_digest expected_request_id
  request="$(jq -cS . "$request_file")"
  expected_request_digest="$(printf '%s\n' "$request" | jq 'del(.request_id,.request_digest)' | json_digest_for)"
  expected_request_id="approval-request:${expected_request_digest#sha256:}"
  if ! printf '%s\n' "$request" | jq -e \
    --arg digest "$expected_request_digest" --arg id "$expected_request_id" \
    '.request_digest == $digest and .request_id == $id' >/dev/null; then
    sley_report_error_json "TRANSACTION_APPROVAL_REQUEST_DIGEST_MISMATCH" "approval request content does not match its bound digest" "$request_file"
    return 2
  fi
  if ! expected_request_output="$(transaction_change_approval_request_json "$preview_file" "$repository")"; then
    printf '%s\n' "$expected_request_output"
    return 2
  fi
  expected_request="$(printf '%s\n' "$expected_request_output" | jq -cS .)"
  if [[ "$request" != "$expected_request" ]]; then
    sley_report_error_json "TRANSACTION_APPROVAL_REQUEST_MISMATCH" "approval request is not the exact compiler-derived request for this preview and repository base" "$request_file"
    return 2
  fi
  if [[ "$confirm_exact_local_grant" != true ]]; then
    sley_report_error_json "TRANSACTION_EXACT_GRANT_CONFIRMATION_REQUIRED" "approve requires --confirm-exact-local-grant" "change:approve"
    return 2
  fi
  if [[ -z "$issuer" || -z "$principal" || -z "$audience" || -z "$purpose" || -z "$nonce" || -z "$created_at" || -z "$expires_at" || -z "$revocation_state" || -z "$max_wall_clock_seconds" ]]; then
    sley_report_error_json "TRANSACTION_GRANT_FIELDS_REQUIRED" "approve requires every operator field named by the approval request" "change:approve"
    return 2
  fi
  if [[ ! "$issuer" =~ ^[A-Za-z0-9._:@/-]{1,256}$ || ! "$principal" =~ ^[A-Za-z0-9._:@/-]{1,256}$ || ! "$audience" =~ ^[A-Za-z0-9._:@/-]{1,256}$ ]]; then
    sley_report_error_json "TRANSACTION_PRINCIPAL_INVALID" "issuer, principal, and audience must be bounded principal identifiers" "change:approve"
    return 2
  fi
  if [[ "${#purpose}" -gt 4096 ]]; then
    sley_report_error_json "TRANSACTION_PURPOSE_INVALID" "grant purpose must contain at most 4096 characters" "change:approve"
    return 2
  fi
  if [[ ! "$nonce" =~ ^[A-Za-z0-9._:-]{8,128}$ ]]; then
    sley_report_error_json "TRANSACTION_NONCE_INVALID" "grant nonce must be 8-128 identifier characters" "change:approve"
    return 2
  fi
  if [[ ! "$created_at" =~ ^[0-9]{4}-[0-9]{2}-[0-9]{2}T[0-9]{2}:[0-9]{2}:[0-9]{2}Z$ ]] \
    || [[ ! "$expires_at" =~ ^[0-9]{4}-[0-9]{2}-[0-9]{2}T[0-9]{2}:[0-9]{2}:[0-9]{2}Z$ ]]; then
    sley_report_error_json "TRANSACTION_TIME_INVALID" "grant times must use UTC RFC3339 seconds" "change:approve"
    return 2
  fi
  local created_epoch expires_epoch transaction_created_epoch transaction_expires_epoch
  created_epoch="$(date -u -d "$created_at" +%s 2>/dev/null || true)"
  expires_epoch="$(date -u -d "$expires_at" +%s 2>/dev/null || true)"
  transaction_created_epoch="$(date -u -d "$(printf '%s\n' "$request" | jq -r '.transaction.created_at')" +%s 2>/dev/null || true)"
  transaction_expires_epoch="$(date -u -d "$(printf '%s\n' "$request" | jq -r '.transaction.expires_at')" +%s 2>/dev/null || true)"
  if [[ -z "$created_epoch" || -z "$expires_epoch" || "$expires_epoch" -le "$created_epoch" ]]; then
    sley_report_error_json "TRANSACTION_TIME_ORDER_INVALID" "grant expiry must be later than grant creation" "change:approve"
    return 2
  fi
  if [[ -z "$transaction_created_epoch" || -z "$transaction_expires_epoch" || "$created_epoch" -lt "$transaction_created_epoch" || "$expires_epoch" -gt "$transaction_expires_epoch" ]]; then
    sley_report_error_json "TRANSACTION_GRANT_TIME_SCOPE_BROADENED" "grant times must remain inside the transaction time bounds" "change:approve"
    return 2
  fi
  if ! printf '%s\n' "$AUTHORITY_LOCAL_GRANT_ISSUABLE_REVOCATION_STATES_JSON" | jq -e --arg state "$revocation_state" 'index($state) != null' >/dev/null; then
    sley_report_error_json "TRANSACTION_REVOCATION_STATE_UNISSUABLE" "local grant issuance only permits an explicit not_revoked assertion" "change:approve"
    return 2
  fi
  if [[ ! "$max_wall_clock_seconds" =~ ^[0-9]+$ || "$max_wall_clock_seconds" -lt 1 || "$max_wall_clock_seconds" -gt 86400 ]]; then
    sley_report_error_json "TRANSACTION_GRANT_BUDGET_INVALID" "max wall-clock seconds must be an integer from 1 through 86400" "change:approve"
    return 2
  fi
  if [[ "${#signature_ref}" -gt 512 || "${#greynucleus_proof_ref}" -gt 512 ]]; then
    sley_report_error_json "TRANSACTION_PROOF_REF_INVALID" "proof references must contain at most 512 characters" "change:approve"
    return 2
  fi
  if ! transaction_transition_allowed "awaiting_approval" "approved"; then
    sley_report_error_json "TRANSACTION_TRANSITION_DENIED" "Sley does not permit awaiting_approval to approved" "change:approve"
    return 2
  fi

  local acknowledgements_json required_acknowledgements duplicate_count
  if [[ "${#acknowledgements[@]}" -eq 0 ]]; then
    acknowledgements_json='[]'
  else
    acknowledgements_json="$(printf '%s\n' "${acknowledgements[@]}" | jq -Rsc 'split("\n") | map(select(length > 0))')"
  fi
  duplicate_count="$(printf '%s\n' "$acknowledgements_json" | jq 'length - (unique | length)')"
  if [[ "$duplicate_count" -ne 0 ]]; then
    sley_report_error_json "TRANSACTION_REVIEW_ACKNOWLEDGEMENT_INVALID" "review acknowledgements must not repeat" "change:approve"
    return 2
  fi
  required_acknowledgements="$(printf '%s\n' "$request" | jq '.derived_scope.review_requirements | unique | sort')"
  acknowledgements_json="$(printf '%s\n' "$acknowledgements_json" | jq 'unique | sort')"
  if [[ "$(printf '%s\n' "$acknowledgements_json" | jq -cS .)" != "$(printf '%s\n' "$required_acknowledgements" | jq -cS .)" ]]; then
    if printf '%s\n' "$acknowledgements_json" | jq -e --argjson required "$required_acknowledgements" 'any(.[]; ($required | index(.)) == null)' >/dev/null; then
      sley_report_error_json "TRANSACTION_GRANT_SCOPE_BROADENED" "review acknowledgements include a requirement outside the compiler-derived request" "change:approve"
    else
      sley_report_error_json "TRANSACTION_REVIEW_ACKNOWLEDGEMENT_REQUIRED" "every compiler-derived review requirement must be acknowledged" "change:approve"
    fi
    return 2
  fi

  local transaction approval_request_ref issuer_json principal_json audience_json candidate allowed bounds replay revocation proofs mutations next_actions
  local replay_key report_core grant_digest grant_id report
  transaction="$(printf '%s\n' "$request" | jq --arg schema "$SCHEMA_CHANGE_GRANT" '
    .transaction
    | .state="approved"
    | .history=["opened","inspected","planned","previewed","awaiting_approval","approved"]
    | .compiler.contract_versions.grant=$schema')"
  approval_request_ref="$(printf '%s\n' "$request" | jq '{
    schema,request_id,request_digest,preview_digest:.preview_ref.preview_digest,
    transaction_id:.transaction.transaction_id
  }')"
  issuer_json="$(jq -n --arg id "$issuer" --arg source "$(printf '%s\n' "$AUTHORITY_LOCAL_GRANT_ISSUER_SOURCES_JSON" | jq -r '.[0]')" '{id:$id,source:$source}')"
  principal_json="$(jq -n --arg id "$principal" '{id:$id}')"
  audience_json="$(jq -n --arg id "$audience" '{id:$id}')"
  candidate="$(printf '%s\n' "$request" | jq '{
    candidate_id:.preview_ref.candidate_id,candidate_digest:.candidate_ref.candidate_digest,
    candidate_source_digest:.candidate_ref.candidate_source_digest,
    candidate_graph_digest:.candidate_ref.candidate_graph_digest,
    source_projection_digest:.candidate_ref.source_projection_digest,
    validation_profile:.candidate_ref.validation.profile
  }')"
  allowed="$(printf '%s\n' "$request" | jq --argjson acknowledgements "$acknowledgements_json" '{
    operation_classes:.derived_scope.operation_classes,nodes:.derived_scope.affected_nodes,
    paths:.derived_scope.allowed_paths,effects:([.derived_scope.effect_changes[].identity] | unique | sort),
    authority_changes:([.derived_scope.authority_changes[].identity] | unique | sort),
    resource_scopes:.derived_scope.resource_scopes,
    review_requirements_acknowledged:$acknowledgements
  }')"
  bounds="$(printf '%s\n' "$request" | jq \
    --argjson max_apply_attempts "$AUTHORITY_LOCAL_GRANT_MAX_APPLY_ATTEMPTS" \
    --argjson max_wall_clock_seconds "$max_wall_clock_seconds" '{
      max_changed_files:.derived_scope.max_changed_files,
      max_source_bytes:.derived_scope.max_source_bytes,
      max_candidate_source_bytes:.derived_scope.max_source_bytes,
      max_apply_attempts:$max_apply_attempts,
      max_wall_clock_seconds:$max_wall_clock_seconds
    }')"
  replay_key="$(jq -n \
    --arg repository_identity "$(printf '%s\n' "$request" | jq -r '.transaction.repository.identity_digest')" \
    --arg base_commit "$(printf '%s\n' "$request" | jq -r '.transaction.base.commit')" \
    --arg candidate_digest "$(printf '%s\n' "$request" | jq -r '.candidate_ref.candidate_digest')" \
    --arg principal "$principal" --arg audience "$audience" --arg nonce "$nonce" '{
      repository_identity:$repository_identity,base_commit:$base_commit,candidate_digest:$candidate_digest,
      principal:$principal,audience:$audience,nonce:$nonce
    }' | json_digest_for)"
  replay="$(jq -n --arg policy "$AUTHORITY_LOCAL_GRANT_REPLAY_POLICY" --arg enforcement "$AUTHORITY_LOCAL_GRANT_REPLAY_ENFORCEMENT" --arg replay_key "$replay_key" '{policy:$policy,enforcement:$enforcement,replay_key:$replay_key}')"
  revocation="$(jq -n --arg state "$revocation_state" --arg checked_at "$created_at" --arg ref "$issuer" --arg kind "$(printf '%s\n' "$AUTHORITY_LOCAL_GRANT_REVOCATION_SOURCE_KINDS_JSON" | jq -r '.[0]')" '{state:$state,checked_at:$checked_at,source:{kind:$kind,ref:$ref}}')"
  proofs="$(jq -n --arg signature "$signature_ref" --arg greynucleus "$greynucleus_proof_ref" \
    --arg signature_kind "$(printf '%s\n' "$AUTHORITY_LOCAL_GRANT_PROOF_KINDS_JSON" | jq -r '.[0]')" \
    --arg greynucleus_kind "$(printf '%s\n' "$AUTHORITY_LOCAL_GRANT_PROOF_KINDS_JSON" | jq -r '.[1]')" \
    --arg status "$(printf '%s\n' "$AUTHORITY_LOCAL_GRANT_PROOF_STATUSES_JSON" | jq -r '.[0]')" '[
    (if $signature == "" then empty else {kind:$signature_kind,ref:$signature,status:$status} end),
    (if $greynucleus == "" then empty else {kind:$greynucleus_kind,ref:$greynucleus,status:$status} end)
  ]')"
  mutations="$(printf '%s\n' "$TRANSACTION_MUTATION_BOUNDARIES_JSON" | jq 'reduce .[] as $name ({}; .[$name] = false)')"
  next_actions='[{"kind":"await_apply_implementation","reason":"S12-403 issues a bound local grant, but apply and replay consumption remain unavailable until S12-404"}]'
  report_core="$(jq -n \
    --arg schema "$SCHEMA_CHANGE_GRANT" --argjson transaction "$transaction" \
    --argjson approval_request_ref "$approval_request_ref" --argjson issuer "$issuer_json" \
    --argjson principal "$principal_json" --argjson audience "$audience_json" \
    --argjson repository "$(printf '%s\n' "$request" | jq '.transaction.repository')" \
    --argjson base "$(printf '%s\n' "$request" | jq '.transaction.base')" \
    --argjson candidate "$candidate" --argjson allowed "$allowed" --argjson bounds "$bounds" \
    --arg purpose "$purpose" --arg nonce "$nonce" --arg created_at "$created_at" --arg expires_at "$expires_at" \
    --argjson replay "$replay" --argjson revocation "$revocation" --argjson proofs "$proofs" \
    --argjson mutations "$mutations" --argjson next_actions "$next_actions" '{
      schema:$schema,status:"approved",transaction:$transaction,approval_request_ref:$approval_request_ref,
      issuer:$issuer,principal:$principal,audience:$audience,repository:$repository,base:$base,candidate:$candidate,
      allowed:$allowed,bounds:$bounds,purpose:$purpose,nonce:$nonce,created_at:$created_at,expires_at:$expires_at,
      replay:$replay,revocation:$revocation,proofs:$proofs,mutations:$mutations,next_actions:$next_actions,issues:[]
    }')"
  grant_digest="$(printf '%s\n' "$report_core" | json_digest_for)"
  grant_id="grant:${grant_digest#sha256:}"
  report="$(printf '%s\n' "$report_core" | jq --arg id "$grant_id" --arg digest "$grant_digest" '{schema,status,grant_id:$id,grant_digest:$digest} + del(.schema,.status)')"
  change_grant_report_from_json "$report"
}

transaction_validate_grant_chain_json() {
  local grant_file="$1" request_file="$2" preview_file="$3" repository="$4" principal="$5" audience="$6" historical="${7:-false}"
  local preview request grant expected_request_output expected_request expected_request_digest expected_request_id
  local expected_grant_digest expected_grant_id expected_transaction expected_request_ref expected_candidate expected_allowed
  local expected_replay_key now_epoch created_epoch expires_epoch transaction_created_epoch transaction_expires_epoch

  transaction_require_regular_artifact "grant" "$grant_file" || return $?
  transaction_require_regular_artifact "approval request" "$request_file" || return $?
  transaction_require_regular_artifact "preview" "$preview_file" || return $?
  if ! contract_jsonschema validate --schema-dir "$ROOT_DIR/docs/schemas" --schema "$SCHEMA_CHANGE_GRANT" "$grant_file" >/dev/null 2>&1; then
    sley_report_error_json "TRANSACTION_GRANT_INVALID" "apply authorization requires a valid local grant artifact" "$grant_file"
    return 2
  fi
  if ! contract_jsonschema validate --schema-dir "$ROOT_DIR/docs/schemas" --schema "$SCHEMA_CHANGE_APPROVAL_REQUEST" "$request_file" >/dev/null 2>&1; then
    sley_report_error_json "TRANSACTION_APPROVAL_REQUEST_INVALID" "apply authorization requires a valid approval request artifact" "$request_file"
    return 2
  fi
  if ! contract_jsonschema validate --schema-dir "$ROOT_DIR/docs/schemas" --schema "$SCHEMA_CHANGE_PREVIEW" "$preview_file" >/dev/null 2>&1; then
    sley_report_error_json "TRANSACTION_PREVIEW_INVALID" "apply authorization requires a valid preview artifact" "$preview_file"
    return 2
  fi

  preview="$(jq -cS . "$preview_file")"
  request="$(jq -cS . "$request_file")"
  grant="$(jq -cS . "$grant_file")"
  expected_request_digest="$(printf '%s\n' "$request" | jq 'del(.request_id,.request_digest)' | json_digest_for)"
  expected_request_id="approval-request:${expected_request_digest#sha256:}"
  if ! printf '%s\n' "$request" | jq -e --arg digest "$expected_request_digest" --arg id "$expected_request_id" \
    '.request_digest == $digest and .request_id == $id' >/dev/null; then
    sley_report_error_json "TRANSACTION_APPROVAL_REQUEST_DIGEST_MISMATCH" "approval request content does not match its bound digest" "$request_file"
    return 2
  fi
  if ! expected_request_output="$(transaction_change_approval_request_json "$preview_file" "$repository" "$historical")"; then
    printf '%s\n' "$expected_request_output"
    return 2
  fi
  expected_request="$(printf '%s\n' "$expected_request_output" | jq -cS .)"
  if [[ "$request" != "$expected_request" ]]; then
    sley_report_error_json "TRANSACTION_APPROVAL_REQUEST_MISMATCH" "grant chain does not contain the exact compiler-derived request for this preview and repository base" "$request_file"
    return 2
  fi

  expected_grant_digest="$(printf '%s\n' "$grant" | jq 'del(.grant_id,.grant_digest)' | json_digest_for)"
  expected_grant_id="grant:${expected_grant_digest#sha256:}"
  if ! printf '%s\n' "$grant" | jq -e --arg digest "$expected_grant_digest" --arg id "$expected_grant_id" \
    '.grant_digest == $digest and .grant_id == $id' >/dev/null; then
    sley_report_error_json "TRANSACTION_GRANT_DIGEST_MISMATCH" "grant content does not match its bound digest" "$grant_file"
    return 2
  fi
  if [[ -z "$principal" || -z "$audience" ]] || ! printf '%s\n' "$grant" | jq -e \
    --arg principal "$principal" --arg audience "$audience" \
    '.principal.id == $principal and .audience.id == $audience' >/dev/null; then
    sley_report_error_json "TRANSACTION_APPLY_PRINCIPAL_MISMATCH" "explicit apply principal and audience must match the approved grant" "change:apply-authorization"
    return 2
  fi

  expected_transaction="$(printf '%s\n' "$request" | jq --arg schema "$SCHEMA_CHANGE_GRANT" '
    .transaction
    | .state="approved"
    | .history=["opened","inspected","planned","previewed","awaiting_approval","approved"]
    | .compiler.contract_versions.grant=$schema')"
  expected_request_ref="$(printf '%s\n' "$request" | jq '{
    schema,request_id,request_digest,preview_digest:.preview_ref.preview_digest,
    transaction_id:.transaction.transaction_id
  }')"
  expected_candidate="$(printf '%s\n' "$request" | jq '{
    candidate_id:.preview_ref.candidate_id,candidate_digest:.candidate_ref.candidate_digest,
    candidate_source_digest:.candidate_ref.candidate_source_digest,
    candidate_graph_digest:.candidate_ref.candidate_graph_digest,
    source_projection_digest:.candidate_ref.source_projection_digest,
    validation_profile:.candidate_ref.validation.profile
  }')"
  expected_allowed="$(printf '%s\n' "$request" | jq '{
    operation_classes:.derived_scope.operation_classes,nodes:.derived_scope.affected_nodes,
    paths:.derived_scope.allowed_paths,effects:([.derived_scope.effect_changes[].identity] | unique | sort),
    authority_changes:([.derived_scope.authority_changes[].identity] | unique | sort),
    resource_scopes:.derived_scope.resource_scopes,
    review_requirements_acknowledged:(.derived_scope.review_requirements | unique | sort)
  }')"
  if ! printf '%s\n' "$grant" | jq -e \
    --argjson transaction "$expected_transaction" \
    --argjson request_ref "$expected_request_ref" \
    --argjson candidate "$expected_candidate" \
    --argjson allowed "$expected_allowed" \
    --argjson request "$request" \
    --argjson max_apply_attempts "$AUTHORITY_LOCAL_GRANT_MAX_APPLY_ATTEMPTS" '
      .transaction == $transaction
      and .approval_request_ref == $request_ref
      and .repository == $request.transaction.repository
      and .base == $request.transaction.base
      and .candidate == $candidate
      and .allowed == $allowed
      and .bounds.max_changed_files == $request.derived_scope.max_changed_files
      and .bounds.max_source_bytes == $request.derived_scope.max_source_bytes
      and .bounds.max_candidate_source_bytes == $request.derived_scope.max_source_bytes
      and .bounds.max_apply_attempts == $max_apply_attempts
      and .revocation.state == "not_revoked"
      and .revocation.source.ref == .issuer.id
      and all(.mutations[]; . == false)
      and (.issues | length) == 0' >/dev/null; then
    sley_report_error_json "TRANSACTION_GRANT_SCOPE_MISMATCH" "grant is not the exact compiler-derived scope for its approval request" "$grant_file"
    return 2
  fi

  expected_replay_key="$(jq -n \
    --arg repository_identity "$(printf '%s\n' "$grant" | jq -r '.repository.identity_digest')" \
    --arg base_commit "$(printf '%s\n' "$grant" | jq -r '.base.commit')" \
    --arg candidate_digest "$(printf '%s\n' "$grant" | jq -r '.candidate.candidate_digest')" \
    --arg principal "$principal" --arg audience "$audience" \
    --arg nonce "$(printf '%s\n' "$grant" | jq -r '.nonce')" '{
      repository_identity:$repository_identity,base_commit:$base_commit,candidate_digest:$candidate_digest,
      principal:$principal,audience:$audience,nonce:$nonce
    }' | json_digest_for)"
  if ! printf '%s\n' "$grant" | jq -e \
    --arg policy "$AUTHORITY_LOCAL_GRANT_REPLAY_POLICY" \
    --arg enforcement "$AUTHORITY_LOCAL_GRANT_REPLAY_ENFORCEMENT" \
    --arg replay_key "$expected_replay_key" \
    '.replay == {policy:$policy,enforcement:$enforcement,replay_key:$replay_key}' >/dev/null; then
    sley_report_error_json "TRANSACTION_REPLAY_KEY_MISMATCH" "grant replay key is not bound to the exact repository, candidate, principal, audience, and nonce" "$grant_file"
    return 2
  fi

  created_epoch="$(date -u -d "$(printf '%s\n' "$grant" | jq -r '.created_at')" +%s 2>/dev/null || true)"
  expires_epoch="$(date -u -d "$(printf '%s\n' "$grant" | jq -r '.expires_at')" +%s 2>/dev/null || true)"
  transaction_created_epoch="$(date -u -d "$(printf '%s\n' "$grant" | jq -r '.transaction.created_at')" +%s 2>/dev/null || true)"
  transaction_expires_epoch="$(date -u -d "$(printf '%s\n' "$grant" | jq -r '.transaction.expires_at')" +%s 2>/dev/null || true)"
  now_epoch="$(date -u +%s)"
  if [[ -z "$created_epoch" || -z "$expires_epoch" || -z "$transaction_created_epoch" || -z "$transaction_expires_epoch" \
    || "$created_epoch" -lt "$transaction_created_epoch" || "$expires_epoch" -gt "$transaction_expires_epoch" \
    || "$expires_epoch" -le "$created_epoch" || ( "$historical" != true && "$expires_epoch" -le "$now_epoch" ) ]]; then
    sley_report_error_json "TRANSACTION_GRANT_EXPIRED" "grant is expired or lies outside the approved transaction time bounds" "$grant_file"
    return 2
  fi

  jq -n --argjson preview "$preview" --argjson request "$request" --argjson grant "$grant" \
    '{preview:$preview,request:$request,grant:$grant}'
}

command_change_apply_authorization() {
  local grant_file="" request_file="" preview_file="" repository="" principal="" audience="" arg
  while [[ "$#" -gt 0 ]]; do
    arg="$1"
    case "$arg" in
      --json) shift ;;
      --grant|--request|--preview|--repository|--principal|--audience)
        if [[ "$#" -lt 2 ]]; then
          sley_report_error_json "TRANSACTION_ARGUMENT_REQUIRED" "$arg requires a value" "change:apply-authorization"
          return 2
        fi
        case "$arg" in
          --grant) grant_file="$2" ;;
          --request) request_file="$2" ;;
          --preview) preview_file="$2" ;;
          --repository) repository="$2" ;;
          --principal) principal="$2" ;;
          --audience) audience="$2" ;;
        esac
        shift 2
        ;;
      --*)
        sley_report_error_json "TRANSACTION_ARGUMENT_UNKNOWN" "unknown change apply-authorization option: $arg" "change:apply-authorization"
        return 2
        ;;
      *)
        sley_report_error_json "TRANSACTION_ARGUMENT_UNKNOWN" "change apply-authorization does not accept positional arguments" "change:apply-authorization"
        return 2
        ;;
    esac
  done

  local chain grant transaction_id_hex replay_key_hex grant_digest_hex
  if ! chain="$(transaction_validate_grant_chain_json "$grant_file" "$request_file" "$preview_file" "$repository" "$principal" "$audience")"; then
    printf '%s\n' "$chain"
    return 2
  fi
  grant="$(printf '%s\n' "$chain" | jq -c '.grant')"
  transaction_id_hex="$(printf '%s\n' "$grant" | jq -r '.transaction.transaction_id | sub("^sha256:";"")')"
  replay_key_hex="$(printf '%s\n' "$grant" | jq -r '.replay.replay_key | sub("^sha256:";"")')"
  grant_digest_hex="$(printf '%s\n' "$grant" | jq -r '.grant_digest | sub("^sha256:";"")')"

  local transaction grant_ref internal_scopes atomicity verification metadata_policy mutation_authority mutations next_actions
  local report_core authorization_digest authorization_id report
  transaction="$(printf '%s\n' "$grant" | jq '.transaction')"
  grant_ref="$(printf '%s\n' "$grant" | jq '{
    schema,grant_id,grant_digest,request_id:.approval_request_ref.request_id,
    preview_digest:.approval_request_ref.preview_digest,transaction_id:.transaction.transaction_id,
    replay_key:.replay.replay_key
  }')"
  internal_scopes="$(printf '%s\n' "$AUTHORITY_APPLY_AUTHORIZATION_INTERNAL_SCOPE_KINDS_JSON" | jq \
    --arg transaction_id "$transaction_id_hex" --arg replay_key "$replay_key_hex" --arg grant_digest "$grant_digest_hex" '[
      {kind:.[0],path:(".sley/transactions/" + $transaction_id)},
      {kind:.[1],path:".sley/locks/change-apply"},
      {kind:.[2],path:(".sley/replay/" + $replay_key)},
      {kind:.[3],path:(".sley/transactions/" + $transaction_id + "/recovery")},
      {kind:.[4],path:(".sley/revocations/" + $grant_digest + ".json")}
    ]')"
  atomicity="$(jq -n \
    --arg platform "linux" \
    --arg strategy "$(printf '%s\n' "$TRANSACTION_ATOMIC_APPLY_STRATEGIES_JSON" | jq -r '.[0]')" \
    --arg topology "$(printf '%s\n' "$TRANSACTION_ATOMIC_APPLY_TOPOLOGIES_JSON" | jq -r '.[0]')" \
    --arg rollback_strategy "$(printf '%s\n' "$TRANSACTION_ATOMIC_ROLLBACK_STRATEGIES_JSON" | jq -r '.[0]')" \
    --arg cancellation_point "$(printf '%s\n' "$TRANSACTION_APPLY_CANCELLATION_POINTS_JSON" | jq -r '.[0]')" '{
      platform:$platform,strategy:$strategy,topology:$topology,rollback_strategy:$rollback_strategy,
      cancellation_point:$cancellation_point,
      limitations:["non_repo_root","single_common_root","no_symlinks","no_special_files","no_submodules","same_filesystem"]
    }')"
  verification="$(jq -n \
    --arg profile "$(printf '%s\n' "$TRANSACTION_APPLY_VERIFICATION_PROFILES_JSON" | jq -r '.[0]')" \
    --argjson required_checks "$TRANSACTION_APPLY_VERIFICATION_REQUIRED_CHECKS_JSON" \
    '{profile:$profile,required_checks:$required_checks,rollback_on_failure:true}')"
  metadata_policy="$(printf '%s\n' "$TRANSACTION_APPLY_METADATA_POLICY_JSON" | jq 'reduce .[] as $row ({}; .[$row.field] = $row.value)')"
  mutation_authority="$(jq -n '{repository_source:true,internal_transaction_state:true,git_index:false,trace_sidecars:false,external_systems:false,provider_state:false}')"
  mutations="$(printf '%s\n' "$TRANSACTION_MUTATION_BOUNDARIES_JSON" | jq 'reduce .[] as $name ({}; .[$name] = false)')"
  next_actions="$(jq -n --arg auth "<apply-authorization.json>" --arg grant "$grant_file" --arg repo "$repository" '[{
    kind:"initialize_revocation_record",
    reason:"create the exact local revocation ledger record before one-use apply",
    command:["sley","change","revocation-record","--json","--authorization",$auth,"--grant",$grant,"--repository",$repo]
  }]')"
  report_core="$(jq -n \
    --arg schema "$SCHEMA_CHANGE_APPLY_AUTHORIZATION" --argjson transaction "$transaction" \
    --argjson grant_ref "$grant_ref" \
    --argjson principal "$(printf '%s\n' "$grant" | jq '.principal')" \
    --argjson audience "$(printf '%s\n' "$grant" | jq '.audience')" \
    --argjson repository_json "$(printf '%s\n' "$grant" | jq '.repository')" \
    --argjson base "$(printf '%s\n' "$grant" | jq '.base')" \
    --argjson candidate "$(printf '%s\n' "$grant" | jq '.candidate')" \
    --argjson source_scope "$(printf '%s\n' "$grant" | jq '.allowed.resource_scopes[0]')" \
    --argjson internal_scopes "$internal_scopes" --argjson atomicity "$atomicity" \
    --argjson verification "$verification" --argjson metadata_policy "$metadata_policy" \
    --argjson mutation_authority "$mutation_authority" --argjson mutations "$mutations" \
    --argjson next_actions "$next_actions" '{
      schema:$schema,status:"ready",transaction:$transaction,grant_ref:$grant_ref,
      principal:$principal,audience:$audience,repository:$repository_json,base:$base,candidate:$candidate,
      source_scope:$source_scope,internal_scopes:$internal_scopes,atomicity:$atomicity,
      verification:$verification,metadata_policy:$metadata_policy,mutation_authority:$mutation_authority,
      mutations:$mutations,next_actions:$next_actions,issues:[]
    }')"
  authorization_digest="$(printf '%s\n' "$report_core" | json_digest_for)"
  authorization_id="apply-authorization:${authorization_digest#sha256:}"
  report="$(printf '%s\n' "$report_core" | jq --arg id "$authorization_id" --arg digest "$authorization_digest" \
    '{schema,status,authorization_id:$id,authorization_digest:$digest} + del(.schema,.status)')"
  change_apply_authorization_report_from_json "$report"
}

transaction_validate_apply_authorization_json() {
  local authorization_file="$1" grant_file="$2" repository="$3"
  local authorization grant expected_authorization_digest expected_authorization_id
  local expected_grant_digest expected_grant_id binding transaction_id_hex replay_key_hex grant_digest_hex expected_internal_scopes

  transaction_require_regular_artifact "apply authorization" "$authorization_file" || return $?
  transaction_require_regular_artifact "grant" "$grant_file" || return $?
  if ! contract_jsonschema validate --schema-dir "$ROOT_DIR/docs/schemas" --schema "$SCHEMA_CHANGE_APPLY_AUTHORIZATION" "$authorization_file" >/dev/null 2>&1; then
    sley_report_error_json "TRANSACTION_APPLY_AUTHORIZATION_INVALID" "operation requires a valid apply authorization artifact" "$authorization_file"
    return 2
  fi
  if ! contract_jsonschema validate --schema-dir "$ROOT_DIR/docs/schemas" --schema "$SCHEMA_CHANGE_GRANT" "$grant_file" >/dev/null 2>&1; then
    sley_report_error_json "TRANSACTION_GRANT_INVALID" "operation requires a valid local grant artifact" "$grant_file"
    return 2
  fi
  authorization="$(jq -cS . "$authorization_file")"
  grant="$(jq -cS . "$grant_file")"
  expected_authorization_digest="$(printf '%s\n' "$authorization" | jq 'del(.authorization_id,.authorization_digest)' | json_digest_for)"
  expected_authorization_id="apply-authorization:${expected_authorization_digest#sha256:}"
  if ! printf '%s\n' "$authorization" | jq -e --arg digest "$expected_authorization_digest" --arg id "$expected_authorization_id" \
    '.authorization_digest == $digest and .authorization_id == $id' >/dev/null; then
    sley_report_error_json "TRANSACTION_APPLY_AUTHORIZATION_DIGEST_MISMATCH" "apply authorization content does not match its bound digest" "$authorization_file"
    return 2
  fi
  expected_grant_digest="$(printf '%s\n' "$grant" | jq 'del(.grant_id,.grant_digest)' | json_digest_for)"
  expected_grant_id="grant:${expected_grant_digest#sha256:}"
  if ! printf '%s\n' "$grant" | jq -e --arg digest "$expected_grant_digest" --arg id "$expected_grant_id" \
    '.grant_digest == $digest and .grant_id == $id' >/dev/null; then
    sley_report_error_json "TRANSACTION_GRANT_DIGEST_MISMATCH" "grant content does not match its bound digest" "$grant_file"
    return 2
  fi
  if ! binding="$(transaction_repository_identity_binding_json "$repository")"; then
    printf '%s\n' "$binding"
    return 2
  fi
  transaction_id_hex="$(printf '%s\n' "$grant" | jq -r '.transaction.transaction_id | sub("^sha256:";"")')"
  replay_key_hex="$(printf '%s\n' "$grant" | jq -r '.replay.replay_key | sub("^sha256:";"")')"
  grant_digest_hex="$(printf '%s\n' "$grant" | jq -r '.grant_digest | sub("^sha256:";"")')"
  expected_internal_scopes="$(printf '%s\n' "$AUTHORITY_APPLY_AUTHORIZATION_INTERNAL_SCOPE_KINDS_JSON" | jq \
    --arg transaction_id "$transaction_id_hex" --arg replay_key "$replay_key_hex" --arg grant_digest "$grant_digest_hex" '[
      {kind:.[0],path:(".sley/transactions/" + $transaction_id)},
      {kind:.[1],path:".sley/locks/change-apply"},
      {kind:.[2],path:(".sley/replay/" + $replay_key)},
      {kind:.[3],path:(".sley/transactions/" + $transaction_id + "/recovery")},
      {kind:.[4],path:(".sley/revocations/" + $grant_digest + ".json")}
    ]')"
  if ! printf '%s\n' "$authorization" | jq -e \
    --argjson grant "$grant" --argjson binding "$binding" --argjson internal_scopes "$expected_internal_scopes" \
    --arg strategy "$(printf '%s\n' "$TRANSACTION_ATOMIC_APPLY_STRATEGIES_JSON" | jq -r '.[0]')" \
    --arg topology "$(printf '%s\n' "$TRANSACTION_ATOMIC_APPLY_TOPOLOGIES_JSON" | jq -r '.[0]')" \
    --arg rollback_strategy "$(printf '%s\n' "$TRANSACTION_ATOMIC_ROLLBACK_STRATEGIES_JSON" | jq -r '.[0]')" \
    --arg cancellation_point "$(printf '%s\n' "$TRANSACTION_APPLY_CANCELLATION_POINTS_JSON" | jq -r '.[0]')" \
    --arg profile "$(printf '%s\n' "$TRANSACTION_APPLY_VERIFICATION_PROFILES_JSON" | jq -r '.[0]')" \
    --argjson required_checks "$TRANSACTION_APPLY_VERIFICATION_REQUIRED_CHECKS_JSON" \
    --argjson metadata_policy "$(printf '%s\n' "$TRANSACTION_APPLY_METADATA_POLICY_JSON" | jq 'reduce .[] as $row ({}; .[$row.field] = $row.value)')" '
      .grant_ref == {
        schema:$grant.schema,grant_id:$grant.grant_id,grant_digest:$grant.grant_digest,
        request_id:$grant.approval_request_ref.request_id,preview_digest:$grant.approval_request_ref.preview_digest,
        transaction_id:$grant.transaction.transaction_id,replay_key:$grant.replay.replay_key
      }
      and .transaction == $grant.transaction
      and .principal == $grant.principal
      and .audience == $grant.audience
      and .repository == $grant.repository
      and .repository.identity_digest == $binding.identity_digest
      and .base == $grant.base
      and .candidate == $grant.candidate
      and .source_scope == $grant.allowed.resource_scopes[0]
      and .internal_scopes == $internal_scopes
      and .atomicity == {
        platform:"linux",strategy:$strategy,topology:$topology,rollback_strategy:$rollback_strategy,
        cancellation_point:$cancellation_point,
        limitations:["non_repo_root","single_common_root","no_symlinks","no_special_files","no_submodules","same_filesystem"]
      }
      and .verification == {profile:$profile,required_checks:$required_checks,rollback_on_failure:true}
      and .metadata_policy == $metadata_policy
      and .mutation_authority == {repository_source:true,internal_transaction_state:true,git_index:false,trace_sidecars:false,external_systems:false,provider_state:false}
      and all(.mutations[]; . == false)
      and (.issues | length) == 0' >/dev/null; then
    sley_report_error_json "TRANSACTION_APPLY_AUTHORIZATION_SCOPE_MISMATCH" "apply authorization is not the exact fixed source and internal scope derived from its grant" "$authorization_file"
    return 2
  fi
  jq -n --argjson authorization "$authorization" --argjson grant "$grant" --argjson binding "$binding" \
    '{authorization:$authorization,grant:$grant,binding:$binding}'
}

transaction_internal_atomic_write() {
  local repository="$1" relative_path="$2" replace="$3" document="$4"
  python3 - "$repository" "$relative_path" "$replace" 3< <(printf '%s\n' "$document") <<'PY'
import errno
import os
import stat
import sys

repository, relative_path, replace = sys.argv[1], sys.argv[2], sys.argv[3] == "true"
payload = os.fdopen(3, "rb", closefd=False).read()
if not relative_path or relative_path.startswith("/") or ".." in relative_path.split("/"):
    raise SystemExit("unsafe internal transaction path")
parts = [part for part in relative_path.split("/") if part not in ("", ".")]
if not parts or parts[0] != ".sley":
    raise SystemExit("internal transaction path must remain under .sley")

flags = os.O_RDONLY | os.O_DIRECTORY | os.O_CLOEXEC | os.O_NOFOLLOW
directory_fd = os.open(repository, flags)
try:
    for component in parts[:-1]:
        try:
            next_fd = os.open(component, flags, dir_fd=directory_fd)
        except FileNotFoundError:
            os.mkdir(component, 0o700, dir_fd=directory_fd)
            next_fd = os.open(component, flags, dir_fd=directory_fd)
        os.close(directory_fd)
        directory_fd = next_fd

    name = parts[-1]
    try:
        existing = os.stat(name, dir_fd=directory_fd, follow_symlinks=False)
    except FileNotFoundError:
        existing = None
    if existing is not None and not stat.S_ISREG(existing.st_mode):
        raise SystemExit("internal transaction target is not a regular file")
    if existing is not None and not replace:
        raise SystemExit("internal transaction target already exists")

    temporary = "." + name + ".tmp." + str(os.getpid())
    file_fd = os.open(
        temporary,
        os.O_WRONLY | os.O_CREAT | os.O_EXCL | os.O_CLOEXEC | os.O_NOFOLLOW,
        0o600,
        dir_fd=directory_fd,
    )
    try:
        view = memoryview(payload)
        while view:
            written = os.write(file_fd, view)
            view = view[written:]
        os.fsync(file_fd)
    finally:
        os.close(file_fd)
    try:
        if replace:
            os.replace(temporary, name, src_dir_fd=directory_fd, dst_dir_fd=directory_fd)
        else:
            os.link(temporary, name, src_dir_fd=directory_fd, dst_dir_fd=directory_fd, follow_symlinks=False)
            os.unlink(temporary, dir_fd=directory_fd)
        os.fsync(directory_fd)
    except BaseException:
        try:
            os.unlink(temporary, dir_fd=directory_fd)
        except FileNotFoundError:
            pass
        raise
finally:
    os.close(directory_fd)
PY
}

transaction_common_path_for_json_array() {
  local paths_json="$1"
  python3 - 3< <(printf '%s\n' "$paths_json") <<'PY'
import json
import os
import sys

paths = json.load(os.fdopen(3, "r", closefd=False))
if not paths:
    raise SystemExit(2)
if len(paths) == 1:
    print(paths[0])
else:
    print(os.path.commonpath([os.path.dirname(path) for path in paths]))
PY
}

transaction_internal_mkdir() {
  local repository="$1" relative_path="$2" exclusive="$3"
  python3 - "$repository" "$relative_path" "$exclusive" <<'PY'
import os
import sys

repository, relative_path, exclusive = sys.argv[1], sys.argv[2], sys.argv[3] == "true"
if not relative_path or relative_path.startswith("/") or ".." in relative_path.split("/"):
    raise SystemExit("unsafe internal transaction directory")
parts = [part for part in relative_path.split("/") if part not in ("", ".")]
if not parts or parts[0] != ".sley":
    raise SystemExit("internal transaction directory must remain under .sley")
flags = os.O_RDONLY | os.O_DIRECTORY | os.O_CLOEXEC | os.O_NOFOLLOW
directory_fd = os.open(repository, flags)
try:
    for index, component in enumerate(parts):
        final = index == len(parts) - 1
        try:
            next_fd = os.open(component, flags, dir_fd=directory_fd)
            if final and exclusive:
                os.close(next_fd)
                raise FileExistsError(component)
        except FileNotFoundError:
            os.mkdir(component, 0o700, dir_fd=directory_fd)
            next_fd = os.open(component, flags, dir_fd=directory_fd)
            os.fsync(directory_fd)
        os.close(directory_fd)
        directory_fd = next_fd
finally:
    os.close(directory_fd)
PY
}

transaction_internal_rmdir() {
  local repository="$1" relative_path="$2"
  python3 - "$repository" "$relative_path" <<'PY'
import os
import sys

repository, relative_path = sys.argv[1], sys.argv[2]
parts = [part for part in relative_path.split("/") if part not in ("", ".")]
if not parts or parts[0] != ".sley" or ".." in parts:
    raise SystemExit(2)
flags = os.O_RDONLY | os.O_DIRECTORY | os.O_CLOEXEC | os.O_NOFOLLOW
directory_fd = os.open(repository, flags)
try:
    for component in parts[:-1]:
        next_fd = os.open(component, flags, dir_fd=directory_fd)
        os.close(directory_fd)
        directory_fd = next_fd
    os.rmdir(parts[-1], dir_fd=directory_fd)
    os.fsync(directory_fd)
finally:
    os.close(directory_fd)
PY
}

transaction_fsync_tree() {
  local target="$1"
  python3 - "$target" <<'PY'
import os
import stat
import sys

root = sys.argv[1]
if os.path.isfile(root):
    fd = os.open(root, os.O_RDONLY | os.O_CLOEXEC | os.O_NOFOLLOW)
    try:
        os.fsync(fd)
    finally:
        os.close(fd)
    root = os.path.dirname(root)
for current, directories, files in os.walk(root, topdown=False, followlinks=False):
    for name in files:
        path = os.path.join(current, name)
        mode = os.lstat(path).st_mode
        if not stat.S_ISREG(mode):
            raise SystemExit("non-regular file in transaction tree")
        fd = os.open(path, os.O_RDONLY | os.O_CLOEXEC | os.O_NOFOLLOW)
        try:
            os.fsync(fd)
        finally:
            os.close(fd)
    fd = os.open(current, os.O_RDONLY | os.O_DIRECTORY | os.O_CLOEXEC | os.O_NOFOLLOW)
    try:
        os.fsync(fd)
    finally:
        os.close(fd)
PY
}

transaction_rename_exchange() {
  local repository="$1" left="$2" right="$3"
  python3 - "$repository" "$left" "$right" <<'PY'
import ctypes
import os
import stat
import sys

repository, left, right = sys.argv[1], sys.argv[2], sys.argv[3]
for relative in (left, right):
    if not relative or relative.startswith("/") or ".." in relative.split("/"):
        raise SystemExit("unsafe exchange path")

directory_flags = os.O_RDONLY | os.O_DIRECTORY | os.O_CLOEXEC | os.O_NOFOLLOW
root_fd = os.open(repository, directory_flags)

def open_parent(relative):
    parts = [part for part in relative.split("/") if part not in ("", ".")]
    current_fd = os.dup(root_fd)
    try:
        for component in parts[:-1]:
            next_fd = os.open(component, directory_flags, dir_fd=current_fd)
            os.close(current_fd)
            current_fd = next_fd
        return current_fd, parts[-1]
    except BaseException:
        os.close(current_fd)
        raise

left_parent_fd, left_name = open_parent(left)
right_parent_fd, right_name = open_parent(right)
left_before = os.stat(left_name, dir_fd=left_parent_fd, follow_symlinks=False)
right_before = os.stat(right_name, dir_fd=right_parent_fd, follow_symlinks=False)
allowed = (stat.S_ISREG, stat.S_ISDIR)
if not any(check(left_before.st_mode) for check in allowed) or not any(check(right_before.st_mode) for check in allowed):
    raise SystemExit("exchange leaves must be regular files or directories")
if stat.S_IFMT(left_before.st_mode) != stat.S_IFMT(right_before.st_mode):
    raise SystemExit("exchange leaves must have the same filesystem type")
if left_before.st_dev != right_before.st_dev:
    raise SystemExit("exchange leaves must be on the same filesystem")

path_flags = os.O_PATH | os.O_CLOEXEC | os.O_NOFOLLOW
left_leaf_fd = os.open(left_name, path_flags, dir_fd=left_parent_fd)
right_leaf_fd = os.open(right_name, path_flags, dir_fd=right_parent_fd)
libc = ctypes.CDLL(None, use_errno=True)
renameat2 = getattr(libc, "renameat2", None)
if renameat2 is None:
    raise SystemExit("renameat2 is unavailable")
renameat2.argtypes = [ctypes.c_int, ctypes.c_char_p, ctypes.c_int, ctypes.c_char_p, ctypes.c_uint]
renameat2.restype = ctypes.c_int
try:
    if renameat2(left_parent_fd, os.fsencode(left_name), right_parent_fd, os.fsencode(right_name), 2) != 0:
        error = ctypes.get_errno()
        raise OSError(error, os.strerror(error))
    left_after = os.stat(left_name, dir_fd=left_parent_fd, follow_symlinks=False)
    right_after = os.stat(right_name, dir_fd=right_parent_fd, follow_symlinks=False)
    if (left_after.st_dev, left_after.st_ino) != (right_before.st_dev, right_before.st_ino):
        raise SystemExit("left exchange identity does not match the pinned candidate")
    if (right_after.st_dev, right_after.st_ino) != (left_before.st_dev, left_before.st_ino):
        raise SystemExit("right exchange identity does not match the pinned preimage")
    os.fsync(left_parent_fd)
    if right_parent_fd != left_parent_fd:
        os.fsync(right_parent_fd)
finally:
    os.close(left_leaf_fd)
    os.close(right_leaf_fd)
    os.close(left_parent_fd)
    os.close(right_parent_fd)
    os.close(root_fd)
PY
}

transaction_resolve_apply_target_json() {
  local preview="$1" repository="$2" base_digest paths_json path count candidate candidate_rel digest matches='[]'
  base_digest="$(printf '%s\n' "$preview" | jq -r '.transaction.base.source_digest')"
  paths_json="$(printf '%s\n' "$preview" | jq '.candidate.source_projection | map(.path) | unique | sort')"
  count="$(printf '%s\n' "$paths_json" | jq 'length')"
  if [[ "$count" -lt 1 ]]; then
    sley_report_error_json "TRANSACTION_CANDIDATE_PROJECTION_EMPTY" "apply requires a non-empty candidate source projection" "change:apply"
    return 2
  fi
  while IFS= read -r path; do
    if [[ "$path" == /* || "$path" == *'/../'* || "$path" == '../'* || "$path" == *'/..' || ! -f "$repository/$path" || -L "$repository/$path" ]]; then
      sley_report_error_json "TRANSACTION_CANDIDATE_PATH_UNSAFE" "candidate projection path is missing, unsafe, or not a regular source file" "$path"
      return 2
    fi
  done < <(printf '%s\n' "$paths_json" | jq -r '.[]')
  if [[ "$count" -eq 1 ]]; then
    candidate_rel="$(printf '%s\n' "$paths_json" | jq -r '.[0]')"
    candidate="$repository/$candidate_rel"
    digest="$(structural_source_digest_for "$candidate")"
    if [[ "$digest" != "$base_digest" ]]; then
      sley_report_error_json "TRANSACTION_STALE" "the single-file apply target no longer matches the approved base source digest" "$candidate_rel"
      return 2
    fi
    jq -n --arg path "$candidate_rel" --arg digest "$digest" '{target:$path,source_digest:$digest}'
    return
  fi
  candidate_rel="$(transaction_common_path_for_json_array "$paths_json")"
  while [[ -n "$candidate_rel" && "$candidate_rel" != "." && "$candidate_rel" != "/" ]]; do
    candidate="$repository/$candidate_rel"
    if [[ -d "$candidate" && ! -L "$candidate" ]]; then
      digest="$(structural_source_digest_for "$candidate")"
      if [[ "$digest" == "$base_digest" ]]; then
        matches="$(printf '%s\n' "$matches" | jq --arg path "$candidate_rel" --arg digest "$digest" '. + [{target:$path,source_digest:$digest}]')"
      fi
    fi
    candidate_rel="$(dirname -- "$candidate_rel")"
  done
  if [[ "$(printf '%s\n' "$matches" | jq 'length')" -eq 0 ]]; then
    sley_report_error_json "TRANSACTION_STALE" "no non-root repository target matches the approved base source digest" "change:apply"
    return 2
  fi
  if [[ "$(printf '%s\n' "$matches" | jq 'length')" -ne 1 ]]; then
    sley_report_error_json "TRANSACTION_TARGET_AMBIGUOUS" "approved base digest does not resolve to exactly one non-root repository target" "change:apply"
    return 2
  fi
  printf '%s\n' "$matches" | jq '.[0]'
}

transaction_validate_apply_topology_json() {
  local preview="$1" grant="$2" repository="$3" target target_rel common_rel common_abs target_abs allowed_json projection_json
  local source_digest graph_digest staged submodule_path
  if [[ "$(uname -s)" != "Linux" ]]; then
    sley_report_error_json "TRANSACTION_ATOMIC_PLATFORM_UNSUPPORTED" "atomic apply v0 requires Linux renameat2 support" "change:apply"
    return 2
  fi
  if ! target="$(transaction_resolve_apply_target_json "$preview" "$repository")"; then
    printf '%s\n' "$target"
    return 2
  fi
  target_rel="$(printf '%s\n' "$target" | jq -r '.target')"
  target_abs="$repository/$target_rel"
  allowed_json="$(printf '%s\n' "$grant" | jq '.allowed.paths | unique | sort')"
  projection_json="$(printf '%s\n' "$preview" | jq '.candidate.source_projection | map(.path) | unique | sort')"
  if ! printf '%s\n' "$allowed_json" | jq -e --arg target "$target_rel" --argjson projection "$projection_json" '
    length > 0 and all(.[]; (. == $target or startswith($target + "/") or ($target | endswith(".sley"))) and ($projection | index(.)) != null)' >/dev/null; then
    sley_report_error_json "TRANSACTION_OWNED_PATH_VIOLATION" "approved source paths do not remain inside the resolved target and candidate projection" "change:apply"
    return 2
  fi
  common_rel="$(transaction_common_path_for_json_array "$allowed_json")"
  if [[ -z "$common_rel" || "$common_rel" == "." || "$common_rel" == ".git" || "$common_rel" == ".sley" \
    || "$common_rel" == .git/* || "$common_rel" == .sley/* ]]; then
    sley_report_error_json "TRANSACTION_ATOMIC_TOPOLOGY_UNSUPPORTED" "atomic apply requires one non-repository common source root outside .git and .sley" "change:apply"
    return 2
  fi
  common_abs="$repository/$common_rel"
  if [[ ! -e "$common_abs" || -L "$common_abs" ]] || find "$target_abs" -type l -print -quit 2>/dev/null | grep -q .; then
    sley_report_error_json "TRANSACTION_ATOMIC_SYMLINK_UNSUPPORTED" "atomic apply rejects symlinks in the target or common root" "$common_rel"
    return 2
  fi
  if find "$target_abs" -mindepth 1 ! -type f ! -type d -print -quit 2>/dev/null | grep -q .; then
    sley_report_error_json "TRANSACTION_ATOMIC_SPECIAL_FILE_UNSUPPORTED" "atomic apply rejects special files in the target" "$target_rel"
    return 2
  fi
  if python3 - "$target_abs" <<'PY'
import os
import sys
if os.listxattr(sys.argv[1], follow_symlinks=False):
    raise SystemExit(0)
for current, directories, files in os.walk(sys.argv[1], followlinks=False):
    for name in directories + files:
        if os.listxattr(os.path.join(current, name), follow_symlinks=False):
            raise SystemExit(0)
raise SystemExit(1)
PY
  then
    sley_report_error_json "TRANSACTION_ATOMIC_XATTR_UNSUPPORTED" "atomic apply rejects targets containing extended attributes" "$target_rel"
    return 2
  fi
  while IFS= read -r submodule_path; do
    [[ -n "$submodule_path" ]] || continue
    if [[ "$submodule_path" == "$target_rel" || "$submodule_path" == "$target_rel/"* || "$target_rel" == "$submodule_path/"* ]]; then
      sley_report_error_json "TRANSACTION_ATOMIC_SUBMODULE_UNSUPPORTED" "atomic apply rejects targets containing or nested in a Git submodule" "$submodule_path"
      return 2
    fi
  done < <(git -C "$repository" ls-files --stage | awk '$1 == "160000" {print $4}')
  if ! git -C "$repository" diff --cached --quiet -- "$common_rel"; then
    sley_report_error_json "TRANSACTION_GIT_INDEX_DIRTY" "atomic apply refuses a common root with staged index changes" "$common_rel"
    return 2
  fi
  source_digest="$(structural_source_digest_for "$target_abs")"
  graph_digest="$(graph_json_for "$target_abs" | json_digest_for)"
  if [[ "$source_digest" != "$(printf '%s\n' "$grant" | jq -r '.base.source_digest')" \
    || "$graph_digest" != "$(printf '%s\n' "$grant" | jq -r '.base.graph_digest')" ]]; then
    sley_report_error_json "TRANSACTION_STALE" "resolved apply target no longer matches the approved base source and graph digests" "$target_rel"
    return 2
  fi
  jq -n --arg target "$target_rel" --arg common_root "$common_rel" \
    --arg source_digest "$source_digest" --arg graph_digest "$graph_digest" \
    '{target:$target,common_root:$common_root,source_digest:$source_digest,graph_digest:$graph_digest}'
}

transaction_validate_revocation_record_json() {
  local record_file="$1" authorization="$2" grant="$3" expected_digest expected_id now_epoch expires_epoch record
  if [[ -L "$record_file" || ! -f "$record_file" ]]; then
    sley_report_error_json "TRANSACTION_REVOCATION_RECORD_REQUIRED" "apply requires the exact local revocation record" "$record_file"
    return 2
  fi
  if ! contract_jsonschema validate --schema-dir "$ROOT_DIR/docs/schemas" --schema "$SCHEMA_CHANGE_REVOCATION_RECORD" "$record_file" >/dev/null 2>&1; then
    sley_report_error_json "TRANSACTION_REVOCATION_RECORD_INVALID" "apply requires a valid local revocation record" "$record_file"
    return 2
  fi
  record="$(jq -cS . "$record_file")"
  expected_digest="$(printf '%s\n' "$record" | jq 'del(.record_id,.record_digest)' | json_digest_for)"
  expected_id="revocation-record:${expected_digest#sha256:}"
  if ! printf '%s\n' "$record" | jq -e --arg digest "$expected_digest" --arg id "$expected_id" \
    --arg authorization_digest "$(printf '%s\n' "$authorization" | jq -r '.authorization_digest')" \
    --arg grant_digest "$(printf '%s\n' "$grant" | jq -r '.grant_digest')" '
      .record_digest == $digest and .record_id == $id
      and .authorization_ref.authorization_digest == $authorization_digest
      and .grant_ref.grant_digest == $grant_digest' >/dev/null; then
    sley_report_error_json "TRANSACTION_REVOCATION_RECORD_MISMATCH" "revocation record is not bound to the exact apply authorization and grant" "$record_file"
    return 2
  fi
  if [[ "$(printf '%s\n' "$record" | jq -r '.state')" != "not_revoked" ]]; then
    sley_report_error_json "TRANSACTION_GRANT_REVOKED" "the exact local grant has been revoked" "$record_file"
    return 2
  fi
  expires_epoch="$(date -u -d "$(printf '%s\n' "$record" | jq -r '.expires_at')" +%s 2>/dev/null || true)"
  now_epoch="$(date -u +%s)"
  if [[ -z "$expires_epoch" || "$expires_epoch" -le "$now_epoch" ]]; then
    sley_report_error_json "TRANSACTION_REVOCATION_RECORD_EXPIRED" "local revocation record has expired" "$record_file"
    return 2
  fi
  printf '%s\n' "$record"
}

transaction_recovery_reseal_json() {
  local record="$1" core digest
  core="$(printf '%s\n' "$record" | jq 'del(.record_digest)')"
  digest="$(printf '%s\n' "$core" | json_digest_for)"
  printf '%s\n' "$core" | jq --arg digest "$digest" '.record_digest=$digest'
}

transaction_recovery_advance_json() {
  local record="$1" phase="$2" state="$3" status="$4" at="$5" detail="$6"
  record="$(printf '%s\n' "$record" | jq \
    --arg phase "$phase" --arg state "$state" --arg status "$status" --arg at "$at" --arg detail "$detail" '
      .phase=$phase | .status=$status | .transaction.state=$state
      | (if (.transaction.history[-1] // "") == $state then . else .transaction.history += [$state] end)
      | .events += [{sequence:((.events | length) + 1),phase:$phase,at:$at,detail:$detail}]')"
  transaction_recovery_reseal_json "$record"
}

transaction_verification_json() {
  local target="$1" grant="$2" source_digest graph_digest check lint check_status lint_status verification_status
  source_digest="$(structural_source_digest_for "$target")"
  graph_digest="$(graph_json_for "$target" | json_digest_for)"
  check="$(check_json "$target")"
  lint="$(lint_json "$target")"
  check_status="$(printf '%s\n' "$check" | jq -r --arg ok "$CHECK_OK_STATUS" 'if .status == $ok then "passed" else "failed" end')"
  lint_status="$(printf '%s\n' "$lint" | jq -r --arg ok "$LINT_OK_STATUS" 'if .status == $ok then "passed" else "failed" end')"
  verification_status=passed
  if [[ "$source_digest" != "$(printf '%s\n' "$grant" | jq -r '.candidate.candidate_source_digest')" \
    || "$graph_digest" != "$(printf '%s\n' "$grant" | jq -r '.candidate.candidate_graph_digest')" \
    || "$check_status" != passed || "$lint_status" != passed ]]; then
    verification_status=failed
  fi
  jq -n --arg status "$verification_status" \
    --arg expected_source "$(printf '%s\n' "$grant" | jq -r '.candidate.candidate_source_digest')" --arg actual_source "$source_digest" \
    --arg expected_graph "$(printf '%s\n' "$grant" | jq -r '.candidate.candidate_graph_digest')" --arg actual_graph "$graph_digest" \
    --arg check_status "$check_status" --arg check_actual "$(printf '%s\n' "$check" | jq -r '.status')" \
    --arg lint_status "$lint_status" --arg lint_actual "$(printf '%s\n' "$lint" | jq -r '.status')" '{
      profile:"compiler_apply_v0",status:$status,checks:[
        {kind:"candidate_source_digest",status:(if $expected_source == $actual_source then "passed" else "failed" end),expected:$expected_source,actual:$actual_source},
        {kind:"candidate_graph_digest",status:(if $expected_graph == $actual_graph then "passed" else "failed" end),expected:$expected_graph,actual:$actual_graph},
        {kind:"compiler_check",status:$check_status,expected:"ok",actual:$check_actual},
        {kind:"compiler_lint",status:$lint_status,expected:"clean",actual:$lint_actual}
      ]
    }'
}

command_change_revocation_record() {
  local authorization_file="" grant_file="" repository="" state="" issuer="" reason="" created_at="" expires_at="" arg
  local confirm_revocation_record=false
  while [[ "$#" -gt 0 ]]; do
    arg="$1"
    case "$arg" in
      --json) shift ;;
      --confirm-revocation-record) confirm_revocation_record=true; shift ;;
      --authorization|--grant|--repository|--state|--issuer|--reason|--created-at|--expires-at)
        if [[ "$#" -lt 2 ]]; then
          sley_report_error_json "TRANSACTION_ARGUMENT_REQUIRED" "$arg requires a value" "change:revocation-record"
          return 2
        fi
        case "$arg" in
          --authorization) authorization_file="$2" ;;
          --grant) grant_file="$2" ;;
          --repository) repository="$2" ;;
          --state) state="$2" ;;
          --issuer) issuer="$2" ;;
          --reason) reason="$2" ;;
          --created-at) created_at="$2" ;;
          --expires-at) expires_at="$2" ;;
        esac
        shift 2
        ;;
      --*)
        sley_report_error_json "TRANSACTION_ARGUMENT_UNKNOWN" "unknown change revocation-record option: $arg" "change:revocation-record"
        return 2
        ;;
      *)
        sley_report_error_json "TRANSACTION_ARGUMENT_UNKNOWN" "change revocation-record does not accept positional arguments" "change:revocation-record"
        return 2
        ;;
    esac
  done
  if [[ "$confirm_revocation_record" != true ]]; then
    sley_report_error_json "TRANSACTION_REVOCATION_CONFIRMATION_REQUIRED" "revocation-record requires --confirm-revocation-record" "change:revocation-record"
    return 2
  fi
  if [[ -z "$state" || -z "$issuer" || -z "$reason" || -z "$created_at" || -z "$expires_at" ]]; then
    sley_report_error_json "TRANSACTION_REVOCATION_FIELDS_REQUIRED" "revocation-record requires state, issuer, reason, created-at, and expires-at" "change:revocation-record"
    return 2
  fi
  if ! printf '%s\n' "$AUTHORITY_LOCAL_REVOCATION_RECORD_STATES_JSON" | jq -e --arg state "$state" 'index($state) != null' >/dev/null; then
    sley_report_error_json "TRANSACTION_REVOCATION_STATE_INVALID" "revocation record state must be not_revoked or revoked" "change:revocation-record"
    return 2
  fi
  if [[ "${#reason}" -gt 4096 || ! "$issuer" =~ ^[A-Za-z0-9._:@/-]{1,256}$ ]]; then
    sley_report_error_json "TRANSACTION_REVOCATION_ASSERTION_INVALID" "revocation issuer or reason is invalid" "change:revocation-record"
    return 2
  fi
  local created_epoch expires_epoch chain authorization grant ledger_path ledger_abs existing="" replace=false
  created_epoch="$(date -u -d "$created_at" +%s 2>/dev/null || true)"
  expires_epoch="$(date -u -d "$expires_at" +%s 2>/dev/null || true)"
  if [[ -z "$created_epoch" || -z "$expires_epoch" || "$expires_epoch" -le "$created_epoch" ]]; then
    sley_report_error_json "TRANSACTION_TIME_ORDER_INVALID" "revocation expiry must be later than record creation" "change:revocation-record"
    return 2
  fi
  if ! chain="$(transaction_validate_apply_authorization_json "$authorization_file" "$grant_file" "$repository")"; then
    printf '%s\n' "$chain"
    return 2
  fi
  authorization="$(printf '%s\n' "$chain" | jq -c '.authorization')"
  grant="$(printf '%s\n' "$chain" | jq -c '.grant')"
  if [[ "$issuer" != "$(printf '%s\n' "$grant" | jq -r '.issuer.id')" ]]; then
    sley_report_error_json "TRANSACTION_REVOCATION_ISSUER_MISMATCH" "revocation issuer must match the grant issuer" "change:revocation-record"
    return 2
  fi
  if [[ "$expires_at" != "$(printf '%s\n' "$grant" | jq -r '.expires_at')" || "$created_epoch" -gt "$expires_epoch" ]]; then
    sley_report_error_json "TRANSACTION_REVOCATION_TIME_SCOPE_MISMATCH" "revocation record expiry must equal the grant expiry" "change:revocation-record"
    return 2
  fi
  ledger_path="$(printf '%s\n' "$authorization" | jq -r '.internal_scopes[] | select(.kind == "revocation_ledger") | .path')"
  ledger_abs="$repository/$ledger_path"
  if [[ -e "$ledger_abs" || -L "$ledger_abs" ]]; then
    if [[ -L "$ledger_abs" || ! -f "$ledger_abs" ]]; then
      sley_report_error_json "TRANSACTION_REVOCATION_LEDGER_UNSAFE" "revocation ledger target must be a regular non-symlink file" "$ledger_path"
      return 2
    fi
    if ! contract_jsonschema validate --schema-dir "$ROOT_DIR/docs/schemas" --schema "$SCHEMA_CHANGE_REVOCATION_RECORD" "$ledger_abs" >/dev/null 2>&1; then
      sley_report_error_json "TRANSACTION_REVOCATION_RECORD_INVALID" "existing revocation record is invalid" "$ledger_path"
      return 2
    fi
    existing="$(jq -cS . "$ledger_abs")"
    if ! printf '%s\n' "$existing" | jq -e --arg auth "$(printf '%s\n' "$authorization" | jq -r '.authorization_digest')" \
      --arg grant_digest "$(printf '%s\n' "$grant" | jq -r '.grant_digest')" --arg path "$ledger_path" '
        .authorization_ref.authorization_digest == $auth and .grant_ref.grant_digest == $grant_digest and .ledger_path == $path' >/dev/null; then
      sley_report_error_json "TRANSACTION_REVOCATION_RECORD_MISMATCH" "existing revocation record belongs to different authority" "$ledger_path"
      return 2
    fi
    if [[ "$(printf '%s\n' "$existing" | jq -r '.state')" == "revoked" && "$state" != "revoked" ]]; then
      sley_report_error_json "TRANSACTION_REVOCATION_IRREVERSIBLE" "a revoked local grant cannot return to not_revoked" "$ledger_path"
      return 2
    fi
    if [[ "$(printf '%s\n' "$existing" | jq -r '.state')" == "$state" ]]; then
      sley_report_error_json "TRANSACTION_REVOCATION_RECORD_EXISTS" "revocation record already has the requested state" "$ledger_path"
      return 2
    fi
    replace=true
  elif [[ "$state" == "revoked" ]]; then
    sley_report_error_json "TRANSACTION_REVOCATION_RECORD_REQUIRED" "initialize a not_revoked record before recording revocation" "$ledger_path"
    return 2
  fi

  local authorization_ref grant_ref mutations internal_mutations report_core record_digest record_id report status
  authorization_ref="$(printf '%s\n' "$authorization" | jq '{schema,authorization_id,authorization_digest}')"
  grant_ref="$(printf '%s\n' "$authorization" | jq '.grant_ref')"
  mutations="$(printf '%s\n' "$TRANSACTION_MUTATION_BOUNDARIES_JSON" | jq 'reduce .[] as $name ({}; .[$name] = false)')"
  internal_mutations="$(printf '%s\n' "$TRANSACTION_INTERNAL_MUTATION_BOUNDARIES_JSON" | jq 'reduce .[] as $name ({}; .[$name] = true)')"
  [[ "$state" == "revoked" ]] && status="revoked" || status="active"
  report_core="$(jq -n --arg schema "$SCHEMA_CHANGE_REVOCATION_RECORD" --arg status "$status" \
    --argjson authorization_ref "$authorization_ref" --argjson grant_ref "$grant_ref" \
    --arg state "$state" --arg issuer "$issuer" \
    --arg source "$(printf '%s\n' "$AUTHORITY_LOCAL_REVOCATION_RECORD_SOURCES_JSON" | jq -r '.[0]')" \
    --arg reason "$reason" --arg created_at "$created_at" --arg expires_at "$expires_at" --arg ledger_path "$ledger_path" \
    --argjson mutations "$mutations" --argjson internal_mutations "$internal_mutations" '{
      schema:$schema,status:$status,authorization_ref:$authorization_ref,grant_ref:$grant_ref,state:$state,
      issuer:{id:$issuer,source:$source},reason:$reason,created_at:$created_at,expires_at:$expires_at,
      ledger_path:$ledger_path,mutations:$mutations,internal_mutations:$internal_mutations,issues:[]
    }')"
  record_digest="$(printf '%s\n' "$report_core" | json_digest_for)"
  record_id="revocation-record:${record_digest#sha256:}"
  report="$(printf '%s\n' "$report_core" | jq --arg id "$record_id" --arg digest "$record_digest" \
    '{schema,status,record_id:$id,record_digest:$digest} + del(.schema,.status)')"
  if ! transaction_internal_atomic_write "$repository" "$ledger_path" "$replace" "$report"; then
    sley_report_error_json "TRANSACTION_REVOCATION_WRITE_FAILED" "failed to atomically write the authorized revocation record" "$ledger_path"
    return 2
  fi
  change_revocation_record_report_from_json "$report"
}

command_change_apply() {
  local authorization_file="" grant_file="" request_file="" preview_file="" repository="" principal="" audience="" arg
  local confirm_atomic_apply=false
  while [[ "$#" -gt 0 ]]; do
    arg="$1"
    case "$arg" in
      --json) shift ;;
      --confirm-atomic-apply) confirm_atomic_apply=true; shift ;;
      --authorization|--grant|--request|--preview|--repository|--principal|--audience)
        if [[ "$#" -lt 2 ]]; then
          sley_report_error_json "TRANSACTION_ARGUMENT_REQUIRED" "$arg requires a value" "change:apply"
          return 2
        fi
        case "$arg" in
          --authorization) authorization_file="$2" ;;
          --grant) grant_file="$2" ;;
          --request) request_file="$2" ;;
          --preview) preview_file="$2" ;;
          --repository) repository="$2" ;;
          --principal) principal="$2" ;;
          --audience) audience="$2" ;;
        esac
        shift 2
        ;;
      --*)
        sley_report_error_json "TRANSACTION_ARGUMENT_UNKNOWN" "unknown change apply option: $arg" "change:apply"
        return 2
        ;;
      *)
        sley_report_error_json "TRANSACTION_ARGUMENT_UNKNOWN" "change apply does not accept positional arguments" "change:apply"
        return 2
        ;;
    esac
  done
  if [[ "$confirm_atomic_apply" != true ]]; then
    sley_report_error_json "TRANSACTION_ATOMIC_APPLY_CONFIRMATION_REQUIRED" "apply requires --confirm-atomic-apply" "change:apply"
    return 2
  fi

  local chain authorization_chain preview grant authorization binding topology revocation ledger_path
  if ! chain="$(transaction_validate_grant_chain_json "$grant_file" "$request_file" "$preview_file" "$repository" "$principal" "$audience")"; then
    printf '%s\n' "$chain"
    return 2
  fi
  preview="$(printf '%s\n' "$chain" | jq -c '.preview')"
  grant="$(printf '%s\n' "$chain" | jq -c '.grant')"
  if ! authorization_chain="$(transaction_validate_apply_authorization_json "$authorization_file" "$grant_file" "$repository")"; then
    printf '%s\n' "$authorization_chain"
    return 2
  fi
  authorization="$(printf '%s\n' "$authorization_chain" | jq -c '.authorization')"
  binding="$(printf '%s\n' "$authorization_chain" | jq -c '.binding')"
  if [[ "$(printf '%s\n' "$authorization" | jq -r '.principal.id')" != "$principal" \
    || "$(printf '%s\n' "$authorization" | jq -r '.audience.id')" != "$audience" ]]; then
    sley_report_error_json "TRANSACTION_APPLY_PRINCIPAL_MISMATCH" "apply principal and audience must match the exact authorization" "change:apply"
    return 2
  fi
  local replay_path
  replay_path="$(printf '%s\n' "$authorization" | jq -r '.internal_scopes[] | select(.kind == "replay_ledger") | .path')"
  if [[ -e "$repository/$replay_path" || -L "$repository/$replay_path" ]]; then
    sley_report_error_json "TRANSACTION_REPLAY_DENIED" "the exact grant replay key has already been consumed" "$replay_path"
    return 2
  fi
  if ! topology="$(transaction_validate_apply_topology_json "$preview" "$grant" "$repository")"; then
    printf '%s\n' "$topology"
    return 2
  fi
  ledger_path="$(printf '%s\n' "$authorization" | jq -r '.internal_scopes[] | select(.kind == "revocation_ledger") | .path')"
  if ! revocation="$(transaction_validate_revocation_record_json "$repository/$ledger_path" "$authorization" "$grant")"; then
    printf '%s\n' "$revocation"
    return 2
  fi

  local transaction_state lock_path recovery_dir recovery_path transaction_id_hex
  local target_rel common_rel target_abs common_abs candidate_target_rel candidate_target_abs candidate_common_rel candidate_common_abs
  transaction_state="$(printf '%s\n' "$authorization" | jq -r '.internal_scopes[] | select(.kind == "transaction_state") | .path')"
  lock_path="$(printf '%s\n' "$authorization" | jq -r '.internal_scopes[] | select(.kind == "transaction_lock") | .path')"
  recovery_dir="$(printf '%s\n' "$authorization" | jq -r '.internal_scopes[] | select(.kind == "recovery_journal") | .path')"
  recovery_path="$recovery_dir/record.json"
  transaction_id_hex="$(printf '%s\n' "$grant" | jq -r '.transaction.transaction_id | sub("^sha256:";"")')"
  target_rel="$(printf '%s\n' "$topology" | jq -r '.target')"
  common_rel="$(printf '%s\n' "$topology" | jq -r '.common_root')"
  target_abs="$repository/$target_rel"
  common_abs="$repository/$common_rel"
  candidate_target_rel="$transaction_state/candidate/$target_rel"
  candidate_target_abs="$repository/$candidate_target_rel"
  candidate_common_rel="$transaction_state/candidate/$common_rel"
  candidate_common_abs="$repository/$candidate_common_rel"

  if [[ -e "$repository/$replay_path" || -L "$repository/$replay_path" ]]; then
    sley_report_error_json "TRANSACTION_REPLAY_DENIED" "the exact grant replay key has already been consumed" "$replay_path"
    return 2
  fi
  if ! transaction_internal_mkdir "$repository" "$lock_path" true 2>/dev/null; then
    sley_report_error_json "TRANSACTION_APPLY_LOCKED" "another cooperative atomic apply holds the repository lock" "$lock_path"
    return 2
  fi
  local apply_lock_held=true
  local apply_cancel_requested=false apply_commit_entered=false
  trap 'if [[ "${apply_commit_entered:-false}" != true ]]; then apply_cancel_requested=true; fi' INT TERM
  trap 'if [[ "${apply_lock_held:-false}" == true ]]; then transaction_internal_rmdir "$repository" "$lock_path" >/dev/null 2>&1 || true; fi; trap - INT TERM' RETURN

  if ! topology="$(transaction_validate_apply_topology_json "$preview" "$grant" "$repository")"; then
    printf '%s\n' "$topology"
    return 2
  fi
  if ! transaction_internal_mkdir "$repository" "$transaction_state" true 2>/dev/null; then
    sley_report_error_json "TRANSACTION_RECOVERY_STATE_EXISTS" "transaction state already exists; inspect its recovery record instead of replaying apply" "$transaction_state"
    return 2
  fi
  if ! transaction_internal_mkdir "$repository" "$(dirname -- "$candidate_target_rel")" false 2>/dev/null; then
    sley_report_error_json "TRANSACTION_CANDIDATE_PREPARE_FAILED" "could not prepare the authorized candidate parent" "$candidate_target_rel"
    return 2
  fi
  if ! cp -a -- "$target_abs" "$candidate_target_abs" 2>/dev/null; then
    sley_report_error_json "TRANSACTION_CANDIDATE_COPY_FAILED" "could not create the durable same-filesystem candidate" "$candidate_target_rel"
    return 2
  fi

  local projection_path projection_digest encoded destination actual_projection_digest
  while IFS=$'\t' read -r projection_path projection_digest encoded; do
    [[ -n "$projection_path" ]] || continue
    destination="$repository/$transaction_state/candidate/$projection_path"
    if [[ ! -f "$destination" || -L "$destination" ]]; then
      sley_report_error_json "TRANSACTION_CANDIDATE_PATH_UNSAFE" "candidate projection destination is not a regular copied source file" "$projection_path"
      return 2
    fi
    if ! printf '%s' "$encoded" | base64 -d >"$destination"; then
      sley_report_error_json "TRANSACTION_CANDIDATE_MATERIALIZATION_FAILED" "could not materialize exact candidate source" "$projection_path"
      return 2
    fi
    actual_projection_digest="$(sha256sum "$destination" | awk '{print "sha256:" $1}')"
    if [[ "$actual_projection_digest" != "$projection_digest" ]]; then
      sley_report_error_json "TRANSACTION_CANDIDATE_PROJECTION_DIGEST_MISMATCH" "materialized candidate file does not match its preview digest" "$projection_path"
      return 2
    fi
  done < <(printf '%s\n' "$preview" | jq -r '.candidate.source_projection[] | [.path,.digest,(.content | @base64)] | @tsv')
  if [[ "$(structural_source_digest_for "$candidate_target_abs")" != "$(printf '%s\n' "$grant" | jq -r '.candidate.candidate_source_digest')" \
    || "$(graph_json_for "$candidate_target_abs" | json_digest_for)" != "$(printf '%s\n' "$grant" | jq -r '.candidate.candidate_graph_digest')" ]]; then
    sley_report_error_json "TRANSACTION_CANDIDATE_DIGEST_MISMATCH" "durable candidate does not match the exact authorized source and graph digests" "$candidate_target_rel"
    return 2
  fi
  if [[ ! -e "$candidate_common_abs" || -L "$candidate_common_abs" \
    || "$(stat -c '%d' "$common_abs")" != "$(stat -c '%d' "$candidate_common_abs")" ]]; then
    sley_report_error_json "TRANSACTION_ATOMIC_FILESYSTEM_MISMATCH" "live and candidate common roots must exist on the same filesystem" "$common_rel"
    return 2
  fi
  transaction_fsync_tree "$repository/$transaction_state"

  local authorization_ref grant_ref revocation_ref paths atomicity metadata_policy transaction recovery replay marker now
  local pending_verification rollback mutations internal_mutations record_id
  authorization_ref="$(printf '%s\n' "$authorization" | jq '{schema,authorization_id,authorization_digest}')"
  grant_ref="$(printf '%s\n' "$authorization" | jq '.grant_ref')"
  revocation_ref="$(printf '%s\n' "$revocation" | jq '{record_id,record_digest,state,ledger_path}')"
  paths="$(jq -n --arg target "$target_rel" --arg common_root "$common_rel" --arg candidate_root "$candidate_target_rel" \
    --arg transaction_state "$transaction_state" --arg recovery_record "$recovery_path" --arg lock "$lock_path" \
    --arg replay "$replay_path" --arg revocation "$ledger_path" \
    --argjson allowed_paths "$(printf '%s\n' "$grant" | jq '.allowed.paths')" '{
      target:$target,common_root:$common_root,candidate_root:$candidate_root,transaction_state:$transaction_state,
      recovery_record:$recovery_record,lock:$lock,replay:$replay,revocation:$revocation,allowed_paths:$allowed_paths
    }')"
  atomicity="$(printf '%s\n' "$authorization" | jq '.atomicity')"
  metadata_policy="$(printf '%s\n' "$authorization" | jq '.metadata_policy')"
  transaction="$(printf '%s\n' "$grant" | jq '{transaction_id:.transaction.transaction_id,state:"applying",history:(.transaction.history + ["applying"])}')"
  replay="$(printf '%s\n' "$grant" | jq --arg enforcement "$AUTHORITY_APPLY_REPLAY_ENFORCEMENT" '.replay | .enforcement=$enforcement | .state="available"')"
  pending_verification="$(printf '%s\n' "$TRANSACTION_APPLY_VERIFICATION_REQUIRED_CHECKS_JSON" | jq '[.[] | {kind:.,status:"pending",expected:null,actual:null}] | {profile:"compiler_apply_v0",status:"pending",checks:.}')"
  rollback="$(jq -n --arg source "$(printf '%s\n' "$grant" | jq -r '.base.source_digest')" \
    --arg graph "$(printf '%s\n' "$grant" | jq -r '.base.graph_digest')" '{
      strategy:"verified_exchange",status:"not_required",expected_source_digest:$source,actual_source_digest:null,
      expected_graph_digest:$graph,actual_graph_digest:null
    }')"
  mutations="$(printf '%s\n' "$TRANSACTION_MUTATION_BOUNDARIES_JSON" | jq 'reduce .[] as $name ({}; .[$name] = false)')"
  internal_mutations="$(printf '%s\n' "$TRANSACTION_INTERNAL_MUTATION_BOUNDARIES_JSON" | jq 'reduce .[] as $name ({}; .[$name] = true)')"
  now="$(date -u +%Y-%m-%dT%H:%M:%SZ)"
  record_id="recovery:$transaction_id_hex"
  recovery="$(jq -n --arg schema "$SCHEMA_CHANGE_RECOVERY" --arg record_id "$record_id" \
    --argjson transaction "$transaction" --argjson authorization_ref "$authorization_ref" --argjson grant_ref "$grant_ref" \
    --argjson repository_json "$(printf '%s\n' "$grant" | jq '.repository')" --argjson base "$(printf '%s\n' "$grant" | jq '.base')" \
    --argjson candidate "$(printf '%s\n' "$grant" | jq '.candidate')" --argjson paths "$paths" --argjson atomicity "$atomicity" \
    --argjson replay "$replay" --argjson revocation_ref "$revocation_ref" --argjson metadata_policy "$metadata_policy" \
    --argjson verification "$pending_verification" --argjson rollback "$rollback" --argjson mutations "$mutations" \
    --argjson internal_mutations "$internal_mutations" --arg now "$now" '{
      schema:$schema,status:"active",record_id:$record_id,phase:"preflight_checked",transaction:$transaction,
      authorization_ref:$authorization_ref,grant_ref:$grant_ref,repository:$repository_json,base:$base,candidate:$candidate,
      paths:$paths,atomicity:$atomicity,replay:$replay,revocation_ref:$revocation_ref,metadata_policy:$metadata_policy,
      verification:$verification,rollback:$rollback,mutations:$mutations,internal_mutations:$internal_mutations,
      events:[{sequence:1,phase:"preflight_checked",at:$now,detail:"authority, revocation, base, topology, metadata, and candidate digests passed"}],issues:[]
    }')"
  recovery="$(transaction_recovery_reseal_json "$recovery")"
  transaction_internal_atomic_write "$repository" "$recovery_path" false "$recovery"
  recovery="$(transaction_recovery_advance_json "$recovery" "prepared" "applying" "active" "$now" "durable candidate and preimage topology prepared and fsynced")"
  transaction_internal_atomic_write "$repository" "$recovery_path" true "$recovery"

  if [[ "${SLEY_ALLOW_TEST_HOOKS:-0}" == "1" && "${SLEY_TEST_CANCEL_BEFORE_COMMIT:-0}" == "1" ]]; then
    apply_cancel_requested=true
  fi
  if [[ "$apply_cancel_requested" == true ]]; then
    recovery="$(transaction_recovery_advance_json "$recovery" "cancelled" "cancelled" "terminal" "$(date -u +%Y-%m-%dT%H:%M:%SZ)" "apply was cancelled before replay consumption and before the atomic source commit point")"
    recovery="$(transaction_recovery_reseal_json "$recovery")"
    transaction_internal_atomic_write "$repository" "$recovery_path" true "$recovery"
    sley_report_error_json "TRANSACTION_CANCELLED" "atomic apply was cancelled before its commit point; repository source was not changed" "$recovery_path"
    return 2
  fi

  marker="$(jq -n --arg replay_key "$(printf '%s\n' "$grant" | jq -r '.replay.replay_key')" \
    --arg transaction_id "$(printf '%s\n' "$grant" | jq -r '.transaction.transaction_id')" \
    --arg authorization_digest "$(printf '%s\n' "$authorization" | jq -r '.authorization_digest')" \
    --arg grant_digest "$(printf '%s\n' "$grant" | jq -r '.grant_digest')" --arg consumed_at "$now" '{
      replay_key:$replay_key,transaction_id:$transaction_id,authorization_digest:$authorization_digest,
      grant_digest:$grant_digest,state:"consumed",consumed_at:$consumed_at
    }')"
  if ! transaction_internal_atomic_write "$repository" "$replay_path" false "$marker"; then
    sley_report_error_json "TRANSACTION_REPLAY_DENIED" "the exact grant replay key could not be consumed exactly once" "$replay_path"
    return 2
  fi
  recovery="$(printf '%s\n' "$recovery" | jq '.replay.state="consumed"')"
  recovery="$(transaction_recovery_reseal_json "$recovery")"
  transaction_internal_atomic_write "$repository" "$recovery_path" true "$recovery"

  if ! topology="$(transaction_validate_apply_topology_json "$preview" "$grant" "$repository")"; then
    recovery="$(transaction_recovery_advance_json "$recovery" "apply_failed" "apply_failed" "terminal" "$(date -u +%Y-%m-%dT%H:%M:%SZ)" "base or topology changed immediately before the commit point")"
    recovery="$(printf '%s\n' "$recovery" | jq '.issues += [{id:"TRANSACTION_STALE",message:"base or topology changed before commit"}]')"
    recovery="$(transaction_recovery_reseal_json "$recovery")"
    transaction_internal_atomic_write "$repository" "$recovery_path" true "$recovery"
    printf '%s\n' "$topology"
    return 2
  fi
  now="$(date -u +%Y-%m-%dT%H:%M:%SZ)"
  recovery="$(transaction_recovery_advance_json "$recovery" "commit_point_entered" "applying" "active" "$now" "durable intent recorded immediately before the sole renameat2 exchange commit point")"
  transaction_internal_atomic_write "$repository" "$recovery_path" true "$recovery"
  apply_commit_entered=true
  if [[ "${SLEY_ALLOW_TEST_HOOKS:-0}" == "1" && "${SLEY_TEST_CRASH_AT:-}" == "before_exchange" ]]; then
    kill -KILL "$BASHPID"
  fi
  if ! transaction_rename_exchange "$repository" "$common_rel" "$candidate_common_rel"; then
    recovery="$(transaction_recovery_advance_json "$recovery" "apply_failed" "apply_failed" "terminal" "$(date -u +%Y-%m-%dT%H:%M:%SZ)" "renameat2 exchange failed before any accepted source write")"
    recovery="$(printf '%s\n' "$recovery" | jq '.issues += [{id:"TRANSACTION_ATOMIC_EXCHANGE_FAILED",message:"renameat2 exchange failed"}]')"
    recovery="$(transaction_recovery_reseal_json "$recovery")"
    transaction_internal_atomic_write "$repository" "$recovery_path" true "$recovery"
    sley_report_error_json "TRANSACTION_ATOMIC_EXCHANGE_FAILED" "the sole atomic source commit point failed" "$common_rel"
    return 2
  fi
  if [[ "${SLEY_ALLOW_TEST_HOOKS:-0}" == "1" && "${SLEY_TEST_CRASH_AT:-}" == "after_exchange" ]]; then
    kill -KILL "$BASHPID"
  fi
  now="$(date -u +%Y-%m-%dT%H:%M:%SZ)"
  recovery="$(transaction_recovery_advance_json "$recovery" "applied_unverified" "applied" "active" "$now" "atomic common-root exchange committed candidate source")"
  recovery="$(printf '%s\n' "$recovery" | jq '.mutations.repository_source=true')"
  recovery="$(transaction_recovery_reseal_json "$recovery")"
  transaction_internal_atomic_write "$repository" "$recovery_path" true "$recovery"
  recovery="$(transaction_recovery_advance_json "$recovery" "verifying" "verifying" "active" "$now" "required compiler verification started after accepted source write")"
  transaction_internal_atomic_write "$repository" "$recovery_path" true "$recovery"

  local verification rollback_status actual_base_source actual_base_graph report_status next_actions issues recovery_ref report
  verification="$(transaction_verification_json "$target_abs" "$grant")"
  recovery="$(printf '%s\n' "$recovery" | jq --argjson verification "$verification" '.verification=$verification')"
  if [[ "$(printf '%s\n' "$verification" | jq -r '.status')" == passed ]]; then
    recovery="$(transaction_recovery_advance_json "$recovery" "verified" "verifying" "terminal" "$(date -u +%Y-%m-%dT%H:%M:%SZ)" "all required source, graph, check, and lint verification passed")"
    report_status=verified
    next_actions='[{"kind":"seal_verified_transaction","reason":"verification passed; S12-405 review and seal may inspect the durable evidence"}]'
    issues='[]'
  else
    recovery="$(transaction_recovery_advance_json "$recovery" "verification_failed" "verification_failed" "active" "$(date -u +%Y-%m-%dT%H:%M:%SZ)" "required verification failed; automatic exchange rollback required")"
    recovery="$(printf '%s\n' "$recovery" | jq '.rollback.status="pending" | .issues += [{id:"TRANSACTION_VERIFICATION_FAILED",message:"required post-apply verification failed"}]')"
    recovery="$(transaction_recovery_reseal_json "$recovery")"
    transaction_internal_atomic_write "$repository" "$recovery_path" true "$recovery"
    recovery="$(transaction_recovery_advance_json "$recovery" "rollback_required" "verification_failed" "active" "$(date -u +%Y-%m-%dT%H:%M:%SZ)" "preimage remains at the exchanged candidate common root")"
    transaction_internal_atomic_write "$repository" "$recovery_path" true "$recovery"
    if [[ "${SLEY_ALLOW_TEST_HOOKS:-0}" == "1" && "${SLEY_TEST_CRASH_AT:-}" == "before_rollback" ]]; then
      kill -KILL "$BASHPID"
    fi
    if ! transaction_rename_exchange "$repository" "$common_rel" "$candidate_common_rel"; then
      recovery="$(printf '%s\n' "$recovery" | jq '.rollback.status="failed" | .issues += [{id:"TRANSACTION_ROLLBACK_EXCHANGE_FAILED",message:"automatic rollback exchange failed"}]')"
      recovery="$(transaction_recovery_reseal_json "$recovery")"
      transaction_internal_atomic_write "$repository" "$recovery_path" true "$recovery"
      sley_report_error_json "TRANSACTION_ROLLBACK_EXCHANGE_FAILED" "automatic rollback exchange failed; inspect the durable recovery record" "$recovery_path"
      return 2
    fi
    actual_base_source="$(structural_source_digest_for "$target_abs")"
    actual_base_graph="$(graph_json_for "$target_abs" | json_digest_for)"
    if [[ "$actual_base_source" != "$(printf '%s\n' "$grant" | jq -r '.base.source_digest')" \
      || "$actual_base_graph" != "$(printf '%s\n' "$grant" | jq -r '.base.graph_digest')" ]]; then
      recovery="$(printf '%s\n' "$recovery" | jq --arg source "$actual_base_source" --arg graph "$actual_base_graph" '
        .rollback.status="failed" | .rollback.actual_source_digest=$source | .rollback.actual_graph_digest=$graph
        | .issues += [{id:"TRANSACTION_ROLLBACK_VERIFICATION_FAILED",message:"automatic rollback did not restore exact base digests"}]')"
      recovery="$(transaction_recovery_reseal_json "$recovery")"
      transaction_internal_atomic_write "$repository" "$recovery_path" true "$recovery"
      sley_report_error_json "TRANSACTION_ROLLBACK_VERIFICATION_FAILED" "automatic rollback did not restore the exact approved base" "$recovery_path"
      return 2
    fi
    recovery="$(printf '%s\n' "$recovery" | jq --arg source "$actual_base_source" --arg graph "$actual_base_graph" '
      .rollback.status="passed" | .rollback.actual_source_digest=$source | .rollback.actual_graph_digest=$graph')"
    recovery="$(transaction_recovery_advance_json "$recovery" "rolled_back" "rolled_back" "terminal" "$(date -u +%Y-%m-%dT%H:%M:%SZ)" "verified exchange rollback restored the exact base source and graph digests")"
    report_status=rolled_back
    next_actions='[{"kind":"review_failed_transaction","reason":"verification failed and automatic rollback restored the exact base"}]'
    issues='[{"id":"TRANSACTION_VERIFICATION_FAILED","message":"required verification failed; automatic rollback passed"}]'
  fi
  transaction_internal_atomic_write "$repository" "$recovery_path" true "$recovery"
  recovery_ref="$(printf '%s\n' "$recovery" | jq --arg path "$recovery_path" '{schema,record_id,record_digest,phase,path:$path}')"
  report="$(jq -n --arg schema "$SCHEMA_CHANGE_APPLY" --arg status "$report_status" \
    --argjson transaction "$(printf '%s\n' "$recovery" | jq '.transaction')" --argjson authorization_ref "$authorization_ref" \
    --argjson grant_ref "$grant_ref" --argjson candidate "$(printf '%s\n' "$grant" | jq '.candidate')" \
    --argjson atomicity "$atomicity" --argjson recovery_ref "$recovery_ref" \
    --argjson replay "$(printf '%s\n' "$recovery" | jq '.replay')" --argjson revocation "$revocation_ref" \
    --argjson verification "$(printf '%s\n' "$recovery" | jq '.verification')" \
    --argjson rollback "$(printf '%s\n' "$recovery" | jq '.rollback')" \
    --argjson mutations "$(printf '%s\n' "$recovery" | jq '.mutations')" \
    --argjson internal_mutations "$internal_mutations" --argjson next_actions "$next_actions" --argjson issues "$issues" '{
      schema:$schema,status:$status,transaction:$transaction,authorization_ref:$authorization_ref,grant_ref:$grant_ref,
      candidate:$candidate,atomicity:$atomicity,recovery_ref:$recovery_ref,replay:$replay,revocation:$revocation,
      verification:$verification,rollback:$rollback,mutations:$mutations,internal_mutations:$internal_mutations,
      next_actions:$next_actions,issues:$issues
    }')"
  change_apply_report_from_json "$report"
  apply_lock_held=false
  transaction_internal_rmdir "$repository" "$lock_path" >/dev/null 2>&1 || true
  trap - INT TERM RETURN
  [[ "$report_status" == verified ]]
}

transaction_validate_recovery_record_json() {
  local recovery_file="$1" authorization="$2" grant="$3" binding="$4"
  local recovery expected_digest transaction_id_hex expected_state expected_recovery expected_lock expected_replay expected_revocation
  if [[ -L "$recovery_file" || ! -f "$recovery_file" ]] \
    || ! contract_jsonschema validate --schema-dir "$ROOT_DIR/docs/schemas" --schema "$SCHEMA_CHANGE_RECOVERY" "$recovery_file" >/dev/null 2>&1; then
    sley_report_error_json "TRANSACTION_RECOVERY_RECORD_INVALID" "recovery requires a valid durable recovery record" "$recovery_file"
    return 2
  fi
  recovery="$(jq -cS . "$recovery_file")"
  expected_digest="$(printf '%s\n' "$recovery" | jq 'del(.record_digest)' | json_digest_for)"
  transaction_id_hex="$(printf '%s\n' "$grant" | jq -r '.transaction.transaction_id | sub("^sha256:";"")')"
  expected_state="$(printf '%s\n' "$authorization" | jq -r '.internal_scopes[] | select(.kind == "transaction_state") | .path')"
  expected_recovery="$(printf '%s\n' "$authorization" | jq -r '.internal_scopes[] | select(.kind == "recovery_journal") | .path')/record.json"
  expected_lock="$(printf '%s\n' "$authorization" | jq -r '.internal_scopes[] | select(.kind == "transaction_lock") | .path')"
  expected_replay="$(printf '%s\n' "$authorization" | jq -r '.internal_scopes[] | select(.kind == "replay_ledger") | .path')"
  expected_revocation="$(printf '%s\n' "$authorization" | jq -r '.internal_scopes[] | select(.kind == "revocation_ledger") | .path')"
  if ! printf '%s\n' "$recovery" | jq -e --arg digest "$expected_digest" --arg id "recovery:$transaction_id_hex" \
    --argjson authorization "$authorization" --argjson grant "$grant" --argjson binding "$binding" \
    --arg state "$expected_state" --arg recovery "$expected_recovery" --arg lock "$expected_lock" \
    --arg replay "$expected_replay" --arg revocation "$expected_revocation" '
      .record_digest == $digest and .record_id == $id
      and .authorization_ref == ($authorization | {schema,authorization_id,authorization_digest})
      and .grant_ref == $authorization.grant_ref
      and .repository == $grant.repository and .repository.identity_digest == $binding.identity_digest
      and .base == $grant.base and .candidate == $grant.candidate and .atomicity == $authorization.atomicity
      and .metadata_policy == $authorization.metadata_policy
      and .paths.transaction_state == $state and .paths.recovery_record == $recovery
      and .paths.lock == $lock and .paths.replay == $replay and .paths.revocation == $revocation
      and (.paths.candidate_root | startswith($state + "/candidate/"))' >/dev/null; then
    sley_report_error_json "TRANSACTION_RECOVERY_RECORD_MISMATCH" "durable recovery record is not the exact state authorized by this chain" "$recovery_file"
    return 2
  fi
  printf '%s\n' "$recovery"
}

transaction_apply_report_from_recovery_json() {
  local recovery="$1" status="$2" next_actions issues
  if [[ "$status" == verified ]]; then
    next_actions='[{"kind":"seal_verified_transaction","reason":"recovered verification passed; S12-405 review and seal may inspect the durable evidence"}]'
    issues='[]'
  else
    next_actions='[{"kind":"review_failed_transaction","reason":"recovery completed a verified rollback to the exact base"}]'
    issues='[{"id":"TRANSACTION_RECOVERED_ROLLBACK","message":"crash recovery restored and verified the exact base"}]'
  fi
  jq -n --arg schema "$SCHEMA_CHANGE_APPLY" --arg status "$status" \
    --argjson transaction "$(printf '%s\n' "$recovery" | jq '.transaction')" \
    --argjson authorization_ref "$(printf '%s\n' "$recovery" | jq '.authorization_ref')" \
    --argjson grant_ref "$(printf '%s\n' "$recovery" | jq '.grant_ref')" \
    --argjson candidate "$(printf '%s\n' "$recovery" | jq '.candidate')" \
    --argjson atomicity "$(printf '%s\n' "$recovery" | jq '.atomicity')" \
    --argjson recovery_ref "$(printf '%s\n' "$recovery" | jq --arg path "$(printf '%s\n' "$recovery" | jq -r '.paths.recovery_record')" '{schema,record_id,record_digest,phase,path:$path}')" \
    --argjson replay "$(printf '%s\n' "$recovery" | jq '.replay')" \
    --argjson revocation "$(printf '%s\n' "$recovery" | jq '.revocation_ref')" \
    --argjson verification "$(printf '%s\n' "$recovery" | jq '.verification')" \
    --argjson rollback "$(printf '%s\n' "$recovery" | jq '.rollback')" \
    --argjson mutations "$(printf '%s\n' "$recovery" | jq '.mutations')" \
    --argjson internal_mutations "$(printf '%s\n' "$recovery" | jq '.internal_mutations')" \
    --argjson next_actions "$next_actions" --argjson issues "$issues" '{
      schema:$schema,status:$status,transaction:$transaction,authorization_ref:$authorization_ref,grant_ref:$grant_ref,
      candidate:$candidate,atomicity:$atomicity,recovery_ref:$recovery_ref,replay:$replay,revocation:$revocation,
      verification:$verification,rollback:$rollback,mutations:$mutations,internal_mutations:$internal_mutations,
      next_actions:$next_actions,issues:$issues
    }'
}

command_change_recover() {
  local authorization_file="" grant_file="" request_file="" preview_file="" repository="" principal="" audience="" arg
  local confirm_recovery=false
  while [[ "$#" -gt 0 ]]; do
    arg="$1"
    case "$arg" in
      --json) shift ;;
      --confirm-recovery) confirm_recovery=true; shift ;;
      --authorization|--grant|--request|--preview|--repository|--principal|--audience)
        if [[ "$#" -lt 2 ]]; then
          sley_report_error_json "TRANSACTION_ARGUMENT_REQUIRED" "$arg requires a value" "change:recover"
          return 2
        fi
        case "$arg" in
          --authorization) authorization_file="$2" ;;
          --grant) grant_file="$2" ;;
          --request) request_file="$2" ;;
          --preview) preview_file="$2" ;;
          --repository) repository="$2" ;;
          --principal) principal="$2" ;;
          --audience) audience="$2" ;;
        esac
        shift 2
        ;;
      --*)
        sley_report_error_json "TRANSACTION_ARGUMENT_UNKNOWN" "unknown change recover option: $arg" "change:recover"
        return 2
        ;;
      *)
        sley_report_error_json "TRANSACTION_ARGUMENT_UNKNOWN" "change recover does not accept positional arguments" "change:recover"
        return 2
        ;;
    esac
  done
  if [[ "$confirm_recovery" != true ]]; then
    sley_report_error_json "TRANSACTION_RECOVERY_CONFIRMATION_REQUIRED" "recover requires --confirm-recovery" "change:recover"
    return 2
  fi

  local chain authorization_chain preview grant authorization binding recovery_path recovery phase lock_path
  if ! chain="$(transaction_validate_grant_chain_json "$grant_file" "$request_file" "$preview_file" "$repository" "$principal" "$audience")"; then
    printf '%s\n' "$chain"
    return 2
  fi
  preview="$(printf '%s\n' "$chain" | jq -c '.preview')"
  grant="$(printf '%s\n' "$chain" | jq -c '.grant')"
  if ! authorization_chain="$(transaction_validate_apply_authorization_json "$authorization_file" "$grant_file" "$repository")"; then
    printf '%s\n' "$authorization_chain"
    return 2
  fi
  authorization="$(printf '%s\n' "$authorization_chain" | jq -c '.authorization')"
  binding="$(printf '%s\n' "$authorization_chain" | jq -c '.binding')"
  if [[ "$(printf '%s\n' "$authorization" | jq -r '.principal.id')" != "$principal" \
    || "$(printf '%s\n' "$authorization" | jq -r '.audience.id')" != "$audience" ]]; then
    sley_report_error_json "TRANSACTION_APPLY_PRINCIPAL_MISMATCH" "recovery principal and audience must match the exact authorization" "change:recover"
    return 2
  fi
  recovery_path="$(printf '%s\n' "$authorization" | jq -r '.internal_scopes[] | select(.kind == "recovery_journal") | .path')/record.json"
  if ! recovery="$(transaction_validate_recovery_record_json "$repository/$recovery_path" "$authorization" "$grant" "$binding")"; then
    printf '%s\n' "$recovery"
    return 2
  fi
  phase="$(printf '%s\n' "$recovery" | jq -r '.phase')"
  if [[ "$(printf '%s\n' "$recovery" | jq -r '.status')" == terminal ]]; then
    sley_report_error_json "TRANSACTION_RECOVERY_TERMINAL" "recovery record is already terminal and cannot be resumed" "$recovery_path"
    return 2
  fi

  lock_path="$(printf '%s\n' "$recovery" | jq -r '.paths.lock')"
  if [[ -d "$repository/$lock_path" ]]; then
    transaction_internal_rmdir "$repository" "$lock_path" >/dev/null 2>&1 || {
      sley_report_error_json "TRANSACTION_APPLY_LOCKED" "recovery found a non-empty cooperative apply lock" "$lock_path"
      return 2
    }
  fi
  if ! transaction_internal_mkdir "$repository" "$lock_path" true 2>/dev/null; then
    sley_report_error_json "TRANSACTION_APPLY_LOCKED" "another cooperative transaction holds the repository lock" "$lock_path"
    return 2
  fi
  local recovery_lock_held=true
  trap 'if [[ "${recovery_lock_held:-false}" == true ]]; then transaction_internal_rmdir "$repository" "$lock_path" >/dev/null 2>&1 || true; fi' RETURN

  local target_rel common_rel candidate_target_rel candidate_common_rel target_abs candidate_target_abs
  local live_source live_graph stored_source stored_graph base_source base_graph candidate_source candidate_graph
  target_rel="$(printf '%s\n' "$recovery" | jq -r '.paths.target')"
  common_rel="$(printf '%s\n' "$recovery" | jq -r '.paths.common_root')"
  candidate_target_rel="$(printf '%s\n' "$recovery" | jq -r '.paths.candidate_root')"
  candidate_common_rel="${candidate_target_rel%"$target_rel"}$common_rel"
  target_abs="$repository/$target_rel"
  candidate_target_abs="$repository/$candidate_target_rel"
  if [[ ! -e "$repository/$common_rel" || ! -e "$repository/$candidate_common_rel" \
    || -L "$repository/$common_rel" || -L "$repository/$candidate_common_rel" ]]; then
    sley_report_error_json "TRANSACTION_RECOVERY_TOPOLOGY_INVALID" "recovery live or preserved common root is missing or unsafe" "$recovery_path"
    return 2
  fi
  live_source="$(structural_source_digest_for "$target_abs")"
  live_graph="$(graph_json_for "$target_abs" | json_digest_for)"
  stored_source="$(structural_source_digest_for "$candidate_target_abs")"
  stored_graph="$(graph_json_for "$candidate_target_abs" | json_digest_for)"
  base_source="$(printf '%s\n' "$recovery" | jq -r '.base.source_digest')"
  base_graph="$(printf '%s\n' "$recovery" | jq -r '.base.graph_digest')"
  candidate_source="$(printf '%s\n' "$recovery" | jq -r '.candidate.candidate_source_digest')"
  candidate_graph="$(printf '%s\n' "$recovery" | jq -r '.candidate.candidate_graph_digest')"

  local now verification actual_source actual_graph report status revocation_path revocation topology
  local replay_marker_path
  now="$(date -u +%Y-%m-%dT%H:%M:%SZ)"
  if [[ "$phase" == prepared && "$(printf '%s\n' "$recovery" | jq -r '.replay.state')" == available \
    && ! -e "$repository/$(printf '%s\n' "$recovery" | jq -r '.paths.replay')" ]]; then
    if [[ "$live_source" != "$base_source" || "$live_graph" != "$base_graph" ]]; then
      sley_report_error_json "TRANSACTION_RECOVERY_AMBIGUOUS" "uncommitted prepared recovery does not observe the exact base" "$recovery_path"
      return 2
    fi
    recovery="$(transaction_recovery_advance_json "$recovery" "cancelled" "cancelled" "terminal" "$now" "recovery closed an interrupted pre-replay apply with no source mutation")"
    transaction_internal_atomic_write "$repository" "$recovery_path" true "$recovery"
    change_recovery_report_from_json "$recovery"
    recovery_lock_held=false
    transaction_internal_rmdir "$repository" "$lock_path" >/dev/null 2>&1 || true
    trap - RETURN
    return 0
  fi

  replay_marker_path="$repository/$(printf '%s\n' "$recovery" | jq -r '.paths.replay')"
  if [[ -L "$replay_marker_path" || ! -f "$replay_marker_path" ]] \
    || ! jq -e --arg replay_key "$(printf '%s\n' "$grant" | jq -r '.replay.replay_key')" \
      --arg transaction_id "$(printf '%s\n' "$grant" | jq -r '.transaction.transaction_id')" \
      --arg authorization_digest "$(printf '%s\n' "$authorization" | jq -r '.authorization_digest')" \
      --arg grant_digest "$(printf '%s\n' "$grant" | jq -r '.grant_digest')" '
        (keys | sort) == (["authorization_digest","consumed_at","grant_digest","replay_key","state","transaction_id"] | sort)
        and .replay_key == $replay_key and .transaction_id == $transaction_id
        and .authorization_digest == $authorization_digest and .grant_digest == $grant_digest
        and .state == "consumed" and (.consumed_at | fromdateiso8601 | type) == "number"
      ' "$replay_marker_path" >/dev/null 2>&1; then
    sley_report_error_json "TRANSACTION_REPLAY_MARKER_MISMATCH" "recovery requires the exact durable replay-consumption marker" "$(printf '%s\n' "$recovery" | jq -r '.paths.replay')"
    return 2
  fi

  if [[ "$phase" == prepared || "$phase" == commit_point_entered ]]; then
    if [[ "$live_source" == "$base_source" && "$live_graph" == "$base_graph" \
      && "$stored_source" == "$candidate_source" && "$stored_graph" == "$candidate_graph" ]]; then
      if ! topology="$(transaction_validate_apply_topology_json "$preview" "$grant" "$repository")"; then
        printf '%s\n' "$topology"
        return 2
      fi
      if [[ "$(printf '%s\n' "$topology" | jq -r '.target')" != "$target_rel" \
        || "$(printf '%s\n' "$topology" | jq -r '.common_root')" != "$common_rel" ]]; then
        sley_report_error_json "TRANSACTION_RECOVERY_TOPOLOGY_MISMATCH" "recovery topology no longer matches the exact approved target and common root" "$recovery_path"
        return 2
      fi
      revocation_path="$repository/$(printf '%s\n' "$recovery" | jq -r '.paths.revocation')"
      if ! revocation="$(transaction_validate_revocation_record_json "$revocation_path" "$authorization" "$grant")"; then
        recovery="$(transaction_recovery_advance_json "$recovery" "cancelled" "cancelled" "terminal" "$now" "recovery refused a forward commit because current revocation authority was not active")"
        recovery="$(printf '%s\n' "$recovery" | jq '.issues += [{id:"TRANSACTION_RECOVERY_AUTHORITY_DENIED",message:"forward recovery authority is no longer active"}]')"
        recovery="$(transaction_recovery_reseal_json "$recovery")"
        transaction_internal_atomic_write "$repository" "$recovery_path" true "$recovery"
        printf '%s\n' "$revocation"
        return 2
      fi
      recovery="$(printf '%s\n' "$recovery" | jq '.replay.state="consumed"')"
      recovery="$(transaction_recovery_advance_json "$recovery" "commit_point_entered" "applying" "active" "$now" "crash recovery resumed the durable forward commit intent")"
      transaction_internal_atomic_write "$repository" "$recovery_path" true "$recovery"
      if ! transaction_rename_exchange "$repository" "$common_rel" "$candidate_common_rel"; then
        sley_report_error_json "TRANSACTION_RECOVERY_EXCHANGE_FAILED" "recovery could not resume the atomic exchange" "$recovery_path"
        return 2
      fi
      live_source="$candidate_source"
      live_graph="$candidate_graph"
      stored_source="$base_source"
      stored_graph="$base_graph"
    elif [[ "$live_source" != "$candidate_source" || "$live_graph" != "$candidate_graph" \
      || "$stored_source" != "$base_source" || "$stored_graph" != "$base_graph" ]]; then
      sley_report_error_json "TRANSACTION_RECOVERY_AMBIGUOUS" "commit-point recovery cannot classify live and preserved trees as exact base/candidate counterparts" "$recovery_path"
      return 2
    fi
    recovery="$(transaction_recovery_advance_json "$recovery" "applied_unverified" "applied" "active" "$now" "crash recovery confirmed the candidate is live and the exact preimage is preserved")"
    recovery="$(printf '%s\n' "$recovery" | jq '.mutations.repository_source=true | .replay.state="consumed"')"
    recovery="$(transaction_recovery_reseal_json "$recovery")"
    transaction_internal_atomic_write "$repository" "$recovery_path" true "$recovery"
    phase=applied_unverified
  fi

  if [[ "$phase" == applied_unverified || "$phase" == verifying ]]; then
    if [[ "$live_source" != "$candidate_source" || "$live_graph" != "$candidate_graph" \
      || "$stored_source" != "$base_source" || "$stored_graph" != "$base_graph" ]]; then
      sley_report_error_json "TRANSACTION_RECOVERY_AMBIGUOUS" "verification recovery requires exact live candidate and preserved base trees" "$recovery_path"
      return 2
    fi
    recovery="$(transaction_recovery_advance_json "$recovery" "verifying" "verifying" "active" "$now" "crash recovery resumed required compiler verification")"
    transaction_internal_atomic_write "$repository" "$recovery_path" true "$recovery"
    verification="$(transaction_verification_json "$target_abs" "$grant")"
    recovery="$(printf '%s\n' "$recovery" | jq --argjson verification "$verification" '.verification=$verification')"
    if [[ "$(printf '%s\n' "$verification" | jq -r '.status')" == passed ]]; then
      recovery="$(transaction_recovery_advance_json "$recovery" "verified" "verifying" "terminal" "$now" "crash recovery completed all required verification")"
      status=verified
    else
      recovery="$(transaction_recovery_advance_json "$recovery" "verification_failed" "verification_failed" "active" "$now" "recovered verification failed and requires rollback")"
      recovery="$(printf '%s\n' "$recovery" | jq '.rollback.status="pending" | .issues += [{id:"TRANSACTION_VERIFICATION_FAILED",message:"required recovered verification failed"}]')"
      recovery="$(transaction_recovery_advance_json "$recovery" "rollback_required" "verification_failed" "active" "$now" "exact preimage is preserved for recovered rollback")"
      transaction_internal_atomic_write "$repository" "$recovery_path" true "$recovery"
      phase=rollback_required
    fi
  fi

  if [[ "$phase" == verification_failed || "$phase" == rollback_required ]]; then
    live_source="$(structural_source_digest_for "$target_abs")"
    live_graph="$(graph_json_for "$target_abs" | json_digest_for)"
    stored_source="$(structural_source_digest_for "$candidate_target_abs")"
    stored_graph="$(graph_json_for "$candidate_target_abs" | json_digest_for)"
    if [[ "$live_source" == "$candidate_source" && "$live_graph" == "$candidate_graph" \
      && "$stored_source" == "$base_source" && "$stored_graph" == "$base_graph" ]]; then
      if ! transaction_rename_exchange "$repository" "$common_rel" "$candidate_common_rel"; then
        sley_report_error_json "TRANSACTION_ROLLBACK_EXCHANGE_FAILED" "crash recovery could not exchange the exact preimage back" "$recovery_path"
        return 2
      fi
      live_source="$base_source"
      live_graph="$base_graph"
    elif [[ "$live_source" != "$base_source" || "$live_graph" != "$base_graph" \
      || "$stored_source" != "$candidate_source" || "$stored_graph" != "$candidate_graph" ]]; then
      sley_report_error_json "TRANSACTION_RECOVERY_AMBIGUOUS" "rollback recovery cannot classify exact live and preserved trees" "$recovery_path"
      return 2
    fi
    actual_source="$(structural_source_digest_for "$target_abs")"
    actual_graph="$(graph_json_for "$target_abs" | json_digest_for)"
    if [[ "$actual_source" != "$base_source" || "$actual_graph" != "$base_graph" ]]; then
      sley_report_error_json "TRANSACTION_ROLLBACK_VERIFICATION_FAILED" "crash recovery rollback did not restore the exact base" "$recovery_path"
      return 2
    fi
    recovery="$(printf '%s\n' "$recovery" | jq --arg source "$actual_source" --arg graph "$actual_graph" '
      .rollback.status="passed" | .rollback.actual_source_digest=$source | .rollback.actual_graph_digest=$graph')"
    recovery="$(transaction_recovery_advance_json "$recovery" "rolled_back" "rolled_back" "terminal" "$now" "crash recovery restored and verified the exact base")"
    status=rolled_back
  fi

  if [[ "${status:-}" != verified && "${status:-}" != rolled_back ]]; then
    sley_report_error_json "TRANSACTION_RECOVERY_PHASE_UNSUPPORTED" "recovery phase cannot be resumed by this command" "$phase"
    return 2
  fi
  transaction_internal_atomic_write "$repository" "$recovery_path" true "$recovery"
  report="$(transaction_apply_report_from_recovery_json "$recovery" "$status")"
  change_apply_report_from_json "$report"
  recovery_lock_held=false
  transaction_internal_rmdir "$repository" "$lock_path" >/dev/null 2>&1 || true
  trap - RETURN
  [[ "$status" == verified ]]
}

command_change_rollback() {
  local apply_file="" repository="" confirm_rollback=false arg
  while [[ "$#" -gt 0 ]]; do
    arg="$1"
    case "$arg" in
      --json) shift ;;
      --confirm-rollback) confirm_rollback=true; shift ;;
      --apply|--repository)
        if [[ "$#" -lt 2 ]]; then
          sley_report_error_json "TRANSACTION_ARGUMENT_REQUIRED" "$arg requires a value" "change:rollback"
          return 2
        fi
        [[ "$arg" == "--apply" ]] && apply_file="$2" || repository="$2"
        shift 2
        ;;
      --*)
        sley_report_error_json "TRANSACTION_ARGUMENT_UNKNOWN" "unknown change rollback option: $arg" "change:rollback"
        return 2
        ;;
      *)
        sley_report_error_json "TRANSACTION_ARGUMENT_UNKNOWN" "change rollback does not accept positional arguments" "change:rollback"
        return 2
        ;;
    esac
  done
  if [[ "$confirm_rollback" != true ]]; then
    sley_report_error_json "TRANSACTION_ROLLBACK_CONFIRMATION_REQUIRED" "rollback requires --confirm-rollback" "change:rollback"
    return 2
  fi
  transaction_require_regular_artifact "apply report" "$apply_file" || return $?
  if ! contract_jsonschema validate --schema-dir "$ROOT_DIR/docs/schemas" --schema "$SCHEMA_CHANGE_APPLY" "$apply_file" >/dev/null 2>&1; then
    sley_report_error_json "TRANSACTION_APPLY_REPORT_INVALID" "rollback requires a valid apply report" "$apply_file"
    return 2
  fi
  local apply recovery_path recovery_abs recovery expected_digest binding target_rel common_rel candidate_target_rel candidate_common_rel
  local target_abs common_abs candidate_target_abs candidate_common_abs lock_path now actual_source actual_graph
  apply="$(jq -cS . "$apply_file")"
  if [[ "$(printf '%s\n' "$apply" | jq -r '.status')" != verified || "$(printf '%s\n' "$apply" | jq -r '.recovery_ref.phase')" != verified ]]; then
    sley_report_error_json "TRANSACTION_ROLLBACK_SOURCE_NOT_VERIFIED" "manual rollback accepts only a verified apply report" "$apply_file"
    return 2
  fi
  if ! binding="$(transaction_repository_identity_binding_json "$repository")"; then
    printf '%s\n' "$binding"
    return 2
  fi
  recovery_path="$(printf '%s\n' "$apply" | jq -r '.recovery_ref.path')"
  if [[ "$recovery_path" != .sley/transactions/*/recovery/record.json || "$recovery_path" == *'/../'* ]]; then
    sley_report_error_json "TRANSACTION_RECOVERY_PATH_UNSAFE" "apply report recovery path is outside the fixed transaction journal" "$recovery_path"
    return 2
  fi
  recovery_abs="$repository/$recovery_path"
  if [[ -L "$recovery_abs" || ! -f "$recovery_abs" ]] \
    || ! contract_jsonschema validate --schema-dir "$ROOT_DIR/docs/schemas" --schema "$SCHEMA_CHANGE_RECOVERY" "$recovery_abs" >/dev/null 2>&1; then
    sley_report_error_json "TRANSACTION_RECOVERY_RECORD_INVALID" "rollback requires the exact durable recovery record" "$recovery_path"
    return 2
  fi
  recovery="$(jq -cS . "$recovery_abs")"
  local transaction_hex review_path
  transaction_hex="$(printf '%s\n' "$apply" | jq -r '.transaction.transaction_id | sub("^sha256:";"")')"
  review_path=".sley/transactions/$transaction_hex/review"
  if [[ -e "$repository/$review_path" || -L "$repository/$review_path" ]]; then
    sley_report_error_json "TRANSACTION_ALREADY_SEALED" "rollback refuses a transaction whose terminal review packet has already been published" "$review_path"
    return 2
  fi
  if [[ "${SLEY_ALLOW_TEST_HOOKS:-0}" == "1" \
    && -n "${SLEY_TEST_ROLLBACK_BEFORE_LOCK_READY:-}" \
    && -n "${SLEY_TEST_ROLLBACK_BEFORE_LOCK_RELEASE:-}" ]]; then
    : >"$SLEY_TEST_ROLLBACK_BEFORE_LOCK_READY"
    IFS= read -r _ <"$SLEY_TEST_ROLLBACK_BEFORE_LOCK_RELEASE" || true
  fi
  expected_digest="$(printf '%s\n' "$recovery" | jq 'del(.record_digest)' | json_digest_for)"
  if ! printf '%s\n' "$recovery" | jq -e --arg digest "$expected_digest" --argjson apply "$apply" --argjson binding "$binding" '
    .record_digest == $digest
    and .phase == "verified"
    and .status == "terminal"
    and .record_id == $apply.recovery_ref.record_id
    and .record_digest == $apply.recovery_ref.record_digest
    and .transaction == $apply.transaction
    and .authorization_ref == $apply.authorization_ref
    and .grant_ref == $apply.grant_ref
    and .candidate == $apply.candidate
    and .atomicity == $apply.atomicity
    and .repository.identity_digest == $binding.identity_digest' >/dev/null; then
    sley_report_error_json "TRANSACTION_RECOVERY_RECORD_MISMATCH" "durable recovery state does not match the verified apply report" "$recovery_path"
    return 2
  fi
  target_rel="$(printf '%s\n' "$recovery" | jq -r '.paths.target')"
  common_rel="$(printf '%s\n' "$recovery" | jq -r '.paths.common_root')"
  candidate_target_rel="$(printf '%s\n' "$recovery" | jq -r '.paths.candidate_root')"
  candidate_common_rel="${candidate_target_rel%"$target_rel"}$common_rel"
  target_abs="$repository/$target_rel"
  common_abs="$repository/$common_rel"
  candidate_target_abs="$repository/$candidate_target_rel"
  candidate_common_abs="$repository/$candidate_common_rel"
  lock_path="$(printf '%s\n' "$recovery" | jq -r '.paths.lock')"
  if ! transaction_internal_mkdir "$repository" "$lock_path" true 2>/dev/null; then
    sley_report_error_json "TRANSACTION_APPLY_LOCKED" "another cooperative atomic transaction holds the repository lock" "$lock_path"
    return 2
  fi
  local rollback_lock_held=true
  trap 'if [[ "${rollback_lock_held:-false}" == true ]]; then transaction_internal_rmdir "$repository" "$lock_path" >/dev/null 2>&1 || true; fi' RETURN
  if [[ -e "$repository/$review_path" || -L "$repository/$review_path" ]]; then
    sley_report_error_json "TRANSACTION_ALREADY_SEALED" "rollback refuses a transaction whose terminal review packet has already been published" "$review_path"
    return 2
  fi
  if [[ ! -e "$common_abs" || ! -e "$candidate_common_abs" || -L "$common_abs" || -L "$candidate_common_abs" ]]; then
    sley_report_error_json "TRANSACTION_ROLLBACK_TOPOLOGY_INVALID" "live or preimage common root is missing or unsafe" "$common_rel"
    return 2
  fi
  if [[ "$(structural_source_digest_for "$target_abs")" != "$(printf '%s\n' "$recovery" | jq -r '.candidate.candidate_source_digest')" \
    || "$(graph_json_for "$target_abs" | json_digest_for)" != "$(printf '%s\n' "$recovery" | jq -r '.candidate.candidate_graph_digest')" ]]; then
    sley_report_error_json "TRANSACTION_ROLLBACK_STALE" "live target no longer matches the verified candidate; rollback refuses to overwrite later work" "$target_rel"
    return 2
  fi
  if ! transaction_rename_exchange "$repository" "$common_rel" "$candidate_common_rel"; then
    sley_report_error_json "TRANSACTION_ROLLBACK_EXCHANGE_FAILED" "manual rollback atomic exchange failed" "$common_rel"
    return 2
  fi
  actual_source="$(structural_source_digest_for "$target_abs")"
  actual_graph="$(graph_json_for "$target_abs" | json_digest_for)"
  if [[ "$actual_source" != "$(printf '%s\n' "$recovery" | jq -r '.base.source_digest')" \
    || "$actual_graph" != "$(printf '%s\n' "$recovery" | jq -r '.base.graph_digest')" ]]; then
    transaction_rename_exchange "$repository" "$common_rel" "$candidate_common_rel" >/dev/null 2>&1 || true
    sley_report_error_json "TRANSACTION_ROLLBACK_VERIFICATION_FAILED" "manual rollback did not restore the exact base and was exchanged forward again" "$recovery_path"
    return 2
  fi
  now="$(date -u +%Y-%m-%dT%H:%M:%SZ)"
  recovery="$(printf '%s\n' "$recovery" | jq --arg source "$actual_source" --arg graph "$actual_graph" '
    .rollback.status="passed" | .rollback.actual_source_digest=$source | .rollback.actual_graph_digest=$graph')"
  recovery="$(transaction_recovery_advance_json "$recovery" "rolled_back" "rolled_back" "terminal" "$now" "operator-confirmed verified exchange rollback restored the exact base")"
  transaction_internal_atomic_write "$repository" "$recovery_path" true "$recovery"

  local apply_ref recovery_ref verification mutations internal_mutations next_actions report
  apply_ref="$(printf '%s\n' "$apply" | jq '{schema,status,transaction_id:.transaction.transaction_id,recovery_record_id:.recovery_ref.record_id}')"
  recovery_ref="$(printf '%s\n' "$recovery" | jq --arg path "$recovery_path" '{schema,record_id,record_digest,phase,path:$path}')"
  verification="$(jq -n --arg source "$actual_source" --arg graph "$actual_graph" '{status:"passed",source_digest:$source,graph_digest:$graph}')"
  mutations="$(printf '%s\n' "$recovery" | jq '.mutations | .repository_source=true')"
  internal_mutations="$(printf '%s\n' "$recovery" | jq '.internal_mutations')"
  next_actions='[{"kind":"review_rolled_back_transaction","reason":"manual rollback restored and verified the exact approved base"}]'
  report="$(jq -n --arg schema "$SCHEMA_CHANGE_ROLLBACK" --argjson transaction "$(printf '%s\n' "$recovery" | jq '.transaction')" \
    --argjson apply_ref "$apply_ref" --argjson authorization_ref "$(printf '%s\n' "$recovery" | jq '.authorization_ref')" \
    --argjson grant_ref "$(printf '%s\n' "$recovery" | jq '.grant_ref')" --argjson atomicity "$(printf '%s\n' "$recovery" | jq '.atomicity')" \
    --argjson recovery_ref "$recovery_ref" --argjson verification "$verification" --argjson mutations "$mutations" \
    --argjson internal_mutations "$internal_mutations" --argjson next_actions "$next_actions" '{
      schema:$schema,status:"rolled_back",transaction:$transaction,apply_ref:$apply_ref,
      authorization_ref:$authorization_ref,grant_ref:$grant_ref,atomicity:$atomicity,recovery_ref:$recovery_ref,
      verification:$verification,mutations:$mutations,internal_mutations:$internal_mutations,next_actions:$next_actions,issues:[]
    }')"
  change_rollback_report_from_json "$report"
  rollback_lock_held=false
  transaction_internal_rmdir "$repository" "$lock_path" >/dev/null 2>&1 || true
  trap - RETURN
}

transaction_artifact_ref_json() {
  local kind="$1" schema="$2" locator="$3" artifact_file="$4" digest
  digest="$(jq -cS . "$artifact_file" | json_digest_for)"
  jq -n --arg kind "$kind" --arg schema "$schema" --arg locator "$locator" --arg digest "$digest" \
    '{kind:$kind,schema:$schema,locator:$locator,digest:$digest}'
}

transaction_validate_review_trace() {
  local trace_path="$1" repository="$2" trace_abs document receipts
  [[ -n "$trace_path" ]] || { printf '[]\n'; return 0; }
  sley_reject_ambiguous_path "$trace_path"
  if [[ -L "$trace_path" ]]; then
    sley_report_error_json "TRANSACTION_TRACE_SYMLINK_DENIED" "change review refuses a symlink trace input" "$trace_path"
    return 2
  fi
  trace_abs="$(realpath -e -- "$trace_path" 2>/dev/null || true)"
  if [[ -z "$trace_abs" || ! -f "$trace_abs" ]] || ! sley_path_is_under "$trace_abs" "$repository"; then
    sley_report_error_json "TRANSACTION_TRACE_OUTSIDE_REPOSITORY" "change review trace input must be a regular file inside the repository" "$trace_path"
    return 2
  fi
  sley_enforce_file_budget "$trace_abs" "$SLEY_MAX_JSON_BYTES"
  if ! receipts="$(jq -cs . "$trace_abs" 2>/dev/null)"; then
    sley_report_error_json "TRANSACTION_TRACE_INVALID" "change review trace input must contain valid JSON documents" "$trace_path"
    return 2
  fi
  while IFS= read -r document; do
    [[ -n "$document" ]] || continue
    if ! contract_jsonschema validate --schema-dir "$ROOT_DIR/docs/schemas" --schema "sley.trace.receipt.v0" \
      <(printf '%s\n' "$document") >/dev/null 2>&1; then
      sley_report_error_json "TRANSACTION_TRACE_RECEIPT_INVALID" "change review requires every trace document to satisfy sley.trace.receipt.v0" "$trace_path"
      return 2
    fi
  done < <(printf '%s\n' "$receipts" | jq -c '.[]')
  printf '%s\n' "$receipts"
}

transaction_publish_review_directory() {
  local repository="$1" transaction_hex="$2" packet="$3" bundle="$4" evidence="$5" seal="$6"
  python3 - "$repository" "$transaction_hex" \
    3< <(printf '%s\n' "$packet") 4< <(printf '%s\n' "$bundle") \
    5< <(printf '%s\n' "$evidence") 6< <(printf '%s\n' "$seal") <<'PY'
import os
import re
import sys
import ctypes

repository, transaction_hex = sys.argv[1], sys.argv[2]
if re.fullmatch(r"[0-9a-f]{64}", transaction_hex) is None:
    raise SystemExit("invalid transaction identity")

payloads = {
    "packet.md": os.fdopen(3, "rb", closefd=False).read(),
    "bundle.json": os.fdopen(4, "rb", closefd=False).read(),
    "evidence.zjx.json": os.fdopen(5, "rb", closefd=False).read(),
    "seal.json": os.fdopen(6, "rb", closefd=False).read(),
}
flags = os.O_RDONLY | os.O_DIRECTORY | os.O_CLOEXEC | os.O_NOFOLLOW
directory_fd = os.open(repository, flags)
try:
    for component in (".sley", "transactions", transaction_hex):
        next_fd = os.open(component, flags, dir_fd=directory_fd)
        os.close(directory_fd)
        directory_fd = next_fd
    try:
        existing_fd = os.open("review", flags, dir_fd=directory_fd)
    except FileNotFoundError:
        existing_fd = None
    if existing_fd is not None:
        os.close(existing_fd)
        raise SystemExit("review packet already exists")

    staging = ".review.tmp." + str(os.getpid())
    os.mkdir(staging, 0o700, dir_fd=directory_fd)
    staging_fd = os.open(staging, flags, dir_fd=directory_fd)
    try:
        for name, payload in payloads.items():
            file_fd = os.open(
                name,
                os.O_WRONLY | os.O_CREAT | os.O_EXCL | os.O_CLOEXEC | os.O_NOFOLLOW,
                0o600,
                dir_fd=staging_fd,
            )
            try:
                view = memoryview(payload)
                while view:
                    written = os.write(file_fd, view)
                    view = view[written:]
                os.fsync(file_fd)
            finally:
                os.close(file_fd)
        os.fsync(staging_fd)
        libc = ctypes.CDLL(None, use_errno=True)
        renameat2 = getattr(libc, "renameat2", None)
        if renameat2 is None:
            raise SystemExit("renameat2 is unavailable")
        renameat2.argtypes = [ctypes.c_int, ctypes.c_char_p, ctypes.c_int, ctypes.c_char_p, ctypes.c_uint]
        renameat2.restype = ctypes.c_int
        if renameat2(directory_fd, os.fsencode(staging), directory_fd, b"review", 1) != 0:
            error = ctypes.get_errno()
            raise OSError(error, os.strerror(error))
        os.fsync(directory_fd)
    except BaseException:
        for name in payloads:
            try:
                os.unlink(name, dir_fd=staging_fd)
            except FileNotFoundError:
                pass
        os.close(staging_fd)
        staging_fd = -1
        try:
            os.rmdir(staging, dir_fd=directory_fd)
        except FileNotFoundError:
            pass
        raise
    finally:
        if staging_fd >= 0:
            os.close(staging_fd)
finally:
    os.close(directory_fd)
PY
}

command_change_review() {
  local authorization_file="" grant_file="" request_file="" preview_file="" apply_file="" rollback_file=""
  local repository="" principal="" audience="" trace_path="" confirm_review=false arg
  while [[ "$#" -gt 0 ]]; do
    arg="$1"
    case "$arg" in
      --json) shift ;;
      --confirm-review-packet) confirm_review=true; shift ;;
      --authorization|--grant|--request|--preview|--apply|--rollback|--repository|--principal|--audience|--trace)
        if [[ "$#" -lt 2 ]]; then
          sley_report_error_json "TRANSACTION_ARGUMENT_REQUIRED" "$arg requires a value" "change:review"
          return 2
        fi
        case "$arg" in
          --authorization) authorization_file="$2" ;;
          --grant) grant_file="$2" ;;
          --request) request_file="$2" ;;
          --preview) preview_file="$2" ;;
          --apply) apply_file="$2" ;;
          --rollback) rollback_file="$2" ;;
          --repository) repository="$2" ;;
          --principal) principal="$2" ;;
          --audience) audience="$2" ;;
          --trace) trace_path="$2" ;;
        esac
        shift 2
        ;;
      --*)
        sley_report_error_json "TRANSACTION_ARGUMENT_UNKNOWN" "unknown change review option: $arg" "change:review"
        return 2
        ;;
      *)
        sley_report_error_json "TRANSACTION_ARGUMENT_UNKNOWN" "change review does not accept positional arguments" "change:review"
        return 2
        ;;
    esac
  done
  if [[ "$confirm_review" != true ]]; then
    sley_report_error_json "TRANSACTION_REVIEW_CONFIRMATION_REQUIRED" "review packet publication requires --confirm-review-packet" "change:review"
    return 2
  fi

  local chain authorization_chain binding preview grant authorization apply rollback recovery recovery_path
  if ! chain="$(transaction_validate_grant_chain_json "$grant_file" "$request_file" "$preview_file" "$repository" "$principal" "$audience" true)"; then
    printf '%s\n' "$chain"
    return 2
  fi
  if ! authorization_chain="$(transaction_validate_apply_authorization_json "$authorization_file" "$grant_file" "$repository")"; then
    printf '%s\n' "$authorization_chain"
    return 2
  fi
  preview="$(printf '%s\n' "$chain" | jq -cS '.preview')"
  grant="$(printf '%s\n' "$chain" | jq -cS '.grant')"
  authorization="$(printf '%s\n' "$authorization_chain" | jq -cS '.authorization')"
  binding="$(printf '%s\n' "$authorization_chain" | jq -cS '.binding')"
  repository="$(printf '%s\n' "$binding" | jq -r '.repository')"

  transaction_require_regular_artifact "apply report" "$apply_file" || return $?
  if ! contract_jsonschema validate --schema-dir "$ROOT_DIR/docs/schemas" --schema "$SCHEMA_CHANGE_APPLY" "$apply_file" >/dev/null 2>&1; then
    sley_report_error_json "TRANSACTION_APPLY_REPORT_INVALID" "review requires a valid terminal apply report" "$apply_file"
    return 2
  fi
  apply="$(jq -cS . "$apply_file")"
  recovery_path="$(printf '%s\n' "$authorization" | jq -r '.internal_scopes[] | select(.kind == "recovery_journal") | .path')/record.json"
  if ! recovery="$(transaction_validate_recovery_record_json "$repository/$recovery_path" "$authorization" "$grant" "$binding")"; then
    printf '%s\n' "$recovery"
    return 2
  fi
  if ! printf '%s\n' "$apply" | jq -e --argjson authorization "$authorization" --argjson grant "$grant" --argjson recovery "$recovery" '
    .transaction.transaction_id == $grant.transaction.transaction_id
    and .authorization_ref == ($authorization | {schema,authorization_id,authorization_digest})
    and .grant_ref == $authorization.grant_ref
    and .candidate == $grant.candidate
    and .atomicity == $authorization.atomicity
    and .recovery_ref.record_id == $recovery.record_id' >/dev/null; then
    sley_report_error_json "TRANSACTION_REVIEW_CHAIN_MISMATCH" "apply report is not bound to the exact grant, authorization, and durable recovery identity" "$apply_file"
    return 2
  fi

  local review_lock_path review_lock_held=false
  review_lock_path="$(printf '%s\n' "$recovery" | jq -r '.paths.lock')"
  if ! transaction_internal_mkdir "$repository" "$review_lock_path" true 2>/dev/null; then
    sley_report_error_json "TRANSACTION_APPLY_LOCKED" "another cooperative atomic transaction holds the repository lock" "$review_lock_path"
    return 2
  fi
  review_lock_held=true
  trap 'if [[ "${review_lock_held:-false}" == true ]]; then transaction_internal_rmdir "$repository" "$review_lock_path" >/dev/null 2>&1 || true; fi' RETURN

  local phase apply_status terminal_outcome final_state expected_live_source expected_live_graph
  local expected_stored_source expected_stored_graph rollback_digest issues='[]'
  local rollback_ref='{"schema":null,"status":"not_required","digest":null}'
  phase="$(printf '%s\n' "$recovery" | jq -r '.phase')"
  apply_status="$(printf '%s\n' "$apply" | jq -r '.status')"
  if [[ "$phase" == verified && "$apply_status" == verified && -z "$rollback_file" ]]; then
    if ! printf '%s\n' "$recovery" | jq -e --argjson apply "$apply" '
      .status == "terminal" and .verification.status == "passed" and .rollback.status == "not_required"
      and .replay.state == "consumed" and .revocation_ref.state == "not_revoked"
      and .record_digest == $apply.recovery_ref.record_digest and .phase == $apply.recovery_ref.phase
      and $apply.replay == .replay and $apply.revocation == .revocation_ref
      and $apply.verification == .verification and $apply.rollback == .rollback
      and $apply.mutations == .mutations and $apply.internal_mutations == .internal_mutations
      and ($apply.issues | length) == 0 and $apply.next_actions[0].kind == "seal_verified_transaction"' >/dev/null; then
      sley_report_error_json "TRANSACTION_REVIEW_TERMINAL_MISMATCH" "verified apply evidence is incomplete or inconsistent with durable recovery" "$recovery_path"
      return 2
    fi
    terminal_outcome="verified_apply"
    final_state="candidate"
    expected_live_source="$(printf '%s\n' "$recovery" | jq -r '.candidate.candidate_source_digest')"
    expected_live_graph="$(printf '%s\n' "$recovery" | jq -r '.candidate.candidate_graph_digest')"
    expected_stored_source="$(printf '%s\n' "$recovery" | jq -r '.base.source_digest')"
    expected_stored_graph="$(printf '%s\n' "$recovery" | jq -r '.base.graph_digest')"
  elif [[ "$phase" == rolled_back && ( "$apply_status" == rolled_back || "$apply_status" == verified ) ]]; then
    if ! printf '%s\n' "$recovery" | jq -e '.status == "terminal" and .rollback.status == "passed" and .replay.state == "consumed"' >/dev/null; then
      sley_report_error_json "TRANSACTION_REVIEW_TERMINAL_MISMATCH" "rolled-back evidence is not a verified terminal recovery state" "$recovery_path"
      return 2
    fi
    if [[ "$apply_status" == rolled_back ]]; then
      if [[ -n "$rollback_file" ]] || ! printf '%s\n' "$apply" | jq -e --argjson recovery "$recovery" '
        .recovery_ref.record_digest == $recovery.record_digest and .recovery_ref.phase == "rolled_back"
        and .replay == $recovery.replay and .revocation == $recovery.revocation_ref
        and .verification == $recovery.verification and .rollback == $recovery.rollback
        and .mutations == $recovery.mutations and .internal_mutations == $recovery.internal_mutations
        and .next_actions[0].kind == "review_failed_transaction"' >/dev/null; then
        sley_report_error_json "TRANSACTION_REVIEW_ROLLBACK_MISMATCH" "automatic rollback review requires its exact rolled-back apply report and no manual rollback report" "$apply_file"
        return 2
      fi
    else
      transaction_require_regular_artifact "rollback report" "$rollback_file" || return $?
      if ! contract_jsonschema validate --schema-dir "$ROOT_DIR/docs/schemas" --schema "$SCHEMA_CHANGE_ROLLBACK" "$rollback_file" >/dev/null 2>&1; then
        sley_report_error_json "TRANSACTION_ROLLBACK_REPORT_INVALID" "manual rollback review requires a valid rollback report" "$rollback_file"
        return 2
      fi
      rollback="$(jq -cS . "$rollback_file")"
      if ! printf '%s\n' "$rollback" | jq -e --argjson apply "$apply" --argjson recovery "$recovery" '
        .transaction == $recovery.transaction
        and .apply_ref.transaction_id == $apply.transaction.transaction_id
        and .apply_ref.recovery_record_id == $apply.recovery_ref.record_id
        and .authorization_ref == $recovery.authorization_ref and .grant_ref == $recovery.grant_ref
        and .recovery_ref.record_id == $recovery.record_id
        and .recovery_ref.record_digest == $recovery.record_digest
        and .verification.source_digest == $recovery.base.source_digest
        and .verification.graph_digest == $recovery.base.graph_digest
        and .mutations == $recovery.mutations and .internal_mutations == $recovery.internal_mutations
        and .next_actions[0].kind == "review_rolled_back_transaction" and (.issues | length) == 0
        and $apply.replay == $recovery.replay and $apply.revocation == $recovery.revocation_ref
        and $apply.verification == $recovery.verification
        and $apply.rollback.status == "not_required"
        and $apply.rollback.expected_source_digest == $recovery.base.source_digest
        and $apply.rollback.expected_graph_digest == $recovery.base.graph_digest
        and $apply.rollback.actual_source_digest == null and $apply.rollback.actual_graph_digest == null
        and $apply.mutations == $recovery.mutations and $apply.internal_mutations == $recovery.internal_mutations
        and $apply.recovery_ref.phase == "verified" and ($apply.issues | length) == 0
        and $apply.next_actions[0].kind == "seal_verified_transaction"' >/dev/null; then
        sley_report_error_json "TRANSACTION_REVIEW_ROLLBACK_MISMATCH" "manual rollback report does not match the exact apply and durable recovery state" "$rollback_file"
        return 2
      fi
      rollback_digest="$(printf '%s\n' "$rollback" | json_digest_for)"
      rollback_ref="$(jq -n --arg schema "$SCHEMA_CHANGE_ROLLBACK" --arg digest "$rollback_digest" '{schema:$schema,status:"rolled_back",digest:$digest}')"
    fi
    terminal_outcome="verified_rollback"
    final_state="restored_base"
    expected_live_source="$(printf '%s\n' "$recovery" | jq -r '.base.source_digest')"
    expected_live_graph="$(printf '%s\n' "$recovery" | jq -r '.base.graph_digest')"
    expected_stored_source="$(printf '%s\n' "$recovery" | jq -r '.candidate.candidate_source_digest')"
    expected_stored_graph="$(printf '%s\n' "$recovery" | jq -r '.candidate.candidate_graph_digest')"
    issues="$(printf '%s\n' "$apply" | jq '
      if (.issues | length) > 0 then .issues
      else [{id:"TRANSACTION_MANUAL_ROLLBACK",message:"operator-confirmed rollback restored and verified the exact base"}] end')"
  else
    sley_report_error_json "TRANSACTION_REVIEW_NOT_TERMINAL" "review accepts only exact verified apply or verified rollback terminal evidence" "$recovery_path"
    return 2
  fi

  local target_rel candidate_target_rel target_abs candidate_target_abs live_source live_graph stored_source stored_graph
  target_rel="$(printf '%s\n' "$recovery" | jq -r '.paths.target')"
  candidate_target_rel="$(printf '%s\n' "$recovery" | jq -r '.paths.candidate_root')"
  target_abs="$repository/$target_rel"
  candidate_target_abs="$repository/$candidate_target_rel"
  if [[ ! -e "$target_abs" || ! -e "$candidate_target_abs" || -L "$target_abs" || -L "$candidate_target_abs" ]]; then
    sley_report_error_json "TRANSACTION_REVIEW_TOPOLOGY_INVALID" "review requires exact live and preserved regular transaction trees" "$recovery_path"
    return 2
  fi
  live_source="$(structural_source_digest_for "$target_abs")"
  live_graph="$(graph_json_for "$target_abs" | json_digest_for)"
  stored_source="$(structural_source_digest_for "$candidate_target_abs")"
  stored_graph="$(graph_json_for "$candidate_target_abs" | json_digest_for)"
  if [[ "$live_source" != "$expected_live_source" || "$live_graph" != "$expected_live_graph" \
    || "$stored_source" != "$expected_stored_source" || "$stored_graph" != "$expected_stored_graph" ]]; then
    sley_report_error_json "TRANSACTION_REVIEW_FINAL_DIGEST_MISMATCH" "live and preserved source trees do not match the terminal transaction outcome" "$recovery_path"
    return 2
  fi

  local trace_receipts='[]' trace_seal trace_digest trace_seal_digest evidence trace_summary evidence_digest transaction_id transaction_hex review_root
  if ! trace_receipts="$(transaction_validate_review_trace "$trace_path" "$repository")"; then
    printf '%s\n' "$trace_receipts"
    return 2
  fi
  trace_seal="$(cd "$repository" && seal_json "$target_rel")"
  trace_digest="$(printf '%s\n' "$trace_receipts" | json_digest_for)"
  trace_seal_digest="$(printf '%s\n%s\n%s\n' \
    "$(printf '%s\n' "$trace_seal" | jq -r '.source_digest')" \
    "$(printf '%s\n' "$trace_seal" | jq -r '.graph_digest')" "$trace_digest" | sha256_stream)"
  trace_seal="$(printf '%s\n' "$trace_seal" | jq --arg trace_digest "$trace_digest" --arg seal_digest "$trace_seal_digest" \
    --argjson receipt_count "$(printf '%s\n' "$trace_receipts" | jq 'length')" \
    '.trace_digest=$trace_digest | .seal_digest=$seal_digest | .receipt_count=$receipt_count')"
  evidence="$(cd "$repository" && zjx_envelope_json "$target_rel")"
  evidence="$(printf '%s\n' "$evidence" | jq --argjson receipts "$trace_receipts" \
    'if ($receipts | length) > 0 then .trace_receipts=$receipts else del(.trace_receipts) end')"
  if ! printf '%s\n' "$trace_seal" | jq -e --arg graph "$live_graph" '.graph_digest == $graph' >/dev/null \
    || ! printf '%s\n' "$evidence" | jq -e --arg graph "$live_graph" '.graph_digest == $graph' >/dev/null; then
    sley_report_error_json "TRANSACTION_REVIEW_FINAL_DIGEST_MISMATCH" "review evidence graph changed after terminal placement validation" "$target_rel"
    return 2
  fi
  if ! contract_jsonschema validate --schema-dir "$ROOT_DIR/docs/schemas" --schema "sley.trace.seal.v0" \
    <(printf '%s\n' "$trace_seal") >/dev/null 2>&1 \
    || ! contract_jsonschema validate --schema-dir "$ROOT_DIR/docs/schemas" --schema "sley.zjx.envelope.v0" \
    <(printf '%s\n' "$evidence") >/dev/null 2>&1; then
    sley_report_error_json "TRANSACTION_REVIEW_EVIDENCE_INVALID" "trace seal or ZJX evidence generation failed strict contract validation" "change:review"
    return 2
  fi
  evidence_digest="$(printf '%s\n' "$evidence" | json_digest_for)"
  trace_summary="$(printf '%s\n' "$trace_seal" | jq '{schema,seal_digest,trace_digest,receipt_count}')"
  transaction_id="$(printf '%s\n' "$recovery" | jq -r '.transaction.transaction_id')"
  transaction_hex="${transaction_id#sha256:}"
  review_root=".sley/transactions/$transaction_hex/review"

  local artifact_refs identity_chain validation_summary test_summary authority_summary adapter_summary mutation_summary no_mutation outputs final
  artifact_refs="$(jq -n \
    --argjson preview "$(transaction_artifact_ref_json preview "$SCHEMA_CHANGE_PREVIEW" input:preview "$preview_file")" \
    --argjson request "$(transaction_artifact_ref_json approval_request "$SCHEMA_CHANGE_APPROVAL_REQUEST" input:approval_request "$request_file")" \
    --argjson grant "$(transaction_artifact_ref_json grant "$SCHEMA_CHANGE_GRANT" input:grant "$grant_file")" \
    --argjson authorization "$(transaction_artifact_ref_json apply_authorization "$SCHEMA_CHANGE_APPLY_AUTHORIZATION" input:apply_authorization "$authorization_file")" \
    --argjson apply "$(transaction_artifact_ref_json apply "$SCHEMA_CHANGE_APPLY" input:apply "$apply_file")" \
    --argjson rollback "$(if [[ -n "$rollback_file" ]]; then transaction_artifact_ref_json rollback "$SCHEMA_CHANGE_ROLLBACK" input:rollback "$rollback_file"; else printf 'null\n'; fi)" \
    --argjson recovery "$(transaction_artifact_ref_json recovery "$SCHEMA_CHANGE_RECOVERY" "$recovery_path" "$repository/$recovery_path")" \
    --arg locator "$review_root/evidence.zjx.json" --arg evidence_digest "$evidence_digest" '
      [$preview,$request,$grant,$authorization,$apply]
      + (if $rollback == null then [] else [$rollback] end)
      + [$recovery,{kind:"zjx_evidence",schema:"sley.zjx.envelope.v0",locator:$locator,digest:$evidence_digest}]')"
  identity_chain="$(jq -n --argjson preview "$preview" --argjson grant "$grant" --argjson authorization "$authorization" \
    --argjson recovery "$recovery" --argjson apply "$apply" --argjson rollback "${rollback:-null}" \
    --arg request_digest "$(jq -r '.request_digest' "$request_file")" --arg rollback_digest "${rollback_digest:-}" '
      [
        {kind:"transaction",id:$recovery.transaction.transaction_id,digest:$recovery.transaction.transaction_id},
        {kind:"candidate",id:$preview.candidate.candidate_id,digest:$preview.candidate.candidate_digest},
        {kind:"approval_request",id:$grant.approval_request_ref.request_id,digest:$request_digest},
        {kind:"grant",id:$grant.grant_id,digest:$grant.grant_digest},
        {kind:"apply_authorization",id:$authorization.authorization_id,digest:$authorization.authorization_digest},
        {kind:"recovery",id:$recovery.record_id,digest:$recovery.record_digest}
      ] + (if $rollback == null then [] else [{kind:"rollback",id:("rollback:" + ($rollback_digest | sub("^sha256:";""))),digest:$rollback_digest}] end)')"
  validation_summary="$(printf '%s\n' "$recovery" | jq '{profile:.verification.profile,status:.verification.status,checks:.verification.checks,rollback:.rollback}')"
  test_summary="$(printf '%s\n' "$preview" | jq '{profile:.validation.profile,status:(if .validation.required_passed then "passed" else "failed" end),tests:.focused_tests}')"
  authority_summary="$(jq -n --argjson grant "$grant" --argjson recovery "$recovery" '{
    issuer:$grant.issuer.id,principal:$grant.principal.id,audience:$grant.audience.id,purpose:$grant.purpose,
    grant_state_at_apply:"approved",revocation_state_at_apply:$recovery.revocation_ref.state,replay_state:$recovery.replay.state
  }')"
  adapter_summary="$(printf '%s\n' "$preview" | jq '[
    {kind:"compiler",name:"sley",version:.transaction.compiler.version,network_used:false},
    {kind:"evidence_envelope",name:"zjx-preview-json",version:"sley.zjx.envelope.v0",network_used:false}
  ]')"
  mutation_summary="$(jq -n --arg outcome "$terminal_outcome" '{attempted_change:true,repository_source_changed:true,rollback_performed:($outcome == "verified_rollback"),review_artifacts_written:true}')"
  no_mutation="$(printf '%s\n' "$TRANSACTION_REVIEW_NO_MUTATION_BOUNDARIES_JSON" | jq 'reduce .[] as $name ({}; .[$name] = false)')"
  outputs="$(jq -n --arg root "$review_root" '{packet:($root+"/packet.md"),bundle:($root+"/bundle.json"),zjx_evidence:($root+"/evidence.zjx.json"),seal:($root+"/seal.json")}')"
  final="$(jq -n --arg state "$final_state" --arg source "$live_source" --arg graph "$live_graph" '{state:$state,source_digest:$source,graph_digest:$graph}')"

  local review_core review_digest review_id review packet packet_digest bundle_digest apply_digest seal_core seal_digest seal_id seal created_at
  review_core="$(jq -n --arg schema "$SCHEMA_CHANGE_REVIEW" --arg outcome "$terminal_outcome" \
    --argjson transaction "$(printf '%s\n' "$recovery" | jq '.transaction')" \
    --argjson repository_json "$(printf '%s\n' "$grant" | jq '.repository')" --argjson base "$(printf '%s\n' "$grant" | jq '.base')" \
    --argjson final "$final" --argjson artifact_refs "$artifact_refs" --argjson identity_chain "$identity_chain" \
    --argjson validation_summary "$validation_summary" --argjson test_summary "$test_summary" --argjson trace_summary "$trace_summary" \
    --argjson authority_summary "$authority_summary" --argjson adapter_summary "$adapter_summary" \
    --argjson mutation_summary "$mutation_summary" --argjson no_mutation "$no_mutation" --argjson outputs "$outputs" --argjson issues "$issues" '{
      schema:$schema,status:"ready",transaction:$transaction,terminal_outcome:$outcome,repository:$repository_json,base:$base,final:$final,
      artifact_refs:$artifact_refs,identity_chain:$identity_chain,validation_summary:$validation_summary,test_summary:$test_summary,
      trace_summary:$trace_summary,authority_summary:$authority_summary,adapter_summary:$adapter_summary,
      mutation_summary:$mutation_summary,no_mutation_boundary:$no_mutation,outputs:$outputs,
      next_actions:[{kind:"inspect_transaction_seal",reason:"inspect packet.md and seal.json before any later release decision"}],issues:$issues
    }')"
  review_digest="$(printf '%s\n' "$review_core" | json_digest_for)"
  review_id="review:${review_digest#sha256:}"
  review="$(printf '%s\n' "$review_core" | jq --arg id "$review_id" --arg digest "$review_digest" \
    '{schema,status,review_id:$id,review_digest:$digest} + del(.schema,.status)')"
  review="$(change_review_report_from_json "$review")"
  if ! contract_jsonschema validate --schema-dir "$ROOT_DIR/docs/schemas" --schema "$SCHEMA_CHANGE_REVIEW" \
    <(printf '%s\n' "$review") >/dev/null 2>&1; then
    sley_report_error_json "TRANSACTION_REVIEW_REPORT_INVALID" "generated review bundle failed its strict contract" "change:review"
    return 2
  fi

  packet="$(printf '%s\n' "$review" | jq -r --arg goal "$(printf '%s\n' "$preview" | jq -r '.transaction.goal')" \
    --arg outcome_text "$(printf '%s\n' "$preview" | jq -r '.plan.requested_outcome')" '
    def clean: gsub("[\u0000-\u001f\u007f]"; " ") | gsub("`"; " ") | .[0:512];
    "# Sley Transaction Review\n\n" +
    "Transaction: `" + .transaction.transaction_id + "`\n\n" +
    "Outcome: **" + .terminal_outcome + "**\n\n" +
    "Goal: " + ($goal | clean) + "\n\n" +
    "Requested change: " + ($outcome_text | clean) + "\n\n" +
    "Final source digest: `" + .final.source_digest + "`\n\n" +
    "Final graph digest: `" + .final.graph_digest + "`\n\n" +
    "Validation: " + .validation_summary.status + " under `" + .validation_summary.profile + "`\n\n" +
    "Focused tests: " + (.test_summary.tests | length | tostring) + " passed\n\n" +
    "Rollback: " + .validation_summary.rollback.status + "\n\n" +
    "Trace receipts: " + (.trace_summary.receipt_count | tostring) + "\n\n" +
    "Authority: `" + (.authority_summary.issuer | clean) + "` -> `" + (.authority_summary.principal | clean) + "` for `" + (.authority_summary.audience | clean) + "`\n\n" +
    "Review digest: `" + .review_digest + "`\n\n" +
    "This review command wrote only the four internal review artifacts listed in the machine bundle. It did not mutate source, the Git index, trace sidecars, external systems, or provider state."
  ')"
  packet_digest="$(printf '%s\n' "$packet" | sha256_stream)"
  bundle_digest="$(printf '%s\n' "$review" | json_digest_for)"
  apply_digest="$(printf '%s\n' "$apply" | json_digest_for)"
  created_at="$(printf '%s\n' "$recovery" | jq -r '.events[-1].at')"
  seal_core="$(jq -n --arg schema "$SCHEMA_CHANGE_TRANSACTION_SEAL" --arg transaction_id "$transaction_id" \
    --arg outcome "$terminal_outcome" --arg source "$live_source" --arg graph "$live_graph" \
    --argjson trace_ref "$(printf '%s\n' "$trace_seal" | jq '{schema,seal_digest,trace_digest}')" \
    --argjson apply_ref "$(jq -n --arg schema "$SCHEMA_CHANGE_APPLY" --arg status "$apply_status" --arg digest "$apply_digest" '{schema:$schema,status:$status,digest:$digest}')" \
    --argjson rollback_ref "$rollback_ref" --arg review_id "$review_id" --arg review_digest "$review_digest" \
    --arg review_path "$review_root/bundle.json" --arg packet_digest "$packet_digest" --arg bundle_digest "$bundle_digest" \
    --arg zjx_digest "$evidence_digest" --arg created_at "$created_at" '{
      schema:$schema,status:"sealed",transaction_id:$transaction_id,terminal_outcome:$outcome,source_digest:$source,graph_digest:$graph,
      trace_seal_ref:$trace_ref,apply_ref:$apply_ref,rollback_ref:$rollback_ref,
      review_ref:{schema:"sley.change.review.v0",review_id:$review_id,review_digest:$review_digest,path:$review_path},
      packet_digest:$packet_digest,bundle_digest:$bundle_digest,zjx_digest:$zjx_digest,created_at:$created_at,issues:[]
    }')"
  seal_digest="$(printf '%s\n' "$seal_core" | json_digest_for)"
  seal_id="transaction-seal:${seal_digest#sha256:}"
  seal="$(printf '%s\n' "$seal_core" | jq --arg id "$seal_id" --arg digest "$seal_digest" \
    '{schema,status,seal_id:$id,seal_digest:$digest} + del(.schema,.status)')"
  seal="$(change_transaction_seal_report_from_json "$seal")"
  if ! contract_jsonschema validate --schema-dir "$ROOT_DIR/docs/schemas" --schema "$SCHEMA_CHANGE_TRANSACTION_SEAL" \
    <(printf '%s\n' "$seal") >/dev/null 2>&1; then
    sley_report_error_json "TRANSACTION_SEAL_INVALID" "generated transaction seal failed its strict contract" "change:review"
    return 2
  fi
  if [[ "$(structural_source_digest_for "$target_abs")" != "$expected_live_source" \
    || "$(graph_json_for "$target_abs" | json_digest_for)" != "$expected_live_graph" \
    || "$(structural_source_digest_for "$candidate_target_abs")" != "$expected_stored_source" \
    || "$(graph_json_for "$candidate_target_abs" | json_digest_for)" != "$expected_stored_graph" ]]; then
    sley_report_error_json "TRANSACTION_REVIEW_FINAL_DIGEST_MISMATCH" "source placement changed before atomic review publication" "$recovery_path"
    return 2
  fi
  if ! transaction_publish_review_directory "$repository" "$transaction_hex" "$packet" "$review" "$evidence" "$seal"; then
    sley_report_error_json "TRANSACTION_REVIEW_PUBLISH_FAILED" "review artifacts could not be atomically published to the fixed transaction journal" "$review_root"
    return 2
  fi
  review_lock_held=false
  transaction_internal_rmdir "$repository" "$review_lock_path" >/dev/null 2>&1 || true
  trap - RETURN
  printf '%s\n' "$review"
}

command_change_inspect() {
  local goal="" actor="" nonce="" created_at="" expires_at="" trace_path="" target="" arg
  local -a positionals=()
  while [[ "$#" -gt 0 ]]; do
    arg="$1"
    case "$arg" in
      --json) shift ;;
      --goal|--actor|--nonce|--created-at|--expires-at|--trace)
        if [[ "$#" -lt 2 ]]; then
          sley_report_error_json "TRANSACTION_ARGUMENT_REQUIRED" "$arg requires a value" "change:inspect"
          return 2
        fi
        case "$arg" in
          --goal) goal="$2" ;;
          --actor) actor="$2" ;;
          --nonce) nonce="$2" ;;
          --created-at) created_at="$2" ;;
          --expires-at) expires_at="$2" ;;
          --trace) trace_path="$2" ;;
        esac
        shift 2
        ;;
      --*)
        sley_report_error_json "TRANSACTION_ARGUMENT_UNKNOWN" "unknown change inspect option: $arg" "change:inspect"
        return 2
        ;;
      *) positionals+=("$arg"); shift ;;
    esac
  done

  if [[ "${#positionals[@]}" -ne 1 ]]; then
    sley_report_error_json "TRANSACTION_TARGET_REQUIRED" "change inspect requires exactly one file-or-project target" "change:inspect"
    return 2
  fi
  target="${positionals[0]}"
  if [[ -z "$goal" || -z "$actor" || -z "$nonce" || -z "$created_at" || -z "$expires_at" ]]; then
    sley_report_error_json "TRANSACTION_BINDING_REQUIRED" "change inspect requires --goal, --actor, --nonce, --created-at, and --expires-at" "change:inspect"
    return 2
  fi
  if [[ ! "$nonce" =~ ^[A-Za-z0-9._:-]{8,128}$ ]]; then
    sley_report_error_json "TRANSACTION_NONCE_INVALID" "transaction nonce must be 8-128 identifier characters" "change:inspect"
    return 2
  fi
  if [[ ! "$created_at" =~ ^[0-9]{4}-[0-9]{2}-[0-9]{2}T[0-9]{2}:[0-9]{2}:[0-9]{2}Z$ ]] \
    || [[ ! "$expires_at" =~ ^[0-9]{4}-[0-9]{2}-[0-9]{2}T[0-9]{2}:[0-9]{2}:[0-9]{2}Z$ ]]; then
    sley_report_error_json "TRANSACTION_TIME_INVALID" "transaction times must use UTC RFC3339 seconds" "change:inspect"
    return 2
  fi
  local created_epoch expires_epoch
  created_epoch="$(date -u -d "$created_at" +%s 2>/dev/null || true)"
  expires_epoch="$(date -u -d "$expires_at" +%s 2>/dev/null || true)"
  if [[ -z "$created_epoch" || -z "$expires_epoch" || "$expires_epoch" -le "$created_epoch" ]]; then
    sley_report_error_json "TRANSACTION_TIME_ORDER_INVALID" "transaction expiry must be later than creation" "change:inspect"
    return 2
  fi
  if [[ -L "$target" ]]; then
    sley_report_error_json "TRANSACTION_TARGET_SYMLINK_DENIED" "change inspect refuses a symlink target" "change:inspect"
    return 2
  fi
  sley_reject_ambiguous_path "$target"
  local target_abs target_dir repo_root repo_display target_display git_dir base_commit identity_digest
  target_abs="$(realpath -e -- "$target" 2>/dev/null || true)"
  if [[ -z "$target_abs" ]]; then
    sley_report_error_json "TRANSACTION_TARGET_NOT_FOUND" "change inspect target does not exist" "$target"
    return 2
  fi
  target_dir="$target_abs"
  [[ -f "$target_abs" ]] && target_dir="$(dirname -- "$target_abs")"
  repo_root="$(git -C "$target_dir" rev-parse --show-toplevel 2>/dev/null || true)"
  if [[ -z "$repo_root" ]] || ! sley_path_is_under "$target_abs" "$repo_root"; then
    sley_report_error_json "TRANSACTION_GIT_REPOSITORY_REQUIRED" "change inspect requires a target inside one Git repository" "$target"
    return 2
  fi
  repo_display="$(realpath --relative-to="$PWD" -- "$repo_root")"
  target_display="$(realpath --relative-to="$repo_root" -- "$target_abs")"
  [[ "$repo_display" == "" ]] && repo_display="."
  [[ "$target_display" == "" ]] && target_display="."
  git_dir="$(git -C "$repo_root" rev-parse --absolute-git-dir)"
  base_commit="$(git -C "$repo_root" rev-parse --verify HEAD 2>/dev/null || true)"
  if [[ -z "$base_commit" ]]; then
    sley_report_error_json "TRANSACTION_BASE_COMMIT_REQUIRED" "change inspect requires a repository with a committed base" "$target"
    return 2
  fi
  identity_digest="$(printf '%s\n' "$git_dir" | sha256_stream)"

  if [[ -n "$trace_path" ]]; then
    sley_reject_ambiguous_path "$trace_path"
    if [[ -L "$trace_path" ]]; then
      sley_report_error_json "TRANSACTION_TRACE_SYMLINK_DENIED" "change inspect refuses a symlink trace input" "$trace_path"
      return 2
    fi
    local trace_abs
    trace_abs="$(realpath -m -- "$trace_path")"
    if ! sley_path_is_under "$trace_abs" "$repo_root"; then
      sley_report_error_json "TRANSACTION_TRACE_OUTSIDE_REPOSITORY" "transaction trace input must remain inside the repository" "$trace_path"
      return 2
    fi
    if [[ ! -f "$trace_abs" ]]; then
      sley_report_error_json "TRANSACTION_TRACE_NOT_FOUND" "transaction trace input must be a regular file" "$trace_path"
      return 2
    fi
    if ! jq -s . "$trace_abs" >/dev/null 2>&1; then
      sley_report_error_json "TRANSACTION_TRACE_INVALID" "transaction trace input must contain valid JSON documents" "$trace_path"
      return 2
    fi
    trace_path="$trace_abs"
  fi

  local source_digest graph graph_digest trace_receipts trace_digest query check lint plan
  local source_bytes=0 context_bytes context_digest transaction_id report state base_mode
  local base_json compiler_json context_core inspection_json transaction_core transaction_json mutations_json next_actions
  local file size
  local -a transaction_source_files=()
  mapfile -t transaction_source_files < <(collect_files "$target_abs")
  for file in "${transaction_source_files[@]}"; do
    size="$(wc -c < "$file" | tr -d ' ')"
    source_bytes=$((source_bytes + size))
  done
  source_digest="$(structural_source_digest_for "$target_abs")"
  graph="$(graph_json_for "$target_abs")"
  graph_digest="$(printf '%s\n' "$graph" | json_digest_for)"
  trace_receipts="$(trace_receipts_json "$trace_path")"
  trace_digest="$(printf '%s\n' "$trace_receipts" | json_digest_for)"
  query="$(query_json "$target_abs" all)"
  check="$(check_json "$target_abs" || true)"
  lint="$(lint_json "$target_abs")"
  plan="$(command_plan --json --graft-templates "$target_abs" || true)"
  state="$(printf '%s\n' "$TRANSACTION_LIFECYCLE_STATES_JSON" | jq -r '.[1]')"
  base_mode="$(printf '%s\n' "$TRANSACTION_BASE_MODES_JSON" | jq -r '.[0]')"

  base_json="$(jq -n \
    --arg mode "$base_mode" \
    --arg commit "$base_commit" \
    --arg source_digest "$source_digest" \
    --arg graph_digest "$graph_digest" \
    --arg trace_digest "$trace_digest" \
    '{mode:$mode,commit:$commit,source_digest:$source_digest,graph_digest:$graph_digest,trace_digest:$trace_digest}')"
  compiler_json="$(jq -n --arg version "$VERSION" --arg inspect "$SCHEMA_TRANSACTION_INSPECT" '{
    version:$version,
    contract_versions:{
      diagnostics:"sley.diagnostics.report.v1",
      graph_slice:"sley.symbol_graph.slice.v1",
      verify:"sley.verify.report.v1",
      inspection:$inspect
    }
  }')"
  context_core="$(jq -n \
    --arg target "$target_display" \
    --argjson query "$query" \
    --argjson check "$check" \
    --argjson lint "$lint" \
    --argjson plan "$plan" '
    {
      target:$target,
      entry_module:($query.entry_module // null),
      summary:{
        module_count:($query.modules | length),
        task_count:($query.tasks | length),
        call_count:($query.calls | length),
        diagnostic_count:($check.diagnostics | length),
        lint_finding_count:($lint.findings | length)
      },
      target_nodes:([$query.tasks[]?.id] | unique | .[:64]),
      authority_requirements:([$query.tasks[]?.effects[]?] | unique),
      graft_affordances:([
        $plan.graft_templates[]?.operation.op,
        $plan.transaction_templates[]?.transaction.ops[]?.op
      ] | unique),
      documentation_refs:["SLEY_AI.md","docs/SleyLanguageSpec.md","docs/contracts.md"],
      validation:{
        check_schema:($check.schema // "sley.diagnostics.report.v0"),
        check_status:($check.status // "error"),
        lint_schema:($lint.schema // "sley.lint.report.v0"),
        lint_status:($lint.status // "findings")
      }
    }')"
  context_digest="$(printf '%s\n' "$context_core" | json_digest_for)"
  context_bytes="$(printf '%s\n' "$context_core" | wc -c | tr -d ' ')"
  inspection_json="$(printf '%s\n' "$context_core" | jq \
    --arg context_digest "$context_digest" \
    --argjson source_bytes "$source_bytes" \
    --argjson context_bytes "$context_bytes" \
    '. + {context_digest:$context_digest,bounds:{max_target_nodes:64,source_bytes:$source_bytes,context_bytes:$context_bytes}}')"
  transaction_core="$(jq -n \
    --arg state "$state" \
    --arg root "$repo_display" \
    --arg identity_digest "$identity_digest" \
    --arg actor "$actor" \
    --arg goal "$goal" \
    --arg nonce "$nonce" \
    --arg created_at "$created_at" \
    --arg expires_at "$expires_at" \
    --argjson lifecycle "$TRANSACTION_LIFECYCLE_STATES_JSON" \
    --argjson base "$base_json" \
    --argjson compiler "$compiler_json" '
    {
      state:$state,
      history:$lifecycle[:2],
      repository:{root:$root,identity_digest:$identity_digest},
      base:$base,
      compiler:$compiler,
      actor:$actor,
      goal:$goal,
      nonce:$nonce,
      created_at:$created_at,
      expires_at:$expires_at
    }')"
  transaction_id="$(printf '%s\n' "$transaction_core" | json_digest_for)"
  transaction_json="$(printf '%s\n' "$transaction_core" | jq --arg transaction_id "$transaction_id" '{transaction_id:$transaction_id} + .')"
  mutations_json="$(printf '%s\n' "$TRANSACTION_MUTATION_BOUNDARIES_JSON" | jq 'reduce .[] as $name ({}; .[$name] = false)')"
  next_actions="$(jq -n --arg target "$target_display" --arg node "$(printf '%s\n' "$inspection_json" | jq -r '.target_nodes[0] // ""')" '
    ([{kind:"plan_with_checked_primitives",reason:"use the existing checked plan surface before constructing an immutable candidate",command:["sley","plan","--json","--graft-templates",$target]}]
    + if $node == "" then [] else [{kind:"inspect_graph_slice",reason:"inspect bounded graph context for the primary target node",command:["sley","graph","--json","--slice",$node,$target]}] end)')"
  report="$(jq -n \
    --arg schema "$SCHEMA_TRANSACTION_INSPECT" \
    --argjson transaction "$transaction_json" \
    --argjson inspection "$inspection_json" \
    --argjson mutations "$mutations_json" \
    --argjson next_actions "$next_actions" '{
      schema:$schema,
      status:"inspected",
      transaction:$transaction,
      inspection:$inspection,
      mutations:$mutations,
      next_actions:$next_actions,
      issues:[]
    }')"
  transaction_inspect_report_from_json "$report"
}

command_change() {
  local subcommand="${1:-}"
  case "$subcommand" in
    inspect) shift; command_change_inspect "$@" ;;
    plan) shift; command_change_plan "$@" ;;
    preview) shift; command_change_preview "$@" ;;
    approval-request) shift; command_change_approval_request "$@" ;;
    approve) shift; command_change_approve "$@" ;;
    apply-authorization) shift; command_change_apply_authorization "$@" ;;
    revocation-record) shift; command_change_revocation_record "$@" ;;
    apply) shift; command_change_apply "$@" ;;
    recover) shift; command_change_recover "$@" ;;
    rollback) shift; command_change_rollback "$@" ;;
    review) shift; command_change_review "$@" ;;
    "")
      sley_report_error_json "TRANSACTION_SUBCOMMAND_REQUIRED" "sley change requires a subcommand; inspect, plan, preview, approval-request, approve, apply-authorization, revocation-record, apply, recover, rollback, and review are available" "change"
      return 2
      ;;
    *)
      sley_report_error_json "TRANSACTION_SUBCOMMAND_UNSUPPORTED" "unsupported sley change subcommand: $subcommand" "change"
      return 2
      ;;
  esac
}
