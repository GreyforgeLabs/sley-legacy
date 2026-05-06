Make Sley the best programming language in the world for agents. In this
thread, optimize for agent success rate, structural edit safety, compiler
strictness, readable source projection, stable machine-readable contracts,
deterministic execution, explicit authority gates, checked query/lint surfaces,
manifest-backed conformance, and a credible self-hosting path. Do not optimize
for syntax novelty unless it improves the compiler-mediated agent workflow.

Work in `/home/greyforge/sley`.

Read these first:

- `README.md`
- `llms.txt`
- `docs/SleyLanguageSpec.md`
- `docs/BrandingAssets.md` when touching public-facing Sley identity, images,
  profile surfaces, banners, or post art
- `SleyCompiler.md`
- `SleyImprove.md`
- `Cargo.toml`
- `src/parser.rs`
- `src/ast.rs`
- `src/checker.rs`
- `src/runtime.rs`
- `src/graft.rs`
- `src/symbols.rs`
- `src/query.rs`
- `src/lint.rs`
- `src/trace.rs`
- `src/zjx.rs`
- `src/main.rs`
- `tests/sley_v0.rs`
- `docs/schemas/*.schema.json`
- `fixtures/contracts/*.json`
- `fixtures/cli_smokes/manifest.json`
- `fixtures/corpus/**/*.json`
- `fixtures/grafts/*.json`
- `examples/*.sley`

Current verified surface:

- `cargo fmt --check` passes.
- `cargo test` passes.
- Current integration coverage is 192 tests.
- `sley query --json` emits `schema: "sley.query.report.v0"` and supports
  `--kind all|modules|tasks|types|effects|calls`, `--module <module>`, and
  `--exported`, including strict task/take/type/effect/call row definitions.
- `sley-contract` is available as an in-tree contract utility scaffold with
  `inventory`, `check-fixtures`, and `validate` JSON Schema validation commands
  over `docs/schemas/` and `fixtures/contracts/`.
- `sley lint --json` emits `schema: "sley.lint.report.v0"` and supports
  `--module <module>`, `--rule unused-private-task`,
  `--rule unreachable-private-task`, `--rule unused-declared-effect`,
  `--rule unused-import`, `--rule unused-take`, `--rule unused-private-type`,
  `--rule unused-private-effect`, `--rule raw-host-adapter`,
  `--rule missing-module-declaration`, `--rule unchecked-result`, and
  `--deny-warnings`.
- The current lint rules are `unused_private_task` and
  `unreachable_private_task`, `unused_declared_effect`, and
  `unused_import`, `unused_take`, `unused_private_type`,
  `unused_private_effect`, `raw_host_adapter`,
  `missing_module_declaration`, and `unchecked_result`.
- CLI smoke coverage is manifest-backed under
  `fixtures/cli_smokes/manifest.json`, including graph-slice replace
  affordances, checked `replace_expression` graft templates, and lint-driven
  declaration delete templates and cleanup transactions, including direct
  declaration surface targeting, direct graft JSON emission, and checked
  `sley fix` dry-run execution from `sley plan --graft-templates`, plus
  lint-driven missing-module declaration templates with target/project-aware
  module-name inference and checked fix dry runs, checked raw-host adapter
  migration templates that rewrite eligible raw calls to fallible `try_`
  calls with `?`, and typed deploy scaffold next-actions whose generated
  first-run sequence is executed by tests, including a strict seeded
  `sley verify --json --deny-warnings` readiness smoke for the generated
  deploy project and warning-state doctor/verify next-actions that route to
  `sley plan --json --graft-templates` plus unambiguous
  `sley fix --dry-run` previews carrying explicit `write_command` vectors with
  a staged write-and-verify smoke for a previewed repair plus a project-level
  previewed unused-import write-and-verify smoke and a generated deploy
  scaffold repair loop re-verified with seeded deploy authority, plus
  scaffold-level and passed-verify seal/ZJX handoff next-actions, doctor/plan
  call-bearing reports that route agents to strict call-row inspection, a
  write/query/verify smoke for the call-row-driven rename-and-update-call-sites
  transaction, a write/query/verify smoke for the unused-take-plus-call-arg
  removal transaction, and checked `unchecked_result` migration templates that
  turn discarded `Result` expression statements into explicit `?` propagation
  when valid, plus checked `unused_import` and
  `unused_private_task` delete templates, fix dry runs, and a write-mode
  `delete_unused_import` cleanup that clears lint before
  `sley verify --deny-warnings`, plus checked dead private task cleanup
  transactions for grouped unused/unreachable private task deletion, plus
  checked `unused_take` and `unused_declared_effect` remove templates and fix
  dry runs, plus manifest-staged temp files for write-mode CLI smokes,
  direct `sley graft --write --trace <path>` source mutation and receipt
  redirection, explicit `sley fix --write --trace <path>` receipt
  redirection, follow-up `sley trace --trace <path>` receipt inspection, and
  `sley seal --trace <path>` plus `sley zjx --trace <path>` over non-empty
  receipt chains with recomputable graph digests while
  `sley fix --dry-run --trace <path>` remains non-mutating.
- Stable JSON roots now include query reports, lint reports, doctor reports,
  edit-plan reports, verify reports, project scaffold reports,
  `sley-contract` utility reports, and the CLI smoke manifest in addition to
  AST, diagnostics, graph, graph slice, trace receipt, trace seal, graft
  outcome, and ZJX envelope roots. The edit-plan schema now pins strict graft
  operation and transaction template envelopes,
  the query schema now exposes strict task/take/type/effect/call row
  definitions,
  graph-slice affordance operations reuse that strict graft operation schema,
  graph-slice focus, task, and call summary payloads are schema-linked,
  the diagnostics schema exposes the shared diagnostic record used by
  graft/doctor/plan/verify report schemas, the graft outcome and trace receipt
  schemas pin accepted provenance records, and the ZJX envelope now carries a
  recomputable graph digest plus schema refs for graph, slice, and trace
  receipt handoff contents.

Product thesis:

Sley should make agents better programmers by replacing fragile raw text edits
with a tight compiler-mediated loop:

1. Read compact docs, examples, schemas, and manifests.
2. Inspect typed AST, symbol graph, query reports, lint reports, or graph
   slices.
3. Propose a strict structural graft or a narrow source change.
4. Run checker, formatter, query, lint, and relevant runtime smokes.
5. Read stable diagnostics, repair hints, query summaries, and lint findings.
6. Iterate until Loom accepts the change and the conformance surface is updated.
7. Leave human-reviewable source, machine-readable contracts, trace receipts,
   and content-addressed seals when applicable.

Hard truth:

Codex and other agents do not have broad Sley pretraining. Sley must teach
itself through tooling: schemas, graph slices, query reports, lint reports,
repair hints, examples, conformance fixtures, CLI smoke manifests, dry-run
grafts, deterministic runtime seeds, and stable diagnostics. If a feature
cannot be learned or verified through that loop, it is not agent-native yet.

Non-negotiable constraints:

- Source text is the human review projection.
- The typed graph is the canonical agent work surface.
- Prefer structural grafts over raw text edits when the operation fits the
  graft surface.
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
- Preserve unrelated local changes. This repo may already be dirty.
- Keep tests, schemas, contract snapshots, and CLI smoke manifests
  authoritative.

World-best criteria:

Sley is moving toward "best programming language for agents" only when it
improves measurable agent work, not when it merely accumulates syntax.

Judge work by:

- Can an agent inspect the relevant program slice without loading the whole
  codebase?
- Can an agent ask `sley query --json` for module, task, type, effect, and call
  facts before editing?
- Can an agent ask `sley lint --json` for warning-grade hygiene before and
  after edits?
- Can an agent propose a legal edit as a graft instead of patching raw text?
- Does Loom reject illegal edits with stable IDs, spans, node IDs, and repair
  hints?
- Does the formatter preserve a clean human-reviewable projection?
- Does `check --json` give enough information for the next repair attempt?
- Do query and lint JSON reports stay stable enough for future helper passes?
- Are effects explicit and enforced at check/runtime boundaries?
- Are runtime examples deterministic and reproducible?
- Are accepted and rejected examples locked into conformance tests?
- Are JSON schemas stable and versioned?
- Are CLI smoke expectations manifest-backed?
- Can future agents resume from `llms.txt`, schemas, manifests, examples, and
  tests without private chat context?

High-leverage work lanes:

1. Agent tool contract:
   - keep `parse`, `format`, `check`, `run`, `ast`, `graph`, `query`, `lint`,
     `trace`, `seal`, `zjx`, and `graft` stable;
   - make every JSON root schema-versioned;
   - keep `query` and `lint` suitable for tool-facing helper passes.

2. Query and lint consumption:
   - start consuming `sley.query.report.v0` and `sley.lint.report.v0` from
     Sley helper passes or deterministic helper tooling;
   - broaden authority, style, migration, and reachability lints only when the
     output contract is clear;
   - preserve `--deny-warnings` as the CI gate for warning-grade lint output.

3. Diagnostics and repair:
   - improve parse errors, unknown names, type mismatches, return mismatches,
     call arity/type errors, private imports, ambiguous imports, effect
     authority errors, stale grafts, unsupported graft operations, and lint
     findings;
   - include repair hints that suggest valid structural edits without bypassing
     authority.

4. Graft-first editing:
   - expand graph-slice grafts where tests prove safety;
   - harden project writeback beyond existing modules;
   - keep dry-run paths explicit and trustworthy;
   - preserve trace receipts and seals for accepted writes.

5. Conformance and contracts:
   - grow accepted/rejected fixtures;
   - lock contract snapshots under `fixtures/contracts/`;
   - keep JSON schemas under `docs/schemas/`;
   - keep CLI smoke coverage manifest-backed under
     `fixtures/cli_smokes/manifest.json`;
   - update schemas, snapshots, manifests, and tests in the same change when a
     JSON output contract changes.

6. Runtime authority:
   - keep seeded v0 adapters deterministic;
   - keep real external calls out of v0 tests;
   - preserve the distinction between authority diagnostics and recoverable
     `Result` host failures;
   - test both authorized and unauthorized paths for authority changes.

7. Self-hosting ladder:
   - keep Rust Loom as the strict oracle;
   - write Sley helper/lint/query passes only after the graph/query/lint
     contracts support them;
   - run self-hosted passes in shadow mode before promotion;
   - preserve a recovery oracle.

Default first moves in any new session:

1. Run:

```bash
cargo fmt --check
cargo test
```

2. Inspect the relevant command behavior before editing:

```bash
cargo run -- check --json examples/hello.sley
cargo run -- graph --json examples/hello.sley
cargo run -- ast --json examples/hello.sley
cargo run -- query --json --kind tasks examples/project
cargo run -- lint --json examples/hello.sley
```

3. If changing grafts, inspect or create a dry-run graft first:

```bash
cargo run -- graft --json --dry-run <target> <graft.json>
```

4. If changing query or lint output:

```bash
cargo run -- query --json --kind all <target>
cargo run -- lint --json <target>
cargo run -- lint --json --rule unused-private-task <target>
cargo run -- lint --json --rule unreachable-private-task <target>
cargo run -- lint --json --rule unused-private-type <target>
cargo run -- lint --json --rule unused-private-effect <target>
cargo run -- lint --json --rule raw-host-adapter <target>
cargo run -- lint --json --rule missing-module-declaration <target>
cargo run -- lint --json --rule unchecked-result <target>
cargo run -- query --json --kind types <target>
cargo run -- query --json --kind effects <target>
```

5. If changing runtime authority, test both the authorized and unauthorized
   path.

6. If changing JSON output, update schemas, locked contract fixtures, CLI smoke
   manifests, and tests in the same change.

Validation commands to prefer:

```bash
cargo fmt --check
cargo test
cargo run -- check --json examples/hello.sley
cargo run -- run --json examples/hello.sley
cargo run -- graph --json examples/hello.sley
cargo run -- query --json --kind tasks examples/project
cargo run -- lint --json examples/hello.sley
cargo run -- seal --json examples/hello.sley
cargo run -- zjx --json examples/hello.sley
cargo run -- run --json --cap DatabaseRead --db-table users=examples/users.json examples/db_gate.sley
cargo run -- run --json --cap DatabaseRead --cap DatabaseWrite --db-table users=examples/users.json examples/db_write_gate.sley
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
- JSON schema, contract, or manifest changes are updated deliberately.
- New diagnostics or lint findings include stable IDs and useful repair hints
  where practical.
- New query/lint behavior is covered by schemas, snapshots, CLI smokes, or
  direct tests.
- New graft behavior is covered by accepted and rejected tests.
- Runtime authority changes include positive and negative coverage.
- Documentation is updated only where it helps future agents use Sley
  correctly.
- The final summary states:
  - behavioral delta;
  - files changed;
  - validation run;
  - remaining risks;
  - next best task.
