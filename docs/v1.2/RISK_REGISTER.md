# Sley 1.2 risk register

Status: active
Owner: Sley maintainers
Review trigger: every work-package closeout

| ID | Risk | Likelihood | Impact | Control | Exit evidence |
|---|---|---:|---:|---|---|
| S12-R01 | `Raw` remains accepted as runnable | high | high | recursive checker pass; runtime stays fail-closed | rejected fixtures and check/run parity |
| S12-R02 | Valid recovery AST use is rejected indiscriminately | medium | high | scope error to executable expressions; characterize accepted corpus | accepted corpus and parser fixtures pass |
| S12-R03 | Host script becomes semantic owner | medium | high | IDs/messages/patterns/pass data live in Sley modules; host executor stays generic | self-hosting ownership tests |
| S12-R04 | Benchmark friction causes gratuitous grammar growth | medium | high | diagnostic/tooling-first ADR gate | no syntax change without independent evidence |
| S12-R05 | Inline `take` fix breaks valid task names or declarations | medium | medium | freeze valid grammar and add positive/negative fixtures | parser/checker corpus pass |
| S12-R06 | Diagnostic contract breaks LSP/CI consumers | medium | high | keep v0 shape; add stable IDs/fixtures; test nested consumers | contracts, LSP, CI focused gates |
| S12-R07 | Prompt tuning replaces compiler work | low | high | Bootstrap 0.2 pinned; successor requires governed evidence | bootstrap drift gate |
| S12-R08 | Transaction work starts before truth/contracts | medium | high | W1-W3 dependency gate | roadmap/status review |
| S12-R09 | Governed writes broaden authority | medium | critical | exact candidate/base/path/effect grants; fail closed | adversarial grant/replay tests |
| S12-R10 | Worker leaks state/authority across requests | medium | critical | digest-addressed immutable cache only; isolation tests | lifecycle and cross-request tests |
| S12-R11 | SleyBench history or held-out custody changes | low | high | frozen v0; versioned future maintenance only | split and digest verification |
| S12-R12 | One-model evidence becomes a broad claim | medium | high | generated claim facts and explicit limitations | docs/claim audit |
| S12-R13 | Siglum parity is mistaken for production readiness | medium | critical | retain non-authoritative status; require separate shadow gates | dogfood ledger and Siglum approval |
| S12-R14 | Scope expands into post-1.2 ecosystem work | high | high | W0-W7 critical path and deferred backlog | status/DAG review |
| S12-R15 | CLI, LSP, docs, and bootstrap guidance diverge for W1 diagnostics | medium | high | one Sley-owned explain catalog; version context and command-preview parity | explain/schema/corpus/LSP fixtures |
| S12-R16 | Historical controller failures are hidden by permissive parsing | medium | high | single-document boundary, one-action schema, exact defect replays | concatenated/multi-action negative fixtures |
| S12-R17 | Schema registration is misreported as producer cutover or stable support | medium | high | explicit stability classes and no-cutover ADR | registry/producer inventory review |
| S12-R18 | Stable v1 roots drift through their v0 schema dependencies | medium | high | freeze stable shapes and run bidirectional compatibility plus strict negatives | compatibility gate |
| S12-R19 | Cancellation evidence is overstated as process preemption | medium | high | model cancellation as requested/observed replay only | cancellation positive/negative fixtures and docs |
| S12-R20 | Read-only inspection is mistaken for a complete governed transaction | medium | critical | expose only `change inspect`; reject unsupported subcommands; document blocked write dependencies | typed negative apply test and ADR-003 |
| S12-R21 | Inspection silently changes repository source, index, or trace state | low | critical | explicit false mutation contract and before/after digests | transaction no-mutation gate |
| S12-R22 | Transaction identity is replayed against a different repository or mutable base | medium | critical | bind repository identity, HEAD, source/graph/trace digests, nonce, and expiry | strict report schema and adversarial binding tests |
| S12-R23 | Preview leaks a mutable workspace or silently changes repository state | medium | critical | disposable candidate copy, stable path normalization, owned-path checks, and false mutation contract | deterministic repeated preview plus source/index digest gate |
| S12-R24 | Model rationale is confused with compiler-derived edit facts | medium | high | separate requested outcome/assumptions/non-goals from compiler nodes/files/diffs and bind both in the plan digest | strict plan schema and focused assertions |
| S12-R25 | Self-digested request or repository text is mistaken for approval authority | medium | critical | regenerate the request from the original preview and repository; require exact comparison and explicit operator fields | re-digested tamper and missing-confirmation regressions |
| S12-R26 | Local issuer or optional proof reference is overstated as authenticated identity | medium | high | label issuer as explicit CLI assertion and proofs as unverified | strict grant schema and documentation |
| S12-R27 | Grant issuance overstates replay, revocation, or working-tree enforcement | medium | critical | bind exact base and scope; state enforcement is deferred; expose no apply path | grant assertions, unsupported apply, and ADR-005 |
| S12-R28 | MCP transport broadens authority or diverges from CLI transaction semantics | medium | critical | named argv adapters only; fixed repository; confined regular artifacts; CLI-only authority issuance; mirrored confirmations and typed outputs | positive/negative CLI-MCP parity and confinement tests |

## W1 closeout

Controls for S12-R01 through S12-R06 are implemented and passing at the W1
boundary. Vulcan identified one S12-R02 cross-file false negative during
review; the per-file context repair and exact collision fixture now pass.
These risks remain active for regression monitoring until the release gate.

## W2 closeout

S12-R06 and S12-R15 controls are additive: diagnostics v0 is unchanged,
`sley.explain.report.v0` is separately versioned, LSP points to the same lookup,
and accepted formatter/corpus coverage locks the documented spellings. Stable
diagnostics v1 promotion remains a W3 compatibility decision.

## W3 closeout

Reconciliation R07-R08, R13, R21, R23, and R31 map to S12-R07/S12-R08 and
S12-R16 through S12-R19 here. The implementation locks strict controller
replays, selects only four stable external v1 roots, registers the exact
combined-summary shape, keeps frozen benchmark custody unchanged, and reserves
future authority/runtime namespaces. Tier 2 and independent review passed;
these controls remain active for regression monitoring through the release
gate.

## W4 progress

ADR-003 activates the S12-R20 through S12-R22 controls for read-only
inspection. ADR-004 activates S12-R23 and S12-R24 through strict planning,
disposable candidate execution, stable evidence identities, owned-path checks,
and repeated no-mutation preview. ADR-005 activates S12-R25 through S12-R27
through compiler regeneration, exact comparison, explicit assertions, honest
proof labels, and deferred enforcement labels. Request and grant issuance keep
all five mutation flags false. ADR-006 activates the S12-404 atomicity, replay,
revocation, cancellation, recovery, and verified rollback controls. ADR-007
activates terminal placement, exact evidence-chain, pinned trace,
no-source-in-packet, atomic no-replace publication, and post-seal rollback
controls. S12-406 activates the S12-R28 controls. The MCP bridge delegates nine
named transaction surfaces to the existing CLI, fixes repository selection to
the configured Git root, rejects outside and symlink artifacts, preserves typed
CLI diagnostics, and keeps grant and revocation issuance CLI-only. The
synchronous stdio bridge continues to document timeout-bounded rather than live
active-call cancellation.

## W5 S12-501 progress

ADR-008 activates the first adapter controls. Canonical adapter and runtime
identities, exact seed-backed scopes, repository-confined inputs, digest-only
records, replay pins, input/output/wall/step/call/depth bounds, typed
cancellation observation, and explicit live-provider denial constrain the
local host. Reports state that OS isolation and persistence remain absent, so
S12-502 cannot inherit an unsupported isolation claim. External adapter,
provider, deployment, and spend execution remain outside the release surface.
Raw text seeds are digest-projected before verifier dispatch, and nested target
symlinks fail closed before digesting or static/runtime inspection.

## W5 S12-502 progress

ADR-009 activates one stable serial worker protocol. Exact message bindings,
unique session nonces, fresh child processes, private temporary homes, clean
environments, closed inherited descriptors, process-group cancellation,
resource limits, disabled caches, no automatic retry, and reset/drain/crash
tests constrain cross-request state and authority leakage. The worker does not
claim namespace, filesystem, or network sandboxing; current host user-namespace
controls are unavailable. External providers and persistent mutation remain
denied by the underlying S12-501 adapter boundary. Private immutable input
snapshots close the preflight/execution path race, and a combined incremental
stdout/stderr cap prevents child output from exhausting controller memory
before budget enforcement.

## W5 S12-503 progress

ADR-010 constrains both host clients to the stable stdio protocol and generated
schema models. Exact request/nonces, one-response correlation, strict UTF-8
JSONL framing, bounded retained messages, separate cancellation, and no
automatic retry reduce semantic drift and replay risk. Local packages build
from temporary sources, install without network or runtime dependencies, and
leave the repository unchanged. Fake-worker regressions prove that an invalid
or unsolicited stream response poisons the reader and makes later sends reject
instead of hanging. The Node package remains private and no registry,
provider, HTTP, deployment, or production authority is opened.
