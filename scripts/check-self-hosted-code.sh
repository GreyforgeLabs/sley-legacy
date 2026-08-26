#!/usr/bin/env bash
set -euo pipefail

ROOT_DIR="$(cd "$(dirname "$0")/.." && pwd)"
cd "$ROOT_DIR"

FORBIDDEN_EXTENSIONS=(
  "*.rs"
  "*.c"
  "*.cc"
  "*.cpp"
  "*.h"
  "*.hpp"
  "*.m"
  "*.mm"
  "*.swift"
  "*.go"
  "*.java"
  "*.kt"
  "*.cs"
  "*.rb"
  "*.php"
  "*.py"
  "*.js"
  "*.mjs"
  "*.ts"
  "*.tsx"
)


echo "Checking for foreign-language source files in: $ROOT_DIR"

FOUND=0

for ext in "${FORBIDDEN_EXTENSIONS[@]}"; do
  while IFS= read -r file; do
    if [[ -n "$file" ]]; then
      case "$file" in
        ./clients/generate-worker-clients.py|./clients/python/sley_worker_client/*|./clients/python/tests/*|./clients/node/index.mjs|./clients/node/index.d.ts|./clients/node/generated/*|./clients/node/test/*)
          continue
          ;;
      esac
      echo "$file"
      FOUND=1
    fi
  done < <(find . \
    \( -path "./.git" -o -path "./target" -o -path "./node_modules" \) -prune -o \
    -type f -name "$ext" -print)
done

if [[ "$FOUND" -eq 1 ]]; then
  echo "" >&2
  echo "Self-hosted gate failed: forbidden foreign-language source files are present." >&2
  echo "Keep compiler/runtime implementation in .sley plus the permitted POSIX shell and worker-client surfaces." >&2
  exit 1
fi

echo "Self-hosted gate passed: no forbidden foreign-language source files outside the worker-client boundary."
