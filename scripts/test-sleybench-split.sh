#!/usr/bin/env bash
set -euo pipefail

ROOT_DIR="$(cd "$(dirname "$0")/.." && pwd)"
SPLIT_TOOL="$ROOT_DIR/bin/sleybench-split"
CONTRACT="$ROOT_DIR/bin/sley-contract"
WORK_DIR="$(mktemp -d)"
trap 'rm -rf -- "$WORK_DIR"' EXIT

fail() {
  printf 'SleyBench split test failed: %s\n' "$*" >&2
  exit 1
}

expect_failure() {
  local name="$1"
  shift
  if "$@" >"$WORK_DIR/$name.json" 2>"$WORK_DIR/$name.err"; then
    fail "$name unexpectedly passed"
  fi
}

sha256_file() {
  printf 'sha256:%s\n' "$(sha256sum "$1" | awk '{print $1}')"
}

tree_digest() {
  local root="$1"
  (
    cd "$root"
    find . -type f -print0 | sort -z | while IFS= read -r -d '' file; do
      printf '%s\0' "${file#./}"
      sha256sum "$file" | awk '{print $1}'
    done
  ) | sha256sum | awk '{print "sha256:" $1}'
}

training_digest() {
  local prompt="$1" workspace="$2"
  { sha256_file "$prompt"; tree_digest "$workspace"; } \
    | sha256sum | awk '{print "sha256:" $1}'
}

families=(
  syntax_compilation semantic_implementation diagnosis_repair
  translation_preservation structural_tool_use repository_patch_integration
)

create_partition() {
  local root="$1" suite="$2" visibility="$3" per_family="$4" offset="$5"
  mkdir -p "$root/cases"
  jq -n --arg suite "$suite" '{schema:"sley.agent_bench.case_manifest.v0",suite_id:$suite,tier:"baseline",version:"0.1",cases:[]}' \
    >"$root/case-manifest.json"
  jq -n --arg ledger "$suite-exclusions" '{schema:"sley.corpus.exclusion_ledger.v0",ledger_id:$ledger,version:"0.1",benchmark_split_status:"pinned",entries:[]}' \
    >"$root/exclusions.json"

  local family_index family ordinal sequence id case_root digest candidate_digest lineage_id lineage_digest tmp
  local max_changed_files max_changed_lines
  for family_index in "${!families[@]}"; do
    family="${families[$family_index]}"
    for ((ordinal = 1; ordinal <= per_family; ordinal++)); do
      sequence=$((offset + ordinal))
      printf -v id 'sleybench-v0-f%d-%03d' "$((family_index + 1))" "$sequence"
      case_root="$root/cases/$id"
      mkdir -p "$case_root/workspace" "$case_root/candidate"
      printf 'Implement deterministic fixture %s.\n' "$id" >"$case_root/prompt.md"
      printf 'module bench.%s\n' "${id//-/_}" >"$case_root/workspace/main.sley"
      printf 'module bench.%s\n\ntask main -> Int {\n  return %d\n}\n' \
        "${id//-/_}" "$sequence" >"$case_root/candidate/main.sley"
      max_changed_files=1
      max_changed_lines=8
      if [[ "$family" == "structural_tool_use" && "$ordinal" == "1" ]]; then
        rm "$case_root/candidate/main.sley"
        rmdir "$case_root/candidate"
        max_changed_files=0
        max_changed_lines=0
      fi
      digest="$(training_digest "$case_root/prompt.md" "$case_root/workspace")"
      if [[ -d "$case_root/candidate" ]]; then
        candidate_digest="$(tree_digest "$case_root/candidate")"
      else
        candidate_digest="sha256:e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855"
      fi
      lineage_id="lineage-${id#sleybench-}"
      lineage_digest="sha256:$(printf '%s' "$lineage_id" | sha256sum | awk '{print $1}')"

      tmp="$root/case-manifest.tmp"
      jq \
        --arg id "$id" --arg family "$family" --arg visibility "$visibility" \
        --arg digest "$digest" --arg candidate_digest "$candidate_digest" \
        --arg lineage_id "$lineage_id" --arg lineage_digest "$lineage_digest" \
        --argjson max_changed_files "$max_changed_files" \
        --argjson max_changed_lines "$max_changed_lines" '
        .cases += [{
          id:$id,
          family:$family,
          visibility:$visibility,
          sley_version:"1.1",
          prompt_path:("cases/" + $id + "/prompt.md"),
          workspace_path:("cases/" + $id + "/workspace"),
          candidate_path:("cases/" + $id + "/candidate"),
          owned_paths:["main.sley"],
          allowed_context:["SLEY_AI.md"],
          allowed_tools:["check"],
          limits:{wall_seconds:120,tool_calls:8,output_bytes:1048576,processes:8,workspace_bytes:1048576},
          policy:{kind:"fixture_candidate"},
          preflight:[],
          oracle:{commands:[{name:"check",argv:["sley","check","--json","main.sley"],expected_exit:0,expected_schema:"sley.diagnostics.report.v0",assertions:[{pointer:"/status",value:true}]}]},
          score_tags:["compile"],
          minimality:{max_changed_files:$max_changed_files,max_changed_lines:$max_changed_lines},
          training_exclusion_id:$digest,
          candidate_digest:$candidate_digest,
          semantic_lineage_id:$lineage_id,
          semantic_lineage_digest:$lineage_digest
        }]
      ' "$root/case-manifest.json" >"$tmp"
      mv "$tmp" "$root/case-manifest.json"

      tmp="$root/exclusions.tmp"
      jq \
        --arg id "$id" --arg visibility "$visibility" --arg digest "$digest" \
        --arg lineage_digest "$lineage_digest" '
        .entries += [{
          exclusion_id:$digest,
          case_id:$id,
          visibility:$visibility,
          exact:[$digest],
          canonical:[$digest],
          structural:[$digest],
          semantic_lineage:[$lineage_digest]
        }]
      ' "$root/exclusions.json" >"$tmp"
      mv "$tmp" "$root/exclusions.json"
    done
  done
}

PUBLIC_ROOT="$WORK_DIR/public"
PRIVATE_ROOT="$WORK_DIR/private"
create_partition "$PUBLIC_ROOT" sleybench-v0-public public 16 0
create_partition "$PRIVATE_ROOT" sleybench-v0-private private_held_out 4 100

public_manifest_digest="$(sha256_file "$PUBLIC_ROOT/case-manifest.json")"
public_ledger_digest="$(sha256_file "$PUBLIC_ROOT/exclusions.json")"
private_manifest_digest="$(sha256_file "$PRIVATE_ROOT/case-manifest.json")"
private_ledger_digest="$(sha256_file "$PRIVATE_ROOT/exclusions.json")"

jq -n \
  --arg public_manifest "$public_manifest_digest" --arg public_ledger "$public_ledger_digest" \
  --arg private_manifest "$private_manifest_digest" --arg private_ledger "$private_ledger_digest" '
  {
    schema:"sley.agent_bench.split_manifest.v0",
    suite_id:"sleybench-v0",
    version:"0.1",
    state:"pinned",
    split_strategy:"semantic_lineage",
    public:{
      suite_id:"sleybench-v0-public",
      case_manifest_path:"public/case-manifest.json",
      case_manifest_digest:$public_manifest,
      exclusion_ledger_path:"public/exclusions.json",
      exclusion_ledger_digest:$public_ledger,
      case_count:96,
      family_counts:{syntax_compilation:16,semantic_implementation:16,diagnosis_repair:16,translation_preservation:16,structural_tool_use:16,repository_patch_integration:16}
    },
    private:{
      suite_id:"sleybench-v0-private",
      case_manifest_digest:$private_manifest,
      exclusion_ledger_digest:$private_ledger,
      case_count:24,
      family_counts:{syntax_compilation:4,semantic_implementation:4,diagnosis_repair:4,translation_preservation:4,structural_tool_use:4,repository_patch_integration:4},
      material_in_public_git:false,
      custody_ref:"fixture:test-private-custody"
    },
    approval:{status:"approved",reviewer:"fixture-reviewer",reviewed_at:"2026-08-23T00:00:00Z"}
  }
' >"$WORK_DIR/split.json"

"$SPLIT_TOOL" verify-public --split "$WORK_DIR/split.json" --json >"$WORK_DIR/public-1.json"
"$SPLIT_TOOL" verify-public --split "$WORK_DIR/split.json" --json >"$WORK_DIR/public-2.json"
cmp -s "$WORK_DIR/public-1.json" "$WORK_DIR/public-2.json" || fail "public verification is not deterministic"
jq -e '
  .status == "passed" and .verification_level == "public_commitment" and
  .private_material_verified == false and .counts == {public:96,private:24,total:120}
' "$WORK_DIR/public-1.json" >/dev/null
"$CONTRACT" validate --schema sley.agent_bench.split_report.v0 "$WORK_DIR/public-1.json" \
  --schemas "$ROOT_DIR/docs/schemas" --json | jq -e '.status == "passed"' >/dev/null

"$SPLIT_TOOL" audit --split "$WORK_DIR/split.json" \
  --private-manifest "$PRIVATE_ROOT/case-manifest.json" \
  --private-exclusions "$PRIVATE_ROOT/exclusions.json" --json >"$WORK_DIR/audit-1.json"
"$SPLIT_TOOL" audit --split "$WORK_DIR/split.json" \
  --private-manifest "$PRIVATE_ROOT/case-manifest.json" \
  --private-exclusions "$PRIVATE_ROOT/exclusions.json" --json >"$WORK_DIR/audit-2.json"
cmp -s "$WORK_DIR/audit-1.json" "$WORK_DIR/audit-2.json" || fail "full split audit is not deterministic"
jq -e '
  .status == "passed" and .verification_level == "full_private_audit" and
  .private_material_verified == true and .counts.total == 120 and .summary.failed_count == 0
' "$WORK_DIR/audit-1.json" >/dev/null
"$CONTRACT" validate --schema sley.agent_bench.split_report.v0 "$WORK_DIR/audit-1.json" \
  --schemas "$ROOT_DIR/docs/schemas" --json | jq -e '.status == "passed"' >/dev/null

jq '.unexpected = true' "$WORK_DIR/split.json" >"$WORK_DIR/unknown-split.json"
expect_failure unknown-field "$SPLIT_TOOL" verify-public --split "$WORK_DIR/unknown-split.json" --json
jq -e '.status == "failed" and ([.issues[].code] | index("json_schema_violation")) != null' \
  "$WORK_DIR/unknown-field.json" >/dev/null

jq 'del(.cases[0].semantic_lineage_digest)' "$PUBLIC_ROOT/case-manifest.json" >"$WORK_DIR/missing-lineage-case.json"
expect_failure missing-lineage-schema "$CONTRACT" validate --schema sley.agent_bench.case_manifest.v0 \
  "$WORK_DIR/missing-lineage-case.json" --schemas "$ROOT_DIR/docs/schemas" --json
jq -e '.status == "failed" and ([.issues[].code] | index("json_schema_violation")) != null' \
  "$WORK_DIR/missing-lineage-schema.json" >/dev/null

cp "$PRIVATE_ROOT/case-manifest.json" "$WORK_DIR/private-original.json"
cp "$PRIVATE_ROOT/exclusions.json" "$WORK_DIR/private-exclusions-original.json"
public_case_id="$(jq -r '.cases[0].id' "$PUBLIC_ROOT/case-manifest.json")"
jq --arg id "$public_case_id" '.cases[0].id = $id' \
  "$PRIVATE_ROOT/case-manifest.json" >"$WORK_DIR/private-id-overlap.json"
mv "$WORK_DIR/private-id-overlap.json" "$PRIVATE_ROOT/case-manifest.json"
jq --arg id "$public_case_id" '.entries[0].case_id = $id' \
  "$PRIVATE_ROOT/exclusions.json" >"$WORK_DIR/private-exclusions-id-overlap.json"
mv "$WORK_DIR/private-exclusions-id-overlap.json" "$PRIVATE_ROOT/exclusions.json"
jq \
  --arg manifest "$(sha256_file "$PRIVATE_ROOT/case-manifest.json")" \
  --arg ledger "$(sha256_file "$PRIVATE_ROOT/exclusions.json")" \
  '.private.case_manifest_digest = $manifest | .private.exclusion_ledger_digest = $ledger' \
  "$WORK_DIR/split.json" >"$WORK_DIR/id-overlap-split.json"
expect_failure case-id-overlap "$SPLIT_TOOL" audit --split "$WORK_DIR/id-overlap-split.json" \
  --private-manifest "$PRIVATE_ROOT/case-manifest.json" --private-exclusions "$PRIVATE_ROOT/exclusions.json" --json
jq -e '.status == "failed" and ([.issues[].code] | index("case_id_split_overlap")) != null' \
  "$WORK_DIR/case-id-overlap.json" >/dev/null
cp "$WORK_DIR/private-original.json" "$PRIVATE_ROOT/case-manifest.json"
cp "$WORK_DIR/private-exclusions-original.json" "$PRIVATE_ROOT/exclusions.json"

jq --arg id "$(jq -r '.cases[0].semantic_lineage_id' "$PUBLIC_ROOT/case-manifest.json")" \
  --arg digest "$(jq -r '.cases[0].semantic_lineage_digest' "$PUBLIC_ROOT/case-manifest.json")" \
  '.cases[0].semantic_lineage_id = $id | .cases[0].semantic_lineage_digest = $digest' \
  "$PRIVATE_ROOT/case-manifest.json" >"$WORK_DIR/private-overlap.json"
mv "$WORK_DIR/private-overlap.json" "$PRIVATE_ROOT/case-manifest.json"
jq --arg digest "$(jq -r '.cases[0].semantic_lineage_digest' "$PUBLIC_ROOT/case-manifest.json")" \
  '.entries[0].semantic_lineage = [$digest]' "$PRIVATE_ROOT/exclusions.json" >"$WORK_DIR/private-exclusions-overlap.json"
mv "$WORK_DIR/private-exclusions-overlap.json" "$PRIVATE_ROOT/exclusions.json"
jq \
  --arg manifest "$(sha256_file "$PRIVATE_ROOT/case-manifest.json")" \
  --arg ledger "$(sha256_file "$PRIVATE_ROOT/exclusions.json")" \
  '.private.case_manifest_digest = $manifest | .private.exclusion_ledger_digest = $ledger' \
  "$WORK_DIR/split.json" >"$WORK_DIR/overlap-split.json"
expect_failure semantic-overlap "$SPLIT_TOOL" audit --split "$WORK_DIR/overlap-split.json" \
  --private-manifest "$PRIVATE_ROOT/case-manifest.json" --private-exclusions "$PRIVATE_ROOT/exclusions.json" --json
jq -e '.status == "failed" and ([.issues[].code] | index("semantic_split_overlap")) != null' \
  "$WORK_DIR/semantic-overlap.json" >/dev/null
jq -e '.private_material_verified == false' "$WORK_DIR/semantic-overlap.json" >/dev/null
"$CONTRACT" validate --schema sley.agent_bench.split_report.v0 "$WORK_DIR/semantic-overlap.json" \
  --schemas "$ROOT_DIR/docs/schemas" --json | jq -e '.status == "passed"' >/dev/null

cp "$WORK_DIR/private-original.json" "$PRIVATE_ROOT/case-manifest.json"
expect_failure stale-private-pin "$SPLIT_TOOL" audit --split "$WORK_DIR/overlap-split.json" \
  --private-manifest "$PRIVATE_ROOT/case-manifest.json" --private-exclusions "$PRIVATE_ROOT/exclusions.json" --json
jq -e '.status == "failed" and ([.issues[].code] | index("private_commitment_mismatch")) != null' \
  "$WORK_DIR/stale-private-pin.json" >/dev/null

expect_failure public-repo-private "$SPLIT_TOOL" audit --split "$WORK_DIR/split.json" \
  --private-manifest "$ROOT_DIR/fixtures/sleybench/smoke-v0/case-manifest.json" \
  --private-exclusions "$ROOT_DIR/fixtures/corpus_governance/exclusions.json" --json
jq -e '.status == "failed" and ([.issues[].code] | index("private_material_in_public_repository")) != null' \
  "$WORK_DIR/public-repo-private.json" >/dev/null

expect_failure missing-private "$SPLIT_TOOL" audit --split "$WORK_DIR/split.json" \
  --private-manifest "$WORK_DIR/private-does-not-exist.json" \
  --private-exclusions "$WORK_DIR/ledger-does-not-exist.json" --json
if grep -Fq "$WORK_DIR/private-does-not-exist.json" "$WORK_DIR/missing-private.json"; then
  fail "private custody path leaked into the audit report"
fi

jq '.entries[0].canonical = []' "$PUBLIC_ROOT/exclusions.json" >"$WORK_DIR/empty-canonical.json"
mv "$WORK_DIR/empty-canonical.json" "$PUBLIC_ROOT/exclusions.json"
jq --arg digest "$(sha256_file "$PUBLIC_ROOT/exclusions.json")" \
  '.public.exclusion_ledger_digest = $digest' "$WORK_DIR/split.json" >"$WORK_DIR/empty-canonical-split.json"
expect_failure empty-canonical "$SPLIT_TOOL" verify-public --split "$WORK_DIR/empty-canonical-split.json" --json
jq -e '.status == "failed" and ([.issues[].code] | index("exclusion_coverage_mismatch")) != null' \
  "$WORK_DIR/empty-canonical.json" >/dev/null

printf 'SleyBench split governance tests passed.\n'
