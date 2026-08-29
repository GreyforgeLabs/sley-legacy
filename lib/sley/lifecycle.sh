# shellcheck shell=bash
# Shared dependency, CLI parsing, JSON, numeric-bound, and temp cleanup helpers.

need_jq() {
  if ! command -v jq >/dev/null 2>&1; then
    echo "sley stage-1 requires jq for JSON shaping" >&2
    exit 127
  fi
}

json_string() {
  jq -Rn --arg v "$1" '$v'
}

json_list_contains_string() {
  local json="$1" item="$2"
  printf '%s\n' "$json" | jq -e --arg item "$item" 'index($item) != null' >/dev/null
}

last_path_arg() {
  local previous="" arg candidate=""
  for arg in "$@"; do
    case "$previous" in
      --schema|--schemas|--kind|--module|--rule|--template|--name|--cap|--db-table|--secret|--http-text|--shell-output|--model-output|--deploy-result|--spend-result|--artifacts-dir|--html|--markdown|--fixtures|--require-tag|--template-surface|--trace|--slice|--goal|--actor|--nonce|--created-at|--expires-at|--inspection|--operation|--repository|--requested-outcome|--assumption|--non-goal|--validation-profile|--plan|--preview|--request|--issuer|--principal|--audience|--purpose|--ack-review|--revocation-state|--max-wall-clock-seconds|--signature-ref|--greynucleus-proof-ref)
        previous=""
        continue
        ;;
    esac
    case "$arg" in
      --*) previous="$arg" ;;
      *) candidate="$arg"; previous="" ;;
    esac
  done
  printf '%s\n' "$candidate"
}

sley_require_bounded_positive_integer() {
  local name="$1" value="$2" maximum="$3"
  if [[ ! "$value" =~ ^[1-9][0-9]*$ ]] \
    || [[ "${#value}" -gt "${#maximum}" ]] \
    || [[ "${#value}" -eq "${#maximum}" && "$value" > "$maximum" ]]; then
    printf 'sley: %s must be a positive integer no greater than %s\n' "$name" "$maximum" >&2
    exit 2
  fi
}

sley_cleanup_temp_dir() {
  local path="$1" temp_root
  [[ -n "$path" && -d "$path" && ! -L "$path" ]] || return 0
  temp_root="$(realpath -m -- "${TMPDIR:-/tmp}")"
  path="$(realpath -m -- "$path")"
  if [[ "$path" == "$temp_root/"* ]]; then
    rm -rf -- "$path"
  else
    printf 'sley: refused cleanup outside temporary root: %s\n' "$path" >&2
    return 1
  fi
}
