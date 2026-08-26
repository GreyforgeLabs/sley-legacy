# Sley 1.2 architecture

Status: active implementation architecture
Owner: Sley maintainers
Review trigger: work-package boundary or semantic ownership change

## One semantic core

All 1.2 interfaces use one Sley-owned semantic path:

```text
source
  -> parser and loss-preserving AST
  -> checker and actionable diagnostics
  -> bounded graph/query/explain surfaces
  -> plan and immutable candidate
  -> exact local grant
  -> atomic apply
  -> verify/rollback
  -> review packet and seal
```

CLI, MCP, LSP, worker clients, and future HTTP surfaces may transport these
contracts. They may not define parallel parsing, authority, transaction, or
verification semantics.

## Ownership rules

| Surface | Semantic owner | Host responsibility |
|---|---|---|
| Expression and declaration classification | `loom.parser` | execute Sley-owned classifier plans |
| Diagnostic IDs, messages, pass order, patterns, and repair kinds | `loom.checker` | generic descriptor execution and JSON assembly |
| Runtime failure identities and limits | `loom.runtime` | bounded process/adapter execution |
| Report roots and field ownership | `loom.reports` plus registered schemas | serialization and schema validation |
| Transaction lifecycle and mutation boundary vocabulary | `loom.transaction` | bounded repository inspection and generic report assembly |
| Local grant scope, replay, revocation, proof, and budget vocabulary | `loom.authority` | exact request regeneration and explicit assertion assembly |
| Local adapter identity, seed, replay, lifecycle, and failure vocabulary | `loom.adapter` | bounded seeded verification and redacted record assembly |
| Bootstrap inventory and self-hosting proof | `loom.bootstrap` | expose status and fail on missing ownership markers |
| Benchmark task/oracle truth | frozen SleyBench manifests and independent oracles | isolated execution only |
| Product authority | incumbent product contract until promotion | no implicit cutover |

## W1 diagnostic architecture

W1 adds diagnostic truth without changing language syntax or `Raw` storage:

1. `loom.checker` owns new pass descriptors, diagnostic IDs/messages, source
   patterns, and repair-hint kinds.
2. The host checker gains only generic executor support required to evaluate
   those descriptors over the AST/source.
3. `Raw` traversal must be recursive so nested conditional branches, call
   arguments, collection members, and future expression nodes cannot escape.
4. Known foreign Boolean/conditional forms receive specific human- and
   agent-readable alternatives. Unknown `Raw` receives a generic error stating
   that runnable status is unproven.
5. Module-level control flow and inline task parameters are source-structure
   diagnostics because the current AST loses or misclassifies those forms.
6. Runtime behavior stays unchanged and fail-closed.
7. Closed recovery exemptions cover only Sley-owned known parser artifacts
   for supported unit, multiline-map, record-literal, and declared host-adapter
   forms. They are matched in the same module and source line where applicable;
   unknown nested `Raw` remains an error.

## Dependency boundaries

- W1 diagnostics precede W2 explain/spec lookup.
- W2 precedes stable diagnostic contract promotion.
- Stable minimum contracts precede the transaction state machine.
- The local transaction precedes governed MCP writes.
- The worker and user-test surface precede operational replay.
- Production Siglum authority is outside Sley 1.2 implementation authority.

## W6 operational reference replay architecture

S12-601 composes existing W5 runtime boundaries rather than creating a second
execution or oracle path:

1. `sley reference-replay siglum-numerology --siglum-root PATH` resolves a
   Sley-owned pin set that the caller cannot override and accepts only the exact
   Siglum commit, candidate package, independent oracle, corpus manifest, 20
   shard files, ruleset, parity harness, and retained parity report.
2. A persistent `sley machine` process measures one cold and two warm requests
   through `sley.machine.invoke.v0`. Source, runtime, and result digests must be
   stable across all three responses.
3. The existing Siglum differential harness remains the full-run controller.
   It invokes fresh Sley machine processes for each shard and compares every
   result with `ReferenceTypeScriptSignatureEngine.compute` through the neutral
   numerology input/result contracts.
4. Two complete 10,000-case passes are mandatory. A missing case, mismatch,
   identity drift, nondeterministic shard, timeout, resource failure, or stale
   pin fails the replay.
5. Per-process virtual address space, CPU, file descriptors, shard wall time,
   evaluation steps, call depth, collection length, response bytes, and output
   bytes are bounded. Process-tree RSS is sampled separately so virtual-address
   reservations are not misreported as resident use.
6. The child environment is allowlisted and credential-bearing provider state
   is absent. OS network namespace isolation is not claimed.
7. `sley.operational.reference_replay.v1` records exact identities,
   correctness, observed local performance, negative evidence, disclosure,
   and promotion state. It grants no product, network, provider, repository,
   deployment, spend, publication, or production authority.
8. The immutable evidence packet is
   `reports/operational/siglum-numerology-reference-v1.json`. Production Siglum
   remains owned by the incumbent TypeScript implementation unless its separate
   promotion gates later pass.

## W6 S12-602 controlled agent workflow architecture

S12-602 reuses the frozen SleyBench case, run, aggregate, and case-result
contracts. It does not create a second model protocol or modify the private
one-shot controller.

1. `loom.operational` owns the workflow identity, equal-control vocabulary,
   required metrics, oracle-step names, controlled interface difference, and
   outcome states.
2. `sley operational-workflow plan` pins one public two-file rename, the raw
   workspace, trusted candidate, Bootstrap 0.2, K1/K3 run manifests, private
   controller digest, empty retrieval set, equal limits, and exact oracle. It
   performs no provider call and requires later approval.
3. K1 is the incumbent raw-workspace interface. K3 receives the same raw
   workspace plus one allowlisted read-only task query. Both use the same
   model, task, context window, spend ceiling, owned paths, and oracle. The
   extra structural turn is the controlled difference.
   Case-manifest resource limits are retained inputs, but the unchanged private
   controller does not generically enforce every declared limit. The plan and
   comparison state this partial-enforcement boundary instead of calling those
   limits verified equal controls.
4. Acceptance requires compile success, exact pipeline task identity and
   signature/effects, exact main call-target identity, locked effect-mocked
   behavior, and a two-file/four-line minimality boundary.
5. The scrubbed assembler verifies aggregate and run identities, measures
   prompt/response and structural bytes from private evidence, requires an
   exact private approval record pinned to the plan, computes per-arm ACT only
   for accepted changes, and retains failed attempts and explicit context
   saving, equality, or expansion.
6. Raw prompts, responses, tool reports, and approval text remain outside the
   repository. The comparison grants no repository, product, deployment,
   publication, or production authority and cannot support a broad claim from
   one trial.
7. Comparison-host hardware is labeled as such. The packet does not imply that
   post-hoc assembly metadata proves the provider execution host.

## W6 S12-603 operational decision architecture

The final W6 registry is a derived, read-only evidence surface rather than a
second benchmark or authority system:

1. `loom.operational` owns the registry identity, exact two workload IDs,
   release, authority mode, and fixed deferral rationale.
2. `sley operational-evidence assemble` accepts only repository-confined,
   schema-valid S12-601 and S12-602 artifacts plus the private controller,
   approval, aggregates, and evidence bundles needed to reassemble and exactly
   match the scrubbed comparison. It writes JSON only to standard output.
3. The reference entry must retain two deterministic 10,000-case exact passes,
   an independent oracle, ancestor commit, non-authoritative state, and the
   full production-evidence deferral.
4. The agent entry must be non-synthetic, bind the current public case and run
   manifests, use an ancestor Sley commit, retain exactly one trial and every
   failure, preserve the K1/K3 identities, bind an exact assertion-based
   execution provenance and independent evidence review, and expose no private
   evidence.
5. The registry keeps both entries, their artifact digests, bounded results,
   limitations, reproducibility commands, and disclosure state visible.
6. Every possible agent outcome remains non-general and production-deferred.
   Registry assembly grants no provider, repository, product, deploy,
   publication, or promotion authority.
7. Registry identity binds the exact registry/comparison assemblers, registry
   schema, complete schema set, and operational vocabulary bytes in addition
   to the current Git commit, so dirty controlling sources cannot hide behind
   `HEAD`.

## W5 local adapter architecture

The first W5 slice activates deterministic adapter replay without claiming an
isolated or persistent worker:

1. `loom.adapter` owns the manifest/replay/report identities, lifecycle,
   ordered event phases, seed families, supported local effects, authority
   mode, replay policy, and typed failures.
2. `sley adapter replay` validates canonical manifest, adapter, runtime,
   target, capability, seed, and record identities before accepting replay.
3. The host delegates execution to the existing seeded `sley verify` path. It
   does not dynamically import adapter code or define parallel runtime
   semantics.
4. Capabilities bind declared effects and operations to exact deterministic
   seed scopes. Live execution and repository, index, trace, external,
   provider, deployment, or spend mutation are denied.
5. Replay records exclude seed values and runtime values. They expose digests,
   counts, redaction boundaries, limits, and ordered lifecycle evidence.
6. Input/output bytes, wall time, static steps, static call depth/cycles, and
   observed structural calls are bounded. Deterministic boundary cancellation
   is observable, while preemptive request cancellation remains S12-502 work.
7. Reports explicitly state that OS isolation and a persistent worker do not
   exist yet. `sley-sandbox-runner` remains unchanged.

## W5 persistent worker architecture

The second W5 slice adds a long-lived bounded controller without duplicating
the runtime:

1. `loom.worker` owns `sley.worker.v1`, four stable request/response/event/
   session roots, the operation set, lifecycle, request states, failure
   classes, contract bindings, isolation class, and disabled-cache policy.
2. Every message binds unique replay identity, worker/runtime/package/source
   digests or explicit absence, exact contract versions, entry point,
   authority, and six resource budgets before dispatch.
3. The controller accepts one invocation at a time and starts a fresh
   `sley adapter replay` child for each request. It never defines alternate
   parser, checker, runtime, adapter, or authority semantics.
4. Each child receives a private temporary home, clean environment, closed
   inherited descriptors, its own process group, and Linux `prlimit` memory,
   CPU, and descriptor ceilings. Progress is bounded and terminal output is
   schema-validated and collection/output limited.
5. Cancel terminates the active process group. Child crashes are contained and
   typed without killing the controller. Drain, reset, health, lifecycle
   limits, and graceful shutdown are explicit protocol operations.
6. No loaded source, seed values, trace values, adapter handles, mutable
   program state, cancellation state, or cache crosses an invocation boundary.
7. Isolation evidence distinguishes enforced fresh-process isolation from
   unavailable namespace containment. The worker reports
   `namespace_isolation_enforced: false` and makes no filesystem or network
   sandbox claim.
8. Python and Node reference clients derive typed models from the four stable
   schemas and add only bounded ordered stdio transport, exact request
   construction, and lifecycle ergonomics. Worker responses remain the sole
   authority for result, failure, retryability, and isolation semantics.
9. Performance reports, external adapters, parallel invocation, and automatic
   retry remain later work.

## W5 first-class test architecture

The fourth W5 slice adds one syntax-neutral test control plane over existing
checked surfaces:

1. `loom.testing` owns the three stable contract identities, discovery rule,
   six case kinds, coverage dimensions and evidence states, property strategy,
   differential comparison, and authority modes.
2. `sley test` discovers strict manifests in byte order and requires identical
   suite source, package, authority, bounds, and exact coverage inventory.
3. Table and property cases use `sley machine`; effect mocks use seeded
   `sley run`; adapter cases use `sley adapter replay`; diagnostic cases use
   `sley check`; differential cases compare two pure machine endpoints.
4. Projected execution and property counts are checked before execution. Wall
   time and final report bytes remain bounded.
5. Coverage distinguishes observed execution authority from declared passing
   witnesses. Branch credit is declared witness coverage, not instrumentation.
6. Changed task nodes expand through the reverse checked caller graph. An
   uncovered requested node fails with typed evidence.
7. `sley.review.packet.v1` binds a passing test report to an existing terminal
   review, seal, transaction, and final source digest without mutating or
   reinterpreting the strict v0 transaction contracts.
8. The entire surface is local and read-only with no provider, external,
   repository, index, deploy, or spend authority.

## W5 validation-report architecture

The fifth W5 slice adds a machine-readable layer over existing validation
authority:

1. `loom.validation` owns the four profile names, check-to-subsystem registry,
   profile memberships, skip reasons, authority mode, cache policy, and stable
   report identity.
2. `quick`, `core`, and `release` execute the existing aggregate Make targets.
   The profile registry is checked against Make prerequisites to prevent drift.
3. `changed` consumes `scripts/check-changed.sh --plan-json` and then invokes
   the same dispatcher. No second path classifier exists in the CLI.
4. `sley.validation.report.v1` records changes, selected and skipped work,
   release-gate truth, separate validation/source-cache truth, wall time,
   outcome, and digested bounded executor evidence.
5. Only `release` executes authoritative `make v1`. Faster profiles cannot
   claim release readiness.
6. Validation remains repository-local and grants no repository mutation,
   provider, network, deploy, or spend authority.

## W2 discoverability architecture

W2 adds one additive, bounded report without promoting the existing diagnostic
root ahead of W3:

1. `loom.checker` owns the four-ID diagnostic explanation catalog, including
   checked spelling, context, repair kind/path, and specification and example
   references.
2. `loom.reports` registers `sley.explain.report.v0` and its field builder.
3. `sley explain` emits JSON or human text from that same catalog; unknown IDs
   fail closed.
4. The report version context is Sley-owned and pins the implementation,
   Sley 1.2 target, and retained Bootstrap 0.2 path and digest without mutating
   `SLEY_AI.md`.
5. LSP exposes a command preview to the same CLI lookup. Formatter coverage is
   a round-trip assertion over the accepted W2 forms. The recovery AST schema
   remains unchanged because W2 adds no grammar or AST node.
6. Stable v1 diagnostic-contract promotion is completed by the W3 contract
   floor below.

## W3 protocol and contract architecture

W3 establishes a strict evidence floor without introducing transaction or
worker semantics:

1. `sley.agent_bench.protocol_replay.v0` admits one action per turn and locks
   normalized owned paths, sequential turns, tool/input/output/context/wall
   budgets, and observed cancellation outcomes.
2. Concatenated response objects fail the single-document boundary before
   schema validation; multi-action objects fail strict JSON Schema validation.
3. Four report roots are registered as stable v1 compatibility targets:
   diagnostics, bounded symbol-graph slices, verification, and machine
   responses. Existing v0 producers remain unchanged.
4. `greyforge.sleybench.mode_summary.v0` registers the exact retained producer
   shape. Its public fixture is synthetic; private cases and result values do
   not enter the repository.
5. `CONTRACT_STABILITY.json` resolves every inventory root to stable,
   extensible, or default experimental. W4 transaction and W5 worker/provider
   namespaces are reserved rather than fabricated.
6. The compatibility gate proves v0/v1 shape parity in both directions and
   rejects discriminator substitution, missing required fields, and unknown
   fields. Protocol replay records observation; it is not preemptive runtime
   cancellation authority.

## W4 transaction architecture

The first W4 slice activates transaction semantics without write authority:

1. `loom.transaction` owns the complete success and failure state vocabulary,
   the initial transition plan, the explicit `working_tree` base mode, and the
   five mutation-boundary names.
2. `sley change inspect` opens and inspects one transaction. It binds repository
   identity, HEAD, structural source, graph, trace, compiler/contracts, actor,
   goal, nonce, creation, and expiry into digest-addressed evidence.
3. Inspection composes existing checked query, check, lint, plan, and graph
   primitives. It does not create a parallel parser, graph, effect, or
   validation model.
4. `sley.transaction.inspect.v0` is strict and extensible. It reports an exact
   `opened -> inspected` history and declares every mutation boundary false.
5. The host CLI rejects ambiguous authority and repository boundaries with
   typed diagnostics. Source and Git-index digest tests prove the command is
   non-mutating.
6. `sley change plan` separates operator outcome, assumptions, and non-goals
   from compiler-derived nodes, files, operations, expected changes, and
   validation selection. The plan digest binds both fact classes to the
   inspected base.
7. `sley change preview` executes only Sley-owned operation mappings in a
   disposable copy outside the repository. Candidate identity excludes the
   temporary path and includes source, graph, operation, plan, transaction,
   and validation evidence.
8. Candidate evidence includes the complete source projection, stable source
   and semantic diffs, diagnostics, focused compiler tests, and blocking review
   requirements. Repository source, index, trace, external, and provider
   mutation flags remain false.
9. `sley change approval-request` derives exact operation, node, path, effect,
   authority, resource, review, and budget scope from one valid preview. It
   binds repository identity and HEAD without claiming mutable working-tree
   freshness.
10. `sley change approve` regenerates and exactly compares that request before
    accepting explicit issuer, principal, audience, purpose, nonce, time,
    review, revocation, and wall-clock assertions. Request text and self-digest
    alone are never authority.
11. `sley.change.grant.v0` records explicit CLI issuer assertion and optional
    unverified proof references honestly. Replay is single-use at apply.
12. `sley change apply-authorization` preserves grant issuance semantics and
    derives fixed transaction-state, lock, replay, recovery, and revocation
    scopes. Source mutation and internal state mutation are reported separately.
13. `sley change apply` accepts only one Linux same-filesystem non-root common
    source root. A durable candidate is fsynced before one
    `renameat2(RENAME_EXCHANGE)` commit point. Stale, replayed, revoked,
    symlinked, special-file, submodule, xattr, staged-index, and unsupported
    topology cases fail closed.
14. Candidate source, graph, compiler check, and compiler lint are required
    after commit. Failure automatically exchanges the preserved preimage back
    and verifies the exact base. `sley change rollback` applies the same
    verified exchange to an unchanged verified result.
15. `sley change recover` resumes non-terminal durable state by classifying the
    exact live and preserved base/candidate digests. It validates replay and
    authority before any resumed forward exchange, then finishes required
    verification or verified rollback. Journal phase alone never authorizes a
    filesystem exchange.
16. `sley change review` accepts only exact terminal verified-apply or
    verified-rollback placement. It pins optional trace receipts, summarizes
    authority and validation without copying source into Markdown, and
    atomically publishes one no-replace review directory containing the human
    packet, machine bundle, ZJX preview evidence, and transaction seal.
17. The transaction seal binds final structural source and graph digests to
    apply, optional rollback, review, trace, packet, bundle, and ZJX digests.
    Review holds the cooperative transaction lock, and manual rollback refuses
    an already sealed transaction.
18. `sley-mcp-bridge` exposes nine named transaction adapters that construct
    argv arrays for the matching CLI commands. Repository selection is fixed to
    the configured root, transaction artifacts must be confined regular
    non-symlink files, typed CLI diagnostics remain structured, and mutation
    confirmations and annotations stay explicit. Grant issuance and
    revocation-record creation remain CLI-only operator authority actions.

## Compatibility

New diagnostics intentionally turn previously false-positive `ok` checks into
errors for unsupported executable source. This is a correctness fix, but it is
a behavior change and requires:

- stable diagnostic IDs and fixtures;
- accepted corpus non-regression;
- rejected corpus additions;
- docs/bootstrap/LSP review;
- a migration note identifying the former false-positive behavior.
