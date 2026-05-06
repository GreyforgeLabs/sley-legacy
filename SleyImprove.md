# Sley Improvement Plan

Date: 2026-05-05
Status: working design note
Scope: agent usability, self-hosting path, compiler feedback loop, binding ontology governance

## Short Version

Sley should not depend on Codex already knowing Sley from training data.

The first Rust Loom should act like a strict teacher, referee, and translator:
it reads Sley, exposes typed graph shards, accepts or rejects grafts, and gives
machine-readable diagnostics. Agents learn Sley through that loop.

The language does not "live in Rust" forever. Rust is the trusted bootstrap
implementation. Sley lives in its specification, typed graph model, examples,
tests, diagnostics, and eventually self-hosted compiler passes.

## Explain It Like I Am 5

Think of Sley as a new game.

Codex has not played this game before, so we do not ask Codex to guess all the
rules from memory.

Instead, we give Codex:

- the rulebook: the Sley spec
- a picture of the board: the typed graph or AST
- legal moves: graft operations
- a referee: the Rust Loom
- correction notes: diagnostics and repair hints

At first, the referee is written in Rust because Rust is already strong,
stable, and known. That does not mean Sley is secretly Rust. It means Rust is
holding the training wheels while Sley grows.

Later, parts of the referee can be rewritten in Sley. When Sley can explain and
check more of itself, the training wheels come off one by one.

## Correct Mental Model

Wrong:

```text
Sley exists only inside the Rust compiler.
```

Better:

```text
Sley semantics -> implemented first by Rust Loom
Sley source -> human review projection
Sley graph -> canonical program structure
ZJX -> compact transport/cache envelope
Rust Loom -> bootstrap oracle and safety gate
Future Sley Loom passes -> self-hosted implementation pieces
```

The Rust compiler is the first trustworthy executable version of the rules. It
should stay in charge until Sley has enough tests, examples, graph stability,
diagnostics, and self-hosted passes to safely take over parts of itself.

## The Real Training Data Strategy

The training-data problem is real: a new language has little or no model
pretraining corpus.

Sley should reduce that problem by making the agent workflow structural instead
of relying on raw text prediction.

The agent should not need to "remember" Sley perfectly. It should be able to:

1. Read compact docs and examples.
2. Request a bounded graph shard.
3. Propose a graft operation.
4. Receive structured diagnostics.
5. Repair the graft.
6. Repeat until the Loom accepts it.

This makes Sley learnable in-context.

The goal is not to beat pretraining with vibes. The goal is to replace guessing
with a tight compiler-mediated feedback loop.

## Improvement 1: Agent-First Tool Contract

Codex and other agents need a small stable command contract:

```bash
sley parse --json <target>
sley check --json <target>
sley ast --json <target>
sley ast --json --node <node-id> <target>
sley graph --json <target>
sley graph --json --slice <node-id> <target>
sley new --json [--template hello|deploy] [--name <name>] [--module <module>] <path>
sley doctor --json [--deny-warnings] <target>
sley plan --json [--deny-warnings] [--graft-templates] [--template-surface <task>] <target>
sley verify --json [--deny-warnings] [runtime gates/seeds] <target>
sley query --json [--kind all|modules|tasks|calls] [--module <module>] <target>
sley lint --json [--rule unused-private-task|unreachable-private-task|unused-declared-effect|unused-import|unused-take|unused-private-type|unused-private-effect|raw-host-adapter] [--module <module>] <target>
sley trace --json <target>
sley seal --json <target>
sley zjx --json [--slice <node-id>] <target>
sley graft --json <target> <graft.json>
sley graft --json --dry-run <target> <graft.json>
sley format <target>
```

Rules:

- JSON output must be stable and versioned.
- AST roots, diagnostic reports, symbol graphs, graph slices, query reports,
  lint reports, doctor reports, verify reports, project scaffold reports, trace
  seals, ZJX envelopes, and graft outcomes carry v0 schema IDs.
- AST, diagnostic-report, graph-slice, query-report, lint-report,
  doctor-report, project-scaffold, and trace-seal snapshots are locked under
  `fixtures/contracts/`.
- JSON Schema files live under `docs/schemas/`; the AST schema covers nested
  declarations, statements, expressions, type expressions, spans, and
  provenance, while the remaining schema files are still root-contract v0
  shapes.
- Diagnostics include stable IDs, node IDs, spans where possible, and repair
  hints for common checker failures.
- `sley doctor` is the first deterministic helper consuming strict check,
  `sley.query.report.v0`, and `sley.lint.report.v0` into a single readiness
  report for pre-edit agent planning.
- `sley plan` consumes the same checked surfaces into ranked task edit
  surfaces, post-edit gate commands, and optional starter graft operation
  templates, rename-plus-call-site transactions, and add-take-plus-call-arg
  transactions, plus safe remove-take-plus-call-arg transactions for unused
  takes and lint-driven delete templates for unused private type/effect
  declarations, with editable JSON pointers; it also consumes selected
  graph-slice movement affordances as `move_statement`, `move_take`, and
  destination-variant templates when legal graph-slice destinations exist.
  Agents can target a specific task surface by node id or qualified name.
- `sley verify` is the deterministic CI/pre-deploy helper consuming strict
  check, `sley.query.report.v0`, `sley.lint.report.v0`, and seeded runtime
  execution into one pass/warnings/blocked report.
- Grafts support dry-run by default and an explicit `--dry-run` flag.
- Graft input JSON is strict: unknown operation or payload fields reject instead
  of being silently ignored.
- Accepted write-mode grafts emit trace receipts, and `sley seal` produces a
  content-addressed digest over source, graph, and receipt content.
- Runtime `Ok(value)`/`Err(error)` values and `?` propagation are implemented
  for Sley-level `Result` flow. Filesystem, seeded database reads, per-run
  database inserts, seeded network text responses, seeded shell command output,
  seeded model completions, seeded secret values, and seeded deployment stage
  results now expose fallible variants returning typed `Error` records.
- Source text should remain the human review projection, not the primary agent
  edit surface.

## Improvement 2: Strong Diagnostics And Repair Hints

Every common compiler rejection should be useful to an agent.

Good diagnostic shape:

```json
{
  "id": "EFFECT_UNAUTHORIZED",
  "severity": "error",
  "node": "task:app.profile.get_user",
  "span": {"line": 12, "column": 10},
  "message": "task `get_user` requires `DatabaseRead`",
  "repair_hints": [
    {
      "kind": "add_required_effect",
      "target": "task:app.profile.get_user",
      "effect": "DatabaseRead"
    }
  ]
}
```

Priority diagnostic families:

- parse errors with expected tokens: implemented with `insert_expected_token`,
  `provide_identifier`, `provide_expression`, and expected-item hints
- unknown identifiers: implemented with `declare_binding`
- unknown tasks: implemented with `declare_or_import_task`
- unknown types: implemented with `declare_or_import_type`
- binding and assignment type mismatch: implemented with type-change hints and
  structural `ReplaceExpression`
- condition, collection, index, and record-field expression mismatches:
  implemented with structural `ReplaceExpression` hints where the expected
  replacement type is clear
- record literal missing/unknown fields: implemented with whole-record
  structural `ReplaceExpression` hints that preserve known fields
- if branch type mismatch: implemented with alternative structural
  `ReplaceExpression` hints for the then and else branch expressions
- non-iterable `each` collections and invalid `len` arguments: implemented with
  conservative structural `ReplaceExpression` starter hints
- non-indexable collection expressions: implemented with structural
  `ReplaceExpression` hints when the index type selects list or map shape
- unary and binary operator mismatches: implemented with structural
  `ReplaceExpression` hints where a specific operand replacement type is clear
- call arity mismatch: implemented with `match_task_arity`, `UpdateCallArgs`,
  and `RemoveCallArg`
- call argument type mismatch: implemented with `replace_argument` and
  caller-scoped `ReplaceCallArg`
- call arity contraction after parameter removal: implemented with
  `RemoveCallArg` and safe remove-take transaction templates for unused takes
- return type mismatch: implemented with `change_return_type`,
  `replace_return_expression`, and structural `ReplaceExpression`
- missing return paths in non-`Unit` tasks: implemented with `MISSING_RETURN`,
  `insert_return`, and `replace_task_body`
- immutable binding mutation: implemented with `use_mutable_binding_kind`
- undeclared effects: implemented with `declare_or_import_effect`
- unauthorized host authority: implemented with `add_required_effect`
- private imported task/type/effect: implemented with export hints
- ambiguous imported task/type/effect: implemented with qualification hints
- stale graft preconditions: implemented with `refresh_graft_precondition`
  hints that tell agents to re-read current target state before retrying
- unsupported graft operation: implemented with `use_supported_graft_operation`
  hints, plus `replace_expression` guidance for expression move/delete attempts
- module namespace conflicts: implemented with `resolve_namespace_conflict`
  hints for duplicate checker declarations and graft add/rename collisions

## Improvement 3: Graft-First Editing

Agents should be trained to prefer grafts over raw file edits.

Minimum useful graft operations:

- `AddTake`
- `RemoveTake`
- `ReplaceTaskBody`
- `AddTask`
- `RenameDeclaration`
- `AddImport`
- `AddEffectDeclaration`
- `AddTypeDeclaration`
- `UpdateCallSites`
- `UpdateCallArgs`
- `ReplaceCallArg`
- `RemoveCallArg`
- `InsertStatement`
- `ReplaceExpression`
- `MoveNode`
- `DeleteNode`

`UpdateCallSites`, `UpdateCallArgs`, `ReplaceCallArg`, `RemoveCallArg`,
`InsertStatement`, `ReplaceExpression`, `DeleteNode`, and `MoveNode` are now
implemented for the v0 in-memory checked program. `DeleteNode` supports checked
deletion of declarations, imports, takes, and statements.
`MoveNode` supports checked in-parent statement reordering and top-level
declaration ordering. Project-aware multi-file writeback now updates existing
module files, creates checked new module files declared by the graft candidate,
and deletes removed module files once the checked candidate no longer imports
them. It also supports module rename by rewriting module ownership and imports,
creating the renamed checked module file, and deleting the old loaded module
file. Entry-module rename updates `sley.toml` during checked project writeback.
Bare imports to missing modules still reject before mutation. Top-level import,
type, effect, and task movement into known modules is implemented for checked
project writeback, including AddImport-then-MoveNode transactions that create
the destination module file from the moved item. Cross-parent statement
movement between existing block parents is implemented with
`payload.destination`. Take reordering within the owning task and checked
cross-task take movement into another task's take list are implemented.
Expression movement and broader graph-contract hardening remain open.
Unsupported expression movement returns a `replace_expression` repair hint so
agents can plan the supported structural edit.
Graph, graph-slice, and query module import summaries now expose canonical
import node ids so agents can copy import graft targets directly from the
machine contract instead of reconstructing them.
Graph slices also expose bounded `MoveNode` affordances for import, type,
effect, task, statement, and take movement planning, including exact parent
ids and destination insertion limits, starter operation JSON, and editable JSON
pointers. They also expose bounded `DeleteNode` affordances for import, type,
effect, task, statement, and take deletion planning, plus `ReplaceExpression`
affordances for task-local expression replacement; `sley plan
--graft-templates` filters selected task-internal delete and expression replace
templates through the checker before surfacing them.

The important rule is not that all operations exist immediately. The important
rule is that unsupported operations reject cleanly with explicit diagnostics.

## Improvement 4: Self-Hosting Ladder

Do not jump from Rust bootstrap to full Sley self-hosting in one leap.

Use a staged ladder:

1. **Rust-only oracle**
   Rust Loom parses, checks, formats, runs, and applies grafts.

2. **Sley standard library**
   Pure libraries and examples are written in Sley and validated by Rust Loom.

3. **Sley compiler helpers**
   Non-authoritative helper passes are written in Sley: graph queries,
   lint rules, migrations, fixture generators, and docs examples.

4. **Shadow compiler passes**
   Sley implementations run beside Rust implementations and must match output.

5. **Promoted Sley passes**
   Specific compiler passes become authoritative after conformance evidence.

6. **Self-hosted Loom core**
   Sley can build, inspect, and evolve large parts of its own compiler.

7. **Rust remains recovery oracle**
   Even after self-hosting, keep a small Rust or frozen reference implementation
   as the recovery checker until the ecosystem is mature.

## Improvement 5: Binding Ontology Governance

Agents may propose new binding kinds, but they should not directly mutate the
stable core ontology.

Core binding kinds should be human-ratified and versioned.

New binding kind proposal template:

```text
name:
status: proposed | experimental | stable | deprecated
purpose:
mutability:
authority behavior:
lifetime behavior:
sharing behavior:
checker rules:
runtime rules:
serialization shape:
example:
counterexample:
migration impact:
why existing binding kinds are insufficient:
```

Recommended process:

1. Agent proposes a binding kind as an experimental extension.
2. Loom validates that the proposal has semantics, examples, and checker rules.
3. Human reviews whether it deserves core status.
4. Experimental use collects evidence.
5. Promotion requires tests, docs, examples, and migration notes.

This keeps the ontology from becoming random model-generated vocabulary.

## Improvement 6: Canonical Agent Onboarding Pack

Codex will use Sley better if each repo exposes a compact onboarding pack:

- `README.md`
- `docs/SleyLanguageSpec.md`
- `llms.txt` or equivalent compact model-facing summary
- `examples/*.sley`
- `fixtures/grafts/*.json`
- `tests/*`
- `sley check --json`
- `sley ast --json`
- `sley graft --json`

The pack should include a short "agent rules" section:

```text
Prefer grafts over raw text edits.
Run `sley check --json` after each proposed change.
Use `sley ast --json --node` before editing a bounded target.
Do not invent binding kinds.
Do not add effects to silence errors unless the authority is semantically real.
Keep source formatting controlled by `sley format`.
```

## Improvement 7: Synthetic Gold Corpus

The synthetic corpus lives under `fixtures/corpus/`. It has a
`manifest.json` that lists each fixture and its coverage tags. It should keep
growing deliberately, with accepted fixtures that must parse/check/round-trip
and rejected fixtures that lock expected diagnostic IDs. The current corpus
already covers declared and missing authority for all deterministic seeded host
adapters.

Corpus categories:

- small pure functions
- records and slots
- list and map logic
- result flow with `?`
- effect propagation
- authority and gate examples
- successful grafts
- rejected stale grafts
- rejected unauthorized grafts
- refactor migrations
- module namespace examples
- formatter round trips
- seeded host authority acceptance and rejection

Each corpus item should include:

- source
- AST or graph shard
- expected diagnostics
- accepted grafts
- rejected grafts
- final formatted output
- human explanation

The manifest is part of the release gate: new corpus files should not be added
silently outside it, and required release coverage tags should remain explicit.

This corpus becomes the real bridge from "Codex does not know Sley" to "agents
can operate Sley reliably."

## Improvement 8: CLI Smoke Conformance

The executable CLI smoke suite lives under `fixtures/cli_smokes/`. Its
`manifest.json` lists stable command lines, optional temp-directory execution,
coverage tags, stdout substrings, and JSON pointer/value expectations. The
Rust integration suite runs those cases against the built `sley` binary.

The current smoke manifest covers:

- parse, format, check, run, ast, graph, graph-slice, query, lint, trace, seal,
  zjx, and graft dry-run commands
- stable JSON roots for AST programs, diagnostics, symbol graphs, graph slices,
  query reports, lint reports, trace seals, graft outcomes, and ZJX preview
  envelopes
- graph-slice replace affordances and checked `replace_expression` graft
  templates in edit-plan reports
- lint-driven declaration delete templates in edit-plan reports
- private declaration hygiene through the checked `unused_private_type` and
  `unused_private_effect` lint rules
- import hygiene through the checked `unused_import` lint rule
- API hygiene through the checked `unused_take` lint rule
- deterministic seeded execution for `FileRead`, `FileWrite`, `DatabaseRead`,
  `DatabaseWrite`, `Network`, `Shell`, `ModelCall`, `SecretRead`, `Deploy`,
  and `Spend`
- temp-directory execution for the file-write case so release tests do not
  mutate the repo checkout

## Improvement 9: Checked Graph Query Reports

`sley query` is the first explicit tooling command on top of the checked symbol
graph. Unlike `sley graph`, it runs the checker before emitting the report, so
tool consumers do not treat an invalid program as a reliable semantic surface.

The v0 query report carries `schema: "sley.query.report.v0"` and supports:

- `--kind all|modules|tasks|calls`
- `--module <module>` filtering
- `--exported` filtering for declaration and task summaries
- task rows with stable ids, qualified names, takes, return types, declared
  effects, and inbound/outbound call counts
- call rows reused from the symbol graph call summary

This is the immediate substrate for lints, migration hints, project dashboards,
and eventually non-authoritative Sley helper passes.

## Improvement 10: Checked Lint Reports

`sley lint` is the first checked helper command built from the graph query
surface. It runs the checker before linting, emits
`schema: "sley.lint.report.v0"`, and keeps warning-grade lint output separate
from hard checker diagnostics.

The v0 lint rules are:

- `unused_private_task`: a non-exported task with zero inbound checked calls is
  reported unless it is the entry module's `main` task.
- `unreachable_private_task`: a non-exported task with inbound private calls is
  reported when it is not reachable from `main` or any exported task.
- `unused_declared_effect`: a task-declared effect is reported when no direct
  host call or resolved called-task effect justifies it.
- `unused_import`: an import is reported when no checked task, type, or effect
  uses it.
- `unused_take`: a normal task take is reported when the task body never reads
  it.
- `unused_private_type`: a non-exported type is reported when no checked task,
  type, or record literal references it.
- `unused_private_effect`: a non-exported effect is reported when no checked
  task declares it.
- `raw_host_adapter`: a legacy diagnostic-failing host call is reported when a
  fallible `try_` adapter should replace it.

The command supports `--module <module>`, `--rule unused-private-task`,
`--rule unreachable-private-task`, `--rule unused-declared-effect`,
`--rule unused-import`, `--rule unused-take`, `--rule unused-private-type`,
`--rule unused-private-effect`, `--rule raw-host-adapter`, and `--deny-warnings`
lets CI turn findings into a failing exit after the JSON report is printed.

This is not production lint coverage yet. It is the first stable surface for
agent-facing hygiene, authority lints, migration hints, and eventually
non-authoritative Sley helper passes that consume `sley.query.report.v0` and
`sley.lint.report.v0`.

## Improvement 11: ZJX Boundary Discipline

ZJX should be the transport/cache envelope, not the semantic identity, until the
canonical graph encoding and canonical ZJX encoding are frozen.

Correct pipeline:

```text
canonical graph bytes -> semantic hash
canonical graph bytes -> ZJX envelope
ZJX envelope -> transport/cache/storage
```

Do not make the compressed envelope define the language semantics too early.
That would couple language correctness to compression implementation details.

## Improvement 12: Safety Against Bad Self-Evolution

The recursive loop needs brakes.

Hard gates:

- no accepted graft without parse/check success
- no compiler change without conformance tests
- no ontology change without human ratification
- no authority/effect change without explicit semantic reason
- no public claim that Sley is production-ready until runtime and host gates are
  mature
- no self-hosted pass becomes authoritative until it matches the Rust oracle on
  a large fixture set

Failure modes to watch:

- agents adding effects just to silence diagnostics
- binding vocabulary bloat
- unstable JSON schemas
- text edits bypassing graft provenance
- ZJX format churn becoming language churn
- self-hosted passes matching happy paths but missing rejection cases

## Practical Next Milestones

Near-term:

1. Harden capability-backed host adapters beyond root-scoped filesystem gates,
   deterministic seeded database surfaces, seeded network text, seeded shell
   command output, seeded model completions, seeded secret values, seeded
   deployment stage results, and seeded spend authorizations. Preserve
   `Result<T, Error>` surfaces for recoverable host failures and keep
   authority failures as diagnostics.
2. Grow the accepted/rejected synthetic gold corpus and CLI smoke manifest with
   graft, module, and runtime authority cases.
3. Start consuming `sley.query.report.v0` and `sley.lint.report.v0` from Sley
   helper passes, then broaden authority, style, and migration lints.
4. Extend graph-slice grafts around move, delete, and replace planning.
5. Harden graph-slice grafts for richer take movement planning, expression edit
   planning, and broader graph-contract checks.

Medium-term:

1. Extend graph-slice grafts beyond call-site, statement, expression, move, and
   delete edits.
2. Move trace seals and ZJX preview payloads into a compressed binary `.zjx`
   handoff.
3. Expand runtime host capability values beyond seeded v0 adapters: deploy
   beyond seeded stage results, spend beyond seeded authorization text, secret
   beyond seeded values, model beyond seeded completions, shell beyond seeded
   command output, network beyond seeded text, and database write beyond
   per-run inserts.
4. Broaden release gates beyond the synthetic gold corpus and CLI smokes into
   packaged examples and migration fixtures.
5. Shadow selected compiler helper passes in Sley.

Long-term:

1. Write Sley lints and graph query helpers in Sley.
2. Shadow Rust checker passes with Sley equivalents.
3. Promote self-hosted passes only after conformance evidence.
4. Keep a frozen recovery oracle.

## Bottom Line

Keeping the first Loom in Rust is not a retreat from Sley. It is how Sley gets a
strict, trustworthy teacher while agents learn the language.

The path is:

```text
Rust bootstrap -> structural agent loop -> gold corpus -> Sley helper passes ->
shadow compiler passes -> promoted self-hosting -> recovery oracle
```

That is the safe version of an agent-native language that can eventually improve
itself without turning into unreviewable model-written mush.
