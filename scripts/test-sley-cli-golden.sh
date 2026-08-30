#!/usr/bin/env bash
set -euo pipefail

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
GOLDEN_DIR="$ROOT_DIR/fixtures/golden/cli"
WORK_DIR="$(mktemp -d "${TMPDIR:-/tmp}/sley-cli-golden.XXXXXXXX")"
trap 'rm -rf -- "$WORK_DIR"' EXIT

assert_command() {
  local name="$1" expected_status="$2"
  shift 2
  local observed_status=0
  if "$@" > "$WORK_DIR/$name.stdout" 2> "$WORK_DIR/$name.stderr"; then
    observed_status=0
  else
    observed_status=$?
  fi
  [[ "$observed_status" -eq "$expected_status" ]] || {
    printf 'golden %s status mismatch: expected %s, observed %s\n' "$name" "$expected_status" "$observed_status" >&2
    return 1
  }
  if [[ -f "$GOLDEN_DIR/$name.stdout" ]]; then
    cmp -s "$GOLDEN_DIR/$name.stdout" "$WORK_DIR/$name.stdout" || {
      diff -u "$GOLDEN_DIR/$name.stdout" "$WORK_DIR/$name.stdout" >&2
      return 1
    }
  elif [[ -s "$WORK_DIR/$name.stdout" ]]; then
    printf 'golden %s emitted unexpected stdout\n' "$name" >&2
    return 1
  fi
  if [[ -f "$GOLDEN_DIR/$name.stderr" ]]; then
    cmp -s "$GOLDEN_DIR/$name.stderr" "$WORK_DIR/$name.stderr" || {
      diff -u "$GOLDEN_DIR/$name.stderr" "$WORK_DIR/$name.stderr" >&2
      return 1
    }
  elif [[ -s "$WORK_DIR/$name.stderr" ]]; then
    printf 'golden %s emitted unexpected stderr\n' "$name" >&2
    return 1
  fi
}

assert_command version 0 "$ROOT_DIR/bin/sley" --version
assert_command help 0 "$ROOT_DIR/bin/sley" --help
assert_command check-ok 0 "$ROOT_DIR/bin/sley" check --json "$ROOT_DIR/examples/hello.sley"
assert_command check-error 1 "$ROOT_DIR/bin/sley" check --json "$ROOT_DIR/fixtures/corpus/rejected/unknown_identifier.sley"
assert_command query-tasks 0 "$ROOT_DIR/bin/sley" query --json --kind tasks "$ROOT_DIR/examples/hello.sley"
assert_command unknown 2 "$ROOT_DIR/bin/sley" definitely-not-a-command

package_root="$WORK_DIR/package"
mkdir -p "$package_root/bin" "$package_root/self-hosted"
cp "$ROOT_DIR/bin/sley" "$package_root/bin/sley"
cp -R "$ROOT_DIR/self-hosted/src" "$package_root/self-hosted/src"
if [[ -d "$ROOT_DIR/lib/sley" ]]; then
  mkdir -p "$package_root/lib"
  cp -R "$ROOT_DIR/lib/sley" "$package_root/lib/sley"
fi
assert_command version 0 "$package_root/bin/sley" --version

signal_status=0
if SLEY_DISABLE_SOURCE_CACHE=1 timeout --preserve-status --signal=TERM 0.1s \
  "$ROOT_DIR/bin/sley" --version \
  > "$WORK_DIR/signal.stdout" 2> "$WORK_DIR/signal.stderr"; then
  signal_status=0
else
  signal_status=$?
fi
[[ "$signal_status" -eq 143 ]] || {
  printf 'golden signal status mismatch: expected 143, observed %s\n' "$signal_status" >&2
  exit 1
}
[[ ! -s "$WORK_DIR/signal.stdout" && ! -s "$WORK_DIR/signal.stderr" ]] || {
  printf 'golden signal case emitted output before termination\n' >&2
  exit 1
}

printf 'Sley CLI golden behavior passed.\n'
