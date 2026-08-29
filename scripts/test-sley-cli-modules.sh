#!/usr/bin/env bash
set -euo pipefail

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
MODULE_DIR="$ROOT_DIR/lib/sley"

mapfile -t modules < <(find "$MODULE_DIR" -maxdepth 1 -type f -name '*.sh' -print | sort)
[[ "${#modules[@]}" -eq 8 ]] || {
  printf 'expected 8 Sley CLI modules, found %s\n' "${#modules[@]}" >&2
  exit 1
}

[[ "$(wc -l < "$ROOT_DIR/bin/sley")" -le 80 ]] || {
  printf 'bin/sley is no longer a minimal launcher\n' >&2
  exit 1
}
[[ "$(wc -l < "$MODULE_DIR/dispatch.sh")" -le 100 ]] || {
  printf 'Sley command dispatcher exceeded its 100-line boundary\n' >&2
  exit 1
}
if rg -n '^[a-zA-Z_][a-zA-Z0-9_]*\(\) \{' "$ROOT_DIR/bin/sley" >/dev/null; then
  printf 'bin/sley contains implementation functions instead of launcher wiring\n' >&2
  exit 1
fi

for module in "${modules[@]}"; do
  bash -n "$module"
  [[ "$(wc -l < "$module")" -le 7000 ]] || {
    printf 'Sley CLI module exceeds the 7000-line family ceiling: %s\n' "$module" >&2
    exit 1
  }
done
bash -n "$ROOT_DIR/bin/sley"

(
  export SLEY_CLI_LIBRARY_ONLY=1
  # shellcheck source=../bin/sley
  source "$ROOT_DIR/bin/sley"
  for function_name in \
    main_sley command_ast command_check command_explain command_doctor \
    command_graph command_graph_diff command_query command_lint command_plan \
    command_adapter command_worker command_test command_validate command_release \
    command_change command_verify command_claim_verify command_run command_machine \
    command_deploy command_trace command_seal command_zjx command_arena \
    command_graft command_format command_new; do
    declare -F "$function_name" >/dev/null || {
      printf 'modular Sley CLI did not load %s\n' "$function_name" >&2
      exit 1
    }
  done
)

if command -v shellcheck >/dev/null 2>&1; then
  shellcheck --severity=error -x "$ROOT_DIR/bin/sley" "$ROOT_DIR/scripts/test-sley-cli-modules.sh"
else
  printf 'shellcheck unavailable; bash syntax and module-boundary checks completed\n' >&2
fi

"$ROOT_DIR/scripts/test-sley-cli-golden.sh"
printf 'Sley CLI module boundaries passed.\n'
