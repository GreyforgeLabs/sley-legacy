# Sley 1.2 canonical documentation index

Status: local release-candidate documentation
Owner: Sley maintainers
Baseline: `codex/sley-ai-foundation-20260822` at `cb9d714`
Review trigger: every completed Sley 1.2 work package

Sley 1.2 keeps **Governed Autonomy** as its north star and implements it in the
evidence-reconciled order below:

```text
diagnostic truth
  -> mechanical discoverability
  -> strict protocols and minimum contracts
  -> bounded local transactions
  -> isolated worker and user validation
  -> non-authoritative operational replay
  -> release closeout
```

Current planning artifacts:

- [`STATUS.md`](STATUS.md): branch checkpoint, completed work, active slice,
  blockers, decisions, and validation evidence.
- [`AUDIT.md`](AUDIT.md): current-state findings against the reconciled 1.2
  release boundary.
- [`ARCHITECTURE.md`](ARCHITECTURE.md): semantic ownership and dependency
  rules.
- [`ROADMAP.md`](ROADMAP.md): implementation waves and work-package DAG.
- [`RISK_REGISTER.md`](RISK_REGISTER.md): release risks, controls, and exit
  evidence.
- [`SLEY_FIRST.md`](SLEY_FIRST.md): bounded workload-selection doctrine.
- [`DOGFOOD_LEDGER.json`](DOGFOOD_LEDGER.json): machine-readable planning
  record for the current reference workload.
- [`OPERATIONAL_EVIDENCE.md`](OPERATIONAL_EVIDENCE.md): evidence methodology
  and current admissible claims.
- [`RELEASE_ARTIFACTS.md`](RELEASE_ARTIFACTS.md): supported Linux archive,
  checksum, provenance, license, SBOM, verification, and authority boundary.
- [`THREAT_MODEL.md`](THREAT_MODEL.md): proportionate release security model,
  controls, and residual risks.
- [`MIGRATION_1.1_TO_1.2.md`](MIGRATION_1.1_TO_1.2.md): compatible upgrade and
  contract-boundary guidance.
- [`POST_GA_PLAN.md`](POST_GA_PLAN.md): ordered deferred adoption, platform,
  ecosystem, and evidence work.
- [`ADR-001-multiline-conditionals.md`](ADR-001-multiline-conditionals.md):
  accepted W1 policy: keep the current expression and statement forms; do not
  add grammar without real-user evidence.
- [`ADR-002-contract-stability-floor.md`](ADR-002-contract-stability-floor.md)
  defines the W3 minimum stable roots, strict compatibility, and producer-cutover
  boundary.
- [`ADR-003-local-transaction-state.md`](ADR-003-local-transaction-state.md):
  W4 read-only transaction identity and inspection boundary.
- [`ADR-004-immutable-candidate-preview.md`](ADR-004-immutable-candidate-preview.md)
  defines the W4 strict planning and disposable candidate preview boundary.
- [`ADR-005-exact-local-grants.md`](ADR-005-exact-local-grants.md) defines the
  W4 compiler-derived approval request and explicit local grant boundary.
- [`ADR-006-bounded-atomic-apply.md`](ADR-006-bounded-atomic-apply.md) defines
  the W4 Linux same-filesystem atomic exchange, cancellation, crash recovery,
  required verification, and verified rollback boundary.
- [`ADR-007-terminal-review-and-seal.md`](ADR-007-terminal-review-and-seal.md)
  defines terminal evidence eligibility, atomic review publication, the
  content-addressed digest chain, and post-seal rollback prohibition.
- [`ADR-008-local-adapter-replay.md`](ADR-008-local-adapter-replay.md) defines
  the W5 local-only adapter identity, exact seeded authority, redacted
  deterministic replay, budget, cancellation-observation, and no-isolation
  boundary.
- [`ADR-009-persistent-local-worker.md`](ADR-009-persistent-local-worker.md)
  defines the stable worker protocol, fresh-process request isolation,
  preemptive cancellation, lifecycle, crash containment, and truthful
  namespace limitation.
- [`ADR-010-worker-reference-clients.md`](ADR-010-worker-reference-clients.md)
  defines schema-generated Python/TypeScript models, dependency-free stdio
  clients, strict framing and cancellation semantics, no automatic retry, and
  clean offline package validation.
- [`ADR-011-first-class-testing.md`](ADR-011-first-class-testing.md) defines
  manifest-backed discovery, six bounded case kinds, truthful coverage,
  graph-derived changed-node focus, stable reports, and read-only review
  evidence binding.
- [`ADR-012-validation-profiles-and-reports.md`](ADR-012-validation-profiles-and-reports.md)
  defines repository-local quick, changed, core, and release profiles,
  machine-readable selection and skip truth, and exclusive `make v1` release
  authority.
- [`ADR-013-operational-reference-replay.md`](ADR-013-operational-reference-replay.md)
  defines the pinned independent-oracle Siglum replay, persistent host probe,
  bounded resources, disclosure boundary, and explicit production deferral.
- [`ADR-014-controlled-agent-workflow.md`](ADR-014-controlled-agent-workflow.md)
  defines the equal-control K1 raw-file versus K3 structural-augmentation
  comparison, strict oracle, ACT/context measures, private evidence custody,
  and separate provider-approval gate.
- [`ADR-015-operational-evidence-registry.md`](ADR-015-operational-evidence-registry.md)
  defines the exact two-entry W6 decision registry, real-evidence admission
  checks, visible limitation retention, and unconditional production deferral.
- [`CONTRACT_STABILITY.json`](CONTRACT_STABILITY.json): machine-readable
  stability classes and remaining reserved namespaces.

The W6 SleyBench evidence remains historically pinned to the exact Bootstrap
0.2 digest used for those runs. The release-candidate `SLEY_AI.md` header is
version-coupled to Sley 1.2.1 and has its own current digest.
These planning files do not authorize benchmark edits, training, provider
calls, production shadowing, deployment, publication, merge, or push.
