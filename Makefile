.PHONY: fmt diff-check test contracts corpus examples smoke v1

fmt:
	cargo fmt -- --check

diff-check:
	git diff --check

test:
	cargo test

contracts:
	cargo run --bin sley-contract -- check-fixtures fixtures/contracts --schemas docs/schemas --json

corpus:
	cargo run --bin sley-ci -- corpus --json fixtures/corpus/manifest.json

examples:
	cargo run --bin sley-ci -- examples --json examples

smoke:
	cargo run --bin sley-ci -- smoke --json --repo-root $(CURDIR) fixtures/cli_smokes/manifest.json

v1: fmt diff-check test contracts corpus examples smoke
