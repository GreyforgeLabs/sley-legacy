.PHONY: fmt diff-check build-cli build-bins test git-guard mcp-bridge contracts conformance public-release-check claim-audit corpus examples smoke lsp editor-shims workbench agent-bench migrate docgen sandbox-runner shadow zjx-tools arena syntax self-hosted-cli v1

SELF_HOSTED_ARGS := --help
export PATH := $(CURDIR)/bin:$(PATH)

fmt:
	bash -n bin/sley
	bash -n bin/sley-arena
	bash -n bin/sley-ci
	bash -n bin/sley-conformance
	bash -n bin/sley-contract
	bash -n bin/git-sley-guard
	bash -n bin/sley-mcp-bridge
	bash -n scripts/check-self-hosted-code.sh
	bash -n scripts/self-hosting-inventory.sh
	bash -n scripts/self-hosted-test.sh
	bash -n scripts/test-git-sley-guard.sh
	bash -n scripts/test-sley-mcp-bridge.sh

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

git-guard:
	scripts/test-git-sley-guard.sh

mcp-bridge:
	scripts/test-sley-mcp-bridge.sh

contracts:
	sley-contract check-fixtures fixtures/contracts --schemas docs/schemas --json
	sley-contract validate --schema sley.conformance.manifest.v0 fixtures/corpus/manifest.json --schemas docs/schemas --json
	sley-contract validate --schema sley.cli_smoke.manifest.v0 fixtures/cli_smokes/manifest.json --schemas docs/schemas --json
	sley-contract validate --schema sley.cli_smoke.manifest.v0 fixtures/ci_smoke_probe/manifest.json --schemas docs/schemas --json
	sley-contract validate --schema sley.claim.manifest.v0 docs/SleyClaimManifest.json --schemas docs/schemas --json

conformance:
	sley-conformance report --json

public-release-check:
	sley-conformance report --json --require-public-release-ready

claim-audit:
	sley claim-verify --json docs/SleyClaimManifest.json

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
	sley-agent-bench run --json

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

v1: fmt diff-check test git-guard mcp-bridge contracts conformance claim-audit corpus examples smoke lsp editor-shims workbench agent-bench migrate docgen sandbox-runner shadow zjx-tools arena syntax
