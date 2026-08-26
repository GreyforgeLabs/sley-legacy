.PHONY: fmt diff-check build-cli build-bins test machine-test git-guard mcp-bridge graph-diff contracts contract-compatibility protocol-contracts transaction-contracts adapter-replay worker worker-clients user-tests validation-reports operational-replay operational-workflow operational-evidence release-contracts release-archive release-security-check release-candidate conformance public-release-check claim-audit ai-foundation corpus-governance corpus examples smoke lsp editor-shims workbench agent-bench migrate docgen sandbox-runner shadow zjx-tools arena syntax self-hosted-cli parser-smoke checker-smoke runtime-smoke lint-smoke validation-planner quick core check-changed v1

SELF_HOSTED_ARGS := --help
export PATH := $(CURDIR)/bin:$(PATH)

fmt:
	bash -n bin/sley
	bash -n bin/sley-arena
	bash -n bin/sley-ci
	bash -n bin/sley-conformance
	bash -n bin/sley-contract
	bash -n bin/sley-corpus
	bash -n bin/sleybench-split
	bash -n bin/git-sley-guard
	bash -n bin/sley-mcp-bridge
	bash -n scripts/check-self-hosted-code.sh
	bash -n scripts/check-ai-foundation.sh
	bash -n scripts/test-sley-corpus.sh
	bash -n scripts/test-sleybench-split.sh
	bash -n scripts/test-sleybench-protocol.sh
	bash -n scripts/test-sleybench-mode-summary.sh
	bash -n scripts/test-contract-compatibility.sh
	bash -n scripts/generate-sleybench-smoke.sh
	bash -n scripts/sleybench-evaluator.sh
	bash -n scripts/test-sleybench-evaluator.sh
	bash -n scripts/self-hosting-inventory.sh
	bash -n scripts/self-hosted-test.sh
	bash -n scripts/test-git-sley-guard.sh
	bash -n scripts/test-sley-mcp-bridge.sh
	bash -n scripts/test-sley-mcp-transaction.sh
	bash -n scripts/test-sley-graph-diff.sh
	bash -n scripts/test-sley-transaction-inspect.sh
	bash -n scripts/test-sley-change-preview.sh
	bash -n scripts/test-sley-change-apply.sh
	bash -n scripts/test-sley-change-review.sh
	bash -n scripts/sley-adapter-replay.sh
	bash -n scripts/test-sley-adapter-replay.sh
	bash -n scripts/sley-worker-local.sh
	bash -n scripts/test-sley-worker.sh
	bash -n scripts/test-sley-worker-clients.sh
	bash -n scripts/sley-test.sh
	bash -n scripts/test-sley-user-tests.sh
	bash -n scripts/sley-validate.sh
	bash -n scripts/test-sley-validation.sh
	bash -n scripts/sley-reference-replay.sh
	bash -n scripts/test-sley-reference-replay.sh
	bash -n scripts/sley-operational-workflow.sh
	bash -n scripts/test-sley-operational-workflow.sh
	bash -n scripts/sley-operational-evidence.sh
	bash -n scripts/test-sley-operational-evidence.sh
	bash -n scripts/sley-release.sh
	bash -n scripts/test-sley-release.sh
	scripts/sley-release.sh --help >/dev/null
	bash -n scripts/check-changed.sh
	bash -n scripts/test-check-changed.sh
	bash -n scripts/test-core-focus.sh
	bash -n scripts/machine-test.sh

diff-check:
	git diff --check

build-cli:
	test -x bin/sley
	test -x bin/sley-arena
	bin/sley --version

build-bins:
	test -x bin/git-sley-guard
	test -x bin/sley-mcp-bridge
	test -x bin/sley-ci
	test -x bin/sley-conformance
	test -x bin/sley-contract
	test -x bin/sley-corpus
	test -x bin/sleybench-split
	test -x bin/sley-docgen
	test -x bin/sley-lsp
	test -x bin/sley-workbench
	test -x bin/sley-agent-bench
	test -x bin/sley-migrate
	test -x bin/sley-sandbox-runner
	test -x bin/sley-shadow
	test -x bin/sley-zjx

test:
	scripts/self-hosted-test.sh

machine-test:
	scripts/machine-test.sh

git-guard:
	scripts/test-git-sley-guard.sh

mcp-bridge:
	scripts/test-sley-mcp-bridge.sh
	scripts/test-sley-mcp-transaction.sh

graph-diff:
	scripts/test-sley-graph-diff.sh

contracts:
	sley-contract check-fixtures fixtures/contracts --schemas docs/schemas --json
	sley-contract validate --schema sley.conformance.manifest.v0 fixtures/corpus/manifest.json --schemas docs/schemas --json
	sley-contract validate --schema sley.cli_smoke.manifest.v0 fixtures/cli_smokes/manifest.json --schemas docs/schemas --json
	sley-contract validate --schema sley.cli_smoke.manifest.v0 fixtures/ci_smoke_probe/manifest.json --schemas docs/schemas --json
	sley-contract validate --schema sley.claim.manifest.v0 docs/SleyClaimManifest.json --schemas docs/schemas --json

protocol-contracts:
	scripts/test-sleybench-protocol.sh
	scripts/test-sleybench-mode-summary.sh

transaction-contracts:
	scripts/test-sley-transaction-inspect.sh
	scripts/test-sley-change-preview.sh
	scripts/test-sley-change-apply.sh
	scripts/test-sley-change-review.sh

adapter-replay:
	scripts/test-sley-adapter-replay.sh

worker:
	scripts/test-sley-worker.sh

worker-clients:
	scripts/test-sley-worker-clients.sh

user-tests:
	scripts/test-sley-user-tests.sh

validation-reports:
	scripts/test-sley-validation.sh

operational-replay:
	scripts/test-sley-reference-replay.sh

operational-workflow:
	scripts/test-sley-operational-workflow.sh

operational-evidence:
	scripts/test-sley-operational-evidence.sh

release-contracts:
	scripts/test-sley-release.sh

release-archive:
	bin/sley release build --output-dir dist --json

release-security-check:
	bin/sley release verify --output-dir dist --json

release-candidate: v1 public-release-check release-archive release-security-check

contract-compatibility:
	scripts/test-contract-compatibility.sh

conformance:
	sley-conformance report --json

public-release-check:
	sley-conformance report --json --require-public-release-ready

claim-audit:
	sley claim-verify --json docs/SleyClaimManifest.json

ai-foundation:
	scripts/check-ai-foundation.sh

corpus-governance:
	scripts/test-sley-corpus.sh

corpus:
	sley-ci corpus --json fixtures/corpus/manifest.json

examples:
	sley-ci examples --json examples

smoke:
	sley-ci smoke --json --repo-root $(CURDIR) fixtures/ci_smoke_probe/manifest.json

lsp:
	sley-lsp --json

editor-shims:
	sley-lsp --validate-editor-shims

workbench:
	sley-workbench --json examples/dead_private_tasks.sley

agent-bench:
	scripts/test-sleybench-evaluator.sh
	scripts/test-sleybench-split.sh

migrate:
	sley-migrate report --json examples/raw_host_migration.sley
	sley-migrate report --json examples/unqualified_import_call_project
	sley-migrate report --json examples/unchecked_result.sley
	sley-migrate report --json examples/unchecked_result_binding.sley

docgen:
	sley-docgen reference --json examples/agent_deploy_pipeline.sley
	sley-docgen reference --json --module agent.pipeline examples/agent_project

sandbox-runner:
	sley-sandbox-runner run --json fixtures/contracts/sandbox_manifest_agent_pipeline.json

shadow:
	sley-shadow report --json examples/agent_deploy_pipeline.sley
	sley-shadow report --json examples/agent_project
	sley-shadow report --json --module agent.pipeline examples/agent_project
	sley-shadow report --json --rule unused_private_task examples/unused_private_task.sley

zjx-tools:
	sley-zjx inspect --json fixtures/contracts/zjx_hello_ready.json
	sley-zjx verify-digest --json fixtures/contracts/zjx_hello_ready.json
	sley-zjx extract-graph --json fixtures/contracts/zjx_hello_ready.json
	sley-zjx diff-envelope --json fixtures/contracts/zjx_hello_ready.json fixtures/contracts/zjx_hello_ready.json

arena:
	sley-arena --json --fast --no-color | jq -e '.schema == "sley.arena.report.v0" and .status == "consensus" and .agent_count == 50'
	sley arena --json --fast --no-color --mode standoff | jq -e '.schema == "sley.arena.report.v0" and .status == "exploded"'

syntax:
	scripts/check-self-hosted-code.sh

self-hosted-cli:
	bin/sley $(SELF_HOSTED_ARGS)

parser-smoke:
	scripts/test-core-focus.sh parser

checker-smoke:
	scripts/test-core-focus.sh checker

runtime-smoke:
	scripts/test-core-focus.sh runtime

lint-smoke:
	scripts/test-core-focus.sh lint

validation-planner:
	scripts/test-check-changed.sh

quick: fmt diff-check syntax validation-planner ai-foundation parser-smoke checker-smoke runtime-smoke lint-smoke smoke

core: parser-smoke checker-smoke runtime-smoke lint-smoke contracts contract-compatibility protocol-contracts transaction-contracts adapter-replay worker worker-clients user-tests validation-reports operational-replay operational-workflow operational-evidence release-contracts conformance ai-foundation corpus-governance smoke syntax

check-changed:
	scripts/check-changed.sh

v1: fmt diff-check test git-guard mcp-bridge graph-diff contracts contract-compatibility protocol-contracts transaction-contracts adapter-replay worker worker-clients user-tests validation-reports operational-replay operational-workflow operational-evidence release-contracts conformance claim-audit ai-foundation corpus-governance corpus examples smoke lsp editor-shims workbench agent-bench migrate docgen sandbox-runner shadow zjx-tools arena syntax validation-planner
