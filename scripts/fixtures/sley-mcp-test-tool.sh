#!/usr/bin/env bash
set -euo pipefail

mode="${SLEY_MCP_TEST_FAKE_MODE:-fast}"
marker="${SLEY_MCP_TEST_MARKER:-}"

if [[ "$mode" == fast ]]; then
  printf '%s\n' '{"schema":"sley.query.report.v0","kind":"tasks","entry_module":"test.fake","filters":{"module":null,"exported_only":false},"modules":[],"tasks":[],"types":[],"effects":[],"calls":[]}'
  exit 0
fi

[[ "$mode" == slow && -n "$marker" ]] || exit 2
printf '%s %s\n' "$$" "$(ps -o pgid= -p "$$" | tr -d '[:space:]')" > "$marker.started"
on_term() {
  printf '%s\n' "$$" > "$marker.terminated"
  exit 143
}
trap on_term TERM INT
while :; do
  sleep 1
done
