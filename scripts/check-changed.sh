#!/usr/bin/env bash
set -euo pipefail

ROOT_DIR="$(cd "$(dirname "$0")/.." && pwd)"
cd "$ROOT_DIR"

plan_only=0
targets_only=0
plan_json=0
base_ref="${CHECK_CHANGED_BASE:-}"
declare -a explicit_files=()

usage() {
  cat <<'EOF'
Usage: scripts/check-changed.sh [--plan] [--targets-only] [--plan-json] [--base REF] [--files PATH...]

Without --files, inspect the branch diff, staged and unstaged changes, and
untracked files. The dispatcher never invokes the full `make v1` gate.
EOF
}

while (($#)); do
  case "$1" in
    --plan)
      plan_only=1
      shift
      ;;
    --targets-only)
      targets_only=1
      plan_only=1
      shift
      ;;
    --plan-json)
      plan_json=1
      plan_only=1
      shift
      ;;
    --base)
      [[ $# -ge 2 ]] || { echo "--base requires a ref" >&2; exit 2; }
      base_ref="$2"
      shift 2
      ;;
    --files)
      shift
      explicit_files=("$@")
      break
      ;;
    --help|-h)
      usage
      exit 0
      ;;
    *)
      echo "unknown argument: $1" >&2
      usage >&2
      exit 2
      ;;
  esac
done

declare -a changed_files=()

collect_git_changes() {
  local comparison_base="$base_ref"
  local upstream=""

  if [[ -z "$comparison_base" ]]; then
    upstream="$(git rev-parse --abbrev-ref --symbolic-full-name '@{upstream}' 2>/dev/null || true)"
    if [[ -n "$upstream" ]]; then
      comparison_base="$(git merge-base HEAD "$upstream")"
    fi
  fi

  {
    if [[ -n "$comparison_base" ]]; then
      git diff --name-only "$comparison_base"...HEAD 2>/dev/null || git diff --name-only "$comparison_base" HEAD
    fi
    git diff --name-only
    git diff --cached --name-only
    git ls-files --others --exclude-standard
  } | awk 'NF && !seen[$0]++'
}

if ((${#explicit_files[@]})); then
  declare -A seen_explicit_files=()
  for path in "${explicit_files[@]}"; do
    if [[ -z "${seen_explicit_files[$path]:-}" ]]; then
      changed_files+=("$path")
      seen_explicit_files["$path"]=1
    fi
  done
else
  mapfile -t changed_files < <(collect_git_changes)
fi

if ((${#changed_files[@]} == 0)); then
  if ((plan_json)); then
    jq -n '{
      schema:"sley.validation.changed_plan.v1",
      changed_files:[],
      selected_subsystems:[],
      selected_targets:[],
      release_gate:{required:false,reason:"not required because no changes were detected"}
    }'
    exit 0
  fi
  if ((targets_only)); then
    exit 0
  fi
  cat <<'EOF'
Changed subsystems:
  none

Validation plan:
  none - no branch or working-tree changes detected

Full V1 validation:
  skipped - not required because no changes were detected
EOF
  exit 0
fi

declare -A subsystems=()
declare -A targets=()
full_gate_reason=""

add_subsystem() {
  subsystems["$1"]=1
}

add_target() {
  targets["$1"]=1
}

add_target diff-check

for path in "${changed_files[@]}"; do
  matched=0

  case "$path" in
    Makefile|.github/actions/sley-v1/*|.github/workflows/v1.yml|scripts/check-changed.sh|scripts/test-check-changed.sh|scripts/test-core-focus.sh)
      add_subsystem validation-architecture
      add_target fmt
      add_target syntax
      add_target validation-planner
      add_target validation-reports
      add_target parser-smoke
      add_target checker-smoke
      add_target runtime-smoke
      add_target lint-smoke
      add_target smoke
      full_gate_reason="required once at integration because the validation architecture changed"
      matched=1
      ;;
  esac

  case "$path" in
    bin/*|scripts/*.sh)
      add_subsystem shell-tooling
      add_target fmt
      matched=1
      ;;
  esac

  case "$path" in
    bin/sley|lib/sley/*|scripts/test-sley-cli-golden.sh|scripts/test-sley-cli-modules.sh|fixtures/golden/cli/*)
      add_subsystem cli-modules
      add_target cli-modules
      add_target fmt
      matched=1
      ;;
  esac

  case "$path" in
    scripts/check-self-hosted-code.sh)
      add_subsystem self-hosted-core
      add_target syntax
      matched=1
      ;;
    scripts/self-hosting-inventory.sh)
      add_subsystem self-hosted-core
      add_target syntax
      add_target parser-smoke
      add_target checker-smoke
      matched=1
      ;;
  esac

  case "$path" in
    self-hosted/src/loom/parser.sley)
      add_subsystem parser
      add_target syntax
      add_target parser-smoke
      matched=1
      ;;
    self-hosted/src/loom/checker.sley)
      add_subsystem checker
      add_target syntax
      add_target parser-smoke
      add_target checker-smoke
      matched=1
      ;;
    self-hosted/src/loom/runtime.sley)
      add_subsystem runtime
      add_target syntax
      add_target parser-smoke
      add_target checker-smoke
      add_target runtime-smoke
      matched=1
      ;;
    self-hosted/src/loom/lint.sley)
      add_subsystem lint
      add_target syntax
      add_target parser-smoke
      add_target checker-smoke
      add_target lint-smoke
      matched=1
      ;;
    self-hosted/src/loom/*.sley|self-hosted/sley.toml)
      add_subsystem self-hosted-core
      add_target syntax
      add_target parser-smoke
      add_target checker-smoke
      add_target runtime-smoke
      add_target lint-smoke
      matched=1
      ;;
  esac

  case "$path" in
    self-hosted/src/loom/transaction.sley|docs/schemas/sley.transaction.*|docs/schemas/sley.change.*|fixtures/contracts/transaction_*|fixtures/contracts/change_*|fixtures/transactions/*|scripts/test-sley-transaction-inspect.sh|scripts/test-sley-change-preview.sh|scripts/test-sley-change-apply.sh|scripts/test-sley-change-review.sh)
      add_subsystem transaction-contracts
      add_target transaction-contracts
      add_target contracts
      add_target contract-compatibility
      add_target conformance
      matched=1
      ;;
    self-hosted/src/loom/adapter.sley|docs/schemas/sley.adapter.*|fixtures/contracts/adapter_*|scripts/sley-adapter-replay.sh|scripts/test-sley-adapter-replay.sh)
      add_subsystem adapter-replay
      add_target adapter-replay
      add_target contracts
      add_target conformance
      matched=1
      ;;
    self-hosted/src/loom/worker.sley|docs/schemas/sley.worker.*|fixtures/contracts/worker_*|scripts/sley-worker-local.sh|scripts/test-sley-worker.sh)
      add_subsystem worker
      add_target worker
      add_target worker-clients
      add_target contracts
      add_target conformance
      matched=1
      ;;
    self-hosted/src/loom/testing.sley|docs/schemas/sley.test.*|docs/schemas/sley.review.packet.v1.schema.json|fixtures/user_tests/*|fixtures/user_tests/**/*|fixtures/contracts/test_*|fixtures/contracts/review_packet_*|scripts/sley-test.sh|scripts/test-sley-user-tests.sh)
      add_subsystem user-tests
      add_target user-tests
      add_target contracts
      add_target conformance
      matched=1
      ;;
    self-hosted/src/loom/validation.sley|docs/schemas/sley.validation.*|fixtures/contracts/validation_*|scripts/sley-validate.sh|scripts/test-sley-validation.sh|docs/v1.2/ADR-012-validation-profiles-and-reports.md)
      add_subsystem validation-architecture
      add_target validation-reports
      add_target validation-planner
      add_target contracts
      add_target conformance
      full_gate_reason="required once at integration because the validation architecture changed"
      matched=1
      ;;
    docs/schemas/sley.operational.reference_*|fixtures/contracts/operational_reference_*|fixtures/siglum/numerology-reference-v1/*|scripts/sley-reference-replay.sh|scripts/test-sley-reference-replay.sh|reports/operational/siglum-numerology-reference-v1.json|docs/v1.2/ADR-013-*)
      add_subsystem operational-evidence
      add_target operational-replay
      add_target contracts
      add_target conformance
      matched=1
      ;;
    self-hosted/src/loom/operational.sley|docs/schemas/sley.operational.agent_*|fixtures/contracts/operational_agent_*|fixtures/operational/agent-workflow-v1/*|fixtures/operational/agent-workflow-v1/**/*|scripts/sley-operational-workflow.sh|scripts/test-sley-operational-workflow.sh|docs/v1.2/ADR-014-*)
      add_subsystem operational-evidence
      add_target operational-workflow
      add_target operational-evidence
      add_target contracts
      add_target conformance
      matched=1
      ;;
    docs/schemas/sley.operational.evidence_*|fixtures/contracts/operational_evidence_*|scripts/sley-operational-evidence.sh|scripts/test-sley-operational-evidence.sh|reports/operational/evidence-registry-v1.json|docs/v1.2/ADR-015-*)
      add_subsystem operational-evidence
      add_target operational-evidence
      add_target contracts
      add_target conformance
      matched=1
      ;;
    docs/v1.2/OPERATIONAL_EVIDENCE.md)
      add_subsystem operational-evidence
      add_target operational-replay
      add_target operational-workflow
      add_target operational-evidence
      add_target contracts
      add_target conformance
      matched=1
      ;;
    self-hosted/src/loom/release.sley|docs/schemas/sley.release.*|docs/schemas/sley.toolchain.doctor.*|fixtures/contracts/release_*|fixtures/contracts/toolchain_doctor_*|scripts/sley-release.sh|scripts/test-sley-release.sh|docs/v1.2/RELEASE_ARTIFACTS.md|docs/v1.2/THREAT_MODEL.md)
      add_subsystem release-artifact
      add_target release-contracts
      add_target contracts
      add_target contract-compatibility
      add_target conformance
      full_gate_reason="required once at integration because the release artifact or security boundary changed"
      matched=1
      ;;
    clients/generate-worker-clients.py|clients/python/*|clients/python/**/*|clients/node/*|clients/node/**/*|scripts/test-sley-worker-clients.sh)
      add_subsystem worker-clients
      add_target worker-clients
      add_target contracts
      add_target conformance
      matched=1
      ;;
    docs/schemas/*|fixtures/contracts/*|bin/sley-contract|bin/sley-conformance)
      add_subsystem contracts
      add_target contracts
      add_target contract-compatibility
      add_target conformance
      matched=1
      ;;
    docs/v1.2/CONTRACT_STABILITY.json|docs/v1.2/ADR-002-contract-stability-floor.md|scripts/test-contract-compatibility.sh)
      add_subsystem contracts
      add_target contract-compatibility
      matched=1
      ;;
    docs/SleyClaimManifest.json)
      add_subsystem claims
      add_target claim-audit
      matched=1
      ;;
    fixtures/corpus/*)
      add_subsystem corpus
      add_target corpus
      matched=1
      ;;
    fixtures/ci_smoke_probe/*|fixtures/cli_smokes/*)
      add_subsystem smoke
      add_target smoke
      matched=1
      ;;
  esac

  case "$path" in
    SLEY_AI.md|docs/SleyAIAudit.md|docs/SleyAISpec.md|docs/SleyBenchSpec.md|docs/SleyCorpusSpec.md|scripts/check-ai-foundation.sh)
      add_subsystem ai-foundation
      add_target ai-foundation
      matched=1
      ;;
    bin/sleybench-split|scripts/sleybench-evaluator.sh|scripts/test-sleybench-evaluator.sh|scripts/test-sleybench-split.sh|scripts/test-sleybench-protocol.sh|scripts/test-sleybench-mode-summary.sh|scripts/generate-sleybench-smoke.sh|fixtures/sleybench/*|docs/schemas/sley.agent_bench.*|docs/schemas/greyforge.sleybench.*)
      add_subsystem agent-bench
      add_target agent-bench
      add_target protocol-contracts
      matched=1
      ;;
    scripts/machine-test.sh|fixtures/machine/*|fixtures/contracts/machine_*|docs/schemas/sley.machine.*)
      add_subsystem machine
      add_target machine-test
      matched=1
      ;;
  esac

  case "$path" in
    bin/sley-corpus|scripts/test-sley-corpus.sh|fixtures/corpus_governance/*|docs/schemas/sley.corpus.*)
      add_subsystem corpus-governance
      add_target corpus-governance
      add_target contracts
      matched=1
      ;;
  esac

  case "$path" in
    bin/sley-lsp|editors/*|tree-sitter-sley/*)
      add_subsystem lsp-editor
      add_target lsp
      add_target editor-shims
      matched=1
      ;;
    bin/sley-migrate|fixtures/contracts/migrate_*|examples/raw_host_migration.sley|examples/unqualified_import_call_project/*|examples/unchecked_result*.sley)
      add_subsystem migration
      add_target migrate
      matched=1
      ;;
    bin/sley-docgen|fixtures/contracts/docgen_*)
      add_subsystem docgen
      add_target docgen
      matched=1
      ;;
    bin/sley-sandbox-runner|fixtures/contracts/sandbox_* )
      add_subsystem sandbox
      add_target sandbox-runner
      matched=1
      ;;
    bin/sley-shadow|fixtures/contracts/shadow_*)
      add_subsystem shadow
      add_target shadow
      matched=1
      ;;
    bin/sley-zjx|fixtures/contracts/zjx_*|docs/SleyZJX*)
      add_subsystem zjx
      add_target zjx-tools
      matched=1
      ;;
    bin/sley-arena|examples/sley_arena/*|docs/SleyArena.md|sleyarena.txt)
      add_subsystem arena
      add_target arena
      matched=1
      ;;
    bin/sley-agent-bench|bin/sleybench-split|fixtures/contracts/agent_bench_*)
      add_subsystem agent-bench
      add_target agent-bench
      matched=1
      ;;
    bin/sley-workbench|fixtures/contracts/workbench_*)
      add_subsystem workbench
      add_target workbench
      matched=1
      ;;
    bin/sley-mcp-bridge|scripts/test-sley-mcp-*.sh|docs/SleyMcpBridge.md)
      add_subsystem mcp-bridge
      add_target mcp-bridge
      matched=1
      ;;
    scripts/test-sley-graph-diff.sh|fixtures/contracts/graph_diff_*|docs/SleyGraphDiffResearch.md)
      add_subsystem graph-diff
      add_target graph-diff
      matched=1
      ;;
    bin/git-sley-guard|scripts/test-git-sley-guard.sh)
      add_subsystem git-guard
      add_target git-guard
      matched=1
      ;;
    fixtures/grafts/*)
      add_subsystem git-guard
      add_target git-guard
      matched=1
      ;;
  esac

  case "$path" in
    scripts/self-hosted-test.sh)
      add_subsystem core-cli
      add_target test
      add_target smoke
      matched=1
      ;;
    bin/sley-ci)
      add_subsystem ci-runner
      add_target corpus
      add_target examples
      add_target smoke
      matched=1
      ;;
    examples/*)
      add_subsystem examples
      add_target examples
      matched=1
      ;;
    README.md|AGENTS.md|CHANGELOG.md|LICENSE|NOTICE|llms.txt|docs/*.md|docs/resume/*|docs/ValidationTiers.md|.gitignore|.pre-commit-config.yaml)
      add_subsystem documentation
      matched=1
      ;;
  esac

  case "$path" in
    README.md|llms.txt|docs/SleyClaimEvidence.md)
      add_subsystem claims
      add_target claim-audit
      matched=1
      ;;
  esac

  if ((matched == 0)); then
    add_subsystem unclassified
    add_target fmt
    add_target syntax
    add_target test
    add_target contracts
    add_target conformance
    full_gate_reason="not selected automatically; assess Tier 3 because an unclassified path changed"
  fi
done

target_order=(
  diff-check fmt syntax validation-planner parser-smoke checker-smoke
  runtime-smoke lint-smoke test smoke contracts contract-compatibility
  protocol-contracts transaction-contracts adapter-replay worker worker-clients user-tests validation-reports operational-replay operational-workflow operational-evidence release-contracts conformance ai-foundation
  claim-audit corpus examples git-guard cli-modules mcp-bridge graph-diff lsp
  editor-shims workbench migrate docgen sandbox-runner shadow zjx-tools
  arena machine-test agent-bench corpus-governance
)

subsystem_order=(
  validation-architecture parser checker runtime lint self-hosted-core core-cli
  contracts claims corpus smoke ci-runner lsp-editor migration docgen sandbox
  shadow zjx arena machine agent-bench ai-foundation corpus-governance workbench mcp-bridge graph-diff git-guard cli-modules adapter-replay
  shell-tooling examples documentation unclassified worker worker-clients user-tests operational-evidence release-artifact
)

selected_targets=()
for target in "${target_order[@]}"; do
  if [[ -n "${targets[$target]:-}" ]]; then
    selected_targets+=("$target")
  fi
done

selected_subsystems=()
for subsystem in "${subsystem_order[@]}"; do
  if [[ -n "${subsystems[$subsystem]:-}" ]]; then
    selected_subsystems+=("$subsystem")
  fi
done

if ((plan_json)); then
  changed_files_json="$(printf '%s\n' "${changed_files[@]}" | jq -Rsc 'split("\n")[:-1]')"
  selected_subsystems_json="$(printf '%s\n' "${selected_subsystems[@]}" | jq -Rsc 'split("\n")[:-1]')"
  selected_targets_json="$(printf '%s\n' "${selected_targets[@]}" | jq -Rsc 'split("\n")[:-1]')"
  jq -n \
    --argjson changed_files "$changed_files_json" \
    --argjson selected_subsystems "$selected_subsystems_json" \
    --argjson selected_targets "$selected_targets_json" \
    --arg release_reason "$full_gate_reason" '{
      schema:"sley.validation.changed_plan.v1",
      changed_files:$changed_files,
      selected_subsystems:$selected_subsystems,
      selected_targets:$selected_targets,
      release_gate:{
        required:($release_reason != ""),
        reason:(if $release_reason == "" then "not required for the detected changes" else $release_reason end)
      }
    }'
  exit 0
fi

if ((targets_only)); then
  printf '%s\n' "${selected_targets[@]}"
  exit 0
fi

echo "Changed files:"
printf '  %s\n' "${changed_files[@]}"
echo
echo "Changed subsystems:"
for subsystem in "${selected_subsystems[@]}"; do
  printf '  %s\n' "$subsystem"
done
echo
echo "Validation plan:"
printf '  make %s\n' "${selected_targets[@]}"
echo
echo "Full V1 validation:"
if [[ -n "$full_gate_reason" ]]; then
  echo "  $full_gate_reason"
else
  echo "  skipped - not required for this change"
fi

if ((plan_only)); then
  exit 0
fi

for target in "${selected_targets[@]}"; do
  echo
  echo "==> make $target"
  make --no-print-directory "$target"
done
