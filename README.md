<p align="center">
  <img src="https://raw.githubusercontent.com/GreyforgeLabs/sley-legacy/public/assets/branding/canonical/sley_loom_graph_banner_1500x500.png" alt="Sley Loom banner" width="100%" />
</p>

<p align="center">
  <strong>Sley (Legacy)</strong><br />
  Software change, made structural.
</p>

<p align="center">
  <a href="https://github.com/sley-lang/sley"><b>Sley 2 (current)</b></a> |
  <a href="https://sleylang.org/">Website</a> |
  <a href="https://sleylang.org/docs">Technical brief</a> |
  <a href="https://sleylang.org/tutorial">Workflow walkthrough</a> |
  <a href="https://greyforge.tech/">Greyforge Labs</a>
</p>

<p align="center">
  <img alt="Sley v1 Gate" src="https://img.shields.io/badge/Sley%20v1%20Gate-38%2F38-22c55e" />
  <img alt="Self-hosted" src="https://img.shields.io/badge/compiler-self--hosted-22d3ee" />
  <img alt="License" src="https://img.shields.io/github/license/GreyforgeLabs/sley-legacy" />
</p>

# Sley (Legacy)

Sley is a self-hosted, agent-native structural programming language for
compiler-mediated, human-reviewed software change.

It gives autonomous coding workflows a structured path from intent to
inspection, planned change, authority, verification, and evidence-ready
handoff. Human-readable source stays central while the compiler exposes the
program structure tools need to work precisely.

> [!IMPORTANT]
> **Sley 1.2.1 completes the human-readable 1.x architecture.** This lineage is
> frozen and retained for historical use, study, experimentation,
> compatibility, and existing users. Active language development is the
> intentionally incompatible machine-native Sley 2.x lineage, developed in the open at
> [sley-lang/sley](https://github.com/sley-lang/sley) and documented at [sleylang.org](https://sleylang.org/docs). The existing
> Sley 1.x license terms remain unchanged. Human governance remains required in
> 2.x, but direct readability of the canonical raw program representation is no
> longer a mandatory design constraint. Read the
> [architectural-transition Chronicle](https://greyforge.tech/chronicles/sley-120-machine-native-break).

## Why Sley

Sley is built around the full software-change lifecycle:

- **Inspect structure:** query program shape, relationships, diagnostics, and
  bounded context.
- **Plan the delta:** express narrow candidate transformations before accepted
  mutation.
- **Check authority:** keep consequential effects behind explicit,
  machine-readable gates.
- **Verify the result:** carry checks, provenance, traces, seals, and handoff
  evidence with accepted work.
- **Integrate safely:** connect external development tools through bounded,
  read-only structural surfaces.

## Sley 1.2 Release Candidate

The 2026-08-26 release-candidate inventory contains:

| Evidence | Verified count |
| --- | ---: |
| Report schemas | 99 |
| Contract fixtures | 187 |
| Accepted corpus cases | 24 |
| Rejected corpus cases | 48 |
| Integration checks | 264 |
| Local v1 gate checks | 38 / 38 |

The self-hosted Sley compiler owns its parser, checker, lint, runtime,
bootstrap, and report semantics in Sley source. The public command layer turns
that language-owned implementation into a practical local workflow and
verification surface.

Canonical release tag: [`v1.2.1`](https://github.com/GreyforgeLabs/sley-legacy/releases/tag/v1.2.1),
published August 29, 2026. The release records 11 of 11 release-packet checks
and 4 of 4 public-release checks.

## Start Locally

The supported distribution is the self-contained Linux x86_64 archive. It
requires Bash 5, jq 1.6, Git 2, Python 3.12 with `jsonschema`, GNU tar 1.30,
gzip 1.10, Make 4, Node 20, npm 10, and `sha256sum`.

```bash
export PATH="$(pwd)/bin:$PATH"
make quick
sley doctor --toolchain --json
sley doctor --json examples/project
sley ast --json examples/hello.sley
sley query --json --kind calls examples/project
sley lint --json examples/empty_for_statement.sley
sley run --json examples/hello.sley
sley machine --source fixtures/machine/package --package-id machine-fixture --package-version v1
sley test --json fixtures/user_tests
sley validate --changed --explain .
sley validate --profile core .
sley reference-replay siglum-numerology --siglum-root /path/to/siglum --execute --json
sley verify --json examples/project
sley-agent-bench run --json --manifest fixtures/sleybench/smoke-v0/case-manifest.json --run-manifest fixtures/sleybench/smoke-v0/run-manifest.json
scripts/test-sleybench-split.sh
```

Maintainers build and verify the local, unsigned release candidate with:

```bash
make release-archive
make release-security-check
```

Those commands create and validate checksums, provenance, the versioned
manifest, license inventory, and SPDX 2.3 SBOM. They do not authorize a tag,
push, upload, publication, deployment, or announcement.

Use `make check-changed` for change-aware validation and reserve `make v1` for
authoritative integration or release boundaries. See
[Sley validation tiers](docs/ValidationTiers.md).

## Structural Workflow

The public command surface covers five connected lanes:

| Lane | Representative surfaces |
| --- | --- |
| Inspect | `ast`, `graph`, `query`, `lint`, `doctor` |
| Plan | `plan`, `fix --dry-run`, `graft --dry-run`, `graph-diff` |
| Authorize | `change inspect`, `change plan`, `change preview`, `change approval-request`, `change approve`, `change apply-authorization`, `change revocation-record`, `change apply`, `change recover`, `change rollback`, `change review` |
| Verify | `check`, `test`, `verify`, `trace`, `seal`, contract and conformance reports |
| Operate | `run`, staged-change guard, bounded governed tool bridge, local workbench |

Use `sley --help` for the complete command index.

**Agent-first. Human-readable. Change you can prove.**

## Developer Integrations

### Modular CLI

`bin/sley` is a minimal public launcher. Internal implementation is grouped by
owned command family under `lib/sley/`; shared dependency, argument, JSON,
numeric-bound, and temporary-cleanup behavior lives in one lifecycle module.
`make cli-modules` validates module boundaries and replays exact pre-extraction
golden stdout, stderr, exit, signal, multicall, and clean-package behavior.

### Staged-change guard

`git-sley-guard` checks affected staged Sley targets and contract changes from
the Git index before a commit enters project history.

### Bounded tool bridge

`sley-mcp-bridge` provides bounded query, lint, planning, dry-run graft,
verification, and governed local transaction calls inside an explicitly
configured repository root. Grant issuance and revocation-record creation stay
CLI-only operator actions. The bridge continues reading protocol frames while
one serial tool call runs, so an MCP cancellation notification can terminate
and reap the active tool process group.

### Structural comparison

`sley graph-diff` produces a three-way semantic report for human review. It
classifies structural overlap and keeps merge authority explicit.

### Persistent worker clients

The prerelease Python and Node reference clients under `clients/` implement
the stable `sley.worker.v1` JSONL stdio protocol. Their typed models and
protocol constants are generated from the canonical worker schemas. Run
`make worker-clients` to build both local packages in clean offline
environments and exercise handshake, invoke, cancellation, crash recovery,
health, deadlines, corrupt/oversized frames, late/duplicate responses, and
shutdown. Timed-out or locally cancelled waits retain only bounded tombstones;
neither client retries automatically, and neither package is published by this
repository workflow.

## Public Evidence

- [Architectural transition Chronicle](https://greyforge.tech/chronicles/sley-120-machine-native-break)
- [Machine-native transition source note](https://greyforge.tech/research/sley-machine-native-break-source-note-2026-08-27.md)
- [Sley technical brief](https://sleylang.org/docs)
- [Workflow walkthrough](https://sleylang.org/tutorial)
- [Claim evidence](docs/SleyClaimEvidence.md)
- [Language specification](docs/SleyLanguageSpec.md)
- [Contract reference](docs/contracts.md)
- [AI bootstrap](SLEY_AI.md)
- [AI interface specification](docs/SleyAISpec.md)
- [SleyBench specification](docs/SleyBenchSpec.md)
- [Sley corpus specification](docs/SleyCorpusSpec.md)
- [Read-only tool bridge](docs/SleyMcpBridge.md)
- [Structural comparison research](docs/SleyGraphDiffResearch.md)

## Repository Map

- `self-hosted/src/loom/` contains the self-hosted Sley compiler.
- `bin/` contains the local command and developer integration surfaces.
- `docs/schemas/` contains machine-checkable report contracts.
- `fixtures/contracts/` contains verified contract fixtures.
- `fixtures/corpus/` contains accepted and rejected language cases.
- `fixtures/sleybench/` contains the deterministic 24-case evaluator smoke
  suite; it is harness evidence, not model-performance evidence. Production
  private held-out material is intentionally absent from this repository.
- `scripts/` contains the deterministic local proof gates.
- `examples/` contains projects and workflow demonstrations.
- `ecosystem/` contains the preserved Sley 1.x contract kit, CI integration,
  Tree-sitter grammar, LSP, workbench, and conformance histories.
- `learning/learnsley/` contains the preserved LearnSley history.
- `research/audit/` contains the preserved Sley Audit history.
- [`MIGRATION_MAP.md`](MIGRATION_MAP.md) records source repositories, immutable
  source tips, destination paths, omitted refs, licenses, and tombstone status.
- [`LICENSE_SCOPE.md`](LICENSE_SCOPE.md) records the license boundary for every
  imported component without changing any source license.

## License

The Sley 1.x root is Apache-2.0. Imported component licenses remain controlling
inside their preserved subtrees. See [LICENSE](LICENSE), [NOTICE](NOTICE), and
[LICENSE_SCOPE.md](LICENSE_SCOPE.md).

Autonomy, Engineered.
