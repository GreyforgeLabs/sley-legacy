# May 8 Sley Claim Audit

Status: internal engineering audit, historical baseline for the stage-1 remediation
Repo audited: `/home/greyforge/sley`  
Branch: `public` at `8a67018 Add self-hosted CLI make target`  
External prior-art spot check: `https://github.com/sbhooley/ainativelang`

## 2026-05-08 Remediation Addendum

This audit remains the baseline that identified the overclaim. It is no longer a
description of the current worktree after the stage-1 remediation.

Current state after remediation:

- forbidden Rust, C/C++, JavaScript, TypeScript, and Python implementation files
  are absent from the working tree;
- `bin/sley` and companion wrappers provide a runnable self-hosting stage-1
  bootstrap command surface;
- `self-hosted/src/loom/` now contains Sley-owned stage-2 semantic source
  modules, and the bootstrap reads version, lint-rule inventory, core report
  IDs, diagnostic IDs, runtime seed values, and parser expression classifiers
  from `.sley` source;
- checker diagnostic status and unknown-identifier message construction are now
  read from `loom.checker`;
- lint finding statuses, messages, and hints are now read from `loom.lint`;
- `sley run --json self-hosted` now executes a bootstrap smoke over the
  Sley-owned lint-rule inventory;
- `make v1` passes without Cargo, Rust, Node, npm, or tree-sitter;
- `make public-release-check` intentionally fails with `PUBLIC_RELEASE_BLOCKED`
  until strict Sley-written parity, operator approval, and a published proof
  bundle exist.

The strict claim "Sley's compiler is written in Sley" remains blocked. The
current accurate claim is that the repo has a foreign-source-free bootstrap,
runnable command envelopes, initial Sley-owned semantic source modules, and
truthful public wording.

## Verdict

The self-hosting claim is not true in the current repo state.

The strongest accurate statement is:

> Sley has a Rust-implemented compiler/tooling stack in the last committed HEAD, plus schemas, fixtures, examples, and a documented migration plan toward a self-hosted implementation. The current working tree is in a broken migration state: the Rust implementation and the self-hosted scaffold are staged for deletion, leaving no runnable compiler from the repo.

The strongest inaccurate or unsupported statements are:

- "Sley is self-hosted."
- "Sley has moved on to a self-hosted language with its own compiler."
- "Successfully self-hosting proves robust data structures, scoping, and functional boundaries."
- "Sley's compiler is written in Sley."
- "The repo currently contains a working self-hosted parser, checker, runtime, or compiler."
- "World's first AI-native language" as an unqualified public claim.

Confidence: high for the Sley repo state, moderate for the external prior-art comparison.

## What The Repo Actually Shows

### Current worktree

The current `/home/greyforge/sley` worktree has no Rust, C, or C header files in the filesystem, but that is not evidence of self-hosting. It is evidence that the implementation files are absent.

Observed current file extension counts:

```text
json 228
md 12
png 4
sh 2
sley 142
toml 5
txt 1
yaml 1
yml 2
```

Current verification commands:

```text
./scripts/check-self-hosted-code.sh
```

Result:

```text
Self-hosted gate passed: no forbidden extensions found.
```

That gate only proves forbidden file extensions are absent. It does not prove a self-hosted compiler exists.

The executable checks fail:

```text
make self-hosted-cli
```

Result:

```text
Error: Cannot find module '/home/greyforge/sley/self-hosted/cli.mjs'
make: *** [Makefile:86: self-hosted-cli] Error 1
```

```text
make test
```

Result:

```text
cargo test
error: could not find `Cargo.toml` in `/home/greyforge/sley` or any parent directory
make: *** [Makefile:18: test] Error 101
```

```text
command -v sley; sley --version
```

Result:

```text
zsh:1: command not found: sley
```

### Staged deletion state

`git status --porcelain=v1` shows staged deletion of the implementation and test stack, including:

- `Cargo.toml`
- `Cargo.lock`
- `self-hosted/cli.mjs`
- all `src/*.rs`
- all `src/bin/*.rs`
- the Rust integration tests under `tests/`
- tree-sitter package files
- VS Code editor shim files

`git diff --cached --stat` reports:

```text
67 files changed, 84269 deletions(-)
```

This means the repo is not merely "self-hosted now." It is currently staged to remove the old Rust implementation before a replacement implementation exists in the working tree.

### Clean HEAD state

I created a detached worktree at `/tmp/sley-head-audit` from `HEAD` to evaluate the last committed state without disturbing the dirty repo.

Clean HEAD extension counts include:

```text
rs 44
mjs 3
js 2
json 237
sley 142
toml 6
```

Clean HEAD contains a Rust crate:

```text
Cargo.toml
src/main.rs
src/parser.rs
src/checker.rs
src/runtime.rs
src/graft.rs
src/lint.rs
src/plan.rs
...
```

Clean HEAD also contains `self-hosted/cli.mjs`, but that file is a Node scaffold, not a self-hosted Sley compiler. Its own output says:

```text
Sley self-hosted CLI scaffold
...
This is a migration scaffold for the self-hosted transition.
```

The scaffold only supports:

```text
doctor [--json]
ast [--json] <path>
check [--json] <path>
```

and reports pending implementation for parser/checker behavior.

Clean HEAD build evidence:

```text
cargo test --no-run
```

passed and built the Rust crate. That proves the committed compiler/tooling stack is Rust-backed.

Clean HEAD self-hosting gate:

```text
./scripts/check-self-hosted-code.sh
```

failed because Rust files are present. That is expected and confirms HEAD is not self-hosted.

Clean HEAD full test run:

```text
cargo test
```

failed with 3 failures in `tests/sley_v0.rs`:

- `deploy_dry_run_reports_verified_package_without_live_mutation`
- `conformance_report_summarizes_release_surface`
- `cli_smoke_manifest_commands_match_stable_release_surface`

The run also reported the conformance summary's current integration-test count as 337, with the readiness blocker:

```text
integration test count is 337, but docs/contracts.md declares None
```

This matters because older notes that say 210 integration cases are stale relative to clean HEAD's current conformance count.

## Documentation Truth Versus Marketing Truth

The self-hosting docs are more honest than the README.

`SELF_HOSTING_MIGRATION_PLAN.md` says:

- Rust source files are currently 0 in the current worktree.
- The active Rust implementation and tree-sitter artifacts are removed from tracked sources.
- Command parity and runtime behavior migration are still in progress.
- The progress checklist leaves these unchecked:
  - self-hosted parser/AST bootstrapped
  - CLI command compatibility achieved
  - runtime host adapter parity achieved
  - tooling and schema parity validated

That is accurate.

The problem is that `README.md`, `llms.txt`, and related public-facing command surfaces still imply a working implemented language:

- "Sley is the world's first AI-native programming language."
- "Sley is the world's first AI-native structural language."
- "Implemented now:" followed by a long command and feature inventory.
- "Start in 10 minutes" commands that cannot run from the current repo.

Those claims are not currently supported by the worktree.

## Audit Of The Gemini Context

### Gemini's first comparison

The first Gemini answer criticized the old Sley as an esolang-like Rust VM with no functions, no normal control flow, and no IR.

That critique is stale for the current Sley design. The current docs/spec and clean HEAD implementation show:

- `task` declarations
- `take` inputs
- module/import/type/effect declarations
- typed AST schemas
- symbol graph and graph-slice reports
- checked graft operations
- lint, doctor, verify, trace, seal, ZJX, and conformance tooling
- runtime gates and deterministic seeded host adapters

So the old critique should not be reused as the current technical description.

### The user's correction

The correction in the conversation was:

> no, we've moved on to a self-hosted language with its own compiler.

That is false against the repo.

Accurate replacement:

> Sley is migrating from a Rust implementation toward a self-hosted compiler/tooling stack. The migration is not complete. The current worktree has removed the old implementation before a replacement compiler exists.

### Gemini's follow-up

Gemini then inferred:

- self-hosting is complete
- parser/AST/IR likely exist in the new implementation
- self-hosting proves scoping, data structures, and functional boundaries
- agents can now refactor the compiler natively

Those are unsupported. The repo's own migration checklist says the parser/AST bootstrap, CLI compatibility, runtime adapter parity, and tooling/schema parity are not complete.

The AST/graph model exists as schemas, docs, fixtures, and Rust HEAD behavior. It is not currently proven as a self-hosted implementation.

## AINativeLang Prior-Art Risk

AINativeLang is real prior art for the phrase "AI Native Lang" and for an agent-oriented graph IR language.

Spot-checked evidence from `sbhooley/ainativelang`:

- repo description and README present AINL as AI Native Lang
- `pyproject.toml` package name is `ainativelang`, version `1.8.0`
- `compiler_v2.py` implements an AINL compiler and canonical graph IR machinery
- `runtime/engine.py` implements a runtime engine
- `scripts/ainl_mcp_server.py` exposes compile, validation, execution, capability, and security-report tools over MCP
- `docs/AINL_SPEC.md` explicitly defines AINL as an agent-native production language and an AI-to-AI intermediate programming language
- repo has hundreds of Python files and tests

This does not make AINL the same thing as Sley. AINL is explicitly an agent-to-agent compact IR/workflow language and says it is not intended for human source review. Sley's strongest differentiator is the opposite: human-reviewable source as the stable projection, with compiler-exposed structure for agent edits.

But it does make the unqualified "world's first AI-native language" claim unsafe. At minimum, AINL already occupies that phrase in public with a working implementation.

## Direct Technical Position Today

### AINL is technically stronger today

AINL currently has a runnable compiler/runtime architecture visible in the public repo. It has a formal spec, graph IR, runtime engine, MCP integration, adapters, and broad test/docs surface.

Sley's clean HEAD has a serious Rust compiler/tooling implementation, but the current repo state is broken by staged deletion. Sley is not currently runnable from the worktree.

So the honest technical answer today is:

> AINL is ahead as a working AI-native workflow/IR language. Sley has a better and more ambitious thesis for human-reviewed, compiler-mediated agent edits, but Sley has not yet earned the self-hosted claim and currently loses on executable proof.

### Sley could become technically stronger in its own category

Sley can beat AINL in a narrower category if it actually lands:

- a self-hosted Sley compiler or clearly defined bootstrap compiler
- a deterministic source-to-typed-graph pipeline
- stable structural grafts over real source
- human-reviewable source as first-class review projection
- reproducible conformance reports from public commands
- clean release gates

That category is not "first AI-native language." It is closer to:

> agent-native structural programming language for compiler-mediated, human-reviewed software change.

## Claim Ledger

| Claim | Current status | Evidence | Fix |
|---|---|---|---|
| Sley is self-hosted | False | no self-hosted compiler in worktree; HEAD is Rust; Node scaffold is missing in current tree and incomplete in HEAD | build parser/checker/runtime in target stack before saying this |
| Sley has its own compiler | Partly true historically | clean HEAD has Rust compiler/tooling; current worktree has no runnable compiler | say "Rust implementation exists in HEAD; self-hosted migration in progress" |
| Sley has AST/graph surfaces | Partly true | schemas, fixtures, docs, and Rust HEAD support AST/graph reports | keep claim tied to Rust HEAD or completed replacement |
| Sley has an IR | Ambiguous | docs say typed graph and symbol graph; no separate compiler IR proven in current worktree | define canonical IR explicitly or avoid "IR" in claims |
| Sley is world's first AI-native language | Not defensible | AINativeLang exists publicly with AI Native Lang name, compiler, runtime, spec | remove or heavily qualify |
| Sley is technically better than AINL today | False on executable proof | current Sley worktree cannot run; AINL repo has compiler/runtime/MCP | say Sley has a stronger target thesis, not a stronger current implementation |
| Sley is a serious language design, not the old esolang caricature | True for design and Rust HEAD | task/type/effect/module/runtime-gate/graft/spec surfaces exist | keep, but separate from self-hosted claim |

## Required Build Path To Make The Strong Claim True

1. Freeze public claims immediately.
   - Replace "world's first AI-native language" with a narrower claim.
   - Recommended internal-safe phrasing: "Sley is Greyforge Labs' agent-native structural programming language for compiler-mediated, human-reviewed software change."
   - Do not say self-hosted until the repo proves it.

2. Do not commit the staged implementation deletion as a release state.
   - Keep the Rust compiler as the oracle until the replacement reaches parity.
   - If the deletion is part of a private migration branch, mark the branch non-release and make README/llms reflect broken migration state.

3. Define "self-hosted" precisely.
   - If it means "compiler written in Sley," then TypeScript/Node is not self-hosted.
   - If it means "not Rust/C, implemented in an internal hosted runtime," rename the claim to "foreign-language-free migration" or "hosted bootstrap" instead.

4. Restore a runnable CLI baseline.
   - `sley --help`
   - `sley --version`
   - `sley check --json examples/hello.sley`
   - `sley ast --json examples/hello.sley`
   - `sley doctor --json examples/hello.sley`

5. Build a parity harness before porting deeper behavior.
   - Golden outputs from Rust HEAD for accepted fixtures.
   - Golden diagnostics for rejected fixtures.
   - Golden schema validation for contracts.
   - Golden command-exit behavior for `llms.txt` command surfaces.

6. Port in this order.
   - tokenizer/parser
   - AST node IDs and schema output
   - checker for task/type/effect/module basics
   - graph/symbol/query reports
   - lint/doctor/plan
   - graft dry-run
   - runtime for pure programs
   - gated seeded host adapters
   - verify/deploy/trace/seal/ZJX

7. Only remove Rust after parity is real.
   - `make test` or replacement equivalent passes.
   - `make v1` or replacement equivalent passes.
   - self-hosted gate passes.
   - conformance report is clean.
   - public README commands run from a fresh checkout.

8. Publish proof before using first-of-kind claims.
   - release tag
   - reproducible install instructions
   - conformance report
   - public minimal examples
   - documented distinction from AINL and other prior art

## Recommended Public Claim Set After Cleanup

Use:

> Sley is an agent-native structural programming language for compiler-mediated, human-reviewed software change.

Use:

> Sley treats source as the review projection and compiler-exposed structure as the agent work surface.

Use only after the migration lands:

> Sley is self-hosted.

Avoid:

> world's first AI-native language

Avoid:

> designed by AI for AI

Avoid:

> implemented now

unless every listed command works from a clean checkout.

## Bottom Line

We have not merely overclaimed. The current worktree contradicts the strongest claim.

Sley has a credible architecture and a serious Rust-backed implementation in clean HEAD, but the current repo state is not a working self-hosted language. The immediate task is to stop making broad first/self-hosted claims, restore or preserve executable proof, and complete the migration behind parity tests before using self-hosting as a technical advantage.
