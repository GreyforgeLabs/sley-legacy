#!/usr/bin/env bash
set -euo pipefail
bash bin/sley-ci help >/dev/null
bash bin/sley-ci smoke examples/hello.sley >/dev/null || true
echo "sley-ci smoke ok"
