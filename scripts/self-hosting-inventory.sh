#!/usr/bin/env bash
set -euo pipefail

ROOT_DIR="$(cd "$(dirname "$0")/.." && pwd)"
cd "$ROOT_DIR"

OUT="${1:-/tmp/self-hosting-inventory.md}"

{
  echo "# Self-Hosting Migration Inventory"
  echo ""
  echo "Generated: $(date -u +%Y-%m-%dT%H:%M:%SZ)"
  echo ""
  echo "## Foreign-language surface scan"
  echo ""
  echo "| Extension | Count |"
  echo "|---|---:|"

  for ext in rs c cc cpp h hpp m mm swift go java kt cs rb php py js mjs ts tsx; do
    count=$(find . \( -path "./.git" -o -path "./target" -o -path "./node_modules" \) -prune -o -type f -name "*.${ext}" -print | wc -l)
    echo "| *.${ext} | ${count} |"
  done

  echo ""
  echo "## Stage-1 executable surfaces"
  echo ""
  if [ -d bin ]; then
    find bin -maxdepth 1 -type f -perm -111 -print | sort | sed 's#^\./##' | sed 's/^/- /'
  else
    echo "- bin directory missing"
  fi

  echo ""
  echo "## Reported CLI commands in llms.txt"
  echo ""
  if [ -f llms.txt ]; then
    awk '
      /^[[:space:]]*(sley|sley-ci|sley-conformance|sley-contract|sley-lsp|sley-workbench|sley-docgen|sley-agent-bench|sley-migrate|sley-sandbox-runner|sley-zjx)[[:space:]]/ {
        line=$0
        sub(/^[[:space:]]*/, "", line)
        count++
        printf "%d. `%s`\n", count, line
      }
    ' llms.txt
  else
    echo "- llms.txt missing"
  fi

  echo ""
  echo "## Make targets in Makefile"
  echo ""
  if [ -f Makefile ]; then
    awk '/^[a-zA-Z0-9_-]+:/ {print $1}' Makefile | sed 's/:$//' | sed 's/^/- /'
  else
    echo "- Makefile missing"
  fi

  echo ""
  echo "## Current tracked files"
  echo ""
  git ls-files | sort

  echo ""
  echo "## Notes"
  echo ""
  echo "- This report is generated with \`./scripts/self-hosting-inventory.sh\`."
  echo "- The extension scan is a release blocker, not a full semantic parity proof."
  echo "- Runtime parity evidence lives in \`scripts/self-hosted-test.sh\` and \`make v1\`."
} > "$OUT"

echo "Inventory written to $OUT"
