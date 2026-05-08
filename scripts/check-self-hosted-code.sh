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
  "*.m"
  "*.mm"
  "*.swift"
)


echo "Checking for foreign-language source files in: $ROOT_DIR"

FOUND=0

for ext in "${FORBIDDEN_EXTENSIONS[@]}"; do
  while IFS= read -r file; do
    if [[ -n "$file" ]]; then
      echo "$file"
      FOUND=1
    fi
  done < <(find . \
    \( -path "./.git" -o -path "./target" -o -path "./node_modules" \) -prune -o \
    -type f -name "$ext" -print)
done

if [[ "$FOUND" -eq 1 ]]; then
  echo "\nSelf-hosted gate failed: foreign-language source files are present." >&2
  echo "Run a staged migration plan before removing this gate." >&2
  exit 1
fi

echo "Self-hosted gate passed: no forbidden extensions found."
