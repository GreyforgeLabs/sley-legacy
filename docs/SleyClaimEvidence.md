# Sley Auditable Claim Evidence

Status: public auditor packet
Last updated: 2026-05-11

## Claim Boundary

Canonical public claim:

> Sley is an agent-native structural programming language for
> compiler-mediated, human-reviewed software change.

The stronger search phrase "world's first fully AI-native programming language,
built by AI for AI" is a disputed category claim, not a standalone proof line.
Use it only with the criteria and evidence below. The public repository can
verify the language-design and implementation-stage claims. It does not, by
itself, prove who authored every line; any authorship claim needs a separate
public provenance record.

Sley does not yet claim strict self-hosting. Strict self-hosting requires the
parser, checker, runtime, and command semantics to execute from Sley source and
pass parity tests with the remaining bootstrap surface reduced to a loader and
test harness.

## Strict Criteria

A language satisfies Sley's agent-native structural claim when it has all of
these properties:

1. Independent language substrate: its own source form, semantic model, checked
   command surface, and runtime or execution contract. A Python, JavaScript, Go,
   or shell workflow wrapper is not enough.
2. Compiler-owned structural model: typed program structure is exposed through
   first-class machine contracts, not inferred from raw text diffs.
3. Native agent edit path: inspection, diagnostics, planned repairs, structural
   grafts, verification, traces, and seals are compiler-mediated surfaces.
4. Deterministic authority gates: host-facing effects are explicit, bounded,
   and reproducible in v0. The public proof path must not require live network,
   shell, secret-store, deployment, payment, spend, wallet, market, or provider
   calls.
5. Independent auditability: a clean public checkout must include exact commands
   that verify the claimed stage without private infrastructure.

This excludes orchestration frameworks, prompt libraries, syntax skins over
legacy language semantics, and model-serving or accelerator languages whose
main claim is faster machine-learning execution rather than compiler-mediated
structural editing.

## Evidence Map

| Claim | Public evidence | Audit command |
| --- | --- | --- |
| The public tree has no forbidden foreign-language implementation files under the self-hosting gate. | `scripts/check-self-hosted-code.sh` | `./scripts/check-self-hosted-code.sh` |
| Sley has a runnable stage-1 command surface. | `bin/sley`, `bin/sley-*`, `Makefile` | `bin/sley --version` |
| Sley-owned stage-2 semantic source modules exist. | `self-hosted/src/loom/*.sley` | `bin/sley self-hosting-status --json` |
| Bootstrap version, status fields, source modules, lint rules, schema IDs, diagnostics, runtime seed values, parser classifiers, report shapes, checker/lint/runtime status, and selected runtime probes are read from `.sley` source. | `self-hosted/src/loom/bootstrap.sley`, `parser.sley`, `checker.sley`, `lint.sley`, `runtime.sley`, `reports.sley` | `scripts/self-hosted-test.sh` |
| Structural inspection is exposed as JSON contracts. | `docs/schemas/`, `fixtures/contracts/`, `docs/contracts.md` | `bin/sley ast --json examples/hello.sley` and `bin/sley query --json --kind calls examples/project` |
| Structural edits can be checked before mutation. | `fixtures/ci_smoke_probe/insert_statement.json` | `bin/sley graft --json --dry-run fixtures/ci_smoke_probe/graft_target.sley fixtures/ci_smoke_probe/insert_statement.json` |
| Deterministic authority gates are modeled without live host calls. | `examples/*_gate.sley`, `scripts/self-hosted-test.sh` | `bin/sley run --json --cap SecretRead --secret api_key redacted examples/secret_gate.sley` |
| The claim boundary, criteria, evidence paths, prior-art candidates, and publication gates are machine-checkable. | `docs/SleyClaimManifest.json`, `docs/schemas/sley.claim.manifest.v0.schema.json`, `docs/schemas/sley.claim.verify.v0.schema.json` | `bin/sley claim-verify --json docs/SleyClaimManifest.json` |
| The public v1 gate is reproducible. | `Makefile` | `make v1` |
| The public release remains intentionally blocked until the proof bar is higher. | `docs/PublicReleaseChecklist.md` | `make public-release-check` |

## Audit Sequence

Run this from a clean checkout of the public branch:

```bash
export PATH="$(pwd)/bin:$PATH"
git status --short
./scripts/check-self-hosted-code.sh
scripts/self-hosted-test.sh
bin/sley claim-verify --json docs/SleyClaimManifest.json
make v1
bin/sley self-hosting-status --json
make public-release-check
```

Expected current result:

- `check-self-hosted-code.sh`, `scripts/self-hosted-test.sh`,
  `bin/sley claim-verify --json docs/SleyClaimManifest.json`, and `make v1`
  pass.
- `make public-release-check` fails intentionally with
  `PUBLIC_RELEASE_BLOCKED` until operator approval, strict Sley-written parity,
  and a published proof bundle are present.

## Prior-Art Discipline

External comparison claims must be made from a primary-source pack, not from
memory or marketing pressure. A public comparison packet should include:

- official repository or documentation links for every compared project;
- the exact criterion each project satisfies or fails;
- a short quote or paraphrase tied to the official source;
- the current date of verification;
- a correction path for maintainers if the classification is wrong.

The current public comparison source pack is
[`docs/SleyPriorArtSourcePack.md`](SleyPriorArtSourcePack.md). It resolves the
current working review queue as:

- Sley
- Dana
- Jac
- Codong, with Codon recorded as a name-collision note rather than the direct
  claimant
- Agentis
- Mojo

Do not open public issues on outside repositories until the source pack exists
and the exact issue text has operator approval. The source pack alone is not
approval to post. Public issues should be framed as classification and
interoperability questions with evidence, not drive-by marketing claims.

## Citable Summary

Sley's present, independently auditable claim is that it is an agent-native
structural programming language with a foreign-source-free public bootstrap
gate, Sley-owned stage-2 semantic modules, compiler-exposed graph/report
contracts, deterministic authority gates, checked graft previews, a
machine-checkable claim manifest, and a local v1 gate. The public proof is the
repository itself plus the commands above.
