# Sley 1.2 implementation roadmap

Status: active dependency plan
Owner: Sley maintainers
Updated: 2026-08-26

## Ordered waves

| Wave | Work packages | Exit evidence |
|---|---|---|
| W0 | `S12-000A` repository audit; `S12-000B` evidence reconciliation; `S12-000C` planning baseline | committed planning artifacts and passing Tier 1 baseline |
| W1 | `S12-101` executable `Raw`; `S12-102` control-flow contexts; `S12-103` task-parameter placement; `S12-104` multiline policy ADR | focused rejected fixtures, consistent JSON/human diagnostics, Tier 2 core |
| W2 | `S12-201` explain/spec lookup; `S12-202` AST/schema/example alignment; `S12-203` formatter/LSP diagnostic alignment | one machine/human answer for supported expression forms |
| W3 | `S12-301` controller protocol regressions; `S12-302` minimum schema set; `S12-303` mode-summary schema; `S12-304` compatibility fixtures | strict replays and registered producer roots |
| W4 | `S12-401` transaction state machine; `S12-402` immutable preview; `S12-403` exact local grants; `S12-404` atomic apply/rollback; `S12-405` review/seal; `S12-406` CLI/MCP parity | governed local transaction demonstrations |
| W5 | `S12-501` minimum adapter/replay; `S12-502` isolated worker; `S12-503` Python/Node clients; `S12-504` user tests; `S12-505` validation reports | worker isolation and full subsystem validation |
| W6 | `S12-601` Siglum reference replay; `S12-602` controlled agent workflow; `S12-603` operational evidence decision | non-authoritative comparison packet and explicit deferral/promotion state |
| W7 | `S12-701` release artifact; `S12-702` security; `S12-703` docs/migration; `S12-704` provenance/SBOM; `S12-705` release candidate gate | clean release candidate; external release remains approval-gated |

Wave state: W0 through W6 are complete. W4 includes S12-401 read-only
inspection, S12-402 immutable plan/preview, S12-403 exact local grants,
S12-404 bounded atomic apply/rollback, and S12-405 terminal review/seal
plus S12-406 governed CLI/MCP transaction parity. W5 is complete with S12-501
local adapter/replay, S12-502 persistent worker, S12-503 Python/Node reference
clients, S12-504 first-class user tests, and S12-505 machine-readable
validation profiles/reports. W6 closed with the bounded Siglum replay, one
approved provider-backed K1/K3 trial, independent review, and an exact
non-authoritative decision registry that defers promotion. W7 is active with
the supported Linux x86_64 archive, security verification, stable release
contracts, provenance, checksum, license inventory, SPDX SBOM, migration and
post-GA docs implemented. The final clean artifact and authoritative release
gate are the remaining mutation-free closeout steps. Tagging, pushing,
uploading, signing, publication, deployment, and announcement remain outside
the granted authority.

## Current file ownership

| Package | Owned paths | Must not modify |
|---|---|---|
| S12-000C | `docs/v1.2/**` | compiler/runtime behavior |
| S12-101 | `self-hosted/src/loom/checker.sley`, generic checker wiring in `bin/sley`, focused fixtures/tests, directly affected docs | runtime semantics, SleyBench cases/oracles, corpus governance state |
| S12-102 | parser/checker descriptors and focused fixtures after S12-101 | grammar without ADR |
| S12-103 | parser/checker declaration validation and focused fixtures | valid task/take syntax |
| S12-104 | ADR/documentation first | parser/runtime until decision is accepted |
| S12-201 | checker explanation catalog, explain report/CLI, exact fixtures | diagnostics v1 promotion, grammar/runtime, pinned `SLEY_AI.md` |
| S12-202 | accepted/rejected examples and unchanged AST/schema alignment | new grammar or AST nodes |
| S12-203 | formatter round-trip and LSP command preview | separate editor truth table |
| S12-301 | strict protocol schema, replay fixtures, and semantic gate | benchmark cases, oracles, provider custody |
| S12-302 | four stable v1 compatibility roots and stability registry | producer cutover or W4/W5 semantics |
| S12-303 | exact mode-summary schema plus synthetic fixture | retained private values or held-out cases |
| S12-304 | v0/v1 positive and strict negative compatibility gate | weakened parsing |
| S12-401 | `loom.transaction`, strict inspect contract, read-only `sley change inspect`, focused no-mutation and boundary tests | candidate creation, approval, source/index writes, MCP writes |
| S12-402 | strict change plan/preview contracts, disposable candidate executors, source and semantic evidence, focused no-mutation/replay tests | approval, grants, repository writes, rollback, MCP writes |
| S12-403 | strict approval-request/grant contracts, local authority vocabulary, exact request regeneration, explicit assertions, and no-mutation/adversarial tests | apply, replay consumption, revocation enforcement, rollback, MCP writes |
| S12-404 | strict apply authorization and revocation records, single-use replay, bounded common-root exchange, cancellation, digest-classified crash recovery, required verification, and automatic/manual rollback | review, seal, and MCP writes remain owned by S12-405 and S12-406 |
| S12-405 | exact terminal chain and placement validation, bounded human/machine review artifacts, pinned trace evidence, atomic no-replace publication, and content-addressed transaction seal | MCP methods and semantic parity remain owned by S12-406 |
| S12-406 | nine named MCP transaction adapters, confined regular artifact paths, exact CLI success/diagnostic forwarding, explicit mutation gates, and end-to-end parity evidence | authority issuance, arbitrary shell/dispatch, HTTP, and transport-private transaction semantics remain excluded |
| S12-501 | `loom.adapter`, three strict extensible v0 contracts, repository-confined local replay CLI, exact seeded authority, redacted records, deterministic pins, typed denials, and focused no-mutation evidence | OS isolation, persistent workers, client SDKs, external adapters, and live providers remain excluded |
| S12-502 | `loom.worker`, four stable v1 protocol roots, serial persistent controller, fresh bounded child per invoke, process-group cancellation, crash containment, reset/drain/health/shutdown, disabled caches, and truthful namespace limitation | Python/Node clients, user tests, performance reports, external adapters, parallel invocation, automatic retry, and namespace sandboxing remain excluded |
| S12-503 | generated Python and TypeScript models, dependency-free Python/Node stdio clients, clean offline package lifecycle tests, strict framing/correlation, explicit cancellation, and no automatic retry | `sley test`, validation reports, HTTP, external adapters, public package publication, parallel invocation, and Rust/Zig clients remain excluded |
| S12-504 | stable manifest/report/review roots, deterministic discovery, six bounded case kinds, graph-derived changed-node focus, truthful four-state coverage, exact identity binding, and focused/Tier 2 regressions | language-native test syntax, arbitrary shell tests, changed-file derivation, validation profiles, live providers, parallel execution, and public release remain excluded |
| S12-505 | stable validation report, Sley-owned quick/changed/core/release registry, machine changed-path plans, bounded digested executor evidence, exact skip/release/cache truth, and Make authority preservation | cross-repository classification, validation-result caching, per-check timing claims, provider/network execution, and release authority outside `make v1` remain excluded |
| S12-601 | exact Siglum/artifact pins, frozen corpus and retained-report validation, persistent cold/warm probe, fresh two-run differential parity, bounded resources, sampled RSS, negative evidence, strict report, and promotion deferral | production shadow, product latency, fallback, rollback, promotion, publication, provider, network, deploy, and spend authority remain excluded |
| S12-602 | `loom.operational`, strict public rename fixture/oracle, equal-control K1/K3 run pins, no-call plan, scrubbed comparison assembler, ACT/context measures, private evidence custody, and deterministic negative-denominator tests | provider execution/spend without exact approval, controller mutation, raw private evidence publication, production promotion, broad model claims, and corpus/benchmark rewrites remain excluded |
| S12-603 | Sley-owned registry vocabulary, strict extensible registry contract, read-only two-artifact assembler, exact ancestry/identity/authority/limitation checks, synthetic fixture, and adversarial focused tests | generating final evidence without a real S12-602 comparison, provider execution, production promotion, publication, deploy, runtime mutation, and W7 release closeout remain excluded |
| S12-701 | Sley-owned release vocabulary, deterministic self-contained Linux x86_64 archive, embedded version manifest, clean-install verification, and toolchain doctor | other platforms, channels, public upload, and package-manager installation remain excluded |
| S12-702 | safe-member validation, no-overwrite output, payload digest inventory, host-path and secret-pattern scrub, dependency/PATH diagnosis, and adversarial tamper tests | signature identity and public hosting security remain excluded |
| S12-703 | canonical index, 1.1 migration guide, changelog, claim boundary, release checklist, and post-GA plan | website deployment and public announcement remain excluded |
| S12-704 | SHA-256 checksum, unsigned provenance, SPDX 2.3 SBOM, Apache-2.0 inventory, stable release contracts, and external/embedded metadata binding | cryptographic signing and provider attestation remain excluded |
| S12-705 | change-aware release target plus clean artifact, public-readiness, and full authoritative local gates | tag, push, upload, publish, deploy, and announce require separate exact approval |

## Validation by package

- W0 documentation: diff check, links/paths, `make quick` baseline.
- W1 parser/checker: targeted probes, parser/checker focus, `make
  check-changed`, rejected corpus, `make core`.
- Contract changes: contract fixtures, compatibility/protocol replays, then
  `make core`.
- Transaction changes: focused `transaction-contracts`, contract inventory,
  compatibility and classifier gates, then `make core` at a bounded W4 unit.
- Runtime/worker: focused lifecycle and isolation tests plus `make core`.
- Adapter replay: focused manifest, authority, redaction, replay, cancellation,
  budget, tamper, and no-mutation tests plus contracts and `make core`.
- Worker clients: generator drift, strict TypeScript consumer, clean offline
  wheel/npm installs, lifecycle/cancellation/crash/no-retry framing tests,
  source-purity boundary, conformance, and `make core` at the bounded unit.
- Validation reports: exact profile-to-Make drift checks, changed-path and no-
  Arena selection, strict fixture and failure reports, bounded-output evidence,
  conformance, live narrow execution, Vulcan review, and `make core` at W5
  closeout.
- Operational reference replay: exact pin and artifact tamper checks, fresh
  parity mismatch rejection, persistent cold/warm determinism, two complete
  independent-oracle runs, report contract validation, negative evidence,
  disclosure and authority review, then change-aware Tier 2 validation.
- Controlled agent workflow: exact task/controller/run pins, strong structural
  and behavioral oracle, equal-control validation, synthetic accepted and
  structural-tool-missing comparisons, private evidence scrub, contract and
  conformance checks, then independent review. Actual model execution is a
  separate operator-approved step.
- Operational evidence decision: exact S12-601/S12-602 schema, digest,
  ancestry, authority, disclosure, limitation, and synthetic-fixture checks;
  deterministic two-entry output; contract/conformance/classifier coverage;
  and independent review before W6 closeout.
- Full `make v1`: only at the release/integration conditions defined by
  `docs/ValidationTiers.md`.

## Deferred backlog

HTTP tenancy, broad external adapters, SleyBridge translators, model training,
corpus expansion, extra Siglum domains, three-pilot rollout, public extraction,
semantic merge, new language constructs, Wasm, and package-registry work are
not on the 1.2 GA critical path.
