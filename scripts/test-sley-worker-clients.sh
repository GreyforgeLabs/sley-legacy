#!/usr/bin/env bash
set -euo pipefail

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
WORKDIR="$(mktemp -d)"
trap 'rm -rf "$WORKDIR"' EXIT

assert_no_source_build_residue() {
  local residue
  residue="$(find "$ROOT_DIR/clients" \
    \( -type d \( -name build -o -name '*.egg-info' -o -name node_modules -o -name __pycache__ \) \
       -o -type f \( -name '*.pyc' -o -name '*.whl' -o -name '*.tgz' \) \) \
    -print -quit)"
  if [[ -n "$residue" ]]; then
    echo "worker client source tree contains generated build residue: $residue" >&2
    return 1
  fi
}

assert_no_source_build_residue

# Initialize the source-task cache before timed worker lifecycle checks. The
# full v1 gate does this in earlier targets; this target must also run alone.
"$ROOT_DIR/bin/sley" --version >/dev/null

python3 "$ROOT_DIR/clients/generate-worker-clients.py" --check
PYTHONPYCACHEPREFIX="$WORKDIR/pycache" python3 -m compileall -q \
  "$ROOT_DIR/clients/generate-worker-clients.py" \
  "$ROOT_DIR/clients/python/sley_worker_client"
NODE_BIN="$(command -v node)"
"$NODE_BIN" --check "$ROOT_DIR/clients/node/index.mjs"

mkdir -p "$WORKDIR/python-dist" "$WORKDIR/python-home"
cp -R "$ROOT_DIR/clients/python" "$WORKDIR/python-source"
python3 -m pip wheel \
  --disable-pip-version-check \
  --no-deps \
  --no-build-isolation \
  --wheel-dir "$WORKDIR/python-dist" \
  "$WORKDIR/python-source" >/dev/null
python3 -m venv "$WORKDIR/python-venv"
"$WORKDIR/python-venv/bin/python" -m pip install \
  --disable-pip-version-check \
  --no-index \
  --no-deps \
  "$WORKDIR"/python-dist/sley_worker_client-*.whl >/dev/null
env -i \
  HOME="$WORKDIR/python-home" \
  PATH="/usr/bin:/bin" \
  LANG="C.UTF-8" \
  LC_ALL="C.UTF-8" \
  PYTHONDONTWRITEBYTECODE=1 \
  "$WORKDIR/python-venv/bin/python" \
  "$ROOT_DIR/clients/python/tests/integration.py" \
  "$ROOT_DIR"

mkdir -p "$WORKDIR/node-pack" "$WORKDIR/node-env" "$WORKDIR/node-home"
npm pack \
  --ignore-scripts \
  --pack-destination "$WORKDIR/node-pack" \
  "$ROOT_DIR/clients/node" >/dev/null
npm install \
  --offline \
  --ignore-scripts \
  --no-audit \
  --no-fund \
  --prefix "$WORKDIR/node-env" \
  "$WORKDIR"/node-pack/sley-worker-client-*.tgz >/dev/null
cp "$ROOT_DIR/clients/node/test/integration.mjs" "$WORKDIR/node-env/integration.mjs"
cp "$ROOT_DIR/clients/node/test/types.ts" "$WORKDIR/node-env/types.ts"
TSC_JS="${SLEY_TYPESCRIPT_COMPILER:-}"
if [[ -z "$TSC_JS" ]]; then
  TSC_JS="$(find "$(npm root -g)" -path '*/typescript/lib/tsc.js' -print -quit 2>/dev/null || true)"
fi
if [[ -n "$TSC_JS" && -f "$TSC_JS" ]]; then
  node "$TSC_JS" \
    --noEmit \
    --strict \
    --target ES2022 \
    --module NodeNext \
    --moduleResolution NodeNext \
    "$WORKDIR/node-env/types.ts"
else
  node --experimental-strip-types --check "$WORKDIR/node-env/types.ts"
  echo "TypeScript compiler unavailable; Node type consumer received syntax-only validation" >&2
fi
env -i \
  HOME="$WORKDIR/node-home" \
  PATH="/usr/bin:/bin" \
  LANG="C.UTF-8" \
  LC_ALL="C.UTF-8" \
  "$NODE_BIN" "$WORKDIR/node-env/integration.mjs" "$ROOT_DIR"

assert_no_source_build_residue
echo "generated Python and Node worker clients passed offline package and lifecycle tests"
