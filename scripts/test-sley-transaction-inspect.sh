#!/usr/bin/env bash
set -euo pipefail

ROOT_DIR="$(cd "$(dirname "$0")/.." && pwd)"
SLEY="$ROOT_DIR/bin/sley"
CONTRACT="$ROOT_DIR/bin/sley-contract"
SCHEMA_DIR="$ROOT_DIR/docs/schemas"
TARGET="$ROOT_DIR/examples/project"
WORK_DIR="$(mktemp -d)"
trap 'rm -rf "$WORK_DIR"' EXIT

fail() {
  echo "transaction inspect contract failed: $*" >&2
  exit 1
}

tree_digest() {
  local target="$1"
  find "$target" -type f -print0 \
    | sort -z \
    | xargs -0 sha256sum \
    | sha256sum \
    | awk '{print $1}'
}

file_digest_or_absent() {
  local path="$1"
  if [[ -f "$path" ]]; then
    sha256sum "$path" | awk '{print $1}'
  else
    printf 'absent\n'
  fi
}

expect_diagnostic() {
  local expected_id="$1"
  shift
  local output="$WORK_DIR/error.json"
  if "$SLEY" "$@" >"$output" 2>/dev/null; then
    fail "command unexpectedly succeeded for $expected_id"
  fi
  jq -e --arg id "$expected_id" '
    .schema == "sley.diagnostics.report.v0"
    and .status == "error"
    and any(.diagnostics[]; .id == $id and .severity == "error")
  ' "$output" >/dev/null || fail "missing typed diagnostic $expected_id"
}

git_index="$(git -C "$ROOT_DIR" rev-parse --git-path index)"
source_before="$(tree_digest "$TARGET")"
index_before="$(file_digest_or_absent "$git_index")"

"$SLEY" change inspect --json \
  --goal "inspect project transaction boundary" \
  --actor "operator:test" \
  --nonce "w4-inspect-001" \
  --created-at "2026-08-25T00:00:00Z" \
  --expires-at "2099-01-01T00:00:00Z" \
  "$TARGET" >"$WORK_DIR/report.json"

"$SLEY" change inspect --json \
  --goal "inspect project transaction boundary" \
  --actor "operator:test" \
  --nonce "w4-inspect-001" \
  --created-at "2026-08-25T00:00:00Z" \
  --expires-at "2099-01-01T00:00:00Z" \
  "$TARGET" >"$WORK_DIR/report-repeat.json"
cmp -s "$WORK_DIR/report.json" "$WORK_DIR/report-repeat.json" \
  || fail "identical inspection inputs did not produce identical evidence"

"$CONTRACT" validate \
  --schema sley.transaction.inspect.v0 \
  "$WORK_DIR/report.json" \
  --schemas "$SCHEMA_DIR" \
  --json >/dev/null

jq -e '
  .schema == "sley.transaction.inspect.v0"
  and .status == "inspected"
  and .transaction.state == "inspected"
  and .transaction.history == ["opened", "inspected"]
  and .transaction.base.mode == "working_tree"
  and (.transaction.base.commit | test("^[a-f0-9]{40,64}$"))
' "$WORK_DIR/report.json" >/dev/null || fail "transaction binding is incomplete"

jq -e '
  .transaction.compiler.contract_versions == {
    diagnostics:"sley.diagnostics.report.v1",
    graph_slice:"sley.symbol_graph.slice.v1",
    verify:"sley.verify.report.v1",
    inspection:"sley.transaction.inspect.v0"
  }
  and .mutations == {
    repository_source:false,
    git_index:false,
    trace_sidecars:false,
    external_systems:false,
    provider_state:false
  }
  and .issues == []
  and (.inspection.target_nodes | length) <= .inspection.bounds.max_target_nodes
  and .inspection.bounds.source_bytes >= 0
  and .inspection.bounds.context_bytes > 0
' "$WORK_DIR/report.json" >/dev/null || fail "inspection boundary is not read-only and bounded"

source_after="$(tree_digest "$TARGET")"
index_after="$(file_digest_or_absent "$git_index")"
[[ "$source_before" == "$source_after" ]] || fail "inspection changed repository source"
[[ "$index_before" == "$index_after" ]] || fail "inspection changed the Git index"

expect_diagnostic TRANSACTION_BINDING_REQUIRED \
  change inspect --json "$TARGET"
expect_diagnostic TRANSACTION_ARGUMENT_UNKNOWN \
  change inspect --json --unknown value "$TARGET"
expect_diagnostic TRANSACTION_NONCE_INVALID \
  change inspect --json --goal inspect --actor operator:test --nonce short \
  --created-at 2026-08-25T00:00:00Z --expires-at 2026-08-26T00:00:00Z "$TARGET"
expect_diagnostic TRANSACTION_TIME_ORDER_INVALID \
  change inspect --json --goal inspect --actor operator:test --nonce w4-expiry-001 \
  --created-at 2026-08-26T00:00:00Z --expires-at 2026-08-25T00:00:00Z "$TARGET"

ln -s "$TARGET" "$WORK_DIR/project-link"
expect_diagnostic TRANSACTION_TARGET_SYMLINK_DENIED \
  change inspect --json --goal inspect --actor operator:test --nonce w4-symlink-001 \
  --created-at 2026-08-25T00:00:00Z --expires-at 2026-08-26T00:00:00Z "$WORK_DIR/project-link"

mkdir "$WORK_DIR/not-a-repository"
printf 'module example\n' >"$WORK_DIR/not-a-repository/main.sley"
expect_diagnostic TRANSACTION_GIT_REPOSITORY_REQUIRED \
  change inspect --json --goal inspect --actor operator:test --nonce w4-no-git-001 \
  --created-at 2026-08-25T00:00:00Z --expires-at 2026-08-26T00:00:00Z "$WORK_DIR/not-a-repository"

git init -q "$WORK_DIR/unborn-repository"
printf 'module example\n' >"$WORK_DIR/unborn-repository/main.sley"
expect_diagnostic TRANSACTION_BASE_COMMIT_REQUIRED \
  change inspect --json --goal inspect --actor operator:test --nonce w4-unborn-001 \
  --created-at 2026-08-25T00:00:00Z --expires-at 2026-08-26T00:00:00Z "$WORK_DIR/unborn-repository"

printf '[]\n' >"$WORK_DIR/outside-trace.json"
expect_diagnostic TRANSACTION_TRACE_OUTSIDE_REPOSITORY \
  change inspect --json --goal inspect --actor operator:test --nonce w4-trace-001 \
  --created-at 2026-08-25T00:00:00Z --expires-at 2026-08-26T00:00:00Z \
  --trace "$WORK_DIR/outside-trace.json" "$TARGET"

expect_diagnostic TRANSACTION_TRACE_NOT_FOUND \
  change inspect --json --goal inspect --actor operator:test --nonce w4-trace-missing \
  --created-at 2026-08-25T00:00:00Z --expires-at 2026-08-26T00:00:00Z \
  --trace "$ROOT_DIR/.sley-transaction-trace-does-not-exist" "$TARGET"

expect_diagnostic TRANSACTION_TRACE_INVALID \
  change inspect --json --goal inspect --actor operator:test --nonce w4-trace-invalid \
  --created-at 2026-08-25T00:00:00Z --expires-at 2026-08-26T00:00:00Z \
  --trace "$ROOT_DIR/README.md" "$TARGET"

expect_diagnostic TRANSACTION_ATOMIC_APPLY_CONFIRMATION_REQUIRED change apply --json
expect_diagnostic TRANSACTION_RECOVERY_CONFIRMATION_REQUIRED change recover --json

echo "transaction inspect contract passed"
