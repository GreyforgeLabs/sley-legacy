# Sley Codex /goal Prompt

Date: 2026-05-05
Project root: `/home/greyforge/sley`

## What /goal Should Do Here

Use `/goal` as persistent thread intent for a long Sley session. The feature is
best suited to keeping Codex aligned around the product thesis while individual
turns still provide concrete tasks, files, tests, and stop conditions.

Official OpenAI documentation currently exposes thread goals through the
experimental Codex app-server API:

- `thread/goal/set`
- `thread/goal/get`
- `thread/goal/clear`

The public slash-command page does not currently list `/goal`, while local
`codex-cli 0.128.0` shows the `goals` feature as under development and disabled.
Enable it if the slash command is absent:

```bash
codex features list
codex features enable goals
```

Restart Codex if prompted, open a fresh session in `/home/greyforge/sley`, and
paste the goal prompt below into `/goal`.

Sources:

- OpenAI Codex app-server API overview:
  https://developers.openai.com/codex/app-server#api-overview
- OpenAI Codex slash commands:
  https://developers.openai.com/codex/cli/slash-commands
- OpenAI Codex best practices:
  https://developers.openai.com/codex/learn/best-practices#strong-first-use-context-and-prompts

## Local Sley Reality Check

Sley is an agent-native structural programming language. The current compiler,
Loom, is written in Rust and exposes:

- human-reviewable `.sley` source;
- typed graph-shaped AST data;
- project manifests;
- module, import, type, effect, and task declarations;
- explicit `take`, `bind`, `state`, `tally`, `slot`, and `forge` concepts;
- static checks for type, effect, namespace, result-flow, and authority errors;
- deterministic seeded runtime adapters for files, database, secrets, deploy,
  spend, network, shell, and model calls;
- JSON AST, diagnostics, symbol graph, graph slices, trace seals, graft
  outcomes, and ZJX envelopes;
- strict structural graft operations with dry-run and explicit `--write`;
- locked contract snapshots, JSON schemas, conformance fixtures, and tests.

The core bet is not "another syntax." The core bet is:

```text
source projection -> typed graph -> checked grafts -> stable diagnostics ->
human review -> safe execution
```

The language becomes strong for agents only if the compiler is a strict
referee, the graph contract is stable, diagnostics are repairable, and side
effects require explicit authority.

## Recommended /goal Prompt

```text
Goal:
Make Sley the best programming language in the world for agents. In this
thread, optimize for agent success rate, structural edit safety, compiler
strictness, readable source projection, stable machine-readable contracts,
deterministic execution, explicit authority gates, and a credible self-hosting
path. Do not optimize for surface syntax novelty unless it improves the
compiler-mediated agent workflow.

Context:
Work in `/home/greyforge/sley`.

Read these first:

- `README.md`
- `llms.txt`
- `docs/SleyLanguageSpec.md`
- `SleyCompiler.md`
- `SleyImprove.md`
- `Cargo.toml`
- `src/parser.rs`
- `src/ast.rs`
- `src/checker.rs`
- `src/runtime.rs`
- `src/graft.rs`
- `src/symbols.rs`
- `src/trace.rs`
- `src/zjx.rs`
- `tests/sley_v0.rs`
- `docs/schemas/*.schema.json`
- `examples/*.sley`

Product thesis:

Sley should make agents better programmers by replacing fragile raw text edits
with a tight compiler-mediated loop:

1. Read compact docs and examples.
2. Inspect typed AST or graph slices.
3. Propose a strict structural graft.
4. Run checker and formatter.
5. Read stable diagnostics and repair hints.
6. Iterate until Loom accepts the change.
7. Leave human-reviewable source, trace receipts, and content-addressed seals.

Hard truth:

Codex and other agents do not have broad Sley pretraining. Sley must therefore
teach itself through tooling: schemas, graph slices, repair hints, examples,
conformance fixtures, dry-run grafts, and deterministic runtime seeds. If a
feature cannot be learned through that loop, it is not agent-native yet.

Non-negotiable constraints:

- Source text is the human review projection.
- The typed graph is the canonical agent work surface.
- Prefer structural grafts over raw text edits.
- Plain graft preview must stay non-mutating unless `--write` is supplied.
- Unknown graft fields must continue to reject instead of being silently
  ignored.
- Authority failures must remain diagnostics, not recoverable userland errors.
- Recoverable host failures should use typed `Result<T, Error>` flows.
- Do not add effects merely to silence diagnostics; authority must be real.
- Do not invent binding kinds casually. The stable vocabulary is governed by
  `docs/SleyLanguageSpec.md`.
- Keep runtime host adapters deterministic in v0. Seeded database, secret,
  deploy, spend, network, shell, and model adapters must not call real external
  systems.
- Do not publish, deploy, spend, call providers, or mutate external systems
  without explicit operator approval.
- Preserve unrelated local changes.
- Keep tests and contract snapshots authoritative.

World-best criteria:

Sley is moving toward "best programming language for agents" only when it
improves measurable agent work, not when it merely accumulates syntax.

Judge work by:

- Can an agent inspect the relevant program slice without loading the whole
  codebase?
- Can an agent propose a legal edit as a graft instead of patching raw text?
- Does Loom reject illegal edits with stable IDs, spans, node IDs, and repair
  hints?
- Does the formatter preserve a clean human-reviewable projection?
- Does `check --json` give enough information for the next repair attempt?
- Are effects explicit and enforced at check/runtime boundaries?
- Are runtime examples deterministic and reproducible?
- Are accepted and rejected examples locked into conformance tests?
- Are JSON schemas stable and versioned?
- Can future agents resume from `llms.txt`, schemas, examples, and tests
  without private chat context?

High-leverage work lanes:

1. Agent tool contract:
   - keep `parse`, `format`, `check`, `run`, `ast`, `graph`, `trace`, `seal`,
     `zjx`, and `graft` stable;
   - add or harden `lint --json` only when there is a clear contract;
   - make every JSON root schema-versioned.

2. Diagnostics and repair:
   - improve parse errors, unknown names, type mismatches, return mismatches,
     call arity/type errors, private imports, ambiguous imports, effect
     authority errors, stale grafts, and unsupported graft operations;
   - include repair hints that suggest valid structural edits without bypassing
     authority.

3. Graft-first editing:
   - expand graph-slice grafts where tests prove safety;
   - harden project writeback beyond existing modules;
   - keep dry-run paths explicit and trustworthy;
   - preserve trace receipts and seals for accepted writes.

4. Conformance and contracts:
   - grow accepted/rejected fixtures;
   - lock contract snapshots under `fixtures/contracts/`;
   - keep JSON schemas under `docs/schemas/`;
   - turn CLI smoke examples into stable tests.

5. Runtime authority:
   - keep seeded v0 adapters deterministic;
   - keep real external calls out of v0 tests;
   - preserve the distinction between authority diagnostics and recoverable
     `Result` host failures.

6. Self-hosting ladder:
   - keep Rust Loom as the strict oracle;
   - write Sley helper/lint/query passes only after the graph contract supports
     them;
   - run self-hosted passes in shadow mode before promotion;
   - preserve a recovery oracle.

Default first moves in any new session:

1. Run:

```bash
cargo test
```

2. Inspect the relevant command behavior before editing:

```bash
cargo run -- check --json examples/hello.sley
cargo run -- graph --json examples/hello.sley
cargo run -- ast --json examples/hello.sley
```

3. If changing grafts, inspect or create a dry-run graft first:

```bash
cargo run -- graft --json --dry-run <target> <graft.json>
```

4. If changing runtime authority, test both the authorized and unauthorized
   path.

5. If changing JSON output, update schemas and locked contract fixtures in the
   same change.

Validation commands to prefer:

```bash
cargo fmt --check
cargo test
cargo run -- check --json examples/hello.sley
cargo run -- run --json examples/hello.sley
cargo run -- graph --json examples/hello.sley
cargo run -- seal --json examples/hello.sley
cargo run -- zjx --json examples/hello.sley
cargo run -- run --json --cap DatabaseRead --db-table users=examples/users.json examples/db_gate.sley
cargo run -- run --json --cap DatabaseRead --cap DatabaseWrite examples/db_write_gate.sley
cargo run -- run --json --cap Network --http-text https://example.test/profile Ada examples/network_gate.sley
cargo run -- run --json --cap Shell --shell-output date 2026-05-05 examples/shell_gate.sley
cargo run -- run --json --cap ModelCall --model-output name Ada examples/model_gate.sley
cargo run -- run --json --cap SecretRead --secret api_key redacted examples/secret_gate.sley
cargo run -- run --json --cap Deploy --deploy-result staging staged examples/deploy_gate.sley
cargo run -- run --json --cap Spend --spend-result ads-budget authorized examples/spend_gate.sley
```

Done when:

- The chosen change improves Sley's agent edit loop, not just its syntax.
- Tests pass, or failures are documented with exact command output summaries.
- JSON schema or contract changes are updated deliberately.
- New diagnostics include stable IDs and useful repair hints where practical.
- New graft behavior is covered by accepted and rejected tests.
- Runtime authority changes include positive and negative coverage.
- Documentation is updated only where it helps future agents use Sley correctly.
- The final summary states:
  - behavioral delta;
  - files changed;
  - validation run;
  - remaining risks;
  - next best task.
```

## Best First Thread

For maximum productivity, start with a narrow task under the goal above:

```text
Use the active /goal. Audit the current Sley agent edit loop from llms.txt,
docs/SleyLanguageSpec.md, graft behavior, diagnostics, JSON schemas, and
tests/sley_v0.rs. Identify the single highest-leverage improvement that would
make agents more successful at editing Sley programs safely. Then implement
that improvement with tests and docs, keeping the change narrow.
```
