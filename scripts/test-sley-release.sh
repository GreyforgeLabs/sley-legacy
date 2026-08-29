#!/usr/bin/env bash
set -euo pipefail

ROOT_DIR="$(cd "$(dirname "$0")/.." && pwd)"
TEST_ROOT="$(mktemp -d)"
OUT_ONE="$TEST_ROOT/one"
OUT_TWO="$TEST_ROOT/two"
mkdir -p "$OUT_ONE" "$OUT_TWO"
trap 'rm -rf -- "$TEST_ROOT"' EXIT

export SLEY_ALLOW_TEST_HOOKS=1
export SLEY_RELEASE_ALLOW_DIRTY=1

if "$ROOT_DIR/bin/sley" release "--repo-root=$TEST_ROOT" build --output-dir "$TEST_ROOT/root-bypass" --json > "$TEST_ROOT/root-bypass.out" 2> "$TEST_ROOT/root-bypass.err"; then
  echo "release wrapper unexpectedly accepted caller-controlled repository root" >&2
  exit 1
fi
grep -Fq -- '--repo-root is internal' "$TEST_ROOT/root-bypass.err"

"$ROOT_DIR/bin/sley" release build --output-dir "$OUT_ONE" --json > "$TEST_ROOT/build-one.json"
"$ROOT_DIR/bin/sley" release build --output-dir "$OUT_TWO" --json > "$TEST_ROOT/build-two.json"

artifact_id="sley-1.2.1-linux-x86_64"
archive="$artifact_id.tar.gz"
tar -tzf "$OUT_ONE/$archive" | grep -Fx "$artifact_id/lib/sley/dispatch.sh" >/dev/null
tar -tzf "$OUT_ONE/$archive" | grep -Fx "$artifact_id/lib/sley/bootstrap.sh" >/dev/null

"$ROOT_DIR/bin/sley-contract" validate --schema-dir "$ROOT_DIR/docs/schemas" --schema sley.release.provenance.v1 "$TEST_ROOT/build-one.json" --json >/dev/null
"$ROOT_DIR/bin/sley-contract" validate --schema-dir "$ROOT_DIR/docs/schemas" --schema sley.release.manifest.v1 "$OUT_ONE/$artifact_id.manifest.json" --json >/dev/null
"$ROOT_DIR/bin/sley-contract" validate --schema-dir "$ROOT_DIR/docs/schemas" --schema sley.release.license_inventory.v1 "$OUT_ONE/$artifact_id.licenses.json" --json >/dev/null
jq -e '.spdxVersion == "SPDX-2.3" and .dataLicense == "CC0-1.0" and (.files | length) > 0' "$OUT_ONE/$artifact_id.sbom.spdx.json" >/dev/null

cmp "$OUT_ONE/$archive" "$OUT_TWO/$archive"
cmp "$OUT_ONE/$artifact_id.manifest.json" "$OUT_TWO/$artifact_id.manifest.json"
cmp "$OUT_ONE/$artifact_id.licenses.json" "$OUT_TWO/$artifact_id.licenses.json"
cmp "$OUT_ONE/$artifact_id.sbom.spdx.json" "$OUT_TWO/$artifact_id.sbom.spdx.json"

"$ROOT_DIR/bin/sley" release verify --output-dir "$OUT_ONE" --skip-client-tests --json > "$TEST_ROOT/verify.json"
"$ROOT_DIR/bin/sley-contract" validate --schema-dir "$ROOT_DIR/docs/schemas" --schema sley.release.verification.v1 "$TEST_ROOT/verify.json" --json >/dev/null
jq -e '.status == "passed" and .failed_count == 0 and (.checks | length) == 11' "$TEST_ROOT/verify.json" >/dev/null

cp "$OUT_ONE/$artifact_id.provenance.json" "$TEST_ROOT/provenance.original.json"
jq '.source.commit = "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa"' "$OUT_ONE/$artifact_id.provenance.json" > "$TEST_ROOT/provenance.stale.json"
mv "$TEST_ROOT/provenance.stale.json" "$OUT_ONE/$artifact_id.provenance.json"
if "$ROOT_DIR/bin/sley" release verify --output-dir "$OUT_ONE" --skip-client-tests --json > "$TEST_ROOT/stale.json"; then
  echo "release verification unexpectedly accepted a stale source commit" >&2
  exit 1
fi
jq -e '.status == "failed" and any(.checks[]; .id == "SOURCE_COMMIT_MATCH" and .status == "failed")' "$TEST_ROOT/stale.json" >/dev/null
cp "$TEST_ROOT/provenance.original.json" "$OUT_ONE/$artifact_id.provenance.json"

printf '%064d  %s\n' 0 "$archive" > "$OUT_ONE/$archive.sha256"
if "$ROOT_DIR/bin/sley" release verify --output-dir "$OUT_ONE" --skip-client-tests --json > "$TEST_ROOT/tampered.json"; then
  echo "release verification unexpectedly accepted a tampered checksum" >&2
  exit 1
fi
jq -e '.status == "failed" and any(.checks[]; .id == "ARCHIVE_DIGEST_MATCH" and .status == "failed")' "$TEST_ROOT/tampered.json" >/dev/null

python3 - "$OUT_TWO/$archive" <<'PY'
import io
import pathlib
import tarfile
import sys

archive = pathlib.Path(sys.argv[1])
with tarfile.open(archive, "w:gz") as handle:
    info = tarfile.TarInfo("../escape")
    content = b"denied\n"
    info.size = len(content)
    handle.addfile(info, io.BytesIO(content))
PY
if "$ROOT_DIR/bin/sley" release verify --output-dir "$OUT_TWO" --skip-client-tests --json > "$TEST_ROOT/unsafe.json"; then
  echo "release verification unexpectedly accepted an unsafe archive path" >&2
  exit 1
fi
jq -e '.status == "failed" and any(.checks[]; .id == "ARCHIVE_PATH_SAFE" and .status == "failed")' "$TEST_ROOT/unsafe.json" >/dev/null

echo "sley release artifact tests: passed"
