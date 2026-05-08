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

  for ext in rs c cc cpp h m mm swift; do
    count=$(find . -type f -name "*.${ext}" \( -path "./.git" -o -path "./target" -o -path "./node_modules" \) -prune -o -type f -name "*.${ext}" -print | wc -l)
    echo "| *.${ext} | ${count} |"
  done

  echo ""
  echo "## Rust bins present"
  echo ""
  if [ -d src/bin ]; then
    for f in src/bin/*.rs; do
      [ -f "$f" ] || continue
      name="$(basename "$f" .rs)"
      echo "- $name"
    done
  fi

  echo ""
  echo "## Reported CLI commands in llms.txt"
  echo ""
  if [ -f llms.txt ]; then
    python - <<'PY'
import re
from pathlib import Path
text = Path('llms.txt').read_text()
cmds = []
for line in text.splitlines():
    line = line.strip()
    if line.startswith("cargo run"):
        m = re.search(r"cargo run(?: --bin\s+([\w-]+))?(.*)", line)
        if m:
            bin_name = m.group(1) or 'sley'
            rest = m.group(2).strip()
            cmds.append(f"{bin_name} {rest}".strip())
for i, cmd in enumerate(cmds, 1):
    print(f"{i}. `{cmd}`")
PY
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
  echo "## Non-Rust/C non-foreign files (tracked)"
  echo ""
  find . -type f \
    \( -path "./.git" -o -path "./target" -o -path "./node_modules" \) -prune -o \
    \( -name "*.ts" -o -name "*.js" -o -name "*.mjs" -o -name "*.json" -o -name "*.md" -o -name "*.yaml" -o -name "*.yml" \) -print \
    | sort

  echo ""
  echo "## Notes"
  echo ""
  echo "- This report is generated with \`./scripts/self-hosting-inventory.sh\`."
  echo "- Treat as evidence for phase planning, not proof of runtime parity."
} > "$OUT"

echo "Inventory written to $OUT"
