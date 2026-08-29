# Sley 1.2 current-state audit

Status: verified through W7 implementation; final release gates run last
Owner: Sley maintainers
Date: 2026-08-26

## Decision

The AI-foundation branch is the correct implementation base. It contains the
public 1.1 line plus the bounded machine protocol, version-coupled AI bootstrap,
governed SleyBench evaluator/split, corpus governance, deterministic Siglum
support intrinsics, and the accepted/rejected Bootstrap evidence on one clean
line.

The first implementation deficiency is compiler truth, not additional prompt
guidance. Current probes demonstrate:

| Source shape | AST result | `sley check` before W1 | Required 1.2 result |
|---|---|---|---|
| `true and false` | executable expression stored as `Raw` | `ok` | actionable unsupported-expression error |
| `if false then 1 else 8` | executable expression stored as `Raw` | `ok` | conditional-spelling error with valid form |
| module-level `if` | omitted from task/declaration AST | `ok` | invalid module control-flow error |
| `task double take value: Int -> Int` | `take` absorbed into task name | `ok` | task-parameter-placement error |

## Existing strengths

- Sley-owned parser/checker/runtime/bootstrap modules drive self-hosting markers
  and diagnostic descriptor data.
- `sley check --json`, AST, bounded graph/query, plan, dry-run graft/fix,
  verify, trace, and seal already expose a strong machine substrate.
- The diagnostics schema supports stable IDs, spans, nodes, and repair hints.
- Validation has explicit quick, changed, core, and full tiers.
- The read-only MCP bridge and benchmark controller already enforce bounded
  roots, paths, time, bytes, and tool shapes.
- SleyBench keeps failures in the denominator and separates public/private
  evidence from training and retrieval.

## Gaps ordered by release dependency

1. Checker success does not imply that every executable expression has a
   supported representation.
2. Parser/checker output does not expose invalid module-level executable
   control flow.
3. Task header parsing accepts an inline `take` shape as a task name.
4. Expression spelling and context are documented but not mechanically
   explainable from diagnostic output.
5. The combined SleyBench mode-summary gap is closed in W3 by an exact
   registered shape and synthetic public fixture.
6. Four minimum stable v1 compatibility targets now exist. Transaction, local
   grant and worker contracts remain intentionally deferred to W4/W5.

## Protected invariants

- `Raw` may remain a loss-preserving recovery representation.
- Runtime failure for unsupported source remains fail-closed.
- Bootstrap 0.2 remains authoritative during W1.
- SleyBench v0 cases, oracles, split, and held-out custody remain frozen.
- Host shell code may execute generic descriptors but must not become the
  semantic owner of new Sley diagnostic identities or rules.
- No new syntax is inferred from benchmark convenience.

## Repository and validation evidence

- Start HEAD: `cb9d714`
- Worktree: clean before planning changes
- Upstream: `origin/public`, ahead 18
- Other worktrees were inspected and left unchanged.
- Tier 1 baseline: `make quick`, 48.60 seconds, passed.
- Tier 3: skipped; the full gate is not an inner-loop planning check.

## W1 closeout evidence

- The four pre-W1 probes now produce stable error diagnostics with nodes,
  spans, messages, and repair hints.
- Executable `Raw` traversal includes nested call arguments. A host-adapter
  line cannot hide an unrelated unsupported argument.
- Closed recovery patterns preserve accepted unit, multiline-map,
  record-literal, and known host-adapter parser artifacts.
- Per-file task-context rows prevent same-module/same-line collisions from
  suppressing module-level control-flow errors in project checks.
- Corpus: 94/94 steps passed, comprising 23 accepted and 48 rejected cases.
- Tier 2 `make core` passed with 146/146 contract fixtures and 222 declared
  integration checks.
- Vulcan found one medium cross-file correlation defect, verified its repair,
  and returned commit-ready with no remaining findings.

## W2 closeout evidence

- `sley explain` resolves exactly the four W1 diagnostic IDs from a Sley-owned
  catalog and renders human and JSON answers from one report model.
- `sley.explain.report.v0` carries checked spelling, context, repair path,
  specification and example references, plus the implementation/release and
  retained Bootstrap 0.2 version context.
- Unknown diagnostic IDs fail closed with `UNKNOWN_DIAGNOSTIC_ID`.
- The LSP command-preview contract points to the same explain invocation; it
  does not duplicate diagnostic semantics.
- The accepted W2 corpus fixture covers Boolean spelling, value and statement
  conditionals, body-level `take`, and formatter round trip. No grammar,
  runtime, recovery-AST, or pinned `SLEY_AI.md` change was made.

## W3 implementation evidence

- Four stable v1 schema targets preserve the v0 diagnostics, bounded graph,
  verification, and machine-response shapes without changing current
  producers or claiming cutover.
- The stability registry classifies those roots as stable, current benchmark
  evidence roots as extensible, and every other inventory root as experimental
  by default. The first W4 inspection root is now extensible; W5
  worker/provider namespaces remain reserved and absent from the inventory.
- The strict protocol replay gate exercises the historical nested-owned-path
  and concatenated-response defects plus one-action, sequencing, six budget,
  unowned-path, and cancellation-observation boundaries.
- The exact retained K3 semantic summary validates against the registered
  mode-summary schema. The repository fixture is synthetic and exposes no
  private cases, oracle values, model responses, or custody metadata.
- A deterministic semantic check additionally locks pass/fail totals,
  public/private totals, and rate arithmetic that JSON Schema cannot express
  portably as cross-field constraints.
- The schema inventory contains 66 roots and all 155 public contract fixtures
  validate. Tier 2 passed, and Vulcan returned commit-ready with no
  report-grade findings after focused independent replay.

## W4 first-slice evidence

- `loom.transaction` is the semantic owner of transaction lifecycle states,
  failure states, initial transitions, explicit base mode, and mutation
  boundaries.
- `sley change inspect` binds one read-only transaction to repository identity,
  HEAD, source/graph/trace digests, compiler/contracts, actor, goal, nonce, and
  time bounds. It reports only `opened -> inspected`.
- `sley.transaction.inspect.v0` is strict, registered as extensible, and has a
  synthetic public fixture. The broad W4 reservation is removed because a real
  root now exists.
- The focused gate validates the live report, proves source and Git-index
  digests do not change, and fails closed across authority, time, symlink,
  repository, trace-boundary, and unsupported-apply cases.
- Inventory now contains 67 schemas and all 156 public contract fixtures
  validate. Atomic preview/apply/rollback and MCP write parity remain absent,
  so no complete governed-write claim is supported.
- Vulcan's independent review found one raw Git failure for repositories with
  an unborn HEAD. The CLI now emits `TRANSACTION_BASE_COMMIT_REQUIRED`, the
  focused regression passes, and re-review returned commit-ready with no
  remaining finding for that blocker.

## W4 immutable-preview evidence

- `sley change plan` rebinds a valid inspection to an explicit repository and
  strict all-or-nothing operation document. Its digest covers compiler facts
  and separately labeled operator rationale.
- `sley change preview` revalidates the plan and base, applies the exact ordered
  operations only in an ephemeral copy, normalizes all candidate paths, and
  deletes that copy before returning.
- The preview exposes complete candidate source, unified source diff,
  graph/type/call/effect/authority changes, diagnostics, focused compiler
  checks, validation results, and blocking review requirements.
- Repeated previews are byte-identical while repository source and Git-index
  digests remain unchanged. Approval and apply subcommands remain typed
  failures.
- Exact grants, repository writes, rollback, sealing, and MCP parity remain
  absent, so this evidence supports immutable preview only.
- The current inventory contains 69 schemas and 158 public contract fixtures.
- Vulcan's independent review found that inspection accepted single-file
  targets while preview initially assumed a directory and leaked raw copy
  failure. Preview now has typed file/project copy branches, stable single-file
  source evidence, and an end-to-end no-mutation regression. Re-review returned
  commit-ready with no report-grade finding.

## W4 exact-local-grant evidence

- `sley change approval-request` accepts only a valid immutable preview plus an
  explicit exact repository root. It recomputes candidate identity, verifies
  repository identity and HEAD, checks expiry and required validation, and
  derives exact operation, node, path, effect, authority, review, resource, and
  budget scope.
- `sley change approve` requires the original preview and regenerates the
  approval request before exact comparison. A schema-valid request with a new
  self-digest cannot widen its scope.
- Approval requires an explicit confirmation, issuer, principal, audience,
  purpose, nonce, bounded time, every review acknowledgement, a `not_revoked`
  assertion, and a wall-clock budget. Candidate text and tool metadata cannot
  synthesize these inputs.
- Issuer identity is recorded as an explicit CLI assertion. Optional signature
  and GreyNucleus references remain unverified, and local grants do not depend
  on GreyNucleus availability.
- Replay is bound to repository, base, candidate, principal, audience, and
  nonce, but consumption is honestly deferred until S12-404. Working-tree
  freshness, revocation re-checking, atomic apply, rollback, and MCP parity
  remain unimplemented.
- Inventory now contains 71 schemas, 160 public contract fixtures, and 237
  declared integration checks. Request and grant issuance leave repository
  source, Git index, traces, external systems, and provider state unchanged.
- Vulcan independently reviewed issuer spoofing, confused-deputy and
  self-digest boundaries, exact regeneration, scope, replay honesty, proof
  labels, and no-mutation behavior. The result was commit-ready with no high or
  medium finding; apply-time enforcement remains assigned to S12-404.
- Tier 2 `make core` passed across focused compiler checks, all 160 fixtures,
  compatibility and protocol replay, transaction inspection and exact grant
  lifecycle, conformance, AI-foundation drift, corpus governance, smoke 5/5,
  and self-hosted syntax. The full release gate remains deferred.

## W4 atomic apply and recovery evidence

- Apply authority binds one exact grant to source and internal transaction
  scopes. Active revocation and single-use replay are checked before the sole
  Linux same-filesystem common-root exchange.
- Durable recovery records are fsynced before the commit point. Recovery
  classifies live and preserved base/candidate placement from source and graph
  digests instead of trusting journal phase alone.
- Candidate source, graph, compiler check, and compiler lint are required.
  Failure completes a verified exchange rollback, and explicit manual rollback
  restores only an unchanged verified candidate.
- Cancellation, three crash points, replay-marker tampering, topology drift,
  stale source, revoked authority, automatic rollback, recovered rollback, and
  manual rollback pass focused regressions.
- Inventory reached 76 schemas, 165 public contract fixtures, and 242 declared
  integration checks. Tier 2 and independent Vulcan review passed with no
  remaining report-grade finding.

## W4 terminal review and seal evidence

- `sley change review` revalidates the exact historical preview, request,
  grant, authorization, apply, durable recovery, and optional manual rollback
  chain. Later expiry does not reactivate or invalidate already consumed
  authority.
- Only exact verified candidate-live/base-preserved or verified
  base-live/candidate-preserved placements are eligible. Active, tampered,
  inconsistent, or digest-mismatched evidence fails before publication.
- Optional trace receipts are read once, bounded, and individually validated.
  The generated Markdown packet contains no source projection or raw trace
  payload.
- Review holds the cooperative transaction lock, rechecks final placement, and
  publishes four fsynced artifacts with `renameat2(RENAME_NOREPLACE)`. Existing
  packets are never replaced, and manual rollback refuses a sealed
  transaction.
- Review, packet, ZJX, trace, apply, rollback, bundle, final source, and final
  graph digests are bound by `sley.change.transaction_seal.v0` without changing
  `sley.trace.seal.v0` or frozen ZJX semantics.
- Focused apply and rollback packet tests, typed tamper/non-terminal/trace/replay
  failures, source/index/recovery no-mutation checks, Tier 1 quick validation,
  78 schemas, 167/167 fixtures, and 244/244 conformance checks pass. Tier 2
  `make core` passed the full transaction lifecycle, conformance,
  AI-foundation drift, corpus governance, smoke 5/5, and self-hosted syntax.
- Vulcan found one medium pre-lock seal-check race in manual rollback. The
  repeated seal check and live topology/digest validation now execute under the
  shared transaction lock, and a deterministic concurrent review/rollback
  regression passes. Focused re-review returned commit-ready with no remaining
  High or Medium finding.

## W4 governed MCP parity evidence

- `sley-mcp-bridge` exposes nine named transaction adapters over the exact CLI
  commands. It fixes repository selection to the configured Git root, accepts
  only confined regular non-symlink artifact paths, builds argv arrays, and
  exposes no generic shell or change dispatcher.
- Operator authority issuance remains outside the agent tool catalog:
  `sley change approve` and `sley change revocation-record` are CLI-only. MCP
  mutation tools require explicit confirmations and publish truthful
  destructive or internal-write annotations.
- Success reports and typed CLI diagnostics remain exact structured content.
  Recovery honestly permits either its recovery or reconstructed apply report.
  The synchronous bridge continues to promise timeout bounds, not live
  active-call cancellation.
- The focused MCP gate passes catalog/lifecycle/framing, injection, confinement,
  size and timeout limits, byte-identical read-only CLI parity, confirmation
  denial, verified apply and verification, terminal review publication, and
  post-seal rollback denial.
- Tier 2 `make core` passes with 78 schemas, 167/167 fixtures, 244/244
  integration checks, the full transaction lifecycle, conformance,
  AI-foundation drift, corpus governance, smoke 5/5, and self-hosted syntax.
  Vulcan independently reran the MCP gate, found no High or Medium issue, and
  returned commit-ready.

## W5 minimum local adapter/replay evidence

- `loom.adapter` now owns the v0 manifest, replay, and report identities plus
  lifecycle, event, failure, seed, effect, authority, and replay vocabulary.
- `sley adapter replay` confines manifests, targets, and optional record pins
  to regular repository paths; validates canonical manifest, adapter, runtime,
  target, capability, seed, and record identities; and delegates execution to
  the existing deterministic seeded verifier.
- Text seeds are digest-projected before verifier dispatch, preventing raw
  values from entering child process arguments. Directory targets reject every
  nested symlink so runtime input, static accounting, and target digests cannot
  disagree about the confined source set.
- Reports and replay records deny live providers and external mutation, expose
  no seeded values, and label OS isolation and persistence as unimplemented
  S12-502 work.
- The focused gate passes exact happy replay and record matching, missing
  capability, scope, seed, replay, cancellation, budget, live-provider,
  manifest-tamper, record-tamper, schema, redaction, and source/index
  no-mutation cases.
- Tier 2 `make core` passed with 81 schemas, 170/170 contract fixtures, 247/247
  integration checks, and all 30 declared v1 targets present. The full
  transaction lifecycle, conformance, AI-foundation drift, corpus governance,
  smoke 5/5, and self-hosted syntax also passed.
- Vulcan's focused re-review confirmed that raw seeded text is digest-projected
  before child verifier arguments and nested target symlinks fail closed before
  accounting, digest, or runtime. The focused adapter gate passed and the
  review returned commit-ready with no findings or blockers.
- S12-501 is closed. OS isolation and persistent workers remain explicitly
  deferred to S12-502; the full `make v1` gate remains deferred to the W5
  integration/release boundary.

## W5 persistent bounded worker evidence

- `loom.worker` owns the stable `sley.worker.v1` request, response, event, and
  session identities plus operations, lifecycle, failure classes, contract
  bindings, isolation class, and disabled-cache policy.
- The controller accepts one invoke at a time and delegates to the existing
  adapter replay path in a fresh process group with a private temporary home,
  clean environment, closed inherited descriptors, and Linux resource limits.
- Every request binds worker/runtime/package/source identities or explicit
  absence, exact contract versions, entry point, authority, nonce, and wall,
  step, call-depth, collection, output, and memory budgets.
- Each invoke uses a private immutable input snapshot. Copied and returned
  package/source digests must match the request. Combined stdout/stderr is
  drained incrementally and retains at most the output budget plus one overflow
  sentinel byte.
- The focused session passes happy replay, strict negative bindings,
  idempotency replay denial, output flood, post-preflight source mutation,
  cancellation, fresh state, crash containment, health, drain/reset, and
  graceful shutdown. All messages validate and private seed values are absent.
- Vulcan found two Medium blockers in the first review: post-buffer output
  enforcement and original-path TOCTOU. Both were fixed with bounded draining
  and immutable snapshots plus exact regressions. Focused re-review found no
  remaining High or Medium issue and returned commit-ready.
- Tier 2 `make core` passed with 85 schemas, 174/174 contract fixtures, 251/251
  integration checks, all 31 declared v1 targets, the full transaction,
  adapter, and worker lifecycles, conformance, AI-foundation drift, corpus
  governance, smoke 5/5, and self-hosted syntax.
- S12-502 is closed. S12-503 now owns the completed Python and Node client
  follow-up. The full `make v1` gate remains deferred to the W5 integration/
  release boundary.

## W5 generated worker client evidence

- `clients/generate-worker-clients.py` derives Python `TypedDict` models,
  TypeScript read-only types, and Node runtime constants from the four stable
  worker schemas. Drift checking prevents the checked-in clients from becoming
  another contract authority.
- The Python wheel and private Node package have no runtime dependencies. They
  start an explicitly selected `sley worker start --json` command and expose
  exact builders and methods for handshake, load, capabilities, invoke,
  cancel, reset, drain, health, and shutdown.
- Both transports retain ordered messages under an explicit host bound, reject
  invalid UTF-8 or schema/protocol discriminators, require increasing event
  sequence numbers, correlate exactly one response to one pending request, and
  fail closed after reader errors.
- Cancellation remains a separate control request and never suppresses the
  authoritative invoke result. Neither client retries automatically. A
  retryable child-crash response is observed once, and worker request counts
  prove no hidden replay.
- Clean offline wheel and npm installs pass handshake, metadata, successful
  replay, deterministic source mismatch, process-group cancellation, child
  crash, health, graceful shutdown, private-value exclusion, and injected
  reader-failure regressions. A strict NodeNext TypeScript consumer validates
  the packed declarations.
- The first focused review found source-tree Python build residue. Building
  from a temporary copy closed it and the gate now proves Git status remains
  unchanged. The second review found a Node send-after-reader-error hang. Both
  clients now reject at send time, and fake-worker regressions pass. Vulcan's
  final review found no High or Medium issue and returned commit-ready.
- The initial Tier 2 `make core` run passed every target through smoke before
  the self-hosted syntax gate rejected the new host-language files. The gate
  now permits only the exact Python/Node client and generator paths while
  retaining Sley-only compiler/runtime ownership. The affected client,
  conformance, syntax, planner, format, and diff targets passed afterward with
  85 schemas, 174/174 fixtures, 251/251 integration checks, smoke 5/5, and all
  32 declared v1 targets.
- S12-503 is closed. First-class `sley test` is S12-504; validation reports are
  S12-505. The full `make v1` gate remains deferred to the W5 integration
  boundary.

## W5 first-class user-test evidence

- `loom.testing` owns stable manifest, report, and read-only review-packet
  identities plus deterministic discovery, case, coverage, property,
  differential, and authority vocabulary.
- `sley test` discovers `sley.test.json` and `*.sley-test.json` in byte order,
  rejects symlinked or escaped source, enforces exact source inventories and
  pre-execution bounds, and reuses machine, run, adapter, and check semantics.
- The fixture suite passes table, property, effect mock, adapter replay,
  expected diagnostic, and translation differential cases without arbitrary
  shell or provider execution.
- Reports separate observed, declared, unsupported, unknown, credited, and
  uncovered task, branch, effect, and capability evidence. Branch credit is
  explicitly a passing manifest witness rather than runtime instrumentation.
- Changed-node focus uses per-request reverse-caller graph expansion. Mixed
  covered and uncovered requests retain a typed failing uncovered node.
- Review binding recomputes the native test, review, seal, and full-bundle
  identities before binding a passing report to the exact terminal source.
  Schema-valid tampering is rejected.
- Vulcan's first review found a High mixed-node false-pass and Medium nested
  source-symlink gap. Both were fixed with per-request expansion, resolved-path
  confinement, regular-file checks, and exact regressions. Re-review found no
  residual report-grade code issue, although its duplicate validation run
  exceeded the council time budget.
- Tier 2 `make core` passed with 88 schemas, 177/177 fixtures, 254/254
  self-hosted checks, all 33 declared v1 targets, full transaction, adapter,
  worker, client, and user-test lifecycles, conformance, AI-foundation drift,
  corpus governance, smoke 5/5, and source purity.
- S12-504 closed the first-class user-test dependency. S12-505 validation
  evidence and the W5 integration boundary are recorded below.

## W5 validation-profile evidence

- `loom.validation` owns the quick, changed, core, and release profiles, one
  ordered check/subsystem registry, skip reasons, authority mode, cache policy,
  and stable `sley.validation.report.v1` identity.
- `sley validate` is restricted to the Sley repository root. It rejects
  ambiguous profile grammar and escaped changed paths, reuses the existing
  changed classifier, and delegates aggregate execution to Make.
- Reports carry every required selection, skip, release, cache, wall-time, and
  outcome field. Executor logs are drained but retained only to an 8 MiB cap;
  byte counts, truncation, and SHA-256 evidence preserve audit truth.
- Narrow changed validation selected only `diff-check` for a documentation
  path. Parser-focused validation excluded Arena, and profile lists are tested
  against exact Make prerequisites.
- Nabu found no architecture blocker under the preserved Make/classifier
  boundary. Vulcan found no High or Medium issue after reviewing grammar,
  confinement, drift, release/cache truth, evidence bounds, schema strictness,
  report wiring, conformance, recursion risk, and tests.
- Tier 2 `make core` passed with 89 schemas, 178/178 fixtures, 255/255 declared
  integration checks, all 34 release targets, full transaction, adapter,
  worker, client, user-test, and validation lifecycles, AI foundation, corpus
  governance, smoke 5/5, and source purity.
- The first W5 boundary `make v1` run exposed a stale assertion expecting `8`
  from `examples/unused_import_project/src/app/used.sley`; the unchanged source
  and runtime correctly return `7`. Commit `b24e97d` restored the oracle and
  the complete self-hosted suite passed.
- The next boundary run reached `examples` and exposed a W1 recovery-table gap:
  the established `User { name: "Ada", age: 37 }.name` example was still
  intentionally supported by lint, fix, runtime, docs, and fixtures but was
  omitted from the later closed `Raw` exemption table. Commit `93d0ed3` added
  one anchored typed-record-field-access recovery shape and a focused checker
  regression. Generic unsupported `Raw`, nested host-call argument `Raw`,
  generic-record, and nested typed-access probes remain rejected. Vulcan found
  no report-grade issue and returned the repair commit-ready.
- The authoritative Tier 3 `make v1` gate then passed on 2026-08-25 at
  `93d0ed3`: all 34 release targets, 89 schemas, 178/178 fixtures, 255/255
  declared integration checks, corpus 96/96, examples 146/146, smoke 5/5,
  full transaction/adapter/worker/client/user-test/validation lifecycles, AI
  foundation, corpus governance, source purity, and changed-path planning.
- S12-505 and W5 are closed. W6 operational evidence may now advance without
  granting production, provider, deploy, or public-release authority.

## W6 S12-601 bounded reference replay

- The retained Siglum reference package initially failed the W1 executable
  `Raw` gate because five multiline typed record constructors were parsed only
  as their opening lines. Commit `b97fba8` added structural multiline
  `RecordLiteral` parsing for those declared types while keeping generic
  unsupported `Raw` rejection closed.
- Commit `6f68bd8` added exact Siglum/artifact pins, 20-shard corpus validation,
  retained-report checks, a persistent machine-host cold/warm probe, two fresh
  independent TypeScript oracle passes, bounded controller execution, strict
  report contracts, tamper regressions, validation routing, and ADR-013.
- Full attempts under 4 GiB and 8 GiB per-process virtual-address ceilings
  failed before corpus execution because Node/V8 could not reserve WebAssembly
  address space. A bounded import-path sweep failed below 32 GiB and passed at
  32 GiB. Commits `28a6dfb` and `73bc5b0` preserve that evidence and distinguish
  virtual-address ceiling from sampled resident memory.
- The original replay at Sley `73bc5b0` and Siglum `2721acb` passed. The
  public-history refresh at sanitized release parent `bfeba32` also passed.
  Each fresh run matched all 10,000 frozen cases with zero mismatches, and all
  stable source, runtime, corpus, mismatch-report, and 20 shard digests agreed
  across both runs.
- The current public-history replay observed 5,325 ms cold, 76/76 ms warm,
  234,887 and 239,517 ms per full run, 42.57 and 41.75 cases per second,
  127,926,272 bytes peak probe-tree RSS, and 1,684,430,848 bytes peak
  full-controller-tree RSS.
- The evidence packet is
  `reports/operational/siglum-numerology-reference-v1.json`. It contains no raw
  corpus, credentials, or real user data and has no publication authority.
- Production shadow, production latency, fallback, rollback, and promotion
  authority remain missing. S12-601 therefore defers production promotion and
  makes no production-adoption claim. S12-602 and S12-603 remain active W6
  work.
- Focused replay, tamper, contract, format, and diff checks passed. The broader
  `make check-changed` Tier 2 gate also passed with 91 schemas, 179/179
  fixtures, 256/256 declared integration checks, corpus 96/96, and 35 v1
  targets. The full `make v1` release gate was not repeated at this W6 slice.
- Independent Vulcan review initially found one Medium CLI coverage defect:
  the public wrapper did not advertise or test its required Siglum root.
  The corrected wrapper requires `--siglum-root PATH`, rejects caller attempts
  to replace the Sley-owned repository root or pin set, and is exercised
  directly by the replay regression. Vulcan's bounded recheck returned PASS
  with no remaining High or Medium findings.

## W6 S12-602 controlled workflow infrastructure

- The existing private one-shot runner records a candidate digest but does not
  compare it with the manifest's trusted candidate. Compilation and changed-
  line limits alone were therefore rejected as insufficient acceptance proof.
- The runner also does not generically enforce every case-manifest resource
  limit. Only equal task, model-context, and spend budgets are claimed. Tool
  and inference turns remain the controlled interface difference, and both
  plan and comparison retain the partial-enforcement limitation.
- The public fixture now uses four independent oracle steps: candidate check,
  exact pipeline task inventory, exact main call inventory, and locked
  effect-mocked behavior. The task and call inventories prove the new symbol
  exists with its original signature/effects and the old symbol is absent.
- K1 and K3 use the same model, thinking level, prompt, workspace, oracle,
  owned paths, Bootstrap, empty retrieval, model context window, non-interface
  resource limits, and per-arm spend ceiling. K3's one structural query and
  possible second inference turn are the explicit action-budget and interface
  difference.
- The no-call plan validates the public manifests and private controller
  digest while reporting provider authorization false and approval required.
- The scrubbed assembler requires an exact private approval record whose
  commit, case, run, controller, model, spend, and denied-action fields match
  the no-call plan. It also requires both exact aggregates/evidence roots and
  a second-turn structural transcript whenever K3 reports one tool call. It
  measures strict success, ACT, token and byte consumption, structural
  response size, explicit context effect, wall time, tool/compiler use,
  invalid actions, minimality, and comparison-host environment without
  embedding private content.
- Deterministic accepted-pair and missing-structural-tool regressions pass. The
  missing-tool attempt is unsuccessful and remains in the denominator.
- Tier 2 `make core` passed across the compiler, contracts, transactions,
  adapters, worker/clients, user tests, validation profiles, both operational
  gates, conformance, corpus governance, smoke, and self-hosted boundary. After
  narrowing the verified-equality claim, focused workflow, syntax, all 181
  contract fixtures, and conformance checks also passed. Conformance reports
  93 schemas, 258/258 declared integration checks, and all 36 release targets.
- Independent Vulcan review found one Medium evidence-binding defect: the
  comparator could trust aggregate success booleans without binding the result
  tree and recorded oracle steps to the controlled candidate. The corrected
  assembler requires exact initial/candidate/final tree, prompt, training,
  bootstrap, preflight/oracle step, step-digest, compiler-cycle, singleton
  count/resource, adapter, family, and status evidence. Candidate-digest and
  oracle-step tampering now fail. Vulcan's blocking recheck returned PASS with
  no remaining High or Medium findings.
- A later approval-free execution-readiness replay ran the exact public K1/K3
  manifests through the pinned private controller with local fake-provider
  envelopes and the real Sley oracle. Both arms passed with zero provider
  spend, but the comparison would have rejected them because it compared the
  runner's complete final-workspace digest with a changed-files-only fixture
  digest. The corrected plan and assembler materialize the expected final tree
  as baseline workspace plus the exact owned-file candidate overlay. A focused
  regression independently reconstructs that tree and rejects a bare-overlay
  identity. The case and both run manifests are repinned to the corrected
  identity. Vulcan independently reran the focused workflow test, confirmed
  the final-workspace digest, rejected an extra candidate file and candidate
  symlink, and returned PASS with no High or Medium finding. Tier 2 `make core`
  then passed with 94 schemas, 182/182 contract fixtures, 259/259 declared
  integration checks, smoke 5/5, and all 37 v1 targets.
- The operator approved the exact two-arm execution. One provider-backed K1
  attempt and one K3 attempt completed with no retry. Both produced the exact
  accepted two-file/four-line change and passed every oracle step. K1 used
  3,404 tokens and 13,223 ms; K3 used 7,815 tokens, one read-only structural
  query, and 36,810 ms. The retained comparison classifies the added structural
  context as expansion and defers promotion. One trial cannot support a
  general model, structural-advantage, or production claim.
- Vulcan inspected the raw evidence and provider-origin path, confirmed one K1
  and one K3 attempt, and returned PASS with 0 High and 0 Medium findings.
  Provider origin remains assertion-based and is not cryptographically
  verified.

## W6 S12-603 operational decision infrastructure

- `sley.operational.evidence_registry.v1` defines exactly one independent
  reference entry and one controlled agent entry with artifact identities,
  bounded results, visible limitations, reproducibility commands, disclosure,
  and unconditional production/general-claim deferral.
- The read-only assembler revalidates both source schemas. It requires exact
  reference parity and independent-oracle truth, exact case/run/controller
  pins for the agent comparison, Git ancestry, one retained trial, K1/K3
  identities, scrubbed disclosure, and mandatory single-trial and partial-
  limit negative evidence.
- The public comparison fixture is explicitly rejected. Digest tampering,
  non-ancestor evidence, weakened reference deferral, symlinked controllers,
  altered private source evidence, and caller-controlled repository roots also
  fail closed. The retained comparison must reassemble exactly from its private
  approval, aggregate, and evidence inputs.
- Admission additionally requires a private record binding those inputs to an
  asserted observed provider execution and a passing independent evidence
  review. Because this path has no cryptographic provider receipt, the
  assertion limitation remains explicit and blocks broader claims.
- The registry binds `HEAD` plus exact digests of both assemblers, the complete
  schema set, the registry schema, and `loom.operational`; uncommitted
  controlling bytes therefore remain visible in the artifact identity.
- Tier 2 `make core` passed across focused compiler checks, 182/182 contract
  fixtures, transaction and adapter boundaries, worker/clients, user tests,
  validation profiles, both operational gates, conformance, corpus governance,
  smoke 5/5, and the self-hosted boundary. Conformance reports 94 schemas,
  259/259 declared integration checks, and all 37 v1 targets.
- Independent Vulcan review initially found two Medium blockers: caller-built
  aggregates could assert provider origin without a reviewed execution record,
  and a clean `HEAD` identity did not expose dirty controlling source/schema
  bytes. The comparison now requires an exact private execution-provenance
  record with observed-attempt and independent-review assertions while
  retaining the explicit non-cryptographic limitation. The registry now binds
  exact assembler, schema-set, registry-schema, and vocabulary digests.
  Vulcan's blocking recheck independently reran both focused operational tests
  and returned PASS with no remaining High or Medium findings.
- The final registry is `reports/operational/evidence-registry-v1.json`. It
  binds the retained Siglum reference replay and the real scrubbed S12-602
  comparison, records `w6_evidence_complete=true`, unblocks release closeout,
  and keeps production promotion deferred.
- Focused workflow, evidence, contract, and conformance gates passed with 94
  schemas, 182/182 fixtures, 259/259 declared integration checks, and all 37
  v1 targets. Vulcan independently reassembled the comparison from the exact
  nested evidence roots and returned PASS with 0 High and 0 Medium findings.
  W6 is complete. The registry grants no provider, runtime, deployment,
  publication, or promotion authority.

## W7 release closeout implementation

- `loom.release` owns the 1.2.1 artifact identity, supported Linux x86_64
  target, required toolchain, archive entrypoints, metadata contracts, and
  non-public authority state.
- The deterministic archive builder rejects dirty production inputs,
  symlinks, unsafe output replacement, missing entrypoints, and caller-chosen
  repository roots. It embeds a version manifest, license inventory, and SPDX
  2.3 SBOM, then emits an external checksum and unsigned provenance.
- Verification checks safe archive paths and types before extraction, binds
  archive and metadata digests, replays the payload inventory, scans for host
  paths and credential patterns, and exercises the unpacked CLI, toolchain
  doctor, checker, contracts, and worker clients.
- Five strict release/toolchain v1 contracts and five fixtures extend the
  inventory to 99 schemas and 187 fixtures. The release gate now contains 38
  targets and the generated self-hosted count is 264.
- Canonical release, migration, threat-model, changelog, claim, checklist, and
  post-GA docs state one supported archive and retain the exact no-publication
  boundary. Signatures, other platforms, package managers, public hosting,
  deployment, and announcements remain deferred and unauthorized.
