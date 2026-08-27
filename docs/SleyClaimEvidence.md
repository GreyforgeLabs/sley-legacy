# Sley Public Evidence

Status: current public auditor packet
Last updated: 2026-08-27

## Version Boundary

Sley 1.2.0 completes the original agent-native, human-readable 1.x
architecture. Greyforge has frozen active feature development of this line and
moved language research to a clean machine-native Sley 2.x architecture. The
public 1.x repository remains available under Apache-2.0. Compatibility is not
promised. The canonical public transition record is:

https://greyforge.tech/chronicles/sley-120-machine-native-break

## Canonical Description

> Sley is a self-hosted, agent-native structural programming language for
> compiler-mediated, human-reviewed software change.

This packet maps that description to inspectable repository evidence and
deterministic local checks.

## Product Criteria

Sley's public category rests on five connected properties:

1. **Independent language substrate:** Sley has its own source form, semantic
   model, checked command surface, and execution contract.
2. **Self-hosted implementation:** parser, checker, lint, runtime, bootstrap,
   and report semantics are owned by Sley source.
3. **Compiler-owned structural model:** typed program structure is exposed
   through first-class machine contracts.
4. **Native structural change path:** inspection, diagnostics, planning,
   structural grafts, verification, traces, seals, and handoff are
   compiler-mediated surfaces.
5. **Independent auditability:** a public checkout includes deterministic
   commands that verify the current implementation and contract surface.

## Evidence Map

| Claim | Repository evidence | Local check |
| --- | --- | --- |
| The implementation tree is foreign-source-free under the Sley self-hosting gate. | `scripts/check-self-hosted-code.sh` | `./scripts/check-self-hosted-code.sh` |
| The self-hosted Sley compiler owns parser, checker, lint, runtime, bootstrap, and report semantics. | `self-hosted/src/loom/`, `scripts/self-hosted-test.sh` | `scripts/self-hosted-test.sh` |
| Sley exposes a runnable local command surface. | `bin/sley`, `bin/sley-*`, `Makefile` | `bin/sley --version` |
| Structural inspection uses machine-checkable report contracts. | `docs/schemas/`, `fixtures/contracts/`, `docs/contracts.md` | `bin/sley query --json --kind calls examples/project` |
| Planned changes support review before accepted mutation. | `fixtures/ci_smoke_probe/`, graft and fix tests | `bin/sley graft --json --dry-run fixtures/ci_smoke_probe/graft_target.sley fixtures/ci_smoke_probe/insert_statement.json` |
| Authority-gated effects are explicit and deterministic. | `examples/*_gate.sley`, runtime tests | `bin/sley run --json --cap SecretRead --secret api_key redacted examples/secret_gate.sley` |
| The complete local v1 proof surface is reproducible. | `Makefile`, schemas, fixtures, corpus, integration tests | `make v1` |
| The supported release candidate is content-addressed and clean-install tested. | `scripts/sley-release.sh`, `docs/v1.2/RELEASE_ARTIFACTS.md`, release schemas | `make release-archive release-security-check` |

## Current Verified Counts

The 2026-08-26 release-candidate inventory contains:

- 99 report schemas
- 187 contract fixtures
- 24 accepted corpus cases
- 48 rejected corpus cases
- 264 integration checks
- 38 local v1 gate targets

## Self-Hosted Compiler

The self-hosted source tree includes these release-critical semantic modules:

- `loom.bootstrap`
- `loom.parser`
- `loom.checker`
- `loom.lint`
- `loom.runtime`
- `loom.reports`
- `loom.transaction`
- `loom.adapter`
- `loom.worker`
- `loom.testing`
- `loom.validation`
- `loom.operational`
- `loom.release`

The local proof gate exercises language-owned parsing, checking, linting,
runtime evaluation, report construction, dispatch, diagnostics, structural
queries, planned changes, and verification paths across those modules.

## Developer Integration Evidence

The current public branch also verifies:

- a staged-change guard that materializes and validates the Git index;
- a bounded read-only tool bridge for structural inspection and planning;
- a report-only three-way structural comparison surface;
- schema-backed contract and conformance reporting;
- provenance, trace, seal, deployment-plan, and handoff report surfaces;
- deterministic local demonstrations and workbench views.

## Audit Sequence

Run from a clean checkout of the public branch:

```bash
export PATH="$(pwd)/bin:$PATH"
git status --short
./scripts/check-self-hosted-code.sh
scripts/self-hosted-test.sh
make v1
make release-archive
make release-security-check
```

Each command is local and deterministic. The complete gate covers syntax,
foreign-source policy, self-hosted semantics, schemas, fixtures, corpus cases,
integration checks, developer integrations, and public contract surfaces.

## Comparison Discipline

Historical and category comparisons use the primary-source packet in
[`docs/SleyPriorArtSourcePack.md`](SleyPriorArtSourcePack.md). Public comparison
statements should name the criteria being compared, link the primary sources,
record the verification date, and provide a correction path.

## Citable Summary

Sley is Greyforge Labs' self-hosted, agent-native structural programming
language for compiler-mediated, human-reviewed software change. Its public
repository includes a language-owned implementation, compiler-exposed
structural contracts, planned-change previews, deterministic authority gates,
verification and handoff artifacts, and a reproducible local v1 proof surface.
