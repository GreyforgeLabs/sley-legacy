.PHONY: fmt diff-check build-cli build-bins test contracts conformance public-release-check corpus examples smoke lsp workbench agent-bench migrate docgen sandbox-runner zjx-tools syntax v1

fmt:
	cargo fmt -- --check

diff-check:
	git diff --check

build-cli:
	cargo build --bin sley

build-bins:
	cargo build --bins

test:
	cargo test

contracts:
	cargo run --bin sley-contract -- check-fixtures fixtures/contracts --schemas docs/schemas --json
	cargo run --bin sley-contract -- validate --schema sley.conformance.manifest.v0 fixtures/corpus/manifest.json --schemas docs/schemas --json
	cargo run --bin sley-contract -- validate --schema sley.cli_smoke.manifest.v0 fixtures/cli_smokes/manifest.json --schemas docs/schemas --json
	cargo run --bin sley-contract -- validate --schema sley.cli_smoke.manifest.v0 fixtures/ci_smoke_probe/manifest.json --schemas docs/schemas --json

conformance:
	cargo run --bin sley-conformance -- report --json

public-release-check:
	cargo run --bin sley-conformance -- report --json --require-public-release-ready

corpus: build-cli
	cargo run --bin sley-ci -- corpus --json fixtures/corpus/manifest.json

examples: build-cli
	cargo run --bin sley-ci -- examples --json examples

smoke: build-bins
	cargo run --bin sley-ci -- smoke --json --repo-root $(CURDIR) fixtures/cli_smokes/manifest.json
	cargo run --bin sley-ci -- smoke --json --repo-root $(CURDIR) fixtures/ci_smoke_probe/manifest.json

lsp:
	cargo test --test sley_lsp

workbench:
	cargo run --bin sley-workbench -- --json examples/dead_private_tasks.sley

agent-bench: build-cli
	cargo run --bin sley-agent-bench -- run --json

migrate:
	cargo run --bin sley-migrate -- report --json examples/raw_host_migration.sley
	cargo run --bin sley-migrate -- report --json examples/unchecked_result.sley

docgen:
	cargo run --bin sley-docgen -- reference --json examples/agent_deploy_pipeline.sley

sandbox-runner:
	cargo run --bin sley-sandbox-runner -- run --json fixtures/contracts/sandbox_manifest_agent_pipeline.json

zjx-tools:
	cargo run --bin sley-zjx -- inspect --json fixtures/contracts/zjx_hello_ready.json
	cargo run --bin sley-zjx -- verify-digest --json fixtures/contracts/zjx_hello_ready.json
	cargo run --bin sley-zjx -- extract-graph --json fixtures/contracts/zjx_hello_ready.json
	cargo run --bin sley-zjx -- diff-envelope --json fixtures/contracts/zjx_hello_ready.json fixtures/contracts/zjx_hello_ready.json

tree-sitter-sley/node_modules/.package-lock.json: tree-sitter-sley/package.json tree-sitter-sley/package-lock.json
	npm --prefix tree-sitter-sley ci

syntax: tree-sitter-sley/node_modules/.package-lock.json
	npm --prefix tree-sitter-sley test

v1: fmt diff-check test contracts conformance corpus examples smoke lsp workbench agent-bench migrate docgen sandbox-runner zjx-tools syntax
