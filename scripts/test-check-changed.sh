#!/usr/bin/env bash
set -euo pipefail

ROOT_DIR="$(cd "$(dirname "$0")/.." && pwd)"
DISPATCHER="$ROOT_DIR/scripts/check-changed.sh"

fail() {
  echo "check-changed classifier test failed: $*" >&2
  exit 1
}

assert_has() {
  local output="$1"
  local expected="$2"
  grep -Fxq "$expected" <<<"$output" || fail "expected target '$expected' in [$output]"
}

assert_lacks() {
  local output="$1"
  local unexpected="$2"
  if grep -Fxq "$unexpected" <<<"$output"; then
    fail "unexpected target '$unexpected' in [$output]"
  fi
}

plan() {
  "$DISPATCHER" --targets-only --files "$@"
}

parser_checker="$(plan self-hosted/src/loom/parser.sley self-hosted/src/loom/checker.sley)"
assert_has "$parser_checker" diff-check
assert_has "$parser_checker" syntax
assert_has "$parser_checker" parser-smoke
assert_has "$parser_checker" checker-smoke
assert_lacks "$parser_checker" test
assert_lacks "$parser_checker" arena
assert_lacks "$parser_checker" agent-bench

runtime="$(plan self-hosted/src/loom/runtime.sley)"
assert_has "$runtime" runtime-smoke
assert_lacks "$runtime" test
assert_lacks "$runtime" contracts

contracts="$(plan docs/schemas/sley.run.report.v0.schema.json fixtures/contracts/run_hello_ready.json)"
assert_has "$contracts" contracts
assert_has "$contracts" contract-compatibility
assert_has "$contracts" conformance
assert_lacks "$contracts" docgen

lsp="$(plan bin/sley-lsp editors/vscode-sley/package.json)"
assert_has "$lsp" lsp
assert_has "$lsp" editor-shims
assert_lacks "$lsp" migrate

tools="$(plan bin/sley-migrate bin/sley-docgen bin/sley-sandbox-runner bin/sley-shadow)"
assert_has "$tools" migrate
assert_has "$tools" docgen
assert_has "$tools" sandbox-runner
assert_has "$tools" shadow
assert_lacks "$tools" arena

specialized="$(plan bin/sley-zjx bin/sley-arena bin/sley-agent-bench)"
assert_has "$specialized" zjx-tools
assert_has "$specialized" arena
assert_has "$specialized" agent-bench

bench_split="$(plan bin/sleybench-split scripts/test-sleybench-split.sh docs/schemas/sley.agent_bench.split_manifest.v0.schema.json)"
assert_has "$bench_split" agent-bench
assert_has "$bench_split" protocol-contracts
assert_has "$bench_split" contracts
assert_has "$bench_split" conformance

machine="$(plan scripts/machine-test.sh fixtures/machine/package/sley.toml docs/schemas/sley.machine.response.v0.schema.json)"
assert_has "$machine" machine-test
assert_has "$machine" contracts
assert_has "$machine" conformance
assert_lacks "$machine" agent-bench

protocol="$(plan scripts/test-sleybench-protocol.sh fixtures/sleybench/protocol-v0/semantic/unowned_tool_target.json docs/schemas/greyforge.sleybench.mode_summary.v0.schema.json)"
assert_has "$protocol" protocol-contracts
assert_has "$protocol" agent-bench
assert_has "$protocol" contracts
assert_has "$protocol" contract-compatibility

transaction="$(plan self-hosted/src/loom/transaction.sley docs/schemas/sley.transaction.inspect.v0.schema.json fixtures/contracts/transaction_inspect_project.json scripts/test-sley-transaction-inspect.sh)"
assert_has "$transaction" transaction-contracts
assert_has "$transaction" contracts
assert_has "$transaction" contract-compatibility
assert_has "$transaction" conformance

review="$(plan docs/schemas/sley.change.review.v0.schema.json scripts/test-sley-change-review.sh)"
assert_has "$review" transaction-contracts
assert_has "$review" contracts
assert_has "$review" contract-compatibility
assert_has "$review" conformance

mcp="$(plan bin/sley-mcp-bridge scripts/test-sley-mcp-transaction.sh docs/SleyMcpBridge.md)"
assert_has "$mcp" mcp-bridge
assert_has "$mcp" fmt
assert_lacks "$mcp" transaction-contracts

adapter="$(plan self-hosted/src/loom/adapter.sley docs/schemas/sley.adapter.manifest.v0.schema.json fixtures/contracts/adapter_manifest_local_replay.json scripts/test-sley-adapter-replay.sh)"
assert_has "$adapter" adapter-replay
assert_has "$adapter" contracts
assert_has "$adapter" conformance

worker="$(plan self-hosted/src/loom/worker.sley docs/schemas/sley.worker.request.v1.schema.json fixtures/contracts/worker_request_invoke.json scripts/test-sley-worker.sh)"
assert_has "$worker" worker
assert_has "$worker" worker-clients
assert_has "$worker" contracts
assert_has "$worker" conformance

worker_clients="$(plan clients/python/sley_worker_client/client.py clients/node/index.mjs clients/generate-worker-clients.py scripts/test-sley-worker-clients.sh)"
assert_has "$worker_clients" worker-clients
assert_has "$worker_clients" contracts
assert_has "$worker_clients" conformance

user_tests="$(plan self-hosted/src/loom/testing.sley docs/schemas/sley.test.report.v1.schema.json fixtures/user_tests/sley.test.json scripts/test-sley-user-tests.sh)"
assert_has "$user_tests" user-tests
assert_has "$user_tests" contracts
assert_has "$user_tests" conformance

validation_reports="$(plan self-hosted/src/loom/validation.sley docs/schemas/sley.validation.report.v1.schema.json scripts/sley-validate.sh)"
assert_has "$validation_reports" validation-reports
assert_has "$validation_reports" validation-planner
assert_has "$validation_reports" contracts
assert_has "$validation_reports" conformance
assert_lacks "$validation_reports" arena

operational_replay="$(plan docs/schemas/sley.operational.reference_replay.v1.schema.json fixtures/siglum/numerology-reference-v1/pins.json scripts/sley-reference-replay.sh)"
assert_has "$operational_replay" operational-replay
assert_has "$operational_replay" contracts
assert_has "$operational_replay" conformance
assert_lacks "$operational_replay" arena

operational_workflow="$(plan self-hosted/src/loom/operational.sley docs/schemas/sley.operational.agent_workflow_plan.v1.schema.json fixtures/operational/agent-workflow-v1/case-manifest.json scripts/sley-operational-workflow.sh)"
assert_has "$operational_workflow" operational-workflow
assert_has "$operational_workflow" operational-evidence
assert_has "$operational_workflow" contracts
assert_has "$operational_workflow" conformance
assert_lacks "$operational_workflow" arena

operational_evidence="$(plan docs/schemas/sley.operational.evidence_registry.v1.schema.json fixtures/contracts/operational_evidence_registry_synthetic.json scripts/sley-operational-evidence.sh)"
assert_has "$operational_evidence" operational-evidence
assert_has "$operational_evidence" contracts
assert_has "$operational_evidence" conformance
assert_lacks "$operational_evidence" operational-replay
assert_lacks "$operational_evidence" arena

release_artifact="$(plan self-hosted/src/loom/release.sley docs/schemas/sley.release.manifest.v1.schema.json fixtures/contracts/release_manifest_linux_x86_64.json scripts/sley-release.sh scripts/test-sley-release.sh)"
assert_has "$release_artifact" release-contracts
assert_has "$release_artifact" contracts
assert_has "$release_artifact" contract-compatibility
assert_has "$release_artifact" conformance
assert_has "$release_artifact" fmt
assert_lacks "$release_artifact" arena

ai="$(plan SLEY_AI.md docs/SleyBenchSpec.md fixtures/sleybench/smoke-v0/case-manifest.json)"
assert_has "$ai" ai-foundation
assert_has "$ai" agent-bench
assert_lacks "$ai" machine-test

corpus_governance="$(plan bin/sley-corpus scripts/test-sley-corpus.sh docs/SleyCorpusSpec.md fixtures/corpus_governance/record.json)"
assert_has "$corpus_governance" corpus-governance
assert_has "$corpus_governance" contracts
assert_has "$corpus_governance" ai-foundation
assert_lacks "$corpus_governance" machine-test

docs="$(plan CHANGELOG.md docs/ValidationTiers.md)"
[[ "$docs" == "diff-check" ]] || fail "documentation-only plan should be diff-check, got [$docs]"

claims="$(plan README.md docs/SleyClaimEvidence.md llms.txt)"
assert_has "$claims" claim-audit
assert_lacks "$claims" test

validation="$(plan Makefile scripts/check-changed.sh scripts/test-core-focus.sh)"
assert_has "$validation" validation-planner
assert_has "$validation" smoke
assert_lacks "$validation" arena

validation_json="$($DISPATCHER --plan-json --files self-hosted/src/loom/parser.sley docs/ValidationTiers.md)"
jq -e '
  .schema == "sley.validation.changed_plan.v1" and
  .changed_files == ["self-hosted/src/loom/parser.sley", "docs/ValidationTiers.md"] and
  (.selected_subsystems | index("parser")) != null and
  (.selected_targets | index("parser-smoke")) != null and
  (.selected_targets | index("arena")) == null and
  .release_gate.required == false
' <<<"$validation_json" >/dev/null || fail "machine-readable plan did not preserve classifier output"

deduplicated_json="$($DISPATCHER --plan-json --files self-hosted/src/loom/parser.sley self-hosted/src/loom/parser.sley)"
jq -e '.changed_files == ["self-hosted/src/loom/parser.sley"]' <<<"$deduplicated_json" >/dev/null \
  || fail "machine-readable plan did not deduplicate explicit paths"

fallback="$(plan future/unknown.surface)"
assert_has "$fallback" test
assert_has "$fallback" contracts
assert_has "$fallback" conformance
assert_lacks "$fallback" arena

echo "check-changed classifier tests passed"
