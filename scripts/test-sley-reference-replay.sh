#!/usr/bin/env bash
set -euo pipefail

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
WORK_DIR="$(mktemp -d "$ROOT_DIR/.sley-reference-test.XXXXXX")"
SIGLUM_ROOT="$WORK_DIR/siglum"
trap 'rm -rf "$WORK_DIR"' EXIT

mkdir -p \
  "$SIGLUM_ROOT/rules/sley/siglum-numerology-reference-v1/src/siglum/numerology" \
  "$SIGLUM_ROOT/rules/sley/siglum-numerology-reference-v1/src/sley/machine" \
  "$SIGLUM_ROOT/corpora/golden/siglum-numerology-reference-v1" \
  "$SIGLUM_ROOT/docs/rulesets" \
  "$SIGLUM_ROOT/src/lib/signature" \
  "$SIGLUM_ROOT/tests/corpus" \
  "$SIGLUM_ROOT/reports" \
  "$SIGLUM_ROOT/rulesets"

printf '%s\n' '[project]' 'name = "siglum.numerology"' >"$SIGLUM_ROOT/rules/sley/siglum-numerology-reference-v1/sley.toml"
printf '%s\n' 'module siglum.numerology.main' >"$SIGLUM_ROOT/rules/sley/siglum-numerology-reference-v1/src/siglum/numerology/main.sley"
printf '%s\n' 'module sley.machine.intrinsics' >"$SIGLUM_ROOT/rules/sley/siglum-numerology-reference-v1/src/sley/machine/intrinsics.sley"
printf '%s\n' '# frozen spec' >"$SIGLUM_ROOT/docs/rulesets/siglum-numerology-reference-v1.md"
printf '%s\n' 'export const boundary = true;' >"$SIGLUM_ROOT/src/lib/signature/numerology-reference-v1.ts"
printf '%s\n' 'export const adapter = true;' >"$SIGLUM_ROOT/src/lib/signature/reference-adapter.ts"
printf '%s\n' 'export const parity = true;' >"$SIGLUM_ROOT/tests/corpus/sley-numerology-parity.test.ts"
printf '%s\n' '{"ruleset":"fixture"}' >"$SIGLUM_ROOT/rulesets/siglum.numerology.reference-v1.json"

shard_rows="$WORK_DIR/shards.jsonl"
: >"$shard_rows"
for shard_index in $(seq 0 19); do
  first=$((shard_index * 500))
  last=$((first + 499))
  shard_name="cases-$(printf '%05d' "$first")-$(printf '%05d' "$last").jsonl"
  shard_path="$SIGLUM_ROOT/corpora/golden/siglum-numerology-reference-v1/$shard_name"
  : >"$shard_path"
  for case_offset in $(seq 0 499); do
    printf '{"case":%d}\n' "$((first + case_offset))" >>"$shard_path"
  done
  shard_digest="sha256:$(sha256sum "$shard_path" | cut -d' ' -f1)"
  shard_bytes="$(wc -c <"$shard_path" | tr -d ' ')"
  jq -cn \
    --arg path "$shard_name" \
    --arg digest "$shard_digest" \
    --argjson bytes "$shard_bytes" \
    '{path:$path,digest:$digest,bytes:$bytes,caseCount:500}' >>"$shard_rows"
done

total_bytes="$(find "$SIGLUM_ROOT/corpora/golden/siglum-numerology-reference-v1" -name 'cases-*.jsonl' -printf '%s\n' | awk '{sum += $1} END {print sum + 0}')"
frozen_digest="sha256:aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa"
jq -s \
  --arg corpus_digest "$frozen_digest" \
  --argjson total_bytes "$total_bytes" \
  '{artifacts:{caseCount:10000,corpusDigest:$corpus_digest,totalBytes:$total_bytes,shards:.}}' \
  "$shard_rows" >"$SIGLUM_ROOT/corpora/golden/siglum-numerology-reference-v1/manifest.json"
printf '%s\n' '{"hashes":"fixture"}' >"$SIGLUM_ROOT/corpora/golden/siglum-numerology-reference-v1/hashes.json"

source_digest="sha256:bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb"
candidate_digest="sha256:cccccccccccccccccccccccccccccccccccccccccccccccccccccccccccccccc"
retained_runtime="sha256:dddddddddddddddddddddddddddddddddddddddddddddddddddddddddddddddd"
fresh_runtime="sha256:eeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeee"
mismatch_digest="sha256:ffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffff"
shard_digests="$(jq -s '[.[].digest]' "$shard_rows")"
runs="$(jq -n \
  --arg source "$source_digest" \
  --arg corpus "$candidate_digest" \
  --arg runtime "$retained_runtime" \
  --arg mismatch "$mismatch_digest" \
  --argjson shards "$shard_digests" \
  '[1,2] | map({runIndex:.,caseCount:10000,exactMatchCount:10000,mismatchCount:0,mismatchReportDigest:$mismatch,candidateCorpusDigest:$corpus,shardDigests:$shards,sourceDigest:$source,runtimeDigest:$runtime,maximumSteps:8116,maximumCalls:250,maximumCallDepth:4,maximumResponseBytes:5127,elapsedMs:1000,casesPerSecond:10000})')"
jq -n \
  --argjson runs "$runs" \
  '{schemaVersion:"siglum.sley.numerology.parity-report.v1",deterministic:true,runs:$runs}' \
  >"$SIGLUM_ROOT/reports/sley-numerology-parity-0.json"

git -C "$SIGLUM_ROOT" init -q
git -C "$SIGLUM_ROOT" config user.name SleyFixture
git -C "$SIGLUM_ROOT" config user.email sley-fixture@example.invalid
git -C "$SIGLUM_ROOT" add .
git -C "$SIGLUM_ROOT" commit -qm fixture
siglum_commit="$(git -C "$SIGLUM_ROOT" rev-parse HEAD)"

artifact() {
  local id="$1" path="$2"
  jq -cn \
    --arg id "$id" \
    --arg path "$path" \
    --arg digest "sha256:$(sha256sum "$SIGLUM_ROOT/$path" | cut -d' ' -f1)" \
    '{id:$id,path:$path,digest:$digest}'
}

artifacts="$WORK_DIR/artifacts.jsonl"
: >"$artifacts"
artifact candidate_manifest rules/sley/siglum-numerology-reference-v1/sley.toml >>"$artifacts"
artifact candidate_main rules/sley/siglum-numerology-reference-v1/src/siglum/numerology/main.sley >>"$artifacts"
artifact candidate_intrinsics rules/sley/siglum-numerology-reference-v1/src/sley/machine/intrinsics.sley >>"$artifacts"
artifact corpus_hashes corpora/golden/siglum-numerology-reference-v1/hashes.json >>"$artifacts"
artifact corpus_manifest corpora/golden/siglum-numerology-reference-v1/manifest.json >>"$artifacts"
artifact normative_spec docs/rulesets/siglum-numerology-reference-v1.md >>"$artifacts"
artifact oracle_boundary src/lib/signature/numerology-reference-v1.ts >>"$artifacts"
artifact oracle_engine_adapter src/lib/signature/reference-adapter.ts >>"$artifacts"
artifact parity_harness tests/corpus/sley-numerology-parity.test.ts >>"$artifacts"
artifact prior_parity_report reports/sley-numerology-parity-0.json >>"$artifacts"
artifact ruleset rulesets/siglum.numerology.reference-v1.json >>"$artifacts"

pins="$WORK_DIR/pins.json"
jq -s \
  --arg commit "$siglum_commit" \
  --arg source "$source_digest" \
  --arg candidate "$candidate_digest" \
  --arg frozen "$frozen_digest" \
  --arg runtime "$retained_runtime" \
  '{
    schema:"sley.operational.reference_pins.v1",
    workload:{id:"siglum.numerology",version:"siglum.numerology.reference-v1",authority:"non_authoritative"},
    siglum_commit:$commit,
    paths:{
      candidate_package:"rules/sley/siglum-numerology-reference-v1",
      corpus_root:"corpora/golden/siglum-numerology-reference-v1",
      parity_test:"tests/corpus/sley-numerology-parity.test.ts",
      prior_report:"reports/sley-numerology-parity-0.json"
    },
    artifacts:.,
    expected:{
      case_count:10000,
      candidate_source_digest:$source,
      candidate_corpus_digest:$candidate,
      frozen_corpus_digest:$frozen,
      retained_runtime_digest:$runtime,
      retained_repeat_count:2,
      retained_exact_match_count:10000,
      retained_mismatch_count:0
    }
  }' "$artifacts" >"$pins"

fresh_runs="$(printf '%s\n' "$runs" | jq --arg runtime "$fresh_runtime" 'map(.runtimeDigest=$runtime)')"
parity="$WORK_DIR/fresh-parity.json"
jq -n \
  --argjson runs "$fresh_runs" \
  '{
    schemaVersion:"siglum.sley.numerology.parity-report.v1",
    deterministic:true,
    configuration:{shardTimeoutMs:120000,shardOutputLimitBytes:33554432},
    runs:$runs
  }' >"$parity"

probe="$WORK_DIR/probe.json"
jq -n \
  --arg source "$source_digest" \
  --arg runtime "$fresh_runtime" \
  '{
    schema:"sley.operational.reference_probe.v1",
    request_count:3,
    cold_first_response_ms:100,
    warm_response_ms:[2,2],
    peak_process_tree_rss_bytes:1048576,
    result_digests:[
      "sha256:1111111111111111111111111111111111111111111111111111111111111111",
      "sha256:1111111111111111111111111111111111111111111111111111111111111111",
      "sha256:1111111111111111111111111111111111111111111111111111111111111111"
    ],
    source_digest:$source,
    runtime_digest:$runtime,
    deterministic:true
  }' >"$probe"

report="$WORK_DIR/report.json"
"$ROOT_DIR/scripts/sley-reference-replay.sh" \
  --repo-root "$ROOT_DIR" \
  --siglum-root "$SIGLUM_ROOT" \
  --pins "$pins" \
  --assemble \
  --parity-report "$parity" \
  --probe-report "$probe" \
  --json >"$report"

"$ROOT_DIR/bin/sley-contract" validate \
  --schema sley.operational.reference_replay.v1 \
  "$report" \
  --schemas "$ROOT_DIR/docs/schemas" \
  --json >/dev/null
jq -e '
  .status == "passed"
  and .correctness == (.correctness | .repeat_count=2 | .deterministic=true | .exact_match_count=10000 | .mismatch_count=0)
  and .authority.mode == "local_replay_only"
  and .authority.candidate_authoritative == false
  and .promotion == {
    decision:"deferred",
    production_claim:false,
    missing_evidence:["production_shadow","production_latency","fallback","promotion_authority","rollback"]
  }
  and .host_boundary.persistent_probe == true
  and .disclosure.raw_corpus_embedded == false
' "$report" >/dev/null

tampered_pins="$WORK_DIR/tampered-pins.json"
jq '(.artifacts[] | select(.id == "ruleset") | .digest) = "sha256:0000000000000000000000000000000000000000000000000000000000000000"' \
  "$pins" >"$tampered_pins"
if "$ROOT_DIR/scripts/sley-reference-replay.sh" \
  --repo-root "$ROOT_DIR" \
  --siglum-root "$SIGLUM_ROOT" \
  --pins "$tampered_pins" \
  --assemble \
  --parity-report "$parity" \
  --probe-report "$probe" \
  --json >"$WORK_DIR/tampered.out" 2>"$WORK_DIR/tampered.err"; then
  echo "tampered reference pin unexpectedly passed" >&2
  exit 1
fi
grep -Fq 'ARTIFACT_DIGEST_MISMATCH: ruleset' "$WORK_DIR/tampered.err"

mismatched_parity="$WORK_DIR/mismatched-parity.json"
jq '.runs[1].mismatchCount = 1 | .runs[1].exactMatchCount = 9999' "$parity" >"$mismatched_parity"
if "$ROOT_DIR/scripts/sley-reference-replay.sh" \
  --repo-root "$ROOT_DIR" \
  --siglum-root "$SIGLUM_ROOT" \
  --pins "$pins" \
  --assemble \
  --parity-report "$mismatched_parity" \
  --probe-report "$probe" \
  --json >"$WORK_DIR/mismatch.out" 2>"$WORK_DIR/mismatch.err"; then
  echo "mismatched fresh parity unexpectedly passed" >&2
  exit 1
fi
grep -Fq 'FRESH_PARITY_INVALID' "$WORK_DIR/mismatch.err"

if "$ROOT_DIR/bin/sley" reference-replay siglum-numerology --json \
  >"$WORK_DIR/missing-root.out" 2>"$WORK_DIR/missing-root.err"; then
  echo "reference replay without an explicit Siglum root unexpectedly passed" >&2
  exit 1
fi
grep -Fq 'reference-replay requires --siglum-root PATH' "$WORK_DIR/missing-root.err"

if "$ROOT_DIR/bin/sley" reference-replay siglum-numerology \
  --siglum-root "$SIGLUM_ROOT" \
  --pins "$pins" \
  --assemble \
  --parity-report "$parity" \
  --probe-report "$probe" \
  --json >"$WORK_DIR/overridden-pins.out" 2>"$WORK_DIR/overridden-pins.err"; then
  echo "reference replay unexpectedly accepted a caller-supplied pin set" >&2
  exit 1
fi
grep -Fq 'reference-replay option is reserved for the Sley-owned replay contract: --pins' \
  "$WORK_DIR/overridden-pins.err"

if "$ROOT_DIR/bin/sley" reference-replay siglum-numerology \
  --siglum-root "$SIGLUM_ROOT" \
  --assemble \
  --parity-report "$parity" \
  --probe-report "$probe" \
  --json >"$WORK_DIR/wrapper.out" 2>"$WORK_DIR/wrapper.err"; then
  echo "fixture root unexpectedly matched the Sley-owned Siglum commit pin" >&2
  exit 1
fi
grep -Fq 'SIGLUM_COMMIT_MISMATCH' "$WORK_DIR/wrapper.err"

if "$ROOT_DIR/bin/sley" reference-replay unknown --json >"$WORK_DIR/unknown.out" 2>"$WORK_DIR/unknown.err"; then
  echo "unknown reference workload unexpectedly passed" >&2
  exit 1
fi
grep -Fq 'unknown Sley reference workload: unknown' "$WORK_DIR/unknown.err"

echo "Sley operational reference replay tests passed"
