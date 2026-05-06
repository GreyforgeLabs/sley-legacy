.PHONY: fmt diff-check test contracts conformance corpus examples smoke lsp workbench agent-bench migrate docgen sandbox-runner zjx-tools syntax v1

fmt:
	cargo fmt -- --check

diff-check:
	git diff --check

test:
	cargo test

contracts:
	cargo run --bin sley-contract -- check-fixtures fixtures/contracts --schemas docs/schemas --json
	cargo run --bin sley-contract -- validate --schema sley.conformance.manifest.v0 fixtures/corpus/manifest.json --schemas docs/schemas --json
	cargo run --bin sley-contract -- validate --schema sley.cli_smoke.manifest.v0 fixtures/cli_smokes/manifest.json --schemas docs/schemas --json
	cargo run --bin sley-contract -- validate --schema sley.cli_smoke.manifest.v0 fixtures/ci_smoke_probe/manifest.json --schemas docs/schemas --json

conformance:
	cargo run --bin sley-conformance -- report --json

corpus:
	cargo run --bin sley-ci -- corpus --json fixtures/corpus/manifest.json

examples:
	cargo run --bin sley-ci -- examples --json examples

smoke:
	cargo run --bin sley-ci -- smoke --json --repo-root $(CURDIR) fixtures/cli_smokes/manifest.json

lsp:
	cargo check --bin sley-lsp

workbench:
	cargo check --bin sley-workbench

agent-bench:
	cargo check --bin sley-agent-bench

migrate:
	cargo check --bin sley-migrate

docgen:
	cargo check --bin sley-docgen

sandbox-runner:
	cargo check --bin sley-sandbox-runner

zjx-tools:
	cargo check --bin sley-zjx

syntax:
	npm --prefix tree-sitter-sley test

v1: fmt diff-check test contracts conformance corpus examples smoke lsp workbench agent-bench migrate docgen sandbox-runner zjx-tools
