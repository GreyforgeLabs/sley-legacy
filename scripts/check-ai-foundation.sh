#!/usr/bin/env bash
set -euo pipefail

repo_root="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
bootstrap="$repo_root/SLEY_AI.md"
ai_spec="$repo_root/docs/SleyAISpec.md"
bench_spec="$repo_root/docs/SleyBenchSpec.md"
corpus_spec="$repo_root/docs/SleyCorpusSpec.md"
audit="$repo_root/docs/SleyAIAudit.md"
language_source="$repo_root/self-hosted/src/loom/bootstrap.sley"

required_files=(
  "$bootstrap"
  "$ai_spec"
  "$bench_spec"
  "$corpus_spec"
  "$audit"
  "$repo_root/docs/SleyLanguageSpec.md"
  "$repo_root/docs/contracts.md"
  "$repo_root/docs/AgentQuickstart.md"
  "$repo_root/docs/schemas"
  "$repo_root/docs/schemas/sley.agent_bench.case_manifest.v0.schema.json"
  "$repo_root/docs/schemas/sley.agent_bench.run_manifest.v0.schema.json"
  "$repo_root/docs/schemas/sley.agent_bench.event.v0.schema.json"
  "$repo_root/docs/schemas/sley.agent_bench.case_result.v0.schema.json"
  "$repo_root/docs/schemas/sley.agent_bench.aggregate.v0.schema.json"
  "$repo_root/docs/schemas/sley.agent_bench.split_manifest.v0.schema.json"
  "$repo_root/docs/schemas/sley.agent_bench.split_report.v0.schema.json"
  "$repo_root/docs/schemas/sley.corpus.record.v0.schema.json"
  "$repo_root/docs/schemas/sley.corpus.exclusion_ledger.v0.schema.json"
  "$repo_root/docs/schemas/sley.corpus.manifest.v0.schema.json"
  "$repo_root/docs/schemas/sley.corpus.audit_report.v0.schema.json"
  "$repo_root/docs/schemas/sley.corpus.deletion_plan.v0.schema.json"
  "$repo_root/docs/schemas/sley.corpus.audit_event_input.v0.schema.json"
  "$repo_root/docs/schemas/sley.corpus.audit_event.v0.schema.json"
  "$repo_root/docs/schemas/sley.corpus.mutation_audit_report.v0.schema.json"
  "$repo_root/fixtures/sleybench/smoke-v0/case-manifest.json"
  "$repo_root/fixtures/sleybench/smoke-v0/run-manifest.json"
  "$repo_root/scripts/generate-sleybench-smoke.sh"
  "$repo_root/scripts/sleybench-evaluator.sh"
  "$repo_root/scripts/test-sleybench-evaluator.sh"
  "$repo_root/bin/sleybench-split"
  "$repo_root/scripts/test-sleybench-split.sh"
  "$repo_root/fixtures/sleybench/baseline-v0/split.json"
  "$repo_root/docs/SleyBenchBaseline20260823.md"
  "$repo_root/fixtures/sleybench/baseline-v0/public/case-manifest.json"
  "$repo_root/fixtures/sleybench/baseline-v0/public/exclusions.json"
  "$repo_root/bin/sley-corpus"
  "$repo_root/scripts/test-sley-corpus.sh"
)

for required in "${required_files[@]}"; do
  if [[ ! -e "$required" ]]; then
    printf 'missing AI foundation path: %s\n' "$required" >&2
    exit 1
  fi
done

baseline_split="$repo_root/fixtures/sleybench/baseline-v0/split.json"
baseline_manifest="$repo_root/fixtures/sleybench/baseline-v0/public/case-manifest.json"
if ! jq -e '
  .suite_id == "sleybench-v0-public" and .tier == "baseline" and
  (.cases | length) == 96 and ([.cases[].id] | unique | length) == 96 and
  ([.cases | group_by(.family)[] | length] | all(. == 16)) and
  all(.cases[]; .visibility == "public" and .policy.kind == "fixture_candidate")
' "$baseline_manifest" >/dev/null; then
  printf 'SleyBench public baseline is not sixteen public cases per family\n' >&2
  exit 1
fi
"$repo_root/bin/sleybench-split" verify-public --split "$baseline_split" --json |
  jq -e '.status == "passed" and .counts == {public:96,private:24,total:120}' >/dev/null

case_manifest="$repo_root/fixtures/sleybench/smoke-v0/case-manifest.json"
run_manifest="$repo_root/fixtures/sleybench/smoke-v0/run-manifest.json"
if ! jq -e '
  .schema == "sley.agent_bench.case_manifest.v0" and
  .suite_id == "sleybench-smoke-v0" and
  .tier == "smoke" and
  (.cases | length) == 24 and
  ([.cases[].id] | unique | length) == 24 and
  ([.cases[].family] | unique | length) == 6 and
  ([.cases | group_by(.family)[] | length] | all(. == 4)) and
  all(.cases[]; .visibility == "public" and .policy.kind == "fixture_candidate")
' "$case_manifest" >/dev/null; then
  printf 'SleyBench smoke manifest is not four public cases per family\n' >&2
  exit 1
fi

case_digest="sha256:$(sha256sum "$case_manifest" | awk '{print $1}')"
bootstrap_digest="sha256:$(sha256sum "$bootstrap" | awk '{print $1}')"
if [[ "$(jq -r '.case_manifest_digest' "$run_manifest")" != "$case_digest" \
  || "$(jq -r '.bootstrap.digest' "$run_manifest")" != "$bootstrap_digest" ]]; then
  printf 'SleyBench run manifest digest pins are stale\n' >&2
  exit 1
fi

implementation_version="$(sed -n 's/^[[:space:]]*return "sley \([^"]*\)"$/\1/p' "$language_source" | head -n 1)"
bootstrap_version="$(sed -n 's/^Sley version: \([^[:space:]]*\).*/\1/p' "$bootstrap" | head -n 1)"

if [[ -z "$implementation_version" || -z "$bootstrap_version" ]]; then
  printf 'could not resolve Sley implementation/bootstrap version\n' >&2
  exit 1
fi

if [[ "$implementation_version" != "$bootstrap_version" ]]; then
  printf 'SLEY_AI.md version %s does not match compiler version %s\n' \
    "$bootstrap_version" "$implementation_version" >&2
  exit 1
fi

line_count="$(wc -l < "$bootstrap")"
if (( line_count > 240 )); then
  printf 'SLEY_AI.md exceeds 240-line context budget: %s\n' "$line_count" >&2
  exit 1
fi

required_bootstrap_text=(
  'docs/SleyLanguageSpec.md'
  'docs/contracts.md'
  'docs/schemas/'
  'docs/AgentQuickstart.md'
  'docs/SleyAISpec.md'
  'docs/SleyBenchSpec.md'
  'docs/SleyCorpusSpec.md'
  'sley-corpus'
  'audit-log'
  'sleybench-split'
  'sley.diagnostics.report.v0'
  'sley.edit_plan.report.v0'
  'sley.graft.outcome.v0'
  'Sley uses symbolic Boolean operators: `&&` for and, `||` for or, and `!` for'
  'return if ready { 1 } else { 0 }'
  'The stage-1 checker may classify unsupported foreign expression syntax as a'
  'scripts/check-ai-foundation.sh'
  'make v1'
)

for needle in "${required_bootstrap_text[@]}"; do
  if ! grep -Fq "$needle" "$bootstrap"; then
    printf 'SLEY_AI.md missing required text: %s\n' "$needle" >&2
    exit 1
  fi
done

for checked_doc in "$bootstrap" "$ai_spec" "$bench_spec" "$corpus_spec" "$audit"; do
  if grep -En '(^|[^A-Za-z])(TODO|TBD|FIXME|PLACEHOLDER)([^A-Za-z]|$)' "$checked_doc" >/dev/null; then
    printf 'unfinished marker in AI foundation document: %s\n' "$checked_doc" >&2
    exit 1
  fi
done

printf 'Sley AI foundation ready: version=%s bootstrap_lines=%s\n' \
  "$implementation_version" "$line_count"
