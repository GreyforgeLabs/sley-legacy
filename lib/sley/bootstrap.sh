# shellcheck shell=bash
# Self-hosted source evaluation and immutable compiler contract bootstrap.

sley_source_cache_fingerprint() {
  stat -c '%n:%y:%s' "$SELF_PATH" "$SLEY_LIB_DIR"/*.sh "$SELF_HOSTED_SOURCE_ROOT"/loom/*.sley 2>/dev/null \
    | sha256sum \
    | awk '{print $1}'
}

SLEY_SOURCE_CACHE_FINGERPRINT="$(sley_source_cache_fingerprint)"

sley_source_cache_path() {
  local kind="$1" file="$2" task="$3" env_json="$4" key
  key="$(printf '%s\037%s\037%s\037%s\037%s\037%s' \
    "$SLEY_SOURCE_CACHE_VERSION" \
    "$SLEY_SOURCE_CACHE_FINGERPRINT" \
    "$kind" \
    "$file" \
    "$task" \
    "$env_json" | sha256sum | awk '{print $1}')"
  printf '%s/%s.cache\n' "$SLEY_SOURCE_CACHE_DIR" "$key"
}

sley_source_cache_read() {
  local path="$1"
  [[ "${SLEY_DISABLE_SOURCE_CACHE:-}" != "1" && -f "$path" ]] || return 1
  cat "$path"
}

sley_source_cache_write() {
  local path="$1" value="$2" tmp
  [[ "${SLEY_DISABLE_SOURCE_CACHE:-}" != "1" ]] || return 0
  mkdir -p "$SLEY_SOURCE_CACHE_DIR"
  tmp="${path}.$$"
  printf '%s\n' "$value" > "$tmp"
  mv "$tmp" "$path"
}

extract_sley_string_task() {
  local file="$1" task="$2"
  [[ -f "$file" ]] || return 0
  awk -v task="$task" '
    $0 ~ "task[ \t]+" task "[ \t]*->" {in_task=1}
    in_task && /^[ \t]*}/ {exit}
    in_task && /return[ \t]+"/ {
      line=$0
      sub(/^.*return[ \t]+"/, "", line)
      sub(/".*$/, "", line)
      print line
      exit
    }
  ' "$file"
}

extract_sley_list_task_json() {
  local file="$1" task="$2"
  [[ -f "$file" ]] || {
    printf '[]\n'
    return 0
  }
  awk -v task="$task" '
    function esc(s){gsub(/\\/, "\\\\", s); gsub(/"/, "\\\"", s); return s}
    BEGIN {printf "["; first=1}
    $0 ~ "task[ \t]+" task "[ \t]*->" {in_task=1}
    in_task && /^[ \t]*}/ {in_task=0}
    in_task {
      line=$0
      while (match(line, /"[^"]+"/)) {
        item=substr(line, RSTART + 1, RLENGTH - 2)
        if (!first) printf ","
        printf "\"%s\"", esc(item)
        first=0
        line=substr(line, RSTART + RLENGTH)
      }
    }
    END {print "]"}
  ' "$file"
}

extract_sley_int_task() {
  local file="$1" task="$2"
  [[ -f "$file" ]] || return 0
  awk -v task="$task" '
    $0 ~ "task[ \t]+" task "[ \t]*->" {in_task=1}
    in_task && /^[ \t]*}/ {exit}
    in_task && /return[ \t]+[0-9]+/ {
      line=$0
      sub(/^.*return[ \t]+/, "", line)
      sub(/[^0-9].*$/, "", line)
      print line
      exit
    }
  ' "$file"
}

extract_sley_bool_task() {
  local file="$1" task="$2"
  [[ -f "$file" ]] || return 0
  awk -v task="$task" '
    $0 ~ "task[ \t]+" task "[ \t]*->" {in_task=1}
    in_task && /^[ \t]*}/ {exit}
    in_task && /return[ \t]+(true|false)/ {
      line=$0
      sub(/^.*return[ \t]+/, "", line)
      sub(/[^a-z].*$/, "", line)
      print line
      exit
    }
  ' "$file"
}

extract_sley_task_body() {
  local file="$1" task="$2"
  [[ -f "$file" ]] || return 0
  awk -v task="$task" '
    function count_char(s, c, n, i){n=0; for(i=1;i<=length(s);i++){if(substr(s,i,1)==c)n++} return n}
    $0 ~ "task[ \t]+" task "[ \t]*->" {
      in_task=1
      depth += count_char($0,"{") - count_char($0,"}")
      next
    }
    in_task {
      next_depth=depth + count_char($0,"{") - count_char($0,"}")
      if(next_depth<=0){exit}
      print
      depth=next_depth
    }
  ' "$file"
}

resolve_sley_import_alias() {
  local file="$1" alias="$2"
  [[ -f "$file" ]] || return 0
  awk -v alias="$alias" '
    function trim(s){gsub(/^[ \t\r\n]+|[ \t\r\n]+$/, "", s); return s}
    /^[ \t]*import[ \t]+/ {
      line=$0
      sub(/^[ \t]*import[ \t]+/, "", line)
      if(line ~ "[ \t]+as[ \t]+" alias "[ \t]*$") {
        sub(/[ \t]+as[ \t]+.*$/, "", line)
        print trim(line)
        exit
      }
      if(line !~ /[ \t]+as[ \t]+/) {
        module=trim(line)
        default_alias=module
        sub(/^.*[.]/, "", default_alias)
        if(default_alias == alias) {
          print module
          exit
        }
      }
    }
  ' "$file"
}

resolve_sley_unqualified_imported_task_file() {
  local file="$1" task="$2" module source_file candidate_file="" candidate_count=0
  [[ -f "$file" && -n "$task" ]] || return 1
  while IFS= read -r module || [[ -n "$module" ]]; do
    [[ -n "$module" ]] || continue
    source_file="$(sley_source_file_for_module "$file" "$module" 2>/dev/null)" || continue
    [[ -f "$source_file" ]] || continue
    if awk -v task="$task" '$0 ~ "^[ \t]*export[ \t]+task[ \t]+" task "([ \t({]|$)" {found=1; exit} END {exit found ? 0 : 1}' "$source_file"; then
      candidate_file="$source_file"
      candidate_count=$((candidate_count + 1))
    fi
  done < <(awk '
    function trim(s){gsub(/^[ \t\r\n]+|[ \t\r\n]+$/, "", s); return s}
    /^[ \t]*import[ \t]+/ {
      line=$0
      sub(/^[ \t]*import[ \t]+/, "", line)
      sub(/[ \t]+as[ \t]+.*$/, "", line)
      print trim(line)
    }
  ' "$file")
  [[ "$candidate_count" -eq 1 && -n "$candidate_file" ]] || return 1
  printf '%s\n' "$candidate_file"
}

module_source_file() {
  local module="$1"
  printf '%s/%s.sley\n' "$SELF_HOSTED_SOURCE_ROOT" "${module//.//}"
}

sley_project_module_source_file() {
  local file="$1" module="$2" dir root_rel candidate
  [[ -f "$file" && -n "$module" ]] || return 1
  dir="$(cd "$(dirname "$file")" && pwd)" || return 1
  while [[ "$dir" != "/" ]]; do
    if [[ -f "$dir/sley.toml" ]]; then
      root_rel="$(awk -F= '/^[[:space:]]*root[[:space:]]*=/ {gsub(/^[ \t"]+|[ \t"]+$/, "", $2); print $2; exit}' "$dir/sley.toml")"
      [[ -n "$root_rel" ]] || root_rel="src"
      candidate="$dir/$root_rel/${module//.//}.sley"
      if [[ -f "$candidate" ]]; then
        printf '%s\n' "$candidate"
        return
      fi
    fi
    dir="$(dirname "$dir")"
  done
  return 1
}

sley_source_file_for_module() {
  local current_file="$1" module="$2" source_file
  source_file="$(module_source_file "$module")"
  if [[ -f "$source_file" ]]; then
    printf '%s\n' "$source_file"
    return
  fi
  sley_project_module_source_file "$current_file" "$module"
}

sley_source_entry_file_for_target() {
  local target="$1" entry="" root_rel="" candidate
  if [[ -d "$target" && -f "$target/sley.toml" ]]; then
    entry="$(awk -F= '/^[[:space:]]*entry[[:space:]]*=/ {gsub(/^[ \t"]+|[ \t"]+$/, "", $2); print $2; exit}' "$target/sley.toml")"
    root_rel="$(awk -F= '/^[[:space:]]*root[[:space:]]*=/ {gsub(/^[ \t"]+|[ \t"]+$/, "", $2); print $2; exit}' "$target/sley.toml")"
    [[ -n "$root_rel" ]] || root_rel="src"
    if [[ -n "$entry" ]]; then
      candidate="$target/$root_rel/${entry//.//}.sley"
      if [[ -f "$candidate" ]]; then
        printf '%s\n' "$candidate"
        return
      fi
    fi
  fi
  collect_files "$target" | head -n 1
}

sley_eval_env_lookup() {
  local env_json="$1" name="$2"
  printf '%s\n' "$env_json" | jq -er --arg name "$name" '
    if type == "object" and has($name) then .[$name] | tostring else empty end
  '
}

sley_eval_env_index() {
  local env_json="$1" base="$2" key="$3"
  printf '%s\n' "$env_json" | jq -er --arg base "$base" --arg key "$key" '
    def scalar:
      if type == "string" or type == "number" or type == "boolean" then tostring else empty end;
    if type == "object" and has($base) then
      .[$base] as $value |
      if ($value | type) == "array" then
        ($key | tonumber? // empty) as $index |
        if ($index < 0 or $index >= ($value | length) or (($index | floor) != $index)) then empty else $value[$index] | scalar end
      elif ($value | type) == "object" then
        if ($value | has($key)) then $value[$key] | scalar else empty end
      else
        empty
      end
    else
      empty
    end
  '
}

sley_eval_env_field() {
  local env_json="$1" base="$2" field="$3"
  printf '%s\n' "$env_json" | jq -er --arg base "$base" --arg field "$field" '
    def scalar:
      if type == "string" or type == "number" or type == "boolean" then tostring else empty end;
    if type == "object" and has($base) and (.[$base] | type) == "object" and (.[$base] | has($field)) then
      .[$base][$field] | scalar
    else
      empty
    end
  '
}

sley_eval_source_task_takes_json() {
  local body="$1"
  printf '%s\n' "$body" | awk '
    function esc(s){gsub(/\\/, "\\\\", s); gsub(/"/, "\\\"", s); return s}
    BEGIN {printf "["; first=1}
    /^[ \t]*take[ \t]+gate[ \t]+/ {next}
    /^[ \t]*take[ \t]+[A-Za-z_][A-Za-z0-9_]*[ \t]*:/ {
      line=$0
      sub(/^[ \t]*take[ \t]+/, "", line)
      sub(/[ \t]*:.*/, "", line)
      if (!first) printf ","
      printf "\"%s\"", esc(line)
      first=0
    }
    END {print "]"}
  '
}

sley_eval_source_static_scalar() {
  local body="$1"
  local line=""
  while IFS= read -r raw || [[ -n "$raw" ]]; do
    raw="$(printf '%s\n' "$raw" | sed 's/^[[:space:]]*//;s/[[:space:]]*$//')"
    [[ -n "$raw" ]] || continue
    [[ -z "$line" ]] || return 1
    line="$raw"
  done <<< "$body"
  [[ -n "$line" ]] || return 1
  if [[ "$line" =~ ^return[[:space:]]+\"([^\"]*)\"$ ]]; then
    printf '%s\n' "${BASH_REMATCH[1]}"
    return 0
  fi
  if [[ "$line" =~ ^return[[:space:]]+(-?[0-9]+)$ ]]; then
    printf '%s\n' "${BASH_REMATCH[1]}"
    return 0
  fi
  if [[ "$line" =~ ^return[[:space:]]+(true|false)$ ]]; then
    printf '%s\n' "${BASH_REMATCH[1]}"
    return 0
  fi
  return 1
}

sley_eval_source_static_list_json() {
  local body="$1"
  local line in_list=false expr="" saw_return=false
  while IFS= read -r line || [[ -n "$line" ]]; do
    line="$(printf '%s\n' "$line" | sed 's/^[[:space:]]*//;s/[[:space:]]*$//')"
    [[ -n "$line" ]] || continue
    if [[ "$saw_return" == false ]]; then
      if [[ "$line" =~ ^return[[:space:]]+(\[.*\])$ ]]; then
        sley_eval_source_list_literal_json "${BASH_REMATCH[1]}"
        return
      fi
      if [[ "$line" == "return [" ]]; then
        saw_return=true
        in_list=true
        expr="["
        continue
      fi
      return 1
    fi
    [[ "$in_list" == true ]] || return 1
    expr+=$'\n'"$line"
    if [[ "$line" == "]" ]]; then
      sley_eval_source_list_literal_json "$expr"
      return
    fi
    [[ "$line" =~ ^\"[^\"]*\",?$ ]] || return 1
  done <<< "$body"
  return 1
}

sley_split_source_args() {
  local args="$1"
  awk '
    function emit() {
      gsub(/^[ \t]+|[ \t]+$/, "", arg)
      if (arg != "") print arg
      arg=""
    }
    {
      for (i = 1; i <= length($0); i++) {
        c = substr($0, i, 1)
        if (escape) {
          arg = arg c
          escape = 0
          continue
        }
        if (c == "\\") {
          arg = arg c
          if (in_string) escape = 1
          continue
        }
        if (c == "\"") {
          in_string = !in_string
          arg = arg c
          continue
        }
        if (!in_string) {
          if (c == "(" || c == "[" || c == "{") depth++
          if (c == ")" || c == "]" || c == "}") depth--
          if (c == "," && depth == 0) {
            emit()
            continue
          }
        }
        arg = arg c
      }
    }
    END {emit()}
  ' <<< "$args"
}

sley_eval_source_args_json() {
  local file="$1" env_json="$2" args_text="$3"
  local args_json='[]' arg value_json
  args_text="$(printf '%s\n' "$args_text" | sed 's/^[[:space:]]*//;s/[[:space:]]*$//')"
  [[ -n "$args_text" ]] || {
    printf '[]\n'
    return 0
  }
  while IFS= read -r arg || [[ -n "$arg" ]]; do
    [[ -n "$arg" ]] || continue
    value_json="$(sley_eval_source_arg_value_json "$file" "$env_json" "$arg")" || return 1
    args_json="$(printf '%s\n' "$args_json" | jq -c --argjson value "$value_json" '. + [$value]')" || return 1
  done < <(sley_split_source_args "$args_text")
  printf '%s\n' "$args_json"
}

sley_eval_source_arg_value_json() {
  local file="$1" env_json="$2" arg="$3" value
  arg="$(printf '%s\n' "$arg" | sed 's/^[[:space:]]*//;s/[[:space:]]*$//')"
  if [[ "$arg" =~ ^[A-Za-z_][A-Za-z0-9_]*$ ]] && printf '%s\n' "$env_json" | jq -e --arg name "$arg" 'type == "object" and has($name) and ((.[$name] | type) == "array" or (.[$name] | type) == "object")' >/dev/null; then
    printf '%s\n' "$env_json" | jq -cer --arg name "$arg" '.[$name]'
    return
  fi
  value="$(sley_eval_source_expr "$file" "$env_json" "$arg")" || return 1
  jq -cn --arg value "$value" '$value'
}

sley_eval_source_call_env_json() {
  local base_env_json="$1" takes_json="$2" args_json="$3"
  jq -cn --argjson base "$base_env_json" --argjson takes "$takes_json" --argjson args "$args_json" '
    if ($takes | length) != ($args | length) then
      empty
    else
      reduce range(0; $takes | length) as $i ($base; . + {($takes[$i]): $args[$i]})
    end
  '
}

sley_runtime_seed_env_json() {
  local http_text="$1" model_text="$2" deploy_text="$3" spend_text="$4" secret_text="$5" shell_text="$6"
  jq -cn \
    --arg http_text "${http_text:-$RUNTIME_DEFAULT_PROFILE}" \
    --arg model_text "${model_text:-$RUNTIME_DEFAULT_MODEL_PLAN}" \
    --arg deploy_text "${deploy_text:-$RUNTIME_DEFAULT_DEPLOY_RESULT}" \
    --arg spend_text "${spend_text:-$RUNTIME_DEFAULT_SPEND_AUTHORIZATION_TEXT}" \
    --arg secret_text "${secret_text:-$RUNTIME_DEFAULT_SECRET_TEXT}" \
    --arg shell_text "${shell_text:-$RUNTIME_DEFAULT_SHELL_TEXT}" \
    --arg database_read_text "$RUNTIME_DEFAULT_DATABASE_READ_TEXT" \
    '{
      __runtime_http_text:$http_text,
      __runtime_model_text:$model_text,
      __runtime_deploy_text:$deploy_text,
      __runtime_spend_text:$spend_text,
      __runtime_secret_text:$secret_text,
      __runtime_shell_text:$shell_text,
      __runtime_database_read_text:$database_read_text
    }'
}

sley_eval_source_host_call() {
  local env_json="$1" callee="$2" args_json="${3:-[]}" key
  case "$callee" in
    http.try_get_text) key="__runtime_http_text" ;;
    shell.try_run) key="__runtime_shell_text" ;;
    model.try_complete) key="__runtime_model_text" ;;
    secrets.try_get) key="__runtime_secret_text" ;;
    deploy.try_stage) key="__runtime_deploy_text" ;;
    spend.try_authorize) key="__runtime_spend_text" ;;
    *) return 1 ;;
  esac
  [[ -n "$args_json" ]] || return 1
  sley_eval_env_lookup "$env_json" "$key"
}

sley_eval_source_host_expr() {
  local file="$1" env_json="$2" expr="$3" path
  expr="$(printf '%s\n' "$expr" | sed 's/^[[:space:]]*//;s/[[:space:]]*$//')"
  if [[ "$expr" =~ ^fs[.]try_read_text\(\"([^\"]+)\"\)$ ]]; then
    runtime_file_read_seed_text "${BASH_REMATCH[1]}"
    return
  fi
  if [[ "$expr" =~ ^fs[.]read_text\(\"([^\"]+)\"\)$ ]]; then
    runtime_file_read_seed_text "${BASH_REMATCH[1]}"
    return
  fi
  if [[ "$expr" =~ ^fs[.]try_read_text\(([A-Za-z_][A-Za-z0-9_]*)\)$ ]]; then
    path="$(sley_eval_env_lookup "$env_json" "${BASH_REMATCH[1]}")" || return 1
    runtime_file_read_seed_text "$path"
    return
  fi
  if [[ "$expr" =~ ^fs[.]read_text\(([A-Za-z_][A-Za-z0-9_]*)\)$ ]]; then
    path="$(sley_eval_env_lookup "$env_json" "${BASH_REMATCH[1]}")" || return 1
    runtime_file_read_seed_text "$path"
    return
  fi
  return 1
}

sley_eval_source_row_text_value() {
  local env_json="$1" row="$2" field="$3"
  printf '%s\n' "$env_json" | jq -er --arg row "$row" --arg field "$field" '
    if type == "object" and has($row) and (.[$row] | type) == "object" and (.[$row] | has($field)) then
      .[$row][$field] | tostring
    else
      empty
    end
  '
}

sley_eval_source_database_default_row_json() {
  local env_json="$1" name
  name="$(sley_eval_env_lookup "$env_json" "__runtime_database_read_text")" || return 1
  jq -cn --arg id "u1" --arg name "$name" --arg email "ada@example.test" '{id:$id,name:$name,email:$email,text:$name}'
}

sley_eval_source_database_bind_value_json() {
  local env_json="$1" expr="$2" call_prefix="${PARSER_CALL_EXPRESSION_PREFIX-}" tail fields id="" name="" row_name
  expr="$(printf '%s\n' "$expr" | sed 's/^[[:space:]]*//;s/[[:space:]]*$//')"
  tail="$expr"
  if [[ -n "$call_prefix" && "$tail" == "$call_prefix"* ]]; then
    tail="${tail#"$call_prefix"}"
  fi
  if [[ "$tail" =~ ^db[.]try_insert\(\"[^\"]+\",[[:space:]]*\{(.*)\}\)$ ]]; then
    fields="${BASH_REMATCH[1]}"
    if [[ "$fields" =~ id:[[:space:]]*\"([^\"]+)\" ]]; then
      id="${BASH_REMATCH[1]}"
    fi
    if [[ "$fields" =~ name:[[:space:]]*\"([^\"]+)\" ]]; then
      name="${BASH_REMATCH[1]}"
    elif [[ "$fields" =~ name:[[:space:]]*([A-Za-z_][A-Za-z0-9_]*)[.]text\(\"([^\"]+)\"\) ]]; then
      name="$(sley_eval_source_row_text_value "$env_json" "${BASH_REMATCH[1]}" "${BASH_REMATCH[2]}")" || return 1
    fi
    [[ -n "$id" ]] || id="u1"
    [[ -n "$name" ]] || return 1
    jq -cn --arg id "$id" --arg name "$name" '{id:$id,name:$name,text:$name}'
    return
  fi
  if [[ "$tail" =~ ^db[.]try_query_one\( || "$tail" =~ ^db[.]query_one\( ]]; then
    row_name="$(printf '%s\n' "$tail" | sed -n 's/.*[^A-Za-z0-9_]\([A-Za-z_][A-Za-z0-9_]*\)[.]text("id").*/\1/p' | head -n 1)"
    if [[ -n "$row_name" ]] && printf '%s\n' "$env_json" | jq -e --arg row "$row_name" 'type == "object" and has($row) and (.[$row] | type) == "object"' >/dev/null; then
      printf '%s\n' "$env_json" | jq -cer --arg row "$row_name" '.[$row]'
      return
    fi
    sley_eval_source_database_default_row_json "$env_json"
    return
  fi
  return 1
}

sley_select_source_return_list_expr() {
  local body="$1"
  printf '%s\n' "$body" | awk '
    /^[ \t]*return[ \t]+\[[ \t]*$/ {in_list=1; print "["; next}
    in_list {
      print
      if ($0 ~ /^[ \t]*\][ \t]*$/) exit
    }
  '
}

sley_eval_source_list_literal_json() {
  local expr="$1"
  printf '%s\n' "$expr" | awk '
    function esc(s){gsub(/\\/, "\\\\", s); gsub(/"/, "\\\"", s); return s}
    BEGIN {printf "["; first=1}
    {
      line=$0
      while (match(line, /"[^"]+"/)) {
        item=substr(line, RSTART + 1, RLENGTH - 2)
        if (!first) printf ","
        printf "\"%s\"", esc(item)
        first=0
        line=substr(line, RSTART + RLENGTH)
      }
    }
    END {print "]"}
  '
}

sley_eval_source_task_list_json() {
  local file="$1" task="$2" env_json="${3:-}"
  local body expr list_json cache_path
  [[ -n "$env_json" ]] || env_json='{}'
  cache_path="$(sley_source_cache_path list "$file" "$task" "$env_json")"
  if sley_source_cache_read "$cache_path"; then
    return
  fi
  body="$(extract_sley_task_body "$file" "$task")"
  [[ -n "$body" ]] || return 1
  if list_json="$(sley_eval_source_static_list_json "$body" 2>/dev/null)"; then
    printf '%s\n' "$list_json"
    sley_source_cache_write "$cache_path" "$list_json"
    return
  fi
  env_json="$(sley_eval_source_bind_env_json "$file" "$env_json" "$body")" || return 1
  expr="$(sley_select_source_return_expr "$body" "$env_json")" || return 1
  if [[ "$expr" == "[" ]]; then
    expr="$(sley_select_source_return_list_expr "$body")" || return 1
  fi
  list_json="$(sley_eval_source_list_expr_json "$file" "$env_json" "$expr")" || return 1
  printf '%s\n' "$list_json"
  sley_source_cache_write "$cache_path" "$list_json"
}

sley_eval_source_list_expr_json() {
  local file="$1" env_json="$2" expr="$3"
  local call_prefix="${PARSER_CALL_EXPRESSION_PREFIX-}" tail callee args_text args_json
  expr="$(printf '%s\n' "$expr" | sed 's/^[[:space:]]*//;s/[[:space:]]*$//')"
  if [[ "$expr" == \[* ]]; then
    sley_eval_source_list_literal_json "$expr"
    return
  fi
  if [[ -n "$call_prefix" && "$expr" == "$call_prefix"* ]]; then
    tail="${expr#"$call_prefix"}"
    if [[ "$tail" =~ ^([A-Za-z_][A-Za-z0-9_.]*)\((.*)\)$ ]]; then
      callee="${BASH_REMATCH[1]}"
      args_text="${BASH_REMATCH[2]}"
      args_json="$(sley_eval_source_args_json "$file" "$env_json" "$args_text")" || return 1
      sley_eval_source_call_list_json "$file" "$callee" "$env_json" "$args_json"
      return
    fi
  fi
  if [[ "$expr" =~ ^[A-Za-z_][A-Za-z0-9_]*$ ]]; then
    printf '%s\n' "$env_json" | jq -cer --arg name "$expr" '
      if type == "object" and has($name) and (.[$name] | type) == "array" then .[$name] else empty end
    '
    return
  fi
  return 1
}

sley_eval_source_call_list_json() {
  local file="$1" callee="$2" env_json="$3" args_json="${4:-[]}"
  local alias task module source_file body imported_file takes_json call_env_json
  if [[ "$callee" == *.* ]]; then
    alias="${callee%%.*}"
    task="${callee#*.}"
    module="$(resolve_sley_import_alias "$file" "$alias")"
    [[ -n "$module" ]] || return 1
    source_file="$(sley_source_file_for_module "$file" "$module")"
  else
    task="$callee"
    source_file="$file"
  fi
  body="$(extract_sley_task_body "$source_file" "$task")"
  if [[ -z "$body" && "$callee" != *.* ]]; then
    imported_file="$(resolve_sley_unqualified_imported_task_file "$file" "$callee")" || return 1
    source_file="$imported_file"
    body="$(extract_sley_task_body "$source_file" "$task")"
  fi
  [[ -n "$body" ]] || return 1
  takes_json="$(sley_eval_source_task_takes_json "$body")" || return 1
  call_env_json="$(sley_eval_source_call_env_json "$env_json" "$takes_json" "$args_json")" || return 1
  [[ -n "$call_env_json" ]] || return 1
  sley_eval_source_task_list_json "$source_file" "$task" "$call_env_json"
}

sley_eval_source_bind_value_json() {
  local file="$1" env_json="$2" expr="$3"
  local call_prefix="${PARSER_CALL_EXPRESSION_PREFIX-}" tail callee args_text args_json list_json value value_json literal_body
  expr="$(printf '%s\n' "$expr" | sed 's/^[[:space:]]*//;s/[[:space:]]*$//')"
  if [[ "$expr" == *\? ]]; then
    expr="${expr%\?}"
    expr="$(printf '%s\n' "$expr" | sed 's/^[[:space:]]*//;s/[[:space:]]*$//')"
  fi
  if [[ "$expr" == \[*\] ]]; then
    printf '%s\n' "$expr" | jq -ce .
    return
  fi
  if [[ "$expr" == map[[:space:]]*\{* && "$expr" == *\} ]]; then
    literal_body="${expr#map}"
    literal_body="$(printf '%s\n' "$literal_body" | sed 's/^[[:space:]]*//;s/[[:space:]]*$//')"
    literal_body="${literal_body#\{}"
    literal_body="${literal_body%\}}"
    literal_body="$(printf '%s\n' "$literal_body" | awk '
      { lines[NR]=$0 }
      END {
        last=NR
        while(last > 0 && lines[last] ~ /^[ \t]*$/) last--
        sub(/,[ \t]*$/, "", lines[last])
        for(i=1; i<=last; i++) print lines[i]
      }
    ')"
    printf '{%s}\n' "$literal_body" | jq -ce .
    return
  fi
  if [[ "$expr" =~ ^[A-Za-z_][A-Za-z0-9_]*[[:space:]]*\{(.*)\}$ ]]; then
    sley_runtime_text_record_json "${BASH_REMATCH[1]}"
    return
  fi
  if value_json="$(sley_eval_source_database_bind_value_json "$env_json" "$expr" 2>/dev/null)"; then
    printf '%s\n' "$value_json"
    return
  fi
  if [[ -n "$call_prefix" && "$expr" == "$call_prefix"* ]]; then
    tail="${expr#"$call_prefix"}"
    if [[ "$tail" =~ ^([A-Za-z_][A-Za-z0-9_.]*)\((.*)\)$ ]]; then
      callee="${BASH_REMATCH[1]}"
      args_text="${BASH_REMATCH[2]}"
      args_json="$(sley_eval_source_args_json "$file" "$env_json" "$args_text")" || return 1
      if list_json="$(sley_eval_source_call_list_json "$file" "$callee" "$env_json" "$args_json" 2>/dev/null)"; then
        printf '%s\n' "$list_json"
        return 0
      fi
    fi
  fi
  value="$(sley_eval_source_expr "$file" "$env_json" "$expr")" || return 1
  jq -cn --arg value "$value" '$value'
}

sley_eval_source_apply_assignment_json() {
  local file="$1" env_json="$2" line="$3"
  local name expr value_json
  line="$(printf '%s\n' "$line" | sed 's/^[[:space:]]*//;s/[[:space:]]*$//')"
  [[ "$line" =~ ^(bind|state|tally|set)[[:space:]]+([A-Za-z_][A-Za-z0-9_]*)([[:space:]]*:[^=]+)?[[:space:]]*=[[:space:]]*(.+)$ ]] || {
    printf '%s\n' "$env_json"
    return
  }
  name="${BASH_REMATCH[2]}"
  expr="${BASH_REMATCH[4]}"
  value_json="$(sley_eval_source_bind_value_json "$file" "$env_json" "$expr")" || return 1
  jq -cn --argjson env "$env_json" --arg name "$name" --argjson value "$value_json" '$env + {($name): $value}'
}

sley_eval_source_each_env_json() {
  local file="$1" env_json="$2" loop_var="$3" collection_expr="$4" body="$5"
  local collection_json item_json body_line
  collection_expr="$(printf '%s\n' "$collection_expr" | sed 's/^[[:space:]]*//;s/[[:space:]]*$//')"
  if collection_json="$(printf '%s\n' "$env_json" | jq -ce --arg name "$collection_expr" '
    if type == "object" and has($name) and (.[$name] | type) == "array" then .[$name] else empty end
  ' 2>/dev/null)"; then
    :
  else
    collection_json="$(sley_eval_source_bind_value_json "$file" "$env_json" "$collection_expr")" || return 1
    printf '%s\n' "$collection_json" | jq -e 'type == "array"' >/dev/null || return 1
  fi
  while IFS= read -r item_json || [[ -n "$item_json" ]]; do
    [[ -n "$item_json" ]] || continue
    env_json="$(jq -cn --argjson env "$env_json" --arg name "$loop_var" --argjson value "$item_json" '$env + {($name): $value}')" || return 1
    while IFS= read -r body_line || [[ -n "$body_line" ]]; do
      [[ -n "$body_line" ]] || continue
      env_json="$(sley_eval_source_apply_assignment_json "$file" "$env_json" "$body_line")" || return 1
    done <<< "$body"
  done < <(printf '%s\n' "$collection_json" | jq -ce '.[]')
  printf '%s\n' "$env_json"
}

sley_eval_source_while_env_json() {
  local file="$1" env_json="$2" condition="$3" body="$4"
  local condition_value body_line next_env_json iterations=0
  [[ -n "$(printf '%s\n' "$body" | sed '/^[[:space:]]*$/d')" ]] || return 1
  while :; do
    condition_value="$(sley_eval_source_expr "$file" "$env_json" "$condition")" || return 1
    [[ "$condition_value" == "true" || "$condition_value" == "false" ]] || return 1
    [[ "$condition_value" == "true" ]] || break
    iterations=$((iterations + 1))
    [[ "$iterations" -le 1000 ]] || return 1
    next_env_json="$env_json"
    while IFS= read -r body_line || [[ -n "$body_line" ]]; do
      [[ -n "$body_line" ]] || continue
      next_env_json="$(sley_eval_source_apply_assignment_json "$file" "$next_env_json" "$body_line")" || return 1
    done <<< "$body"
    [[ "$next_env_json" != "$env_json" ]] || return 1
    env_json="$next_env_json"
  done
  printf '%s\n' "$env_json"
}

sley_eval_source_bind_env_json() {
  local file="$1" env_json="$2" body="$3"
  local raw line name expr value_json loop_var list_name loop_body condition then_body else_body condition_value active_body in_else
  while IFS= read -r raw || [[ -n "$raw" ]]; do
    line="$(printf '%s\n' "$raw" | sed 's/^[[:space:]]*//;s/[[:space:]]*$//')"
    if [[ "$line" =~ ^if[[:space:]]+(.+)[[:space:]]*\{[[:space:]]*$ ]]; then
      condition="${BASH_REMATCH[1]}"
      then_body=""
      else_body=""
      in_else=false
      while IFS= read -r raw || [[ -n "$raw" ]]; do
        line="$(printf '%s\n' "$raw" | sed 's/^[[:space:]]*//;s/[[:space:]]*$//')"
        if [[ "$line" =~ ^\}[[:space:]]*else[[:space:]]*\{[[:space:]]*$ ]]; then
          in_else=true
          continue
        fi
        [[ "$line" == "}" ]] && break
        if "$in_else"; then
          else_body+="$line"$'\n'
        else
          then_body+="$line"$'\n'
        fi
      done
      if condition_value="$(sley_eval_source_expr "$file" "$env_json" "$condition" 2>/dev/null)"; then
        :
      elif sley_eval_source_condition "$env_json" "$condition"; then
        condition_value="true"
      else
        condition_value="false"
      fi
      [[ "$condition_value" == "true" || "$condition_value" == "false" ]] || return 1
      if [[ "$condition_value" == "true" ]]; then
        active_body="$then_body"
      else
        active_body="$else_body"
      fi
      env_json="$(sley_eval_source_bind_env_json "$file" "$env_json" "$active_body")" || return 1
      continue
    fi
    if [[ "$line" =~ ^(each|for)[[:space:]]+([A-Za-z_][A-Za-z0-9_]*)[[:space:]]+in[[:space:]]+(.+)[[:space:]]*\{[[:space:]]*$ ]]; then
      loop_var="${BASH_REMATCH[2]}"
      list_name="${BASH_REMATCH[3]}"
      loop_body=""
      while IFS= read -r raw || [[ -n "$raw" ]]; do
        line="$(printf '%s\n' "$raw" | sed 's/^[[:space:]]*//;s/[[:space:]]*$//')"
        [[ "$line" == "}" ]] && break
        loop_body+="$line"$'\n'
      done
      env_json="$(sley_eval_source_each_env_json "$file" "$env_json" "$loop_var" "$list_name" "$loop_body")" || return 1
      continue
    fi
    if [[ "$line" =~ ^while[[:space:]]+(.+)[[:space:]]*\{[[:space:]]*$ ]]; then
      condition="${BASH_REMATCH[1]}"
      loop_body=""
      while IFS= read -r raw || [[ -n "$raw" ]]; do
        line="$(printf '%s\n' "$raw" | sed 's/^[[:space:]]*//;s/[[:space:]]*$//')"
        [[ "$line" == "}" ]] && break
        loop_body+="$line"$'\n'
      done
      env_json="$(sley_eval_source_while_env_json "$file" "$env_json" "$condition" "$loop_body")" || return 1
      continue
    fi
    [[ "$line" =~ ^(bind|state|tally|set)[[:space:]]+([A-Za-z_][A-Za-z0-9_]*)([[:space:]]*:[^=]+)?[[:space:]]*=[[:space:]]*(.+)$ ]] || continue
    name="${BASH_REMATCH[2]}"
    expr="${BASH_REMATCH[4]}"
    if [[ "$expr" == map[[:space:]]*\{ && "$expr" != *\} ]]; then
      while IFS= read -r raw || [[ -n "$raw" ]]; do
        line="$(printf '%s\n' "$raw" | sed 's/^[[:space:]]*//;s/[[:space:]]*$//')"
        expr+=$'\n'"$line"
        [[ "$line" == "}" ]] && break
      done
    fi
    value_json="$(sley_eval_source_bind_value_json "$file" "$env_json" "$expr")" || return 1
    env_json="$(jq -cn --argjson env "$env_json" --arg name "$name" --argjson value "$value_json" '$env + {($name): $value}')" || return 1
  done <<< "$body"
  printf '%s\n' "$env_json"
}

sley_eval_source_condition() {
  local env_json="$1" condition="$2"
  local name op right left
  condition="$(printf '%s\n' "$condition" | sed 's/^[[:space:]]*//;s/[[:space:]]*$//')"
  if [[ "$condition" == "true" ]]; then
    return 0
  fi
  if [[ "$condition" == "false" ]]; then
    return 1
  fi
  if [[ "$condition" =~ ^([A-Za-z_][A-Za-z0-9_]*)$ ]]; then
    left="$(sley_eval_env_lookup "$env_json" "${BASH_REMATCH[1]}")" || return 1
    [[ "$left" == "true" ]]
    return
  fi
  if [[ "$condition" =~ ^len\(([A-Za-z_][A-Za-z0-9_]*)\)[[:space:]]*(>=|<=|==|!=|\>|<)[[:space:]]*(-?[0-9]+)$ ]]; then
    name="${BASH_REMATCH[1]}"
    op="${BASH_REMATCH[2]}"
    right="${BASH_REMATCH[3]}"
    left="$(printf '%s\n' "$env_json" | jq -er --arg name "$name" '
      if type == "object" and has($name) and ((.[$name] | type) == "string" or (.[$name] | type) == "array" or (.[$name] | type) == "object") then
        .[$name] | length
      else
        empty
      end
    ')" || return 1
    case "$op" in
      "==") [[ "$left" -eq "$right" ]] ;;
      "!=") [[ "$left" -ne "$right" ]] ;;
      ">") [[ "$left" -gt "$right" ]] ;;
      "<") [[ "$left" -lt "$right" ]] ;;
      ">=") [[ "$left" -ge "$right" ]] ;;
      "<=") [[ "$left" -le "$right" ]] ;;
      *) return 1 ;;
    esac
    return
  fi
  [[ "$condition" =~ ^([A-Za-z_][A-Za-z0-9_]*)[[:space:]]*(>=|<=|==|!=|\>|<)[[:space:]]*(.+)$ ]] || return 1
  name="${BASH_REMATCH[1]}"
  op="${BASH_REMATCH[2]}"
  right="$(printf '%s\n' "${BASH_REMATCH[3]}" | sed 's/^[[:space:]]*//;s/[[:space:]]*$//')"
  left="$(sley_eval_env_lookup "$env_json" "$name")" || return 1
  if [[ "$right" =~ ^\"([^\"]*)\"$ ]]; then
    right="${BASH_REMATCH[1]}"
  elif [[ "$right" =~ ^[A-Za-z_][A-Za-z0-9_]*$ ]]; then
    right="$(sley_eval_env_lookup "$env_json" "$right" 2>/dev/null || printf '%s\n' "$right")"
  fi
  case "$op" in
    "==") [[ "$left" == "$right" ]] ;;
    "!=") [[ "$left" != "$right" ]] ;;
    ">")
      [[ "$left" =~ ^-?[0-9]+$ && "$right" =~ ^-?[0-9]+$ ]] || return 1
      [[ "$left" -gt "$right" ]]
      ;;
    "<")
      [[ "$left" =~ ^-?[0-9]+$ && "$right" =~ ^-?[0-9]+$ ]] || return 1
      [[ "$left" -lt "$right" ]]
      ;;
    ">=")
      [[ "$left" =~ ^-?[0-9]+$ && "$right" =~ ^-?[0-9]+$ ]] || return 1
      [[ "$left" -ge "$right" ]]
      ;;
    "<=")
      [[ "$left" =~ ^-?[0-9]+$ && "$right" =~ ^-?[0-9]+$ ]] || return 1
      [[ "$left" -le "$right" ]]
      ;;
    *) return 1 ;;
  esac
}

sley_select_source_return_expr() {
  local body="$1" env_json="$2"
  local raw line in_if=false if_condition=false if_active=false skip_depth=0 delta
  while IFS= read -r raw || [[ -n "$raw" ]]; do
    line="$(printf '%s\n' "$raw" | sed 's/^[[:space:]]*//;s/[[:space:]]*$//')"
    [[ -n "$line" ]] || continue
    if [[ "$skip_depth" -gt 0 ]]; then
      delta="$(printf '%s\n' "$line" | awk '{print gsub(/{/,"{") - gsub(/}/,"}")}')"
      skip_depth=$((skip_depth + delta))
      continue
    fi
    if [[ "$line" =~ ^(each|for|while)[[:space:]].*\{[[:space:]]*$ ]]; then
      skip_depth=1
      continue
    fi
    if [[ "$line" =~ ^if[[:space:]]+(.+)[[:space:]]*\{$ ]]; then
      in_if=true
      if sley_eval_source_condition "$env_json" "${BASH_REMATCH[1]}"; then
        if_condition=true
        if_active=true
      else
        if_condition=false
        if_active=false
      fi
      continue
    fi
    if [[ "$in_if" == true && "$line" =~ ^\}[[:space:]]*else[[:space:]]*\{$ ]]; then
      if "$if_condition"; then
        if_active=false
      else
        if_active=true
      fi
      continue
    fi
    if [[ "$line" == "}" ]]; then
      if [[ "$in_if" == true ]]; then
        in_if=false
        if_condition=false
        if_active=false
        continue
      fi
    fi
    if [[ "$line" =~ ^return[[:space:]]+(.+)$ ]]; then
      if [[ "$in_if" != true || "$if_active" == true ]]; then
        printf '%s\n' "${BASH_REMATCH[1]}"
        return 0
      fi
    fi
  done <<< "$body"
  return 1
}

sley_eval_source_if_expr() {
  local file="$1" env_json="$2" expr="$3"
  local condition then_expr else_expr condition_value then_value else_value
  [[ "$expr" =~ ^if[[:space:]]+([^{}]+)[[:space:]]*\{[[:space:]]*([^{}]+)[[:space:]]*\}[[:space:]]*else[[:space:]]*\{[[:space:]]*([^{}]+)[[:space:]]*\}$ ]] || return 1
  condition="${BASH_REMATCH[1]}"
  then_expr="${BASH_REMATCH[2]}"
  else_expr="${BASH_REMATCH[3]}"
  condition_value="$(sley_eval_source_expr "$file" "$env_json" "$condition")" || return 1
  [[ "$condition_value" == "true" || "$condition_value" == "false" ]] || return 1
  then_value="$(sley_eval_source_expr "$file" "$env_json" "$then_expr")" || return 1
  else_value="$(sley_eval_source_expr "$file" "$env_json" "$else_expr")" || return 1
  if [[ "$file" == "$SELF_HOSTED_SOURCE_ROOT/loom/runtime.sley" ]]; then
    if [[ "$condition_value" == "true" ]]; then printf '%s\n' "$then_value"; else printf '%s\n' "$else_value"; fi
  elif [[ "$then_value" == "true" || "$then_value" == "false" || "$else_value" == "true" || "$else_value" == "false" ]]; then
    eval_runtime_if_value_task "Bool" "$condition_value" "$then_value" "$else_value"
  elif [[ "$then_value" =~ ^-?[0-9]+$ && "$else_value" =~ ^-?[0-9]+$ ]]; then
    eval_runtime_if_value_task "Int" "$condition_value" "$then_value" "$else_value"
  else
    eval_runtime_if_value_task "Text" "$condition_value" "$then_value" "$else_value"
  fi
}

sley_eval_source_call() {
  local file="$1" callee="$2" env_json="$3" args_json="${4:-[]}"
  local alias task module source_file body imported_file takes_json call_env_json value
  if value="$(sley_eval_source_host_call "$env_json" "$callee" "$args_json" 2>/dev/null)"; then
    printf '%s\n' "$value"
    return
  fi
  if [[ "$callee" == *.* ]]; then
    alias="${callee%%.*}"
    task="${callee#*.}"
    module="$(resolve_sley_import_alias "$file" "$alias")"
    [[ -n "$module" ]] || return 1
    source_file="$(sley_source_file_for_module "$file" "$module")"
  else
    task="$callee"
    source_file="$file"
  fi
  body="$(extract_sley_task_body "$source_file" "$task")"
  if [[ -z "$body" && "$callee" != *.* ]]; then
    imported_file="$(resolve_sley_unqualified_imported_task_file "$file" "$callee")" || return 1
    source_file="$imported_file"
    body="$(extract_sley_task_body "$source_file" "$task")"
  fi
  [[ -n "$body" ]] || return 1
  takes_json="$(sley_eval_source_task_takes_json "$body")" || return 1
  call_env_json="$(sley_eval_source_call_env_json "$env_json" "$takes_json" "$args_json")" || return 1
  [[ -n "$call_env_json" ]] || return 1
  sley_eval_source_task "$source_file" "$task" "$call_env_json"
}

sley_expr_wrapped_by_parens() {
  local expr="$1" len="${#1}" depth=0 in_string=false i ch
  (( len >= 2 )) || return 1
  [[ "${expr:0:1}" == "(" && "${expr: -1}" == ")" ]] || return 1
  for ((i = 0; i < len; i++)); do
    ch="${expr:i:1}"
    if [[ "$ch" == '"' ]]; then
      if [[ "$in_string" == true ]]; then in_string=false; else in_string=true; fi
      continue
    fi
    [[ "$in_string" == true ]] && continue
    case "$ch" in
      "(") ((depth += 1)) ;;
      ")")
        ((depth -= 1))
        (( depth >= 0 )) || return 1
        if (( depth == 0 && i < len - 1 )); then
          return 1
        fi
        ;;
      *) ;;
    esac
  done
  [[ "$in_string" == false && "$depth" -eq 0 ]]
}

sley_split_top_level_token() {
  local expr="$1" token="$2" len="${#1}" token_len="${#2}" paren_depth=0 bracket_depth=0 brace_depth=0 in_string=false i ch left right
  (( token_len > 0 && len >= token_len )) || return 1
  for ((i = 0; i <= len - token_len; i++)); do
    ch="${expr:i:1}"
    if [[ "$ch" == '"' ]]; then
      if [[ "$in_string" == true ]]; then in_string=false; else in_string=true; fi
      continue
    fi
    if [[ "$in_string" == false && "$paren_depth" -eq 0 && "$bracket_depth" -eq 0 && "$brace_depth" -eq 0 && "${expr:i:token_len}" == "$token" ]]; then
      left="${expr:0:i}"
      right="${expr:i+token_len}"
      [[ -n "${left//[[:space:]]/}" && -n "${right//[[:space:]]/}" ]] || return 1
      printf '%s\t%s\n' "$left" "$right"
      return 0
    fi
    [[ "$in_string" == true ]] && continue
    case "$ch" in
      "(") ((paren_depth += 1)) ;;
      ")") ((paren_depth -= 1)); (( paren_depth >= 0 )) || return 1 ;;
      "[") ((bracket_depth += 1)) ;;
      "]") ((bracket_depth -= 1)); (( bracket_depth >= 0 )) || return 1 ;;
      "{") ((brace_depth += 1)) ;;
      "}") ((brace_depth -= 1)); (( brace_depth >= 0 )) || return 1 ;;
      *) ;;
    esac
  done
  return 1
}

sley_split_top_level_comparison() {
  local expr="$1" len="${#1}" paren_depth=0 bracket_depth=0 brace_depth=0 in_string=false i ch op token_len left right
  for ((i = 0; i < len; i++)); do
    ch="${expr:i:1}"
    if [[ "$ch" == '"' ]]; then
      if [[ "$in_string" == true ]]; then in_string=false; else in_string=true; fi
      continue
    fi
    if [[ "$in_string" == false && "$paren_depth" -eq 0 && "$bracket_depth" -eq 0 && "$brace_depth" -eq 0 ]]; then
      op=""
      token_len=0
      case "${expr:i:2}" in
        ">="|"<="|"=="|"!=")
          op="${expr:i:2}"
          token_len=2
          ;;
        *)
          case "$ch" in
            ">"|"<")
              op="$ch"
              token_len=1
              ;;
            *) ;;
          esac
          ;;
      esac
      if [[ -n "$op" ]]; then
        left="${expr:0:i}"
        right="${expr:i+token_len}"
        [[ -n "${left//[[:space:]]/}" && -n "${right//[[:space:]]/}" ]] || return 1
        printf '%s\t%s\t%s\n' "$left" "$op" "$right"
        return 0
      fi
    fi
    [[ "$in_string" == true ]] && continue
    case "$ch" in
      "(") ((paren_depth += 1)) ;;
      ")") ((paren_depth -= 1)); (( paren_depth >= 0 )) || return 1 ;;
      "[") ((bracket_depth += 1)) ;;
      "]") ((bracket_depth -= 1)); (( bracket_depth >= 0 )) || return 1 ;;
      "{") ((brace_depth += 1)) ;;
      "}") ((brace_depth -= 1)); (( brace_depth >= 0 )) || return 1 ;;
      *) ;;
    esac
  done
  return 1
}

sley_eval_source_comparison_value() {
  local file="$1" left="$2" op="$3" right="$4" value
  case "$op" in
    "==")
      if [[ "$file" == "$SELF_HOSTED_SOURCE_ROOT/loom/runtime.sley" ]]; then
        if [[ "$left" == "$right" ]]; then printf 'true\n'; else printf 'false\n'; fi
      elif [[ "$left" == "true" || "$left" == "false" || "$right" == "true" || "$right" == "false" ]]; then
        eval_runtime_equality_operator_value_task "Bool" "==" "$left" "$right"
      elif [[ "$left" =~ ^-?[0-9]+$ && "$right" =~ ^-?[0-9]+$ ]]; then
        eval_runtime_int_comparison_operator_value_task "$op" "$left" "$right"
      else
        eval_runtime_equality_operator_value_task "Text" "==" "$left" "$right"
      fi
      ;;
    "!=")
      if [[ "$file" == "$SELF_HOSTED_SOURCE_ROOT/loom/runtime.sley" ]]; then
        if [[ "$left" != "$right" ]]; then printf 'true\n'; else printf 'false\n'; fi
      elif [[ "$left" == "true" || "$left" == "false" || "$right" == "true" || "$right" == "false" ]]; then
        value="$(eval_runtime_equality_operator_value_task "Bool" "==" "$left" "$right")" || return 1
        eval_runtime_bool_unary_operator_value_task "!" "$value"
      elif [[ "$left" =~ ^-?[0-9]+$ && "$right" =~ ^-?[0-9]+$ ]]; then
        eval_runtime_int_comparison_operator_value_task "$op" "$left" "$right"
      else
        value="$(eval_runtime_equality_operator_value_task "Text" "==" "$left" "$right")" || return 1
        eval_runtime_bool_unary_operator_value_task "!" "$value"
      fi
      ;;
    ">")
      [[ "$left" =~ ^-?[0-9]+$ && "$right" =~ ^-?[0-9]+$ ]] || return 1
      if [[ "$file" == "$SELF_HOSTED_SOURCE_ROOT/loom/runtime.sley" ]]; then
        if [[ "$left" -gt "$right" ]]; then printf 'true\n'; else printf 'false\n'; fi
      else
        eval_runtime_int_comparison_operator_value_task "$op" "$left" "$right"
      fi
      ;;
    "<")
      [[ "$left" =~ ^-?[0-9]+$ && "$right" =~ ^-?[0-9]+$ ]] || return 1
      if [[ "$file" == "$SELF_HOSTED_SOURCE_ROOT/loom/runtime.sley" ]]; then
        if [[ "$left" -lt "$right" ]]; then printf 'true\n'; else printf 'false\n'; fi
      else
        eval_runtime_int_comparison_operator_value_task "$op" "$left" "$right"
      fi
      ;;
    ">=")
      [[ "$left" =~ ^-?[0-9]+$ && "$right" =~ ^-?[0-9]+$ ]] || return 1
      if [[ "$file" == "$SELF_HOSTED_SOURCE_ROOT/loom/runtime.sley" ]]; then
        if [[ "$left" -ge "$right" ]]; then printf 'true\n'; else printf 'false\n'; fi
      else
        eval_runtime_int_comparison_operator_value_task "$op" "$left" "$right"
      fi
      ;;
    "<=")
      [[ "$left" =~ ^-?[0-9]+$ && "$right" =~ ^-?[0-9]+$ ]] || return 1
      if [[ "$file" == "$SELF_HOSTED_SOURCE_ROOT/loom/runtime.sley" ]]; then
        if [[ "$left" -le "$right" ]]; then printf 'true\n'; else printf 'false\n'; fi
      else
        eval_runtime_int_comparison_operator_value_task "$op" "$left" "$right"
      fi
      ;;
    *) return 1 ;;
  esac
}

sley_eval_source_expr() {
  local file="$1" env_json="$2" expr="$3"
  local call_prefix="${PARSER_CALL_EXPRESSION_PREFIX-}" tail callee args_text args_json tokens token out="" seen=false string_seen=false value left right op split rest binary_re comparison_re indexed_re field_re
  expr="$(printf '%s\n' "$expr" | sed 's/^[[:space:]]*//;s/[[:space:]]*$//')"
  if [[ "$expr" =~ ^Ok\((.*)\)$ ]]; then
    sley_eval_source_expr "$file" "$env_json" "${BASH_REMATCH[1]}"
    return
  fi
  if [[ "$expr" =~ ^Err\(.*\)$ ]]; then
    return 1
  fi
  if value="$(sley_eval_source_host_expr "$file" "$env_json" "$expr" 2>/dev/null)"; then
    printf '%s\n' "$value"
    return
  fi
  if value="$(sley_eval_source_if_expr "$file" "$env_json" "$expr" 2>/dev/null)"; then
    printf '%s\n' "$value"
    return
  fi
  if sley_expr_wrapped_by_parens "$expr"; then
    sley_eval_source_expr "$file" "$env_json" "${expr:1:${#expr}-2}"
    return
  fi
  if [[ "$expr" =~ ^!(.+)$ ]]; then
    value="$(sley_eval_source_expr "$file" "$env_json" "${BASH_REMATCH[1]}")" || return 1
    [[ "$value" == "true" || "$value" == "false" ]] || return 1
    eval_runtime_bool_unary_operator_value_task "!" "$value"
    return
  fi
  if [[ -n "$call_prefix" && "$expr" == "$call_prefix"* ]]; then
    tail="${expr#"$call_prefix"}"
    if [[ "$tail" =~ ^([A-Za-z_][A-Za-z0-9_.]*)\((.*)\)$ ]]; then
      callee="${BASH_REMATCH[1]}"
      args_text="${BASH_REMATCH[2]}"
      args_json="$(sley_eval_source_args_json "$file" "$env_json" "$args_text")" || return 1
      sley_eval_source_call "$file" "$callee" "$env_json" "$args_json"
      return
    fi
  fi
  if [[ "$expr" =~ ^\"([^\"]*)\"$ ]]; then
    printf '%s\n' "${BASH_REMATCH[1]}"
    return 0
  fi
  if [[ "$expr" =~ ^len\(([A-Za-z_][A-Za-z0-9_]*)\)$ ]]; then
    printf '%s\n' "$env_json" | jq -er --arg name "${BASH_REMATCH[1]}" '
      if type == "object" and has($name) and (.[$name] | type) == "array" then .[$name] | length else empty end
    '
    return
  fi
  if [[ "$expr" =~ ^len\(\[([^\]]*)\]\)$ ]]; then
    value="$(printf '[%s]\n' "${BASH_REMATCH[1]}" | jq -er 'if type == "array" then length else empty end')" || return 1
    if [[ "$file" == "$SELF_HOSTED_SOURCE_ROOT/loom/runtime.sley" ]]; then
      printf '%s\n' "$value"
    else
      eval_runtime_list_len_value_task "$value"
    fi
    return
  fi
  if [[ "$expr" =~ ^([A-Za-z_][A-Za-z0-9_]*)[.]text\(\"([^\"]+)\"\)$ ]]; then
    sley_eval_source_row_text_value "$env_json" "${BASH_REMATCH[1]}" "${BASH_REMATCH[2]}"
    return
  fi
  if [[ "$expr" =~ ^\[([^\]]*)\]\[([0-9]+)\]$ ]]; then
    if value="$(eval_runtime_collection_index_operator_value_task "list" "Int" "${BASH_REMATCH[1]}" "${BASH_REMATCH[2]}" 2>/dev/null)"; then
      printf '%s\n' "$value"
      return
    fi
    eval_runtime_collection_index_operator_value_task "list" "Text" "${BASH_REMATCH[1]}" "${BASH_REMATCH[2]}"
    return
  fi
  if [[ "$expr" =~ ^map[[:space:]]*\{(.*)\}\[\"([^\"]+)\"\]$ ]]; then
    if value="$(eval_runtime_collection_index_operator_value_task "map" "Int" "${BASH_REMATCH[1]}" "${BASH_REMATCH[2]}" 2>/dev/null)"; then
      printf '%s\n' "$value"
      return
    fi
    eval_runtime_collection_index_operator_value_task "map" "Text" "${BASH_REMATCH[1]}" "${BASH_REMATCH[2]}"
    return
  fi
  if [[ "$expr" =~ ^[A-Za-z_][A-Za-z0-9_]*[[:space:]]*\{(.*)\}[.]([A-Za-z_][A-Za-z0-9_]*)$ ]]; then
    eval_runtime_record_field_access_value_task "Text" "${BASH_REMATCH[1]}" "${BASH_REMATCH[2]}"
    return
  fi
  if [[ "$expr" =~ ^-?[0-9]+$ || "$expr" == "true" || "$expr" == "false" ]]; then
    printf '%s\n' "$expr"
    return 0
  fi
  if split="$(sley_split_top_level_token "$expr" "||")"; then
    left="$(sley_eval_source_expr "$file" "$env_json" "${split%%$'\t'*}")" || return 1
    right="$(sley_eval_source_expr "$file" "$env_json" "${split#*$'\t'}")" || return 1
    [[ "$left" == "true" || "$left" == "false" ]] || return 1
    [[ "$right" == "true" || "$right" == "false" ]] || return 1
    if [[ "$file" == "$SELF_HOSTED_SOURCE_ROOT/loom/runtime.sley" ]]; then
      if [[ "$left" == "true" || "$right" == "true" ]]; then printf 'true\n'; else printf 'false\n'; fi
    else
      eval_runtime_bool_binary_operator_value_task "||" "$left" "$right"
    fi
    return
  fi
  if split="$(sley_split_top_level_token "$expr" "&&")"; then
    left="$(sley_eval_source_expr "$file" "$env_json" "${split%%$'\t'*}")" || return 1
    right="$(sley_eval_source_expr "$file" "$env_json" "${split#*$'\t'}")" || return 1
    [[ "$left" == "true" || "$left" == "false" ]] || return 1
    [[ "$right" == "true" || "$right" == "false" ]] || return 1
    if [[ "$file" == "$SELF_HOSTED_SOURCE_ROOT/loom/runtime.sley" ]]; then
      if [[ "$left" == "true" && "$right" == "true" ]]; then printf 'true\n'; else printf 'false\n'; fi
    else
      eval_runtime_bool_binary_operator_value_task "&&" "$left" "$right"
    fi
    return
  fi
  if split="$(sley_split_top_level_comparison "$expr")"; then
    left="$(sley_eval_source_expr "$file" "$env_json" "${split%%$'\t'*}")" || return 1
    rest="${split#*$'\t'}"
    op="${rest%%$'\t'*}"
    right="$(sley_eval_source_expr "$file" "$env_json" "${rest#*$'\t'}")" || return 1
    sley_eval_source_comparison_value "$file" "$left" "$op" "$right"
    return
  fi
  if split="$(sley_split_top_level_token "$expr" "+")"; then
    left="$(sley_eval_source_expr "$file" "$env_json" "${split%%$'\t'*}")" || return 1
    right="$(sley_eval_source_expr "$file" "$env_json" "${split#*$'\t'}")" || return 1
    if [[ "$left" =~ ^-?[0-9]+$ && "$right" =~ ^-?[0-9]+$ ]]; then
      if [[ "$file" == "$SELF_HOSTED_SOURCE_ROOT/loom/runtime.sley" ]]; then
        printf '%s\n' "$((left + right))"
      else
        eval_runtime_int_binary_operator_value_task "+" "$left" "$right"
      fi
    elif [[ "$file" == "$SELF_HOSTED_SOURCE_ROOT/loom/runtime.sley" ]]; then
      printf '%s%s\n' "$left" "$right"
    else
      eval_runtime_text_binary_operator_value_task "+" "$left" "$right"
    fi
    return
  fi
  if split="$(sley_split_top_level_token "$expr" "*")"; then
    left="$(sley_eval_source_expr "$file" "$env_json" "${split%%$'\t'*}")" || return 1
    right="$(sley_eval_source_expr "$file" "$env_json" "${split#*$'\t'}")" || return 1
    [[ "$left" =~ ^-?[0-9]+$ && "$right" =~ ^-?[0-9]+$ ]] || return 1
    if [[ "$file" == "$SELF_HOSTED_SOURCE_ROOT/loom/runtime.sley" ]]; then
      printf '%s\n' "$((left * right))"
    else
      eval_runtime_int_binary_operator_value_task "*" "$left" "$right"
    fi
    return
  fi
  comparison_re='^(.+)[[:space:]](>=|<=|==|!=|>|<)[[:space:]](.+)$'
  if [[ "$expr" =~ $comparison_re ]]; then
    left="$(sley_eval_source_expr "$file" "$env_json" "${BASH_REMATCH[1]}")" || return 1
    op="${BASH_REMATCH[2]}"
    right="$(sley_eval_source_expr "$file" "$env_json" "${BASH_REMATCH[3]}")" || return 1
    sley_eval_source_comparison_value "$file" "$left" "$op" "$right"
    return
  fi
  indexed_re='^([A-Za-z_][A-Za-z0-9_]*)\[(.+)\]$'
  if [[ "$expr" =~ $indexed_re ]]; then
    value="$(sley_eval_source_expr "$file" "$env_json" "${BASH_REMATCH[2]}")" || return 1
    sley_eval_env_index "$env_json" "${BASH_REMATCH[1]}" "$value"
    return
  fi
  field_re='^([A-Za-z_][A-Za-z0-9_]*)\.([A-Za-z_][A-Za-z0-9_]*)$'
  if [[ "$expr" =~ $field_re ]]; then
    sley_eval_env_field "$env_json" "${BASH_REMATCH[1]}" "${BASH_REMATCH[2]}"
    return
  fi
  binary_re='^("[^"]*"|-?[0-9]+|true|false|[A-Za-z_][A-Za-z0-9_]*)[[:space:]]*(==|!=|>=|<=|&&|\+|\*|>|<)[[:space:]]*("[^"]*"|-?[0-9]+|true|false|[A-Za-z_][A-Za-z0-9_]*)$'
  if [[ "$expr" =~ $binary_re ]]; then
    left="$(sley_eval_source_expr "$file" "$env_json" "${BASH_REMATCH[1]}")" || return 1
    op="${BASH_REMATCH[2]}"
    right="$(sley_eval_source_expr "$file" "$env_json" "${BASH_REMATCH[3]}")" || return 1
    case "$op" in
      "+")
        if [[ "$left" =~ ^-?[0-9]+$ && "$right" =~ ^-?[0-9]+$ ]]; then
          if [[ "$file" == "$SELF_HOSTED_SOURCE_ROOT/loom/runtime.sley" ]]; then
            printf '%s\n' "$((left + right))"
          else
            eval_runtime_int_binary_operator_value_task "+" "$left" "$right"
          fi
        else
          if [[ "$file" == "$SELF_HOSTED_SOURCE_ROOT/loom/runtime.sley" ]]; then
            printf '%s%s\n' "$left" "$right"
          else
            eval_runtime_text_binary_operator_value_task "+" "$left" "$right"
          fi
        fi
        ;;
      "*")
        [[ "$left" =~ ^-?[0-9]+$ && "$right" =~ ^-?[0-9]+$ ]] || return 1
        if [[ "$file" == "$SELF_HOSTED_SOURCE_ROOT/loom/runtime.sley" ]]; then
          printf '%s\n' "$((left * right))"
        else
          eval_runtime_int_binary_operator_value_task "*" "$left" "$right"
        fi
        ;;
      "=="|"!="|">"|"<"|">="|"<=")
        sley_eval_source_comparison_value "$file" "$left" "$op" "$right"
        ;;
      "&&")
        if [[ "$file" == "$SELF_HOSTED_SOURCE_ROOT/loom/runtime.sley" ]]; then
          [[ "$left" == "true" || "$left" == "false" ]] || return 1
          [[ "$right" == "true" || "$right" == "false" ]] || return 1
          if [[ "$left" == "true" && "$right" == "true" ]]; then printf 'true\n'; else printf 'false\n'; fi
        else
          eval_runtime_bool_binary_operator_value_task "&&" "$left" "$right"
        fi
        ;;
      *) return 1 ;;
    esac
    return 0
  fi
  if [[ "$expr" =~ ^[A-Za-z_][A-Za-z0-9_]*$ ]]; then
    sley_eval_env_lookup "$env_json" "$expr"
    return
  fi
  tokens="$(printf '%s\n' "$expr" | awk -v call_prefix="$call_prefix" '
    function ltrim(s){sub(/^[ \t]+/, "", s); return s}
    {
      line=$0
      while (length(line) > 0) {
        line=ltrim(line)
        if (line == "") break
        if (substr(line, 1, 1) == "+") {
          line=substr(line, 2)
          continue
        }
        if (call_prefix != "" && index(line, call_prefix) == 1) {
          tail=substr(line, length(call_prefix) + 1)
          if (match(tail, /^[A-Za-z_][A-Za-z0-9_.]*\(\)/)) {
            callee=substr(tail, RSTART, RLENGTH)
            sub(/\(\)$/, "", callee)
            print "CALL:" callee
            line=substr(tail, RSTART + RLENGTH)
            continue
          }
        }
        if (match(line, /^"[^"]*"/)) {
          print "STRING:" substr(line, RSTART + 1, RLENGTH - 2)
          line=substr(line, RSTART + RLENGTH)
          continue
        }
        if (match(line, /^-?[0-9]+/)) {
          print "LITERAL:" substr(line, RSTART, RLENGTH)
          line=substr(line, RSTART + RLENGTH)
          continue
        }
        if (match(line, /^[A-Za-z_][A-Za-z0-9_]*/)) {
          print "IDENT:" substr(line, RSTART, RLENGTH)
          line=substr(line, RSTART + RLENGTH)
          continue
        }
        exit 7
      }
    }
  ')" || return 1
  while IFS= read -r token || [[ -n "$token" ]]; do
    [[ -n "$token" ]] || continue
    case "$token" in
      STRING:*) value="${token#STRING:}"; string_seen=true ;;
      LITERAL:*) value="${token#LITERAL:}" ;;
      IDENT:*) value="$(sley_eval_env_lookup "$env_json" "${token#IDENT:}")" || return 1 ;;
      CALL:*) value="$(sley_eval_source_call "$file" "${token#CALL:}" "$env_json")" || return 1 ;;
      *) return 1 ;;
    esac
    if "$seen"; then
      out="$(eval_runtime_text_binary_operator_value_task "+" "$out" "$value")" || return 1
    else
      out="$value"
    fi
    seen=true
  done <<< "$tokens"
  if [[ "$expr" == *+* && "$string_seen" != true ]]; then
    return 1
  fi
  [[ "$seen" == true ]] || return 1
  printf '%s\n' "$out"
}

sley_eval_source_task() {
  local file="$1" task="$2" env_json="${3:-}"
  local body expr value cache_path
  [[ -n "$env_json" ]] || env_json='{}'
  cache_path="$(sley_source_cache_path scalar "$file" "$task" "$env_json")"
  if sley_source_cache_read "$cache_path"; then
    return
  fi
  body="$(extract_sley_task_body "$file" "$task")"
  [[ -n "$body" ]] || return 1
  if value="$(sley_eval_source_static_scalar "$body" 2>/dev/null)"; then
    printf '%s\n' "$value"
    sley_source_cache_write "$cache_path" "$value"
    return
  fi
  env_json="$(sley_eval_source_bind_env_json "$file" "$env_json" "$body")" || return 1
  expr="$(sley_select_source_return_expr "$body" "$env_json")" || return 1
  value="$(sley_eval_source_expr "$file" "$env_json" "$expr")" || return 1
  printf '%s\n' "$value"
  sley_source_cache_write "$cache_path" "$value"
}

eval_parser_expression_classifiers_json() {
  local parser_file="$SELF_HOSTED_SOURCE_ROOT/loom/parser.sley"
  local source kind
  for source in "true" "false" "[]" "__fallback__"; do
    local probe="$source"
    [[ "$source" == "__fallback__" ]] && probe="value"
    kind="$(sley_eval_source_task "$parser_file" classify_expression "$(jq -cn --arg source "$probe" '{source:$source}')")" || return 1
    [[ -n "$kind" ]] || return 1
    jq -cn --arg key "$source" --arg value "$kind" '{key:$key,value:$value}'
  done | jq -s 'from_entries'
}

eval_parser_expression_feature_classifiers_json() {
  local parser_file="$SELF_HOSTED_SOURCE_ROOT/loom/parser.sley"
  local key is_int is_string is_true_literal is_false_literal is_empty_list is_identifier kind env_json
  for key in int string true false empty_list identifier fallback; do
    is_int=false
    is_string=false
    is_true_literal=false
    is_false_literal=false
    is_empty_list=false
    is_identifier=false
    case "$key" in
      int) is_int=true ;;
      string) is_string=true ;;
      true) is_true_literal=true ;;
      false) is_false_literal=true ;;
      empty_list) is_empty_list=true ;;
      identifier) is_identifier=true ;;
      fallback) ;;
      *) return 1 ;;
    esac
	    env_json="$(jq -cn \
	      --argjson is_int "$is_int" \
	      --argjson is_string "$is_string" \
	      --argjson is_true_literal "$is_true_literal" \
	      --argjson is_false_literal "$is_false_literal" \
	      --argjson is_empty_list "$is_empty_list" \
	      --argjson is_identifier "$is_identifier" \
	      '{is_int:$is_int,is_string:$is_string,is_true_literal:$is_true_literal,is_false_literal:$is_false_literal,is_empty_list:$is_empty_list,is_identifier:$is_identifier}')" || return 1
    kind="$(sley_eval_source_task "$parser_file" classify_expression_features "$env_json")" || return 1
    [[ -n "$kind" ]] || return 1
    jq -cn --arg key "$key" --arg value "$kind" '{key:$key,value:$value}'
  done | jq -s 'from_entries'
}

eval_parser_operator_feature_classifiers_json() {
  local parser_file="$SELF_HOSTED_SOURCE_ROOT/loom/parser.sley"
  local key has_unary_operator has_logical_operator has_comparison_operator has_arithmetic_operator kind env_json
  for key in unary logical comparison arithmetic fallback; do
    has_unary_operator=false
    has_logical_operator=false
    has_comparison_operator=false
    has_arithmetic_operator=false
    case "$key" in
      unary) has_unary_operator=true ;;
      logical) has_logical_operator=true ;;
      comparison) has_comparison_operator=true ;;
      arithmetic) has_arithmetic_operator=true ;;
      fallback) ;;
      *) return 1 ;;
    esac
    env_json="$(jq -cn \
      --argjson has_unary_operator "$has_unary_operator" \
      --argjson has_logical_operator "$has_logical_operator" \
      --argjson has_comparison_operator "$has_comparison_operator" \
      --argjson has_arithmetic_operator "$has_arithmetic_operator" \
      '{has_unary_operator:$has_unary_operator,has_logical_operator:$has_logical_operator,has_comparison_operator:$has_comparison_operator,has_arithmetic_operator:$has_arithmetic_operator}')" || return 1
    kind="$(sley_eval_source_task "$parser_file" classify_expression_operator_features "$env_json")" || return 1
    [[ -n "$kind" ]] || return 1
    jq -cn --arg key "$key" --arg value "$kind" '{key:$key,value:$value}'
  done | jq -s 'from_entries'
}

eval_parser_expression_surface_classifiers_json() {
  local parser_file="$SELF_HOSTED_SOURCE_ROOT/loom/parser.sley"
  local key has_try_suffix has_if_expression has_call_prefix has_bare_call has_field_access has_index_access has_list_literal has_map_literal has_record_literal kind env_json
  for key in try_suffix if_expression call bare_call field index list map record fallback; do
    has_try_suffix=false
    has_if_expression=false
    has_call_prefix=false
    has_bare_call=false
    has_field_access=false
    has_index_access=false
    has_list_literal=false
    has_map_literal=false
    has_record_literal=false
    case "$key" in
      try_suffix) has_try_suffix=true ;;
      if_expression) has_if_expression=true ;;
      call) has_call_prefix=true ;;
      bare_call) has_bare_call=true ;;
      field) has_field_access=true ;;
      index) has_index_access=true ;;
      list) has_list_literal=true ;;
      map) has_map_literal=true ;;
      record) has_record_literal=true ;;
      fallback) ;;
      *) return 1 ;;
    esac
    env_json="$(jq -cn \
      --argjson has_try_suffix "$has_try_suffix" \
      --argjson has_if_expression "$has_if_expression" \
      --argjson has_call_prefix "$has_call_prefix" \
      --argjson has_bare_call "$has_bare_call" \
      --argjson has_field_access "$has_field_access" \
      --argjson has_index_access "$has_index_access" \
      --argjson has_list_literal "$has_list_literal" \
      --argjson has_map_literal "$has_map_literal" \
      --argjson has_record_literal "$has_record_literal" \
      '{has_try_suffix:$has_try_suffix,has_if_expression:$has_if_expression,has_call_prefix:$has_call_prefix,has_bare_call:$has_bare_call,has_field_access:$has_field_access,has_index_access:$has_index_access,has_list_literal:$has_list_literal,has_map_literal:$has_map_literal,has_record_literal:$has_record_literal}')" || return 1
    kind="$(sley_eval_source_task "$parser_file" classify_expression_surface_features "$env_json")" || return 1
    [[ -n "$kind" ]] || return 1
    jq -cn --arg key "$key" --arg value "$kind" '{key:$key,value:$value}'
  done | jq -s 'from_entries'
}

eval_parser_declaration_feature_classifiers_json() {
  local parser_file="$SELF_HOSTED_SOURCE_ROOT/loom/parser.sley"
  local key is_module is_import is_type is_effect is_task kind env_json
  for key in module import type effect task fallback; do
    is_module=false
    is_import=false
    is_type=false
    is_effect=false
    is_task=false
    case "$key" in
      module) is_module=true ;;
      import) is_import=true ;;
      type) is_type=true ;;
      effect) is_effect=true ;;
      task) is_task=true ;;
      fallback) ;;
      *) return 1 ;;
    esac
    env_json="$(jq -cn \
      --argjson is_module "$is_module" \
      --argjson is_import "$is_import" \
      --argjson is_type "$is_type" \
      --argjson is_effect "$is_effect" \
      --argjson is_task "$is_task" \
      '{is_module:$is_module,is_import:$is_import,is_type:$is_type,is_effect:$is_effect,is_task:$is_task}')" || return 1
    kind="$(sley_eval_source_task "$parser_file" classify_declaration_features "$env_json")" || return 1
    [[ -n "$kind" ]] || return 1
    jq -cn --arg key "$key" --arg value "$kind" '{key:$key,value:$value}'
  done | jq -s 'from_entries'
}

eval_parser_binary_operator_names_json() {
  local parser_file="$SELF_HOSTED_SOURCE_ROOT/loom/parser.sley"
  local operator name
  for operator in '||' '&&' '==' '!=' '<' '<=' '>' '>=' '+' '*'; do
    name="$(sley_eval_source_task "$parser_file" binary_operator_name "$(jq -cn --arg operator "$operator" '{operator:$operator}')")" || return 1
    [[ -n "$name" ]] || return 1
    jq -cn --arg key "$operator" --arg value "$name" '{key:$key,value:$value}'
  done | jq -s 'from_entries'
}

eval_parser_unary_operator_names_json() {
  local parser_file="$SELF_HOSTED_SOURCE_ROOT/loom/parser.sley"
  local operator name
  for operator in '!' '-'; do
    name="$(sley_eval_source_task "$parser_file" unary_operator_name "$(jq -cn --arg operator "$operator" '{operator:$operator}')")" || return 1
    [[ -n "$name" ]] || return 1
    jq -cn --arg key "$operator" --arg value "$name" '{key:$key,value:$value}'
  done | jq -s 'from_entries'
}

eval_parser_expression_dispatch_plan_text() {
  local parser_file="$SELF_HOSTED_SOURCE_ROOT/loom/parser.sley"
  sley_eval_source_task_list_json "$parser_file" expression_dispatch_plan '{}' | jq -r '.[]'
}

eval_parser_declaration_dispatch_plan_text() {
  local parser_file="$SELF_HOSTED_SOURCE_ROOT/loom/parser.sley"
  sley_eval_source_task_list_json "$parser_file" declaration_dispatch_plan '{}' | jq -r '.[]'
}

eval_parser_statement_dispatch_plan_text() {
  local parser_file="$SELF_HOSTED_SOURCE_ROOT/loom/parser.sley"
  sley_eval_source_task_list_json "$parser_file" statement_dispatch_plan '{}' | jq -r '.[]'
}

eval_parser_statement_source_dispatch_plan_text() {
  local parser_file="$SELF_HOSTED_SOURCE_ROOT/loom/parser.sley"
  sley_eval_source_task_list_json "$parser_file" statement_source_dispatch_plan '{}' | jq -r '.[]'
}

eval_parser_take_source_dispatch_plan_text() {
  local parser_file="$SELF_HOSTED_SOURCE_ROOT/loom/parser.sley"
  sley_eval_source_task_list_json "$parser_file" take_source_dispatch_plan '{}' | jq -r '.[]'
}

eval_parser_statement_feature_classifiers_json() {
  local parser_file="$SELF_HOSTED_SOURCE_ROOT/loom/parser.sley"
  local key is_return is_call_expression is_binding is_set is_for is_while is_if is_forge kind env_json
  for key in return call_expression binding set for while if forge fallback; do
    is_return=false
    is_call_expression=false
    is_binding=false
    is_set=false
    is_for=false
    is_while=false
    is_if=false
    is_forge=false
    case "$key" in
      return) is_return=true ;;
      call_expression) is_call_expression=true ;;
      binding) is_binding=true ;;
      set) is_set=true ;;
      for) is_for=true ;;
      while) is_while=true ;;
      if) is_if=true ;;
      forge) is_forge=true ;;
      fallback) ;;
      *) return 1 ;;
    esac
	    env_json="$(jq -cn \
	      --argjson is_return "$is_return" \
	      --argjson is_call_expression "$is_call_expression" \
	      --argjson is_binding "$is_binding" \
	      --argjson is_set "$is_set" \
	      --argjson is_for "$is_for" \
	      --argjson is_while "$is_while" \
	      --argjson is_if "$is_if" \
	      --argjson is_forge "$is_forge" \
	      '{is_return:$is_return,is_call_expression:$is_call_expression,is_binding:$is_binding,is_set:$is_set,is_for:$is_for,is_while:$is_while,is_if:$is_if,is_forge:$is_forge}')" || return 1
    kind="$(sley_eval_source_task "$parser_file" classify_statement_features "$env_json")" || return 1
    [[ -n "$kind" ]] || return 1
    jq -cn --arg key "$key" --arg value "$kind" '{key:$key,value:$value}'
  done | jq -s 'from_entries'
}

eval_parser_binding_feature_classifiers_json() {
  local parser_file="$SELF_HOSTED_SOURCE_ROOT/loom/parser.sley"
  local key is_take is_gate is_state is_tally kind env_json
  for key in take gate bind state tally; do
    is_take=false
    is_gate=false
    is_state=false
    is_tally=false
    case "$key" in
      take) is_take=true ;;
      gate) is_gate=true ;;
      bind) ;;
      state) is_state=true ;;
      tally) is_tally=true ;;
      *) return 1 ;;
    esac
	    env_json="$(jq -cn \
	      --argjson is_take "$is_take" \
	      --argjson is_gate "$is_gate" \
	      --argjson is_state "$is_state" \
	      --argjson is_tally "$is_tally" \
	      '{is_take:$is_take,is_gate:$is_gate,is_state:$is_state,is_tally:$is_tally}')" || return 1
    kind="$(sley_eval_source_task "$parser_file" classify_binding_features "$env_json")" || return 1
    [[ -n "$kind" ]] || return 1
    jq -cn --arg key "$key" --arg value "$kind" '{key:$key,value:$value}'
  done | jq -s 'from_entries'
}

eval_parser_id_template() {
  local task="$1"
  local parser_file="$SELF_HOSTED_SOURCE_ROOT/loom/parser.sley"
  sley_eval_source_task "$parser_file" "$task" "$(sley_parser_template_env_json)"
}

sley_parser_template_env_json() {
  jq -cn '{
    task_id:"{{task_id}}",
    module_name:"{{module}}",
    import_module:"{{import_module}}",
    task_name:"{{task}}",
    type_name:"{{type}}",
    effect_name:"{{effect}}",
    take_name:"{{take}}",
    statement_id:"{{statement}}",
    expression_id:"{{expression}}",
    branch_name:"{{branch}}",
    side_name:"{{side}}",
    index:"{{index}}"
  }'
}

sley_message_template_env_json() {
  jq -cn '{
    identifier_name:"::identifier_name::",
    expression_text:"::expression_text::",
    statement_text:"::statement_text::",
    type_name:"::type_name::",
    task_name:"::task_name::",
    binding_name:"::binding_name::",
    take_name:"::take_name::",
    key_name:"::key_name::",
    field_name:"::field_name::",
    actual_type:"::actual_type::",
    effect_name:"::effect_name::",
    diagnostic_id:"::diagnostic_id::",
    node_id:"::node_id::"
  }'
}

eval_parser_message_template() {
  local task="$1" arg_name="$2"
  local parser_file="$SELF_HOSTED_SOURCE_ROOT/loom/parser.sley"
  local out
  out="$(sley_eval_source_task "$parser_file" "$task" "$(sley_message_template_env_json)")" || return 1
  [[ "$out" == *"::$arg_name::"* ]] || return 1
  printf '%s\n' "$out"
}

eval_checker_message_template() {
  local task="$1" arg_name="$2"
  local checker_file="$SELF_HOSTED_SOURCE_ROOT/loom/checker.sley"
  local out
  out="$(sley_eval_source_task "$checker_file" "$task" "$(sley_message_template_env_json)")" || return 1
  [[ "$out" == *"::$arg_name::"* ]] || return 1
  printf '%s\n' "$out"
}

eval_bootstrap_smoke_task() {
  local bootstrap_file="$SELF_HOSTED_SOURCE_ROOT/loom/bootstrap.sley"
  sley_eval_source_task "$bootstrap_file" smoke '{}'
}

eval_bootstrap_list_count_task() {
  local task_name="$1"
  local bootstrap_file="$SELF_HOSTED_SOURCE_ROOT/loom/bootstrap.sley"
  sley_eval_source_task "$bootstrap_file" "$task_name" '{}'
}

eval_lint_status_task() {
  local finding_count="$1"
  local lint_file="$SELF_HOSTED_SOURCE_ROOT/loom/lint.sley"
  local env_json
  [[ "$finding_count" =~ ^-?[0-9]+$ ]] || return 1
  env_json="$(jq -cn --argjson finding_count "$finding_count" '{finding_count:$finding_count}')"
  sley_eval_source_task "$lint_file" lint_status "$env_json"
}

eval_checker_status_task() {
  local error_count="$1"
  local checker_file="$SELF_HOSTED_SOURCE_ROOT/loom/checker.sley"
  local env_json
  [[ "$error_count" =~ ^-?[0-9]+$ ]] || return 1
  env_json="$(jq -cn --argjson error_count "$error_count" '{error_count:$error_count}')"
  sley_eval_source_task "$checker_file" diagnostic_status "$env_json"
}

eval_checker_builtin_types_json() {
  local checker_file="$SELF_HOSTED_SOURCE_ROOT/loom/checker.sley"
  sley_eval_source_task_list_json "$checker_file" builtin_types '{}'
}

sley_source_task() {
  local module="$1" task="$2"
  local source_file
  source_file="$(module_source_file "$module")"
  sley_eval_source_task "$source_file" "$task" '{}'
}

sley_source_list_task_json() {
  local module="$1" task="$2"
  local source_file
  source_file="$(module_source_file "$module")"
  sley_eval_source_task_list_json "$source_file" "$task" '{}'
}

eval_runtime_status_task() {
  local diagnostic_count="$1"
  local runtime_file="$SELF_HOSTED_SOURCE_ROOT/loom/runtime.sley"
  local env_json
  [[ "$diagnostic_count" =~ ^-?[0-9]+$ ]] || return 1
  env_json="$(jq -cn --argjson diagnostic_count "$diagnostic_count" '{diagnostic_count:$diagnostic_count}')"
  sley_eval_source_task "$runtime_file" runtime_status "$env_json"
}

eval_runtime_text_probe_task() {
  local task_name="$1"
  local runtime_file="$SELF_HOSTED_SOURCE_ROOT/loom/runtime.sley"
  sley_eval_source_task "$runtime_file" "$task_name" '{}'
}

sley_runtime_int_list_json() {
  printf '[%s]\n' "$1" | jq -ce '
    if type == "array" and all(.[]; if type == "number" then floor == . else false end) then . else empty end
  '
}

sley_runtime_text_list_json() {
  printf '[%s]\n' "$1" | jq -ce '
    if type == "array" and all(.[]; type == "string") then . else empty end
  '
}

sley_runtime_text_map_json() {
  printf '{%s}\n' "$1" | jq -ce '
    if type == "object" and all(.[]; type == "string") then . else empty end
  '
}

sley_runtime_int_map_json() {
  printf '{%s}\n' "$1" | jq -ce '
    if type == "object" and all(.[]; if type == "number" then floor == . else false end) then . else empty end
  '
}

sley_runtime_text_record_json() {
  printf '%s\n' "$1" | awk '
    function trim(s){gsub(/^[ \t\r\n]+|[ \t\r\n]+$/, "", s); return s}
    {
      entry_count=split($0, entries, ",")
      for(i=1; i<=entry_count; i++) {
        split(entries[i], fields, ":")
        if(length(fields) != 2) exit 1
        field_name=trim(fields[1])
        field_value=trim(fields[2])
        if(field_name !~ /^[A-Za-z_][A-Za-z0-9_]*$/) exit 1
        if(field_value !~ /^"[^"]*"$/) next
        sub(/^"/, "", field_value)
        sub(/"$/, "", field_value)
        print field_name "\t" field_value
      }
    }
  ' | jq -Rsce '
    [split("\n")[] | select(length > 0) | split("\t") | select(length == 2)] as $pairs |
    if ($pairs | length) == 0 then empty else
      reduce $pairs[] as $pair ({}; .[$pair[0]] = $pair[1])
    end
  '
}

eval_seeded_agent_deploy_value() {
  local profile="$1" model_plan="$2" deploy_result="$3"
  local runtime_file="$SELF_HOSTED_SOURCE_ROOT/loom/runtime.sley"
  local env_json
  env_json="$(jq -cn \
    --arg profile "$profile" \
    --arg model_plan "$model_plan" \
    --arg deploy_result "$deploy_result" \
    '{profile:$profile, model_plan:$model_plan, deploy_result:$deploy_result}')"
  sley_eval_source_task "$runtime_file" seeded_agent_deploy_value "$env_json"
}

eval_runtime_spend_prefixed_value_task() {
  local authorization="$1"
  local runtime_file="$SELF_HOSTED_SOURCE_ROOT/loom/runtime.sley"
  local env_json
  env_json="$(jq -cn --arg authorization "$authorization" '{authorization:$authorization}')"
  sley_eval_source_task "$runtime_file" spend_prefixed_value "$env_json"
}

eval_project_ready_value_task() {
  local runtime_file="$SELF_HOSTED_SOURCE_ROOT/loom/runtime.sley"
  sley_eval_source_task "$runtime_file" project_ready_value '{}'
}

eval_runtime_list_len_value_task() {
  local item_count="$1"
  local runtime_file="$SELF_HOSTED_SOURCE_ROOT/loom/runtime.sley"
  local env_json
  [[ "$item_count" =~ ^[0-9]+$ ]] || return 1
  env_json="$(jq -cn --argjson item_count "$item_count" '{item_count:$item_count}')"
  sley_eval_source_task "$runtime_file" list_len_value "$env_json"
}

eval_runtime_int_identity_value_task() {
  local value="$1"
  local runtime_file="$SELF_HOSTED_SOURCE_ROOT/loom/runtime.sley"
  local env_json
  [[ "$value" =~ ^-?[0-9]+$ ]] || return 1
  env_json="$(jq -cn --argjson value "$value" '{value:$value}')"
  sley_eval_source_task "$runtime_file" int_identity_value "$env_json"
}

eval_runtime_text_identity_value_task() {
  local value="$1"
  local runtime_file="$SELF_HOSTED_SOURCE_ROOT/loom/runtime.sley"
  local env_json
  env_json="$(jq -cn --arg value "$value" '{value:$value}')"
  sley_eval_source_task "$runtime_file" text_identity_value "$env_json"
}

eval_runtime_text_concat_value_task() {
  local left="$1" right="$2"
  local runtime_file="$SELF_HOSTED_SOURCE_ROOT/loom/runtime.sley"
  local env_json
  env_json="$(jq -cn --arg left "$left" --arg right "$right" '{left:$left, right:$right}')"
  sley_eval_source_task "$runtime_file" text_concat_value "$env_json"
}

runtime_text_binary_task_name() {
  local op="$1"
  printf '%s\n' "$RUNTIME_TEXT_BINARY_OPERATOR_PLAN_JSON" | jq -er --arg op "$op" '.[] | select(.op == $op) | .task'
}

eval_runtime_text_binary_operator_value_task() {
  local op="$1" left="$2" right="$3"
  local runtime_file="$SELF_HOSTED_SOURCE_ROOT/loom/runtime.sley"
  local task_name env_json
  task_name="$(runtime_text_binary_task_name "$op")" || return 1
  env_json="$(jq -cn --arg left "$left" --arg right "$right" '{left:$left, right:$right}')"
  sley_eval_source_task "$runtime_file" "$task_name" "$env_json"
}

eval_runtime_bool_identity_value_task() {
  local value="$1"
  local runtime_file="$SELF_HOSTED_SOURCE_ROOT/loom/runtime.sley"
  local env_json
  [[ "$value" == "true" || "$value" == "false" ]] || return 1
  env_json="$(jq -cn --argjson value "$value" '{value:$value}')"
  sley_eval_source_task "$runtime_file" bool_identity_value "$env_json"
}

runtime_int_binary_task_name() {
  local op="$1"
  printf '%s\n' "$RUNTIME_INT_BINARY_OPERATOR_PLAN_JSON" | jq -er --arg op "$op" '.[] | select(.op == $op) | .task'
}

eval_runtime_int_binary_value_task() {
  local task_name="$1" op="$2" left="$3" right="$4"
  local runtime_file="$SELF_HOSTED_SOURCE_ROOT/loom/runtime.sley"
  local env_json expected_task_name
  [[ "$left" =~ ^-?[0-9]+$ && "$right" =~ ^-?[0-9]+$ ]] || return 1
  expected_task_name="$(runtime_int_binary_task_name "$op")" || return 1
  [[ "$task_name" == "$expected_task_name" ]] || return 1
  env_json="$(jq -cn --argjson left "$left" --argjson right "$right" '{left:$left, right:$right}')"
  sley_eval_source_task "$runtime_file" "$task_name" "$env_json"
}

eval_runtime_int_binary_operator_value_task() {
  local op="$1" left="$2" right="$3" task_name
  task_name="$(runtime_int_binary_task_name "$op")" || return 1
  eval_runtime_int_binary_value_task "$task_name" "$op" "$left" "$right"
}

runtime_int_comparison_descriptor_json() {
  local op="$1"
  printf '%s\n' "$RUNTIME_INT_COMPARISON_OPERATOR_PLAN_JSON" | jq -cer --arg op "$op" '.[] | select(.op == $op)'
}

eval_runtime_int_comparison_operator_value_task() {
  local op="$1" left="$2" right="$3"
  local runtime_file="$SELF_HOSTED_SOURCE_ROOT/loom/runtime.sley"
  local descriptor task_name order negate cmp_left cmp_right env_json value
  [[ "$left" =~ ^-?[0-9]+$ && "$right" =~ ^-?[0-9]+$ ]] || return 1
  descriptor="$(runtime_int_comparison_descriptor_json "$op")" || return 1
  task_name="$(printf '%s\n' "$descriptor" | jq -er '.task')" || return 1
  order="$(printf '%s\n' "$descriptor" | jq -er '.order')" || return 1
  negate="$(printf '%s\n' "$descriptor" | jq -er '.negate')" || return 1
  if [[ "$order" == "reverse" ]]; then
    cmp_left="$right"
    cmp_right="$left"
  else
    [[ "$order" == "direct" ]] || return 1
    cmp_left="$left"
    cmp_right="$right"
  fi
  env_json="$(jq -cn --argjson left "$cmp_left" --argjson right "$cmp_right" '{left:$left, right:$right}')"
  value="$(sley_eval_source_task "$runtime_file" "$task_name" "$env_json")" || return 1
  if [[ "$negate" == "true" ]]; then
    eval_runtime_bool_unary_operator_value_task "!" "$value"
  else
    [[ "$negate" == "false" ]] || return 1
    printf '%s\n' "$value"
  fi
}

eval_runtime_int_less_than_value_task() {
  local left="$1" right="$2"
  local runtime_file="$SELF_HOSTED_SOURCE_ROOT/loom/runtime.sley"
  local env_json
  [[ "$left" =~ ^-?[0-9]+$ && "$right" =~ ^-?[0-9]+$ ]] || return 1
  env_json="$(jq -cn --argjson left "$left" --argjson right "$right" '{left:$left, right:$right}')"
  sley_eval_source_task "$runtime_file" int_less_than_value "$env_json"
}

eval_runtime_int_equal_value_task() {
  local left="$1" right="$2"
  local runtime_file="$SELF_HOSTED_SOURCE_ROOT/loom/runtime.sley"
  local env_json
  [[ "$left" =~ ^-?[0-9]+$ && "$right" =~ ^-?[0-9]+$ ]] || return 1
  env_json="$(jq -cn --argjson left "$left" --argjson right "$right" '{left:$left, right:$right}')"
  sley_eval_source_task "$runtime_file" int_equal_value "$env_json"
}

eval_runtime_int_greater_equal_value_task() {
  local left="$1" right="$2"
  local runtime_file="$SELF_HOSTED_SOURCE_ROOT/loom/runtime.sley"
  local env_json
  [[ "$left" =~ ^-?[0-9]+$ && "$right" =~ ^-?[0-9]+$ ]] || return 1
  env_json="$(jq -cn --argjson left "$left" --argjson right "$right" '{left:$left, right:$right}')"
  sley_eval_source_task "$runtime_file" int_greater_equal_value "$env_json"
}

runtime_collection_index_descriptor_json() {
  local collection_kind="$1" value_kind="$2"
  printf '%s\n' "$RUNTIME_COLLECTION_INDEX_OPERATOR_PLAN_JSON" | jq -cer \
    --arg collection_kind "$collection_kind" \
    --arg value_kind "$value_kind" \
    '.[] | select(.collection_kind == $collection_kind and .value_kind == $value_kind)'
}

eval_runtime_collection_index_operator_value_task() {
  local collection_kind="$1" value_kind="$2" collection_source="$3" key="$4"
  local runtime_file="$SELF_HOSTED_SOURCE_ROOT/loom/runtime.sley"
  local descriptor task_name values_json env_json
  descriptor="$(runtime_collection_index_descriptor_json "$collection_kind" "$value_kind")" || return 1
  task_name="$(printf '%s\n' "$descriptor" | jq -er '.task')" || return 1
  case "$collection_kind:$value_kind" in
    list:Int)
      [[ "$key" =~ ^[0-9]+$ ]] || return 1
      values_json="$(sley_runtime_int_list_json "$collection_source")" || return 1
      env_json="$(jq -cn --argjson values "$values_json" --argjson index "$key" '{values:$values, index:$index}')"
      ;;
    list:Text)
      [[ "$key" =~ ^[0-9]+$ ]] || return 1
      values_json="$(sley_runtime_text_list_json "$collection_source")" || return 1
      env_json="$(jq -cn --argjson values "$values_json" --argjson index "$key" '{values:$values, index:$index}')"
      ;;
    map:Int)
      values_json="$(sley_runtime_int_map_json "$collection_source")" || return 1
      env_json="$(jq -cn --argjson values "$values_json" --arg key "$key" '{values:$values, key:$key}')"
      ;;
    map:Text)
      values_json="$(sley_runtime_text_map_json "$collection_source")" || return 1
      env_json="$(jq -cn --argjson values "$values_json" --arg key "$key" '{values:$values, key:$key}')"
      ;;
    *)
      return 1
      ;;
  esac
  sley_eval_source_task "$runtime_file" "$task_name" "$env_json"
}

eval_runtime_list_index_int_value_task() {
  local values_source="$1" index="$2"
  eval_runtime_collection_index_operator_value_task "list" "Int" "$values_source" "$index"
}

eval_runtime_list_index_text_value_task() {
  local values_source="$1" index="$2"
  eval_runtime_collection_index_operator_value_task "list" "Text" "$values_source" "$index"
}

eval_runtime_map_index_text_value_task() {
  local pairs_source="$1" key="$2"
  eval_runtime_collection_index_operator_value_task "map" "Text" "$pairs_source" "$key"
}

eval_runtime_map_index_int_value_task() {
  local pairs_source="$1" key="$2"
  eval_runtime_collection_index_operator_value_task "map" "Int" "$pairs_source" "$key"
}

runtime_record_field_access_descriptor_json() {
  local field="$1" value_kind="$2"
  printf '%s\n' "$RUNTIME_RECORD_FIELD_ACCESS_PLAN_JSON" | jq -cer \
    --arg field "$field" \
    --arg value_kind "$value_kind" \
    '.[] | select(.field == $field and .value_kind == $value_kind)'
}

eval_runtime_record_field_access_value_task() {
  local value_kind="$1" fields_source="$2" field="$3"
  local runtime_file="$SELF_HOSTED_SOURCE_ROOT/loom/runtime.sley"
  local descriptor task_name record_json env_json
  descriptor="$(runtime_record_field_access_descriptor_json "$field" "$value_kind")" || return 1
  task_name="$(printf '%s\n' "$descriptor" | jq -er '.task')" || return 1
  case "$value_kind" in
    Text)
      record_json="$(sley_runtime_text_record_json "$fields_source")" || return 1
      printf '%s\n' "$record_json" | jq -e --arg field "$field" 'has($field)' >/dev/null || return 1
      env_json="$(jq -cn --argjson user "$record_json" '{user:$user}')"
      ;;
    *)
      return 1
      ;;
  esac
  sley_eval_source_task "$runtime_file" "$task_name" "$env_json"
}

eval_runtime_record_field_text_value_task() {
  local fields_source="$1" field="$2"
  eval_runtime_record_field_access_value_task "Text" "$fields_source" "$field"
}

eval_runtime_bool_equal_value_task() {
  local left="$1" right="$2"
  local runtime_file="$SELF_HOSTED_SOURCE_ROOT/loom/runtime.sley"
  local env_json
  [[ "$left" == "true" || "$left" == "false" ]] || return 1
  [[ "$right" == "true" || "$right" == "false" ]] || return 1
  env_json="$(jq -cn --argjson left "$left" --argjson right "$right" '{left:$left, right:$right}')"
  sley_eval_source_task "$runtime_file" bool_equal_value "$env_json"
}

eval_runtime_text_equal_value_task() {
  local left="$1" right="$2"
  local runtime_file="$SELF_HOSTED_SOURCE_ROOT/loom/runtime.sley"
  local env_json
  env_json="$(jq -cn --arg left "$left" --arg right "$right" '{left:$left, right:$right}')"
  sley_eval_source_task "$runtime_file" text_equal_value "$env_json"
}

runtime_equality_operator_descriptor_json() {
  local value_kind="$1" op="$2"
  printf '%s\n' "$RUNTIME_EQUALITY_OPERATOR_PLAN_JSON" | jq -cer \
    --arg value_kind "$value_kind" \
    --arg op "$op" \
    '.[] | select(.value_kind == $value_kind and .op == $op)'
}

eval_runtime_equality_operator_value_task() {
  local value_kind="$1" op="$2" left="$3" right="$4"
  local runtime_file="$SELF_HOSTED_SOURCE_ROOT/loom/runtime.sley"
  local descriptor task_name env_json
  descriptor="$(runtime_equality_operator_descriptor_json "$value_kind" "$op")" || return 1
  task_name="$(printf '%s\n' "$descriptor" | jq -er '.task')" || return 1
  case "$value_kind" in
    Bool)
      [[ "$left" == "true" || "$left" == "false" ]] || return 1
      [[ "$right" == "true" || "$right" == "false" ]] || return 1
      env_json="$(jq -cn --argjson left "$left" --argjson right "$right" '{left:$left, right:$right}')"
      ;;
    Text)
      env_json="$(jq -cn --arg left "$left" --arg right "$right" '{left:$left, right:$right}')"
      ;;
    *)
      return 1
      ;;
  esac
  sley_eval_source_task "$runtime_file" "$task_name" "$env_json"
}

runtime_bool_binary_task_name() {
  local op="$1"
  printf '%s\n' "$RUNTIME_BOOL_BINARY_OPERATOR_PLAN_JSON" | jq -er --arg op "$op" '.[] | select(.op == $op) | .task'
}

eval_runtime_bool_binary_operator_value_task() {
  local op="$1" left="$2" right="$3"
  local runtime_file="$SELF_HOSTED_SOURCE_ROOT/loom/runtime.sley"
  local task_name env_json
  [[ "$left" == "true" || "$left" == "false" ]] || return 1
  [[ "$right" == "true" || "$right" == "false" ]] || return 1
  task_name="$(runtime_bool_binary_task_name "$op")" || return 1
  env_json="$(jq -cn --argjson left "$left" --argjson right "$right" '{left:$left, right:$right}')"
  sley_eval_source_task "$runtime_file" "$task_name" "$env_json"
}

runtime_bool_unary_task_name() {
  local op="$1"
  printf '%s\n' "$RUNTIME_BOOL_UNARY_OPERATOR_PLAN_JSON" | jq -er --arg op "$op" '.[] | select(.op == $op) | .task'
}

eval_runtime_bool_unary_operator_value_task() {
  local op="$1" value="$2"
  local runtime_file="$SELF_HOSTED_SOURCE_ROOT/loom/runtime.sley"
  local task_name env_json
  [[ "$value" == "true" || "$value" == "false" ]] || return 1
  task_name="$(runtime_bool_unary_task_name "$op")" || return 1
  env_json="$(jq -cn --argjson value "$value" '{value:$value}')"
  sley_eval_source_task "$runtime_file" "$task_name" "$env_json"
}

eval_runtime_bool_and_value_task() {
  local left="$1" right="$2"
  local runtime_file="$SELF_HOSTED_SOURCE_ROOT/loom/runtime.sley"
  local env_json
  [[ "$left" == "true" || "$left" == "false" ]] || return 1
  [[ "$right" == "true" || "$right" == "false" ]] || return 1
  env_json="$(jq -cn --argjson left "$left" --argjson right "$right" '{left:$left, right:$right}')"
  sley_eval_source_task "$runtime_file" bool_and_value "$env_json"
}

eval_runtime_bool_or_value_task() {
  local left="$1" right="$2"
  local runtime_file="$SELF_HOSTED_SOURCE_ROOT/loom/runtime.sley"
  local env_json
  [[ "$left" == "true" || "$left" == "false" ]] || return 1
  [[ "$right" == "true" || "$right" == "false" ]] || return 1
  env_json="$(jq -cn --argjson left "$left" --argjson right "$right" '{left:$left, right:$right}')"
  sley_eval_source_task "$runtime_file" bool_or_value "$env_json"
}

eval_runtime_bool_not_value_task() {
  local value="$1"
  local runtime_file="$SELF_HOSTED_SOURCE_ROOT/loom/runtime.sley"
  local env_json
  [[ "$value" == "true" || "$value" == "false" ]] || return 1
  env_json="$(jq -cn --argjson value "$value" '{value:$value}')"
  sley_eval_source_task "$runtime_file" bool_not_value "$env_json"
}

eval_runtime_bool_if_value_task() {
  local condition="$1" then_value="$2" else_value="$3"
  local runtime_file="$SELF_HOSTED_SOURCE_ROOT/loom/runtime.sley"
  local env_json
  [[ "$condition" == "true" || "$condition" == "false" ]] || return 1
  [[ "$then_value" == "true" || "$then_value" == "false" ]] || return 1
  [[ "$else_value" == "true" || "$else_value" == "false" ]] || return 1
  env_json="$(jq -cn --argjson condition "$condition" --argjson then_value "$then_value" --argjson else_value "$else_value" '{condition:$condition, then_value:$then_value, else_value:$else_value}')"
  sley_eval_source_task "$runtime_file" bool_if_value "$env_json"
}

eval_runtime_int_if_value_task() {
  local condition="$1" then_value="$2" else_value="$3"
  local runtime_file="$SELF_HOSTED_SOURCE_ROOT/loom/runtime.sley"
  local env_json
  [[ "$condition" == "true" || "$condition" == "false" ]] || return 1
  [[ "$then_value" =~ ^-?[0-9]+$ && "$else_value" =~ ^-?[0-9]+$ ]] || return 1
  env_json="$(jq -cn --argjson condition "$condition" --argjson then_value "$then_value" --argjson else_value "$else_value" '{condition:$condition, then_value:$then_value, else_value:$else_value}')"
  sley_eval_source_task "$runtime_file" int_if_value "$env_json"
}

eval_runtime_text_if_value_task() {
  local condition="$1" then_value="$2" else_value="$3"
  local runtime_file="$SELF_HOSTED_SOURCE_ROOT/loom/runtime.sley"
  local env_json
  [[ "$condition" == "true" || "$condition" == "false" ]] || return 1
  env_json="$(jq -cn --argjson condition "$condition" --arg then_value "$then_value" --arg else_value "$else_value" '{condition:$condition, then_value:$then_value, else_value:$else_value}')"
  sley_eval_source_task "$runtime_file" text_if_value "$env_json"
}

runtime_if_value_descriptor_json() {
  local value_kind="$1"
  printf '%s\n' "$RUNTIME_IF_VALUE_PLAN_JSON" | jq -cer \
    --arg value_kind "$value_kind" \
    '.[] | select(.value_kind == $value_kind)'
}

eval_runtime_if_value_task() {
  local value_kind="$1" condition="$2" then_value="$3" else_value="$4"
  local runtime_file="$SELF_HOSTED_SOURCE_ROOT/loom/runtime.sley"
  local descriptor task_name env_json
  [[ "$condition" == "true" || "$condition" == "false" ]] || return 1
  descriptor="$(runtime_if_value_descriptor_json "$value_kind")" || return 1
  task_name="$(printf '%s\n' "$descriptor" | jq -er '.task')" || return 1
  case "$value_kind" in
    Bool)
      [[ "$then_value" == "true" || "$then_value" == "false" ]] || return 1
      [[ "$else_value" == "true" || "$else_value" == "false" ]] || return 1
      env_json="$(jq -cn --argjson condition "$condition" --argjson then_value "$then_value" --argjson else_value "$else_value" '{condition:$condition, then_value:$then_value, else_value:$else_value}')"
      ;;
    Int)
      [[ "$then_value" =~ ^-?[0-9]+$ && "$else_value" =~ ^-?[0-9]+$ ]] || return 1
      env_json="$(jq -cn --argjson condition "$condition" --argjson then_value "$then_value" --argjson else_value "$else_value" '{condition:$condition, then_value:$then_value, else_value:$else_value}')"
      ;;
    Text)
      env_json="$(jq -cn --argjson condition "$condition" --arg then_value "$then_value" --arg else_value "$else_value" '{condition:$condition, then_value:$then_value, else_value:$else_value}')"
      ;;
    *)
      return 1
      ;;
  esac
  sley_eval_source_task "$runtime_file" "$task_name" "$env_json"
}

eval_runtime_ok_int_value_task() {
  local value="$1"
  local runtime_file="$SELF_HOSTED_SOURCE_ROOT/loom/runtime.sley"
  local env_json
  [[ "$value" =~ ^-?[0-9]+$ ]] || return 1
  env_json="$(jq -cn --argjson value "$value" '{value:$value}')"
  sley_eval_source_task "$runtime_file" ok_int_value "$env_json"
}

eval_runtime_ok_text_value_task() {
  local value="$1"
  local runtime_file="$SELF_HOSTED_SOURCE_ROOT/loom/runtime.sley"
  local env_json
  env_json="$(jq -cn --arg value "$value" '{value:$value}')"
  sley_eval_source_task "$runtime_file" ok_text_value "$env_json"
}

eval_runtime_err_text_value_task() {
  local value="$1"
  local runtime_file="$SELF_HOSTED_SOURCE_ROOT/loom/runtime.sley"
  local env_json
  env_json="$(jq -cn --arg value "$value" '{value:$value}')"
  sley_eval_source_task "$runtime_file" err_text_value "$env_json"
}

runtime_project_ready_probe_matches() {
  local target="$1"
  ast_json "$target" | jq -e \
    --arg call_probe "$RUNTIME_PROJECT_READY_CALL_PROBE" \
    --arg binding_probe "$RUNTIME_PROJECT_READY_BINDING_PROBE" '
    [.tasks[].body.statements[]?.expr?.source?] | any(. == $call_probe or . == $binding_probe)
  ' >/dev/null
}

runtime_source_has_call() {
  local target="$1" needle="$2"
  if ast_json "$target" | jq -e --arg needle "$needle" '
    .. | objects | .source? // empty | select(contains($needle))
  ' >/dev/null; then
    return 0
  fi
  while IFS= read -r file; do
    if grep -Fq -- "$needle" "$file"; then
      return 0
    fi
  done < <(runtime_context_files "$target")
  return 1
}

checker_host_effect_needles_for() {
  local effect="$1"
  printf '%s\n' "$CHECKER_HOST_EFFECT_NEEDLES_JSON" | jq -r --arg effect "$effect" '
    .[]? |
    split("|") as $parts |
    select(($parts | length) >= 2 and $parts[0] == $effect) |
    $parts[1]
  '
}

runtime_source_has_host_effect() {
  local target="$1" effect="$2" needle
  while IFS= read -r needle; do
    [[ -n "$needle" ]] || continue
    if runtime_source_has_call "$target" "$needle"; then
      return 0
    fi
  done < <(checker_host_effect_needles_for "$effect")
  return 1
}

runtime_source_has_host_effects() {
  local target="$1" effect
  shift
  for effect in "$@"; do
    runtime_source_has_host_effect "$target" "$effect" || return 1
  done
  return 0
}

parser_default_qualified_import_call_source() {
  printf '%smath.double(21)\n' "$PARSER_CALL_EXPRESSION_PREFIX"
}

parser_default_project_ready_binding_call_source() {
  printf '%sdouble(base)\n' "$PARSER_CALL_EXPRESSION_PREFIX"
}

source_has_unqualified_double_call_return() {
  local content="$1" call_prefix
  call_prefix="$PARSER_CALL_EXPRESSION_PREFIX"
  printf '%s\n' "$content" | awk -v call_prefix="$call_prefix" '
    function trim(s){gsub(/^[ \t\r\n]+|[ \t\r\n]+$/, "", s); return s}
    {
      line=trim($0)
      if(line ~ /^return[ \t]+/) {
        sub(/^return[ \t]+/, "", line)
        if(line == call_prefix "double(21)") found=1
      }
    }
    END {exit found ? 0 : 1}
  '
}

runtime_deploy_stage_probe_matches() {
  runtime_source_has_host_effect "$1" "Deploy"
}

runtime_agent_pipeline_probe_matches() {
  runtime_source_has_host_effects "$1" "SecretRead" "Network" "ModelCall" "Deploy"
}

runtime_spend_authorize_probe_matches() {
  runtime_source_has_host_effect "$1" "Spend"
}

runtime_spend_prefix_probe_matches() {
  runtime_source_has_call "$1" "budget gate: "
}

sha256_stream() {
  sha256sum | awk '{print "sha256:" $1}'
}

source_digest_for() {
  local target="$1"
  mapfile -t files < <(collect_files "$target")
  if [[ "${#files[@]}" -eq 1 ]]; then
    sha256sum "${files[0]}" | awk '{print "sha256:" $1}'
    return
  fi
  local file
  for file in "${files[@]}"; do
    printf 'path:%s\n' "$file"
    sed -n '1,$p' "$file"
    printf '\n'
  done | sha256_stream
}

structural_source_digest_for() {
  local target="$1" target_root file file_real
  mapfile -t files < <(collect_files "$target")
  if [[ "${#files[@]}" -eq 1 ]]; then
    sha256sum "${files[0]}" | awk '{print "sha256:" $1}'
    return
  fi
  target_root="$(realpath -e -- "$target")"
  for file in "${files[@]}"; do
    file_real="$(realpath -e -- "$file")"
    printf 'path:%s\n' "${file_real#"$target_root"/}"
    sed -n '1,$p' "$file_real"
    printf '\n'
  done | sha256_stream
}

graph_json_for() {
  command_graph --json "$1"
}

json_digest_for() {
  jq -cS . | sha256_stream
}

trace_receipts_json() {
  local trace_path="${1:-}"
  if [[ -n "$trace_path" && -f "$trace_path" ]]; then
    sley_reject_ambiguous_path "$trace_path"
    sley_enforce_file_budget "$trace_path" "$SLEY_MAX_JSON_BYTES"
    jq -s '.' "$trace_path"
  else
    printf '[]\n'
  fi
}

seal_json() {
  local target="$1" trace_path="${2:-}" graph receipts source_digest graph_digest trace_digest seal_digest
  graph="$(graph_json_for "$target")"
  receipts="$(trace_receipts_json "$trace_path")"
  source_digest="$(source_digest_for "$target")"
  graph_digest="$(printf '%s\n' "$graph" | json_digest_for)"
  trace_digest="$(printf '%s\n' "$receipts" | json_digest_for)"
  seal_digest="$(printf '%s\n%s\n%s\n' "$source_digest" "$graph_digest" "$trace_digest" | sha256_stream)"
  jq -n \
    --arg target "$target" \
    --arg source_digest "$source_digest" \
    --arg graph_digest "$graph_digest" \
    --arg trace_digest "$trace_digest" \
    --arg seal_digest "$seal_digest" \
    --argjson graph "$graph" \
    --argjson receipts "$receipts" '{
      schema:"sley.trace.seal.v0",
      target:$target,
      source_digest:$source_digest,
      graph_digest:$graph_digest,
      trace_digest:$trace_digest,
      seal_digest:$seal_digest,
      module_count:($graph.modules | length),
      task_count:([$graph.modules[].tasks[]?] | length),
      receipt_count:($receipts | length)
    }'
}

zjx_envelope_json() {
  local target="$1" trace_path="${2:-}" slice="${3:-}" graph query receipts graph_digest
  graph="$(graph_json_for "$target")"
  query="$(query_json "$target" all)"
  receipts="$(trace_receipts_json "$trace_path")"
  graph_digest="$(printf '%s\n' "$graph" | json_digest_for)"
  jq -n \
    --arg target "$target" \
    --arg graph_digest "$graph_digest" \
    --arg slice "$slice" \
    --argjson graph "$graph" \
    --argjson query "$query" \
    --argjson receipts "$receipts" '
    def focus_from_id($id):
      ($id | sub("^task:"; "")) as $qualified |
      ($qualified | split(".")) as $parts |
      {
        kind:"task",
        id:$id,
        module:($parts[0:-1] | join(".")),
        name:($parts[-1])
      };
    def slice_report($id):
      (focus_from_id($id)) as $focus |
      ($graph.modules[]? | select(.module == $focus.module)) as $module |
      {
        schema:"sley.symbol_graph.slice.v0",
        target:$target,
        entry_module:$graph.entry_module,
        focus:$focus,
        imports:($module.imports // []),
        types:($module.types // []),
        effects:($module.effects // []),
        tasks:($module.tasks // []),
        add_affordances:[],
        insert_affordances:[],
        move_affordances:[],
        delete_affordances:[],
        replace_affordances:[],
        call_site_affordances:[],
        call_arg_affordances:[],
        outbound_calls:[$query.calls[]? | select(.from == ($focus.module + "." + $focus.name))],
        inbound_calls:[$query.calls[]? | select(.target == ($focus.module + "." + $focus.name))]
      };
    {
      schema:"sley.zjx.envelope.v0",
      format:"zjx-preview-json",
      compression:"none",
      target:$target,
      graph_digest:$graph_digest,
      graph:$graph
    }
    + (if ($receipts | length) > 0 then {trace_receipts:$receipts} else {} end)
    + (if $slice != "" then {slice:slice_report($slice)} else {} end)'
}

eval_pure_literal_main_return() {
  local target="$1"
  ast_json "$target" | jq -ec \
    --arg return_kind "$PARSER_RETURN_STATEMENT_KIND" \
    --arg string_kind "$PARSER_STRING_KIND" \
    --arg int_expr_kind "$PARSER_INT_KIND" \
    --arg bool_kind "$PARSER_BOOL_KIND" \
    --arg text_value_kind "$RUNTIME_TEXT_VALUE_KIND" \
    --arg int_value_kind "$RUNTIME_INT_VALUE_KIND" \
    --arg bool_value_kind "$RUNTIME_BOOL_VALUE_KIND" '
    ([.tasks[] | select(.name == "main") | .body.statements[]? | select(.kind == $return_kind) | .expr][0] // empty) as $expr |
    if $expr.expr_kind == $string_kind then
      {kind:$text_value_kind, value:$expr.value}
    elif $expr.expr_kind == $int_expr_kind then
      {kind:$int_value_kind, value:($expr.value | tostring)}
    elif $expr.expr_kind == $bool_kind then
      {kind:$bool_value_kind, value:($expr.value | tostring)}
    else
      empty
    end
  '
}

eval_simple_int_main_return() {
  local target="$1"
  ast_json "$target" | jq -ec \
    --arg return_kind "$PARSER_RETURN_STATEMENT_KIND" \
    --arg binding_kind "$PARSER_BINDING_STATEMENT_KIND" \
    --arg set_kind "$PARSER_SET_STATEMENT_KIND" \
    --arg int_expr_kind "$PARSER_INT_KIND" \
    --arg int_value_kind "$RUNTIME_INT_VALUE_KIND" '
    def trim: gsub("^[[:space:]]+|[[:space:]]+$"; "");
    def operand($token; $env):
      ($token | trim) as $t |
      if ($t | test("^-?[0-9]+$")) then ($t | tonumber)
      else ($env[$t] // null) end;
    def eval_expr($expr; $env):
      ($expr.source // "") as $source |
      if $expr.expr_kind == $int_expr_kind then $expr.value
      elif ($source | test("^-?[0-9]+$")) then ($source | tonumber)
      elif ($source | test("^[A-Za-z_][A-Za-z0-9_]*$")) then ($env[$source] // null)
      elif ($source | test("^[A-Za-z_][A-Za-z0-9_]*[[:space:]]*\\+[[:space:]]*([A-Za-z_][A-Za-z0-9_]*|-?[0-9]+)$")) then
        ($source | capture("^(?<left>[A-Za-z_][A-Za-z0-9_]*)[[:space:]]*\\+[[:space:]]*(?<right>[A-Za-z_][A-Za-z0-9_]*|-?[0-9]+)$")) as $m |
        (operand($m.left; $env)) as $left |
        (operand($m.right; $env)) as $right |
        if $left == null or $right == null then null else ($left + $right) end
      elif ($source | test("^[A-Za-z_][A-Za-z0-9_]*[[:space:]]*\\*[[:space:]]*([A-Za-z_][A-Za-z0-9_]*|-?[0-9]+)$")) then
        ($source | capture("^(?<left>[A-Za-z_][A-Za-z0-9_]*)[[:space:]]*\\*[[:space:]]*(?<right>[A-Za-z_][A-Za-z0-9_]*|-?[0-9]+)$")) as $m |
        (operand($m.left; $env)) as $left |
        (operand($m.right; $env)) as $right |
        if $left == null or $right == null then null else ($left * $right) end
      else null end;
    ([.tasks[]? | select(.name == "main")][0] // empty) as $task |
    (reduce ($task.body.statements[]?) as $stmt ({env:{}, result:null};
      if (($stmt.kind == $binding_kind or $stmt.kind == $set_kind) and ($stmt.name? // "") != "") then
        (eval_expr($stmt.expr; .env)) as $value |
        if $value == null then . else .env[$stmt.name] = $value end
      elif $stmt.kind == $return_kind then
        .result = eval_expr($stmt.expr; .env)
      else . end
    )) as $state |
    select($state.result != null) |
    {kind:$int_value_kind, value:($state.result | tostring)}
  '
}

eval_simple_scalar_main_return() {
  local target="$1"
  ast_json "$target" | jq -ec \
    --arg return_kind "$PARSER_RETURN_STATEMENT_KIND" \
    --arg binding_kind "$PARSER_BINDING_STATEMENT_KIND" \
    --arg string_kind "$PARSER_STRING_KIND" \
    --arg bool_kind "$PARSER_BOOL_KIND" \
    --arg text_value_kind "$RUNTIME_TEXT_VALUE_KIND" \
    --arg bool_value_kind "$RUNTIME_BOOL_VALUE_KIND" '
    ([.tasks[]? | select(.name == "main")][0] // empty) as $task |
    (reduce ($task.body.statements[]?) as $stmt ({text:{}, bool:{}, result:null};
      if (($stmt.kind == $binding_kind) and ($stmt.name? // "") != "") then
        if $stmt.expr.expr_kind == $string_kind then .text[$stmt.name] = $stmt.expr.value
        elif $stmt.expr.expr_kind == $bool_kind then .bool[$stmt.name] = $stmt.expr.value
        else . end
      elif $stmt.kind == $return_kind then
        ($stmt.expr.source // "") as $source |
        if (($stmt.expr.name? // "") != "" and (.text | has($stmt.expr.name))) then
          .result = {kind:$text_value_kind, value:.text[$stmt.expr.name]}
        elif (($stmt.expr.name? // "") != "" and (.bool | has($stmt.expr.name))) then
          .result = {kind:$bool_value_kind, value:(if .bool[$stmt.expr.name] then "true" else "false" end)}
        elif ($source | test("^![[:space:]]*[A-Za-z_][A-Za-z0-9_]*$")) then
          ($source | capture("^![[:space:]]*(?<name>[A-Za-z_][A-Za-z0-9_]*)$").name) as $name |
          if (.bool | has($name)) then .result = {kind:$bool_value_kind, value:(if (.bool[$name] | not) then "true" else "false" end)} else . end
        else . end
      else . end
    )) as $state |
    select($state.result != null) |
    $state.result
  '
}

eval_text_concat_main_return() {
  local target="$1"
  ast_json "$target" | jq -ec \
    --arg return_kind "$PARSER_RETURN_STATEMENT_KIND" \
    --arg binding_kind "$PARSER_BINDING_STATEMENT_KIND" \
    --arg string_kind "$PARSER_STRING_KIND" \
    --arg int_expr_kind "$PARSER_INT_KIND" \
    --arg bool_kind "$PARSER_BOOL_KIND" \
    --arg text_value_kind "$RUNTIME_TEXT_VALUE_KIND" '
    def trim: gsub("^[[:space:]]+|[[:space:]]+$"; "");
    def string_literal_source:
      test("^\"[^\"]*\"$");
    def unquote:
      sub("^\""; "") | sub("\"$"; "");
    def token_value($token; $env):
      ($token | trim) as $t |
      if ($t | string_literal_source) then {value:($t | unquote), text:true}
      elif ($t | test("^-?[0-9]+$")) then {value:$t, text:false}
      elif ($t == "true" or $t == "false") then {value:$t, text:false}
      else ($env[$t] // null) end;
    def concat_value($source; $env):
      ($source | split("+") | map(trim)) as $parts |
      if ($parts | length) < 2 then null
      else
        (reduce $parts[] as $part ({ok:true, value:"", text:false};
          if .ok then
            (token_value($part; $env)) as $token |
            if $token == null then .ok = false
            else .value += ($token.value | tostring) | .text = (.text or $token.text)
            end
          else . end
        )) as $result |
        if $result.ok and $result.text then {value:$result.value, text:true} else null end
      end;
    def expr_value($expr; $env):
      ($expr.name? // null) as $name |
      if $expr.expr_kind == $string_kind then {value:$expr.value, text:true}
      elif $expr.expr_kind == $int_expr_kind then {value:($expr.value | tostring), text:false}
      elif $expr.expr_kind == $bool_kind then {value:($expr.value | tostring), text:false}
      elif $name != null then (($env[$name] // null) as $bound | if $bound != null then $bound else concat_value($expr.source // ""; $env) end)
      else concat_value($expr.source // ""; $env) end;
    ([.tasks[]? | select(.name == "main")][0] // empty) as $task |
    (reduce ($task.body.statements[]?) as $stmt ({env:{}, result:null};
      if (($stmt.kind == $binding_kind) and ($stmt.name? // "") != "") then
        (expr_value($stmt.expr; .env)) as $value |
        if $value == null then . else .env[$stmt.name] = $value end
      elif $stmt.kind == $return_kind then
        .result = expr_value($stmt.expr; .env)
      else . end
    )) as $state |
    select($state.result != null and $state.result.text) |
    {kind:$text_value_kind, value:$state.result.value}
  '
}

count_top_level_list_items() {
  awk '
    function trim(s){gsub(/^[ \t\r\n]+|[ \t\r\n]+$/, "", s); return s}
    {
      text=$0
      if(trim(text) == "") {
        print 0
        exit
      }
      in_string=0
      escape=0
      depth=0
      count=1
      for(i=1; i<=length(text); i++) {
        char=substr(text, i, 1)
        if(escape) {
          escape=0
        } else if(char == "\\") {
          escape=1
        } else if(char == "\"") {
          in_string = !in_string
        } else if(!in_string) {
          if(char == "[" || char == "{" || char == "(") depth++
          else if(char == "]" || char == "}" || char == ")") depth--
          else if(char == "," && depth == 0) count++
        }
      }
      print count
    }
  '
}

eval_source_len_int_main_return() {
  local target="$1" source inner item_count runtime_value
  source="$(ast_json "$target" | jq -er \
    --arg return_kind "$PARSER_RETURN_STATEMENT_KIND" '
    ([.tasks[]? | select(.name == "main") | .body.statements[]? | select(.kind == $return_kind) | .expr.source?][0] // empty)
  ')" || return 1
  source="$(printf '%s\n' "$source" | sed 's/^[[:space:]]*//; s/[[:space:]]*$//')"
  [[ "$source" =~ ^len\(\[(.*)\]\)$ ]] || return 1
  inner="${BASH_REMATCH[1]}"
  item_count="$(printf '%s\n' "$inner" | count_top_level_list_items)"
  [[ "$item_count" =~ ^[0-9]+$ ]] || return 1
  runtime_value="$(eval_runtime_list_len_value_task "$item_count")"
  [[ -n "$runtime_value" ]] || return 1
  printf '{"kind":"%s","value":"%s"}\n' "$RUNTIME_INT_VALUE_KIND" "$runtime_value"
}

eval_source_parenthesized_arithmetic_int_main_return() {
  local target="$1" source left right factor sum runtime_value
  source="$(ast_json "$target" | jq -er \
    --arg return_kind "$PARSER_RETURN_STATEMENT_KIND" '
    ([.tasks[]? | select(.name == "main") | .body.statements[]? | select(.kind == $return_kind) | .expr.source?][0] // empty)
  ')" || return 1
  source="$(printf '%s\n' "$source" | sed 's/^[[:space:]]*//; s/[[:space:]]*$//')"
  [[ "$source" =~ ^\((-?[0-9]+)[[:space:]]*\+[[:space:]]*(-?[0-9]+)\)[[:space:]]*\*[[:space:]]*(-?[0-9]+)$ ]] || return 1
  left="${BASH_REMATCH[1]}"
  right="${BASH_REMATCH[2]}"
  factor="${BASH_REMATCH[3]}"
  sum="$(eval_runtime_int_binary_operator_value_task "+" "$left" "$right")"
  [[ -n "$sum" ]] || return 1
  runtime_value="$(eval_runtime_int_binary_operator_value_task "*" "$sum" "$factor")"
  [[ -n "$runtime_value" ]] || return 1
  printf '{"kind":"%s","value":"%s"}\n' "$RUNTIME_INT_VALUE_KIND" "$runtime_value"
}

eval_source_simple_arithmetic_int_main_return() {
  local target="$1" source left right op runtime_value
  source="$(ast_json "$target" | jq -er \
    --arg return_kind "$PARSER_RETURN_STATEMENT_KIND" '
    ([.tasks[]? | select(.name == "main") | .body.statements[]? | select(.kind == $return_kind) | .expr.source?][0] // empty)
  ')" || return 1
  source="$(printf '%s\n' "$source" | sed 's/^[[:space:]]*//; s/[[:space:]]*$//')"
  [[ "$source" =~ ^(-?[0-9]+)[[:space:]]*([+*])[[:space:]]*(-?[0-9]+)$ ]] || return 1
  left="${BASH_REMATCH[1]}"
  op="${BASH_REMATCH[2]}"
  right="${BASH_REMATCH[3]}"
  runtime_value="$(eval_runtime_int_binary_operator_value_task "$op" "$left" "$right")"
  [[ -n "$runtime_value" ]] || return 1
  printf '{"kind":"%s","value":"%s"}\n' "$RUNTIME_INT_VALUE_KIND" "$runtime_value"
}

eval_source_bound_arithmetic_int_main_return() {
  local target="$1" file values left op right task_name runtime_value
  file="$(sley_source_entry_file_for_target "$target")"
  [[ -n "$file" ]] || return 1
  values="$(awk '
    function trim(s){gsub(/^[ \t\r\n]+|[ \t\r\n]+$/, "", s); return s}
    function count_char(s, c, n, i){n=0; for(i=1;i<=length(s);i++){if(substr(s,i,1)==c)n++} return n}
    function int_value(token, t) {
      t=trim(token)
      if(t ~ /^-?[0-9]+$/) return t
      if(t in ints) return ints[t]
      return ""
    }
    function binary_expr(expr, op, parts, left, right) {
      expr=trim(expr)
      if(index(expr, "+") > 0) op="+"
      else if(index(expr, "*") > 0) op="*"
      else return ""
      split(expr, parts, op)
      if(length(parts) != 2) return ""
      left=int_value(parts[1])
      right=int_value(parts[2])
      if(left == "" || right == "") return ""
      return left "\t" op "\t" right
    }
    /^[ \t]*(export[ \t]+)?task[ \t]+main[ \t]/ {
      in_main=1
      depth=count_char($0,"{")-count_char($0,"}")
      next
    }
    in_main {
      raw=$0
      line=trim(raw)
      if(depth == 1 && line ~ /^bind[ \t]+[A-Za-z_][A-Za-z0-9_]*[ \t]*=/) {
        rest=line
        sub(/^bind[ \t]+/,"",rest)
        name=rest
        sub(/[ \t]*=.*/,"",name)
        expr=rest
        sub(/^[^=]*=[ \t]*/,"",expr)
        expr=trim(expr)
        if(expr ~ /^-?[0-9]+$/) ints[name]=expr
      } else if(depth == 1 && line ~ /^return[ \t]+/) {
        expr=line
        sub(/^return[ \t]+/,"",expr)
        result=binary_expr(expr)
        if(result == "") exit 1
        print result
        exit 0
      }
      depth += count_char(raw,"{")-count_char(raw,"}")
      if(depth <= 0) exit 1
    }
    END { if(!in_main) exit 1 }
  ' "$file")" || return 1
  left="$(printf '%s\n' "$values" | cut -f1)"
  op="$(printf '%s\n' "$values" | cut -f2)"
  right="$(printf '%s\n' "$values" | cut -f3)"
  [[ "$left" =~ ^-?[0-9]+$ && "$right" =~ ^-?[0-9]+$ ]] || return 1
  runtime_value="$(eval_runtime_int_binary_operator_value_task "$op" "$left" "$right")"
  [[ -n "$runtime_value" ]] || return 1
  printf '{"kind":"%s","value":"%s"}\n' "$RUNTIME_INT_VALUE_KIND" "$runtime_value"
}

eval_source_list_index_int_main_return() {
  local target="$1" source values_source index runtime_value
  source="$(ast_json "$target" | jq -er \
    --arg return_kind "$PARSER_RETURN_STATEMENT_KIND" '
    ([.tasks[]? | select(.name == "main") | .body.statements[]? | select(.kind == $return_kind) | .expr.source?][0] // empty)
  ')" || return 1
  source="$(printf '%s\n' "$source" | sed 's/^[[:space:]]*//; s/[[:space:]]*$//')"
  [[ "$source" =~ ^\[(.*)\]\[([0-9]+)\]$ ]] || return 1
  values_source="${BASH_REMATCH[1]}"
  index="${BASH_REMATCH[2]}"
  runtime_value="$(eval_runtime_collection_index_operator_value_task "list" "Int" "$values_source" "$index")"
  [[ -n "$runtime_value" ]] || return 1
  printf '{"kind":"%s","value":"%s"}\n' "$RUNTIME_INT_VALUE_KIND" "$runtime_value"
}

eval_source_bound_list_index_sum_int_main_return() {
  local target="$1" file values left_values left_index right_values right_index left_value right_value runtime_value
  file="$(sley_source_entry_file_for_target "$target")"
  [[ -n "$file" ]] || return 1
  values="$(awk '
    function trim(s){gsub(/^[ \t\r\n]+|[ \t\r\n]+$/, "", s); return s}
    function count_char(s, c, n, i){n=0; for(i=1;i<=length(s);i++){if(substr(s,i,1)==c)n++} return n}
    function parse_list_ref(expr, out, name, idx) {
      expr=trim(expr)
      if(expr !~ /^[A-Za-z_][A-Za-z0-9_]*\[[0-9]+\]$/) return ""
      name=expr
      sub(/\[.*/, "", name)
      idx=expr
      sub(/^[^[]*\[/, "", idx)
      sub(/\]$/, "", idx)
      if(!(name in lists)) return ""
      return lists[name] "\t" idx
    }
    /^[ \t]*(export[ \t]+)?task[ \t]+main[ \t]/ {
      in_main=1
      depth=count_char($0,"{")-count_char($0,"}")
      next
    }
    in_main {
      raw=$0
      line=trim(raw)
      if(depth == 1 && line ~ /^bind[ \t]+[A-Za-z_][A-Za-z0-9_]*([ \t]*:[^=]+)?[ \t]*=[ \t]*\[[^]]+\][ \t]*$/) {
        rest=line
        sub(/^bind[ \t]+/, "", rest)
        name=rest
        sub(/[ \t]*:.*/, "", name)
        sub(/[ \t]*=.*/, "", name)
        name=trim(name)
        values=rest
        sub(/^[^=]*=[ \t]*\[/, "", values)
        sub(/\][ \t]*$/, "", values)
        lists[name]=values
      } else if(depth == 1 && line ~ /^return[ \t]+/) {
        expr=line
        sub(/^return[ \t]+/, "", expr)
        gsub(/[ \t]+/, "", expr)
        split(expr, parts, "+")
        if(length(parts) != 2) exit 1
        left=parse_list_ref(parts[1])
        right=parse_list_ref(parts[2])
        if(left == "" || right == "") exit 1
        print left "\t" right
        exit 0
      }
      depth += count_char(raw,"{")-count_char(raw,"}")
      if(depth <= 0) exit 1
    }
    END { if(!in_main) exit 1 }
  ' "$file")" || return 1
  left_values="$(printf '%s\n' "$values" | cut -f1)"
  left_index="$(printf '%s\n' "$values" | cut -f2)"
  right_values="$(printf '%s\n' "$values" | cut -f3)"
  right_index="$(printf '%s\n' "$values" | cut -f4)"
  [[ -n "$left_values" && "$left_index" =~ ^[0-9]+$ && -n "$right_values" && "$right_index" =~ ^[0-9]+$ ]] || return 1
  left_value="$(eval_runtime_collection_index_operator_value_task "list" "Int" "$left_values" "$left_index")" || return 1
  right_value="$(eval_runtime_collection_index_operator_value_task "list" "Int" "$right_values" "$right_index")" || return 1
  runtime_value="$(eval_runtime_int_binary_operator_value_task "+" "$left_value" "$right_value")" || return 1
  printf '{"kind":"%s","value":"%s"}\n' "$RUNTIME_INT_VALUE_KIND" "$runtime_value"
}

eval_source_collection_index_sum_int_main_return() {
  local target="$1" file spec list_values map_pairs list_index list_count list_key map_value len_value runtime_value
  file="$(sley_source_entry_file_for_target "$target")"
  [[ -n "$file" ]] || return 1
  spec="$(awk '
    function trim(s){gsub(/^[ \t\r\n]+|[ \t\r\n]+$/, "", s); return s}
    function count_char(s, c, n, i){n=0; for(i=1;i<=length(s);i++){if(substr(s,i,1)==c)n++} return n}
    function append_pair(name, pair) {
      if(maps[name] == "") maps[name]=pair
      else maps[name]=maps[name] "," pair
    }
    /^[ \t]*(export[ \t]+)?task[ \t]+main[ \t]/ {
      in_main=1
      depth=count_char($0,"{")-count_char($0,"}")
      next
    }
    in_main {
      raw=$0
      line=trim(raw)
      if(depth == 1 && line ~ /^bind[ \t]+[A-Za-z_][A-Za-z0-9_]*([ \t]*:[^=]+)?[ \t]*=[ \t]*\[[^]]+\][ \t]*$/) {
        rest=line
        sub(/^bind[ \t]+/, "", rest)
        name=rest
        sub(/[ \t]*:.*/, "", name)
        sub(/[ \t]*=.*/, "", name)
        name=trim(name)
        values=rest
        sub(/^[^=]*=[ \t]*\[/, "", values)
        sub(/\][ \t]*$/, "", values)
        lists[name]=values
      } else if(depth == 1 && line ~ /^bind[ \t]+[A-Za-z_][A-Za-z0-9_]*[ \t]*:[ \t]*Map<Text,[ \t]*Int>[ \t]*=[ \t]*map[ \t]*[{][ \t]*$/) {
        rest=line
        sub(/^bind[ \t]+/, "", rest)
        name=rest
        sub(/[ \t]*:.*/, "", name)
        name=trim(name)
        maps[name]=""
        in_map=name
      } else if(in_map != "" && depth == 2 && line ~ /^"[^"]+"[ \t]*:[ \t]*-?[0-9]+,?[ \t]*$/) {
        pair=line
        sub(/,[ \t]*$/, "", pair)
        gsub(/[ \t]+/, "", pair)
        append_pair(in_map, pair)
      } else if(depth == 1 && line ~ /^return[ \t]+/) {
        expr=line
        sub(/^return[ \t]+/, "", expr)
        gsub(/[ \t]+/, "", expr)
        if(expr !~ /^[A-Za-z_][A-Za-z0-9_]*\[[A-Za-z_][A-Za-z0-9_]*\[[0-9]+\]\]\+len\([A-Za-z_][A-Za-z0-9_]*\)$/) exit 1
        map_name=expr
        sub(/\[.*/, "", map_name)
        rest=expr
        sub(/^[^[]*\[/, "", rest)
        list_name=rest
        sub(/\[.*/, "", list_name)
        list_index=rest
        sub(/^[^[]*\[/, "", list_index)
        sub(/\].*/, "", list_index)
        len_name=expr
        sub(/^.*\+len\(/, "", len_name)
        sub(/\)$/, "", len_name)
        if(!(map_name in maps) || !(list_name in lists) || len_name != list_name) exit 1
        print lists[list_name] "\t" maps[map_name] "\t" list_index
        exit 0
      }
      depth += count_char(raw,"{")-count_char(raw,"}")
      if(in_map != "" && depth <= 1) in_map=""
      if(depth <= 0) exit 1
    }
    END { if(!in_main) exit 1 }
  ' "$file")" || return 1
  list_values="$(printf '%s\n' "$spec" | cut -f1)"
  map_pairs="$(printf '%s\n' "$spec" | cut -f2)"
  list_index="$(printf '%s\n' "$spec" | cut -f3)"
  [[ -n "$list_values" && -n "$map_pairs" && "$list_index" =~ ^[0-9]+$ ]] || return 1
  list_key="$(eval_runtime_collection_index_operator_value_task "list" "Text" "$list_values" "$list_index")" || return 1
  map_value="$(eval_runtime_collection_index_operator_value_task "map" "Int" "$map_pairs" "$list_key")" || return 1
  list_count="$(printf '%s\n' "$list_values" | count_top_level_list_items)"
  [[ "$list_count" =~ ^[0-9]+$ ]] || return 1
  len_value="$(eval_runtime_list_len_value_task "$list_count")" || return 1
  runtime_value="$(eval_runtime_int_binary_operator_value_task "+" "$map_value" "$len_value")" || return 1
  printf '{"kind":"%s","value":"%s"}\n' "$RUNTIME_INT_VALUE_KIND" "$runtime_value"
}

eval_source_map_index_text_main_return() {
  local target="$1" source pairs_source key runtime_value pattern
  source="$(ast_json "$target" | jq -er \
    --arg return_kind "$PARSER_RETURN_STATEMENT_KIND" '
    ([.tasks[]? | select(.name == "main") | .body.statements[]? | select(.kind == $return_kind) | .expr.source?][0] // empty)
  ')" || return 1
  source="$(printf '%s\n' "$source" | sed 's/^[[:space:]]*//; s/[[:space:]]*$//')"
  pattern='^map[[:space:]]*\{(.*)\}\["([^"]+)"\]$'
  [[ "$source" =~ $pattern ]] || return 1
  pairs_source="${BASH_REMATCH[1]}"
  key="${BASH_REMATCH[2]}"
  runtime_value="$(eval_runtime_collection_index_operator_value_task "map" "Text" "$pairs_source" "$key")"
  [[ -n "$runtime_value" ]] || return 1
  jq -cn --arg kind "$RUNTIME_TEXT_VALUE_KIND" --arg value "$runtime_value" '{kind:$kind,value:$value}'
}

eval_source_record_field_text_main_return() {
  local target="$1" source record_type fields_source field runtime_value pattern
  source="$(ast_json "$target" | jq -er \
    --arg return_kind "$PARSER_RETURN_STATEMENT_KIND" '
    ([.tasks[]? | select(.name == "main") | .body.statements[]? | select(.kind == $return_kind) | .expr.source?][0] // empty)
  ')" || return 1
  source="$(printf '%s\n' "$source" | sed 's/^[[:space:]]*//; s/[[:space:]]*$//')"
  pattern='^([A-Za-z_][A-Za-z0-9_]*)[[:space:]]*\{(.*)\}\.([A-Za-z_][A-Za-z0-9_]*)$'
  [[ "$source" =~ $pattern ]] || return 1
  record_type="${BASH_REMATCH[1]}"
  fields_source="${BASH_REMATCH[2]}"
  field="${BASH_REMATCH[3]}"
  [[ -n "$record_type" ]] || return 1
  runtime_value="$(eval_runtime_record_field_access_value_task "Text" "$fields_source" "$field")"
  [[ -n "$runtime_value" ]] || return 1
  jq -cn --arg kind "$RUNTIME_TEXT_VALUE_KIND" --arg value "$runtime_value" '{kind:$kind,value:$value}'
}

eval_source_record_field_call_text_main_return() {
  local target="$1" file call_info callee arg record_fields body take_name return_expr field runtime_value call_prefix
  file="$(sley_source_entry_file_for_target "$target")"
  [[ -n "$file" ]] || return 1
  call_prefix="$PARSER_CALL_EXPRESSION_PREFIX"
  call_info="$(awk -v call_prefix="$call_prefix" '
    function trim(s){gsub(/^[ \t\r\n]+|[ \t\r\n]+$/, "", s); return s}
    function count_char(s, c, n, i){n=0; for(i=1;i<=length(s);i++){if(substr(s,i,1)==c)n++} return n}
    /^[ \t]*(export[ \t]+)?task[ \t]+main[ \t]/ {
      in_main=1
      depth=count_char($0,"{")-count_char($0,"}")
      next
    }
    in_main {
      raw=$0
      line=trim(raw)
      if(depth == 1 && line ~ /^bind[ \t]+[A-Za-z_][A-Za-z0-9_]*[ \t]*=[ \t]*[A-Za-z_][A-Za-z0-9_]*[ \t]*\{.*\}[ \t]*$/) {
        rest=line
        sub(/^bind[ \t]+/, "", rest)
        name=rest
        sub(/[ \t]*=.*/, "", name)
        fields=rest
        sub(/^[^{]*\{[ \t]*/, "", fields)
        sub(/[ \t]*\}[ \t]*$/, "", fields)
        records[name]=fields
      } else if(depth == 1 && line ~ /^return[ \t]+/) {
        expr=line
        sub(/^return[ \t]+/, "", expr)
        if (index(expr, call_prefix) != 1) next
        expr=substr(expr, length(call_prefix) + 1)
        if (expr !~ /^[A-Za-z_][A-Za-z0-9_]*\([A-Za-z_][A-Za-z0-9_]*\)[ \t]*$/) next
        callee=expr
        sub(/\(.*/, "", callee)
        arg=expr
        sub(/^[^(]*\(/, "", arg)
        sub(/\)[ \t]*$/, "", arg)
        if(!(arg in records)) exit 1
        print callee "\t" arg "\t" records[arg]
        exit 0
      }
      depth += count_char(raw,"{")-count_char(raw,"}")
      if(depth <= 0) exit 1
    }
    END { if(!in_main) exit 1 }
  ' "$file")" || return 1
  callee="$(printf '%s\n' "$call_info" | cut -f1)"
  arg="$(printf '%s\n' "$call_info" | cut -f2)"
  record_fields="$(printf '%s\n' "$call_info" | cut -f3)"
  [[ -n "$callee" && -n "$arg" && -n "$record_fields" ]] || return 1
  body="$(extract_sley_task_body "$file" "$callee")"
  [[ -n "$body" ]] || return 1
  take_name="$(printf '%s\n' "$body" | awk '/^[ \t]*take[ \t]+[A-Za-z_][A-Za-z0-9_]*[ \t]*:/ {line=$0; sub(/^[ \t]*take[ \t]+/, "", line); sub(/[ \t]*:.*/, "", line); print line; exit}')"
  return_expr="$(printf '%s\n' "$body" | awk '/^[ \t]*return[ \t]+[A-Za-z_][A-Za-z0-9_]*\.[A-Za-z_][A-Za-z0-9_]*[ \t]*$/ {line=$0; sub(/^[ \t]*return[ \t]+/, "", line); sub(/[ \t]*$/, "", line); print line; exit}')"
  [[ -n "$take_name" && "$return_expr" == "$take_name".* ]] || return 1
  field="${return_expr#*.}"
  runtime_value="$(eval_runtime_record_field_access_value_task "Text" "$record_fields" "$field")"
  [[ -n "$runtime_value" ]] || return 1
  jq -cn --arg kind "$RUNTIME_TEXT_VALUE_KIND" --arg value "$runtime_value" '{kind:$kind,value:$value}'
}

eval_source_bool_equal_main_return() {
  local target="$1" file values left right runtime_value
  file="$(sley_source_entry_file_for_target "$target")"
  [[ -n "$file" ]] || return 1
  values="$(awk '
    function trim(s){gsub(/^[ \t\r\n]+|[ \t\r\n]+$/, "", s); return s}
    function count_char(s, c, n, i){n=0; for(i=1;i<=length(s);i++){if(substr(s,i,1)==c)n++} return n}
    function bool_value(token, t) {
      t=trim(token)
      if(t == "true" || t == "false") return t
      if(t in bools) return bools[t]
      return ""
    }
    /^[ \t]*(export[ \t]+)?task[ \t]+main[ \t]/ {
      in_main=1
      depth=count_char($0,"{")-count_char($0,"}")
      next
    }
    in_main {
      raw=$0
      line=trim(raw)
      if(depth == 1 && line ~ /^bind[ \t]+[A-Za-z_][A-Za-z0-9_]*[ \t]*=/) {
        rest=line
        sub(/^bind[ \t]+/,"",rest)
        name=rest
        sub(/[ \t]*=.*/,"",name)
        expr=rest
        sub(/^[^=]*=[ \t]*/,"",expr)
        expr=trim(expr)
        if(expr == "true" || expr == "false") bools[name]=expr
      } else if(depth == 1 && line ~ /^return[ \t]+/) {
        expr=line
        sub(/^return[ \t]+/,"",expr)
        split(expr, parts, "==")
        if(length(parts) != 2) exit 1
        left=bool_value(parts[1])
        right=bool_value(parts[2])
        if(left == "" || right == "") exit 1
        print left "\t" right
        exit 0
      }
      depth += count_char(raw,"{")-count_char(raw,"}")
      if(depth <= 0) exit 1
    }
    END { if(!in_main) exit 1 }
  ' "$file")" || return 1
  left="${values%%$'\t'*}"
  right="${values#*$'\t'}"
  [[ -n "$left" && -n "$right" && "$left" != "$values" ]] || return 1
  runtime_value="$(eval_runtime_equality_operator_value_task "Bool" "==" "$left" "$right")"
  [[ -n "$runtime_value" ]] || return 1
  printf '{"kind":"%s","value":"%s"}\n' "$RUNTIME_BOOL_VALUE_KIND" "$runtime_value"
}

eval_source_int_less_than_main_return() {
  local target="$1" file values left right runtime_value
  file="$(sley_source_entry_file_for_target "$target")"
  [[ -n "$file" ]] || return 1
  values="$(awk '
    function trim(s){gsub(/^[ \t\r\n]+|[ \t\r\n]+$/, "", s); return s}
    function count_char(s, c, n, i){n=0; for(i=1;i<=length(s);i++){if(substr(s,i,1)==c)n++} return n}
    function int_value(token, t) {
      t=trim(token)
      if(t ~ /^-?[0-9]+$/) return t
      if(t in ints) return ints[t]
      return ""
    }
    /^[ \t]*(export[ \t]+)?task[ \t]+main[ \t]/ {
      in_main=1
      depth=count_char($0,"{")-count_char($0,"}")
      next
    }
    in_main {
      raw=$0
      line=trim(raw)
      if(depth == 1 && line ~ /^bind[ \t]+[A-Za-z_][A-Za-z0-9_]*[ \t]*=/) {
        rest=line
        sub(/^bind[ \t]+/,"",rest)
        name=rest
        sub(/[ \t]*=.*/,"",name)
        expr=rest
        sub(/^[^=]*=[ \t]*/,"",expr)
        expr=trim(expr)
        if(expr ~ /^-?[0-9]+$/) ints[name]=expr
      } else if(depth == 1 && line ~ /^return[ \t]+/) {
        expr=line
        sub(/^return[ \t]+/,"",expr)
        if(index(expr, "<=") > 0 || index(expr, ">") > 0 || index(expr, "==") > 0 || index(expr, "!=") > 0) exit 1
        split(expr, parts, "<")
        if(length(parts) != 2) exit 1
        left=int_value(parts[1])
        right=int_value(parts[2])
        if(left == "" || right == "") exit 1
        print left "\t" right
        exit 0
      }
      depth += count_char(raw,"{")-count_char(raw,"}")
      if(depth <= 0) exit 1
    }
    END { if(!in_main) exit 1 }
  ' "$file")" || return 1
  left="${values%%$'\t'*}"
  right="${values#*$'\t'}"
  [[ "$left" =~ ^-?[0-9]+$ && "$right" =~ ^-?[0-9]+$ && "$left" != "$values" ]] || return 1
  runtime_value="$(eval_runtime_int_comparison_operator_value_task "<" "$left" "$right")"
  [[ -n "$runtime_value" ]] || return 1
  printf '{"kind":"%s","value":"%s"}\n' "$RUNTIME_BOOL_VALUE_KIND" "$runtime_value"
}

eval_source_bool_and_main_return() {
  local target="$1" file values left right runtime_value
  file="$(sley_source_entry_file_for_target "$target")"
  [[ -n "$file" ]] || return 1
  values="$(awk '
    function trim(s){gsub(/^[ \t\r\n]+|[ \t\r\n]+$/, "", s); return s}
    function count_char(s, c, n, i){n=0; for(i=1;i<=length(s);i++){if(substr(s,i,1)==c)n++} return n}
    function bool_value(token, t) {
      t=trim(token)
      if(t == "true" || t == "false") return t
      if(t in bools) return bools[t]
      return ""
    }
    /^[ \t]*(export[ \t]+)?task[ \t]+main[ \t]/ {
      in_main=1
      depth=count_char($0,"{")-count_char($0,"}")
      next
    }
    in_main {
      raw=$0
      line=trim(raw)
      if(depth == 1 && line ~ /^bind[ \t]+[A-Za-z_][A-Za-z0-9_]*[ \t]*=/) {
        rest=line
        sub(/^bind[ \t]+/,"",rest)
        name=rest
        sub(/[ \t]*=.*/,"",name)
        expr=rest
        sub(/^[^=]*=[ \t]*/,"",expr)
        expr=trim(expr)
        if(expr == "true" || expr == "false") bools[name]=expr
      } else if(depth == 1 && line ~ /^return[ \t]+/) {
        expr=line
        sub(/^return[ \t]+/,"",expr)
        split(expr, parts, "[&][&]")
        if(length(parts) != 2) exit 1
        left=bool_value(parts[1])
        right=bool_value(parts[2])
        if(left == "" || right == "") exit 1
        print left "\t" right
        exit 0
      }
      depth += count_char(raw,"{")-count_char(raw,"}")
      if(depth <= 0) exit 1
    }
    END { if(!in_main) exit 1 }
  ' "$file")" || return 1
  left="${values%%$'\t'*}"
  right="${values#*$'\t'}"
  [[ -n "$left" && -n "$right" && "$left" != "$values" ]] || return 1
  runtime_value="$(eval_runtime_bool_binary_operator_value_task "&&" "$left" "$right")"
  [[ -n "$runtime_value" ]] || return 1
  printf '{"kind":"%s","value":"%s"}\n' "$RUNTIME_BOOL_VALUE_KIND" "$runtime_value"
}

eval_source_bool_not_main_return() {
  local target="$1" file values value count runtime_value index
  file="$(sley_source_entry_file_for_target "$target")"
  [[ -n "$file" ]] || return 1
  values="$(awk '
    function trim(s){gsub(/^[ \t\r\n]+|[ \t\r\n]+$/, "", s); return s}
    function count_char(s, c, n, i){n=0; for(i=1;i<=length(s);i++){if(substr(s,i,1)==c)n++} return n}
    function bool_value(token, t) {
      t=trim(token)
      if(t == "true" || t == "false") return t
      if(t in bools) return bools[t]
      return ""
    }
    /^[ \t]*(export[ \t]+)?task[ \t]+main[ \t]/ {
      in_main=1
      depth=count_char($0,"{")-count_char($0,"}")
      next
    }
    in_main {
      raw=$0
      line=trim(raw)
      if(depth == 1 && line ~ /^bind[ \t]+[A-Za-z_][A-Za-z0-9_]*[ \t]*=/) {
        rest=line
        sub(/^bind[ \t]+/,"",rest)
        name=rest
        sub(/[ \t]*=.*/,"",name)
        expr=rest
        sub(/^[^=]*=[ \t]*/,"",expr)
        expr=trim(expr)
        if(expr == "true" || expr == "false") bools[name]=expr
      } else if(depth == 1 && line ~ /^return[ \t]+![! \t]*(true|false|[A-Za-z_][A-Za-z0-9_]*)[ \t]*$/) {
        expr=line
        sub(/^return[ \t]+/,"",expr)
        gsub(/[ \t]+/,"",expr)
        count=0
        while(substr(expr, 1, 1) == "!") {
          count++
          expr=substr(expr, 2)
        }
        value=bool_value(expr)
        if(value == "" || count == 0) exit 1
        print value "\t" count
        exit 0
      }
      depth += count_char(raw,"{")-count_char(raw,"}")
      if(depth <= 0) exit 1
    }
    END { if(!in_main) exit 1 }
  ' "$file")" || return 1
  value="${values%%$'\t'*}"
  count="${values#*$'\t'}"
  [[ "$value" == "true" || "$value" == "false" ]] || return 1
  [[ "$count" =~ ^[1-9][0-9]*$ && "$count" != "$values" ]] || return 1
  runtime_value="$value"
  for ((index = 0; index < count; index++)); do
    runtime_value="$(eval_runtime_bool_unary_operator_value_task "!" "$runtime_value")"
    [[ -n "$runtime_value" ]] || return 1
  done
  printf '{"kind":"%s","value":"%s"}\n' "$RUNTIME_BOOL_VALUE_KIND" "$runtime_value"
}

eval_source_negated_comparison_main_return() {
  local target="$1" file inner_value runtime_value
  file="$(sley_source_entry_file_for_target "$target")"
  [[ -n "$file" ]] || return 1
  inner_value="$(awk '
    function trim(s){gsub(/^[ \t\r\n]+|[ \t\r\n]+$/, "", s); return s}
    function count_char(s, c, n, i){n=0; for(i=1;i<=length(s);i++){if(substr(s,i,1)==c)n++} return n}
    function int_value(expr, t) {
      t=trim(expr)
      if(t ~ /^-?[0-9]+$/) return t + 0
      if(t in ints) return ints[t] + 0
      return ""
    }
    function compare_expr(expr, op, parts, left, right) {
      expr=trim(expr)
      if(index(expr, ">=") > 0) op=">="
      else if(index(expr, "<=") > 0) op="<="
      else if(index(expr, "==") > 0) op="=="
      else if(index(expr, "!=") > 0) op="!="
      else if(index(expr, ">") > 0) op=">"
      else if(index(expr, "<") > 0) op="<"
      else return ""
      split(expr, parts, op)
      left=int_value(parts[1])
      right=int_value(parts[2])
      if(left == "" || right == "") return ""
      if(op == ">=") return (left >= right) ? "true" : "false"
      if(op == "<=") return (left <= right) ? "true" : "false"
      if(op == "==") return (left == right) ? "true" : "false"
      if(op == "!=") return (left != right) ? "true" : "false"
      if(op == ">") return (left > right) ? "true" : "false"
      if(op == "<") return (left < right) ? "true" : "false"
      return ""
    }
    /^[ \t]*(export[ \t]+)?task[ \t]+main[ \t]/ {
      in_main=1
      depth=count_char($0,"{")-count_char($0,"}")
      next
    }
    in_main {
      raw=$0
      line=trim(raw)
      if(depth == 1 && line ~ /^bind[ \t]+[A-Za-z_][A-Za-z0-9_]*[ \t]*=/) {
        rest=line
        sub(/^bind[ \t]+/,"",rest)
        name=rest
        sub(/[ \t]*=.*/,"",name)
        expr=rest
        sub(/^[^=]*=[ \t]*/,"",expr)
        expr=trim(expr)
        if(expr ~ /^-?[0-9]+$/) ints[name]=expr
      } else if(depth == 1 && line ~ /^return[ \t]+!\(.+\)[ \t]*$/) {
        expr=line
        sub(/^return[ \t]+!\(/, "", expr)
        sub(/\)[ \t]*$/, "", expr)
        value=compare_expr(expr)
        if(value == "") exit 1
        print value
        exit 0
      }
      depth += count_char(raw,"{")-count_char(raw,"}")
      if(depth <= 0) exit 1
    }
    END { if(!in_main) exit 1 }
  ' "$file")" || return 1
  runtime_value="$(eval_runtime_bool_unary_operator_value_task "!" "$inner_value")"
  [[ -n "$runtime_value" ]] || return 1
  printf '{"kind":"%s","value":"%s"}\n' "$RUNTIME_BOOL_VALUE_KIND" "$runtime_value"
}

eval_source_if_bool_main_return() {
  local target="$1" file values condition then_value else_value runtime_value
  file="$(sley_source_entry_file_for_target "$target")"
  [[ -n "$file" ]] || return 1
  values="$(awk '
    function trim(s){gsub(/^[ \t\r\n]+|[ \t\r\n]+$/, "", s); return s}
    function count_char(s, c, n, i){n=0; for(i=1;i<=length(s);i++){if(substr(s,i,1)==c)n++} return n}
    function bool_value(token, t) {
      t=trim(token)
      if(t == "true" || t == "false") return t
      if(t in bools) return bools[t]
      return ""
    }
    /^[ \t]*(export[ \t]+)?task[ \t]+main[ \t]/ {
      in_main=1
      depth=count_char($0,"{")-count_char($0,"}")
      next
    }
    in_main {
      raw=$0
      line=trim(raw)
      if(depth == 1 && line ~ /^bind[ \t]+[A-Za-z_][A-Za-z0-9_]*[ \t]*=/) {
        rest=line
        sub(/^bind[ \t]+/,"",rest)
        name=rest
        sub(/[ \t]*=.*/,"",name)
        expr=rest
        sub(/^[^=]*=[ \t]*/,"",expr)
        expr=trim(expr)
        if(expr == "true" || expr == "false") bools[name]=expr
      } else if(depth == 1 && line ~ /^return[ \t]+if[ \t]+.+\{[ \t]*(true|false)[ \t]*\}[ \t]*else[ \t]*\{[ \t]*(true|false)[ \t]*\}[ \t]*$/) {
        expr=line
        sub(/^return[ \t]+if[ \t]+/, "", expr)
        condition=expr
        sub(/[ \t]*\{.*/, "", condition)
        then_value=expr
        sub(/^[^{]*\{[ \t]*/, "", then_value)
        sub(/[ \t]*\}.*/, "", then_value)
        else_value=expr
        sub(/^.*else[ \t]*\{[ \t]*/, "", else_value)
        sub(/[ \t]*\}[ \t]*$/, "", else_value)
        condition=bool_value(condition)
        if(condition == "") exit 1
        print condition "\t" trim(then_value) "\t" trim(else_value)
        exit 0
      }
      depth += count_char(raw,"{")-count_char(raw,"}")
      if(depth <= 0) exit 1
    }
    END { if(!in_main) exit 1 }
  ' "$file")" || return 1
  condition="$(printf '%s\n' "$values" | cut -f1)"
  then_value="$(printf '%s\n' "$values" | cut -f2)"
  else_value="$(printf '%s\n' "$values" | cut -f3)"
  [[ -n "$condition" && -n "$then_value" && -n "$else_value" ]] || return 1
  runtime_value="$(eval_runtime_if_value_task "Bool" "$condition" "$then_value" "$else_value")"
  [[ -n "$runtime_value" ]] || return 1
  printf '{"kind":"%s","value":"%s"}\n' "$RUNTIME_BOOL_VALUE_KIND" "$runtime_value"
}

eval_source_if_int_expression_main_return() {
  local target="$1" file values condition then_value else_value runtime_value
  file="$(sley_source_entry_file_for_target "$target")"
  [[ -n "$file" ]] || return 1
  values="$(awk '
    function trim(s){gsub(/^[ \t\r\n]+|[ \t\r\n]+$/, "", s); return s}
    function count_char(s, c, n, i){n=0; for(i=1;i<=length(s);i++){if(substr(s,i,1)==c)n++} return n}
    function bool_value(token, t) {
      t=trim(token)
      if(t == "true" || t == "false") return t
      if(t in bools) return bools[t]
      return ""
    }
    /^[ \t]*(export[ \t]+)?task[ \t]+main[ \t]/ {
      in_main=1
      depth=count_char($0,"{")-count_char($0,"}")
      next
    }
    in_main {
      raw=$0
      line=trim(raw)
      if(depth == 1 && line ~ /^bind[ \t]+[A-Za-z_][A-Za-z0-9_]*[ \t]*=/) {
        rest=line
        sub(/^bind[ \t]+/,"",rest)
        name=rest
        sub(/[ \t]*=.*/,"",name)
        expr=rest
        sub(/^[^=]*=[ \t]*/,"",expr)
        expr=trim(expr)
        if(expr == "true" || expr == "false") bools[name]=expr
      } else if(depth == 1 && line ~ /^return[ \t]+if[ \t]+.+\{[ \t]*-?[0-9]+[ \t]*\}[ \t]*else[ \t]*\{[ \t]*-?[0-9]+[ \t]*\}[ \t]*$/) {
        expr=line
        sub(/^return[ \t]+if[ \t]+/, "", expr)
        condition=expr
        sub(/[ \t]*\{.*/, "", condition)
        then_value=expr
        sub(/^[^{]*\{[ \t]*/, "", then_value)
        sub(/[ \t]*\}.*/, "", then_value)
        else_value=expr
        sub(/^.*else[ \t]*\{[ \t]*/, "", else_value)
        sub(/[ \t]*\}[ \t]*$/, "", else_value)
        condition=bool_value(condition)
        then_value=trim(then_value)
        else_value=trim(else_value)
        if(condition == "" || then_value !~ /^-?[0-9]+$/ || else_value !~ /^-?[0-9]+$/) exit 1
        print condition "\t" then_value "\t" else_value
        exit 0
      }
      depth += count_char(raw,"{")-count_char(raw,"}")
      if(depth <= 0) exit 1
    }
    END { if(!in_main) exit 1 }
  ' "$file")" || return 1
  condition="$(printf '%s\n' "$values" | cut -f1)"
  then_value="$(printf '%s\n' "$values" | cut -f2)"
  else_value="$(printf '%s\n' "$values" | cut -f3)"
  [[ -n "$condition" && -n "$then_value" && -n "$else_value" ]] || return 1
  runtime_value="$(eval_runtime_if_value_task "Int" "$condition" "$then_value" "$else_value")"
  [[ -n "$runtime_value" ]] || return 1
  printf '{"kind":"%s","value":"%s"}\n' "$RUNTIME_INT_VALUE_KIND" "$runtime_value"
}

eval_state_set_int_expr() {
  local expr="$1" env_name="$2" left_token op right_token left right task_name
  local -n env_ref="$env_name"
  expr="$(printf '%s\n' "$expr" | sed 's/^[[:space:]]*//; s/[[:space:]]*$//')"
  if [[ "$expr" =~ ^-?[0-9]+$ ]]; then
    printf '%s\n' "$expr"
    return 0
  fi
  if [[ "$expr" =~ ^[A-Za-z_][A-Za-z0-9_]*$ ]]; then
    [[ -n "${env_ref[$expr]+set}" ]] || return 1
    printf '%s\n' "${env_ref[$expr]}"
    return 0
  fi
  if [[ "$expr" =~ ^([A-Za-z_][A-Za-z0-9_]*|-?[0-9]+)[[:space:]]*([+*])[[:space:]]*([A-Za-z_][A-Za-z0-9_]*|-?[0-9]+)$ ]]; then
    left_token="${BASH_REMATCH[1]}"
    op="${BASH_REMATCH[2]}"
    right_token="${BASH_REMATCH[3]}"
    if [[ "$left_token" =~ ^-?[0-9]+$ ]]; then
      left="$left_token"
    else
      [[ -n "${env_ref[$left_token]+set}" ]] || return 1
      left="${env_ref[$left_token]}"
    fi
    if [[ "$right_token" =~ ^-?[0-9]+$ ]]; then
      right="$right_token"
    else
      [[ -n "${env_ref[$right_token]+set}" ]] || return 1
      right="${env_ref[$right_token]}"
    fi
    eval_runtime_int_binary_operator_value_task "$op" "$left" "$right"
    return
  fi
  return 1
}

sley_count_char_text() {
  local text="$1" char="$2" without
  without="${text//"$char"/}"
  printf '%s\n' "$(( ${#text} - ${#without} ))"
}

eval_source_state_set_int_main_return() {
  local target="$1" file raw line depth=0 in_main=false saw_set=false rest prefix name expr value
  declare -A state_env=()
  file="$(sley_source_entry_file_for_target "$target")"
  [[ -n "$file" ]] || return 1
  while IFS= read -r raw; do
    line="$(printf '%s\n' "$raw" | sed 's/^[[:space:]]*//; s/[[:space:]]*$//')"
    if ! "$in_main"; then
      if [[ "$line" =~ ^(export[[:space:]]+)?task[[:space:]]+main[[:space:]] ]]; then
        in_main=true
        depth=$(( $(sley_count_char_text "$raw" "{") - $(sley_count_char_text "$raw" "}") ))
      fi
      continue
    fi
    if (( depth == 1 )); then
      if [[ "$line" =~ ^(bind|state)[[:space:]]+[A-Za-z_][A-Za-z0-9_]*[[:space:]]*= ]]; then
        prefix="${BASH_REMATCH[1]}"
        rest="$line"
        rest="${rest#"$prefix"}"
        rest="$(printf '%s\n' "$rest" | sed 's/^[[:space:]]*//')"
        name="${rest%%=*}"
        name="$(printf '%s\n' "$name" | sed 's/[[:space:]]*$//')"
        expr="${rest#*=}"
        if value="$(eval_state_set_int_expr "$expr" state_env 2>/dev/null)"; then
          state_env["$name"]="$value"
        fi
      elif [[ "$line" =~ ^set[[:space:]]+[A-Za-z_][A-Za-z0-9_]*[[:space:]]*= ]]; then
        rest="${line#set}"
        rest="$(printf '%s\n' "$rest" | sed 's/^[[:space:]]*//')"
        name="${rest%%=*}"
        name="$(printf '%s\n' "$name" | sed 's/[[:space:]]*$//')"
        expr="${rest#*=}"
        value="$(eval_state_set_int_expr "$expr" state_env)" || return 1
        state_env["$name"]="$value"
        saw_set=true
      elif [[ "$line" =~ ^return[[:space:]]+ ]]; then
        "$saw_set" || return 1
        expr="${line#return}"
        value="$(eval_state_set_int_expr "$expr" state_env)" || return 1
        printf '{"kind":"%s","value":"%s"}\n' "$RUNTIME_INT_VALUE_KIND" "$value"
        return 0
      elif [[ -z "$line" || "$line" == "}" ]]; then
        :
      elif [[ "$line" =~ ^(if|while|for|forge)[[:space:]] ]]; then
        return 1
      fi
    fi
    depth=$(( depth + $(sley_count_char_text "$raw" "{") - $(sley_count_char_text "$raw" "}") ))
    if (( depth <= 0 )); then
      break
    fi
  done < "$file"
  return 1
}

eval_source_tally_set_int_main_return() {
  local target="$1" file raw line depth=0 in_main=false saw_tally=false saw_set=false rest prefix name expr value runtime_value
  declare -A tally_env=()
  file="$(sley_source_entry_file_for_target "$target")"
  [[ -n "$file" ]] || return 1
  while IFS= read -r raw; do
    line="$(printf '%s\n' "$raw" | sed 's/^[[:space:]]*//; s/[[:space:]]*$//')"
    if ! "$in_main"; then
      if [[ "$line" =~ ^(export[[:space:]]+)?task[[:space:]]+main[[:space:]] ]]; then
        in_main=true
        depth=$(( $(sley_count_char_text "$raw" "{") - $(sley_count_char_text "$raw" "}") ))
      fi
      continue
    fi
    if (( depth == 1 )); then
      if [[ "$line" =~ ^(bind|state|tally)[[:space:]]+[A-Za-z_][A-Za-z0-9_]*[[:space:]]*= ]]; then
        prefix="${BASH_REMATCH[1]}"
        rest="$line"
        rest="${rest#"$prefix"}"
        rest="$(printf '%s\n' "$rest" | sed 's/^[[:space:]]*//')"
        name="${rest%%=*}"
        name="$(printf '%s\n' "$name" | sed 's/[[:space:]]*$//')"
        expr="${rest#*=}"
        value="$(eval_state_set_int_expr "$expr" tally_env)" || return 1
        tally_env["$name"]="$value"
        [[ "$prefix" == "tally" ]] && saw_tally=true
      elif [[ "$line" =~ ^set[[:space:]]+[A-Za-z_][A-Za-z0-9_]*[[:space:]]*= ]]; then
        rest="${line#set}"
        rest="$(printf '%s\n' "$rest" | sed 's/^[[:space:]]*//')"
        name="${rest%%=*}"
        name="$(printf '%s\n' "$name" | sed 's/[[:space:]]*$//')"
        expr="${rest#*=}"
        value="$(eval_state_set_int_expr "$expr" tally_env)" || return 1
        tally_env["$name"]="$value"
        saw_set=true
      elif [[ "$line" =~ ^return[[:space:]]+ ]]; then
        "$saw_tally" && "$saw_set" || return 1
        expr="${line#return}"
        value="$(eval_state_set_int_expr "$expr" tally_env)" || return 1
        runtime_value="$(eval_runtime_int_identity_value_task "$value")" || return 1
        printf '{"kind":"%s","value":"%s"}\n' "$RUNTIME_INT_VALUE_KIND" "$runtime_value"
        return 0
      elif [[ -z "$line" || "$line" == "}" ]]; then
        :
      else
        return 1
      fi
    fi
    depth=$(( depth + $(sley_count_char_text "$raw" "{") - $(sley_count_char_text "$raw" "}") ))
    if (( depth <= 0 )); then
      break
    fi
  done < "$file"
  return 1
}

eval_source_each_sum_int_main_return() {
  local target="$1" file spec list_values total_name total_value loop_var set_target left_token right_token return_name item left right runtime_value
  local -a items=()
  file="$(sley_source_entry_file_for_target "$target")"
  [[ -n "$file" ]] || return 1
  spec="$(awk '
    function trim(s){gsub(/^[ \t\r\n]+|[ \t\r\n]+$/, "", s); return s}
    function count_char(s, c, n, i){n=0; for(i=1;i<=length(s);i++){if(substr(s,i,1)==c)n++} return n}
    /^[ \t]*(export[ \t]+)?task[ \t]+main[ \t]/ {
      in_main=1
      depth=count_char($0,"{")-count_char($0,"}")
      next
    }
    in_main {
      raw=$0
      line=trim(raw)
      if(depth == 1 && line ~ /^bind[ \t]+[A-Za-z_][A-Za-z0-9_]*[ \t]*=[ \t]*\[[^]]+\][ \t]*$/) {
        rest=line
        sub(/^bind[ \t]+/, "", rest)
        name=rest
        sub(/[ \t]*=.*/, "", name)
        values=rest
        sub(/^[^=]*=[ \t]*\[/, "", values)
        sub(/\][ \t]*$/, "", values)
        lists[trim(name)]=values
      } else if(depth == 1 && line ~ /^tally[ \t]+[A-Za-z_][A-Za-z0-9_]*[ \t]*=[ \t]*-?[0-9]+[ \t]*$/) {
        rest=line
        sub(/^tally[ \t]+/, "", rest)
        total_name=rest
        sub(/[ \t]*=.*/, "", total_name)
        total_value=rest
        sub(/^[^=]*=[ \t]*/, "", total_value)
        total_name=trim(total_name)
        total_value=trim(total_value)
      } else if(depth == 1 && line ~ /^each[ \t]+[A-Za-z_][A-Za-z0-9_]*[ \t]+in[ \t]+[A-Za-z_][A-Za-z0-9_]*[ \t]*\{[ \t]*$/) {
        rest=line
        sub(/^each[ \t]+/, "", rest)
        loop_var=rest
        sub(/[ \t]+in[ \t]+.*/, "", loop_var)
        list_name=rest
        sub(/^.*[ \t]+in[ \t]+/, "", list_name)
        sub(/[ \t]*\{[ \t]*$/, "", list_name)
        loop_var=trim(loop_var)
        list_name=trim(list_name)
        in_each=1
      } else if(in_each && depth == 2 && line ~ /^set[ \t]+[A-Za-z_][A-Za-z0-9_]*[ \t]*=[ \t]*[A-Za-z_][A-Za-z0-9_]*[ \t]*\+[ \t]*[A-Za-z_][A-Za-z0-9_]*[ \t]*$/) {
        rest=line
        sub(/^set[ \t]+/, "", rest)
        set_target=rest
        sub(/[ \t]*=.*/, "", set_target)
        expr=rest
        sub(/^[^=]*=[ \t]*/, "", expr)
        gsub(/[ \t]+/, "", expr)
        split(expr, parts, "+")
        left=parts[1]
        right=parts[2]
        set_target=trim(set_target)
      } else if(depth == 1 && line ~ /^return[ \t]+[A-Za-z_][A-Za-z0-9_]*[ \t]*$/) {
        return_name=line
        sub(/^return[ \t]+/, "", return_name)
        return_name=trim(return_name)
      }
      depth += count_char(raw,"{")-count_char(raw,"}")
      if(in_each && depth <= 1) in_each=0
      if(depth <= 0) {
        if(total_name != "" && total_value != "" && loop_var != "" && list_name != "" && set_target != "" && left != "" && right != "" && return_name != "" && (list_name in lists)) {
          print lists[list_name] "\t" total_name "\t" total_value "\t" loop_var "\t" set_target "\t" left "\t" right "\t" return_name
          exit 0
        }
        exit 1
      }
    }
    END { if(!in_main) exit 1 }
  ' "$file")" || return 1
  list_values="$(printf '%s\n' "$spec" | cut -f1)"
  total_name="$(printf '%s\n' "$spec" | cut -f2)"
  total_value="$(printf '%s\n' "$spec" | cut -f3)"
  loop_var="$(printf '%s\n' "$spec" | cut -f4)"
  set_target="$(printf '%s\n' "$spec" | cut -f5)"
  left_token="$(printf '%s\n' "$spec" | cut -f6)"
  right_token="$(printf '%s\n' "$spec" | cut -f7)"
  return_name="$(printf '%s\n' "$spec" | cut -f8)"
  [[ "$total_value" =~ ^-?[0-9]+$ && "$set_target" == "$total_name" && "$return_name" == "$total_name" ]] || return 1
  IFS=',' read -r -a items <<< "$list_values"
  [[ "${#items[@]}" -gt 0 ]] || return 1
  for item in "${items[@]}"; do
    item="$(printf '%s\n' "$item" | sed 's/^[[:space:]]*//; s/[[:space:]]*$//')"
    [[ "$item" =~ ^-?[0-9]+$ ]] || return 1
    case "$left_token" in
      "$total_name") left="$total_value" ;;
      "$loop_var") left="$item" ;;
      *) [[ "$left_token" =~ ^-?[0-9]+$ ]] || return 1; left="$left_token" ;;
    esac
    case "$right_token" in
      "$total_name") right="$total_value" ;;
      "$loop_var") right="$item" ;;
      *) [[ "$right_token" =~ ^-?[0-9]+$ ]] || return 1; right="$right_token" ;;
    esac
    total_value="$(eval_runtime_int_binary_operator_value_task "+" "$left" "$right")" || return 1
  done
  runtime_value="$(eval_runtime_int_identity_value_task "$total_value")" || return 1
  printf '{"kind":"%s","value":"%s"}\n' "$RUNTIME_INT_VALUE_KIND" "$runtime_value"
}

eval_source_each_map_sum_int_main_return() {
  local target="$1" file spec list_values map_pairs total_value list_count index list_key map_value runtime_value
  file="$(sley_source_entry_file_for_target "$target")"
  [[ -n "$file" ]] || return 1
  spec="$(awk '
    function trim(s){gsub(/^[ \t\r\n]+|[ \t\r\n]+$/, "", s); return s}
    function count_char(s, c, n, i){n=0; for(i=1;i<=length(s);i++){if(substr(s,i,1)==c)n++} return n}
    function append_pair(name, pair) {
      if(maps[name] == "") maps[name]=pair
      else maps[name]=maps[name] "," pair
    }
    /^[ \t]*(export[ \t]+)?task[ \t]+main[ \t]/ {
      in_main=1
      depth=count_char($0,"{")-count_char($0,"}")
      next
    }
    in_main {
      raw=$0
      line=trim(raw)
      if(depth == 1 && line ~ /^bind[ \t]+[A-Za-z_][A-Za-z0-9_]*([ \t]*:[^=]+)?[ \t]*=[ \t]*\[[^]]+\][ \t]*$/) {
        rest=line
        sub(/^bind[ \t]+/, "", rest)
        name=rest
        sub(/[ \t]*:.*/, "", name)
        sub(/[ \t]*=.*/, "", name)
        name=trim(name)
        values=rest
        sub(/^[^=]*=[ \t]*\[/, "", values)
        sub(/\][ \t]*$/, "", values)
        lists[name]=values
      } else if(depth == 1 && line ~ /^bind[ \t]+[A-Za-z_][A-Za-z0-9_]*[ \t]*:[ \t]*Map<Text,[ \t]*Int>[ \t]*=[ \t]*map[ \t]*[{][ \t]*$/) {
        rest=line
        sub(/^bind[ \t]+/, "", rest)
        name=rest
        sub(/[ \t]*:.*/, "", name)
        name=trim(name)
        maps[name]=""
        in_map=name
      } else if(in_map != "" && depth == 2 && line ~ /^"[^"]+"[ \t]*:[ \t]*-?[0-9]+,?[ \t]*$/) {
        pair=line
        sub(/,[ \t]*$/, "", pair)
        gsub(/[ \t]+/, "", pair)
        append_pair(in_map, pair)
      } else if(depth == 1 && line ~ /^tally[ \t]+[A-Za-z_][A-Za-z0-9_]*[ \t]*=[ \t]*-?[0-9]+[ \t]*$/) {
        rest=line
        sub(/^tally[ \t]+/, "", rest)
        total_name=rest
        sub(/[ \t]*=.*/, "", total_name)
        total_value=rest
        sub(/^[^=]*=[ \t]*/, "", total_value)
        total_name=trim(total_name)
        total_value=trim(total_value)
      } else if(depth == 1 && line ~ /^each[ \t]+[A-Za-z_][A-Za-z0-9_]*[ \t]+in[ \t]+[A-Za-z_][A-Za-z0-9_]*[ \t]*[{][ \t]*$/) {
        rest=line
        sub(/^each[ \t]+/, "", rest)
        loop_var=rest
        sub(/[ \t]+in[ \t]+.*/, "", loop_var)
        list_name=rest
        sub(/^.*[ \t]+in[ \t]+/, "", list_name)
        sub(/[ \t]*[{][ \t]*$/, "", list_name)
        loop_var=trim(loop_var)
        list_name=trim(list_name)
        in_each=1
      } else if(in_each && depth == 2 && line ~ /^set[ \t]+[A-Za-z_][A-Za-z0-9_]*[ \t]*=[ \t]*[A-Za-z_][A-Za-z0-9_]*[ \t]*\+[ \t]*[A-Za-z_][A-Za-z0-9_]*\[[A-Za-z_][A-Za-z0-9_]*\][ \t]*$/) {
        rest=line
        sub(/^set[ \t]+/, "", rest)
        set_target=rest
        sub(/[ \t]*=.*/, "", set_target)
        expr=rest
        sub(/^[^=]*=[ \t]*/, "", expr)
        gsub(/[ \t]+/, "", expr)
        split(expr, parts, "+")
        left_token=parts[1]
        map_ref=parts[2]
        map_name=map_ref
        sub(/\[.*/, "", map_name)
        key_name=map_ref
        sub(/^[^[]*\[/, "", key_name)
        sub(/\]$/, "", key_name)
        set_target=trim(set_target)
      } else if(depth == 1 && line ~ /^return[ \t]+[A-Za-z_][A-Za-z0-9_]*[ \t]*$/) {
        return_name=line
        sub(/^return[ \t]+/, "", return_name)
        return_name=trim(return_name)
      }
      depth += count_char(raw,"{")-count_char(raw,"}")
      if(in_map != "" && depth <= 1) in_map=""
      if(in_each && depth <= 1) in_each=0
      if(depth <= 0) {
        if(total_name != "" && total_value != "" && loop_var != "" && list_name != "" && set_target == total_name && left_token == total_name && key_name == loop_var && return_name == total_name && (list_name in lists) && (map_name in maps)) {
          print lists[list_name] "\t" maps[map_name] "\t" total_value
          exit 0
        }
        exit 1
      }
    }
    END { if(!in_main) exit 1 }
  ' "$file")" || return 1
  list_values="$(printf '%s\n' "$spec" | cut -f1)"
  map_pairs="$(printf '%s\n' "$spec" | cut -f2)"
  total_value="$(printf '%s\n' "$spec" | cut -f3)"
  [[ -n "$list_values" && -n "$map_pairs" && "$total_value" =~ ^-?[0-9]+$ ]] || return 1
  list_count="$(printf '%s\n' "$list_values" | count_top_level_list_items)"
  [[ "$list_count" =~ ^[0-9]+$ ]] || return 1
  for ((index = 0; index < list_count; index++)); do
    list_key="$(eval_runtime_collection_index_operator_value_task "list" "Text" "$list_values" "$index")" || return 1
    map_value="$(eval_runtime_collection_index_operator_value_task "map" "Int" "$map_pairs" "$list_key")" || return 1
    total_value="$(eval_runtime_int_binary_operator_value_task "+" "$total_value" "$map_value")" || return 1
  done
  runtime_value="$(eval_runtime_int_identity_value_task "$total_value")" || return 1
  printf '{"kind":"%s","value":"%s"}\n' "$RUNTIME_INT_VALUE_KIND" "$runtime_value"
}

eval_source_while_list_sum_int_main_return() {
  local target="$1" file main_info list_values expected_count callee else_value body sum_info list_count condition index total item runtime_value
  file="$(sley_source_entry_file_for_target "$target")"
  [[ -n "$file" ]] || return 1
  main_info="$(awk -v call_prefix="$PARSER_CALL_EXPRESSION_PREFIX" '
    function trim(s){gsub(/^[ \t\r\n]+|[ \t\r\n]+$/, "", s); return s}
    function count_char(s, c, n, i){n=0; for(i=1;i<=length(s);i++){if(substr(s,i,1)==c)n++} return n}
    /^[ \t]*(export[ \t]+)?task[ \t]+main[ \t]/ {
      in_main=1
      depth=count_char($0,"{")-count_char($0,"}")
      next
    }
    in_main {
      raw=$0
      line=trim(raw)
      if(depth == 1 && line ~ /^bind[ \t]+[A-Za-z_][A-Za-z0-9_]*([ \t]*:[^=]+)?[ \t]*=[ \t]*\[[^]]+\][ \t]*$/) {
        rest=line
        sub(/^bind[ \t]+/, "", rest)
        list_name=rest
        sub(/[ \t]*:.*/, "", list_name)
        sub(/[ \t]*=.*/, "", list_name)
        list_name=trim(list_name)
        values=rest
        sub(/^[^=]*=[ \t]*\[/, "", values)
        sub(/\][ \t]*$/, "", values)
      } else if(depth == 1 && line ~ /^if[ \t]+len\([A-Za-z_][A-Za-z0-9_]*\)[ \t]*==[ \t]*[0-9]+[ \t]*[{][ \t]*$/) {
        cond=line
        sub(/^if[ \t]+len\(/, "", cond)
        cond_name=cond
        sub(/\).*/, "", cond_name)
        expected=cond
        sub(/^.*==[ \t]*/, "", expected)
        sub(/[ \t]*[{][ \t]*$/, "", expected)
        cond_name=trim(cond_name)
        expected=trim(expected)
        in_if=1
      } else if(in_if && depth == 2 && line ~ /^return[ \t]+-?[0-9]+[ \t]*$/) {
        fallback=line
        sub(/^return[ \t]+/, "", fallback)
        fallback=trim(fallback)
      } else if(in_if && depth == 2 && line ~ /^return[ \t]+/) {
        expr=line
        sub(/^return[ \t]+/, "", expr)
        if(index(expr, call_prefix) == 1) {
          expr=substr(expr, length(call_prefix) + 1)
          if(expr ~ /^[A-Za-z_][A-Za-z0-9_]*\([A-Za-z_][A-Za-z0-9_]*\)[ \t]*$/) {
            callee=expr
            sub(/\(.*/, "", callee)
            call_arg=expr
            sub(/^[^(]*\(/, "", call_arg)
            sub(/\)[ \t]*$/, "", call_arg)
          }
        }
      }
      depth += count_char(raw,"{")-count_char(raw,"}")
      if(in_if && depth <= 1) in_if=0
      if(depth <= 0) {
        if(values != "" && expected != "" && callee != "" && fallback != "" && cond_name == list_name && call_arg == list_name) {
          print values "\t" expected "\t" callee "\t" fallback
          exit 0
        }
        exit 1
      }
    }
    END { if(!in_main) exit 1 }
  ' "$file")" || return 1
  list_values="$(printf '%s\n' "$main_info" | cut -f1)"
  expected_count="$(printf '%s\n' "$main_info" | cut -f2)"
  callee="$(printf '%s\n' "$main_info" | cut -f3)"
  else_value="$(printf '%s\n' "$main_info" | cut -f4)"
  [[ -n "$list_values" && "$expected_count" =~ ^[0-9]+$ && -n "$callee" && "$else_value" =~ ^-?[0-9]+$ ]] || return 1
  body="$(extract_sley_task_body "$file" "$callee")"
  [[ -n "$body" ]] || return 1
  sum_info="$(printf '%s\n' "$body" | awk '
    function trim(s){gsub(/^[ \t\r\n]+|[ \t\r\n]+$/, "", s); return s}
    {
      line=trim($0)
      if(line ~ /^take[ \t]+[A-Za-z_][A-Za-z0-9_]*[ \t]*:[ \t]*List<Int>[ \t]*$/) {
        take=line
        sub(/^take[ \t]+/, "", take)
        sub(/[ \t]*:.*/, "", take)
      } else if(line ~ /^state[ \t]+[A-Za-z_][A-Za-z0-9_]*[ \t]*=[ \t]*0[ \t]*$/) {
        rest=line
        sub(/^state[ \t]+/, "", rest)
        name=rest
        sub(/[ \t]*=.*/, "", name)
        name=trim(name)
        if(index_name == "") index_name=name
        else if(total_name == "") total_name=name
      } else if(line ~ /^while[ \t]+[A-Za-z_][A-Za-z0-9_]*[ \t]*<[ \t]*len\([A-Za-z_][A-Za-z0-9_]*\)[ \t]*[{][ \t]*$/) {
        cond=line
        sub(/^while[ \t]+/, "", cond)
        while_left=cond
        sub(/[ \t]*<.*/, "", while_left)
        while_values=cond
        sub(/^.*len\(/, "", while_values)
        sub(/\).*/, "", while_values)
        while_left=trim(while_left)
        while_values=trim(while_values)
      } else if(line ~ /^set[ \t]+[A-Za-z_][A-Za-z0-9_]*[ \t]*=[ \t]*[A-Za-z_][A-Za-z0-9_]*[ \t]*\+[ \t]*[A-Za-z_][A-Za-z0-9_]*\[[A-Za-z_][A-Za-z0-9_]*\][ \t]*$/) {
        rest=line
        sub(/^set[ \t]+/, "", rest)
        sum_target=rest
        sub(/[ \t]*=.*/, "", sum_target)
        expr=rest
        sub(/^[^=]*=[ \t]*/, "", expr)
        gsub(/[ \t]+/, "", expr)
        split(expr, parts, "+")
        sum_left=parts[1]
        list_ref=parts[2]
        sum_list=list_ref
        sub(/\[.*/, "", sum_list)
        sum_index=list_ref
        sub(/^[^[]*\[/, "", sum_index)
        sub(/\]$/, "", sum_index)
        sum_target=trim(sum_target)
      } else if(line ~ /^set[ \t]+[A-Za-z_][A-Za-z0-9_]*[ \t]*=[ \t]*[A-Za-z_][A-Za-z0-9_]*[ \t]*\+[ \t]*1[ \t]*$/) {
        rest=line
        sub(/^set[ \t]+/, "", rest)
        inc_target=rest
        sub(/[ \t]*=.*/, "", inc_target)
        expr=rest
        sub(/^[^=]*=[ \t]*/, "", expr)
        gsub(/[ \t]+/, "", expr)
        split(expr, parts, "+")
        inc_left=parts[1]
        inc_right=parts[2]
        inc_target=trim(inc_target)
      } else if(line ~ /^return[ \t]+[A-Za-z_][A-Za-z0-9_]*[ \t]*$/) {
        return_name=line
        sub(/^return[ \t]+/, "", return_name)
        return_name=trim(return_name)
      }
    }
    END {
      if(take != "" && index_name != "" && total_name != "" && while_left == index_name && while_values == take && sum_target == total_name && sum_left == total_name && sum_list == take && sum_index == index_name && inc_target == index_name && inc_left == index_name && inc_right == "1" && return_name == total_name) {
        print index_name "\t" total_name
        exit 0
      }
      exit 1
    }
  ')" || return 1
  [[ -n "$sum_info" ]] || return 1
  list_count="$(printf '%s\n' "$list_values" | count_top_level_list_items)"
  [[ "$list_count" =~ ^[0-9]+$ ]] || return 1
  condition="$(eval_runtime_int_comparison_operator_value_task "==" "$list_count" "$expected_count")" || return 1
  if [[ "$condition" != "true" ]]; then
    runtime_value="$(eval_runtime_int_identity_value_task "$else_value")" || return 1
    printf '{"kind":"%s","value":"%s"}\n' "$RUNTIME_INT_VALUE_KIND" "$runtime_value"
    return 0
  fi
  index=0
  total=0
  while [[ "$(eval_runtime_int_comparison_operator_value_task "<" "$index" "$list_count")" == "true" ]]; do
    item="$(eval_runtime_collection_index_operator_value_task "list" "Int" "$list_values" "$index")" || return 1
    total="$(eval_runtime_int_binary_operator_value_task "+" "$total" "$item")" || return 1
    index="$(eval_runtime_int_binary_operator_value_task "+" "$index" "1")" || return 1
  done
  runtime_value="$(eval_runtime_int_identity_value_task "$total")" || return 1
  printf '{"kind":"%s","value":"%s"}\n' "$RUNTIME_INT_VALUE_KIND" "$runtime_value"
}

eval_source_compute_text_call_main_return() {
  local target="$1" file main_info callee arg left right arg_value body class_info threshold then_value else_value condition runtime_value call_prefix
  file="$(sley_source_entry_file_for_target "$target")"
  [[ -n "$file" ]] || return 1
  call_prefix="$PARSER_CALL_EXPRESSION_PREFIX"
  main_info="$(awk -v call_prefix="$call_prefix" '
    function trim(s){gsub(/^[ \t\r\n]+|[ \t\r\n]+$/, "", s); return s}
    function count_char(s, c, n, i){n=0; for(i=1;i<=length(s);i++){if(substr(s,i,1)==c)n++} return n}
    /^[ \t]*(export[ \t]+)?task[ \t]+main[ \t]/ {
      in_main=1
      depth=count_char($0,"{")-count_char($0,"}")
      next
    }
    in_main {
      raw=$0
      line=trim(raw)
      if(depth == 1 && line ~ /^bind[ \t]+[A-Za-z_][A-Za-z0-9_]*[ \t]*=[ \t]*-?[0-9]+[ \t]*\+[ \t]*-?[0-9]+[ \t]*$/) {
        rest=line
        sub(/^bind[ \t]+/, "", rest)
        arg=rest
        sub(/[ \t]*=.*/, "", arg)
        expr=rest
        sub(/^[^=]*=[ \t]*/, "", expr)
        gsub(/[ \t]+/, "", expr)
        split(expr, parts, "+")
        left=parts[1]
        right=parts[2]
      } else if(depth == 1 && line ~ /^return[ \t]+/) {
        expr=line
        sub(/^return[ \t]+/, "", expr)
        if (index(expr, call_prefix) != 1) next
        expr=substr(expr, length(call_prefix) + 1)
        if (expr !~ /^[A-Za-z_][A-Za-z0-9_]*\([A-Za-z_][A-Za-z0-9_]*\)[ \t]*$/) next
        callee=expr
        sub(/\(.*/, "", callee)
        call_arg=expr
        sub(/^[^(]*\(/, "", call_arg)
        sub(/\)[ \t]*$/, "", call_arg)
      }
      depth += count_char(raw,"{")-count_char(raw,"}")
      if(depth <= 0) {
        if(arg != "" && callee != "" && call_arg == arg && left ~ /^-?[0-9]+$/ && right ~ /^-?[0-9]+$/) {
          print callee "\t" arg "\t" left "\t" right
          exit 0
        }
        exit 1
      }
    }
    END { if(!in_main) exit 1 }
  ' "$file")" || return 1
  callee="$(printf '%s\n' "$main_info" | cut -f1)"
  arg="$(printf '%s\n' "$main_info" | cut -f2)"
  left="$(printf '%s\n' "$main_info" | cut -f3)"
  right="$(printf '%s\n' "$main_info" | cut -f4)"
  [[ -n "$callee" && -n "$arg" && "$left" =~ ^-?[0-9]+$ && "$right" =~ ^-?[0-9]+$ ]] || return 1
  arg_value="$(eval_runtime_int_binary_operator_value_task "+" "$left" "$right")" || return 1
  body="$(extract_sley_task_body "$file" "$callee")"
  [[ -n "$body" ]] || return 1
  class_info="$(printf '%s\n' "$body" | awk '
    function trim(s){gsub(/^[ \t\r\n]+|[ \t\r\n]+$/, "", s); return s}
    {
      line=trim($0)
      if(line ~ /^take[ \t]+[A-Za-z_][A-Za-z0-9_]*[ \t]*:[ \t]*Int[ \t]*$/) {
        take=line
        sub(/^take[ \t]+/, "", take)
        sub(/[ \t]*:.*/, "", take)
      } else if(line ~ /^return[ \t]+if[ \t]+[A-Za-z_][A-Za-z0-9_]*[ \t]*>=[ \t]*-?[0-9]+[ \t]*[{][ \t]*"[^"]+"[ \t]*}[ \t]*else[ \t]*[{][ \t]*"[^"]+"[ \t]*}[ \t]*$/) {
        expr=line
        sub(/^return[ \t]+/, "", expr)
        cond_name=expr
        sub(/^if[ \t]+/, "", cond_name)
        sub(/[ \t]*>=.*/, "", cond_name)
        threshold=expr
        sub(/^if[ \t]+[A-Za-z_][A-Za-z0-9_]*[ \t]*>=[ \t]*/, "", threshold)
        sub(/[ \t]*[{].*/, "", threshold)
        then_value=expr
        sub(/^if[ \t]+[A-Za-z_][A-Za-z0-9_]*[ \t]*>=[ \t]*-?[0-9]+[ \t]*[{][ \t]*"/, "", then_value)
        sub(/"[ \t]*}[ \t]*else[ \t]*[{].*$/, "", then_value)
        else_value=expr
        sub(/^.*else[ \t]*[{][ \t]*"/, "", else_value)
        sub(/"[ \t]*}[ \t]*$/, "", else_value)
        cond_name=trim(cond_name)
        threshold=trim(threshold)
      }
    }
    END {
      if(take != "" && cond_name == take && threshold ~ /^-?[0-9]+$/ && then_value != "" && else_value != "") {
        print threshold "\t" then_value "\t" else_value
        exit 0
      }
      exit 1
    }
  ')" || return 1
  threshold="$(printf '%s\n' "$class_info" | cut -f1)"
  then_value="$(printf '%s\n' "$class_info" | cut -f2)"
  else_value="$(printf '%s\n' "$class_info" | cut -f3)"
  [[ "$threshold" =~ ^-?[0-9]+$ && -n "$then_value" && -n "$else_value" ]] || return 1
  condition="$(eval_runtime_int_comparison_operator_value_task ">=" "$arg_value" "$threshold")" || return 1
  runtime_value="$(eval_runtime_if_value_task "Text" "$condition" "$then_value" "$else_value")" || return 1
  jq -cn --arg kind "$RUNTIME_TEXT_VALUE_KIND" --arg value "$runtime_value" '{kind:$kind,value:$value}'
}

eval_source_result_flow_int_main_return() {
  local target="$1" file main_info callee call_arg add_right body parse_info bad_literal ok_value condition parsed_value result_value runtime_value call_prefix
  file="$(sley_source_entry_file_for_target "$target")"
  [[ -n "$file" ]] || return 1
  call_prefix="$PARSER_CALL_EXPRESSION_PREFIX"
  main_info="$(awk -v call_prefix="$call_prefix" '
    function trim(s){gsub(/^[ \t\r\n]+|[ \t\r\n]+$/, "", s); return s}
    function count_char(s, c, n, i){n=0; for(i=1;i<=length(s);i++){if(substr(s,i,1)==c)n++} return n}
    /^[ \t]*(export[ \t]+)?task[ \t]+main[ \t]/ {
      in_main=1
      depth=count_char($0,"{")-count_char($0,"}")
      next
    }
    in_main {
      raw=$0
      line=trim(raw)
      if(depth == 1 && line ~ /^bind[ \t]+[A-Za-z_][A-Za-z0-9_]*[ \t]*=/) {
        rest=line
        sub(/^bind[ \t]+/, "", rest)
        bind_name=rest
        sub(/[ \t]*=.*/, "", bind_name)
        expr=rest
        sub(/^[^=]*=[ \t]*/, "", expr)
        if (index(expr, call_prefix) != 1) next
        expr=substr(expr, length(call_prefix) + 1)
        if (expr !~ /^[A-Za-z_][A-Za-z0-9_]*\("[^"]+"\)\?[ \t]*$/) next
        callee=expr
        sub(/\(.*/, "", callee)
        arg=expr
        sub(/^[^(]*\("/, "", arg)
        sub(/"\)\?[ \t]*$/, "", arg)
        bind_name=trim(bind_name)
      } else if(depth == 1 && line ~ /^return[ \t]+Ok\([A-Za-z_][A-Za-z0-9_]*[ \t]*\+[ \t]*-?[0-9]+\)[ \t]*$/) {
        expr=line
        sub(/^return[ \t]+Ok\(/, "", expr)
        sub(/\)[ \t]*$/, "", expr)
        gsub(/[ \t]+/, "", expr)
        split(expr, parts, "+")
        return_name=parts[1]
        add_right=parts[2]
      }
      depth += count_char(raw,"{")-count_char(raw,"}")
      if(depth <= 0) {
        if(bind_name != "" && callee != "" && arg != "" && return_name == bind_name && add_right ~ /^-?[0-9]+$/) {
          print callee "\t" arg "\t" add_right
          exit 0
        }
        exit 1
      }
    }
    END { if(!in_main) exit 1 }
  ' "$file")" || return 1
  callee="$(printf '%s\n' "$main_info" | cut -f1)"
  call_arg="$(printf '%s\n' "$main_info" | cut -f2)"
  add_right="$(printf '%s\n' "$main_info" | cut -f3)"
  [[ -n "$callee" && -n "$call_arg" && "$add_right" =~ ^-?[0-9]+$ ]] || return 1
  body="$(extract_sley_task_body "$file" "$callee")"
  [[ -n "$body" ]] || return 1
  parse_info="$(printf '%s\n' "$body" | awk '
    function trim(s){gsub(/^[ \t\r\n]+|[ \t\r\n]+$/, "", s); return s}
    {
      line=trim($0)
      if(line ~ /^take[ \t]+[A-Za-z_][A-Za-z0-9_]*[ \t]*:[ \t]*Text[ \t]*$/) {
        take=line
        sub(/^take[ \t]+/, "", take)
        sub(/[ \t]*:.*/, "", take)
      } else if(line ~ /^if[ \t]+[A-Za-z_][A-Za-z0-9_]*[ \t]*==[ \t]*"[^"]+"[ \t]*[{][ \t]*$/) {
        cond=line
        sub(/^if[ \t]+/, "", cond)
        cond_name=cond
        sub(/[ \t]*==.*/, "", cond_name)
        bad=cond
        sub(/^[^=]*==[ \t]*"/, "", bad)
        sub(/"[ \t]*[{][ \t]*$/, "", bad)
        cond_name=trim(cond_name)
      } else if(line ~ /^return[ \t]+Err\("[^"]+"\)[ \t]*$/) {
        saw_err=1
      } else if(line ~ /^return[ \t]+Ok\(-?[0-9]+\)[ \t]*$/) {
        ok=line
        sub(/^return[ \t]+Ok\(/, "", ok)
        sub(/\)[ \t]*$/, "", ok)
        ok=trim(ok)
      }
    }
    END {
      if(take != "" && cond_name == take && bad != "" && saw_err == 1 && ok ~ /^-?[0-9]+$/) {
        print bad "\t" ok
        exit 0
      }
      exit 1
    }
  ')" || return 1
  bad_literal="$(printf '%s\n' "$parse_info" | cut -f1)"
  ok_value="$(printf '%s\n' "$parse_info" | cut -f2)"
  [[ -n "$bad_literal" && "$ok_value" =~ ^-?[0-9]+$ ]] || return 1
  condition="$(eval_runtime_equality_operator_value_task "Text" "==" "$call_arg" "$bad_literal")" || return 1
  [[ "$condition" == "false" ]] || return 1
  parsed_value="$(eval_runtime_ok_int_value_task "$ok_value")" || return 1
  result_value="$(eval_runtime_int_binary_operator_value_task "+" "$parsed_value" "$add_right")" || return 1
  runtime_value="$(eval_runtime_ok_int_value_task "$result_value")" || return 1
  printf '{"kind":"%s","value":"%s"}\n' "$RUNTIME_OK_INT_VALUE_KIND" "$runtime_value"
}

eval_source_bound_int_main_return() {
  local target="$1" file raw line depth=0 in_main=false rest prefix name expr value runtime_value
  declare -A int_env=()
  file="$(sley_source_entry_file_for_target "$target")"
  [[ -n "$file" ]] || return 1
  while IFS= read -r raw; do
    line="$(printf '%s\n' "$raw" | sed 's/^[[:space:]]*//; s/[[:space:]]*$//')"
    if ! "$in_main"; then
      if [[ "$line" =~ ^(export[[:space:]]+)?task[[:space:]]+main[[:space:]] ]]; then
        in_main=true
        depth=$(( $(sley_count_char_text "$raw" "{") - $(sley_count_char_text "$raw" "}") ))
      fi
      continue
    fi
    if (( depth == 1 )); then
      if [[ "$line" =~ ^bind[[:space:]]+[A-Za-z_][A-Za-z0-9_]*[[:space:]]*= ]]; then
        prefix="bind"
        rest="$line"
        rest="${rest#"$prefix"}"
        rest="$(printf '%s\n' "$rest" | sed 's/^[[:space:]]*//')"
        name="${rest%%=*}"
        name="$(printf '%s\n' "$name" | sed 's/[[:space:]]*$//')"
        expr="${rest#*=}"
        if value="$(eval_state_set_int_expr "$expr" int_env 2>/dev/null)"; then
          int_env["$name"]="$value"
        fi
      elif [[ "$line" =~ ^return[[:space:]]+ ]]; then
        expr="${line#return}"
        value="$(eval_state_set_int_expr "$expr" int_env)" || return 1
        runtime_value="$(eval_runtime_int_identity_value_task "$value")" || return 1
        printf '{"kind":"%s","value":"%s"}\n' "$RUNTIME_INT_VALUE_KIND" "$runtime_value"
        return 0
      elif [[ -z "$line" || "$line" == "}" ]]; then
        :
      elif [[ "$line" =~ ^[A-Za-z_][A-Za-z0-9_]*[[:space:]]*[+*][[:space:]]*([A-Za-z_][A-Za-z0-9_]*|-?[0-9]+)$ ]]; then
        :
      elif [[ "$line" =~ ^(state|set|if|while|for|forge)[[:space:]] ]]; then
        return 1
      else
        return 1
      fi
    fi
    depth=$(( depth + $(sley_count_char_text "$raw" "{") - $(sley_count_char_text "$raw" "}") ))
    if (( depth <= 0 )); then
      break
    fi
  done < "$file"
  return 1
}

eval_source_state_if_set_int_main_return() {
  local target="$1" file raw line depth=0 in_main=false in_if=false saw_if=false branch="" rest prefix name expr value condition condition_value set_name="" then_value="" else_value="" runtime_value final_value
  declare -A int_env=()
  declare -A bool_env=()
  file="$(sley_source_entry_file_for_target "$target")"
  [[ -n "$file" ]] || return 1
  while IFS= read -r raw; do
    line="$(printf '%s\n' "$raw" | sed 's/^[[:space:]]*//; s/[[:space:]]*$//')"
    if ! "$in_main"; then
      if [[ "$line" =~ ^(export[[:space:]]+)?task[[:space:]]+main[[:space:]] ]]; then
        in_main=true
        depth=$(( $(sley_count_char_text "$raw" "{") - $(sley_count_char_text "$raw" "}") ))
      fi
      continue
    fi
    if (( depth == 1 )); then
      if [[ "$line" =~ ^(bind|state)[[:space:]]+[A-Za-z_][A-Za-z0-9_]*[[:space:]]*= ]]; then
        prefix="${BASH_REMATCH[1]}"
        rest="$line"
        rest="${rest#"$prefix"}"
        rest="$(printf '%s\n' "$rest" | sed 's/^[[:space:]]*//')"
        name="${rest%%=*}"
        name="$(printf '%s\n' "$name" | sed 's/[[:space:]]*$//')"
        expr="${rest#*=}"
        expr="$(printf '%s\n' "$expr" | sed 's/^[[:space:]]*//; s/[[:space:]]*$//')"
        if value="$(eval_state_set_int_expr "$expr" int_env 2>/dev/null)"; then
          int_env["$name"]="$value"
        elif [[ "$expr" == "true" || "$expr" == "false" ]]; then
          bool_env["$name"]="$expr"
        else
          return 1
        fi
      elif [[ "$line" =~ ^if[[:space:]]+.+[[:space:]]*\{[[:space:]]*$ ]]; then
        condition="$line"
        condition="${condition#if}"
        condition="${condition%\{}"
        condition="$(printf '%s\n' "$condition" | sed 's/^[[:space:]]*//; s/[[:space:]]*$//')"
        if [[ "$condition" == "true" || "$condition" == "false" ]]; then
          condition_value="$condition"
        elif [[ -n "${bool_env[$condition]+set}" ]]; then
          condition_value="${bool_env[$condition]}"
        else
          return 1
        fi
        in_if=true
        saw_if=true
        branch="then"
      elif [[ "$line" =~ ^return[[:space:]]+ ]]; then
        "$saw_if" || return 1
        expr="${line#return}"
        final_value="$(eval_state_set_int_expr "$expr" int_env)" || return 1
        runtime_value="$(eval_runtime_int_identity_value_task "$final_value")" || return 1
        printf '{"kind":"%s","value":"%s"}\n' "$RUNTIME_INT_VALUE_KIND" "$runtime_value"
        return 0
      elif [[ -z "$line" || "$line" == "}" ]]; then
        :
      else
        return 1
      fi
    elif "$in_if" && (( depth == 2 )); then
      if [[ "$line" =~ ^\}[[:space:]]*else[[:space:]]*\{[[:space:]]*$ ]]; then
        branch="else"
      elif [[ "$line" =~ ^set[[:space:]]+[A-Za-z_][A-Za-z0-9_]*[[:space:]]*= ]]; then
        rest="${line#set}"
        rest="$(printf '%s\n' "$rest" | sed 's/^[[:space:]]*//')"
        name="${rest%%=*}"
        name="$(printf '%s\n' "$name" | sed 's/[[:space:]]*$//')"
        expr="${rest#*=}"
        value="$(eval_state_set_int_expr "$expr" int_env)" || return 1
        if [[ -z "$set_name" ]]; then
          set_name="$name"
        elif [[ "$set_name" != "$name" ]]; then
          return 1
        fi
        if [[ "$branch" == "then" ]]; then
          then_value="$value"
        elif [[ "$branch" == "else" ]]; then
          else_value="$value"
        else
          return 1
        fi
      elif [[ -z "$line" ]]; then
        :
      elif [[ "$line" == "}" ]]; then
        :
      else
        return 1
      fi
    fi
    depth=$(( depth + $(sley_count_char_text "$raw" "{") - $(sley_count_char_text "$raw" "}") ))
    if "$in_if" && (( depth <= 1 )); then
      [[ -n "$set_name" && -n "$then_value" && -n "$else_value" ]] || return 1
      runtime_value="$(eval_runtime_if_value_task "Int" "$condition_value" "$then_value" "$else_value")" || return 1
      int_env["$set_name"]="$runtime_value"
      in_if=false
      branch=""
      set_name=""
      then_value=""
      else_value=""
      condition_value=""
    fi
    if (( depth <= 0 )); then
      break
    fi
  done < "$file"
  return 1
}

eval_source_bool_if_statement_main_return() {
  local target="$1" file raw line depth=0 in_main=false in_if=false branch="" rest name expr value condition condition_value then_value="" else_value="" runtime_value
  declare -A bool_env=()
  file="$(sley_source_entry_file_for_target "$target")"
  [[ -n "$file" ]] || return 1
  while IFS= read -r raw; do
    line="$(printf '%s\n' "$raw" | sed 's/^[[:space:]]*//; s/[[:space:]]*$//')"
    if ! "$in_main"; then
      if [[ "$line" =~ ^(export[[:space:]]+)?task[[:space:]]+main[[:space:]] ]]; then
        in_main=true
        depth=$(( $(sley_count_char_text "$raw" "{") - $(sley_count_char_text "$raw" "}") ))
      fi
      continue
    fi
    if (( depth == 1 )); then
      if [[ "$line" =~ ^bind[[:space:]]+[A-Za-z_][A-Za-z0-9_]*[[:space:]]*= ]]; then
        rest="${line#bind}"
        rest="$(printf '%s\n' "$rest" | sed 's/^[[:space:]]*//')"
        name="${rest%%=*}"
        name="$(printf '%s\n' "$name" | sed 's/[[:space:]]*$//')"
        expr="${rest#*=}"
        expr="$(printf '%s\n' "$expr" | sed 's/^[[:space:]]*//; s/[[:space:]]*$//')"
        [[ "$expr" == "true" || "$expr" == "false" ]] || return 1
        bool_env["$name"]="$expr"
      elif [[ "$line" =~ ^if[[:space:]]+.+[[:space:]]*\{[[:space:]]*$ ]]; then
        condition="$line"
        condition="${condition#if}"
        condition="${condition%\{}"
        condition="$(printf '%s\n' "$condition" | sed 's/^[[:space:]]*//; s/[[:space:]]*$//')"
        if [[ "$condition" == "true" || "$condition" == "false" ]]; then
          condition_value="$condition"
        elif [[ -n "${bool_env[$condition]+set}" ]]; then
          condition_value="${bool_env[$condition]}"
        else
          return 1
        fi
        in_if=true
        branch="then"
      elif [[ -z "$line" || "$line" == "}" ]]; then
        :
      else
        return 1
      fi
    elif "$in_if" && (( depth == 2 )); then
      if [[ "$line" =~ ^\}[[:space:]]*else[[:space:]]*\{[[:space:]]*$ ]]; then
        branch="else"
      elif [[ "$line" =~ ^return[[:space:]]+ ]]; then
        expr="${line#return}"
        expr="$(printf '%s\n' "$expr" | sed 's/^[[:space:]]*//; s/[[:space:]]*$//')"
        if [[ "$expr" == "true" || "$expr" == "false" ]]; then
          value="$expr"
        elif [[ -n "${bool_env[$expr]+set}" ]]; then
          value="${bool_env[$expr]}"
        else
          return 1
        fi
        if [[ "$branch" == "then" ]]; then
          then_value="$value"
        elif [[ "$branch" == "else" ]]; then
          else_value="$value"
        else
          return 1
        fi
      elif [[ -z "$line" || "$line" == "}" ]]; then
        :
      else
        return 1
      fi
    fi
    depth=$(( depth + $(sley_count_char_text "$raw" "{") - $(sley_count_char_text "$raw" "}") ))
    if "$in_if" && (( depth <= 1 )); then
      [[ -n "$then_value" && -n "$else_value" ]] || return 1
      runtime_value="$(eval_runtime_if_value_task "Bool" "$condition_value" "$then_value" "$else_value")" || return 1
      printf '{"kind":"%s","value":"%s"}\n' "$RUNTIME_BOOL_VALUE_KIND" "$runtime_value"
      return 0
    fi
    if (( depth <= 0 )); then
      break
    fi
  done < "$file"
  return 1
}

eval_source_literal_if_int_statement_main_return() {
  local target="$1" file raw line depth=0 in_main=false in_if=false saw_if=false rest prefix name expr value condition_value then_value="" else_value="" runtime_value
  declare -A int_env=()
  file="$(sley_source_entry_file_for_target "$target")"
  [[ -n "$file" ]] || return 1
  while IFS= read -r raw; do
    line="$(printf '%s\n' "$raw" | sed 's/^[[:space:]]*//; s/[[:space:]]*$//')"
    if ! "$in_main"; then
      if [[ "$line" =~ ^(export[[:space:]]+)?task[[:space:]]+main[[:space:]] ]]; then
        in_main=true
        depth=$(( $(sley_count_char_text "$raw" "{") - $(sley_count_char_text "$raw" "}") ))
      fi
      continue
    fi
    if (( depth == 1 )); then
      if [[ "$line" =~ ^(bind|state)[[:space:]]+[A-Za-z_][A-Za-z0-9_]*[[:space:]]*= ]]; then
        prefix="${BASH_REMATCH[1]}"
        rest="$line"
        rest="${rest#"$prefix"}"
        rest="$(printf '%s\n' "$rest" | sed 's/^[[:space:]]*//')"
        name="${rest%%=*}"
        name="$(printf '%s\n' "$name" | sed 's/[[:space:]]*$//')"
        expr="${rest#*=}"
        value="$(eval_state_set_int_expr "$expr" int_env)" || return 1
        int_env["$name"]="$value"
      elif [[ "$line" =~ ^if[[:space:]]+(true|false)[[:space:]]*\{[[:space:]]*$ ]]; then
        condition_value="${BASH_REMATCH[1]}"
        in_if=true
      elif [[ "$line" =~ ^return[[:space:]]+ ]]; then
        "$saw_if" || return 1
        expr="${line#return}"
        else_value="$(eval_state_set_int_expr "$expr" int_env)" || return 1
        runtime_value="$(eval_runtime_if_value_task "Int" "$condition_value" "$then_value" "$else_value")" || return 1
        printf '{"kind":"%s","value":"%s"}\n' "$RUNTIME_INT_VALUE_KIND" "$runtime_value"
        return 0
      elif [[ -z "$line" || "$line" == "}" ]]; then
        :
      else
        return 1
      fi
    elif "$in_if" && (( depth == 2 )); then
      if [[ "$line" =~ ^return[[:space:]]+ ]]; then
        expr="${line#return}"
        then_value="$(eval_state_set_int_expr "$expr" int_env)" || return 1
      elif [[ -z "$line" || "$line" == "}" ]]; then
        :
      else
        return 1
      fi
    fi
    depth=$(( depth + $(sley_count_char_text "$raw" "{") - $(sley_count_char_text "$raw" "}") ))
    if "$in_if" && (( depth <= 1 )); then
      [[ -n "$then_value" ]] || return 1
      in_if=false
      saw_if=true
    fi
    if (( depth <= 0 )); then
      break
    fi
  done < "$file"
  return 1
}

eval_source_comparison_if_int_statement_main_return() {
  local target="$1" file raw line depth=0 in_main=false in_if=false branch="" rest prefix name expr value condition left_token right_token left right condition_value then_value="" else_value="" runtime_value
  declare -A int_env=()
  file="$(sley_source_entry_file_for_target "$target")"
  [[ -n "$file" ]] || return 1
  while IFS= read -r raw; do
    line="$(printf '%s\n' "$raw" | sed 's/^[[:space:]]*//; s/[[:space:]]*$//')"
    if ! "$in_main"; then
      if [[ "$line" =~ ^(export[[:space:]]+)?task[[:space:]]+main[[:space:]] ]]; then
        in_main=true
        depth=$(( $(sley_count_char_text "$raw" "{") - $(sley_count_char_text "$raw" "}") ))
      fi
      continue
    fi
    if (( depth == 1 )); then
      if [[ "$line" =~ ^(bind|state)[[:space:]]+[A-Za-z_][A-Za-z0-9_]*[[:space:]]*= ]]; then
        prefix="${BASH_REMATCH[1]}"
        rest="$line"
        rest="${rest#"$prefix"}"
        rest="$(printf '%s\n' "$rest" | sed 's/^[[:space:]]*//')"
        name="${rest%%=*}"
        name="$(printf '%s\n' "$name" | sed 's/[[:space:]]*$//')"
        expr="${rest#*=}"
        value="$(eval_state_set_int_expr "$expr" int_env)" || return 1
        int_env["$name"]="$value"
      elif [[ "$line" =~ ^if[[:space:]]+.+[[:space:]]*\{[[:space:]]*$ ]]; then
        condition="$line"
        condition="${condition#if}"
        condition="${condition%\{}"
        condition="$(printf '%s\n' "$condition" | sed 's/^[[:space:]]*//; s/[[:space:]]*$//')"
        [[ "$condition" =~ ^([A-Za-z_][A-Za-z0-9_]*|-?[0-9]+)[[:space:]]*([<>])[[:space:]]*([A-Za-z_][A-Za-z0-9_]*|-?[0-9]+)$ ]] || return 1
        left_token="${BASH_REMATCH[1]}"
        right_token="${BASH_REMATCH[3]}"
        if [[ "$left_token" =~ ^-?[0-9]+$ ]]; then
          left="$left_token"
        else
          [[ -n "${int_env[$left_token]+set}" ]] || return 1
          left="${int_env[$left_token]}"
        fi
        if [[ "$right_token" =~ ^-?[0-9]+$ ]]; then
          right="$right_token"
        else
          [[ -n "${int_env[$right_token]+set}" ]] || return 1
          right="${int_env[$right_token]}"
        fi
        case "${BASH_REMATCH[2]}" in
          "<"|">") condition_value="$(eval_runtime_int_comparison_operator_value_task "${BASH_REMATCH[2]}" "$left" "$right")" ;;
          *) return 1 ;;
        esac
        [[ "$condition_value" == "true" || "$condition_value" == "false" ]] || return 1
        in_if=true
        branch="then"
      elif [[ -z "$line" || "$line" == "}" ]]; then
        :
      else
        return 1
      fi
    elif "$in_if" && (( depth == 2 )); then
      if [[ "$line" =~ ^\}[[:space:]]*else[[:space:]]*\{[[:space:]]*$ ]]; then
        branch="else"
      elif [[ "$line" =~ ^return[[:space:]]+ ]]; then
        expr="${line#return}"
        value="$(eval_state_set_int_expr "$expr" int_env)" || return 1
        if [[ "$branch" == "then" ]]; then
          then_value="$value"
        elif [[ "$branch" == "else" ]]; then
          else_value="$value"
        else
          return 1
        fi
      elif [[ -z "$line" || "$line" == "}" ]]; then
        :
      else
        return 1
      fi
    fi
    depth=$(( depth + $(sley_count_char_text "$raw" "{") - $(sley_count_char_text "$raw" "}") ))
    if "$in_if" && (( depth <= 1 )); then
      [[ -n "$then_value" && -n "$else_value" ]] || return 1
      runtime_value="$(eval_runtime_if_value_task "Int" "$condition_value" "$then_value" "$else_value")" || return 1
      printf '{"kind":"%s","value":"%s"}\n' "$RUNTIME_INT_VALUE_KIND" "$runtime_value"
      return 0
    fi
    if (( depth <= 0 )); then
      break
    fi
  done < "$file"
  return 1
}

eval_source_false_while_int_main_return() {
  local target="$1" file raw line depth=0 in_main=false in_while=false saw_while=false rest prefix name expr value body_value="" fallthrough_value="" runtime_value
  declare -A int_env=()
  file="$(sley_source_entry_file_for_target "$target")"
  [[ -n "$file" ]] || return 1
  while IFS= read -r raw; do
    line="$(printf '%s\n' "$raw" | sed 's/^[[:space:]]*//; s/[[:space:]]*$//')"
    if ! "$in_main"; then
      if [[ "$line" =~ ^(export[[:space:]]+)?task[[:space:]]+main[[:space:]] ]]; then
        in_main=true
        depth=$(( $(sley_count_char_text "$raw" "{") - $(sley_count_char_text "$raw" "}") ))
      fi
      continue
    fi
    if (( depth == 1 )); then
      if [[ "$line" =~ ^(bind|state)[[:space:]]+[A-Za-z_][A-Za-z0-9_]*[[:space:]]*= ]]; then
        prefix="${BASH_REMATCH[1]}"
        rest="$line"
        rest="${rest#"$prefix"}"
        rest="$(printf '%s\n' "$rest" | sed 's/^[[:space:]]*//')"
        name="${rest%%=*}"
        name="$(printf '%s\n' "$name" | sed 's/[[:space:]]*$//')"
        expr="${rest#*=}"
        value="$(eval_state_set_int_expr "$expr" int_env)" || return 1
        int_env["$name"]="$value"
      elif [[ "$line" =~ ^while[[:space:]]+false[[:space:]]*\{[[:space:]]*$ ]]; then
        in_while=true
      elif [[ "$line" =~ ^return[[:space:]]+ ]]; then
        "$saw_while" || return 1
        expr="${line#return}"
        fallthrough_value="$(eval_state_set_int_expr "$expr" int_env)" || return 1
        runtime_value="$(eval_runtime_if_value_task "Int" false "$body_value" "$fallthrough_value")" || return 1
        printf '{"kind":"%s","value":"%s"}\n' "$RUNTIME_INT_VALUE_KIND" "$runtime_value"
        return 0
      elif [[ -z "$line" || "$line" == "}" ]]; then
        :
      else
        return 1
      fi
    elif "$in_while" && (( depth == 2 )); then
      if [[ "$line" =~ ^return[[:space:]]+ ]]; then
        expr="${line#return}"
        body_value="$(eval_state_set_int_expr "$expr" int_env)" || return 1
      elif [[ -z "$line" || "$line" == "}" ]]; then
        :
      else
        return 1
      fi
    fi
    depth=$(( depth + $(sley_count_char_text "$raw" "{") - $(sley_count_char_text "$raw" "}") ))
    if "$in_while" && (( depth <= 1 )); then
      [[ -n "$body_value" ]] || return 1
      in_while=false
      saw_while=true
    fi
    if (( depth <= 0 )); then
      break
    fi
  done < "$file"
  return 1
}

eval_source_if_int_main_return() {
  local target="$1" file
  file="$(sley_source_entry_file_for_target "$target")"
  [[ -n "$file" ]] || return 1
  awk -v int_value_kind="$RUNTIME_INT_VALUE_KIND" '
    function trim(s){gsub(/^[ \t\r\n]+|[ \t\r\n]+$/, "", s); return s}
    function count_char(s, c, n, i){n=0; for(i=1;i<=length(s);i++){if(substr(s,i,1)==c)n++} return n}
    function if_int_expr(expr, tmp, cond, then_value, else_value) {
      expr=trim(expr)
      if(expr ~ /^if[ \t]+(true|false|[A-Za-z_][A-Za-z0-9_]*)[ \t]*\{[ \t]*-?[0-9]+[ \t]*\}[ \t]*else[ \t]*\{[ \t]*-?[0-9]+[ \t]*\}$/) {
        tmp=expr
        sub(/^if[ \t]+/,"",tmp)
        cond=tmp
        sub(/[ \t]*\{.*/,"",cond)
        then_value=tmp
        sub(/^[^{]*\{[ \t]*/,"",then_value)
        sub(/[ \t]*\}.*/,"",then_value)
        else_value=tmp
        sub(/^.*else[ \t]*\{[ \t]*/,"",else_value)
        sub(/[ \t]*\}[ \t]*$/,"",else_value)
        return eval_cond(cond) ? then_value : else_value
      }
      return ""
    }
    function assign_value(line, prefix, rest, name, expr, value) {
      rest=line
      sub("^" prefix "[ \t]+","",rest)
      name=rest
      sub(/[ \t]*=.*/,"",name)
      expr=rest
      sub(/^[^=]*=[ \t]*/,"",expr)
      value=eval_expr(expr)
      if(value != "") ints[name]=value
      else if(expr == "true") bools[name]=1
      else if(expr == "false") bools[name]=0
    }
    function eval_atom(expr, t, if_value) {
      t=trim(expr)
      if_value=if_int_expr(t)
      if(if_value != "") return if_value
      if(t ~ /^-?[0-9]+$/) return t
      if(t in ints) return ints[t]
      return ""
    }
    function eval_expr(expr, t, parts, left, right) {
      t=trim(expr)
      if(index(t, "+") > 0) {
        split(t, parts, "+")
        left=eval_atom(parts[1])
        right=eval_atom(parts[2])
        if(left != "" && right != "") return left + right
      }
      return eval_atom(t)
    }
    function eval_cond(cond, left, right, parts) {
      cond=trim(cond)
      if(cond == "true") return 1
      if(cond == "false") return 0
      if(cond in bools) return bools[cond]
      if(index(cond, ">") > 0) {
        split(cond, parts, ">")
        left=eval_expr(parts[1])
        right=eval_expr(parts[2])
        return (left != "" && right != "" && left + 0 > right + 0)
      }
      return 0
    }
    function emit(value) {
      if(value != "") {
        printf "{\"kind\":\"%s\",\"value\":\"%s\"}\n", int_value_kind, value
        exit 0
      }
    }
    /^[ \t]*(export[ \t]+)?task[ \t]+main[ \t]/ {
      in_main=1
      depth=count_char($0,"{")-count_char($0,"}")
      active_if=0
      in_if=0
      next
    }
    in_main {
      raw=$0
      line=trim(raw)
      if(depth == 1 && line ~ /^(bind|state)[ \t]+[A-Za-z_][A-Za-z0-9_]*[ \t]*=/) {
        if(line ~ /^bind[ \t]+/) assign_value(line, "bind")
        else assign_value(line, "state")
      } else if(depth == 1 && line ~ /^if[ \t].*\{[ \t]*$/) {
        cond=line
        sub(/^if[ \t]+/,"",cond)
        sub(/[ \t]*\{[ \t]*$/,"",cond)
        if_condition=eval_cond(cond)
        active_if=if_condition
        in_if=1
      } else if(in_if && depth == 2 && line ~ /^}[ \t]*else[ \t]*\{[ \t]*$/) {
        active_if=!if_condition
      } else if(active_if && depth == 2 && line ~ /^set[ \t]+[A-Za-z_][A-Za-z0-9_]*[ \t]*=/) {
        assign_value(line, "set")
      } else if(active_if && line ~ /^return[ \t]+/) {
        expr=line
        sub(/^return[ \t]+/,"",expr)
        emit(eval_expr(expr))
      } else if(depth == 1 && line ~ /^return[ \t]+/) {
        expr=line
        sub(/^return[ \t]+/,"",expr)
        emit(eval_expr(expr))
      }
      depth += count_char(raw,"{")-count_char(raw,"}")
      if(depth <= 1) {
        active_if=0
        if_condition=0
        in_if=0
      }
      if(depth <= 0) exit 1
    }
  ' "$file"
}

eval_source_bool_main_return() {
  local target="$1" file
  file="$(sley_source_entry_file_for_target "$target")"
  [[ -n "$file" ]] || return 1
  awk -v bool_value_kind="$RUNTIME_BOOL_VALUE_KIND" '
    function trim(s){gsub(/^[ \t\r\n]+|[ \t\r\n]+$/, "", s); return s}
    function count_char(s, c, n, i){n=0; for(i=1;i<=length(s);i++){if(substr(s,i,1)==c)n++} return n}
    function int_expr(expr, t) {
      t=trim(expr)
      if(t ~ /^-?[0-9]+$/) return t + 0
      if(t in ints) return ints[t] + 0
      return ""
    }
    function compare_expr(expr, op, parts, left, right) {
      expr=trim(expr)
      if(index(expr, ">=") > 0) op=">="
      else if(index(expr, "<=") > 0) op="<="
      else if(index(expr, "==") > 0) op="=="
      else if(index(expr, "!=") > 0) op="!="
      else if(index(expr, ">") > 0) op=">"
      else if(index(expr, "<") > 0) op="<"
      else return ""
      split(expr, parts, op)
      left=int_expr(parts[1])
      right=int_expr(parts[2])
      if(left == "" || right == "") return ""
      if(op == ">=") return (left >= right) ? "1" : "0"
      if(op == "<=") return (left <= right) ? "1" : "0"
      if(op == "==") return (left == right) ? "1" : "0"
      if(op == "!=") return (left != right) ? "1" : "0"
      if(op == ">") return (left > right) ? "1" : "0"
      if(op == "<") return (left < right) ? "1" : "0"
      return ""
    }
    function bool_expr(expr, parts, left, right) {
      expr=trim(expr)
      if(expr == "true") return "1"
      if(expr == "false") return "0"
      if(expr in bools) return bools[expr] ? "1" : "0"
      if(expr ~ /^![ \t]*/) {
        right=bool_expr(substr(expr, 2))
        if(right == "") return ""
        return right == "1" ? "0" : "1"
      }
      if(index(expr, "&&") > 0) {
        split(expr, parts, "&&")
        left=bool_expr(parts[1])
        right=bool_expr(parts[2])
        if(left == "" || right == "") return ""
        return (left == "1" && right == "1") ? "1" : "0"
      }
      if(index(expr, "||") > 0) {
        split(expr, parts, "[|][|]")
        left=bool_expr(parts[1])
        right=bool_expr(parts[2])
        if(left == "" || right == "") return ""
        return (left == "1" || right == "1") ? "1" : "0"
      }
      return compare_expr(expr)
    }
    function emit_bool(value) {
      if(value != "") {
        printf "{\"kind\":\"%s\",\"value\":\"%s\"}\n", bool_value_kind, value == "1" ? "true" : "false"
        exit 0
      }
    }
    /^[ \t]*(export[ \t]+)?task[ \t]+main[ \t]/ {
      in_main=1
      depth=count_char($0,"{")-count_char($0,"}")
      next
    }
    in_main {
      raw=$0
      line=trim(raw)
      if(depth == 1 && line ~ /^bind[ \t]+[A-Za-z_][A-Za-z0-9_]*[ \t]*=/) {
        rest=line
        sub(/^bind[ \t]+/,"",rest)
        name=rest
        sub(/[ \t]*=.*/,"",name)
        expr=rest
        sub(/^[^=]*=[ \t]*/,"",expr)
        if(expr ~ /^-?[0-9]+$/) ints[name]=expr
        else if(expr == "true") bools[name]=1
        else if(expr == "false") bools[name]=0
      } else if(depth == 1 && line ~ /^return[ \t]+/) {
        expr=line
        sub(/^return[ \t]+/,"",expr)
        emit_bool(bool_expr(expr))
      }
      depth += count_char(raw,"{")-count_char(raw,"}")
      if(depth <= 0) exit 1
    }
  ' "$file"
}

eval_source_unit_main_return() {
  local target="$1" file matched
  file="$(sley_source_entry_file_for_target "$target")"
  [[ -n "$file" ]] || return 1
  matched="$(awk '
    function trim(s){gsub(/^[ \t\r\n]+|[ \t\r\n]+$/, "", s); return s}
    function count_char(s, c, n, i){n=0; for(i=1;i<=length(s);i++){if(substr(s,i,1)==c)n++} return n}
    /^[ \t]*(export[ \t]+)?task[ \t]+main[ \t]*->[ \t]*Unit[ \t]*[{]/ {
      in_main=1
      depth=count_char($0,"{")-count_char($0,"}")
      if(depth <= 0) {
        print "unit"
        exit 0
      }
      next
    }
    in_main {
      raw=$0
      line=trim(raw)
      depth += count_char(raw,"{")-count_char(raw,"}")
      if(depth <= 0) {
        print "unit"
        exit 0
      }
      if(line != "") exit 1
    }
    END { if(!in_main) exit 1 }
  ' "$file")" || return 1
  [[ "$matched" == "unit" ]] || return 1
  printf '{"kind":"%s","value":""}\n' "$RUNTIME_UNIT_VALUE_KIND"
}

sley_source_main_return_type() {
  local file="$1"
  awk '
    function trim(s){gsub(/^[ \t\r\n]+|[ \t\r\n]+$/, "", s); return s}
    /^[ \t]*(export[ \t]+)?task[ \t]+main[ \t]*->[ \t]*/ {
      line=$0
      sub(/^[ \t]*(export[ \t]+)?task[ \t]+main[ \t]*->[ \t]*/, "", line)
      sub(/[ \t]+uses[ \t]+.*/, "", line)
      sub(/[ \t]*\{.*/, "", line)
      print trim(line)
      exit
    }
  ' "$file"
}

sley_source_type_alias_target() {
  local file="$1" type_name="$2"
  awk -v type_name="$type_name" '
    function trim(s){gsub(/^[ \t\r\n]+|[ \t\r\n]+$/, "", s); return s}
    $0 ~ "^[ \t]*(export[ \t]+)?type[ \t]+" type_name "[ \t]*=" {
      line=$0
      sub(/^[ \t]*(export[ \t]+)?type[ \t]+[A-Za-z_][A-Za-z0-9_]*[ \t]*=[ \t]*/, "", line)
      print trim(line)
      exit
    }
  ' "$file"
}

sley_resolve_source_return_type_alias() {
  local file="$1" return_type="$2" alias_target depth=0
  while [[ "$return_type" =~ ^[A-Za-z_][A-Za-z0-9_]*$ ]]; do
    alias_target="$(sley_source_type_alias_target "$file" "$return_type")"
    [[ -n "$alias_target" ]] || break
    return_type="$alias_target"
    depth=$((depth + 1))
    [[ "$depth" -le 32 ]] || return 1
  done
  printf '%s\n' "$return_type"
}

runtime_generic_return_base() {
  local return_type="$1"
  if [[ "$return_type" == Result\<* ]]; then
    printf 'Result\n'
  else
    printf '%s\n' "$return_type"
  fi
}

runtime_generic_result_type() {
  local return_type="$1" result_re='^Result<([^,>]+),'
  if [[ "$return_type" =~ $result_re ]]; then
    printf '%s\n' "${BASH_REMATCH[1]}" | sed 's/^[[:space:]]*//;s/[[:space:]]*$//'
  fi
}

runtime_generic_return_descriptor_json() {
  local return_type="$1" result_status="$2" result_type="${3:-}" return_base
  return_base="$(runtime_generic_return_base "$return_type")"
  printf '%s\n' "$RUNTIME_GENERIC_RETURN_PLAN_JSON" | jq -cer \
    --arg return_base "$return_base" \
    --arg result_status "$result_status" \
    --arg result_type "$result_type" '
      [
        .[]
        | select(.return_base == $return_base)
        | select(.result_status == $result_status)
        | select((.result_type == "") or (.result_type == $result_type))
      ][0] // empty
    '
}

runtime_generic_value_env_json() {
  local value_kind="$1" value="$2"
  if [[ "$value_kind" == "$RUNTIME_INT_VALUE_KIND" || "$value_kind" == "$RUNTIME_OK_INT_VALUE_KIND" ]]; then
    [[ "$value" =~ ^-?[0-9]+$ ]] || return 1
    jq -cn --argjson value "$value" '{value:$value}'
  elif [[ "$value_kind" == "$RUNTIME_BOOL_VALUE_KIND" ]]; then
    [[ "$value" == "true" || "$value" == "false" ]] || return 1
    jq -cn --argjson value "$value" '{value:$value}'
  elif [[ "$value_kind" == "$RUNTIME_TEXT_VALUE_KIND" || "$value_kind" == "$RUNTIME_OK_TEXT_VALUE_KIND" || "$value_kind" == "$RUNTIME_ERR_TEXT_VALUE_KIND" ]]; then
    jq -cn --arg value "$value" '{value:$value}'
  else
    return 1
  fi
}

runtime_generic_eval_value_task() {
  local value_kind="$1" value_task="$2" value="$3" runtime_file="$SELF_HOSTED_SOURCE_ROOT/loom/runtime.sley" env_json
  [[ "$value_task" =~ ^[A-Za-z_][A-Za-z0-9_]*$ ]] || return 1
  env_json="$(runtime_generic_value_env_json "$value_kind" "$value")" || return 1
  sley_eval_source_task "$runtime_file" "$value_task" "$env_json"
}

sley_source_main_body_supports_generic_runtime() {
  local file="$1" body prefix
  body="$(extract_sley_task_body "$file" main)" || return 1
  [[ -n "$body" ]] || return 1
  while IFS= read -r prefix || [[ -n "$prefix" ]]; do
    [[ -n "$prefix" ]] || continue
    if printf '%s\n' "$body" | grep -Eq "^[[:space:]]*$prefix([[:space:]{}]|$)"; then
      return 1
    fi
  done < <(printf '%s\n' "$RUNTIME_GENERIC_UNSUPPORTED_BLOCK_PREFIXES_JSON" | jq -r '.[]')
  return 0
}

eval_source_generic_main_return() {
  local target="$1" http_text="$2" model_text="$3" deploy_text="$4" spend_text="$5" secret_text="$6" shell_text="$7"
  local file return_type return_base result_type result_status descriptor_json value_kind value_task env_json value runtime_value body expr
  file="$(sley_source_entry_file_for_target "$target")"
  [[ -n "$file" ]] || return 1
  sley_source_main_body_supports_generic_runtime "$file" || return 1
  return_type="$(sley_source_main_return_type "$file")"
  [[ -n "$return_type" ]] || return 1
  return_type="$(sley_resolve_source_return_type_alias "$file" "$return_type")" || return 1
  env_json="$(sley_runtime_seed_env_json "$http_text" "$model_text" "$deploy_text" "$spend_text" "$secret_text" "$shell_text")" || return 1
  return_base="$(runtime_generic_return_base "$return_type")"
  if [[ "$return_base" == "Result" ]]; then
    body="$(extract_sley_task_body "$file" main)" || return 1
    [[ -n "$body" ]] || return 1
    env_json="$(sley_eval_source_bind_env_json "$file" "$env_json" "$body")" || return 1
    expr="$(sley_select_source_return_expr "$body" "$env_json")" || return 1
    if [[ "$expr" =~ ^Err\((.*)\)$ ]]; then
      result_status="err"
      result_type=""
      value="$(sley_eval_source_expr "$file" "$env_json" "${BASH_REMATCH[1]}")" || return 1
    else
      result_status="ok"
      result_type="$(runtime_generic_result_type "$return_type")"
      [[ -n "$result_type" ]] || return 1
      value="$(sley_eval_source_expr "$file" "$env_json" "$expr")" || return 1
    fi
  else
    result_status="value"
    result_type=""
    value="$(sley_eval_source_task "$file" main "$env_json")" || return 1
  fi
  descriptor_json="$(runtime_generic_return_descriptor_json "$return_type" "$result_status" "$result_type")" || return 1
  value_kind="$(printf '%s\n' "$descriptor_json" | jq -er '.value_kind')" || return 1
  value_task="$(printf '%s\n' "$descriptor_json" | jq -er '.value_task')" || return 1
  runtime_value="$(runtime_generic_eval_value_task "$value_kind" "$value_task" "$value")" || return 1
  jq -cn --arg kind "$value_kind" --arg value "$runtime_value" '{kind:$kind,value:$value}'
}

eval_source_value_main_return() {
  local target="$1" expected_return_type="${2:-}" file return_type value descriptor_json value_kind value_task runtime_value
  file="$(sley_source_entry_file_for_target "$target")"
  [[ -n "$file" ]] || return 1
  return_type="$(sley_source_main_return_type "$file")"
  [[ -n "$return_type" ]] || return 1
  [[ -z "$expected_return_type" || "$return_type" == "$expected_return_type" ]] || return 1
  case "$return_type" in
    Int|Text|Bool) ;;
    *) return 1 ;;
  esac
  value="$(sley_eval_source_task "$file" main '{}')" || return 1
  descriptor_json="$(runtime_generic_return_descriptor_json "$return_type" "value")" || return 1
  value_kind="$(printf '%s\n' "$descriptor_json" | jq -er '.value_kind')" || return 1
  value_task="$(printf '%s\n' "$descriptor_json" | jq -er '.value_task')" || return 1
  runtime_value="$(runtime_generic_eval_value_task "$value_kind" "$value_task" "$value")" || return 1
  jq -cn --arg kind "$value_kind" --arg value "$runtime_value" '{kind:$kind,value:$value}'
}

eval_source_local_call_int_main_return() {
  eval_source_value_main_return "$1" "Int"
}

runtime_file_read_seed_text() {
  local path="$1" source_file text
  [[ -n "$path" && "$path" != /* ]] || return 1
  source_file="$ROOT_DIR/$path"
  [[ -f "$source_file" ]] || return 1
  text="$(sley_eval_source_task "$source_file" main '{}')" || return 1
  [[ -n "$text" ]] || return 1
  eval_runtime_text_identity_value_task "$text"
}

runtime_context_files() {
  local target="$1" dir
  if [[ -f "$target" ]]; then
    dir="$(cd "$(dirname "$target")" && pwd)" || return 1
    while [[ "$dir" != "/" ]]; do
      if [[ -f "$dir/sley.toml" ]]; then
        collect_files "$dir"
        return
      fi
      dir="$(dirname "$dir")"
    done
  fi
  collect_files "$target"
}

resolve_project_import_prefix() {
  local file="$1" prefix="$2"
  [[ -f "$file" && -n "$prefix" ]] || return 1
  awk -v prefix="$prefix" '
    function trim(s){gsub(/^[ \t\r\n]+|[ \t\r\n]+$/, "", s); return s}
    /^[ \t]*import[ \t]+/ {
      line=$0
      sub(/^[ \t]*import[ \t]+/, "", line)
      line=trim(line)
      if(line ~ "[ \t]+as[ \t]+" prefix "[ \t]*$") {
        sub(/[ \t]+as[ \t]+.*$/, "", line)
        print trim(line)
        exit 0
      }
      if(line !~ /[ \t]+as[ \t]+/) {
        module=line
        alias=module
        sub(/^.*[.]/, "", alias)
        if(alias == prefix) {
          print module
          exit 0
        }
      }
    }
  ' "$file"
}

eval_source_direct_file_read_text_main_return() {
  eval_source_file_read_main_return "$1" "Text"
}

eval_source_direct_file_read_ok_text_main_return() {
  eval_source_file_read_main_return "$1" "Result"
}

runtime_source_has_seeded_host_effect() {
  local target="$1" effect
  for effect in FileRead FileWrite DatabaseRead DatabaseWrite Network Shell ModelCall SecretRead Deploy Spend; do
    runtime_source_has_host_effect "$target" "$effect" && return 0
  done
  return 1
}

eval_source_return_type_matches() {
  local target="$1" expected_return_base="$2" file return_type
  file="$(sley_source_entry_file_for_target "$target")"
  [[ -n "$file" ]] || return 1
  return_type="$(sley_source_main_return_type "$file")"
  [[ -n "$return_type" ]] || return 1
  if [[ "$expected_return_base" == "Result" ]]; then
    [[ "$(runtime_generic_return_base "$return_type")" == "Result" ]]
  else
    [[ "$return_type" == "$expected_return_base" ]]
  fi
}

eval_source_file_read_main_return() {
  local target="$1" expected_return_base="$2"
  runtime_source_has_host_effect "$target" "FileRead" || return 1
  eval_source_return_type_matches "$target" "$expected_return_base" || return 1
  eval_source_generic_main_return "$target"
}

eval_source_seeded_host_main_return() {
  local target="$1" expected_return_base="$2"
  shift 2
  runtime_source_has_seeded_host_effect "$target" || return 1
  eval_source_return_type_matches "$target" "$expected_return_base" || return 1
  eval_source_generic_main_return "$target" "$@"
}

eval_source_seeded_host_result_main_return() {
  local target="$1"
  shift
  eval_source_seeded_host_main_return "$target" "Result" "$@"
}

eval_source_seeded_host_text_main_return() {
  local target="$1"
  shift
  eval_source_seeded_host_main_return "$target" "Text" "$@"
}

eval_source_zero_arg_project_call_main_return() {
  eval_source_value_main_return "$1"
}

eval_source_project_call_int_main_return() {
  eval_source_value_main_return "$1" "Int"
}

eval_main_call_int_return() {
  eval_source_value_main_return "$1" "Int"
}

require_source_text() {
  local name="$1" value="$2"
  if [[ -z "$value" ]]; then
    printf 'sley: required Sley source text task returned empty: %s\n' "$name" >&2
    exit 1
  fi
}

require_source_list() {
  local name="$1" value="$2"
  if [[ "$value" != \[* || "$value" == "[]" ]]; then
    printf 'sley: required Sley source list task returned empty: %s\n' "$name" >&2
    exit 1
  fi
}

VERSION="$(sley_source_task loom.bootstrap implementation_version)"
require_source_text VERSION "$VERSION"
TARGET_RELEASE="$(sley_source_task loom.bootstrap target_release)"
AI_BOOTSTRAP_VERSION="$(sley_source_task loom.bootstrap ai_bootstrap_version)"
AI_BOOTSTRAP_PATH="$(sley_source_task loom.bootstrap ai_bootstrap_path)"
AI_BOOTSTRAP_DIGEST="$(sley_source_task loom.bootstrap ai_bootstrap_digest)"
require_source_text TARGET_RELEASE "$TARGET_RELEASE"
require_source_text AI_BOOTSTRAP_VERSION "$AI_BOOTSTRAP_VERSION"
require_source_text AI_BOOTSTRAP_PATH "$AI_BOOTSTRAP_PATH"
require_source_text AI_BOOTSTRAP_DIGEST "$AI_BOOTSTRAP_DIGEST"

RELEASE_VERSION="$(sley_source_task loom.release release_version)"
RELEASE_ARTIFACT_ID="$(sley_source_task loom.release artifact_id)"
RELEASE_PLATFORM_OS="$(sley_source_task loom.release platform_os)"
RELEASE_PLATFORM_ARCHITECTURE="$(sley_source_task loom.release platform_architecture)"
RELEASE_ARCHIVE_FORMAT="$(sley_source_task loom.release archive_format)"
RELEASE_SUPPORT_STATUS="$(sley_source_task loom.release support_status)"
RELEASE_AUTHORITY_MODE="$(sley_source_task loom.release authority_mode)"
SCHEMA_RELEASE_MANIFEST="$(sley_source_task loom.release manifest_schema)"
SCHEMA_RELEASE_LICENSE_INVENTORY="$(sley_source_task loom.release license_inventory_schema)"
SCHEMA_RELEASE_PROVENANCE="$(sley_source_task loom.release provenance_schema)"
SCHEMA_RELEASE_VERIFICATION="$(sley_source_task loom.release verification_schema)"
SCHEMA_TOOLCHAIN_DOCTOR="$(sley_source_task loom.release toolchain_doctor_schema)"
RELEASE_REQUIRED_TOOLS_JSON="$(sley_source_list_task_json loom.release required_tools)"
RELEASE_ENTRYPOINTS_JSON="$(sley_source_list_task_json loom.release archive_entrypoints)"
RELEASE_METADATA_PATHS_JSON="$(sley_source_list_task_json loom.release release_metadata_paths)"
RELEASE_PUBLICATION_AUTHORIZED="$(sley_source_task loom.release publication_authorized)"

for _sley_release_text_name in \
  RELEASE_VERSION RELEASE_ARTIFACT_ID RELEASE_PLATFORM_OS RELEASE_PLATFORM_ARCHITECTURE \
  RELEASE_ARCHIVE_FORMAT RELEASE_SUPPORT_STATUS RELEASE_AUTHORITY_MODE \
  SCHEMA_RELEASE_MANIFEST SCHEMA_RELEASE_LICENSE_INVENTORY SCHEMA_RELEASE_PROVENANCE \
  SCHEMA_RELEASE_VERIFICATION SCHEMA_TOOLCHAIN_DOCTOR RELEASE_PUBLICATION_AUTHORIZED; do
  require_source_text "$_sley_release_text_name" "${!_sley_release_text_name}"
done
for _sley_release_list_name in RELEASE_REQUIRED_TOOLS_JSON RELEASE_ENTRYPOINTS_JSON RELEASE_METADATA_PATHS_JSON; do
  require_source_list "$_sley_release_list_name" "${!_sley_release_list_name}"
done
unset _sley_release_text_name _sley_release_list_name

RUNTIME_TEXT_BINARY_OPERATOR_PLAN_JSON="$(sley_source_list_task_json loom.runtime text_binary_operator_plan | jq 'map(split("=>") | {op:.[0], task:.[1]})')"
RUNTIME_COLLECTION_INDEX_OPERATOR_PLAN_JSON="$(sley_source_list_task_json loom.runtime collection_index_operator_plan | jq 'map(split("|") | {collection_kind:.[0], value_kind:.[1], task:.[2]})')"
RUNTIME_RECORD_FIELD_ACCESS_PLAN_JSON="$(sley_source_list_task_json loom.runtime record_field_access_plan | jq 'map(split("|") | {field:.[0], value_kind:.[1], task:.[2]})')"

DEFAULT_RULES_JSON="$(sley_source_list_task_json loom.lint default_lint_rules)"

REPORT_BUILDER_VALUE_TYPE_PLAN_JSON="$(sley_source_list_task_json loom.reports report_builder_value_type_plan | jq 'map(split("|") | {value_type:.[0], json_type:.[1], item_type:(.[2] // "")})')"
REPORT_BUILDER_REGISTRY_JSON="$(sley_source_list_task_json loom.reports report_builder_registry | jq 'map(split("|") | {namespace:.[0], schema_task:.[1], builder_task:.[2]})')"
ADAPTER_LIFECYCLE_STATES_JSON="$(sley_source_list_task_json loom.adapter lifecycle_states)"
ADAPTER_REPLAY_EVENT_PHASES_JSON="$(sley_source_list_task_json loom.adapter replay_event_phases)"
ADAPTER_FAILURE_CODES_JSON="$(sley_source_list_task_json loom.adapter failure_codes)"
ADAPTER_SEED_FAMILIES_JSON="$(sley_source_list_task_json loom.adapter deterministic_seed_families)"
ADAPTER_SUPPORTED_EFFECTS_JSON="$(sley_source_list_task_json loom.adapter supported_effects)"
ADAPTER_AUTHORITY_MODE="$(sley_source_task loom.adapter authority_mode)"
ADAPTER_REPLAY_POLICY="$(sley_source_task loom.adapter replay_policy)"
ADAPTER_KIND="$(sley_source_task loom.adapter adapter_kind)"
ADAPTER_LIVE_EXECUTION="$(sley_source_task loom.adapter live_execution)"
WORKER_PROTOCOL="$(sley_source_task loom.worker protocol_version)"
WORKER_REQUEST_SCHEMA="$(sley_source_task loom.worker request_schema)"
WORKER_RESPONSE_SCHEMA="$(sley_source_task loom.worker response_schema)"
WORKER_EVENT_SCHEMA="$(sley_source_task loom.worker event_schema)"
WORKER_SESSION_SCHEMA="$(sley_source_task loom.worker session_schema)"
WORKER_RUNTIME_DIGEST="$(sley_source_task loom.worker runtime_digest)"
WORKER_OPERATIONS_JSON="$(sley_source_list_task_json loom.worker operations)"
WORKER_LIFECYCLE_STATES_JSON="$(sley_source_list_task_json loom.worker lifecycle_states)"
WORKER_REQUEST_STATES_JSON="$(sley_source_list_task_json loom.worker request_states)"
WORKER_FAILURE_CLASSES_JSON="$(sley_source_list_task_json loom.worker failure_classes)"
WORKER_CONTRACT_VERSIONS_JSON="$(sley_source_list_task_json loom.worker contract_versions)"
WORKER_ISOLATION_CLASS="$(sley_source_task loom.worker isolation_class)"
WORKER_CACHE_POLICY="$(sley_source_task loom.worker cache_policy)"
TEST_MANIFEST_SCHEMA="$(sley_source_task loom.testing manifest_schema)"
TEST_REPORT_SCHEMA="$(sley_source_task loom.testing report_schema)"
TEST_REVIEW_PACKET_SCHEMA="$(sley_source_task loom.testing review_packet_schema)"
TEST_DISCOVERY_CONVENTION="$(sley_source_task loom.testing discovery_convention)"
TEST_CASE_KINDS_JSON="$(sley_source_list_task_json loom.testing case_kinds)"
TEST_COVERAGE_DIMENSIONS_JSON="$(sley_source_list_task_json loom.testing coverage_dimensions)"
TEST_COVERAGE_EVIDENCE_STATES_JSON="$(sley_source_list_task_json loom.testing coverage_evidence_states)"
TEST_PROPERTY_STRATEGY="$(sley_source_task loom.testing property_strategy)"
TEST_DIFFERENTIAL_COMPARISON="$(sley_source_task loom.testing differential_comparison)"
TEST_AUTHORITY_MODE="$(sley_source_task loom.testing authority_mode)"
TEST_REVIEW_BINDING_AUTHORITY_MODE="$(sley_source_task loom.testing review_binding_authority_mode)"
VALIDATION_REPORT_SCHEMA="$(sley_source_task loom.validation report_schema)"
VALIDATION_PROFILES_JSON="$(sley_source_list_task_json loom.validation profiles | jq 'map(split("|") | {id:.[0], executor:.[1], release_authority:(.[2] == "true")})')"
VALIDATION_CHECK_REGISTRY_JSON="$(sley_source_list_task_json loom.validation check_registry | jq 'map(split("|") | {id:.[0], subsystem:.[1]})')"
VALIDATION_QUICK_CHECKS_JSON="$(sley_source_list_task_json loom.validation quick_checks)"
VALIDATION_CORE_CHECKS_JSON="$(sley_source_list_task_json loom.validation core_checks)"
VALIDATION_RELEASE_CHECKS_JSON="$(sley_source_list_task_json loom.validation release_checks)"
VALIDATION_SKIP_REASONS_JSON="$(sley_source_list_task_json loom.validation skip_reasons)"
VALIDATION_AUTHORITY_MODE="$(sley_source_task loom.validation authority_mode)"
VALIDATION_RESULT_CACHE_POLICY="$(sley_source_task loom.validation result_cache_policy)"
OPERATIONAL_PLAN_SCHEMA="$(sley_source_task loom.operational agent_workflow_plan_schema)"
OPERATIONAL_COMPARISON_SCHEMA="$(sley_source_task loom.operational agent_workflow_comparison_schema)"
OPERATIONAL_WORKFLOW_ID="$(sley_source_task loom.operational agent_workflow_id)"
OPERATIONAL_MODES_JSON="$(sley_source_list_task_json loom.operational agent_workflow_modes)"
OPERATIONAL_EQUAL_CONTROLS_JSON="$(sley_source_list_task_json loom.operational agent_workflow_equal_controls)"
OPERATIONAL_REQUIRED_METRICS_JSON="$(sley_source_list_task_json loom.operational agent_workflow_required_metrics)"
OPERATIONAL_AUTHORITY_MODE="$(sley_source_task loom.operational agent_workflow_authority_mode)"
OPERATIONAL_DECISIONS_JSON="$(sley_source_list_task_json loom.operational agent_workflow_decisions)"
OPERATIONAL_ORACLE_STEPS_JSON="$(sley_source_list_task_json loom.operational agent_workflow_oracle_steps)"
OPERATIONAL_CONTROLLED_DIFFERENCE_JSON="$(sley_source_list_task_json loom.operational agent_workflow_controlled_difference)"
OPERATIONAL_REGISTRY_SCHEMA="$(sley_source_task loom.operational evidence_registry_schema)"
OPERATIONAL_REGISTRY_RELEASE="$(sley_source_task loom.operational evidence_registry_release)"
OPERATIONAL_REGISTRY_AUTHORITY_MODE="$(sley_source_task loom.operational evidence_registry_authority_mode)"
OPERATIONAL_REGISTRY_ENTRY_IDS_JSON="$(sley_source_list_task_json loom.operational evidence_registry_entry_ids)"
OPERATIONAL_REGISTRY_DECISION_RATIONALE_JSON="$(sley_source_list_task_json loom.operational evidence_registry_decision_rationale)"
TRANSACTION_LIFECYCLE_STATES_JSON="$(sley_source_list_task_json loom.transaction lifecycle_states)"
TRANSACTION_FAILURE_STATES_JSON="$(sley_source_list_task_json loom.transaction failure_states)"
TRANSACTION_TRANSITION_PLAN_JSON="$(sley_source_list_task_json loom.transaction allowed_transition_plan | jq 'map(split("|") | {from:.[0], to:.[1]})')"
TRANSACTION_BASE_MODES_JSON="$(sley_source_list_task_json loom.transaction base_modes)"
TRANSACTION_MUTATION_BOUNDARIES_JSON="$(sley_source_list_task_json loom.transaction mutation_boundaries)"
TRANSACTION_PREVIEW_OPERATION_PLAN_JSON="$(sley_source_list_task_json loom.transaction preview_operation_plan | jq 'map(split("|") | {op:.[0], executor:.[1]})')"
TRANSACTION_PLAN_VALIDATION_PROFILES_JSON="$(sley_source_list_task_json loom.transaction plan_validation_profiles)"
TRANSACTION_CANDIDATE_WORKSPACE_KINDS_JSON="$(sley_source_list_task_json loom.transaction candidate_workspace_kinds)"
TRANSACTION_PREVIEW_REVIEW_REQUIREMENT_KINDS_JSON="$(sley_source_list_task_json loom.transaction preview_review_requirement_kinds)"
TRANSACTION_APPLY_RECORD_PHASES_JSON="$(sley_source_list_task_json loom.transaction apply_record_phases)"
TRANSACTION_APPLY_REJECTION_REASONS_JSON="$(sley_source_list_task_json loom.transaction apply_rejection_reasons)"
TRANSACTION_ATOMIC_APPLY_STRATEGIES_JSON="$(sley_source_list_task_json loom.transaction atomic_apply_strategies)"
TRANSACTION_ATOMIC_APPLY_TOPOLOGIES_JSON="$(sley_source_list_task_json loom.transaction atomic_apply_topologies)"
TRANSACTION_ATOMIC_ROLLBACK_STRATEGIES_JSON="$(sley_source_list_task_json loom.transaction atomic_rollback_strategies)"
TRANSACTION_APPLY_CANCELLATION_POINTS_JSON="$(sley_source_list_task_json loom.transaction apply_cancellation_points)"
TRANSACTION_APPLY_VERIFICATION_PROFILES_JSON="$(sley_source_list_task_json loom.transaction apply_verification_profiles)"
TRANSACTION_APPLY_VERIFICATION_REQUIRED_CHECKS_JSON="$(sley_source_list_task_json loom.transaction apply_verification_required_checks)"
TRANSACTION_APPLY_METADATA_POLICY_JSON="$(sley_source_list_task_json loom.transaction apply_metadata_policy | jq 'map(split("|") | {field:.[0], value:.[1]})')"
TRANSACTION_INTERNAL_MUTATION_BOUNDARIES_JSON="$(sley_source_list_task_json loom.transaction internal_mutation_boundaries)"
TRANSACTION_REVIEW_TERMINAL_OUTCOMES_JSON="$(sley_source_list_task_json loom.transaction review_terminal_outcomes)"
TRANSACTION_REVIEW_ARTIFACT_NAMES_JSON="$(sley_source_list_task_json loom.transaction review_artifact_names)"
TRANSACTION_REVIEW_NO_MUTATION_BOUNDARIES_JSON="$(sley_source_list_task_json loom.transaction review_no_mutation_boundaries)"
AUTHORITY_LOCAL_GRANT_RESOURCE_SCOPE_KINDS_JSON="$(sley_source_list_task_json loom.authority local_grant_resource_scope_kinds)"
AUTHORITY_LOCAL_GRANT_OPERATOR_FIELDS_JSON="$(sley_source_list_task_json loom.authority local_grant_operator_fields)"
AUTHORITY_LOCAL_GRANT_ISSUER_SOURCES_JSON="$(sley_source_list_task_json loom.authority local_grant_issuer_sources)"
AUTHORITY_LOCAL_GRANT_REPLAY_POLICY="$(sley_source_task loom.authority local_grant_replay_policy)"
AUTHORITY_LOCAL_GRANT_REPLAY_ENFORCEMENT="$(sley_source_task loom.authority local_grant_replay_enforcement)"
AUTHORITY_LOCAL_GRANT_REVOCATION_STATES_JSON="$(sley_source_list_task_json loom.authority local_grant_revocation_states)"
AUTHORITY_LOCAL_GRANT_ISSUABLE_REVOCATION_STATES_JSON="$(sley_source_list_task_json loom.authority local_grant_issuable_revocation_states)"
AUTHORITY_LOCAL_GRANT_REVOCATION_SOURCE_KINDS_JSON="$(sley_source_list_task_json loom.authority local_grant_revocation_source_kinds)"
AUTHORITY_LOCAL_GRANT_PROOF_KINDS_JSON="$(sley_source_list_task_json loom.authority local_grant_proof_kinds)"
AUTHORITY_LOCAL_GRANT_PROOF_STATUSES_JSON="$(sley_source_list_task_json loom.authority local_grant_proof_statuses)"
AUTHORITY_LOCAL_GRANT_MAX_APPLY_ATTEMPTS="$(sley_source_task loom.authority local_grant_max_apply_attempts)"
AUTHORITY_APPLY_AUTHORIZATION_INTERNAL_SCOPE_KINDS_JSON="$(sley_source_list_task_json loom.authority apply_authorization_internal_scope_kinds)"
AUTHORITY_APPLY_REPLAY_ENFORCEMENT="$(sley_source_task loom.authority apply_replay_enforcement)"
AUTHORITY_LOCAL_REVOCATION_RECORD_STATES_JSON="$(sley_source_list_task_json loom.authority local_revocation_record_states)"
AUTHORITY_LOCAL_REVOCATION_RECORD_SOURCES_JSON="$(sley_source_list_task_json loom.authority local_revocation_record_sources)"
require_source_list TRANSACTION_LIFECYCLE_STATES_JSON "$TRANSACTION_LIFECYCLE_STATES_JSON"
require_source_list ADAPTER_LIFECYCLE_STATES_JSON "$ADAPTER_LIFECYCLE_STATES_JSON"
require_source_list ADAPTER_REPLAY_EVENT_PHASES_JSON "$ADAPTER_REPLAY_EVENT_PHASES_JSON"
require_source_list ADAPTER_FAILURE_CODES_JSON "$ADAPTER_FAILURE_CODES_JSON"
require_source_list ADAPTER_SEED_FAMILIES_JSON "$ADAPTER_SEED_FAMILIES_JSON"
require_source_list ADAPTER_SUPPORTED_EFFECTS_JSON "$ADAPTER_SUPPORTED_EFFECTS_JSON"
require_source_list WORKER_OPERATIONS_JSON "$WORKER_OPERATIONS_JSON"
require_source_list WORKER_LIFECYCLE_STATES_JSON "$WORKER_LIFECYCLE_STATES_JSON"
require_source_list WORKER_REQUEST_STATES_JSON "$WORKER_REQUEST_STATES_JSON"
require_source_list WORKER_FAILURE_CLASSES_JSON "$WORKER_FAILURE_CLASSES_JSON"
require_source_list WORKER_CONTRACT_VERSIONS_JSON "$WORKER_CONTRACT_VERSIONS_JSON"
require_source_text ADAPTER_AUTHORITY_MODE "$ADAPTER_AUTHORITY_MODE"
require_source_text ADAPTER_REPLAY_POLICY "$ADAPTER_REPLAY_POLICY"
require_source_text TEST_MANIFEST_SCHEMA "$TEST_MANIFEST_SCHEMA"
require_source_text TEST_REPORT_SCHEMA "$TEST_REPORT_SCHEMA"
require_source_text TEST_REVIEW_PACKET_SCHEMA "$TEST_REVIEW_PACKET_SCHEMA"
require_source_text TEST_DISCOVERY_CONVENTION "$TEST_DISCOVERY_CONVENTION"
require_source_list TEST_CASE_KINDS_JSON "$TEST_CASE_KINDS_JSON"
require_source_list TEST_COVERAGE_DIMENSIONS_JSON "$TEST_COVERAGE_DIMENSIONS_JSON"
require_source_list TEST_COVERAGE_EVIDENCE_STATES_JSON "$TEST_COVERAGE_EVIDENCE_STATES_JSON"
require_source_text TEST_PROPERTY_STRATEGY "$TEST_PROPERTY_STRATEGY"
require_source_text TEST_DIFFERENTIAL_COMPARISON "$TEST_DIFFERENTIAL_COMPARISON"
require_source_text TEST_AUTHORITY_MODE "$TEST_AUTHORITY_MODE"
require_source_text TEST_REVIEW_BINDING_AUTHORITY_MODE "$TEST_REVIEW_BINDING_AUTHORITY_MODE"
require_source_text VALIDATION_REPORT_SCHEMA "$VALIDATION_REPORT_SCHEMA"
require_source_list VALIDATION_PROFILES_JSON "$VALIDATION_PROFILES_JSON"
require_source_list VALIDATION_CHECK_REGISTRY_JSON "$VALIDATION_CHECK_REGISTRY_JSON"
require_source_list VALIDATION_QUICK_CHECKS_JSON "$VALIDATION_QUICK_CHECKS_JSON"
require_source_list VALIDATION_CORE_CHECKS_JSON "$VALIDATION_CORE_CHECKS_JSON"
require_source_list VALIDATION_RELEASE_CHECKS_JSON "$VALIDATION_RELEASE_CHECKS_JSON"
require_source_list VALIDATION_SKIP_REASONS_JSON "$VALIDATION_SKIP_REASONS_JSON"
require_source_text VALIDATION_AUTHORITY_MODE "$VALIDATION_AUTHORITY_MODE"
require_source_text VALIDATION_RESULT_CACHE_POLICY "$VALIDATION_RESULT_CACHE_POLICY"
require_source_text OPERATIONAL_PLAN_SCHEMA "$OPERATIONAL_PLAN_SCHEMA"
require_source_text OPERATIONAL_COMPARISON_SCHEMA "$OPERATIONAL_COMPARISON_SCHEMA"
require_source_text OPERATIONAL_WORKFLOW_ID "$OPERATIONAL_WORKFLOW_ID"
require_source_list OPERATIONAL_MODES_JSON "$OPERATIONAL_MODES_JSON"
require_source_list OPERATIONAL_EQUAL_CONTROLS_JSON "$OPERATIONAL_EQUAL_CONTROLS_JSON"
require_source_list OPERATIONAL_REQUIRED_METRICS_JSON "$OPERATIONAL_REQUIRED_METRICS_JSON"
require_source_text OPERATIONAL_AUTHORITY_MODE "$OPERATIONAL_AUTHORITY_MODE"
require_source_list OPERATIONAL_DECISIONS_JSON "$OPERATIONAL_DECISIONS_JSON"
require_source_list OPERATIONAL_ORACLE_STEPS_JSON "$OPERATIONAL_ORACLE_STEPS_JSON"
require_source_list OPERATIONAL_CONTROLLED_DIFFERENCE_JSON "$OPERATIONAL_CONTROLLED_DIFFERENCE_JSON"
require_source_text OPERATIONAL_REGISTRY_SCHEMA "$OPERATIONAL_REGISTRY_SCHEMA"
require_source_text OPERATIONAL_REGISTRY_RELEASE "$OPERATIONAL_REGISTRY_RELEASE"
require_source_text OPERATIONAL_REGISTRY_AUTHORITY_MODE "$OPERATIONAL_REGISTRY_AUTHORITY_MODE"
require_source_list OPERATIONAL_REGISTRY_ENTRY_IDS_JSON "$OPERATIONAL_REGISTRY_ENTRY_IDS_JSON"
require_source_list OPERATIONAL_REGISTRY_DECISION_RATIONALE_JSON "$OPERATIONAL_REGISTRY_DECISION_RATIONALE_JSON"
require_source_text ADAPTER_KIND "$ADAPTER_KIND"
require_source_text ADAPTER_LIVE_EXECUTION "$ADAPTER_LIVE_EXECUTION"
require_source_list TRANSACTION_FAILURE_STATES_JSON "$TRANSACTION_FAILURE_STATES_JSON"
require_source_list TRANSACTION_TRANSITION_PLAN_JSON "$TRANSACTION_TRANSITION_PLAN_JSON"
require_source_list TRANSACTION_BASE_MODES_JSON "$TRANSACTION_BASE_MODES_JSON"
require_source_list TRANSACTION_MUTATION_BOUNDARIES_JSON "$TRANSACTION_MUTATION_BOUNDARIES_JSON"
require_source_list TRANSACTION_PREVIEW_OPERATION_PLAN_JSON "$TRANSACTION_PREVIEW_OPERATION_PLAN_JSON"
require_source_list TRANSACTION_PLAN_VALIDATION_PROFILES_JSON "$TRANSACTION_PLAN_VALIDATION_PROFILES_JSON"
require_source_list TRANSACTION_CANDIDATE_WORKSPACE_KINDS_JSON "$TRANSACTION_CANDIDATE_WORKSPACE_KINDS_JSON"
require_source_list TRANSACTION_PREVIEW_REVIEW_REQUIREMENT_KINDS_JSON "$TRANSACTION_PREVIEW_REVIEW_REQUIREMENT_KINDS_JSON"
require_source_list TRANSACTION_APPLY_RECORD_PHASES_JSON "$TRANSACTION_APPLY_RECORD_PHASES_JSON"
require_source_list TRANSACTION_APPLY_REJECTION_REASONS_JSON "$TRANSACTION_APPLY_REJECTION_REASONS_JSON"
require_source_list TRANSACTION_ATOMIC_APPLY_STRATEGIES_JSON "$TRANSACTION_ATOMIC_APPLY_STRATEGIES_JSON"
require_source_list TRANSACTION_ATOMIC_APPLY_TOPOLOGIES_JSON "$TRANSACTION_ATOMIC_APPLY_TOPOLOGIES_JSON"
require_source_list TRANSACTION_ATOMIC_ROLLBACK_STRATEGIES_JSON "$TRANSACTION_ATOMIC_ROLLBACK_STRATEGIES_JSON"
require_source_list TRANSACTION_APPLY_CANCELLATION_POINTS_JSON "$TRANSACTION_APPLY_CANCELLATION_POINTS_JSON"
require_source_list TRANSACTION_APPLY_VERIFICATION_PROFILES_JSON "$TRANSACTION_APPLY_VERIFICATION_PROFILES_JSON"
require_source_list TRANSACTION_APPLY_VERIFICATION_REQUIRED_CHECKS_JSON "$TRANSACTION_APPLY_VERIFICATION_REQUIRED_CHECKS_JSON"
require_source_list TRANSACTION_APPLY_METADATA_POLICY_JSON "$TRANSACTION_APPLY_METADATA_POLICY_JSON"
require_source_list TRANSACTION_INTERNAL_MUTATION_BOUNDARIES_JSON "$TRANSACTION_INTERNAL_MUTATION_BOUNDARIES_JSON"
require_source_list AUTHORITY_LOCAL_GRANT_RESOURCE_SCOPE_KINDS_JSON "$AUTHORITY_LOCAL_GRANT_RESOURCE_SCOPE_KINDS_JSON"
require_source_list AUTHORITY_LOCAL_GRANT_OPERATOR_FIELDS_JSON "$AUTHORITY_LOCAL_GRANT_OPERATOR_FIELDS_JSON"
require_source_list AUTHORITY_LOCAL_GRANT_ISSUER_SOURCES_JSON "$AUTHORITY_LOCAL_GRANT_ISSUER_SOURCES_JSON"
require_source_text AUTHORITY_LOCAL_GRANT_REPLAY_POLICY "$AUTHORITY_LOCAL_GRANT_REPLAY_POLICY"
require_source_text AUTHORITY_LOCAL_GRANT_REPLAY_ENFORCEMENT "$AUTHORITY_LOCAL_GRANT_REPLAY_ENFORCEMENT"
require_source_list AUTHORITY_LOCAL_GRANT_REVOCATION_STATES_JSON "$AUTHORITY_LOCAL_GRANT_REVOCATION_STATES_JSON"
require_source_list AUTHORITY_LOCAL_GRANT_ISSUABLE_REVOCATION_STATES_JSON "$AUTHORITY_LOCAL_GRANT_ISSUABLE_REVOCATION_STATES_JSON"
require_source_list AUTHORITY_LOCAL_GRANT_REVOCATION_SOURCE_KINDS_JSON "$AUTHORITY_LOCAL_GRANT_REVOCATION_SOURCE_KINDS_JSON"
require_source_list AUTHORITY_LOCAL_GRANT_PROOF_KINDS_JSON "$AUTHORITY_LOCAL_GRANT_PROOF_KINDS_JSON"
require_source_list AUTHORITY_LOCAL_GRANT_PROOF_STATUSES_JSON "$AUTHORITY_LOCAL_GRANT_PROOF_STATUSES_JSON"
require_source_text AUTHORITY_LOCAL_GRANT_MAX_APPLY_ATTEMPTS "$AUTHORITY_LOCAL_GRANT_MAX_APPLY_ATTEMPTS"
require_source_list AUTHORITY_APPLY_AUTHORIZATION_INTERNAL_SCOPE_KINDS_JSON "$AUTHORITY_APPLY_AUTHORIZATION_INTERNAL_SCOPE_KINDS_JSON"
require_source_text AUTHORITY_APPLY_REPLAY_ENFORCEMENT "$AUTHORITY_APPLY_REPLAY_ENFORCEMENT"
require_source_list AUTHORITY_LOCAL_REVOCATION_RECORD_STATES_JSON "$AUTHORITY_LOCAL_REVOCATION_RECORD_STATES_JSON"
require_source_list AUTHORITY_LOCAL_REVOCATION_RECORD_SOURCES_JSON "$AUTHORITY_LOCAL_REVOCATION_RECORD_SOURCES_JSON"

SCHEMA_AST_PROGRAM="$(sley_source_task loom.reports ast_program_schema)"
AST_PROGRAM_REPORT_FIELDS_JSON="$(sley_source_list_task_json loom.reports ast_program_report_fields)"
AST_PROGRAM_REPORT_BUILDER_JSON="$(sley_source_list_task_json loom.reports ast_program_report_builder)"
SCHEMA_AST_NODE="$(sley_source_task loom.reports ast_node_schema)"
AST_NODE_REPORT_FIELDS_JSON="$(sley_source_list_task_json loom.reports ast_node_report_fields)"
AST_NODE_REPORT_BUILDER_JSON="$(sley_source_list_task_json loom.reports ast_node_report_builder)"
SCHEMA_DIAGNOSTICS="$(sley_source_task loom.reports diagnostics_schema)"
DIAGNOSTICS_REPORT_FIELDS_JSON="$(sley_source_list_task_json loom.reports diagnostics_report_fields)"
DIAGNOSTICS_REPORT_BUILDER_JSON="$(sley_source_list_task_json loom.reports diagnostics_report_builder)"
SCHEMA_EXPLAIN="$(sley_source_task loom.reports explain_schema)"
EXPLAIN_REPORT_FIELDS_JSON="$(sley_source_list_task_json loom.reports explain_report_fields)"
EXPLAIN_REPORT_BUILDER_JSON="$(sley_source_list_task_json loom.reports explain_report_builder)"
CHECKER_DIAGNOSTIC_EXPLAIN_CATALOG_JSON="$(sley_source_list_task_json loom.checker diagnostic_explain_catalog | jq 'map(split("|") | {
  diagnostic_id:.[0],
  title:.[1],
  category:.[2],
  context:.[3],
  message_template:.[4],
  checked_spelling:.[5],
  repair_kind:.[6],
  repair_path:.[7],
  spec_ref:.[8],
  accepted_example:.[9],
  rejected_example:.[10],
  command:.[11]
})')"
DIAG_EXPLAIN_UNKNOWN="$(sley_source_task loom.checker explain_unknown_diagnostic_id)"
EXPLAIN_UNKNOWN_DIAGNOSTIC_MESSAGE_TEMPLATE="$(eval_checker_message_template explain_unknown_diagnostic_message diagnostic_id)"
SCHEMA_QUERY="$(sley_source_task loom.reports query_schema)"
QUERY_REPORT_FIELDS_JSON="$(sley_source_list_task_json loom.reports query_report_fields)"
QUERY_REPORT_BUILDER_JSON="$(sley_source_list_task_json loom.reports query_report_builder)"
SCHEMA_SYMBOL_GRAPH="$(sley_source_task loom.reports symbol_graph_schema)"
SYMBOL_GRAPH_REPORT_FIELDS_JSON="$(sley_source_list_task_json loom.reports symbol_graph_report_fields)"
SYMBOL_GRAPH_REPORT_BUILDER_JSON="$(sley_source_list_task_json loom.reports symbol_graph_report_builder)"
SCHEMA_LINT="$(sley_source_task loom.reports lint_schema)"
LINT_REPORT_FIELDS_JSON="$(sley_source_list_task_json loom.reports lint_report_fields)"
LINT_REPORT_BUILDER_JSON="$(sley_source_list_task_json loom.reports lint_report_builder)"
SCHEMA_DOCTOR="$(sley_source_task loom.reports doctor_schema)"
DOCTOR_REPORT_FIELDS_JSON="$(sley_source_list_task_json loom.reports doctor_report_fields)"
DOCTOR_REPORT_BUILDER_JSON="$(sley_source_list_task_json loom.reports doctor_report_builder)"
SCHEMA_RUN="$(sley_source_task loom.reports run_schema)"
RUN_REPORT_FIELDS_JSON="$(sley_source_list_task_json loom.reports run_report_fields)"
RUN_REPORT_BUILDER_JSON="$(sley_source_list_task_json loom.reports run_report_builder)"
SCHEMA_VERIFY="$(sley_source_task loom.reports verify_schema)"
VERIFY_REPORT_FIELDS_JSON="$(sley_source_list_task_json loom.reports verify_report_fields)"
VERIFY_REPORT_BUILDER_JSON="$(sley_source_list_task_json loom.reports verify_report_builder)"
SCHEMA_GRAFT_OUTCOME="$(sley_source_task loom.reports graft_outcome_schema)"
GRAFT_OUTCOME_REPORT_FIELDS_JSON="$(sley_source_list_task_json loom.reports graft_outcome_report_fields)"
GRAFT_OUTCOME_REPORT_BUILDER_JSON="$(sley_source_list_task_json loom.reports graft_outcome_report_builder)"
SCHEMA_TRANSACTION_INSPECT="$(sley_source_task loom.reports transaction_inspect_schema)"
TRANSACTION_INSPECT_REPORT_FIELDS_JSON="$(sley_source_list_task_json loom.reports transaction_inspect_report_fields)"
TRANSACTION_INSPECT_REPORT_BUILDER_JSON="$(sley_source_list_task_json loom.reports transaction_inspect_report_builder)"
SCHEMA_CHANGE_PLAN="$(sley_source_task loom.reports change_plan_schema)"
CHANGE_PLAN_REPORT_FIELDS_JSON="$(sley_source_list_task_json loom.reports change_plan_report_fields)"
CHANGE_PLAN_REPORT_BUILDER_JSON="$(sley_source_list_task_json loom.reports change_plan_report_builder)"
SCHEMA_CHANGE_PREVIEW="$(sley_source_task loom.reports change_preview_schema)"
CHANGE_PREVIEW_REPORT_FIELDS_JSON="$(sley_source_list_task_json loom.reports change_preview_report_fields)"
CHANGE_PREVIEW_REPORT_BUILDER_JSON="$(sley_source_list_task_json loom.reports change_preview_report_builder)"
SCHEMA_CHANGE_APPROVAL_REQUEST="$(sley_source_task loom.reports change_approval_request_schema)"
CHANGE_APPROVAL_REQUEST_REPORT_FIELDS_JSON="$(sley_source_list_task_json loom.reports change_approval_request_report_fields)"
CHANGE_APPROVAL_REQUEST_REPORT_BUILDER_JSON="$(sley_source_list_task_json loom.reports change_approval_request_report_builder)"
SCHEMA_CHANGE_GRANT="$(sley_source_task loom.reports change_grant_schema)"
CHANGE_GRANT_REPORT_FIELDS_JSON="$(sley_source_list_task_json loom.reports change_grant_report_fields)"
CHANGE_GRANT_REPORT_BUILDER_JSON="$(sley_source_list_task_json loom.reports change_grant_report_builder)"
SCHEMA_CHANGE_APPLY_AUTHORIZATION="$(sley_source_task loom.reports change_apply_authorization_schema)"
CHANGE_APPLY_AUTHORIZATION_REPORT_FIELDS_JSON="$(sley_source_list_task_json loom.reports change_apply_authorization_report_fields)"
CHANGE_APPLY_AUTHORIZATION_REPORT_BUILDER_JSON="$(sley_source_list_task_json loom.reports change_apply_authorization_report_builder)"
SCHEMA_CHANGE_REVOCATION_RECORD="$(sley_source_task loom.reports change_revocation_record_schema)"
CHANGE_REVOCATION_RECORD_REPORT_FIELDS_JSON="$(sley_source_list_task_json loom.reports change_revocation_record_report_fields)"
CHANGE_REVOCATION_RECORD_REPORT_BUILDER_JSON="$(sley_source_list_task_json loom.reports change_revocation_record_report_builder)"
SCHEMA_CHANGE_RECOVERY="$(sley_source_task loom.reports change_recovery_schema)"
CHANGE_RECOVERY_REPORT_FIELDS_JSON="$(sley_source_list_task_json loom.reports change_recovery_report_fields)"
CHANGE_RECOVERY_REPORT_BUILDER_JSON="$(sley_source_list_task_json loom.reports change_recovery_report_builder)"
SCHEMA_CHANGE_APPLY="$(sley_source_task loom.reports change_apply_schema)"
CHANGE_APPLY_REPORT_FIELDS_JSON="$(sley_source_list_task_json loom.reports change_apply_report_fields)"
CHANGE_APPLY_REPORT_BUILDER_JSON="$(sley_source_list_task_json loom.reports change_apply_report_builder)"
SCHEMA_CHANGE_ROLLBACK="$(sley_source_task loom.reports change_rollback_schema)"
CHANGE_ROLLBACK_REPORT_FIELDS_JSON="$(sley_source_list_task_json loom.reports change_rollback_report_fields)"
CHANGE_ROLLBACK_REPORT_BUILDER_JSON="$(sley_source_list_task_json loom.reports change_rollback_report_builder)"
SCHEMA_CHANGE_REVIEW="$(sley_source_task loom.reports change_review_schema)"
CHANGE_REVIEW_REPORT_FIELDS_JSON="$(sley_source_list_task_json loom.reports change_review_report_fields)"
CHANGE_REVIEW_REPORT_BUILDER_JSON="$(sley_source_list_task_json loom.reports change_review_report_builder)"
SCHEMA_CHANGE_TRANSACTION_SEAL="$(sley_source_task loom.reports change_transaction_seal_schema)"
CHANGE_TRANSACTION_SEAL_REPORT_FIELDS_JSON="$(sley_source_list_task_json loom.reports change_transaction_seal_report_fields)"
CHANGE_TRANSACTION_SEAL_REPORT_BUILDER_JSON="$(sley_source_list_task_json loom.reports change_transaction_seal_report_builder)"
SCHEMA_TEST_REPORT="$(sley_source_task loom.reports test_schema)"
TEST_REPORT_FIELDS_JSON="$(sley_source_list_task_json loom.reports test_report_fields)"
TEST_REPORT_BUILDER_JSON="$(sley_source_list_task_json loom.reports test_report_builder)"
SCHEMA_REVIEW_PACKET="$(sley_source_task loom.reports review_packet_schema)"
REVIEW_PACKET_FIELDS_JSON="$(sley_source_list_task_json loom.reports review_packet_report_fields)"
REVIEW_PACKET_BUILDER_JSON="$(sley_source_list_task_json loom.reports review_packet_report_builder)"
SCHEMA_VALIDATION_REPORT="$(sley_source_task loom.reports validation_schema)"
VALIDATION_REPORT_FIELDS_JSON="$(sley_source_list_task_json loom.reports validation_report_fields)"
VALIDATION_REPORT_BUILDER_JSON="$(sley_source_list_task_json loom.reports validation_report_builder)"
SCHEMA_CLAIM_VERIFY="$(sley_source_task loom.reports claim_verify_schema)"
CLAIM_VERIFY_REPORT_FIELDS_JSON="$(sley_source_list_task_json loom.reports claim_verify_report_fields)"
CLAIM_VERIFY_REPORT_BUILDER_JSON="$(sley_source_list_task_json loom.reports claim_verify_report_builder)"
SCHEMA_CONFORMANCE_REPORT="$(sley_source_task loom.reports conformance_report_schema)"
CONFORMANCE_REPORT_FIELDS_JSON="$(sley_source_list_task_json loom.reports conformance_report_fields)"
CONFORMANCE_REPORT_BUILDER_JSON="$(sley_source_list_task_json loom.reports conformance_report_builder)"
SCHEMA_CONFORMANCE_COVERAGE="$(sley_source_task loom.reports conformance_coverage_schema)"
CONFORMANCE_COVERAGE_FIELDS_JSON="$(sley_source_list_task_json loom.reports conformance_coverage_fields)"
CONFORMANCE_COVERAGE_BUILDER_JSON="$(sley_source_list_task_json loom.reports conformance_coverage_builder)"
SCHEMA_CI_REPORT="$(sley_source_task loom.reports ci_report_schema)"
CI_REPORT_FIELDS_JSON="$(sley_source_list_task_json loom.reports ci_report_fields)"
CI_REPORT_BUILDER_JSON="$(sley_source_list_task_json loom.reports ci_report_builder)"
SCHEMA_CONTRACT_INVENTORY="$(sley_source_task loom.reports contract_inventory_schema)"
CONTRACT_INVENTORY_FIELDS_JSON="$(sley_source_list_task_json loom.reports contract_inventory_fields)"
CONTRACT_INVENTORY_BUILDER_JSON="$(sley_source_list_task_json loom.reports contract_inventory_builder)"
SCHEMA_CONTRACT_VALIDATE="$(sley_source_task loom.reports contract_validate_schema)"
CONTRACT_VALIDATE_FIELDS_JSON="$(sley_source_list_task_json loom.reports contract_validate_fields)"
CONTRACT_VALIDATE_BUILDER_JSON="$(sley_source_list_task_json loom.reports contract_validate_builder)"
SCHEMA_CONTRACT_FIXTURE_CHECK="$(sley_source_task loom.reports contract_fixture_check_schema)"
CONTRACT_FIXTURE_CHECK_FIELDS_JSON="$(sley_source_list_task_json loom.reports contract_fixture_check_fields)"
CONTRACT_FIXTURE_CHECK_BUILDER_JSON="$(sley_source_list_task_json loom.reports contract_fixture_check_builder)"
SCHEMA_DEPLOY_REPORT="$(sley_source_task loom.reports deploy_report_schema)"
DEPLOY_REPORT_FIELDS_JSON="$(sley_source_list_task_json loom.reports deploy_report_fields)"
DEPLOY_REPORT_BUILDER_JSON="$(sley_source_list_task_json loom.reports deploy_report_builder)"
SCHEMA_DEPLOY_ARTIFACTS="$(sley_source_task loom.reports deploy_artifacts_schema)"
SCHEMA_DEPLOY_ARTIFACT_CHECK="$(sley_source_task loom.reports deploy_artifact_check_schema)"
DEPLOY_ARTIFACT_CHECK_FIELDS_JSON="$(sley_source_list_task_json loom.reports deploy_artifact_check_fields)"
DEPLOY_ARTIFACT_CHECK_BUILDER_JSON="$(sley_source_list_task_json loom.reports deploy_artifact_check_builder)"
SCHEMA_MIGRATE="$(sley_source_task loom.reports migrate_schema)"
MIGRATE_REPORT_FIELDS_JSON="$(sley_source_list_task_json loom.reports migrate_report_fields)"
MIGRATE_REPORT_BUILDER_JSON="$(sley_source_list_task_json loom.reports migrate_report_builder)"
SCHEMA_DOCGEN="$(sley_source_task loom.reports docgen_schema)"
DOCGEN_REPORT_FIELDS_JSON="$(sley_source_list_task_json loom.reports docgen_report_fields)"
DOCGEN_REPORT_BUILDER_JSON="$(sley_source_list_task_json loom.reports docgen_report_builder)"
SCHEMA_WORKBENCH="$(sley_source_task loom.reports workbench_schema)"
WORKBENCH_REPORT_FIELDS_JSON="$(sley_source_list_task_json loom.reports workbench_report_fields)"
WORKBENCH_REPORT_BUILDER_JSON="$(sley_source_list_task_json loom.reports workbench_report_builder)"
SCHEMA_SANDBOX="$(sley_source_task loom.reports sandbox_schema)"
SANDBOX_REPORT_FIELDS_JSON="$(sley_source_list_task_json loom.reports sandbox_report_fields)"
SANDBOX_REPORT_BUILDER_JSON="$(sley_source_list_task_json loom.reports sandbox_report_builder)"
SCHEMA_ADAPTER="$(sley_source_task loom.reports adapter_schema)"
ADAPTER_REPORT_FIELDS_JSON="$(sley_source_list_task_json loom.reports adapter_report_fields)"
ADAPTER_REPORT_BUILDER_JSON="$(sley_source_list_task_json loom.reports adapter_report_builder)"
SCHEMA_WORKER_RESPONSE="$(sley_source_task loom.reports worker_response_schema)"
WORKER_RESPONSE_REPORT_FIELDS_JSON="$(sley_source_list_task_json loom.reports worker_response_report_fields)"
WORKER_RESPONSE_REPORT_BUILDER_JSON="$(sley_source_list_task_json loom.reports worker_response_report_builder)"
SCHEMA_SHADOW="$(sley_source_task loom.reports shadow_schema)"
SCHEMA_AGENT_BENCH="$(sley_source_task loom.reports agent_bench_schema)"
AGENT_BENCH_REPORT_FIELDS_JSON="$(sley_source_list_task_json loom.reports agent_bench_report_fields)"
AGENT_BENCH_REPORT_BUILDER_JSON="$(sley_source_list_task_json loom.reports agent_bench_report_builder)"
SCHEMA_ZJX_TOOL="$(sley_source_task loom.reports zjx_tool_schema)"
ZJX_TOOL_REPORT_FIELDS_JSON="$(sley_source_list_task_json loom.reports zjx_tool_report_fields)"
ZJX_TOOL_REPORT_BUILDER_JSON="$(sley_source_list_task_json loom.reports zjx_tool_report_builder)"
SCHEMA_GRAPH_DIFF="$(sley_source_task loom.reports graph_diff_schema)"
GRAPH_DIFF_REPORT_FIELDS_JSON="$(sley_source_list_task_json loom.reports graph_diff_report_fields)"
GRAPH_DIFF_REPORT_BUILDER_JSON="$(sley_source_list_task_json loom.reports graph_diff_report_builder)"
SCHEMA_SELF_HOSTING="$(sley_source_task loom.reports self_hosting_status_schema)"
SELF_HOSTING_REPORT_SOURCE_ROOT="$(sley_source_task loom.reports self_hosting_source_root)"
SELF_HOSTING_STATUS_FIELDS_JSON="$(sley_source_list_task_json loom.reports self_hosting_status_fields)"
SELF_HOSTING_STATUS_BUILDER_JSON="$(sley_source_list_task_json loom.reports self_hosting_status_builder)"

for _sley_source_text_name in \
  SCHEMA_AST_PROGRAM SCHEMA_AST_NODE SCHEMA_DIAGNOSTICS SCHEMA_QUERY \
  SCHEMA_SYMBOL_GRAPH SCHEMA_LINT SCHEMA_DOCTOR SCHEMA_RUN SCHEMA_VERIFY \
  SCHEMA_GRAFT_OUTCOME SCHEMA_CLAIM_VERIFY SCHEMA_CONFORMANCE_REPORT \
  SCHEMA_CONFORMANCE_COVERAGE SCHEMA_CI_REPORT SCHEMA_CONTRACT_INVENTORY \
  SCHEMA_CONTRACT_VALIDATE SCHEMA_CONTRACT_FIXTURE_CHECK SCHEMA_DEPLOY_REPORT \
  SCHEMA_DEPLOY_ARTIFACTS SCHEMA_DEPLOY_ARTIFACT_CHECK SCHEMA_MIGRATE \
  SCHEMA_DOCGEN SCHEMA_WORKBENCH SCHEMA_SANDBOX SCHEMA_ADAPTER SCHEMA_WORKER_RESPONSE SCHEMA_SHADOW \
  SCHEMA_AGENT_BENCH SCHEMA_ZJX_TOOL SCHEMA_GRAPH_DIFF SCHEMA_SELF_HOSTING \
  SELF_HOSTING_REPORT_SOURCE_ROOT; do
  require_source_text "$_sley_source_text_name" "${!_sley_source_text_name}"
done

for _sley_source_list_name in \
  DEFAULT_RULES_JSON \
  REPORT_BUILDER_VALUE_TYPE_PLAN_JSON REPORT_BUILDER_REGISTRY_JSON \
  AST_PROGRAM_REPORT_FIELDS_JSON AST_PROGRAM_REPORT_BUILDER_JSON \
  AST_NODE_REPORT_FIELDS_JSON AST_NODE_REPORT_BUILDER_JSON \
  DIAGNOSTICS_REPORT_FIELDS_JSON DIAGNOSTICS_REPORT_BUILDER_JSON \
  QUERY_REPORT_FIELDS_JSON QUERY_REPORT_BUILDER_JSON \
  SYMBOL_GRAPH_REPORT_FIELDS_JSON SYMBOL_GRAPH_REPORT_BUILDER_JSON \
  LINT_REPORT_FIELDS_JSON LINT_REPORT_BUILDER_JSON \
  DOCTOR_REPORT_FIELDS_JSON DOCTOR_REPORT_BUILDER_JSON \
  RUN_REPORT_FIELDS_JSON RUN_REPORT_BUILDER_JSON \
  VERIFY_REPORT_FIELDS_JSON VERIFY_REPORT_BUILDER_JSON \
  GRAFT_OUTCOME_REPORT_FIELDS_JSON GRAFT_OUTCOME_REPORT_BUILDER_JSON \
  CLAIM_VERIFY_REPORT_FIELDS_JSON CLAIM_VERIFY_REPORT_BUILDER_JSON \
  CONFORMANCE_REPORT_FIELDS_JSON CONFORMANCE_REPORT_BUILDER_JSON \
  CONFORMANCE_COVERAGE_FIELDS_JSON CONFORMANCE_COVERAGE_BUILDER_JSON \
  CI_REPORT_FIELDS_JSON CI_REPORT_BUILDER_JSON \
  CONTRACT_INVENTORY_FIELDS_JSON CONTRACT_INVENTORY_BUILDER_JSON \
  CONTRACT_VALIDATE_FIELDS_JSON CONTRACT_VALIDATE_BUILDER_JSON \
  CONTRACT_FIXTURE_CHECK_FIELDS_JSON CONTRACT_FIXTURE_CHECK_BUILDER_JSON \
  DEPLOY_REPORT_FIELDS_JSON DEPLOY_REPORT_BUILDER_JSON \
  DEPLOY_ARTIFACT_CHECK_FIELDS_JSON DEPLOY_ARTIFACT_CHECK_BUILDER_JSON \
  MIGRATE_REPORT_FIELDS_JSON MIGRATE_REPORT_BUILDER_JSON \
  DOCGEN_REPORT_FIELDS_JSON DOCGEN_REPORT_BUILDER_JSON \
  WORKBENCH_REPORT_FIELDS_JSON WORKBENCH_REPORT_BUILDER_JSON \
  SANDBOX_REPORT_FIELDS_JSON SANDBOX_REPORT_BUILDER_JSON \
  ADAPTER_REPORT_FIELDS_JSON ADAPTER_REPORT_BUILDER_JSON \
  WORKER_RESPONSE_REPORT_FIELDS_JSON WORKER_RESPONSE_REPORT_BUILDER_JSON \
  AGENT_BENCH_REPORT_FIELDS_JSON AGENT_BENCH_REPORT_BUILDER_JSON \
  ZJX_TOOL_REPORT_FIELDS_JSON ZJX_TOOL_REPORT_BUILDER_JSON \
  GRAPH_DIFF_REPORT_FIELDS_JSON GRAPH_DIFF_REPORT_BUILDER_JSON \
  SELF_HOSTING_STATUS_FIELDS_JSON SELF_HOSTING_STATUS_BUILDER_JSON; do
  require_source_list "$_sley_source_list_name" "${!_sley_source_list_name}"
done
unset _sley_source_text_name _sley_source_list_name


PARSER_CALL_EXPRESSION_PREFIX="$(sley_source_task loom.parser call_expression_prefix)"
require_source_text PARSER_CALL_EXPRESSION_PREFIX "$PARSER_CALL_EXPRESSION_PREFIX"

CHECKER_DIAGNOSTIC_EXECUTORS_JSON="$(sley_source_list_task_json loom.checker diagnostic_executors)"
CHECKER_DIAGNOSTIC_PASS_DESCRIPTORS_JSON="$(sley_source_list_task_json loom.checker diagnostic_pass_plan | jq 'map(split("|") as $parts | {name:$parts[0], executor_id:($parts[1] | tonumber), executor:$parts[2], diagnostic_id_task:$parts[3], message_task:$parts[4], message_arg:$parts[5], subject:$parts[6], node_scope:$parts[7]})')"
DIAG_UNKNOWN_IDENTIFIER="$(sley_source_task loom.checker unknown_identifier_id)"
DIAG_UNSUPPORTED_RAW_EXPRESSION="$(sley_source_task loom.checker unsupported_raw_expression_id)"
DIAG_CONTROL_FLOW_EXPRESSION_BOUNDARY="$(sley_source_task loom.checker control_flow_expression_boundary_id)"
DIAG_MODULE_CONTROL_FLOW_NOT_ALLOWED="$(sley_source_task loom.checker module_control_flow_not_allowed_id)"
DIAG_INLINE_TASK_PARAMETER_PLACEMENT="$(sley_source_task loom.checker inline_task_parameter_placement_id)"
DIAG_UNKNOWN_TYPE="$(sley_source_task loom.checker unknown_type_id)"
DIAG_UNKNOWN_TASK="$(sley_source_task loom.checker unknown_task_id)"
DIAG_CALL_ARITY_MISMATCH="$(sley_source_task loom.checker call_arity_mismatch_id)"
DIAG_CALL_ARGUMENT_TYPE_MISMATCH="$(sley_source_task loom.checker call_argument_type_mismatch_id)"
DIAG_TYPE_MISMATCH="$(sley_source_task loom.checker type_mismatch_id)"
DIAG_RETURN_TYPE_MISMATCH="$(sley_source_task loom.checker return_type_mismatch_id)"
DIAG_DUPLICATE_TAKE="$(sley_source_task loom.checker duplicate_take_id)"
DIAG_DUPLICATE_MAP_KEY="$(sley_source_task loom.checker duplicate_map_key_id)"
DIAG_DUPLICATE_FIELD="$(sley_source_task loom.checker duplicate_field_id)"
DIAG_DUPLICATE_RECORD_LITERAL_FIELD="$(sley_source_task loom.checker duplicate_record_literal_field_id)"
DIAG_RECORD_FIELD_MISSING="$(sley_source_task loom.checker record_field_missing_id)"
DIAG_RECORD_FIELD_UNKNOWN="$(sley_source_task loom.checker record_field_unknown_id)"
DIAG_RECORD_FIELD_TYPE_MISMATCH="$(sley_source_task loom.checker record_field_type_mismatch_id)"
DIAG_UNKNOWN_RECORD_FIELD="$(sley_source_task loom.checker unknown_record_field_id)"
DIAG_RECORD_LITERAL_NON_RECORD_TYPE="$(sley_source_task loom.checker record_literal_non_record_type_id)"
DIAG_LIST_ELEMENT_TYPE_MISMATCH="$(sley_source_task loom.checker list_element_type_mismatch_id)"
DIAG_INDEX_NOT_INT="$(sley_source_task loom.checker index_not_int_id)"
DIAG_INDEX_KEY_TYPE_MISMATCH="$(sley_source_task loom.checker index_key_type_mismatch_id)"
DIAG_MAP_KEY_TYPE_MISMATCH="$(sley_source_task loom.checker map_key_type_mismatch_id)"
DIAG_MAP_VALUE_TYPE_MISMATCH="$(sley_source_task loom.checker map_value_type_mismatch_id)"
DIAG_DUPLICATE_EFFECT="$(sley_source_task loom.checker duplicate_effect_id)"
DIAG_DUPLICATE_TYPE="$(sley_source_task loom.checker duplicate_type_id)"
DIAG_DUPLICATE_TASK="$(sley_source_task loom.checker duplicate_task_id)"
DIAG_UNKNOWN_EFFECT="$(sley_source_task loom.checker unknown_effect_id)"
DIAG_GATE_TAKE_TYPE_MISMATCH="$(sley_source_task loom.checker gate_take_type_mismatch_id)"
DIAG_GATE_EFFECT_UNDECLARED="$(sley_source_task loom.checker gate_effect_undeclared_id)"
DIAG_EFFECT_UNAUTHORIZED="$(sley_source_task loom.checker effect_unauthorized_id)"
DIAG_MISSING_RETURN="$(sley_source_task loom.checker missing_return_id)"
DIAG_QUESTION_REQUIRES_RESULT="$(sley_source_task loom.checker question_requires_result_id)"
CHECK_OK_STATUS="$(sley_source_task loom.checker ok_status)"
CHECK_ERROR_STATUS="$(sley_source_task loom.checker error_status)"
UNKNOWN_IDENTIFIER_MESSAGE_PREFIX="$(sley_source_task loom.checker unknown_identifier_message_prefix)"
UNKNOWN_IDENTIFIER_MESSAGE_SUFFIX="$(sley_source_task loom.checker unknown_identifier_message_suffix)"
UNKNOWN_IDENTIFIER_MESSAGE_TEMPLATE="$(eval_checker_message_template unknown_identifier_message identifier_name)"
UNSUPPORTED_RAW_EXPRESSION_MESSAGE_TEMPLATE="$(eval_checker_message_template unsupported_raw_expression_message expression_text)"
CONTROL_FLOW_EXPRESSION_BOUNDARY_MESSAGE_TEMPLATE="$(eval_checker_message_template control_flow_expression_boundary_message expression_text)"
MODULE_CONTROL_FLOW_NOT_ALLOWED_MESSAGE_TEMPLATE="$(eval_checker_message_template module_control_flow_not_allowed_message statement_text)"
INLINE_TASK_PARAMETER_PLACEMENT_MESSAGE_TEMPLATE="$(eval_checker_message_template inline_task_parameter_placement_message task_name)"
UNKNOWN_TYPE_MESSAGE_PREFIX="$(sley_source_task loom.checker unknown_type_message_prefix)"
UNKNOWN_TYPE_MESSAGE_SUFFIX="$(sley_source_task loom.checker unknown_type_message_suffix)"
UNKNOWN_TYPE_MESSAGE_TEMPLATE="$(eval_checker_message_template unknown_type_message type_name)"
UNKNOWN_TASK_MESSAGE_PREFIX="$(sley_source_task loom.checker unknown_task_message_prefix)"
UNKNOWN_TASK_MESSAGE_SUFFIX="$(sley_source_task loom.checker unknown_task_message_suffix)"
UNKNOWN_TASK_MESSAGE_TEMPLATE="$(eval_checker_message_template unknown_task_message task_name)"
CALL_ARITY_MISMATCH_MESSAGE_PREFIX="$(sley_source_task loom.checker call_arity_mismatch_message_prefix)"
CALL_ARITY_MISMATCH_MESSAGE_SUFFIX="$(sley_source_task loom.checker call_arity_mismatch_message_suffix)"
CALL_ARITY_MISMATCH_MESSAGE_TEMPLATE="$(eval_checker_message_template call_arity_mismatch_message task_name)"
CALL_ARGUMENT_TYPE_MISMATCH_MESSAGE_PREFIX="$(sley_source_task loom.checker call_argument_type_mismatch_message_prefix)"
CALL_ARGUMENT_TYPE_MISMATCH_MESSAGE_SUFFIX="$(sley_source_task loom.checker call_argument_type_mismatch_message_suffix)"
CALL_ARGUMENT_TYPE_MISMATCH_MESSAGE_TEMPLATE="$(eval_checker_message_template call_argument_type_mismatch_message task_name)"
TYPE_MISMATCH_MESSAGE_PREFIX="$(sley_source_task loom.checker type_mismatch_message_prefix)"
TYPE_MISMATCH_MESSAGE_SUFFIX="$(sley_source_task loom.checker type_mismatch_message_suffix)"
TYPE_MISMATCH_MESSAGE_TEMPLATE="$(eval_checker_message_template type_mismatch_message binding_name)"
RETURN_TYPE_MISMATCH_MESSAGE_PREFIX="$(sley_source_task loom.checker return_type_mismatch_message_prefix)"
RETURN_TYPE_MISMATCH_MESSAGE_SUFFIX="$(sley_source_task loom.checker return_type_mismatch_message_suffix)"
RETURN_TYPE_MISMATCH_MESSAGE_TEMPLATE="$(eval_checker_message_template return_type_mismatch_message task_name)"
DUPLICATE_TAKE_MESSAGE_PREFIX="$(sley_source_task loom.checker duplicate_take_message_prefix)"
DUPLICATE_TAKE_MESSAGE_SUFFIX="$(sley_source_task loom.checker duplicate_take_message_suffix)"
DUPLICATE_TAKE_MESSAGE_TEMPLATE="$(eval_checker_message_template duplicate_take_message take_name)"
DUPLICATE_MAP_KEY_MESSAGE_PREFIX="$(sley_source_task loom.checker duplicate_map_key_message_prefix)"
DUPLICATE_MAP_KEY_MESSAGE_SUFFIX="$(sley_source_task loom.checker duplicate_map_key_message_suffix)"
DUPLICATE_MAP_KEY_MESSAGE_TEMPLATE="$(eval_checker_message_template duplicate_map_key_message key_name)"
DUPLICATE_FIELD_MESSAGE_PREFIX="$(sley_source_task loom.checker duplicate_field_message_prefix)"
DUPLICATE_FIELD_MESSAGE_SUFFIX="$(sley_source_task loom.checker duplicate_field_message_suffix)"
DUPLICATE_FIELD_MESSAGE_TEMPLATE="$(eval_checker_message_template duplicate_field_message field_name)"
DUPLICATE_RECORD_LITERAL_FIELD_MESSAGE_PREFIX="$(sley_source_task loom.checker duplicate_record_literal_field_message_prefix)"
DUPLICATE_RECORD_LITERAL_FIELD_MESSAGE_SUFFIX="$(sley_source_task loom.checker duplicate_record_literal_field_message_suffix)"
DUPLICATE_RECORD_LITERAL_FIELD_MESSAGE_TEMPLATE="$(eval_checker_message_template duplicate_record_literal_field_message field_name)"
RECORD_FIELD_MISSING_MESSAGE_PREFIX="$(sley_source_task loom.checker record_field_missing_message_prefix)"
RECORD_FIELD_MISSING_MESSAGE_SUFFIX="$(sley_source_task loom.checker record_field_missing_message_suffix)"
RECORD_FIELD_MISSING_MESSAGE_TEMPLATE="$(eval_checker_message_template record_field_missing_message field_name)"
RECORD_FIELD_UNKNOWN_MESSAGE_PREFIX="$(sley_source_task loom.checker record_field_unknown_message_prefix)"
RECORD_FIELD_UNKNOWN_MESSAGE_SUFFIX="$(sley_source_task loom.checker record_field_unknown_message_suffix)"
RECORD_FIELD_UNKNOWN_MESSAGE_TEMPLATE="$(eval_checker_message_template record_field_unknown_message field_name)"
RECORD_FIELD_TYPE_MISMATCH_MESSAGE_PREFIX="$(sley_source_task loom.checker record_field_type_mismatch_message_prefix)"
RECORD_FIELD_TYPE_MISMATCH_MESSAGE_SUFFIX="$(sley_source_task loom.checker record_field_type_mismatch_message_suffix)"
RECORD_FIELD_TYPE_MISMATCH_MESSAGE_TEMPLATE="$(eval_checker_message_template record_field_type_mismatch_message field_name)"
UNKNOWN_RECORD_FIELD_MESSAGE_PREFIX="$(sley_source_task loom.checker unknown_record_field_message_prefix)"
UNKNOWN_RECORD_FIELD_MESSAGE_SUFFIX="$(sley_source_task loom.checker unknown_record_field_message_suffix)"
UNKNOWN_RECORD_FIELD_MESSAGE_TEMPLATE="$(eval_checker_message_template unknown_record_field_message field_name)"
RECORD_LITERAL_NON_RECORD_TYPE_MESSAGE_PREFIX="$(sley_source_task loom.checker record_literal_non_record_type_message_prefix)"
RECORD_LITERAL_NON_RECORD_TYPE_MESSAGE_SUFFIX="$(sley_source_task loom.checker record_literal_non_record_type_message_suffix)"
RECORD_LITERAL_NON_RECORD_TYPE_MESSAGE_TEMPLATE="$(eval_checker_message_template record_literal_non_record_type_message type_name)"
LIST_ELEMENT_TYPE_MISMATCH_MESSAGE_PREFIX="$(sley_source_task loom.checker list_element_type_mismatch_message_prefix)"
LIST_ELEMENT_TYPE_MISMATCH_MESSAGE_SUFFIX="$(sley_source_task loom.checker list_element_type_mismatch_message_suffix)"
LIST_ELEMENT_TYPE_MISMATCH_MESSAGE_TEMPLATE="$(eval_checker_message_template list_element_type_mismatch_message actual_type)"
INDEX_NOT_INT_MESSAGE_PREFIX="$(sley_source_task loom.checker index_not_int_message_prefix)"
INDEX_NOT_INT_MESSAGE_SUFFIX="$(sley_source_task loom.checker index_not_int_message_suffix)"
INDEX_NOT_INT_MESSAGE_TEMPLATE="$(eval_checker_message_template index_not_int_message actual_type)"
INDEX_KEY_TYPE_MISMATCH_MESSAGE_PREFIX="$(sley_source_task loom.checker index_key_type_mismatch_message_prefix)"
INDEX_KEY_TYPE_MISMATCH_MESSAGE_SUFFIX="$(sley_source_task loom.checker index_key_type_mismatch_message_suffix)"
INDEX_KEY_TYPE_MISMATCH_MESSAGE_TEMPLATE="$(eval_checker_message_template index_key_type_mismatch_message actual_type)"
MAP_KEY_TYPE_MISMATCH_MESSAGE_PREFIX="$(sley_source_task loom.checker map_key_type_mismatch_message_prefix)"
MAP_KEY_TYPE_MISMATCH_MESSAGE_SUFFIX="$(sley_source_task loom.checker map_key_type_mismatch_message_suffix)"
MAP_KEY_TYPE_MISMATCH_MESSAGE_TEMPLATE="$(eval_checker_message_template map_key_type_mismatch_message actual_type)"
MAP_VALUE_TYPE_MISMATCH_MESSAGE_PREFIX="$(sley_source_task loom.checker map_value_type_mismatch_message_prefix)"
MAP_VALUE_TYPE_MISMATCH_MESSAGE_SUFFIX="$(sley_source_task loom.checker map_value_type_mismatch_message_suffix)"
MAP_VALUE_TYPE_MISMATCH_MESSAGE_TEMPLATE="$(eval_checker_message_template map_value_type_mismatch_message actual_type)"
DUPLICATE_EFFECT_MESSAGE_PREFIX="$(sley_source_task loom.checker duplicate_effect_message_prefix)"
DUPLICATE_EFFECT_MESSAGE_SUFFIX="$(sley_source_task loom.checker duplicate_effect_message_suffix)"
DUPLICATE_EFFECT_MESSAGE_TEMPLATE="$(eval_checker_message_template duplicate_effect_message effect_name)"
DUPLICATE_TYPE_MESSAGE_PREFIX="$(sley_source_task loom.checker duplicate_type_message_prefix)"
DUPLICATE_TYPE_MESSAGE_SUFFIX="$(sley_source_task loom.checker duplicate_type_message_suffix)"
DUPLICATE_TYPE_MESSAGE_TEMPLATE="$(eval_checker_message_template duplicate_type_message type_name)"
DUPLICATE_TASK_MESSAGE_PREFIX="$(sley_source_task loom.checker duplicate_task_message_prefix)"
DUPLICATE_TASK_MESSAGE_SUFFIX="$(sley_source_task loom.checker duplicate_task_message_suffix)"
DUPLICATE_TASK_MESSAGE_TEMPLATE="$(eval_checker_message_template duplicate_task_message task_name)"
UNKNOWN_EFFECT_MESSAGE_PREFIX="$(sley_source_task loom.checker unknown_effect_message_prefix)"
UNKNOWN_EFFECT_MESSAGE_SUFFIX="$(sley_source_task loom.checker unknown_effect_message_suffix)"
UNKNOWN_EFFECT_MESSAGE_TEMPLATE="$(eval_checker_message_template unknown_effect_message effect_name)"
GATE_TAKE_TYPE_MISMATCH_MESSAGE_PREFIX="$(sley_source_task loom.checker gate_take_type_mismatch_message_prefix)"
GATE_TAKE_TYPE_MISMATCH_MESSAGE_SUFFIX="$(sley_source_task loom.checker gate_take_type_mismatch_message_suffix)"
GATE_TAKE_TYPE_MISMATCH_MESSAGE_TEMPLATE="$(eval_checker_message_template gate_take_type_mismatch_message actual_type)"
GATE_EFFECT_UNDECLARED_MESSAGE_PREFIX="$(sley_source_task loom.checker gate_effect_undeclared_message_prefix)"
GATE_EFFECT_UNDECLARED_MESSAGE_SUFFIX="$(sley_source_task loom.checker gate_effect_undeclared_message_suffix)"
GATE_EFFECT_UNDECLARED_MESSAGE_TEMPLATE="$(eval_checker_message_template gate_effect_undeclared_message effect_name)"
EFFECT_UNAUTHORIZED_MESSAGE_PREFIX="$(sley_source_task loom.checker effect_unauthorized_message_prefix)"
EFFECT_UNAUTHORIZED_MESSAGE_SUFFIX="$(sley_source_task loom.checker effect_unauthorized_message_suffix)"
EFFECT_UNAUTHORIZED_MESSAGE_TEMPLATE="$(eval_checker_message_template effect_unauthorized_message effect_name)"
MISSING_RETURN_MESSAGE_PREFIX="$(sley_source_task loom.checker missing_return_message_prefix)"
MISSING_RETURN_MESSAGE_SUFFIX="$(sley_source_task loom.checker missing_return_message_suffix)"
MISSING_RETURN_MESSAGE_TEMPLATE="$(eval_checker_message_template missing_return_message task_name)"
QUESTION_REQUIRES_RESULT_MESSAGE_PREFIX="$(sley_source_task loom.checker question_requires_result_message_prefix)"
QUESTION_REQUIRES_RESULT_MESSAGE_SUFFIX="$(sley_source_task loom.checker question_requires_result_message_suffix)"
QUESTION_REQUIRES_RESULT_MESSAGE_TEMPLATE="$(eval_checker_message_template question_requires_result_message task_name)"
CHECKER_IDENTIFIER_INPUTS_JSON="$(sley_source_list_task_json loom.checker identifier_resolution_inputs)"
CHECKER_BUILTIN_EFFECTS_JSON="$(sley_source_list_task_json loom.checker builtin_effects)"
CHECKER_EFFECT_ALIASES_JSON="$(sley_source_list_task_json loom.checker effect_authorization_aliases)"
CHECKER_HOST_EFFECT_NEEDLES_JSON="$(sley_source_list_task_json loom.checker host_effect_needles)"
CHECKER_FOREIGN_CONDITIONAL_EXPRESSION_PATTERN="$(sley_source_task loom.checker foreign_conditional_expression_pattern)"
CHECKER_MODULE_CONTROL_FLOW_PREFIXES_JSON="$(sley_source_list_task_json loom.checker module_control_flow_prefixes)"
CHECKER_INLINE_TASK_PARAMETER_PATTERN="$(sley_source_task loom.checker inline_task_parameter_pattern)"
CHECKER_TASK_DECLARATION_PATTERN="$(sley_source_task loom.checker task_declaration_pattern)"
CHECKER_HOST_ADAPTER_RAW_RECOVERY_PATTERN="$(sley_source_task loom.checker host_adapter_raw_recovery_pattern)"
CHECKER_SUPPORTED_RAW_RECOVERY_PATTERNS_JSON="$(sley_source_list_task_json loom.checker supported_raw_recovery_patterns)"
CHECKER_ASSUME_QUALIFIED_TASK_CALLS_KNOWN="$(sley_source_task loom.checker assume_qualified_task_calls_known)"
case "$CHECKER_ASSUME_QUALIFIED_TASK_CALLS_KNOWN" in true|false) ;; *) exit 1 ;; esac
CHECKER_DECLARE_OR_IMPORT_TASK_HINT_KIND="$(sley_source_task loom.checker declare_or_import_task_hint_kind)"
CHECKER_INSPECT_RETURN_TYPE_HINT_KIND="$(sley_source_task loom.checker inspect_return_type_hint_kind)"
CHECKER_INSERT_RETURN_HINT_KIND="$(sley_source_task loom.checker insert_return_hint_kind)"
CHECKER_REPLACE_TASK_BODY_HINT_KIND="$(sley_source_task loom.checker replace_task_body_hint_kind)"
CHECKER_REPLACE_EXPRESSION_HINT_KIND="$(sley_source_task loom.checker replace_expression_hint_kind)"
CHECKER_REWRITE_SUPPORTED_EXPRESSION_HINT_KIND="$(sley_source_task loom.checker rewrite_supported_expression_hint_kind)"
CHECKER_REWRITE_SUPPORTED_EXPRESSION_HINT="$(sley_source_task loom.checker rewrite_supported_expression_hint)"
CHECKER_REWRITE_CONDITIONAL_HINT_KIND="$(sley_source_task loom.checker rewrite_conditional_hint_kind)"
CHECKER_REWRITE_CONDITIONAL_HINT="$(sley_source_task loom.checker rewrite_conditional_hint)"
CHECKER_MOVE_CONTROL_FLOW_INTO_TASK_HINT_KIND="$(sley_source_task loom.checker move_control_flow_into_task_hint_kind)"
CHECKER_MOVE_CONTROL_FLOW_INTO_TASK_HINT="$(sley_source_task loom.checker move_control_flow_into_task_hint)"
CHECKER_MOVE_TAKE_INTO_TASK_BODY_HINT_KIND="$(sley_source_task loom.checker move_take_into_task_body_hint_kind)"
CHECKER_MOVE_TAKE_INTO_TASK_BODY_HINT="$(sley_source_task loom.checker move_take_into_task_body_hint)"
CHECKER_INT_TYPE="$(sley_source_task loom.checker int_type_name)"
CHECKER_TEXT_TYPE="$(sley_source_task loom.checker text_type_name)"
CHECKER_BOOL_TYPE="$(sley_source_task loom.checker bool_type_name)"
CHECKER_UNIT_TYPE="$(sley_source_task loom.checker unit_type_name)"
CHECKER_RESULT_TYPE="$(sley_source_task loom.checker result_type_name)"
CHECKER_ERROR_TYPE="$(sley_source_task loom.checker error_type_name)"
CHECKER_GATE_TYPE="$(sley_source_task loom.checker gate_type_name)"
CHECKER_LIST_TYPE="$(sley_source_task loom.checker list_type_name)"
CHECKER_MAP_TYPE="$(sley_source_task loom.checker map_type_name)"
LINT_EMPTY_FOR="$(sley_source_task loom.lint empty_for_statement_id)"
LINT_EMPTY_WHILE="$(sley_source_task loom.lint empty_while_statement_id)"
LINT_EMPTY_FORGE="$(sley_source_task loom.lint empty_forge_statement_id)"
LINT_EMPTY_IF="$(sley_source_task loom.lint empty_if_statement_id)"
LINT_EMPTY_ELSE="$(sley_source_task loom.lint empty_else_statement_id)"
LINT_OK_STATUS="$(sley_source_task loom.lint ok_status)"
LINT_FINDINGS_STATUS="$(sley_source_task loom.lint findings_status)"
LINT_EMPTY_FOR_RULE="$(sley_source_task loom.lint empty_for_statement_rule)"
LINT_EMPTY_WHILE_RULE="$(sley_source_task loom.lint empty_while_statement_rule)"
LINT_EMPTY_FORGE_RULE="$(sley_source_task loom.lint empty_forge_statement_rule)"
LINT_EMPTY_IF_RULE="$(sley_source_task loom.lint empty_if_statement_rule)"
LINT_EMPTY_ELSE_RULE="$(sley_source_task loom.lint empty_else_statement_rule)"
LINT_EMPTY_FOR_MESSAGE="$(sley_source_task loom.lint empty_for_statement_message)"
LINT_EMPTY_WHILE_MESSAGE="$(sley_source_task loom.lint empty_while_statement_message)"
LINT_EMPTY_FORGE_MESSAGE="$(sley_source_task loom.lint empty_forge_statement_message)"
LINT_EMPTY_IF_MESSAGE="$(sley_source_task loom.lint empty_if_statement_message)"
LINT_EMPTY_ELSE_MESSAGE="$(sley_source_task loom.lint empty_else_statement_message)"
LINT_EMPTY_FOR_HINT="$(sley_source_task loom.lint empty_for_statement_hint)"
LINT_EMPTY_WHILE_HINT="$(sley_source_task loom.lint empty_while_statement_hint)"
LINT_EMPTY_FORGE_HINT="$(sley_source_task loom.lint empty_forge_statement_hint)"
LINT_EMPTY_IF_HINT="$(sley_source_task loom.lint empty_if_statement_hint)"
LINT_EMPTY_ELSE_HINT="$(sley_source_task loom.lint empty_else_statement_hint)"

RUNTIME_HELLO_VALUE="$(sley_source_task loom.runtime hello_value)"
RUNTIME_DEFAULT_PROFILE="$(sley_source_task loom.runtime default_profile_text)"
RUNTIME_DEFAULT_MODEL_PLAN="$(sley_source_task loom.runtime default_model_plan_text)"
RUNTIME_DEFAULT_DEPLOY_RESULT="$(sley_source_task loom.runtime default_deploy_result_text)"
RUNTIME_DEFAULT_RAW_VALUE="$(sley_source_task loom.runtime default_raw_value)"
RUNTIME_DEFAULT_FILE_WRITE_TEXT="$(sley_source_task loom.runtime default_file_write_text)"
RUNTIME_DEFAULT_DATABASE_WRITE_TEXT="$(sley_source_task loom.runtime default_database_write_text)"
RUNTIME_DEFAULT_AGENT_DATA_WRITE_TEXT="$(sley_source_task loom.runtime default_agent_data_write_text)"
RUNTIME_DEFAULT_DATABASE_READ_TEXT="$(sley_source_task loom.runtime default_database_read_text)"
RUNTIME_DEFAULT_DATABASE_TABLE="$(sley_source_task loom.runtime default_database_table)"
RUNTIME_DEFAULT_SHELL_TEXT="$(sley_source_task loom.runtime default_shell_text)"
RUNTIME_DEFAULT_SECRET_TEXT="$(sley_source_task loom.runtime default_secret_text)"
RUNTIME_DEFAULT_SPEND_AUTHORIZATION_TEXT="$(sley_source_task loom.runtime default_spend_authorization_text)"
RUNTIME_PASSED_STATUS="$(sley_source_task loom.runtime passed_status)"
RUNTIME_FAILED_STATUS="$(sley_source_task loom.runtime failed_status)"
RUNTIME_SKIPPED_STATUS="$(sley_source_task loom.runtime skipped_status)"
RUNTIME_DISPATCH_ORDER_JSON="$(sley_source_list_task_json loom.runtime runtime_dispatch_order)"
RUNTIME_DISPATCH_PLAN_JSON="$(sley_source_list_task_json loom.runtime runtime_dispatch_plan | jq 'map(split("|") | {evaluator_id:(.[0] | tonumber), candidate:.[1], evaluator_function:.[2]})')"
RUNTIME_INT_BINARY_OPERATOR_PLAN_JSON="$(sley_source_list_task_json loom.runtime int_binary_operator_plan | jq 'map(split("|") | {op:.[0], task:.[1]})')"
RUNTIME_BOOL_BINARY_OPERATOR_PLAN_JSON="$(sley_source_list_task_json loom.runtime bool_binary_operator_plan | jq 'map(split("=>") | {op:.[0], task:.[1]})')"
RUNTIME_BOOL_UNARY_OPERATOR_PLAN_JSON="$(sley_source_list_task_json loom.runtime bool_unary_operator_plan | jq 'map(split("=>") | {op:.[0], task:.[1]})')"
RUNTIME_EQUALITY_OPERATOR_PLAN_JSON="$(sley_source_list_task_json loom.runtime equality_operator_plan | jq 'map(split("|") | {value_kind:.[0], op:.[1], task:.[2]})')"
RUNTIME_IF_VALUE_PLAN_JSON="$(sley_source_list_task_json loom.runtime if_value_plan | jq 'map(split("|") | {value_kind:.[0], task:.[1]})')"
RUNTIME_INT_COMPARISON_OPERATOR_PLAN_JSON="$(sley_source_list_task_json loom.runtime int_comparison_operator_plan | jq 'map(split("|") | {op:.[0], task:.[1], order:.[2], negate:.[3]})')"
RUNTIME_CAPABILITY_REQUIRED_ID="$(sley_source_task loom.runtime capability_required_diagnostic_id)"
RUNTIME_CAPABILITY_SCOPE_DENIED_ID="$(sley_source_task loom.runtime capability_scope_denied_diagnostic_id)"
RUNTIME_TAKE_REQUIRED_ID="$(sley_source_task loom.runtime main_take_required_diagnostic_id)"
RUNTIME_DISPATCH_MISS_ID="$(sley_source_task loom.runtime dispatch_miss_diagnostic_id)"
RUNTIME_CAPABILITY_REQUIRED_MESSAGE_PREFIX="$(sley_source_task loom.runtime capability_required_message_prefix)"
RUNTIME_CAPABILITY_SCOPE_DENIED_MESSAGE_MIDDLE="$(sley_source_task loom.runtime capability_scope_denied_message_middle)"
RUNTIME_CAPABILITY_SCOPE_DENIED_MESSAGE_SUFFIX="$(sley_source_task loom.runtime capability_scope_denied_message_suffix)"
RUNTIME_TAKE_REQUIRED_MESSAGE_PREFIX="$(sley_source_task loom.runtime main_take_required_message_prefix)"
RUNTIME_TAKE_REQUIRED_MESSAGE_SUFFIX="$(sley_source_task loom.runtime main_take_required_message_suffix)"
RUNTIME_DISPATCH_MISS_MESSAGE_PREFIX="$(sley_source_task loom.runtime dispatch_miss_message_prefix)"
RUNTIME_DISPATCH_MISS_MESSAGE_SUFFIX="$(sley_source_task loom.runtime dispatch_miss_message_suffix)"
RUNTIME_INT_VALUE_KIND="$(sley_source_task loom.runtime int_value_kind)"
RUNTIME_TEXT_VALUE_KIND="$(sley_source_task loom.runtime text_value_kind)"
RUNTIME_BOOL_VALUE_KIND="$(sley_source_task loom.runtime bool_value_kind)"
RUNTIME_RAW_VALUE_KIND="$(sley_source_task loom.runtime raw_value_kind)"
RUNTIME_UNIT_VALUE_KIND="$(sley_source_task loom.runtime unit_value_kind)"
RUNTIME_OK_TEXT_VALUE_KIND="$(sley_source_task loom.runtime ok_text_value_kind)"
RUNTIME_OK_INT_VALUE_KIND="$(sley_source_task loom.runtime ok_int_value_kind)"
RUNTIME_ERR_TEXT_VALUE_KIND="$(sley_source_task loom.runtime err_text_value_kind)"
RUNTIME_SELF_HOSTED_TARGETS_JSON="$(sley_source_list_task_json loom.runtime self_hosted_target_aliases)"
RUNTIME_GENERIC_UNSUPPORTED_BLOCK_PREFIXES_JSON="$(sley_source_list_task_json loom.runtime generic_runtime_unsupported_block_prefixes)"
RUNTIME_GENERIC_RETURN_PLAN_JSON="$(sley_source_list_task_json loom.runtime generic_runtime_return_type_plan | jq 'map(split("|") | {return_base:.[0], result_status:.[1], result_type:.[2], value_kind:.[3], value_task:.[4]})')"
RUNTIME_AGENT_DEPLOY_SUFFIX="$(sley_source_task loom.runtime agent_deploy_target_suffix)"
RUNTIME_PROJECT_READY_CALL_PROBE="$(eval_runtime_text_probe_task project_ready_call_probe || sley_source_task loom.runtime project_ready_call_probe)"
RUNTIME_PROJECT_READY_BINDING_PROBE="$(eval_runtime_text_probe_task project_ready_binding_probe || sley_source_task loom.runtime project_ready_binding_probe)"
PARSER_INT_PATTERN="$(sley_source_task loom.parser int_literal_pattern)"
PARSER_IDENTIFIER_PATTERN="$(sley_source_task loom.parser identifier_pattern)"
PARSER_MULTILINE_RECORD_LITERAL_OPEN_PATTERN="$(sley_source_task loom.parser multiline_record_literal_open_pattern)"
PARSER_INT_KIND="$(sley_source_task loom.parser int_literal_kind)"
PARSER_STRING_KIND="$(sley_source_task loom.parser string_literal_kind)"
PARSER_BOOL_KIND="$(sley_source_task loom.parser bool_literal_kind)"
PARSER_IDENTIFIER_KIND="$(sley_source_task loom.parser identifier_kind)"
PARSER_LIST_KIND="$(sley_source_task loom.parser list_literal_kind)"
PARSER_RAW_KIND="$(sley_source_task loom.parser raw_expr_kind)"
PARSER_CLASSIFIERS_JSON="$(eval_parser_expression_classifiers_json)"
PARSER_FEATURE_CLASSIFIERS_JSON="$(eval_parser_expression_feature_classifiers_json)"
PARSER_OPERATOR_FEATURE_CLASSIFIERS_JSON="$(eval_parser_operator_feature_classifiers_json)"
PARSER_SURFACE_CLASSIFIERS_JSON="$(eval_parser_expression_surface_classifiers_json)"
PARSER_DECLARATION_FEATURE_CLASSIFIERS_JSON="$(eval_parser_declaration_feature_classifiers_json)"
PARSER_BINARY_OPERATOR_NAMES_JSON="$(eval_parser_binary_operator_names_json)"
PARSER_UNARY_OPERATOR_NAMES_JSON="$(eval_parser_unary_operator_names_json)"
PARSER_EXPRESSION_DISPATCH_PLAN_TEXT="$(eval_parser_expression_dispatch_plan_text)"
PARSER_DECLARATION_DISPATCH_PLAN_TEXT="$(eval_parser_declaration_dispatch_plan_text)"
PARSER_STATEMENT_DISPATCH_PLAN_TEXT="$(eval_parser_statement_dispatch_plan_text)"
PARSER_STATEMENT_SOURCE_DISPATCH_PLAN_TEXT="$(eval_parser_statement_source_dispatch_plan_text)"
PARSER_TAKE_SOURCE_DISPATCH_PLAN_TEXT="$(eval_parser_take_source_dispatch_plan_text)"
PARSER_FEATURE_INT_KIND="$(printf '%s\n' "$PARSER_FEATURE_CLASSIFIERS_JSON" | jq -er '.int')"
PARSER_FEATURE_STRING_KIND="$(printf '%s\n' "$PARSER_FEATURE_CLASSIFIERS_JSON" | jq -er '.string')"
PARSER_FEATURE_TRUE_KIND="$(printf '%s\n' "$PARSER_FEATURE_CLASSIFIERS_JSON" | jq -er '.true')"
PARSER_FEATURE_FALSE_KIND="$(printf '%s\n' "$PARSER_FEATURE_CLASSIFIERS_JSON" | jq -er '.false')"
PARSER_FEATURE_EMPTY_LIST_KIND="$(printf '%s\n' "$PARSER_FEATURE_CLASSIFIERS_JSON" | jq -er '.empty_list')"
PARSER_FEATURE_IDENTIFIER_KIND="$(printf '%s\n' "$PARSER_FEATURE_CLASSIFIERS_JSON" | jq -er '.identifier')"
PARSER_FEATURE_FALLBACK_KIND="$(printf '%s\n' "$PARSER_FEATURE_CLASSIFIERS_JSON" | jq -er '.fallback')"
PARSER_FEATURE_BINARY_KIND="$(printf '%s\n' "$PARSER_OPERATOR_FEATURE_CLASSIFIERS_JSON" | jq -er '.logical')"
PARSER_FEATURE_UNARY_KIND="$(printf '%s\n' "$PARSER_OPERATOR_FEATURE_CLASSIFIERS_JSON" | jq -er '.unary')"
PARSER_FEATURE_CALL_KIND="$(printf '%s\n' "$PARSER_SURFACE_CLASSIFIERS_JSON" | jq -er '.call')"
PARSER_FEATURE_BARE_CALL_KIND="$(printf '%s\n' "$PARSER_SURFACE_CLASSIFIERS_JSON" | jq -er '.bare_call')"
PARSER_FEATURE_FIELD_ACCESS_KIND="$(printf '%s\n' "$PARSER_SURFACE_CLASSIFIERS_JSON" | jq -er '.field')"
PARSER_FEATURE_INDEX_ACCESS_KIND="$(printf '%s\n' "$PARSER_SURFACE_CLASSIFIERS_JSON" | jq -er '.index')"
PARSER_FEATURE_LIST_LITERAL_KIND="$(printf '%s\n' "$PARSER_SURFACE_CLASSIFIERS_JSON" | jq -er '.list')"
PARSER_FEATURE_MAP_LITERAL_KIND="$(printf '%s\n' "$PARSER_SURFACE_CLASSIFIERS_JSON" | jq -er '.map')"
PARSER_FEATURE_RECORD_LITERAL_KIND="$(printf '%s\n' "$PARSER_SURFACE_CLASSIFIERS_JSON" | jq -er '.record')"
PARSER_FEATURE_TRY_KIND="$(printf '%s\n' "$PARSER_SURFACE_CLASSIFIERS_JSON" | jq -er '.try_suffix')"
PARSER_FEATURE_IF_EXPR_KIND="$(printf '%s\n' "$PARSER_SURFACE_CLASSIFIERS_JSON" | jq -er '.if_expression')"
PARSER_FEATURE_MODULE_DECLARATION_KIND="$(printf '%s\n' "$PARSER_DECLARATION_FEATURE_CLASSIFIERS_JSON" | jq -er '.module')"
PARSER_FEATURE_IMPORT_DECLARATION_KIND="$(printf '%s\n' "$PARSER_DECLARATION_FEATURE_CLASSIFIERS_JSON" | jq -er '.import')"
PARSER_FEATURE_TYPE_DECLARATION_KIND="$(printf '%s\n' "$PARSER_DECLARATION_FEATURE_CLASSIFIERS_JSON" | jq -er '.type')"
PARSER_FEATURE_EFFECT_DECLARATION_KIND="$(printf '%s\n' "$PARSER_DECLARATION_FEATURE_CLASSIFIERS_JSON" | jq -er '.effect')"
PARSER_FEATURE_TASK_DECLARATION_KIND="$(printf '%s\n' "$PARSER_DECLARATION_FEATURE_CLASSIFIERS_JSON" | jq -er '.task')"
PARSER_OP_OR="$(printf '%s\n' "$PARSER_BINARY_OPERATOR_NAMES_JSON" | jq -er '."||"')"
PARSER_OP_AND="$(printf '%s\n' "$PARSER_BINARY_OPERATOR_NAMES_JSON" | jq -er '."&&"')"
PARSER_OP_EQUAL="$(printf '%s\n' "$PARSER_BINARY_OPERATOR_NAMES_JSON" | jq -er '."=="')"
PARSER_OP_NOT_EQUAL="$(printf '%s\n' "$PARSER_BINARY_OPERATOR_NAMES_JSON" | jq -er '."!="')"
PARSER_OP_LESS="$(printf '%s\n' "$PARSER_BINARY_OPERATOR_NAMES_JSON" | jq -er '."<"')"
PARSER_OP_LESS_EQUAL="$(printf '%s\n' "$PARSER_BINARY_OPERATOR_NAMES_JSON" | jq -er '."<="')"
PARSER_OP_GREATER="$(printf '%s\n' "$PARSER_BINARY_OPERATOR_NAMES_JSON" | jq -er '.">"')"
PARSER_OP_GREATER_EQUAL="$(printf '%s\n' "$PARSER_BINARY_OPERATOR_NAMES_JSON" | jq -er '.">="')"
PARSER_OP_ADD="$(printf '%s\n' "$PARSER_BINARY_OPERATOR_NAMES_JSON" | jq -er '."+"')"
PARSER_OP_MULTIPLY="$(printf '%s\n' "$PARSER_BINARY_OPERATOR_NAMES_JSON" | jq -er '."*"')"
PARSER_UNARY_OP_NOT="$(printf '%s\n' "$PARSER_UNARY_OPERATOR_NAMES_JSON" | jq -er '."!"')"
PARSER_UNARY_OP_NEGATE="$(printf '%s\n' "$PARSER_UNARY_OPERATOR_NAMES_JSON" | jq -er '."-"')"
PARSER_TRUE_KIND="$(printf '%s\n' "$PARSER_CLASSIFIERS_JSON" | jq -er '."true"')"
PARSER_FALSE_KIND="$(printf '%s\n' "$PARSER_CLASSIFIERS_JSON" | jq -er '."false"')"
PARSER_EMPTY_LIST_KIND="$(printf '%s\n' "$PARSER_CLASSIFIERS_JSON" | jq -er '."[]"')"
PARSER_FALLBACK_KIND="$(printf '%s\n' "$PARSER_CLASSIFIERS_JSON" | jq -er '.__fallback__')"
PARSER_TASK_ID_TEMPLATE="$(eval_parser_id_template task_id_via_call)"
PARSER_CALL_TARGET_TASK_ID_TEMPLATE="$(eval_parser_id_template call_target_task_id)"
PARSER_IMPORT_ID_TEMPLATE="$(eval_parser_id_template import_id)"
PARSER_TYPE_ID_TEMPLATE="$(eval_parser_id_template type_id)"
PARSER_EFFECT_ID_TEMPLATE="$(eval_parser_id_template effect_id)"
PARSER_TAKE_ID_TEMPLATE="$(eval_parser_id_template take_id)"
PARSER_TAKE_SURFACE_ID_TEMPLATE="$(eval_parser_id_template take_surface_id)"
PARSER_MODULE_SURFACE_ID_TEMPLATE="$(eval_parser_id_template module_surface_id)"
PARSER_MODULE_TASK_LIST_SURFACE_ID_TEMPLATE="$(eval_parser_id_template module_task_list_surface_id)"
PARSER_MODULE_IMPORT_LIST_SURFACE_ID_TEMPLATE="$(eval_parser_id_template module_import_list_surface_id)"
PARSER_MODULE_TYPE_LIST_SURFACE_ID_TEMPLATE="$(eval_parser_id_template module_type_list_surface_id)"
PARSER_MODULE_EFFECT_LIST_SURFACE_ID_TEMPLATE="$(eval_parser_id_template module_effect_list_surface_id)"
PARSER_BLOCK_TASK_SURFACE_ID_TEMPLATE="$(eval_parser_id_template block_task_surface_id)"
PARSER_TASK_STATEMENT_SURFACE_ID_TEMPLATE="$(eval_parser_id_template task_statement_surface_id)"
PARSER_BRANCH_STATEMENT_SURFACE_ID_TEMPLATE="$(eval_parser_id_template branch_statement_surface_id)"
PARSER_STATEMENT_ID_TEMPLATE="$(eval_parser_id_template statement_id)"
PARSER_EXPRESSION_ID_TEMPLATE="$(eval_parser_id_template expression_id)"
PARSER_COLLECTION_EXPRESSION_ID_TEMPLATE="$(eval_parser_id_template collection_expression_id)"
PARSER_CONDITION_EXPRESSION_ID_TEMPLATE="$(eval_parser_id_template condition_expression_id)"
PARSER_FOR_ITEM_NAME_TEMPLATE="$(sley_source_task loom.parser for_item_name_template)"
PARSER_FOR_COLLECTION_SOURCE_TEMPLATE="$(sley_source_task loom.parser for_collection_source_template)"
PARSER_CONDITION_SOURCE_TEMPLATE="$(sley_source_task loom.parser condition_source_template)"
PARSER_BODY_DEPTH_OFFSET="$(sley_source_task loom.parser nested_body_depth_offset)"
if [[ ! "$PARSER_BODY_DEPTH_OFFSET" =~ ^[0-9]+$ ]]; then
  echo "invalid parser nested body depth offset: $PARSER_BODY_DEPTH_OFFSET" >&2
  exit 1
fi
PARSER_CALL_ARGUMENT_SURFACE_ID_TEMPLATE="$(eval_parser_id_template call_argument_surface_id)"
PARSER_EXPRESSION_SIDE_SURFACE_ID_TEMPLATE="$(eval_parser_id_template expression_side_surface_id)"
PARSER_RETURN_STATEMENT_KIND="$(sley_source_task loom.parser return_statement_kind)"
PARSER_EXPR_STATEMENT_KIND="$(sley_source_task loom.parser expression_statement_kind)"
PARSER_BINDING_STATEMENT_KIND="$(sley_source_task loom.parser binding_statement_kind)"
PARSER_SET_STATEMENT_KIND="$(sley_source_task loom.parser set_statement_kind)"
PARSER_FOR_STATEMENT_KIND="$(sley_source_task loom.parser for_statement_kind)"
PARSER_WHILE_STATEMENT_KIND="$(sley_source_task loom.parser while_statement_kind)"
PARSER_IF_STATEMENT_KIND="$(sley_source_task loom.parser if_statement_kind)"
PARSER_FORGE_STATEMENT_KIND="$(sley_source_task loom.parser forge_statement_kind)"
PARSER_STATEMENT_FEATURE_CLASSIFIERS_JSON="$(eval_parser_statement_feature_classifiers_json)"
PARSER_FEATURE_RETURN_STATEMENT_KIND="$(printf '%s\n' "$PARSER_STATEMENT_FEATURE_CLASSIFIERS_JSON" | jq -er '.return')"
PARSER_FEATURE_CALL_STATEMENT_KIND="$(printf '%s\n' "$PARSER_STATEMENT_FEATURE_CLASSIFIERS_JSON" | jq -er '.call_expression')"
PARSER_FEATURE_BINDING_STATEMENT_KIND="$(printf '%s\n' "$PARSER_STATEMENT_FEATURE_CLASSIFIERS_JSON" | jq -er '.binding')"
PARSER_FEATURE_SET_STATEMENT_KIND="$(printf '%s\n' "$PARSER_STATEMENT_FEATURE_CLASSIFIERS_JSON" | jq -er '.set')"
PARSER_FEATURE_FOR_STATEMENT_KIND="$(printf '%s\n' "$PARSER_STATEMENT_FEATURE_CLASSIFIERS_JSON" | jq -er '.for')"
PARSER_FEATURE_WHILE_STATEMENT_KIND="$(printf '%s\n' "$PARSER_STATEMENT_FEATURE_CLASSIFIERS_JSON" | jq -er '.while')"
PARSER_FEATURE_IF_STATEMENT_KIND="$(printf '%s\n' "$PARSER_STATEMENT_FEATURE_CLASSIFIERS_JSON" | jq -er '.if')"
PARSER_FEATURE_FORGE_STATEMENT_KIND="$(printf '%s\n' "$PARSER_STATEMENT_FEATURE_CLASSIFIERS_JSON" | jq -er '.forge')"
PARSER_FEATURE_FALLBACK_STATEMENT_KIND="$(printf '%s\n' "$PARSER_STATEMENT_FEATURE_CLASSIFIERS_JSON" | jq -er '.fallback')"
PARSER_TAKE_BINDING_KIND="$(sley_source_task loom.parser take_binding_kind)"
PARSER_GATE_BINDING_KIND="$(sley_source_task loom.parser gate_binding_kind)"
PARSER_STATE_BINDING_KIND="$(sley_source_task loom.parser state_binding_kind)"
PARSER_TALLY_BINDING_KIND="$(sley_source_task loom.parser tally_binding_kind)"
PARSER_BIND_BINDING_KIND="$(sley_source_task loom.parser bind_binding_kind)"
PARSER_BINDING_FEATURE_CLASSIFIERS_JSON="$(eval_parser_binding_feature_classifiers_json)"
PARSER_FEATURE_TAKE_BINDING_KIND="$(printf '%s\n' "$PARSER_BINDING_FEATURE_CLASSIFIERS_JSON" | jq -er '.take')"
PARSER_FEATURE_GATE_BINDING_KIND="$(printf '%s\n' "$PARSER_BINDING_FEATURE_CLASSIFIERS_JSON" | jq -er '.gate')"
PARSER_FEATURE_BIND_BINDING_KIND="$(printf '%s\n' "$PARSER_BINDING_FEATURE_CLASSIFIERS_JSON" | jq -er '.bind')"
PARSER_FEATURE_STATE_BINDING_KIND="$(printf '%s\n' "$PARSER_BINDING_FEATURE_CLASSIFIERS_JSON" | jq -er '.state')"
PARSER_FEATURE_TALLY_BINDING_KIND="$(printf '%s\n' "$PARSER_BINDING_FEATURE_CLASSIFIERS_JSON" | jq -er '.tally')"
sley_template_replace() {
  local out="$1" key value needle
  shift
  while (($# >= 2)); do
    key="$1"
    value="$2"
    needle="{{${key}}}"
    out="${out//$needle/$value}"
    shift 2
  done
  printf '%s\n' "$out"
}

parser_task_id_value() {
  local module_name="$1" task_name="$2"
  sley_template_replace "$PARSER_TASK_ID_TEMPLATE" module "$module_name" task "$task_name"
}

parser_block_task_surface_value() {
  local module_name="$1" task_name="$2" task_id
  task_id="$(parser_task_id_value "$module_name" "$task_name")"
  sley_template_replace "$PARSER_BLOCK_TASK_SURFACE_ID_TEMPLATE" task_id "$task_id"
}

parser_task_statement_surface_value() {
  local module_name="$1" task_name="$2" index="$3" task_id
  task_id="$(parser_task_id_value "$module_name" "$task_name")"
  sley_template_replace "$PARSER_TASK_STATEMENT_SURFACE_ID_TEMPLATE" task_id "$task_id" index "$index"
}

parser_task_expression_surface_value() {
  local module_name="$1" task_name="$2" index="$3" statement_id
  statement_id="$(parser_task_statement_surface_value "$module_name" "$task_name" "$index")"
  sley_template_replace "$PARSER_EXPRESSION_ID_TEMPLATE" statement "$statement_id"
}

parser_task_expression_side_surface_value() {
  local module_name="$1" task_name="$2" index="$3" side_name="$4" expression_id
  expression_id="$(parser_task_expression_surface_value "$module_name" "$task_name" "$index")"
  sley_template_replace "$PARSER_EXPRESSION_SIDE_SURFACE_ID_TEMPLATE" expression "$expression_id" side "$side_name"
}

PARSER_AST_TASK_NODE_KIND="$(sley_source_task loom.parser ast_task_node_kind)"
PARSER_AST_STATEMENT_NODE_KIND="$(sley_source_task loom.parser ast_statement_node_kind)"
PARSER_AST_EXPRESSION_NODE_KIND="$(sley_source_task loom.parser ast_expression_node_kind)"
PARSER_AST_NODE_NOT_FOUND_ID="$(sley_source_task loom.parser ast_node_not_found_id)"
PARSER_AST_NODE_NOT_FOUND_MESSAGE_TEMPLATE="$(eval_parser_message_template ast_node_not_found_message node_id)"
