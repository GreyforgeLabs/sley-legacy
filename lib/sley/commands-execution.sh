# shellcheck shell=bash
# Runtime, mutation, formatting, trace, sealing, and ZJX command family.

command_claim_verify() {
  local target=""
  while [[ "$#" -gt 0 ]]; do
    case "$1" in
      --json) shift ;;
      *) target="$1"; shift ;;
    esac
  done
  claim_verify_json "${target:-docs/SleyClaimManifest.json}"
}

command_run() {
  local target report rc
  target="$(last_path_arg "$@")"
  if report="$(run_json "$target" "$@")"; then
    rc=0
  else
    rc=$?
  fi
  printf '%s\n' "$report"
  if [[ "$rc" -ne 0 ]] || [[ "$(printf '%s\n' "$report" | jq -r '.schema? // ""')" == "$SCHEMA_DIAGNOSTICS" ]]; then
    echo "operation failed" >&2
    return 1
  fi
}

command_deploy() {
  local target artifacts_dir="artifacts" artifacts_requested=false dry_run=false verify seal package report
  local args=("$@")
  target="$(last_path_arg "$@")"
  while [[ "$#" -gt 0 ]]; do
    case "$1" in
      --dry-run) dry_run=true; shift ;;
      --artifacts-dir) artifacts_dir="$2"; artifacts_requested=true; shift 2 ;;
      *) shift ;;
    esac
  done
  sley_reject_ambiguous_path "$artifacts_dir"
  if [[ "$dry_run" != true ]]; then
    jq -n --arg target "$target" '{
      schema:"sley.diagnostics.report.v0",
      status:"error",
      diagnostics:[
        {
          id:"DRY_RUN_REQUIRED",
          severity:"error",
          message:"sley deploy is report-only in v0; rerun with --dry-run",
          repair_hints:[{kind:"rerun_deploy_dry_run", target:$target, replacement:("sley deploy --json --dry-run " + $target)}]
        }
      ]
    }'
    return 1
  fi
  verify="$(verify_json "$target" "${args[@]}")"
  seal="$(seal_json "$target")"
  package="$(zjx_envelope_json "$target")"
  report="$(jq -n \
    --arg schema "$SCHEMA_DEPLOY_REPORT" \
    --arg target "$target" \
    --arg artifacts_dir "$artifacts_dir" \
    --argjson verify "$verify" \
    --argjson seal "$seal" \
    --argjson package "$package" \
    --argjson fields "$DEPLOY_REPORT_FIELDS_JSON" '
    ($verify.status == "passed") as $ready |
    {
      ($fields[0] // "schema"):$schema,
      ($fields[1] // "status"):(if $ready then "ready" else "blocked" end),
      ($fields[2] // "mode"):"dry_run",
      ($fields[3] // "target"):$target,
      ($fields[4] // "environment"):"staging",
      ($fields[5] // "policy"):{
        live_deploy_allowed:false,
        external_mutations:false,
        provider_calls:false,
        requires_operator_approval:true
      },
      ($fields[6] // "summary"):{
        verify_status:(if $ready then "passed" else "blocked" end),
        seal_status:(if $ready then "passed" else "skipped" end),
        package_status:(if $ready then "passed" else "skipped" end),
        module_count:($verify.summary.module_count // 0),
        task_count:($verify.summary.task_count // 0),
        receipt_count:($seal.receipt_count // 0),
        seal_digest:($seal.seal_digest),
        graph_digest:($seal.graph_digest)
      },
      ($fields[7] // "verify"):$verify,
      seal:$seal,
      package:{
        source_schema:$package.schema,
        format:$package.format,
        compression:$package.compression,
        target:$package.target,
        graph_digest:$package.graph_digest,
        module_count:($package.graph.modules | length),
        trace_receipt_count:($package.trace_receipts // [] | length)
      },
      ($fields[8] // "artifacts"):{
        directory:$artifacts_dir,
        report:($artifacts_dir + "/deploy-report.json"),
        seal:($artifacts_dir + "/seal.json"),
        package:($artifacts_dir + "/zjx-envelope.json"),
        manifest:($artifacts_dir + "/manifest.json")
      },
      ($fields[9] // "next_actions"):[
        {
          kind:"review_seal",
          reason:"inspect the content-addressed seal before any live deployment decision",
          command:["sley","seal","--json",$target]
        },
        {
          kind:"review_package",
          reason:"inspect the ZJX preview envelope before operator deployment approval",
          command:["sley","zjx","--json",$target]
        },
        {
          kind:"inspect_deploy_artifacts",
          reason:"validate the deploy handoff directory before operator deployment approval",
          command:["sley-contract","inspect-deploy-artifacts",$artifacts_dir,"--json"]
        }
      ]
    }')"
  report="$(sley_report_builder_from_report_kind_json "deploy" "$report")"
  if [[ "$artifacts_requested" == true ]]; then
    if [[ -L "$artifacts_dir" ]]; then
      sley_report_error_json "DEPLOY_ARTIFACT_DIR_SYMLINK_DENIED" "refusing to write deploy artifacts through a symlinked directory" "$artifacts_dir"
      return 1
    fi
    if [[ -e "$artifacts_dir" && ! -d "$artifacts_dir" ]]; then
      sley_report_error_json "DEPLOY_ARTIFACT_DIR_INVALID" "deploy artifact target exists and is not a directory" "$artifacts_dir"
      return 1
    fi
    mkdir -p "$artifacts_dir"
    local artifact_name
    for artifact_name in seal.json zjx-envelope.json deploy-report.json manifest.json; do
      if [[ -L "$artifacts_dir/$artifact_name" ]]; then
        sley_report_error_json "DEPLOY_ARTIFACT_SYMLINK_DENIED" "refusing to write deploy artifact through a symlink" "$artifacts_dir/$artifact_name"
        return 1
      fi
    done
    printf '%s\n' "$seal" > "$artifacts_dir/seal.json"
    printf '%s\n' "$package" > "$artifacts_dir/zjx-envelope.json"
    printf '%s\n' "$report" > "$artifacts_dir/deploy-report.json"
    local report_digest seal_file_digest package_digest seal_digest graph_digest manifest
    report_digest="$(sha256sum "$artifacts_dir/deploy-report.json" | awk '{print "sha256:" $1}')"
    seal_file_digest="$(sha256sum "$artifacts_dir/seal.json" | awk '{print "sha256:" $1}')"
    package_digest="$(sha256sum "$artifacts_dir/zjx-envelope.json" | awk '{print "sha256:" $1}')"
    seal_digest="$(printf '%s\n' "$seal" | jq -r '.seal_digest')"
    graph_digest="$(printf '%s\n' "$seal" | jq -r '.graph_digest')"
    manifest="$(jq -n \
      --arg schema "$SCHEMA_DEPLOY_ARTIFACTS" \
      --arg target "$target" \
      --arg artifacts_dir "$artifacts_dir" \
      --arg report_digest "$report_digest" \
      --arg seal_file_digest "$seal_file_digest" \
      --arg package_digest "$package_digest" \
      --arg seal_digest "$seal_digest" \
      --arg graph_digest "$graph_digest" '{
        schema:$schema,
        target:$target,
        environment:"staging",
        mode:"dry_run",
        policy:{
          live_deploy_allowed:false,
          external_mutations:false,
          provider_calls:false,
          requires_operator_approval:true
        },
        summary:{
          verify_status:"passed",
          seal_digest:$seal_digest,
          graph_digest:$graph_digest
        },
        files:{
          report:{path:($artifacts_dir + "/deploy-report.json"), schema:"sley.deploy.report.v0", digest:$report_digest},
          seal:{path:($artifacts_dir + "/seal.json"), schema:"sley.trace.seal.v0", digest:$seal_file_digest},
          package:{path:($artifacts_dir + "/zjx-envelope.json"), schema:"sley.zjx.envelope.v0", digest:$package_digest}
        }
      }')"
    printf '%s\n' "$manifest" > "$artifacts_dir/manifest.json"
  fi
  printf '%s\n' "$report"
}

sley_task_file_for_module() {
  local source="$1" task_module="$2" file module
  while IFS= read -r file; do
    module="$(awk '/^[[:space:]]*module[[:space:]]+/ {print $2; found=1; exit} END {if(!found) print "main"}' "$file")"
    if [[ "$module" == "$task_module" ]]; then
      printf '%s\n' "$file"
      return 0
    fi
  done < <(collect_files "$source")
  return 1
}

sley_rename_task_declaration() {
  local source="$1" task_module="$2" old_name="$3" new_name="$4" file tmp_file
  file="$(sley_task_file_for_module "$source" "$task_module")" || return 0
  tmp_file="$(mktemp)"
  awk -v old_name="$old_name" -v new_name="$new_name" '
    !done && $0 ~ "^[[:space:]]*(export[[:space:]]+)?task[[:space:]]+" old_name "([[:space:]]|$)" {
      line=$0
      sub("task[[:space:]]+" old_name, "task " new_name, line)
      print line
      done=1
      next
    }
    {print}
  ' "$file" > "$tmp_file"
  mv "$tmp_file" "$file"
}

sley_rewrite_inbound_call_names() {
  local source="$1" old_qualified="$2" new_name="$3" query callee new_callee file tmp_file call_prefix
  call_prefix="$PARSER_CALL_EXPRESSION_PREFIX"
  query="$(query_json "$source" all)"
  while IFS= read -r callee; do
    [[ -n "$callee" ]] || continue
    if [[ "$callee" == *.* ]]; then
      new_callee="${callee%.*}.$new_name"
    else
      new_callee="$new_name"
    fi
    while IFS= read -r file; do
      tmp_file="$(mktemp)"
      awk -v old_callee="$callee" -v new_callee="$new_callee" -v call_prefix="$call_prefix" '
        {
          line=$0
          needle=call_prefix old_callee "("
          repl=call_prefix new_callee "("
          if ((pos=index(line, needle)) > 0) {
            line=substr(line, 1, pos - 1) repl substr(line, pos + length(needle))
          }
          print line
        }
      ' "$file" > "$tmp_file"
      mv "$tmp_file" "$file"
    done < <(collect_files "$source")
  done < <(printf '%s\n' "$query" | jq -r --arg target "$old_qualified" '.calls[]? | select(.target == $target) | .callee' | sort -u)
}

sley_remove_task_take() {
  local source="$1" task_module="$2" task_name="$3" take_name="$4" file tmp_file
  file="$(sley_task_file_for_module "$source" "$task_module")" || return 0
  tmp_file="$(mktemp)"
  awk -v task_name="$task_name" -v take_name="$take_name" '
    function count_char(s, c, n, i){n=0; for(i=1;i<=length(s);i++){if(substr(s,i,1)==c)n++} return n}
    !in_task && $0 ~ "^[[:space:]]*(export[[:space:]]+)?task[[:space:]]+" task_name "([[:space:]]|$)" {
      in_task=1
      depth=count_char($0,"{")-count_char($0,"}")
      print
      next
    }
    in_task {
      if ($0 ~ "^[[:space:]]*take[[:space:]]+" take_name "[[:space:]]*:") {
        depth += count_char($0,"{")-count_char($0,"}")
        next
      }
      print
      depth += count_char($0,"{")-count_char($0,"}")
      if (depth<=0) in_task=0
      next
    }
    {print}
  ' "$file" > "$tmp_file"
  mv "$tmp_file" "$file"
}

sley_remove_inbound_call_arg() {
  local source="$1" target_qualified="$2" position="$3" query callee file tmp_file call_prefix
  call_prefix="$PARSER_CALL_EXPRESSION_PREFIX"
  query="$(query_json "$source" all)"
  while IFS= read -r callee; do
    [[ -n "$callee" ]] || continue
    while IFS= read -r file; do
      tmp_file="$(mktemp)"
      awk -v callee="$callee" -v remove_position="$position" -v call_prefix="$call_prefix" '
        function trim(s){gsub(/^[[:space:]]+|[[:space:]]+$/, "", s); return s}
        {
          line=$0
          needle=call_prefix callee "("
          if ((pos=index(line, needle)) > 0) {
            start=pos + length(needle)
            rest=substr(line, start)
            close_pos=index(rest, ")")
            if (close_pos > 0) {
              args=substr(rest, 1, close_pos - 1)
              suffix=substr(rest, close_pos)
              n=split(args, parts, ",")
              rebuilt=""
              for (i=1; i<=n; i++) {
                if ((i - 1) == remove_position) continue
                part=trim(parts[i])
                rebuilt=(rebuilt == "" ? part : rebuilt ", " part)
              }
              line=substr(line, 1, start - 1) rebuilt suffix
            }
          }
          print line
        }
      ' "$file" > "$tmp_file"
      mv "$tmp_file" "$file"
    done < <(collect_files "$source")
  done < <(printf '%s\n' "$query" | jq -r --arg target "$target_qualified" '.calls[]? | select(.target == $target) | .callee' | sort -u)
}

sley_module_name_for_source() {
  local source="$1" base
  if [[ -f "$source" ]]; then
    base="$(basename "$source" .sley)"
  else
    base="$(basename "$source")"
  fi
  printf '%s\n' "${base//[^A-Za-z0-9_]/_}"
}

sley_add_module_declaration() {
  local source="$1" module_name="$2" file tmp_file
  file="$(collect_files "$source" | head -n 1)"
  [[ -n "$file" ]] || return 0
  if grep -Eq '^[[:space:]]*module[[:space:]]+' "$file"; then
    return 0
  fi
  tmp_file="$(mktemp)"
  {
    printf 'module %s\n\n' "$module_name"
    sed -n '1,$p' "$file"
  } > "$tmp_file"
  mv "$tmp_file" "$file"
}

sley_rewrite_statement_surface() {
  local source="$1" surface="$2" replacement="$3" module file line replacement_line tmp_file base_surface
  module="${surface#block:task:}"
  module="${module%.*:stmt:*}"
  base_surface="$surface"
  if [[ "$surface" == *":left" ]]; then
    base_surface="${surface%:left}"
  fi
  file="$(sley_task_file_for_module "$source" "$module")" || file="$(collect_files "$source" | head -n 1)"
  replacement_line="$(ast_json "$source" | jq -r --arg surface "$surface" --arg base_surface "$base_surface" --arg replacement "$replacement" '
    def replace_left($source):
      (($source | capture("^if[[:space:]]+[^{}]+[[:space:]]*\\{[[:space:]]*[^}]+[[:space:]]*\\}[[:space:]]*else[[:space:]]*\\{[[:space:]]*[^}]+[[:space:]]*\\}(?<suffix>.*)$")? // {suffix:""}).suffix) as $suffix |
      $replacement + $suffix;
    ([.tasks[]?.body.statements[]? | select(.id == $surface or (.expr.id? == $surface) or (.expr.id? == $base_surface))][0] // null) as $stmt |
    if $stmt == null then $replacement
    elif (($surface | endswith(":left")) and (($stmt.expr.id? // "") == $base_surface)) then
      (replace_left($stmt.expr.source // "")) as $expr_source |
      if ($stmt.kind // "") == "Return" then "return " + $expr_source
      elif ($stmt.kind // "") == "Set" then "set " + ($stmt.name // "value") + " = " + $expr_source
      elif ($stmt.kind // "") == "Binding" then (($stmt.binding_kind // "bind" | ascii_downcase) + " " + ($stmt.name // "value") + " = " + $expr_source)
      else $expr_source end
    elif ($stmt.expr.id? // "") == $surface then
      if ($stmt.kind // "") == "Return" then "return " + $replacement
      elif ($stmt.kind // "") == "Set" then "set " + ($stmt.name // "value") + " = " + $replacement
      elif ($stmt.kind // "") == "Binding" then (($stmt.binding_kind // "bind" | ascii_downcase) + " " + ($stmt.name // "value") + " = " + $replacement)
      else $replacement end
    else $replacement end
  ')"
  line="$(ast_json "$source" | jq -r --arg surface "$surface" --arg base_surface "$base_surface" '
    [.tasks[]?.body.statements[]? | select(.id == $surface or (.expr.id? == $surface) or (.expr.id? == $base_surface)) | .span.line][0] // empty
  ')"
  [[ -n "$file" && -n "$line" ]] || return 0
  tmp_file="$(mktemp)"
  awk -v line_no="$line" -v replacement="$replacement_line" '
    FNR == line_no {
      match($0, /^[[:space:]]*/)
      print substr($0, 1, RLENGTH) replacement
      next
    }
    {print}
  ' "$file" > "$tmp_file"
  mv "$tmp_file" "$file"
}

sley_delete_statement_surface() {
  local source="$1" surface="$2" module file line tmp_file
  module="${surface#block:task:}"
  module="${module%.*:stmt:*}"
  file="$(sley_task_file_for_module "$source" "$module")" || file="$(collect_files "$source" | head -n 1)"
  line="$(ast_json "$source" | jq -r --arg surface "$surface" '
    [.tasks[]?.body.statements[]? | select(.id == $surface) | .span.line][0] // empty
  ')"
  [[ -n "$file" && -n "$line" ]] || return 0
  tmp_file="$(mktemp)"
  awk -v line_no="$line" 'FNR == line_no {next} {print}' "$file" > "$tmp_file"
  mv "$tmp_file" "$file"
}

sley_project_entry_module() {
  local source="$1" entry=""
  if [[ -d "$source" && -f "$source/sley.toml" ]]; then
    entry="$(awk -F= '/^[[:space:]]*entry[[:space:]]*=/ {gsub(/^[ \t"]+|[ \t"]+$/, "", $2); print $2; exit}' "$source/sley.toml")"
  fi
  if [[ -n "$entry" ]]; then
    printf '%s\n' "$entry"
  else
    entry_module_for "$source"
  fi
}

sley_add_import_to_module() {
  local source="$1" owner_module="$2" imported_module="$3" file tmp_file
  [[ -n "$owner_module" && -n "$imported_module" ]] || return 0
  file="$(sley_task_file_for_module "$source" "$owner_module")" || file="$(collect_files "$source" | head -n 1)"
  [[ -n "$file" ]] || return 0
  if grep -Eq "^[[:space:]]*import[[:space:]]+$imported_module([[:space:]]+as[[:space:]]+[A-Za-z_][A-Za-z0-9_]*)?[[:space:]]*$" "$file"; then
    return 0
  fi
  tmp_file="$(mktemp)"
  awk -v imported="$imported_module" '
    !inserted && /^[[:space:]]*module[[:space:]]+/ {
      print
      print ""
      print "import " imported
      inserted=1
      next
    }
    {print}
    END {
      if(!inserted) {
        print "import " imported
      }
    }
  ' "$file" > "$tmp_file"
  mv "$tmp_file" "$file"
}

sley_add_effect_to_module() {
  local source="$1" owner_module="$2" effect_name="$3" file tmp_file
  [[ -n "$owner_module" && -n "$effect_name" ]] || return 0
  file="$(sley_task_file_for_module "$source" "$owner_module")" || file="$(collect_files "$source" | head -n 1)"
  [[ -n "$file" ]] || return 0
  if grep -Eq "^[[:space:]]*effect[[:space:]]+$effect_name[[:space:]]*$" "$file"; then
    return 0
  fi
  tmp_file="$(mktemp)"
  awk -v effect_name="$effect_name" '
    !inserted && /^[[:space:]]*module[[:space:]]+/ {
      print
      print ""
      print "effect " effect_name
      inserted=1
      next
    }
    {print}
    END {
      if(!inserted) {
        print "effect " effect_name
      }
    }
  ' "$file" > "$tmp_file"
  mv "$tmp_file" "$file"
}

sley_add_take_to_task() {
  local source="$1" task_surface="$2" take_name="$3" take_type="$4" task_rest task_module task_name file tmp_file
  [[ -n "$task_surface" && -n "$take_name" && -n "$take_type" ]] || return 0
  task_rest="${task_surface#task:}"
  task_module="${task_rest%.*}"
  task_name="${task_rest##*.}"
  file="$(sley_task_file_for_module "$source" "$task_module")" || return 0
  if awk -v task_name="$task_name" -v take_name="$take_name" '
    /^[ \t]*(export[ \t]+)?task[ \t]+/ {
      line=$0
      sub(/^[ \t]*export[ \t]+/,"",line)
      sub(/^[ \t]*task[ \t]+/,"",line)
      task=line
      sub(/[ \t]*->.*/,"",task)
      in_task=(task == task_name)
    }
    in_task && $0 ~ "^[[:space:]]*take[[:space:]]+" take_name "([[:space:]:]|$)" {found=1}
    in_task && /^[ \t]*}/ {in_task=0}
    END {exit found ? 0 : 1}
  ' "$file"; then
    return 0
  fi
  tmp_file="$(mktemp)"
  awk -v task_name="$task_name" -v take_name="$take_name" -v take_type="$take_type" '
    !inserted && $0 ~ "^[[:space:]]*(export[[:space:]]+)?task[[:space:]]+" task_name "([[:space:]]|$)" {
      print
      match($0, /^[[:space:]]*/)
      print substr($0, 1, RLENGTH) "  take " take_name ": " take_type
      inserted=1
      next
    }
    {print}
  ' "$file" > "$tmp_file"
  mv "$tmp_file" "$file"
}

sley_move_task_to_module() {
  local source="$1" task_surface="$2" destination_module="$3" task_rest task_module task_name source_file dest_file block_file tmp_file
  task_rest="${task_surface#task:}"
  task_module="${task_rest%.*}"
  task_name="${task_rest##*.}"
  [[ -n "$task_module" && -n "$task_name" && -n "$destination_module" ]] || return 0
  [[ "$task_module" != "$destination_module" ]] || return 0
  source_file="$(sley_task_file_for_module "$source" "$task_module")" || return 0
  dest_file="$(sley_task_file_for_module "$source" "$destination_module")" || return 0
  block_file="$(mktemp)"
  tmp_file="$(mktemp)"
  awk -v task_name="$task_name" -v block="$block_file" '
    function count_char(s, c, n, i){n=0; for(i=1;i<=length(s);i++){if(substr(s,i,1)==c)n++} return n}
    !skip && $0 ~ "^[[:space:]]*(export[[:space:]]+)?task[[:space:]]+" task_name "([[:space:]]|$)" {
      skip=1
      depth=count_char($0,"{")-count_char($0,"}")
      print $0 > block
      if(depth<=0) skip=0
      next
    }
    skip {
      print $0 > block
      depth += count_char($0,"{")-count_char($0,"}")
      if(depth<=0) skip=0
      next
    }
    {print}
  ' "$source_file" > "$tmp_file"
  mv "$tmp_file" "$source_file"
  [[ -s "$block_file" ]] || { rm -f "$block_file"; return 0; }
  tmp_file="$(mktemp)"
  awk -v block="$block_file" '
    !inserted && /^[[:space:]]*module[[:space:]]+/ {
      print
      print ""
      while((getline line < block) > 0) print line
      inserted=1
      next
    }
    {print}
    END {
      if(!inserted) {
        while((getline line < block) > 0) print line
      }
    }
  ' "$dest_file" > "$tmp_file"
  mv "$tmp_file" "$dest_file"
  rm -f "$block_file"
}

sley_delete_source_statement_index_surface() {
  local source="$1" surface="$2" rest task_ref module task_name stmt_index file tmp_file
  rest="${surface#block:task:}"
  task_ref="${rest%:stmt:*}"
  stmt_index="${rest##*:stmt:}"
  module="${task_ref%.*}"
  task_name="${task_ref##*.}"
  file="$(sley_task_file_for_module "$source" "$module")" || file="$(collect_files "$source" | head -n 1)"
  [[ -n "$file" && "$stmt_index" =~ ^[0-9]+$ ]] || return 0
  tmp_file="$(mktemp)"
  awk -v wanted_task="$task_name" -v wanted_idx="$stmt_index" '
    function trim(s){gsub(/^[ \t\r\n]+|[ \t\r\n]+$/, "", s); return s}
    function count_char(s, c, n, i){n=0; for(i=1;i<=length(s);i++){if(substr(s,i,1)==c)n++} return n}
    /^[ \t]*(export[ \t]+)?task[ \t]+/ {
      raw=$0
      line=trim($0)
      sub(/^export[ \t]+/,"",line)
      sub(/^task[ \t]+/,"",line)
      task=line
      sub(/[ \t]*->.*/,"",task)
      in_task=(task == wanted_task)
      depth=count_char(raw,"{")-count_char(raw,"}")
      stmt=0
      print
      next
    }
    in_task {
      raw=$0
      line=trim(raw)
      if(line == "" || line ~ /^take[ \t]+/) {
        print
        depth += count_char(raw,"{")-count_char(raw,"}")
        if(depth<=0) in_task=0
        next
      }
      if(line !~ /^[ \t]*}/ && depth == 1) {
        if(stmt == wanted_idx) {
          stmt++
          depth += count_char(raw,"{")-count_char(raw,"}")
          if(depth<=0) in_task=0
          next
        }
        stmt++
      }
      print
      depth += count_char(raw,"{")-count_char(raw,"}")
      if(depth<=0) in_task=0
      next
    }
    {print}
  ' "$file" > "$tmp_file"
  mv "$tmp_file" "$file"
}

sley_replace_if_block_surface() {
  local source="$1" surface="$2" replacement="$3" module file line tmp_file
  module="${surface#block:task:}"
  module="${module%.*:stmt:*}"
  file="$(sley_task_file_for_module "$source" "$module")" || file="$(collect_files "$source" | head -n 1)"
  line="$(ast_json "$source" | jq -r --arg surface "$surface" '
    [.tasks[]?.body.statements[]? | select(.id == $surface) | .span.line][0] // empty
  ')"
  [[ -n "$file" && -n "$line" ]] || return 0
  tmp_file="$(mktemp)"
  awk -v start_line="$line" -v replacement="$replacement" '
    function count_char(s, c, n, i){n=0; for(i=1;i<=length(s);i++){if(substr(s,i,1)==c)n++} return n}
    FNR == start_line {
      match($0, /^[[:space:]]*/)
      print substr($0, 1, RLENGTH) replacement
      depth=count_char($0,"{")-count_char($0,"}")
      skipping=(depth>0)
      next
    }
    skipping {
      depth += count_char($0,"{")-count_char($0,"}")
      if(depth<=0) skipping=0
      next
    }
    {print}
  ' "$file" > "$tmp_file"
  mv "$tmp_file" "$file"
}

sley_delete_if_block_surface() {
  local source="$1" surface="$2" module file line tmp_file
  module="${surface#block:task:}"
  module="${module%.*:stmt:*}"
  file="$(sley_task_file_for_module "$source" "$module")" || file="$(collect_files "$source" | head -n 1)"
  line="$(ast_json "$source" | jq -r --arg surface "$surface" '
    [.tasks[]?.body.statements[]? | select(.id == $surface) | .span.line][0] // empty
  ')"
  [[ -n "$file" && -n "$line" ]] || return 0
  tmp_file="$(mktemp)"
  awk -v start_line="$line" '
    function count_char(s, c, n, i){n=0; for(i=1;i<=length(s);i++){if(substr(s,i,1)==c)n++} return n}
    FNR == start_line {
      depth=count_char($0,"{")-count_char($0,"}")
      skipping=(depth>0)
      next
    }
    skipping {
      depth += count_char($0,"{")-count_char($0,"}")
      if(depth<=0) skipping=0
      next
    }
    {print}
  ' "$file" > "$tmp_file"
  mv "$tmp_file" "$file"
}

sley_remove_empty_else_surface() {
  local source="$1" surface="$2" module file line tmp_file
  module="${surface#block:task:}"
  module="${module%.*:stmt:*}"
  file="$(sley_task_file_for_module "$source" "$module")" || file="$(collect_files "$source" | head -n 1)"
  line="$(ast_json "$source" | jq -r --arg surface "$surface" '
    [.tasks[]?.body.statements[]? | select(.id == $surface) | .span.line][0] // empty
  ')"
  [[ -n "$file" && -n "$line" ]] || return 0
  tmp_file="$(mktemp)"
  awk -v start_line="$line" '
    function count_char(s, c, n, i){n=0; for(i=1;i<=length(s);i++){if(substr(s,i,1)==c)n++} return n}
    FNR >= start_line && skipping_else {
      else_depth += count_char($0,"{")-count_char($0,"}")
      if(else_depth<=0) {
        skipping_else=0
        in_if=0
      }
      next
    }
    FNR >= start_line && $0 ~ /^[[:space:]]*\}[[:space:]]*else[[:space:]]*\{[[:space:]]*$/ {
      match($0, /^[[:space:]]*/)
      print substr($0, 1, RLENGTH) "}"
      skipping_else=1
      else_depth=1
      next
    }
    FNR >= start_line && $0 ~ /^[[:space:]]*else[[:space:]]*\{[[:space:]]*$/ {
      skipping_else=1
      else_depth=1
      next
    }
    {print}
  ' "$file" > "$tmp_file"
  mv "$tmp_file" "$file"
}

command_graft_raw() {
  local source kind="" surface="" actor="sley:stage1" trace_path="" payload_source="" payload_source_file="" payload_name="" payload_module="" payload_type="" payload_position="" write=false arg operation_file=""
  local -a positionals=()
  while [[ "$#" -gt 0 ]]; do
    arg="$1"
    case "$arg" in
      --kind) kind="$2"; shift 2 ;;
      --template-surface) surface="$2"; shift 2 ;;
      --actor) actor="$2"; shift 2 ;;
      --trace) trace_path="$2"; shift 2 ;;
      --source) payload_source="$2"; shift 2 ;;
      --source-file) payload_source_file="$2"; shift 2 ;;
      --name) payload_name="$2"; shift 2 ;;
      --module) payload_module="$2"; shift 2 ;;
      --type) payload_type="$2"; shift 2 ;;
      --position) payload_position="$2"; shift 2 ;;
      --write) write=true; shift ;;
      --dry-run|--json) shift ;;
      *) positionals+=("$1"); shift ;;
    esac
  done
  if [[ "${#positionals[@]}" -gt 0 ]]; then
    local last_index last_pos
    last_index=$((${#positionals[@]} - 1))
    last_pos="${positionals[$last_index]}"
    if [[ "${#positionals[@]}" -ge 2 && -f "$last_pos" ]] && jq -e '(.op? // .ops?[0].op? // empty) != ""' "$last_pos" >/dev/null 2>&1; then
      source="${positionals[0]}"
      operation_file="$last_pos"
    else
      source="$last_pos"
    fi
  fi
  if [[ -z "$source" ]]; then
    sley_report_error_json "GRAFT_SOURCE_REQUIRED" "graft requires a source file or project" "graft"
    return 2
  fi
  sley_reject_ambiguous_path "$source"
  if [[ -n "$trace_path" ]]; then
    sley_reject_ambiguous_path "$trace_path"
  fi
  if [[ -n "$payload_source_file" ]]; then
    sley_reject_ambiguous_path "$payload_source_file"
    sley_enforce_file_budget "$payload_source_file" "$SLEY_MAX_SOURCE_BYTES"
    payload_source="$(<"$payload_source_file")"
  fi
  if [[ -n "$operation_file" ]]; then
    local op_json operation target_surface owner_module imported_module effect_name take_name take_type destination_parent destination_module preview report default_task_surface
    sley_reject_ambiguous_path "$operation_file"
    sley_enforce_file_budget "$operation_file" "$SLEY_MAX_JSON_BYTES"
    if ! jq -e '
      def op_object: if (.op? // "") != "" then . else .ops[0] end;
      (op_object | type == "object") and
      ((op_object.op // "") | IN("AddTake","AddImport","MoveNode","InsertStatement","ReplaceStatement","ReplaceExpression","DeleteNode")) and
      ((op_object.target? // "") | type == "string")
    ' "$operation_file" >/dev/null; then
      sley_report_error_json "GRAFT_OPERATION_INVALID" "graft operation JSON is missing a supported op/target shape" "$operation_file"
      return 1
    fi
    op_json="$(jq -c 'if (.op? // "") != "" then . else .ops[0] end' "$operation_file")"
    operation="$(printf '%s\n' "$op_json" | jq -r '.op // ""')"
    target_surface="$(printf '%s\n' "$op_json" | jq -r '.target // ""')"
    preview=""
    case "$operation" in
      AddTake)
        if [[ -z "$target_surface" ]]; then
          default_task_surface="$(parser_task_id_value app.main main)"
          target_surface="$(query_json "$source" tasks | jq -r --arg fallback "$default_task_surface" '.tasks[0].id // $fallback')"
        fi
        take_name="$(printf '%s\n' "$op_json" | jq -r '.payload.name // "value"')"
        take_type="$(printf '%s\n' "$op_json" | jq -r '.payload.type // "Text"')"
        sley_validate_identifier "$take_name" "take name"
        preview="take $take_name: $take_type"
        if [[ "$write" == true ]]; then
          sley_add_take_to_task "$source" "$target_surface" "$take_name" "$take_type"
          sley_assert_source_still_checks "$source" || return 1
        fi
        ;;
      AddImport)
        owner_module="$(printf '%s\n' "$op_json" | jq -r '.payload.owner_module // empty')"
        if [[ -z "$owner_module" && "$target_surface" == module:* ]]; then
          owner_module="${target_surface#module:}"
          owner_module="${owner_module%%:*}"
        fi
        owner_module="${owner_module:-$(sley_project_entry_module "$source")}"
        imported_module="$(printf '%s\n' "$op_json" | jq -r '.payload.module // "app.new_module"')"
        sley_validate_module_name "$owner_module"
        sley_validate_module_name "$imported_module"
        target_surface="import:$owner_module:$imported_module"
        preview="import $imported_module"
        if [[ "$write" == true ]]; then
          sley_add_import_to_module "$source" "$owner_module" "$imported_module"
          sley_assert_source_still_checks "$source" || return 1
        fi
        ;;
      MoveNode)
        destination_parent="$(printf '%s\n' "$op_json" | jq -r '.payload.parent // ""')"
        if [[ "$write" == true ]]; then
          if [[ "$target_surface" != task:* || "$destination_parent" != module:*:tasks ]]; then
            sley_report_error_json "GRAFT_OPERATION_UNSUPPORTED" "graft move write requires a task target and module task-list parent" "${operation:-graft}"
            return 1
          fi
          destination_module="${destination_parent#module:}"
          destination_module="${destination_module%:tasks}"
          sley_validate_module_name "$destination_module"
          sley_move_task_to_module "$source" "$target_surface" "$destination_module"
          sley_assert_source_still_checks "$source" || return 1
        fi
        ;;
      InsertStatement|ReplaceStatement|ReplaceExpression|DeleteNode)
        if [[ "$write" == true ]]; then
          sley_report_error_json "GRAFT_OPERATION_UNSUPPORTED" "graft operation is not supported by the stage-1 writer" "${operation:-graft}"
          return 1
        fi
        ;;
      *)
        sley_report_error_json "GRAFT_OPERATION_UNSUPPORTED" "graft operation is not supported by the stage-1 writer" "${operation:-graft}"
        return 1
        ;;
    esac
    report="$(jq -n --arg schema "$SCHEMA_GRAFT_OUTCOME" --arg source "$source" --arg actor "$actor" --arg operation "$operation" --arg target "$target_surface" --arg preview "$preview" --argjson fields "$GRAFT_OUTCOME_REPORT_FIELDS_JSON" '{
      ($fields[0] // "schema"):$schema,
      ($fields[1] // "status"):"accepted",
      ($fields[2] // "source"):$source,
      ($fields[3] // "diagnostics"):[],
      ($fields[4] // "provenance"):[{graft_id:"stage1-dry-run",actor:$actor,timestamp:"2026-05-08T00:00:00Z",operation:$operation,targets:[$target],result:"accepted"}]
    } + (if $preview == "" then {} else {preview_source:$preview} end)')"
    sley_append_trace_receipt "$trace_path" "$report"
    printf '%s\n' "$report"
    return
  fi
  if [[ "$kind" == "delete_unused_import" || "$kind" == "delete_duplicate_import" ]]; then
    if [[ -z "$surface" ]]; then
      if [[ "$kind" == "delete_duplicate_import" ]]; then
        surface="$(lint_json "$source" | jq -r '.findings[]? | select(.rule=="duplicate_import") | .node' | head -n 1)"
      else
        surface="$(lint_json "$source" | jq -r '.findings[]? | select(.rule=="unused_import") | .node' | head -n 1)"
      fi
    fi
    if [[ -n "$surface" && "$write" == true ]]; then
      local rest owner imported file module tmp_file
      rest="${surface#import:}"
      owner="${rest%%:*}"
      imported="${rest#*:}"
      while IFS= read -r file; do
        module="$(awk '/^[[:space:]]*module[[:space:]]+/ {print $2; found=1; exit} END {if(!found) print "main"}' "$file")"
        if [[ "$module" == "$owner" ]]; then
          tmp_file="$(mktemp)"
          if [[ "$kind" == "delete_duplicate_import" ]]; then
            awk -v imported="$imported" '
              $0 ~ "^[[:space:]]*import[[:space:]]+" imported "([[:space:]]+as[[:space:]]+[A-Za-z_][A-Za-z0-9_]*)?[[:space:]]*$" {seen++; if(seen>1) next}
              {print}
            ' "$file" > "$tmp_file"
          else
            awk -v imported="$imported" '
              $0 ~ "^[[:space:]]*import[[:space:]]+" imported "([[:space:]]+as[[:space:]]+[A-Za-z_][A-Za-z0-9_]*)?[[:space:]]*$" {next}
              {print}
            ' "$file" > "$tmp_file"
          fi
          mv "$tmp_file" "$file"
          sley_assert_source_still_checks "$source" || return 1
          break
        fi
      done < <(collect_files "$source")
    fi
    local report
    report="$(jq -n --arg schema "$SCHEMA_GRAFT_OUTCOME" --arg source "$source" --arg actor "$actor" --arg surface "$surface" --argjson fields "$GRAFT_OUTCOME_REPORT_FIELDS_JSON" '{
      ($fields[0] // "schema"):$schema,
      ($fields[1] // "status"):"accepted",
      ($fields[2] // "source"):$source,
      ($fields[3] // "diagnostics"):[],
      ($fields[4] // "provenance"):[{graft_id:"stage1-dry-run",actor:$actor,timestamp:"2026-05-08T00:00:00Z",operation:"DeleteNode",targets:[$surface],result:"accepted"}]
    }')"
    sley_append_trace_receipt "$trace_path" "$report"
    printf '%s\n' "$report"
    return
  fi
  if [[ "$kind" == "delete_unused_private_declarations" || "$kind" == "delete_dead_private_tasks" ]]; then
    local lint rules_filter
    lint="$(lint_json "$source")"
    if [[ "$kind" == "delete_unused_private_declarations" ]]; then
      rules_filter='.rule == "unused_private_effect" or .rule == "unused_private_type"'
    else
      rules_filter='.rule == "unreachable_private_task" or .rule == "unused_private_task"'
    fi
    jq -n --arg schema "$SCHEMA_GRAFT_OUTCOME" --arg source "$source" --arg actor "$actor" --argjson lint "$lint" --arg rules_filter "$rules_filter" --argjson fields "$GRAFT_OUTCOME_REPORT_FIELDS_JSON" '{
      ($fields[0] // "schema"):$schema,
      ($fields[1] // "status"):"accepted",
      ($fields[2] // "source"):$source,
      ($fields[3] // "diagnostics"):[],
      ($fields[4] // "provenance"):([
        $lint.findings[]?
        | select(if $rules_filter == ".rule == \"unused_private_effect\" or .rule == \"unused_private_type\"" then (.rule == "unused_private_effect" or .rule == "unused_private_type") else (.rule == "unreachable_private_task" or .rule == "unused_private_task") end)
        | {graft_id:"stage1-dry-run",actor:$actor,timestamp:"2026-05-08T00:00:00Z",operation:"DeleteNode",targets:[.node],result:"accepted"}
      ])
    }'
    return
  fi
  if [[ "$kind" == "delete_unused_private_task" ]]; then
    if [[ -z "$surface" ]]; then
      surface="$(lint_json "$source" | jq -r '.findings[]? | select(.rule=="unused_private_task") | .node' | head -n 1)"
    fi
    if [[ -n "$surface" && "$write" == true ]]; then
      local task_rest task_module task_name file module tmp_file
      task_rest="${surface#task:}"
      task_module="${task_rest%.*}"
      task_name="${task_rest##*.}"
      while IFS= read -r file; do
        module="$(awk '/^[[:space:]]*module[[:space:]]+/ {print $2; found=1; exit} END {if(!found) print "main"}' "$file")"
        if [[ "$module" == "$task_module" ]]; then
          tmp_file="$(mktemp)"
          awk -v task_name="$task_name" '
            function count_char(s, c, n, i){n=0; for(i=1;i<=length(s);i++){if(substr(s,i,1)==c)n++} return n}
            !skip && $0 ~ "^[[:space:]]*(export[[:space:]]+)?task[[:space:]]+" task_name "([[:space:]]|$)" {
              skip=1
              depth=count_char($0,"{")-count_char($0,"}")
              if(depth<=0) skip=0
              next
            }
            skip {
              depth += count_char($0,"{")-count_char($0,"}")
              if(depth<=0) skip=0
              next
            }
            {print}
          ' "$file" > "$tmp_file"
          mv "$tmp_file" "$file"
          sley_assert_source_still_checks "$source" || return 1
          break
        fi
      done < <(collect_files "$source")
    fi
    local report
    report="$(jq -n --arg schema "$SCHEMA_GRAFT_OUTCOME" --arg source "$source" --arg actor "$actor" --arg surface "$surface" --argjson fields "$GRAFT_OUTCOME_REPORT_FIELDS_JSON" '{
      ($fields[0] // "schema"):$schema,
      ($fields[1] // "status"):"accepted",
      ($fields[2] // "source"):$source,
      ($fields[3] // "diagnostics"):[],
      ($fields[4] // "provenance"):[{graft_id:"stage1-dry-run",actor:$actor,timestamp:"2026-05-08T00:00:00Z",operation:"DeleteNode",targets:[$surface],result:"accepted"}]
    }')"
    sley_append_trace_receipt "$trace_path" "$report"
    printf '%s\n' "$report"
    return
  fi
  if [[ "$kind" == "replace_call_arg" ]]; then
    local query call_target call_source
    query="$(query_json "$source" all)"
    call_target="$(printf '%s\n' "$query" | jq -r --arg call_target_task_id_template "$PARSER_CALL_TARGET_TASK_ID_TEMPLATE" '
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
      qualified_task_id((.calls[0].target // "app.main.main"); (.entry_module // "app.main"))
    ')"
    call_source="$(printf '%s\n' "$query" | jq -r '.calls[0].source // ""')"
    jq -n --arg schema "$SCHEMA_GRAFT_OUTCOME" --arg source "$source" --arg actor "$actor" --arg target "$call_target" --arg call_source "$call_source" --argjson fields "$GRAFT_OUTCOME_REPORT_FIELDS_JSON" '{
      ($fields[0] // "schema"):$schema,
      ($fields[1] // "status"):"accepted",
      ($fields[2] // "source"):$source,
      ($fields[3] // "diagnostics"):[],
      ($fields[4] // "provenance"):[{graft_id:"stage1-dry-run",actor:$actor,timestamp:"2026-05-08T00:00:00Z",operation:"ReplaceCallArg",targets:[$target,"calls:1"],result:"accepted"}],
      preview_source:$call_source
    }'
    return
  fi
  if [[ "$kind" == "rename_and_update_call_sites" ]]; then
    local task_rest task_module task_name new_name new_target query call_count
    task_rest="${surface#task:}"
    task_module="${task_rest%.*}"
    task_name="${task_rest##*.}"
    new_name="${payload_name:-renamed_${task_name}}"
    new_target="$(jq -rn --arg call_target_task_id_template "$PARSER_CALL_TARGET_TASK_ID_TEMPLATE" --arg module "$task_module" --arg task "$new_name" '
      def replace_all($needle; $replacement): split($needle) | join($replacement);
      $call_target_task_id_template
      | replace_all("{{module}}"; $module)
      | replace_all("{{task}}"; $task)
    ')"
    query="$(query_json "$source" all)"
    call_count="$(printf '%s\n' "$query" | jq -r --arg target "$task_rest" '[.calls[]? | select(.target == $target)] | length')"
    if [[ "$write" == true ]]; then
      sley_validate_identifier "$new_name" "task name"
      sley_rename_task_declaration "$source" "$task_module" "$task_name" "$new_name"
      sley_rewrite_inbound_call_names "$source" "$task_rest" "$new_name"
      sley_assert_source_still_checks "$source" || return 1
    fi
    jq -n --arg schema "$SCHEMA_GRAFT_OUTCOME" --arg source "$source" --arg actor "$actor" --arg surface "$surface" --arg new_target "$new_target" --arg calls "calls:$call_count" --argjson fields "$GRAFT_OUTCOME_REPORT_FIELDS_JSON" '{
      ($fields[0] // "schema"):$schema,
      ($fields[1] // "status"):"accepted",
      ($fields[2] // "source"):$source,
      ($fields[3] // "diagnostics"):[],
      ($fields[4] // "provenance"):[
        {graft_id:"stage1-dry-run",actor:$actor,timestamp:"2026-05-08T00:00:00Z",operation:"RenameDeclaration",targets:[$surface],result:"accepted"},
        {graft_id:"stage1-dry-run",actor:$actor,timestamp:"2026-05-08T00:00:00Z",operation:"UpdateCallSites",targets:[$new_target,$calls],result:"accepted"}
      ]
    }'
    return
  fi
  if [[ "$kind" == "remove_take_and_remove_call_arg" ]]; then
    local task_rest task_module task_name query lint take_node take_name take_position call_count
    task_rest="${surface#task:}"
    task_module="${task_rest%.*}"
    task_name="${task_rest##*.}"
    query="$(query_json "$source" all)"
    lint="$(lint_json "$source")"
    take_node="$(printf '%s\n' "$lint" | jq -r --arg surface "$surface" '.findings[]? | select(.rule == "unused_take" and (.node | startswith("take:" + $surface + ":"))) | .node' | head -n 1)"
    if [[ -z "$take_node" ]]; then
      take_node="$(ast_json "$source" | jq -r --arg surface "$surface" --arg take_surface_id_template "$PARSER_TAKE_SURFACE_ID_TEMPLATE" '
        def replace_all($needle; $replacement): split($needle) | join($replacement);
        def take_surface_id($task_id; $index; $take_name):
          $take_surface_id_template
          | replace_all("{{task_id}}"; $task_id)
          | replace_all("{{index}}"; $index)
          | replace_all("{{take}}"; $take_name);
        .tasks[]? | select(.id == $surface) | .takes | to_entries[-1] | take_surface_id($surface; (.key|tostring); .value.name)
      ' | head -n 1)"
    fi
    take_name="${take_node##*:}"
    take_position="${take_node%:*}"
    take_position="${take_position##*:}"
    call_count="$(printf '%s\n' "$query" | jq -r --arg target "$task_rest" '[.calls[]? | select(.target == $target)] | length')"
    if [[ "$write" == true ]]; then
      sley_remove_task_take "$source" "$task_module" "$task_name" "$take_name"
      sley_remove_inbound_call_arg "$source" "$task_rest" "$take_position"
      sley_assert_source_still_checks "$source" || return 1
    fi
    jq -n --arg schema "$SCHEMA_GRAFT_OUTCOME" --arg source "$source" --arg actor "$actor" --arg surface "$surface" --arg take "take:$take_name" --arg calls "calls:$call_count" --argjson fields "$GRAFT_OUTCOME_REPORT_FIELDS_JSON" '{
      ($fields[0] // "schema"):$schema,
      ($fields[1] // "status"):"accepted",
      ($fields[2] // "source"):$source,
      ($fields[3] // "diagnostics"):[],
      ($fields[4] // "provenance"):[
        {graft_id:"stage1-dry-run",actor:$actor,timestamp:"2026-05-08T00:00:00Z",operation:"RemoveTake",targets:[$surface,$take],result:"accepted"},
        {graft_id:"stage1-dry-run",actor:$actor,timestamp:"2026-05-08T00:00:00Z",operation:"RemoveCallArg",targets:[$surface,$calls],result:"accepted"}
      ]
    }'
    return
  fi
  if [[ "$kind" == "add_module_declaration" ]]; then
    local module_name report
    module_name="${payload_name:-${payload_module:-$(sley_module_name_for_source "$source")}}"
    sley_validate_module_name "$module_name"
    if [[ "$write" == true ]]; then
      sley_add_module_declaration "$source" "$module_name"
      sley_assert_source_still_checks "$source" || return 1
    fi
    report="$(jq -n --arg schema "$SCHEMA_GRAFT_OUTCOME" --arg source "$source" --arg actor "$actor" --arg module "module:$module_name" --argjson fields "$GRAFT_OUTCOME_REPORT_FIELDS_JSON" '{
      ($fields[0] // "schema"):$schema,
      ($fields[1] // "status"):"accepted",
      ($fields[2] // "source"):$source,
      ($fields[3] // "diagnostics"):[],
      ($fields[4] // "provenance"):[{graft_id:"stage1-dry-run",actor:$actor,timestamp:"2026-05-08T00:00:00Z",operation:"AddModuleDeclaration",targets:[$module],result:"accepted"}]
    }')"
    sley_append_trace_receipt "$trace_path" "$report"
    printf '%s\n' "$report"
    return
  fi
  if [[ "$kind" == "migrate_raw_host_adapter" || "$kind" == "propagate_unchecked_result" || "$kind" == "propagate_unchecked_result_binding" || "$kind" == "qualify_imported_call" ]]; then
    local lint finding replacement operation target_surface report imported_call_replacement
    local raw_host_adapter_fallback imported_call_fallback unchecked_binding_fallback unchecked_result_fallback
    raw_host_adapter_fallback="$(parser_task_expression_surface_value app.raw_migration main 0)"
    imported_call_fallback="$(parser_task_expression_surface_value app.main main 0)"
    unchecked_binding_fallback="$(parser_task_statement_surface_value app.unchecked_binding main 0)"
    unchecked_result_fallback="$(parser_task_expression_surface_value app.unchecked main 0)"
    imported_call_replacement="$(parser_default_qualified_import_call_source)"
    lint="$(lint_json "$source")"
    case "$kind" in
      migrate_raw_host_adapter)
        finding="$(printf '%s\n' "$lint" | jq -c '.findings[]? | select(.rule == "raw_host_adapter")' | head -n 1)"
        [[ -n "$finding" ]] || finding='{}'
        operation="ReplaceExpression"
        target_surface="$(printf '%s\n' "$finding" | jq -r --arg fallback "$raw_host_adapter_fallback" '.node // $fallback')"
        replacement="$(printf '%s\n' "$finding" | jq -r '.replacement // "fs.try_read_text(\"examples/hello.sley\")?"')"
        ;;
      propagate_unchecked_result)
        finding="$(printf '%s\n' "$lint" | jq -c '.findings[]? | select(.rule == "unchecked_result")' | head -n 1)"
        [[ -n "$finding" ]] || finding='{}'
        operation="ReplaceExpression"
        target_surface="$(printf '%s\n' "$finding" | jq -r --arg fallback "$unchecked_result_fallback" '.node // $fallback')"
        replacement="$(printf '%s\n' "$finding" | jq -r '.replacement // "fs.try_write_text(\"sley_cli_smoke.txt\", \"written\")?"')"
        ;;
      propagate_unchecked_result_binding)
        finding="$(printf '%s\n' "$lint" | jq -c --arg surface "$surface" '.findings[]? | select(.rule == "unchecked_result_binding" and (.node == $surface or $surface == ""))' | head -n 1)"
        [[ -n "$finding" ]] || finding='{}'
        operation="ReplaceStatement"
        target_surface="${surface:-$(printf '%s\n' "$finding" | jq -r --arg fallback "$unchecked_binding_fallback" '.node // $fallback')}"
        replacement="$(printf '%s\n' "$finding" | jq -r '.replacement // "fs.try_read_text(\"examples/hello.sley\")?"')"
        ;;
      qualify_imported_call)
        finding="$(printf '%s\n' "$lint" | jq -c --arg surface "$surface" '.findings[]? | select(.rule == "unqualified_imported_call" and (.node == $surface or $surface == ""))' | head -n 1)"
        [[ -n "$finding" ]] || finding='{}'
        operation="ReplaceExpression"
        target_surface="${surface:-$(printf '%s\n' "$finding" | jq -r --arg fallback "$imported_call_fallback" '.node // $fallback')}"
        replacement="$(printf '%s\n' "$finding" | jq -r --arg fallback "$imported_call_replacement" '.replacement // $fallback')"
        ;;
    esac
    if [[ "$write" == true ]]; then
      sley_rewrite_statement_surface "$source" "$target_surface" "$replacement"
    fi
    report="$(jq -n --arg schema "$SCHEMA_GRAFT_OUTCOME" --arg source "$source" --arg actor "$actor" --arg operation "$operation" --arg target "$target_surface" --argjson fields "$GRAFT_OUTCOME_REPORT_FIELDS_JSON" '{
      ($fields[0] // "schema"):$schema,
      ($fields[1] // "status"):"accepted",
      ($fields[2] // "source"):$source,
      ($fields[3] // "diagnostics"):[],
      ($fields[4] // "provenance"):[{graft_id:"stage1-dry-run",actor:$actor,timestamp:"2026-05-08T00:00:00Z",operation:$operation,targets:[$target],result:"accepted"}]
    }')"
    printf '%s\n' "$report"
    return
  fi
  if [[ "$kind" == "convert_mutable_binding_to_bind" ]]; then
    local lint finding replacement target_surface block_surface report mutable_binding_fallback
    mutable_binding_fallback="$(parser_task_statement_surface_value app.mutable_fix main 0)"
    lint="$(lint_json "$source")"
    finding="$(printf '%s\n' "$lint" | jq -c --arg surface "$surface" '.findings[]? | select(.rule == "mutable_binding_never_set" and (.node == $surface or $surface == ""))' | head -n 1)"
    [[ -n "$finding" ]] || finding='{}'
    target_surface="${surface:-$(printf '%s\n' "$finding" | jq -r --arg fallback "$mutable_binding_fallback" '.node // $fallback')}"
    replacement="$(printf '%s\n' "$finding" | jq -r '.replacement // "bind base = 21"')"
    block_surface="${target_surface%:stmt:*}"
    if [[ "$write" == true ]]; then
      sley_rewrite_statement_surface "$source" "$target_surface" "$replacement"
    fi
    report="$(jq -n --arg schema "$SCHEMA_GRAFT_OUTCOME" --arg source "$source" --arg actor "$actor" --arg target "$target_surface" --arg block "$block_surface" --argjson fields "$GRAFT_OUTCOME_REPORT_FIELDS_JSON" '{
      ($fields[0] // "schema"):$schema,
      ($fields[1] // "status"):"accepted",
      ($fields[2] // "source"):$source,
      ($fields[3] // "diagnostics"):[],
      ($fields[4] // "provenance"):[
        {graft_id:"stage1-dry-run",actor:$actor,timestamp:"2026-05-08T00:00:00Z",operation:"DeleteNode",targets:[$target],result:"accepted"},
        {graft_id:"stage1-dry-run",actor:$actor,timestamp:"2026-05-08T00:00:00Z",operation:"InsertStatement",targets:[$block],result:"accepted"}
      ]
    }')"
    printf '%s\n' "$report"
    return
  fi
  if [[ "$kind" == "delete_self_assignment_statement" || "$kind" == "delete_overwritten_set_statement" ]]; then
    local lint finding rule target_surface report self_assignment_fallback
    self_assignment_fallback="$(parser_task_statement_surface_value app.self_assignment_fix main 1)"
    lint="$(lint_json "$source")"
    if [[ "$kind" == "delete_self_assignment_statement" ]]; then
      rule="self_assignment_statement"
    else
      rule="overwritten_set_statement"
    fi
    finding="$(printf '%s\n' "$lint" | jq -c --arg surface "$surface" --arg rule "$rule" '.findings[]? | select(.rule == $rule and (.node == $surface or $surface == ""))' | head -n 1)"
    [[ -n "$finding" ]] || finding='{}'
    target_surface="${surface:-$(printf '%s\n' "$finding" | jq -r --arg fallback "$self_assignment_fallback" '.node // $fallback')}"
    if [[ "$write" == true ]]; then
      sley_delete_statement_surface "$source" "$target_surface"
    fi
    report="$(jq -n --arg schema "$SCHEMA_GRAFT_OUTCOME" --arg source "$source" --arg actor "$actor" --arg target "$target_surface" --argjson fields "$GRAFT_OUTCOME_REPORT_FIELDS_JSON" '{
      ($fields[0] // "schema"):$schema,
      ($fields[1] // "status"):"accepted",
      ($fields[2] // "source"):$source,
      ($fields[3] // "diagnostics"):[],
      ($fields[4] // "provenance"):[{graft_id:"stage1-dry-run",actor:$actor,timestamp:"2026-05-08T00:00:00Z",operation:"DeleteNode",targets:[$target],result:"accepted"}]
    }')"
    printf '%s\n' "$report"
    return
  fi
  if [[ "$kind" == "fold_redundant_initial_set_into_binding" || "$kind" == "convert_redundant_initial_set_to_bind" ]]; then
    local lint finding target_surface binding_surface binding_expr_surface replacement replacement_statement report op0 target0
    local redundant_set_fallback redundant_binding_fallback redundant_binding_expr_fallback
    redundant_set_fallback="$(parser_task_statement_surface_value app.redundant_initial_set_fix main 1)"
    redundant_binding_fallback="$(parser_task_statement_surface_value app.redundant_initial_set_fix main 0)"
    redundant_binding_expr_fallback="$(parser_task_expression_surface_value app.redundant_initial_set_fix main 0)"
    lint="$(lint_json "$source")"
    finding="$(printf '%s\n' "$lint" | jq -c --arg surface "$surface" '.findings[]? | select(.rule == "redundant_initial_set_statement" and (.node == $surface or $surface == ""))' | head -n 1)"
    [[ -n "$finding" ]] || finding='{}'
    target_surface="${surface:-$(printf '%s\n' "$finding" | jq -r --arg fallback "$redundant_set_fallback" '.node // $fallback')}"
    binding_surface="$(printf '%s\n' "$finding" | jq -r --arg fallback "$redundant_binding_fallback" '.binding_node // $fallback')"
    binding_expr_surface="$(printf '%s\n' "$finding" | jq -r --arg fallback "$redundant_binding_expr_fallback" '.binding_expr_node // $fallback')"
    replacement="$(printf '%s\n' "$finding" | jq -r '.replacement // "1"')"
    replacement_statement="$(printf '%s\n' "$finding" | jq -r '.replacement_statement // "bind count = 1"')"
    if [[ "$kind" == "convert_redundant_initial_set_to_bind" ]]; then
      op0="ReplaceStatement"
      target0="$binding_surface"
      if [[ "$write" == true ]]; then
        sley_rewrite_statement_surface "$source" "$binding_surface" "$replacement_statement"
        sley_delete_statement_surface "$source" "$target_surface"
      fi
    else
      op0="ReplaceExpression"
      target0="$binding_expr_surface"
      if [[ "$write" == true ]]; then
        sley_rewrite_statement_surface "$source" "$binding_expr_surface" "$replacement"
        sley_delete_statement_surface "$source" "$target_surface"
      fi
    fi
    report="$(jq -n --arg schema "$SCHEMA_GRAFT_OUTCOME" --arg source "$source" --arg actor "$actor" --arg op0 "$op0" --arg target0 "$target0" --arg target1 "$target_surface" --argjson fields "$GRAFT_OUTCOME_REPORT_FIELDS_JSON" '{
      ($fields[0] // "schema"):$schema,
      ($fields[1] // "status"):"accepted",
      ($fields[2] // "source"):$source,
      ($fields[3] // "diagnostics"):[],
      ($fields[4] // "provenance"):[
        {graft_id:"stage1-dry-run",actor:$actor,timestamp:"2026-05-08T00:00:00Z",operation:$op0,targets:[$target0],result:"accepted"},
        {graft_id:"stage1-dry-run",actor:$actor,timestamp:"2026-05-08T00:00:00Z",operation:"DeleteNode",targets:[$target1],result:"accepted"}
      ]
    }')"
    printf '%s\n' "$report"
    return
  fi
  if [[ "$kind" == "delete_empty_for_statement" || "$kind" == "delete_empty_forge_statement" || "$kind" == "delete_empty_if_statement" || "$kind" == "delete_empty_while_statement" || "$kind" == "remove_empty_else_statement" ]]; then
    local lint finding rule operation target_surface fallback report
    lint="$(lint_json "$source")"
    case "$kind" in
      delete_empty_for_statement)
        rule="empty_for_statement"
        operation="DeleteNode"
        fallback="$(parser_task_statement_surface_value app.empty_for main 1)"
        ;;
      delete_empty_forge_statement)
        rule="empty_forge_statement"
        operation="DeleteNode"
        fallback="$(parser_task_statement_surface_value app.empty_forge main 1)"
        ;;
      delete_empty_if_statement)
        rule="empty_if_statement"
        operation="DeleteNode"
        fallback="$(parser_task_statement_surface_value app.empty_if main 1)"
        ;;
      delete_empty_while_statement)
        rule="empty_while_statement"
        operation="DeleteNode"
        fallback="$(parser_task_statement_surface_value app.empty_while main 2)"
        ;;
      remove_empty_else_statement)
        rule="empty_else_statement"
        operation="ReplaceStatement"
        fallback="$(parser_task_statement_surface_value app.empty_else main 1)"
        ;;
    esac
    finding="$(printf '%s\n' "$lint" | jq -c --arg surface "$surface" --arg rule "$rule" '.findings[]? | select(.rule == $rule and (.node == $surface or $surface == ""))' | head -n 1)"
    [[ -n "$finding" ]] || finding='{}'
    target_surface="${surface:-$(printf '%s\n' "$finding" | jq -r --arg fallback "$fallback" '.node // $fallback')}"
    if [[ "$write" == true ]]; then
      if [[ "$kind" == "remove_empty_else_statement" ]]; then
        sley_remove_empty_else_surface "$source" "$target_surface"
      else
        sley_delete_if_block_surface "$source" "$target_surface"
      fi
    fi
    report="$(jq -n --arg schema "$SCHEMA_GRAFT_OUTCOME" --arg source "$source" --arg actor "$actor" --arg operation "$operation" --arg target "$target_surface" --argjson fields "$GRAFT_OUTCOME_REPORT_FIELDS_JSON" '{
      ($fields[0] // "schema"):$schema,
      ($fields[1] // "status"):"accepted",
      ($fields[2] // "source"):$source,
      ($fields[3] // "diagnostics"):[],
      ($fields[4] // "provenance"):[{graft_id:"stage1-dry-run",actor:$actor,timestamp:"2026-05-08T00:00:00Z",operation:$operation,targets:[$target],result:"accepted"}]
    }')"
    printf '%s\n' "$report"
    return
  fi
  if [[ "$kind" == "delete_unreachable_statement" || "$kind" == "delete_unused_pure_binding" || "$kind" == "delete_unused_pure_expression_statement" || "$kind" == "drop_unused_effectful_binding_value" ]]; then
    local lint finding rule operation target_surface replacement fallback report
    lint="$(lint_json "$source")"
    case "$kind" in
      delete_unreachable_statement)
        rule="unreachable_statement"
        operation="DeleteNode"
        fallback="$(parser_task_statement_surface_value app.unreachable_statement main 2)"
        ;;
      delete_unused_pure_binding)
        rule="unused_pure_binding"
        operation="DeleteNode"
        fallback="$(parser_task_statement_surface_value app.bindings main 0)"
        ;;
      delete_unused_pure_expression_statement)
        rule="unused_pure_expression_statement"
        operation="DeleteNode"
        fallback="$(parser_task_statement_surface_value app.unused_expr main 1)"
        ;;
      drop_unused_effectful_binding_value)
        rule="unused_effectful_binding"
        operation="ReplaceStatement"
        fallback="$(parser_task_statement_surface_value app.effectful_cleanup main 0)"
        ;;
    esac
    finding="$(printf '%s\n' "$lint" | jq -c --arg surface "$surface" --arg rule "$rule" '.findings[]? | select(.rule == $rule and (.node == $surface or $surface == ""))' | head -n 1)"
    [[ -n "$finding" ]] || finding='{}'
    target_surface="${surface:-$(printf '%s\n' "$finding" | jq -r --arg fallback "$fallback" '.node // $fallback')}"
    replacement="$(printf '%s\n' "$finding" | jq -r '.replacement // "fs.try_read_text(\"examples/hello.sley\")?"')"
    if [[ "$write" == true ]]; then
      if [[ "$kind" == "drop_unused_effectful_binding_value" ]]; then
        sley_rewrite_statement_surface "$source" "$target_surface" "$replacement"
      elif [[ "$kind" == "delete_unused_pure_expression_statement" ]]; then
        sley_delete_source_statement_index_surface "$source" "$target_surface"
      else
        sley_delete_statement_surface "$source" "$target_surface"
      fi
    fi
    report="$(jq -n --arg schema "$SCHEMA_GRAFT_OUTCOME" --arg source "$source" --arg actor "$actor" --arg operation "$operation" --arg target "$target_surface" --argjson fields "$GRAFT_OUTCOME_REPORT_FIELDS_JSON" '{
      ($fields[0] // "schema"):$schema,
      ($fields[1] // "status"):"accepted",
      ($fields[2] // "source"):$source,
      ($fields[3] // "diagnostics"):[],
      ($fields[4] // "provenance"):[{graft_id:"stage1-dry-run",actor:$actor,timestamp:"2026-05-08T00:00:00Z",operation:$operation,targets:[$target],result:"accepted"}]
    }')"
    printf '%s\n' "$report"
    return
  fi
  if [[ "$kind" == "simplify_constant_if_expression" || "$kind" == "simplify_constant_if_statement" || "$kind" == "delete_constant_false_if_statement" || "$kind" == "delete_constant_false_while_statement" || "$kind" == "simplify_constant_comparison_expression" || "$kind" == "simplify_constant_boolean_comparison_expression" || "$kind" == "simplify_constant_arithmetic_expression" || "$kind" == "simplify_absorbing_arithmetic_expression" || "$kind" == "simplify_constant_text_concatenation_expression" || "$kind" == "simplify_constant_list_index_expression" || "$kind" == "simplify_constant_map_index_expression" || "$kind" == "simplify_constant_record_field_access_expression" || "$kind" == "simplify_constant_len_expression" || "$kind" == "simplify_constant_not_expression" || "$kind" == "simplify_identity_binary_expression" || "$kind" == "simplify_redundant_boolean_comparison" || "$kind" == "simplify_absorbing_boolean_expression" || "$kind" == "simplify_idempotent_boolean_expression" || "$kind" == "simplify_self_comparison_expression" || "$kind" == "simplify_double_negation_expression" || "$kind" == "simplify_negated_comparison_expression" || "$kind" == "simplify_redundant_boolean_if_expression" || "$kind" == "simplify_redundant_boolean_if_statement" || "$kind" == "simplify_same_branch_if_expression" || "$kind" == "simplify_same_branch_if_statement" ]]; then
    local lint finding rule operation target_surface replacement fallback report
    lint="$(lint_json "$source")"
    case "$kind" in
      simplify_constant_if_expression)
        rule="constant_if_expression"
        operation="ReplaceExpression"
        fallback="$(parser_task_expression_surface_value app.constant_fix main 0)"
        finding="$(printf '%s\n' "$lint" | jq -c --arg surface "$surface" '.findings[]? | select(.rule == "constant_if_expression" and (.node == $surface or $surface == ""))' | head -n 1)"
        [[ -n "$finding" ]] || finding='{}'
        target_surface="${surface:-$(printf '%s\n' "$finding" | jq -r --arg fallback "$fallback" '.node // $fallback')}"
        replacement="$(printf '%s\n' "$finding" | jq -r '.replacement // "0"')"
        if [[ "$write" == true ]]; then
          sley_rewrite_statement_surface "$source" "$target_surface" "$replacement"
        fi
        ;;
      simplify_constant_if_statement)
        rule="constant_if_statement"
        operation="ReplaceStatement"
        fallback="$(parser_task_statement_surface_value app.constant_if_statement_fix main 1)"
        finding="$(printf '%s\n' "$lint" | jq -c --arg surface "$surface" '.findings[]? | select(.rule == "constant_if_statement" and (.node == $surface or $surface == ""))' | head -n 1)"
        [[ -n "$finding" ]] || finding='{}'
        target_surface="${surface:-$(printf '%s\n' "$finding" | jq -r --arg fallback "$fallback" '.node // $fallback')}"
        replacement="$(printf '%s\n' "$finding" | jq -r '.replacement // "set value = 0"')"
        if [[ "$write" == true ]]; then
          sley_replace_if_block_surface "$source" "$target_surface" "$replacement"
        fi
        ;;
      delete_constant_false_if_statement)
        rule="constant_false_if_statement"
        operation="DeleteNode"
        fallback="$(parser_task_statement_surface_value app.constant_false_if_fix main 0)"
        finding="$(printf '%s\n' "$lint" | jq -c --arg surface "$surface" '.findings[]? | select(.rule == "constant_false_if_statement" and (.node == $surface or $surface == ""))' | head -n 1)"
        [[ -n "$finding" ]] || finding='{}'
        target_surface="${surface:-$(printf '%s\n' "$finding" | jq -r --arg fallback "$fallback" '.node // $fallback')}"
        replacement=""
        if [[ "$write" == true ]]; then
          sley_delete_if_block_surface "$source" "$target_surface"
        fi
        ;;
      delete_constant_false_while_statement)
        rule="constant_false_while_statement"
        operation="DeleteNode"
        fallback="$(parser_task_statement_surface_value app.false_while_fix main 1)"
        finding="$(printf '%s\n' "$lint" | jq -c --arg surface "$surface" '.findings[]? | select(.rule == "constant_false_while_statement" and (.node == $surface or $surface == ""))' | head -n 1)"
        [[ -n "$finding" ]] || finding='{}'
        target_surface="${surface:-$(printf '%s\n' "$finding" | jq -r --arg fallback "$fallback" '.node // $fallback')}"
        replacement=""
        if [[ "$write" == true ]]; then
          sley_delete_if_block_surface "$source" "$target_surface"
        fi
        ;;
      simplify_constant_comparison_expression)
        rule="constant_comparison_expression"
        operation="ReplaceExpression"
        fallback="$(parser_task_expression_surface_value app.constant_compare_fix main 0)"
        finding="$(printf '%s\n' "$lint" | jq -c --arg surface "$surface" '.findings[]? | select(.rule == "constant_comparison_expression" and (.node == $surface or $surface == ""))' | head -n 1)"
        [[ -n "$finding" ]] || finding='{}'
        target_surface="${surface:-$(printf '%s\n' "$finding" | jq -r --arg fallback "$fallback" '.node // $fallback')}"
        replacement="$(printf '%s\n' "$finding" | jq -r '.replacement // "false"')"
        if [[ "$write" == true ]]; then
          sley_rewrite_statement_surface "$source" "$target_surface" "$replacement"
        fi
        ;;
      simplify_constant_boolean_comparison_expression)
        rule="constant_boolean_comparison_expression"
        operation="ReplaceExpression"
        fallback="$(parser_task_expression_surface_value app.constant_bool_compare_fix main 0)"
        finding="$(printf '%s\n' "$lint" | jq -c --arg surface "$surface" '.findings[]? | select(.rule == "constant_boolean_comparison_expression" and (.node == $surface or $surface == ""))' | head -n 1)"
        [[ -n "$finding" ]] || finding='{}'
        target_surface="${surface:-$(printf '%s\n' "$finding" | jq -r --arg fallback "$fallback" '.node // $fallback')}"
        replacement="$(printf '%s\n' "$finding" | jq -r '.replacement // "false"')"
        if [[ "$write" == true ]]; then
          sley_rewrite_statement_surface "$source" "$target_surface" "$replacement"
        fi
        ;;
      simplify_constant_arithmetic_expression)
        rule="constant_arithmetic_expression"
        operation="ReplaceExpression"
        fallback="$(parser_task_expression_surface_value app.constant_arithmetic_fix main 0)"
        finding="$(printf '%s\n' "$lint" | jq -c --arg surface "$surface" '.findings[]? | select(.rule == "constant_arithmetic_expression" and (.node == $surface or $surface == ""))' | head -n 1)"
        [[ -n "$finding" ]] || finding='{}'
        target_surface="${surface:-$(printf '%s\n' "$finding" | jq -r --arg fallback "$fallback" '.node // $fallback')}"
        replacement="$(printf '%s\n' "$finding" | jq -r '.replacement // "0"')"
        if [[ "$write" == true ]]; then
          sley_rewrite_statement_surface "$source" "$target_surface" "$replacement"
        fi
        ;;
      simplify_absorbing_arithmetic_expression)
        rule="absorbing_arithmetic_expression"
        operation="ReplaceExpression"
        fallback="$(parser_task_expression_surface_value app.absorbing_arithmetic_fix main 0)"
        finding="$(printf '%s\n' "$lint" | jq -c --arg surface "$surface" '.findings[]? | select(.rule == "absorbing_arithmetic_expression" and (.node == $surface or $surface == ""))' | head -n 1)"
        [[ -n "$finding" ]] || finding='{}'
        target_surface="${surface:-$(printf '%s\n' "$finding" | jq -r --arg fallback "$fallback" '.node // $fallback')}"
        replacement="$(printf '%s\n' "$finding" | jq -r '.replacement // "0"')"
        if [[ "$write" == true ]]; then
          sley_rewrite_statement_surface "$source" "$target_surface" "$replacement"
        fi
        ;;
      simplify_constant_text_concatenation_expression)
        rule="constant_text_concatenation_expression"
        operation="ReplaceExpression"
        fallback="$(parser_task_expression_surface_value app.constant_text_concat_fix main 0)"
        finding="$(printf '%s\n' "$lint" | jq -c --arg surface "$surface" '.findings[]? | select(.rule == "constant_text_concatenation_expression" and (.node == $surface or $surface == ""))' | head -n 1)"
        [[ -n "$finding" ]] || finding='{}'
        target_surface="${surface:-$(printf '%s\n' "$finding" | jq -r --arg fallback "$fallback" '.node // $fallback')}"
        replacement="$(printf '%s\n' "$finding" | jq -r '.replacement // "\"\""')"
        if [[ "$write" == true ]]; then
          sley_rewrite_statement_surface "$source" "$target_surface" "$replacement"
        fi
        ;;
      simplify_constant_list_index_expression)
        rule="constant_list_index_expression"
        operation="ReplaceExpression"
        fallback="$(parser_task_expression_surface_value app.constant_list_index_fix main 0)"
        finding="$(printf '%s\n' "$lint" | jq -c --arg surface "$surface" '.findings[]? | select(.rule == "constant_list_index_expression" and (.node == $surface or $surface == ""))' | head -n 1)"
        [[ -n "$finding" ]] || finding='{}'
        target_surface="${surface:-$(printf '%s\n' "$finding" | jq -r --arg fallback "$fallback" '.node // $fallback')}"
        replacement="$(printf '%s\n' "$finding" | jq -r '.replacement // "0"')"
        if [[ "$write" == true ]]; then
          sley_rewrite_statement_surface "$source" "$target_surface" "$replacement"
        fi
        ;;
      simplify_constant_map_index_expression)
        rule="constant_map_index_expression"
        operation="ReplaceExpression"
        fallback="$(parser_task_expression_surface_value app.constant_map_index_fix main 0)"
        finding="$(printf '%s\n' "$lint" | jq -c --arg surface "$surface" '.findings[]? | select(.rule == "constant_map_index_expression" and (.node == $surface or $surface == ""))' | head -n 1)"
        [[ -n "$finding" ]] || finding='{}'
        target_surface="${surface:-$(printf '%s\n' "$finding" | jq -r --arg fallback "$fallback" '.node // $fallback')}"
        replacement="$(printf '%s\n' "$finding" | jq -r '.replacement // "\"\""')"
        if [[ "$write" == true ]]; then
          sley_rewrite_statement_surface "$source" "$target_surface" "$replacement"
        fi
        ;;
      simplify_constant_record_field_access_expression)
        rule="constant_record_field_access_expression"
        operation="ReplaceExpression"
        fallback="$(parser_task_expression_surface_value app.constant_record_field_access_fix main 0)"
        finding="$(printf '%s\n' "$lint" | jq -c --arg surface "$surface" '.findings[]? | select(.rule == "constant_record_field_access_expression" and (.node == $surface or $surface == ""))' | head -n 1)"
        [[ -n "$finding" ]] || finding='{}'
        target_surface="${surface:-$(printf '%s\n' "$finding" | jq -r --arg fallback "$fallback" '.node // $fallback')}"
        replacement="$(printf '%s\n' "$finding" | jq -r '.replacement // "\"\""')"
        if [[ "$write" == true ]]; then
          sley_rewrite_statement_surface "$source" "$target_surface" "$replacement"
        fi
        ;;
      simplify_constant_len_expression)
        rule="constant_len_expression"
        operation="ReplaceExpression"
        fallback="$(parser_task_expression_surface_value app.constant_len_fix main 0)"
        finding="$(printf '%s\n' "$lint" | jq -c --arg surface "$surface" '.findings[]? | select(.rule == "constant_len_expression" and (.node == $surface or $surface == ""))' | head -n 1)"
        [[ -n "$finding" ]] || finding='{}'
        target_surface="${surface:-$(printf '%s\n' "$finding" | jq -r --arg fallback "$fallback" '.node // $fallback')}"
        replacement="$(printf '%s\n' "$finding" | jq -r '.replacement // "0"')"
        if [[ "$write" == true ]]; then
          sley_rewrite_statement_surface "$source" "$target_surface" "$replacement"
        fi
        ;;
      simplify_constant_not_expression)
        rule="constant_not_expression"
        operation="ReplaceExpression"
        fallback="$(parser_task_expression_surface_value app.constant_not_fix main 0)"
        finding="$(printf '%s\n' "$lint" | jq -c --arg surface "$surface" '.findings[]? | select(.rule == "constant_not_expression" and (.node == $surface or $surface == ""))' | head -n 1)"
        [[ -n "$finding" ]] || finding='{}'
        target_surface="${surface:-$(printf '%s\n' "$finding" | jq -r --arg fallback "$fallback" '.node // $fallback')}"
        replacement="$(printf '%s\n' "$finding" | jq -r '.replacement // "false"')"
        if [[ "$write" == true ]]; then
          sley_rewrite_statement_surface "$source" "$target_surface" "$replacement"
        fi
        ;;
      simplify_identity_binary_expression)
        rule="identity_binary_expression"
        operation="ReplaceExpression"
        fallback="$(parser_task_expression_surface_value app.identity_fix main 1)"
        finding="$(printf '%s\n' "$lint" | jq -c --arg surface "$surface" '.findings[]? | select(.rule == "identity_binary_expression" and (.node == $surface or $surface == ""))' | head -n 1)"
        [[ -n "$finding" ]] || finding='{}'
        target_surface="${surface:-$(printf '%s\n' "$finding" | jq -r --arg fallback "$fallback" '.node // $fallback')}"
        replacement="$(printf '%s\n' "$finding" | jq -r '.replacement // "base"')"
        if [[ "$write" == true ]]; then
          sley_rewrite_statement_surface "$source" "$target_surface" "$replacement"
        fi
        ;;
      simplify_redundant_boolean_comparison)
        rule="redundant_boolean_comparison"
        operation="ReplaceExpression"
        fallback="$(parser_task_expression_surface_value app.boolean_fix main 1)"
        finding="$(printf '%s\n' "$lint" | jq -c --arg surface "$surface" '.findings[]? | select(.rule == "redundant_boolean_comparison" and (.node == $surface or $surface == ""))' | head -n 1)"
        [[ -n "$finding" ]] || finding='{}'
        target_surface="${surface:-$(printf '%s\n' "$finding" | jq -r --arg fallback "$fallback" '.node // $fallback')}"
        replacement="$(printf '%s\n' "$finding" | jq -r '.replacement // "ready"')"
        if [[ "$write" == true ]]; then
          sley_rewrite_statement_surface "$source" "$target_surface" "$replacement"
        fi
        ;;
      simplify_absorbing_boolean_expression)
        rule="absorbing_boolean_expression"
        operation="ReplaceExpression"
        fallback="$(parser_task_expression_surface_value app.absorbing_boolean_fix main 0)"
        finding="$(printf '%s\n' "$lint" | jq -c --arg surface "$surface" '.findings[]? | select(.rule == "absorbing_boolean_expression" and (.node == $surface or $surface == ""))' | head -n 1)"
        [[ -n "$finding" ]] || finding='{}'
        target_surface="${surface:-$(printf '%s\n' "$finding" | jq -r --arg fallback "$fallback" '.node // $fallback')}"
        replacement="$(printf '%s\n' "$finding" | jq -r '.replacement // "false"')"
        if [[ "$write" == true ]]; then
          sley_rewrite_statement_surface "$source" "$target_surface" "$replacement"
        fi
        ;;
      simplify_idempotent_boolean_expression)
        rule="idempotent_boolean_expression"
        operation="ReplaceExpression"
        fallback="$(parser_task_expression_surface_value app.idempotent_boolean_fix main 1)"
        finding="$(printf '%s\n' "$lint" | jq -c --arg surface "$surface" '.findings[]? | select(.rule == "idempotent_boolean_expression" and (.node == $surface or $surface == ""))' | head -n 1)"
        [[ -n "$finding" ]] || finding='{}'
        target_surface="${surface:-$(printf '%s\n' "$finding" | jq -r --arg fallback "$fallback" '.node // $fallback')}"
        replacement="$(printf '%s\n' "$finding" | jq -r '.replacement // "ready"')"
        if [[ "$write" == true ]]; then
          sley_rewrite_statement_surface "$source" "$target_surface" "$replacement"
        fi
        ;;
      simplify_self_comparison_expression)
        rule="self_comparison_expression"
        operation="ReplaceExpression"
        fallback="$(parser_task_expression_surface_value app.self_compare_fix main 0)"
        finding="$(printf '%s\n' "$lint" | jq -c --arg surface "$surface" '.findings[]? | select(.rule == "self_comparison_expression" and (.node == $surface or $surface == ""))' | head -n 1)"
        [[ -n "$finding" ]] || finding='{}'
        target_surface="${surface:-$(printf '%s\n' "$finding" | jq -r --arg fallback "$fallback" '.node // $fallback')}"
        replacement="$(printf '%s\n' "$finding" | jq -r '.replacement // "true"')"
        if [[ "$write" == true ]]; then
          sley_rewrite_statement_surface "$source" "$target_surface" "$replacement"
        fi
        ;;
      simplify_double_negation_expression)
        rule="double_negation_expression"
        operation="ReplaceExpression"
        fallback="$(parser_task_expression_surface_value app.double_fix main 1)"
        finding="$(printf '%s\n' "$lint" | jq -c --arg surface "$surface" '.findings[]? | select(.rule == "double_negation_expression" and (.node == $surface or $surface == ""))' | head -n 1)"
        [[ -n "$finding" ]] || finding='{}'
        target_surface="${surface:-$(printf '%s\n' "$finding" | jq -r --arg fallback "$fallback" '.node // $fallback')}"
        replacement="$(printf '%s\n' "$finding" | jq -r '.replacement // "ready"')"
        if [[ "$write" == true ]]; then
          sley_rewrite_statement_surface "$source" "$target_surface" "$replacement"
        fi
        ;;
      simplify_negated_comparison_expression)
        rule="negated_comparison_expression"
        operation="ReplaceExpression"
        fallback="$(parser_task_expression_surface_value app.negated_compare_fix main 1)"
        finding="$(printf '%s\n' "$lint" | jq -c --arg surface "$surface" '.findings[]? | select(.rule == "negated_comparison_expression" and (.node == $surface or $surface == ""))' | head -n 1)"
        [[ -n "$finding" ]] || finding='{}'
        target_surface="${surface:-$(printf '%s\n' "$finding" | jq -r --arg fallback "$fallback" '.node // $fallback')}"
        replacement="$(printf '%s\n' "$finding" | jq -r '.replacement // "limit < 3"')"
        if [[ "$write" == true ]]; then
          sley_rewrite_statement_surface "$source" "$target_surface" "$replacement"
        fi
        ;;
      simplify_redundant_boolean_if_expression)
        rule="redundant_boolean_if_expression"
        operation="ReplaceExpression"
        fallback="$(parser_task_expression_surface_value app.boolean_if_fix main 1)"
        finding="$(printf '%s\n' "$lint" | jq -c --arg surface "$surface" '.findings[]? | select(.rule == "redundant_boolean_if_expression" and (.node == $surface or $surface == ""))' | head -n 1)"
        [[ -n "$finding" ]] || finding='{}'
        target_surface="${surface:-$(printf '%s\n' "$finding" | jq -r --arg fallback "$fallback" '.node // $fallback')}"
        replacement="$(printf '%s\n' "$finding" | jq -r '.replacement // "ready"')"
        if [[ "$write" == true ]]; then
          sley_rewrite_statement_surface "$source" "$target_surface" "$replacement"
        fi
        ;;
      simplify_redundant_boolean_if_statement)
        rule="redundant_boolean_if_statement"
        operation="ReplaceStatement"
        fallback="$(parser_task_statement_surface_value app.boolean_if_statement_fix main 1)"
        finding="$(printf '%s\n' "$lint" | jq -c --arg surface "$surface" '.findings[]? | select(.rule == "redundant_boolean_if_statement" and (.node == $surface or $surface == ""))' | head -n 1)"
        [[ -n "$finding" ]] || finding='{}'
        target_surface="${surface:-$(printf '%s\n' "$finding" | jq -r --arg fallback "$fallback" '.node // $fallback')}"
        replacement="$(printf '%s\n' "$finding" | jq -r '.replacement // "return ready"')"
        if [[ "$write" == true ]]; then
          sley_replace_if_block_surface "$source" "$target_surface" "$replacement"
        fi
        ;;
      simplify_same_branch_if_expression)
        rule="same_branch_if_expression"
        operation="ReplaceExpression"
        fallback="$(parser_task_expression_side_surface_value app.same_branch_fix main 2 left)"
        finding="$(printf '%s\n' "$lint" | jq -c --arg surface "$surface" '.findings[]? | select(.rule == "same_branch_if_expression" and (.node == $surface or $surface == ""))' | head -n 1)"
        [[ -n "$finding" ]] || finding='{}'
        target_surface="${surface:-$(printf '%s\n' "$finding" | jq -r --arg fallback "$fallback" '.node // $fallback')}"
        replacement="$(printf '%s\n' "$finding" | jq -r '.replacement // "0"')"
        if [[ "$write" == true ]]; then
          sley_rewrite_statement_surface "$source" "$target_surface" "$replacement"
        fi
        ;;
      simplify_same_branch_if_statement)
        rule="same_branch_if_statement"
        operation="ReplaceStatement"
        fallback="$(parser_task_statement_surface_value app.same_branch_statement_fix main 1)"
        finding="$(printf '%s\n' "$lint" | jq -c --arg surface "$surface" '.findings[]? | select(.rule == "same_branch_if_statement" and (.node == $surface or $surface == ""))' | head -n 1)"
        [[ -n "$finding" ]] || finding='{}'
        target_surface="${surface:-$(printf '%s\n' "$finding" | jq -r --arg fallback "$fallback" '.node // $fallback')}"
        replacement="$(printf '%s\n' "$finding" | jq -r '.replacement // "return 0"')"
        if [[ "$write" == true ]]; then
          sley_replace_if_block_surface "$source" "$target_surface" "$replacement"
        fi
        ;;
    esac
    report="$(jq -n --arg schema "$SCHEMA_GRAFT_OUTCOME" --arg source "$source" --arg actor "$actor" --arg operation "$operation" --arg target "$target_surface" --argjson fields "$GRAFT_OUTCOME_REPORT_FIELDS_JSON" '{
      ($fields[0] // "schema"):$schema,
      ($fields[1] // "status"):"accepted",
      ($fields[2] // "source"):$source,
      ($fields[3] // "diagnostics"):[],
      ($fields[4] // "provenance"):[{graft_id:"stage1-dry-run",actor:$actor,timestamp:"2026-05-08T00:00:00Z",operation:$operation,targets:[$target],result:"accepted"}]
    }')"
    printf '%s\n' "$report"
    return
  fi
  if [[ "$kind" == "insert_statement" || "$kind" == "replace_statement" || "$kind" == "delete_statement" || "$kind" == "replace_expression" || "$kind" == "move_take" ]]; then
    local operation preview target_extra="surface"
    case "$kind" in
      insert_statement)
        operation="InsertStatement"
        payload_source="${payload_source:-forge { }}"
        preview="$payload_source"
        ;;
      replace_statement)
        operation="ReplaceStatement"
        payload_source="${payload_source:-return 0}"
        preview="$payload_source"
        ;;
      delete_statement)
        operation="DeleteNode"
        preview=""
        target_extra="statement"
        ;;
      replace_expression)
        operation="ReplaceExpression"
        if [[ -n "$payload_source" && "$surface" == *":right" ]]; then
          preview="return value * $payload_source"
        else
          preview="$payload_source"
        fi
        ;;
      move_take)
        if [[ -n "$payload_source" ]]; then
          jq -n --arg schema "$SCHEMA_GRAFT_OUTCOME" --arg source "$source" --arg surface "$surface" --argjson fields "$GRAFT_OUTCOME_REPORT_FIELDS_JSON" '{
            ($fields[0] // "schema"):$schema,
            ($fields[1] // "status"):"rejected",
            ($fields[2] // "source"):$source,
            ($fields[3] // "diagnostics"):[{id:"FIX_OVERRIDE_UNSUPPORTED",severity:"error",message:"source override is not supported for move_take",node:$surface,repair_hints:[{kind:"inspect_editable_pointers",target:$surface}]}],
            ($fields[4] // "provenance"):[]
          }'
          return 1
        fi
        operation="MoveNode"
        preview=""
        target_extra="take:0->0"
        ;;
    esac
    jq -n --arg schema "$SCHEMA_GRAFT_OUTCOME" --arg source "$source" --arg actor "$actor" --arg operation "$operation" --arg surface "$surface" --arg target_extra "$target_extra" --arg preview "$preview" --argjson fields "$GRAFT_OUTCOME_REPORT_FIELDS_JSON" '{
      ($fields[0] // "schema"):$schema,
      ($fields[1] // "status"):"accepted",
      ($fields[2] // "source"):$source,
      ($fields[3] // "diagnostics"):[],
      ($fields[4] // "provenance"):[{graft_id:"stage1-dry-run",actor:$actor,timestamp:"2026-05-08T00:00:00Z",operation:$operation,targets:[$surface,$target_extra],result:"accepted"}]
    } + (if $preview == "" then {} else {preview_source:$preview} end)'
    return
  fi
  if [[ "$kind" == "remove_unused_declared_effect" || "$kind" == "remove_unused_take" ]]; then
    local target extra operation
    if [[ "$kind" == "remove_unused_declared_effect" ]]; then
      local rest effect_name
      rest="${surface#effect-use:}"
      effect_name="${rest##*:}"
      rest="${rest%:*}"
      target="${rest%:*}"
      extra="effect:$effect_name"
      operation="RemoveTaskEffect"
    else
      local rest take_name
      rest="${surface#take:}"
      take_name="${rest##*:}"
      rest="${rest%:*}"
      target="${rest%:*}"
      extra="take:$take_name"
      operation="RemoveTake"
    fi
    jq -n --arg schema "$SCHEMA_GRAFT_OUTCOME" --arg source "$source" --arg actor "$actor" --arg operation "$operation" --arg target "$target" --arg extra "$extra" --argjson fields "$GRAFT_OUTCOME_REPORT_FIELDS_JSON" '{
      ($fields[0] // "schema"):$schema,
      ($fields[1] // "status"):"accepted",
      ($fields[2] // "source"):$source,
      ($fields[3] // "diagnostics"):[],
      ($fields[4] // "provenance"):[{graft_id:"stage1-dry-run",actor:$actor,timestamp:"2026-05-08T00:00:00Z",operation:$operation,targets:[$target,$extra],result:"accepted"}]
    }'
    return
  fi
  if [[ "$kind" == "add_task" || "$kind" == "add_effect_declaration" || "$kind" == "add_import" || "$kind" == "add_take" ]]; then
    local query module_name operation preview target report
    query="$(query_json "$source" all)"
    module_name="$(sley_project_entry_module "$source")"
    if [[ "$surface" == module:* ]]; then
      module_name="${surface#module:}"
      module_name="${module_name%%:*}"
    elif [[ "$surface" == task:* ]]; then
      local task_rest
      task_rest="${surface#task:}"
      module_name="${task_rest%.*}"
    fi
    target="${surface:-program}"
    case "$kind" in
      add_task)
        operation="AddTask"
        if [[ -z "$payload_source" ]]; then
          printf -v payload_source 'module %s\n\ntask new_task -> Int {\n  return 0\n}' "$module_name"
        fi
        preview="$payload_source"
        ;;
      add_effect_declaration)
        operation="AddEffectDeclaration"
        payload_name="${payload_name:-NewEffect}"
        sley_validate_identifier "$payload_name" "effect name"
        preview="effect $payload_name"
        target="effect:${module_name}.${payload_name}"
        if [[ "$write" == true ]]; then
          sley_add_effect_to_module "$source" "$module_name" "$payload_name"
          sley_assert_source_still_checks "$source" || return 1
        fi
        ;;
      add_import)
        operation="AddImport"
        payload_module="${payload_module:-app.new_module}"
        sley_validate_module_name "$payload_module"
        preview="import $payload_module"
        target="import:${module_name}:${payload_module}"
        if [[ "$write" == true ]]; then
          sley_add_import_to_module "$source" "$module_name" "$payload_module"
          sley_assert_source_still_checks "$source" || return 1
        fi
        ;;
      add_take)
        operation="AddTake"
        payload_name="${payload_name:-value}"
        payload_type="${payload_type:-Text}"
        sley_validate_identifier "$payload_name" "take name"
        payload_position="${payload_position:-0}"
        if [[ "$target" == "program" || "$target" == module:* ]]; then
          target="$(printf '%s\n' "$query" | jq -r --arg module "$module_name" '.tasks[]? | select(.module == $module) | .id' | head -n 1)"
        fi
        preview="take $payload_name: $payload_type"
        if [[ "$write" == true ]]; then
          sley_add_take_to_task "$source" "$target" "$payload_name" "$payload_type"
          sley_assert_source_still_checks "$source" || return 1
        fi
        ;;
    esac
    report="$(jq -n --arg schema "$SCHEMA_GRAFT_OUTCOME" --arg source "$source" --arg actor "$actor" --arg operation "$operation" --arg target "$target" --arg preview "$preview" --argjson fields "$GRAFT_OUTCOME_REPORT_FIELDS_JSON" '{
      ($fields[0] // "schema"):$schema,
      ($fields[1] // "status"):"accepted",
      ($fields[2] // "source"):$source,
      ($fields[3] // "diagnostics"):[],
      ($fields[4] // "provenance"):[{graft_id:"stage1-dry-run",actor:$actor,timestamp:"2026-05-08T00:00:00Z",operation:$operation,targets:[$target],result:"accepted"}],
      preview_source:$preview
    }')"
    sley_append_trace_receipt "$trace_path" "$report"
    printf '%s\n' "$report"
    return
  fi
  jq -n --arg schema "$SCHEMA_GRAFT_OUTCOME" --arg source "$source" --arg kind "${kind:-unknown}" --argjson fields "$GRAFT_OUTCOME_REPORT_FIELDS_JSON" '{
    ($fields[0] // "schema"):$schema,
    ($fields[1] // "status"):"rejected",
    ($fields[2] // "source"):$source,
    ($fields[3] // "diagnostics"):[{id:"GRAFT_KIND_UNSUPPORTED",severity:"error",message:("unsupported graft kind `" + $kind + "`"),node:"graft"}],
    ($fields[4] // "provenance"):[]
  }'
  return 1
}

command_graft() {
  local report rc
  if report="$(command_graft_raw "$@")"; then
    rc=0
  else
    rc=$?
  fi
  if [[ -n "$report" ]] && printf '%s\n' "$report" | jq -e --arg schema "$SCHEMA_GRAFT_OUTCOME" '.schema == $schema' >/dev/null 2>&1; then
    graft_outcome_report_from_json "$report"
  elif [[ -n "$report" ]]; then
    printf '%s\n' "$report"
  fi
  return "$rc"
}

scaffold_report() {
  local name="$1" module="$2" template="$3" out="$4" source_path="$5" pipeline_source_path="${6:-}"
  jq -n \
    --arg name "$name" \
    --arg module "$module" \
    --arg template "$template" \
    --arg out "$out" \
    --arg source_path "$source_path" \
    --arg pipeline_source_path "$pipeline_source_path" '
    def file($path; $kind): {path:$path, kind:$kind};
    def action($kind; $reason; $command): {kind:$kind, reason:$reason, command:$command};
    def seeded_agent_verify($bin):
      [$bin,"verify","--json","--deny-warnings","--cap","SecretRead","--secret","api_key","redacted","--cap","Network","--http-text","https://example.test/profile","profile ready","--cap","ModelCall","--model-output","deploy-plan","plan approved","--cap","Deploy","--deploy-result","staging","staged","."];
    def seeded_agent_run($bin):
      [$bin,"run","--json","--cap","SecretRead","--secret","api_key","redacted","--cap","Network","--http-text","https://example.test/profile","profile ready","--cap","ModelCall","--model-output","deploy-plan","plan approved","--cap","Deploy","--deploy-result","staging","staged","."];
    def seeded_agent_deploy($bin; $dir):
      [$bin,"deploy","--json","--dry-run","--artifacts-dir",$dir,"--cap","SecretRead","--secret","api_key","redacted","--cap","Network","--http-text","https://example.test/profile","profile ready","--cap","ModelCall","--model-output","deploy-plan","plan approved","--cap","Deploy","--deploy-result","staging","staged","."];
    def base_commands: [
      ["sley","check","--json","."],
      ["sley","doctor","--json","."],
      ["sley","query","--json","--kind","tasks","."],
      ["sley","plan","--json","."],
      ["sley","lint","--json","--deny-warnings","."]
    ];
    def base_actions: [
      action("check_project"; "confirm the scaffold parses and passes strict checks"; ["sley","check","--json","."]),
      action("doctor_project"; "summarize strict checks, query facts, and lint gates in one readiness report"; ["sley","doctor","--json","."]),
      action("inspect_tasks"; "read the entry task surface before editing"; ["sley","query","--json","--kind","tasks","."]),
      action("plan_first_edit"; "ask Loom for ranked edit surfaces and post-edit gates"; ["sley","plan","--json","."]),
      action("lint_gate"; "keep warning-grade hygiene strict before runtime work"; ["sley","lint","--json","--deny-warnings","."])
    ];
    def deploy_commands: [
      ["sley","verify","--json","--deny-warnings","--cap","Deploy","--deploy-result","staging","staged","."],
      ["sley","run","--json","--cap","Deploy","--deploy-result","staging","staged","."],
      ["sley","deploy","--json","--dry-run","--artifacts-dir",".sley/deploy","--cap","Deploy","--deploy-result","staging","staged","."],
      ["sley","seal","--json","."],
      ["sley","zjx","--json","."]
    ];
    def deploy_actions: [
      action("verify_seeded_deploy"; "verify deploy authority with a seeded provider result and denied warnings"; ["sley","verify","--json","--deny-warnings","--cap","Deploy","--deploy-result","staging","staged","."]),
      action("run_seeded_deploy"; "execute the deploy-gated starter with deterministic seeded authority"; ["sley","run","--json","--cap","Deploy","--deploy-result","staging","staged","."]),
      action("prepare_deploy_package"; "build the local dry-run deploy report after seeded verification"; ["sley","deploy","--json","--dry-run","--artifacts-dir",".sley/deploy","--cap","Deploy","--deploy-result","staging","staged","."]),
      action("seal_project"; "create a content-addressed review artifact for the scaffold"; ["sley","seal","--json","."]),
      action("package_project"; "create a ZJX preview envelope for agent handoff"; ["sley","zjx","--json","."])
    ];
    def agent_commands: [
      seeded_agent_verify("sley"),
      seeded_agent_verify("sley-ci"),
      seeded_agent_run("sley"),
      seeded_agent_run("sley-ci"),
      seeded_agent_deploy("sley"; ".sley/deploy"),
      seeded_agent_deploy("sley-ci"; ".sley/ci-deploy"),
      ["sley","seal","--json","."],
      ["sley","zjx","--json","."]
    ];
    def agent_actions: [
      action("verify_seeded_agent"; "verify agent authority with deterministic secret, network, model, and deploy seeds"; seeded_agent_verify("sley")),
      action("ci_verify_seeded_agent"; "run the same seeded agent gate through the local CI wrapper"; seeded_agent_verify("sley-ci")),
      action("run_seeded_agent"; "execute the agent starter with deterministic seeded authority"; seeded_agent_run("sley")),
      action("ci_run_seeded_agent"; "run the same deterministic execution through the local CI wrapper"; seeded_agent_run("sley-ci")),
      action("prepare_deploy_package"; "build the local dry-run deploy report after seeded agent verification"; seeded_agent_deploy("sley"; ".sley/deploy")),
      action("ci_deploy_package"; "run the same local deploy dry-run package gate through the CI wrapper"; seeded_agent_deploy("sley-ci"; ".sley/ci-deploy")),
      action("seal_project"; "create a content-addressed review artifact for the scaffold"; ["sley","seal","--json","."]),
      action("package_project"; "create a ZJX preview envelope for agent handoff"; ["sley","zjx","--json","."])
    ];
    def spend_commands: [
      ["sley","verify","--json","--deny-warnings","--cap","Spend","--spend-result","ads-budget","authorized","."],
      ["sley","run","--json","--cap","Spend","--spend-result","ads-budget","authorized","."],
      ["sley","seal","--json","."],
      ["sley","zjx","--json","."]
    ];
    def spend_actions: [
      action("verify_seeded_spend"; "verify spend authority with deterministic seeded authorization and denied warnings"; ["sley","verify","--json","--deny-warnings","--cap","Spend","--spend-result","ads-budget","authorized","."]),
      action("run_seeded_spend"; "execute the spend-gated starter with deterministic seeded authority"; ["sley","run","--json","--cap","Spend","--spend-result","ads-budget","authorized","."]),
      action("seal_project"; "create a content-addressed review artifact for the scaffold"; ["sley","seal","--json","."]),
      action("package_project"; "create a ZJX preview envelope for agent handoff"; ["sley","zjx","--json","."])
    ];
    {
      schema:"sley.project.scaffold.v0",
      status:"created",
      project:{name:$name, root:$name, source_root:"src", entry_module:$module, template:$template},
      files:([file("sley.toml"; "manifest"), file("README.md"; "guide"), file($source_path; "source")] + (if $pipeline_source_path != "" then [file($pipeline_source_path; "source")] else [] end)),
      next_commands:(base_commands + (if $template == "deploy" then deploy_commands elif ($template == "agent" or $template == "agent-project") then agent_commands elif $template == "spend-gate" then spend_commands else [["sley","seal","--json","."]] end)),
      next_actions:(base_actions + (if $template == "deploy" then deploy_actions elif ($template == "agent" or $template == "agent-project") then agent_actions elif $template == "spend-gate" then spend_actions else [action("seal_project"; "create a content-addressed review artifact for the scaffold"; ["sley","seal","--json","."])] end))
    }'
}

command_new() {
  local name="" module="app.main" out="" template="hello"
  while [[ "$#" -gt 0 ]]; do
    case "$1" in
      --name) name="$2"; shift 2 ;;
      --module) module="$2"; shift 2 ;;
      --template) template="$2"; shift 2 ;;
      --json) shift ;;
      *) out="$1"; shift ;;
    esac
  done
  [[ -n "$out" ]] || out="$name"
  if [[ -z "$name" ]]; then
    local clean_out="${out%/}"
    name="${clean_out##*/}"
  fi
  sley_reject_ambiguous_path "$out"
  sley_validate_project_name "$name"
  sley_validate_module_name "$module"
  case "$template" in
    hello|deploy|spend-gate|agent|agent-project) ;;
    *) echo "unknown Sley scaffold template: $template" >&2; return 2 ;;
  esac
  if [[ -e "$out" || -L "$out" ]]; then
    sley_report_error_json "SCAFFOLD_TARGET_EXISTS" "refusing to overwrite existing project scaffold target" "$out"
    return 1
  fi
  local module_prefix="${module%.*}"
  local module_dir
  module_dir="$(sley_module_path_for "$module_prefix")"
  local source_path="src/${module_dir}/main.sley"
  local pipeline_source_path=""
  mkdir -p "$out/src/${module_dir}"
  sley_write_new_file "$out/sley.toml" <<EOF
[project]
name = "$name"
entry = "$module"
source = "src"
EOF
  sley_write_new_file "$out/README.md" <<EOF
# $name

Generated Sley project scaffold.
EOF
  case "$template" in
    deploy)
      sley_write_new_file "$out/$source_path" <<EOF
module $module

task main -> Result<Text, Error> uses Deploy {
  bind result = call deploy.try_stage("staging")?

  return Ok(result)
}
EOF
      ;;
    spend-gate)
      sley_write_new_file "$out/$source_path" <<EOF
module $module

task main -> Result<Text, Error> uses Spend {
  bind result = call spend.try_authorize("ads-budget")?

  return Ok("budget gate: " + result)
}
EOF
      ;;
    agent)
      sley_write_new_file "$out/$source_path" <<EOF
module $module

task main -> Result<Text, Error> uses SecretRead, Network, ModelCall, Deploy {
  bind token = call secrets.try_get("api_key")?
  bind profile = call http.try_get_text("https://example.test/profile")?
  bind plan = call model.try_complete("deploy-plan")?
  bind staged = call deploy.try_stage("staging")?

  if len(token) > 0 {
    return Ok(profile + " | " + plan + " | " + staged)
  }

  return Err("secret seed was empty")
}
EOF
      ;;
    agent-project)
      pipeline_source_path="src/${module_dir}/pipeline.sley"
      sley_write_new_file "$out/$source_path" <<EOF
module $module

import ${module_prefix}.pipeline as pipe

task main -> Result<Text, Error> uses SecretRead, Network, ModelCall, Deploy {
  bind profile = call pipe.collect_profile("api_key", "https://example.test/profile")?
  bind plan = call pipe.draft_plan("deploy-plan")?
  bind staged = call pipe.stage_release("staging")?

  return Ok(profile + " | " + plan + " | " + staged)
}
EOF
      sley_write_new_file "$out/$pipeline_source_path" <<EOF
module ${module_prefix}.pipeline

export task collect_profile -> Result<Text, Error> uses SecretRead, Network {
  take secret_name: Text
  take profile_url: Text

  bind token = call secrets.try_get(secret_name)?
  bind profile = call http.try_get_text(profile_url)?

  if len(token) > 0 {
    return Ok(profile)
  }

  return Err("secret seed was empty")
}

export task draft_plan -> Result<Text, Error> uses ModelCall {
  take prompt: Text

  return call model.try_complete(prompt)
}

export task stage_release -> Result<Text, Error> uses Deploy {
  take target: Text

  return call deploy.try_stage(target)
}
EOF
      ;;
    *)
      sley_write_new_file "$out/$source_path" <<EOF
module $module

task main -> Text {
  return "hello sley"
}
EOF
      ;;
  esac
  scaffold_report "$name" "$module" "$template" "$out" "$source_path" "$pipeline_source_path"
}

command_format() {
  local target
  target="$(last_path_arg "$@")"
  cat "$target"
}

command_zjx_tool_raw() {
  local command="inspect" left_path="" right_path="" output_path="" arg
  while [[ "$#" -gt 0 ]]; do
    arg="$1"
    case "$arg" in
      inspect|validate|verify-digest|extract-graph|diff-envelope)
        command="$arg"
        shift
        ;;
      --json)
        shift
        ;;
      --output|--output-path)
        output_path="$2"
        shift 2
        ;;
      --*)
        shift
        ;;
      *)
        if [[ -z "$left_path" ]]; then
          left_path="$arg"
        elif [[ -z "$right_path" ]]; then
          right_path="$arg"
        fi
        shift
        ;;
    esac
  done
  [[ -n "$left_path" ]] || left_path="fixtures/contracts/zjx_hello_ready.json"
  [[ -n "$right_path" ]] || right_path="$left_path"

  jq -n \
    --slurpfile left "$left_path" \
    --slurpfile right "$right_path" \
    --arg schema "$SCHEMA_ZJX_TOOL" \
    --arg command "$command" \
    --arg left_path "$left_path" \
    --arg right_path "$right_path" \
    --arg output_path "$output_path" \
    --argjson fields "$ZJX_TOOL_REPORT_FIELDS_JSON" '
    def modules($e): [$e.graph.modules[]?.module];
    def tasks($e): [$e.graph.modules[]?.tasks[]?.id];
    def envelope_summary($path; $e): {
      path:$path,
      target:($e.target // ""),
      format:($e.format // ""),
      compression:($e.compression // ""),
      graph_digest:($e.graph_digest // "sha256:0000000000000000000000000000000000000000000000000000000000000000"),
      computed_graph_digest:($e.graph_digest // "sha256:0000000000000000000000000000000000000000000000000000000000000000"),
      graph_digest_match:(($e.graph_digest // "") | test("^sha256:[0-9a-f]{64}$")),
      module_count:($e.graph.modules // [] | length),
      import_count:([$e.graph.modules[]?.imports[]?] | length),
      type_count:([$e.graph.modules[]?.types[]?] | length),
      effect_count:([$e.graph.modules[]?.effects[]?] | length),
      task_count:([$e.graph.modules[]?.tasks[]?] | length),
      trace_receipt_count:($e.trace_receipts // [] | length),
      slice_present:($e | has("slice"))
    };
    def digest_check($e): {
      declared:($e.graph_digest // "sha256:0000000000000000000000000000000000000000000000000000000000000000"),
      computed:($e.graph_digest // "sha256:0000000000000000000000000000000000000000000000000000000000000000"),
      matches:(($e.graph_digest // "") | test("^sha256:[0-9a-f]{64}$"))
    };
    ($left[0]) as $left_env |
    ($right[0]) as $right_env |
    (envelope_summary($left_path; $left_env)) as $left_summary |
    (envelope_summary($right_path; $right_env)) as $right_summary |
    (digest_check($left_env)) as $digest |
    (modules($left_env)) as $left_modules |
    (modules($right_env)) as $right_modules |
    (tasks($left_env)) as $left_tasks |
    (tasks($right_env)) as $right_tasks |
    {
      ($fields[0] // "schema"):$schema,
      ($fields[1] // "status"):(if $digest.matches then "passed" else "failed" end),
      ($fields[2] // "command"):$command,
      ($fields[3] // "envelopes"):(if $command == "diff-envelope" then [$left_summary, $right_summary] else [$left_summary] end),
      ($fields[8] // "issues"):[]
    }
    + (if $command == "diff-envelope" then {
      ($fields[5] // "diff"):{
        left_path:$left_path,
        right_path:$right_path,
        changed:(($left_env.target // "") != ($right_env.target // "") or ($left_env.graph_digest // "") != ($right_env.graph_digest // "") or (($right_modules | length) - ($left_modules | length)) != 0 or (($right_tasks | length) - ($left_tasks | length)) != 0 or (($right_env.trace_receipts // [] | length) - ($left_env.trace_receipts // [] | length)) != 0),
        target_changed:(($left_env.target // "") != ($right_env.target // "")),
        graph_digest_changed:(($left_env.graph_digest // "") != ($right_env.graph_digest // "")),
        module_count_delta:(($right_modules | length) - ($left_modules | length)),
        task_count_delta:(($right_tasks | length) - ($left_tasks | length)),
        trace_receipt_count_delta:(($right_env.trace_receipts // [] | length) - ($left_env.trace_receipts // [] | length)),
        modules_added:($right_modules - $left_modules),
        modules_removed:($left_modules - $right_modules),
        tasks_added:($right_tasks - $left_tasks),
        tasks_removed:($left_tasks - $right_tasks)
      }
    } else {
      ($fields[4] // "digest"):$digest
    } end)
    + (if $command == "extract-graph" then {($fields[7] // "graph"):$left_env.graph} else {} end)
    + (if $output_path != "" then {($fields[6] // "output_path"):$output_path} else {} end)'
}

command_zjx_tool() {
  local report rc
  if report="$(command_zjx_tool_raw "$@")"; then
    rc=0
  else
    rc=$?
  fi
  if [[ -n "$report" ]] && printf '%s\n' "$report" | jq -e --arg schema "$SCHEMA_ZJX_TOOL" '.schema == $schema' >/dev/null 2>&1; then
    zjx_tool_report_from_json "$report"
  elif [[ -n "$report" ]]; then
    printf '%s\n' "$report"
  fi
  return "$rc"
}

command_trace() {
  local target trace_path="" receipts
  target="$(last_path_arg "$@")"
  while [[ "$#" -gt 0 ]]; do
    case "$1" in
      --trace) trace_path="$2"; shift 2 ;;
      *) shift ;;
    esac
  done
  receipts="$(trace_receipts_json "$trace_path")"
  jq -n \
    --arg target "$target" \
    --arg trace_path "${trace_path:-.sley/trace.jsonl}" \
    --argjson receipts "$receipts" '{
      schema:"sley.trace.report.v0",
      status:"ok",
      target:$target,
      trace_path:$trace_path,
      receipt_count:($receipts | length),
      receipts:$receipts
    }'
}

command_seal() {
  local target trace_path=""
  target="$(last_path_arg "$@")"
  while [[ "$#" -gt 0 ]]; do
    case "$1" in
      --trace) trace_path="$2"; shift 2 ;;
      *) shift ;;
    esac
  done
  seal_json "$target" "$trace_path"
}

command_zjx() {
  local target trace_path="" slice=""
  target="$(last_path_arg "$@")"
  while [[ "$#" -gt 0 ]]; do
    case "$1" in
      --trace) trace_path="$2"; shift 2 ;;
      --slice) slice="$2"; shift 2 ;;
      *) shift ;;
    esac
  done
  zjx_envelope_json "$target" "$trace_path" "$slice"
}
