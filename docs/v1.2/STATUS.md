# Sley 1.2 status

Status: W7 public release cut approved for `v1.2.0`
Owner: Sley maintainers
Updated: 2026-08-26

## Baseline

- Branch: `codex/sley-ai-foundation-20260822`
- HEAD at implementation start: `cb9d714`
- Upstream: `origin/public`
- Upstream relation: 18 commits ahead at implementation start
- Working tree at implementation start: clean
- Public reference baseline: `c973956`
- Sley version: `1.2.0`
- Reconciled release: Sley 1.2 Governed Autonomy
- Current wave: W7 release publication active under exact operator approval.
  W6 is complete with the bounded
  Siglum replay, one controlled K1/K3 agent trial, a reviewed scrubbed
  comparison, and a non-authoritative evidence registry that defers promotion
- Last Tier 1 baseline: `make quick`, passed in 48.60 seconds on 2026-08-24
- Last Tier 2 validation: S12-602 final-workspace identity repair `make core`
  passed on 2026-08-26 with 94 schemas, 182/182 fixtures, 259/259 declared
  integration checks, corpus governance, smoke 5/5, and all 37 declared v1
  targets
- Current W7 conformance inventory: 99 schemas, 187/187 fixtures, 264/264
  declared integration checks, and all 38 release targets present
- Last full release gate: `make v1` passed on 2026-08-25 at `93d0ed3` with all
  34 release targets, 89 schemas, 178/178 fixtures, 255/255 declared
  integration checks, corpus 96/96, examples 146/146, and smoke 5/5
- Authoritative AI bootstrap: Bootstrap 0.2,
  `sha256:e0c388056373d011d80d2ecddf659390f1ebde843bd8cd9e31afef612e9fe061`
- Historical W6 trials retain their original Bootstrap 0.2 digest in the
  immutable comparison and decision evidence; the current digest changes only
  the release-candidate version header and remains excluded from retroactive
  operational claims

## Completed work packages

| ID | Evidence | Validation | Notes |
|---|---|---|---|
| S12-000A | Current branch, history, remotes, worktrees, docs, contracts, commands, and validation map inspected | `make quick` passed | No local work discarded |
| S12-000B | SleyBench baseline/controller/protocol/semantic/Bootstrap 0.3 audits reconciled | Evidence paths verified | No benchmark or corpus mutation |
| S12-000C | `docs/v1.2` planning baseline | `make diff-check fmt syntax` passed | Commits `69db279`, `12cb33e` |
| S12-101 | Recursive executable-`Raw` diagnostics with closed supported-recovery policy | focused probes; corpus 94/94; `make core` passed | Runtime and AST recovery semantics unchanged |
| S12-102 | Foreign conditional and module control-flow context diagnostics | single-file and same-module/same-line project regressions passed | Vulcan blocker fixed and re-reviewed |
| S12-103 | Inline task-parameter placement diagnostic | focused rejected fixture passed | Valid body-level `take` unchanged |
| S12-104 | Multiline conditional decision | ADR-001 review; syntax/core gates passed | No grammar expansion |
| S12-201 | Four-ID Sley-owned diagnostic explanation catalog and human/JSON `sley explain` | exact CLI probes; comprehensive self-hosted suite passed | Unknown IDs fail closed |
| S12-202 | Versioned explain schema, inventory, contract fixtures, spec and accepted/rejected example alignment | 149/149 contract fixtures; corpus 96/96 | Existing diagnostics and recovery AST v0 roots unchanged |
| S12-203 | Formatter and LSP diagnostic alignment | accepted-form formatter round trip; LSP command-preview parity | No editor-local truth table |
| S12-301 | Strict owned-path, one-action, sequential-turn, budget, and cancellation-observation replay contract | deterministic historical and negative replays; Tier 2 passed | Frozen cases/oracles and provider adapter unchanged |
| S12-302 | Four stable v1 compatibility targets plus machine-readable stability registry | bidirectional v0/v1 and strict negative gate passed | Existing v0 producers did not cut over |
| S12-303 | Exact retained mode-summary producer shape with synthetic public fixture | retained artifact and cross-field arithmetic validated | No private values or cases committed |
| S12-304 | Inventory, compatibility fixtures, validation routing, and v1 gate registration | 66 schemas; 155/155 fixtures; 28-target conformance inventory | Strict parsing retained |

## Completed W4 packages

- `S12-401`: Loom-owned transaction lifecycle, strict read-only inspection,
  reproducible Git/base binding, and no-mutation evidence.
- `S12-402`: strict `sley.change.plan.v0` and
  `sley.change.preview.v0`, Sley-owned ordered candidate executors,
  disposable candidate materialization, stable source/semantic evidence,
  focused compiler validation, and blocking review requirements.
- `S12-403`: strict `sley.change.approval_request.v0` and
  `sley.change.grant.v0`, compiler-regenerated non-broadenable scope, explicit
  issuer/principal/audience/review assertions, typed resource and budget
  bounds, honest unverified proof labels, and deferred replay enforcement.
- `S12-404`: strict apply authorization and one-way revocation, single-use
  replay, Linux same-filesystem common-root exchange, pre-commit cancellation,
  digest-classified crash recovery, required verification, and verified
  automatic/manual rollback.
- `S12-405`: exact historical-chain validation, terminal apply/rollback review,
  pinned trace receipts, human and machine review packets, content-addressed
  transaction seals, atomic no-replace publication, and post-seal rollback
  denial under the shared transaction lock.
- `S12-406`: nine named MCP transaction adapters, fixed repository selection,
  confined regular artifacts, exact CLI success/diagnostic forwarding,
  explicit mutation confirmations and annotations, CLI-only authority issuance,
  and end-to-end inspect-through-sealed-review parity.

## Completed W5 packages

- `S12-501`: provider-neutral local adapter manifest, repository-confined
  deterministic replay, exact seeded authority, redacted records, typed
  denials, replay pins, bounded cancellation and budget enforcement, and
  source/index no-mutation evidence.
- `S12-502`: stable v1 worker protocol, serial persistent controller, fresh
  bounded process and immutable input snapshot per invoke, preemptive
  cancellation, crash containment, idempotency replay denial, reset/drain/
  health/shutdown lifecycle, disabled caches, and truthful namespace limits.
- `S12-503`: schema-generated typed Python and TypeScript models,
  dependency-free Python and Node stdio clients, exact lifecycle request
  builders, ordered bounded framing, authoritative cancellation and failure
  responses, no automatic retry, and clean offline package tests.
- `S12-504`: first-class manifest-backed `sley test`, deterministic discovery,
  six bounded case kinds, graph-derived changed-node focus, truthful coverage,
  stable reports, and exact read-only terminal review evidence binding.
- `S12-505`: repository-local quick, changed, core, and release validation
  profiles, one Sley-owned check registry, machine-readable changed planning,
  stable reports with complete skip/release/cache truth, bounded digested
  executor evidence, and exclusive `make v1` release authority.

## Completed W6 packages

- `S12-601`: exact Siglum and artifact pins, frozen 20-shard corpus checks,
  retained and fresh independent-oracle validation, persistent cold/warm probe,
  two fresh 10,000-case full runs, bounded controller and worker resources,
  sampled resident-memory evidence, preserved failures, strict replay report,
  and explicit non-authoritative promotion deferral.
- `S12-602`: one approved provider-backed K1/K3 controlled maintenance trial,
  exact private evidence and reviewed assertion-based provenance, strict
  structural and behavior oracles, a scrubbed comparison, visible context
  expansion, and no general or production claim.
- `S12-603`: exact two-entry operational evidence registry binding the Siglum
  replay and controlled agent comparison, fail-closed reassembly, retained
  limitations, independent final review, and explicit promotion deferral.

## Active work packages

- IDs: `S12-701`, `S12-702`, `S12-703`, `S12-704`, `S12-705`
- Goal: produce one supported self-contained release artifact and reconcile
  security, provenance, checksums, SBOM, license inventory, version manifest,
  canonical docs, migration notes, claims, and post-GA planning.
- Completed dependency: W6 decision record at
  `reports/operational/evidence-registry-v1.json`.
- W6 focused validation: operational workflow and evidence gates, all 182
  contract fixtures, and conformance passed with 94 schemas, 259/259 declared
  integration checks, and all 37 v1 targets.
- W6 independent closeout: Vulcan reassembled the comparison from the exact
  nested evidence roots and returned PASS with 0 High and 0 Medium findings.
- Protected surfaces: W1-W6 semantics, contracts, private evidence, frozen
  benchmark/corpus evidence, and all production/publication authority gates.
- Required W7 validation: clean-install artifact checks, worker clients,
  proportionate security and provenance review, public release check, and the
  authoritative `make v1` release gate.
- Implemented W7 surface: one deterministic Linux x86_64 archive, strict
  manifest/license/provenance/verification/toolchain contracts, SHA-256
  checksum, SPDX 2.3 SBOM, payload inventory, safe extraction gate, scrub,
  unpacked smoke, migration guide, threat model, release checklist, changelog,
  and post-GA plan.

## Active dogfood pilots

| Candidate | Class | Authority | Parity | Runtime | Decision |
|---|---|---|---|---|---|
| Siglum numerology reference v1 | deterministic rules | non-authoritative | fresh two-run 10,000-case zero-mismatch public-history replay at `bfeba32` | cold 5,325 ms; warm 76/76 ms; full 42.57/41.75 cases/s; peak tree RSS 1,684,430,848 bytes | retain as 1.2 reference workload; no production promotion |
| Cross-module rename K1/K3 | controlled agent maintenance | local isolated evaluation only | both arms strict-success in one retained trial | K1 3,404 tokens / 13,223 ms; K3 7,815 tokens / 36,810 ms | retain for bounded follow-up; structural context expanded; no promotion |

## Blocked work

None. W5 satisfies the bounded runtime, test, and validation dependency for W6.

## Decisions

| Decision | Options | Current recommendation | Authority |
|---|---|---|---|
| Executable `Raw` severity | error / warning / verify-only | **decided:** error in `check`; runtime remains fail-closed | W1 checker implementation |
| Multiline conditional | support / reject explicitly / formatter normalization | **decided:** retain current expression/statement forms | ADR-001 |
| Minimum stable schemas | selected external v1 roots / all v0 roots | **decided:** four W3 roots, four S12-502 worker roots, three S12-504 testing/review roots, the S12-505 validation report, and both S12-601 operational replay roots are stable; W4 transaction, S12-501 adapter, S12-602 comparison, and S12-603 registry roots remain extensible; provider namespaces remain reserved | ADR-002 through ADR-015, plus registry |

## Metrics

- Quick validation: 48.60 seconds, passed before W1 edits
- Changed validation: passed, including the comprehensive self-hosted suite,
  smoke 5/5, and corpus before the bounded blocker fix
- Post-fix focused validation: checker smoke passed; valid in-task control and
  same-module/same-line collision probes passed
- W2 targeted validation: all four IDs resolved; human/JSON parity, unknown-ID
  failure, LSP command preview, accepted-source check/format, and schema probes
  passed
- W2 integration remediation: the first change-aware run exposed nullable
  report-builder incompatibility; `explanation` became optional only on error,
  and the comprehensive self-hosted rerun passed
- Corpus: 96/96 steps passed; 24 accepted and 48 rejected cases
- Core validation: passed for W2; 149/149 contract fixtures, 59 schemas,
  226 declared integration checks, conformance, AI foundation, corpus
  governance, smoke 5/5, and syntax
- W1 independent review: Vulcan commit-ready, no findings after one medium
  cross-file correlation blocker was fixed
- W2 independent review: Vulcan commit-ready after the startup-template and
  strict LSP argument blockers were fixed; no remaining findings
- W3 targeted validation: 66-schema inventory matches its fixture; 155/155
  contract fixtures pass; stable registry and four bidirectional v0/v1
  compatibility pairs pass strict positive/negative checks
- W3 protocol validation: nested owned paths and one-action turns pass;
  concatenated/multi-action responses, unowned paths, sequential-turn drift,
  all six resource budgets, and ignored cancellation fail closed
- W3 retained-evidence check: the exact operator-custody K3 semantic mode
  summary validates against `greyforge.sleybench.mode_summary.v0`; only a
  synthetic value set is committed
- W3 Tier 2: `make core` passed with 66 schemas, 155/155 contract fixtures,
  232/232 declared integration checks, conformance, AI foundation, corpus
  governance, smoke 5/5, and syntax; focused conformance rerun passed with all
  28 v1 targets registered
- W3 independent review: Vulcan commit-ready with no report-grade findings;
  exact protocol, summary, compatibility, inventory, fixture, and diff checks
  passed
- W4 S12-401 focused validation: live inspection and its fixture validate;
  source and Git-index digests remain unchanged; malformed bindings, unknown
  options, invalid nonce/time order, symlinks, non-Git targets, out-of-repo
  repositories without a committed base, missing/invalid/out-of-repo traces,
  and unsupported apply fail with typed diagnostics
- W4 inventory after the first slice: 67 schemas, 156/156 contract fixtures,
  233 generated integration checks, and 29 v1 gate targets
- W4 independent review: Vulcan found one high-severity unborn-HEAD path that
  returned raw Git failure; `TRANSACTION_BASE_COMMIT_REQUIRED` and a real
  unborn-repository regression fixed it, and focused re-review is commit-ready
- W4 S12-402 inventory: 69 schemas and 158 contract fixtures; the focused
  transaction gate covers deterministic plan/preview replay, disposable
  candidate evidence, typed tamper/operation/symlink rejection, and unchanged
  repository source/index digests
- W4 S12-402 independent review: Vulcan found a high-severity mismatch between
  accepted file inspection and directory-only candidate copying; typed
  file/project copy handling plus a live single-file no-mutation regression
  resolved it, and focused re-review returned commit-ready
- W4 S12-403 inventory: 71 schemas, 160 contract fixtures, and 237 declared
  integration checks; focused grant
  coverage validates deterministic request/grant issuance, exact
  compiler-derived scope, explicit review acknowledgement, no-mutation
  boundaries, missing confirmation, missing review, re-digested request
  tampering, and the still-unsupported apply path
- W4 S12-403 independent review: Vulcan found no high or medium
  correctness/security defect and returned commit-ready; apply-time freshness,
  replay consumption, and revocation re-checking remain explicit S12-404 work
- W4 S12-403 Tier 2: `make core` passed with parser/checker/runtime/lint focus,
  71 schemas, 160/160 fixtures, compatibility and protocol replay, the full
  transaction lifecycle, 237/237 integration checks, AI-foundation drift,
  corpus governance, smoke 5/5, and self-hosted syntax
- W4 S12-404 focused validation: apply authorization, active and irreversible
  revocation, stale/replay denial, cancellation, three injected crash points,
  replay-marker tamper, topology drift, atomic exchange, required verification,
  automatic rollback, recovered rollback, and manual rollback passed
- W4 S12-404 inventory and Tier 2: 76 schemas, 165/165 contract fixtures,
  242/242 integration checks, compatibility and protocol replay, the full
  transaction lifecycle, AI-foundation drift, corpus governance, smoke 5/5,
  and self-hosted syntax passed under `make core`
- W4 S12-404 independent review: Vulcan reported no report-grade static
  security or correctness finding across replay, recovery placement,
  revocation, topology, dirfd exchange, lock, cancellation, verification, and
  rollback boundaries and returned commit-ready after the completed Tier 2
  evidence resolved its sole validation blocker
- W4 S12-405 focused validation: verified-apply and verified-rollback packets,
  exact historical chains, semantic tamper rejection, pinned trace validation,
  atomic no-replace publication, digest recomputation, replay denial,
  source/index/recovery no-mutation, and post-seal rollback denial passed
- W4 S12-405 inventory and Tier 2: 78 schemas, 167/167 contract fixtures,
  244/244 integration checks, compatibility and protocol replay, the full
  transaction lifecycle, conformance, AI-foundation drift, corpus governance,
  smoke 5/5, and self-hosted syntax passed under `make core`
- W4 S12-405 independent review: Vulcan found one medium pre-lock seal-check
  race; moving the repeated seal check and all live topology/digest validation
  under the shared lock plus a deterministic concurrent regression closed it.
  Focused re-review returned commit-ready with no remaining High or Medium
  finding
- W4 S12-406 focused MCP validation: the 14-tool catalog, lifecycle, framing,
  injection, path confinement, response/request ceilings, timeout behavior,
  and cancellation honesty pass; approve and revocation-record remain absent
- W4 S12-406 transaction parity: CLI-identical inspect, plan, preview, approval
  request, and apply authorization; typed confirmation denial; outside/symlink
  artifact rejection; verified apply and verification; terminal review; and
  post-seal rollback denial pass through the MCP adapter
- W4 S12-406 Tier 2 and independent review: `make core` passed with 78 schemas,
  167/167 fixtures, 244/244 integration checks, the full transaction lifecycle,
  conformance, AI-foundation drift, corpus governance, smoke 5/5, and syntax.
  Vulcan independently reran `make mcp-bridge`, found no High or Medium issue,
  and returned commit-ready
- W5 S12-501 focused validation: exact replay and record matching, typed
  authority/scope/seed/replay/cancellation/budget/live-provider denials,
  manifest and record tamper rejection, strict schema validation, raw-seed
  redaction, nested-symlink confinement, and source/index no-mutation passed
- W5 S12-501 Tier 2 and independent review: `make core` passed with 81 schemas,
  170/170 fixtures, 247/247 integration checks, all 30 declared v1 targets,
  the full transaction lifecycle, conformance, AI-foundation drift, corpus
  governance, smoke 5/5, and self-hosted syntax. Vulcan reran the focused gate,
  confirmed both prior Medium findings closed, found no remaining issue, and
  returned commit-ready
- W5 S12-502 focused validation: handshake, load, capabilities, deterministic
  invoke, digest/authority/budget/protocol denials, idempotency replay denial,
  bounded combined output flooding, post-preflight source mutation, process-
  group cancellation, fresh cancellation state, child crash containment,
  health, drain refusal, reset, and graceful shutdown passed in one session
- W5 S12-502 Tier 2 and independent review: `make core` passed with 85 schemas,
  174/174 fixtures, 251/251 integration checks, all 31 declared v1 targets,
  full transaction/adapter/worker lifecycles, conformance, AI-foundation drift,
  corpus governance, smoke 5/5, and self-hosted syntax. Vulcan's first review
  found two Medium output-buffer and preflight TOCTOU blockers; incremental
  bounded draining, immutable snapshots, copied/returned digest checks, and
  deterministic regressions closed both. Re-review found no remaining High or
  Medium issue and returned commit-ready
- W5 S12-503 focused validation: generated Python models compile; the packed
  Node declarations pass a strict NodeNext TypeScript consumer; local wheel
  and private npm packages install without network or runtime dependencies;
  both clients pass handshake, load, capabilities, invoke, deterministic
  rejection, cancellation, crash, health, shutdown, no-retry request counts,
  ordered framing, private-value exclusion, and post-reader-error rejection
- W5 S12-503 Tier 2 and independent review: the initial `make core` run passed
  parser, checker, runtime, lint, 85 schemas, 174/174 fixtures, compatibility,
  all transaction/adapter/worker lifecycles, 251/251 integration checks,
  conformance, AI-foundation drift, corpus governance, and smoke 5/5 before the
  syntax gate identified the unclassified host-client boundary. The narrow
  client allowlist plus affected `worker-clients`, conformance, syntax,
  validation-planner, format, and diff targets then passed with 32 declared v1
  targets. Vulcan found two Medium source-tree build-residue and post-reader-
  error send blockers; isolated package builds and fail-closed send guards plus
  regressions closed both. Final re-review found no High or Medium issue and
  returned commit-ready
- W5 S12-504 focused validation: deterministic two-manifest discovery, all six
  case kinds, exact task/branch/effect/capability coverage classification,
  byte-stable focused reports, uncovered-node failure, human output, strict
  contracts, exact review/seal/test identities, and tamper rejection passed
- W5 S12-504 Tier 2 and independent review: `make core` passed with 88 schemas,
  177/177 fixtures, 254/254 self-hosted checks, all 33 declared v1 targets,
  full transaction/adapter/worker/client/user-test lifecycles, conformance,
  AI-foundation drift, corpus governance, smoke 5/5, and source purity.
  Vulcan found a High mixed-node false-pass and Medium nested-source-symlink
  blocker; per-request graph expansion, strict source confinement, and exact
  regressions closed both. Re-review found no residual report-grade finding;
  its duplicate gate exceeded the council time budget, while the orchestrator
  completed the exact focused gate and Tier 2 gate successfully
- W5 boundary validation: the first full-gate run exposed a stale
  unused-import runtime oracle, corrected in `b24e97d`. The next run exposed a
  missing closed recovery exemption for the established typed record literal
  field-access example, corrected in `93d0ed3` after focused regression and
  Vulcan review found no report-grade issue
- Release validation: `make v1` passed on 2026-08-25 at `93d0ed3` with all 34
  release targets, 89 schemas, 178/178 fixtures, 255/255 declared integration
  checks, corpus 96/96, examples 146/146, and smoke 5/5
- W6 S12-601 focused validation: artifact pin tamper, fresh parity mismatch,
  unknown workload, strict contract, Bash syntax, and diff checks passed
- W6 S12-601 change-aware Tier 2: `make check-changed` passed at `73bc5b0`
  with the self-hosted core, transaction, adapter, worker, client, test,
  validation, conformance, corpus, MCP, machine, agent-bench, and governance
  targets; 91 schemas, 179/179 fixtures, 256/256 declared integration checks,
  corpus 96/96, and all 35 v1 targets were present
- Historical W6 S12-601 reference evidence: Sley `73bc5b0`, Siglum `2721acb`,
  two fresh 10,000-case runs, 20,000 total exact comparisons, zero mismatches, identical
  run identities, cold 5,084 ms, warm 80/81 ms, 43.52/43.41 cases/s, probe
  peak RSS 122,748,928 bytes, and full-controller-tree peak RSS 1,691,553,792
  bytes. Promotion remains deferred and non-authoritative
- W7 public-history replay refresh: sanitized Sley parent `bfeba32`, Siglum
  `2721acb`, two fresh 10,000-case runs, 20,000 total exact comparisons, zero
  mismatches, identical stable run identities, cold 5,325 ms, warm 76/76 ms,
  42.57/41.75 cases/s, probe peak RSS 127,926,272 bytes, and
  full-controller-tree peak RSS 1,684,430,848 bytes. Promotion remains
  deferred and non-authoritative
- W6 S12-601 independent QA: Vulcan's initial Medium wrapper finding was fixed
  with an explicit required Siglum root, Sley-owned pin enforcement, and
  direct CLI regressions; bounded recheck passed with no remaining High or
  Medium findings
- Full `make v1` was not repeated for S12-601; the last authoritative release
  boundary remains the W5 pass at `93d0ed3`
- SleyBench retained best controlled result: 109/120 strict, 111/120 oracle,
  Bootstrap 0.2
- Candidate 0.3: rejected at 107/120 strict, 109/120 oracle
