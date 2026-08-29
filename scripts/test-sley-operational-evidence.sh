#!/usr/bin/env bash
set -euo pipefail

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
WORK_DIR="$(mktemp -d "$ROOT_DIR/.sley-operational-evidence-test.XXXXXX")"
trap 'rm -rf "$WORK_DIR"' EXIT

fail() {
  echo "operational evidence test failed: $*" >&2
  exit 1
}

expect_failure() {
  local code="$1"
  shift
  if "$@" >"$WORK_DIR/failure.out" 2>"$WORK_DIR/failure.err"; then
    fail "command unexpectedly passed; expected $code"
  fi
  grep -Fq "$code" "$WORK_DIR/failure.err" || fail "missing failure code $code"
}

BUNDLE_DIR="$WORK_DIR/private-bundle"
SLEY_OPERATIONAL_TEST_EXPORT_DIR="$BUNDLE_DIR" \
  "$ROOT_DIR/scripts/test-sley-operational-workflow.sh" >/dev/null

CONTROLLER="$BUNDLE_DIR/controller.py"
REFERENCE="$ROOT_DIR/reports/operational/siglum-numerology-reference-v1.json"
COMPARISON="$BUNDLE_DIR/comparison.json"
SOURCE_ARGS=(
  --raw-aggregate "$BUNDLE_DIR/raw-aggregate.json"
  --raw-evidence "$BUNDLE_DIR/raw-evidence"
  --structural-aggregate "$BUNDLE_DIR/structural-aggregate.json"
  --structural-evidence "$BUNDLE_DIR/structural-evidence"
  --approval-record "$BUNDLE_DIR/approval.json"
  --execution-provenance "$BUNDLE_DIR/execution-provenance.json"
)

REGISTRY="$WORK_DIR/registry.json"
"$ROOT_DIR/bin/sley" operational-evidence assemble \
  --reference-replay "$REFERENCE" \
  --agent-comparison "$COMPARISON" \
  --controller "$CONTROLLER" \
  "${SOURCE_ARGS[@]}" \
  --json >"$REGISTRY"

jq -e '
  .schema == "sley.operational.evidence_registry.v1"
  and .status == "decision_recorded"
  and .release == "1.2.1"
  and .entries.reference_replay.id == "siglum-numerology-reference-v1"
  and .entries.reference_replay.correctness == {
    repeat_count:2,
    case_count:10000,
    exact_match_count:10000,
    mismatch_count:0,
    deterministic:true,
    independent_oracle:true
  }
  and .entries.agent_workflow.id == "sley-agent-project-cross-module-rename-v1"
  and .entries.agent_workflow.comparison.trial_count == 1
  and .entries.agent_workflow.comparison.failures_retained == true
  and .entries.agent_workflow.arms.raw_source.mode == "K1"
  and .entries.agent_workflow.arms.sley_structural.mode == "K3"
  and .entries.agent_workflow.arms.raw_source.inference_attempts == 1
  and .entries.agent_workflow.arms.sley_structural.inference_attempts == 2
  and .entries.agent_workflow.promotion.decision == "deferred"
  and .decision.w6_evidence_complete == true
  and .decision.release_closeout_unblocked == true
  and .decision.production_promotion == "deferred"
  and .decision.general_claim_supported == false
  and .decision.production_claim == false
  and .authority.provider_calls == false
  and .authority.promotion_authority == false
  and .disclosure.training_eligible == false
  and .disclosure.retrieval_eligible == false
  and .disclosure.publication_authorized == false
  and (.entries.agent_workflow.known_limitations | index("SINGLE_TRIAL_ONLY")) != null
  and (.entries.agent_workflow.known_limitations | index("MANIFEST_LIMIT_ENFORCEMENT_PARTIAL")) != null
  and (.entries.agent_workflow.known_limitations | index("PROVIDER_ORIGIN_ASSERTION_NOT_CRYPTOGRAPHIC")) != null
  and (.identity.execution_provenance_digest | startswith("sha256:"))
  and (.identity.assembler_digest | startswith("sha256:"))
  and (.identity.comparison_assembler_digest | startswith("sha256:"))
  and (.identity.registry_schema_digest | startswith("sha256:"))
  and (.identity.schema_set_digest | startswith("sha256:"))
  and (.identity.operational_vocabulary_digest | startswith("sha256:"))
' "$REGISTRY" >/dev/null || fail "assembled registry did not preserve the W6 decision boundary"

"$ROOT_DIR/bin/sley-contract" validate \
  --schema sley.operational.evidence_registry.v1 \
  "$REGISTRY" \
  --schemas "$ROOT_DIR/docs/schemas" \
  --json >/dev/null

SECOND="$WORK_DIR/registry-second.json"
"$ROOT_DIR/bin/sley" operational-evidence assemble \
  --reference-replay "$REFERENCE" \
  --agent-comparison "$COMPARISON" \
  --controller "$CONTROLLER" \
  "${SOURCE_ARGS[@]}" \
  --json >"$SECOND"
cmp -s "$REGISTRY" "$SECOND" || fail "registry assembly was not deterministic"

if grep -Fq "$CONTROLLER" "$REGISTRY"; then
  fail "registry exposed the private controller path"
fi

expect_failure 'AGENT_COMPARISON_SYNTHETIC' \
  "$ROOT_DIR/bin/sley" operational-evidence assemble \
  --reference-replay "$REFERENCE" \
  --agent-comparison "$ROOT_DIR/fixtures/contracts/operational_agent_workflow_comparison_synthetic.json" \
  --controller "$CONTROLLER" \
  "${SOURCE_ARGS[@]}" \
  --json

TAMPERED="$WORK_DIR/tampered.json"
jq '.identity.case_manifest_digest = "sha256:aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa"' "$COMPARISON" >"$TAMPERED"
expect_failure 'AGENT_COMPARISON_IDENTITY_MISMATCH' \
  "$ROOT_DIR/bin/sley" operational-evidence assemble \
  --reference-replay "$REFERENCE" \
  --agent-comparison "$TAMPERED" \
  --controller "$CONTROLLER" \
  "${SOURCE_ARGS[@]}" \
  --json

NON_ANCESTOR="$WORK_DIR/non-ancestor.json"
jq '.identity.sley_commit = "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa"' "$COMPARISON" >"$NON_ANCESTOR"
expect_failure 'EVIDENCE_COMMIT_NOT_ANCESTOR: agent workflow' \
  "$ROOT_DIR/bin/sley" operational-evidence assemble \
  --reference-replay "$REFERENCE" \
  --agent-comparison "$NON_ANCESTOR" \
  --controller "$CONTROLLER" \
  "${SOURCE_ARGS[@]}" \
  --json

REFERENCE_MISMATCH="$WORK_DIR/reference-mismatch.json"
jq '.promotion.missing_evidence = .promotion.missing_evidence[:-1]' "$REFERENCE" >"$REFERENCE_MISMATCH"
expect_failure 'REFERENCE_EVIDENCE_INVALID' \
  "$ROOT_DIR/bin/sley" operational-evidence assemble \
  --reference-replay "$REFERENCE_MISMATCH" \
  --agent-comparison "$COMPARISON" \
  --controller "$CONTROLLER" \
  "${SOURCE_ARGS[@]}" \
  --json

ln -s "$CONTROLLER" "$WORK_DIR/controller-link.py"
expect_failure 'INPUT_PATH_INVALID: controller' \
  "$ROOT_DIR/bin/sley" operational-evidence assemble \
  --reference-replay "$REFERENCE" \
  --agent-comparison "$COMPARISON" \
  --controller "$WORK_DIR/controller-link.py" \
  "${SOURCE_ARGS[@]}" \
  --json

MISMATCHED_SOURCE="$WORK_DIR/mismatched-source.json"
jq '.cases[0].measurements.input_tokens += 1' "$BUNDLE_DIR/raw-aggregate.json" >"$MISMATCHED_SOURCE"
MISMATCHED_ARGS=(
  --raw-aggregate "$MISMATCHED_SOURCE"
  --raw-evidence "$BUNDLE_DIR/raw-evidence"
  --structural-aggregate "$BUNDLE_DIR/structural-aggregate.json"
  --structural-evidence "$BUNDLE_DIR/structural-evidence"
  --approval-record "$BUNDLE_DIR/approval.json"
  --execution-provenance "$BUNDLE_DIR/execution-provenance.json"
)
expect_failure 'AGENT_COMPARISON_REVALIDATION_FAILED' \
  "$ROOT_DIR/bin/sley" operational-evidence assemble \
  --reference-replay "$REFERENCE" \
  --agent-comparison "$COMPARISON" \
  --controller "$CONTROLLER" \
  "${MISMATCHED_ARGS[@]}" \
  --json

if "$ROOT_DIR/bin/sley" operational-evidence assemble \
  --repo-root "$ROOT_DIR" \
  --reference-replay "$REFERENCE" \
  --agent-comparison "$COMPARISON" \
  --controller "$CONTROLLER" \
  "${SOURCE_ARGS[@]}" \
  --json >"$WORK_DIR/reserved.out" 2>"$WORK_DIR/reserved.err"; then
  fail "caller-controlled repository root unexpectedly passed"
fi
grep -Fq 'operational-evidence option is reserved for the Sley-owned contract' "$WORK_DIR/reserved.err" \
  || fail "reserved repository-root denial was not typed"

echo "sley operational evidence tests passed"
