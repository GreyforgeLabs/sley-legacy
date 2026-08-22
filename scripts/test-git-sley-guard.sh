#!/usr/bin/env bash
set -euo pipefail

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
GUARD="$ROOT_DIR/bin/git-sley-guard"
TEST_ROOT="$(mktemp -d "${TMPDIR:-/tmp}/git-sley-guard-test.XXXXXX")"

cleanup() {
  rm -rf -- "$TEST_ROOT"
}
trap cleanup EXIT HUP INT TERM

new_repo() {
  local name="$1" repo
  repo="$TEST_ROOT/$name"
  mkdir -p "$repo"
  git -C "$repo" init -q
  git -C "$repo" config user.name "Sley Guard Test"
  git -C "$repo" config user.email "sley-guard@example.invalid"
  printf '%s\n' "$repo"
}

commit_all() {
  local repo="$1"
  git -C "$repo" add -A
  git -C "$repo" commit -qm baseline
}

expect_pass() {
  local repo="$1" name="$2"
  if ! SLEY_BIN="$ROOT_DIR/bin/sley" \
    SLEY_CONTRACT_BIN="$ROOT_DIR/bin/sley-contract" \
    "$GUARD" --repo-root "$repo" >"$TEST_ROOT/$name.out" 2>"$TEST_ROOT/$name.err"; then
    printf 'expected pass: %s\n' "$name" >&2
    cat "$TEST_ROOT/$name.out" >&2
    cat "$TEST_ROOT/$name.err" >&2
    return 1
  fi
}

expect_fail() {
  local repo="$1" name="$2"
  if SLEY_BIN="$ROOT_DIR/bin/sley" \
    SLEY_CONTRACT_BIN="$ROOT_DIR/bin/sley-contract" \
    "$GUARD" --repo-root "$repo" >"$TEST_ROOT/$name.out" 2>"$TEST_ROOT/$name.err"; then
    printf 'expected failure: %s\n' "$name" >&2
    cat "$TEST_ROOT/$name.out" >&2
    cat "$TEST_ROOT/$name.err" >&2
    return 1
  fi
}

repo="$(new_repo no_relevant)"
printf 'baseline\n' >"$repo/README.md"
commit_all "$repo"
printf 'staged docs only\n' >"$repo/README.md"
git -C "$repo" add README.md
expect_pass "$repo" no_relevant

repo="$(new_repo staged_snapshot)"
cp "$ROOT_DIR/examples/hello.sley" "$repo/main.sley"
commit_all "$repo"
cp "$ROOT_DIR/fixtures/corpus/accepted/pure_main.sley" "$repo/main.sley"
git -C "$repo" add main.sley
printf 'this is invalid mutable worktree text\n' >"$repo/main.sley"
expect_pass "$repo" staged_snapshot

repo="$(new_repo invalid_source)"
cp "$ROOT_DIR/examples/hello.sley" "$repo/main.sley"
commit_all "$repo"
cp "$ROOT_DIR/fixtures/corpus/rejected/unknown_identifier.sley" "$repo/main.sley"
git -C "$repo" add main.sley
expect_fail "$repo" invalid_source
rg -q 'staged check rejected' "$TEST_ROOT/invalid_source.err"

repo="$(new_repo missing_authority)"
cp "$ROOT_DIR/examples/hello.sley" "$repo/main.sley"
commit_all "$repo"
cp "$ROOT_DIR/fixtures/corpus/rejected/authority/missing_network_effect.sley" \
  "$repo/main.sley"
git -C "$repo" add main.sley
expect_fail "$repo" missing_authority
rg -q 'staged check rejected' "$TEST_ROOT/missing_authority.err"

repo="$(new_repo lint_warning)"
cp "$ROOT_DIR/examples/hello.sley" "$repo/main.sley"
commit_all "$repo"
cp "$ROOT_DIR/examples/unused_private_task.sley" "$repo/main.sley"
git -C "$repo" add main.sley
expect_fail "$repo" lint_warning
rg -q 'staged lint rejected' "$TEST_ROOT/lint_warning.err"

repo="$(new_repo project_context)"
cp -R "$ROOT_DIR/examples/project/." "$repo/"
commit_all "$repo"
printf '\n' >>"$repo/src/app/main.sley"
git -C "$repo" add src/app/main.sley
expect_pass "$repo" project_context
rg -q 'check staged target: \.' "$TEST_ROOT/project_context.out"

repo="$(new_repo deleted_manifest)"
cp -R "$ROOT_DIR/examples/project/." "$repo/"
commit_all "$repo"
git -C "$repo" rm -q sley.toml
expect_pass "$repo" deleted_manifest
rg -q 'check staged target: \.' "$TEST_ROOT/deleted_manifest.out"

repo="$(new_repo symlink_escape)"
printf 'baseline\n' >"$repo/README.md"
commit_all "$repo"
ln -s "$ROOT_DIR/examples/hello.sley" "$repo/outside.sley"
git -C "$repo" add outside.sley
expect_fail "$repo" symlink_escape
rg -q 'staged validation path is a symlink' "$TEST_ROOT/symlink_escape.err"

repo="$(new_repo invalid_contract)"
mkdir -p "$repo/docs" "$repo/fixtures"
cp -R "$ROOT_DIR/docs/schemas" "$repo/docs/"
cp -R "$ROOT_DIR/fixtures/contracts" "$repo/fixtures/"
commit_all "$repo"
printf '{ invalid json\n' >"$repo/fixtures/contracts/ast_minimal_program.json"
git -C "$repo" add fixtures/contracts/ast_minimal_program.json
expect_fail "$repo" invalid_contract
rg -q 'staged contract fixtures were rejected' "$TEST_ROOT/invalid_contract.err"

printf 'git-sley-guard tests passed\n'
