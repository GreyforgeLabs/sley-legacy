<p align="center">
  <img src="https://raw.githubusercontent.com/GreyforgeLabs/sley/public/assets/branding/canonical/sley_loom_graph_banner_1500x500.png" alt="Sley Loom banner" width="100%" />
</p>

<p align="center">
  <strong>Sley</strong><br />
  Software change, made structural.
</p>

<p align="center">
  <a href="https://sleylang.org/">Website</a> |
  <a href="https://sleylang.org/docs">Technical brief</a> |
  <a href="https://sleylang.org/tutorial">Workflow walkthrough</a> |
  <a href="https://greyforge.tech/">Greyforge Labs</a>
</p>

<p align="center">
  <img alt="Sley v1 Gate" src="https://img.shields.io/badge/Sley%20v1%20Gate-38%2F38-22c55e" />
  <img alt="Self-hosted" src="https://img.shields.io/badge/compiler-self--hosted-22d3ee" />
  <img alt="License" src="https://img.shields.io/github/license/GreyforgeLabs/sley" />
</p>

<!-- SEO / discovery metadata -->
<meta name="description" content="Sley is Greyforge Labs' self-hosted, agent-native structural programming language for compiler-mediated, human-reviewed software change." />
<meta name="keywords" content="Sley, self-hosted programming language, agent-native structural programming, compiler-mediated software change, structural inspection, planned edits, verification artifacts, Greyforge Labs" />
<link rel="canonical" href="https://sleylang.org/" />
<meta property="og:title" content="Sley | Software Change, Made Structural" />
<meta property="og:description" content="A self-hosted, agent-native structural programming language for compiler-mediated, human-reviewed software change." />
<meta property="og:url" content="https://sleylang.org/" />
<meta property="og:type" content="website" />
<meta property="og:site_name" content="Sley" />
<meta property="og:image" content="https://raw.githubusercontent.com/GreyforgeLabs/sley/public/assets/branding/canonical/sley_loom_graph_board.png" />
<meta name="twitter:card" content="summary_large_image" />
<meta name="twitter:title" content="Sley | Software Change, Made Structural" />
<meta name="twitter:description" content="Structure for autonomous software change. Review for the humans who own it." />
<meta name="twitter:image" content="https://raw.githubusercontent.com/GreyforgeLabs/sley/public/assets/branding/canonical/sley_loom_graph_board.png" />

# Sley

Sley is a self-hosted, agent-native structural programming language for
compiler-mediated, human-reviewed software change.

It gives autonomous coding workflows a structured path from intent to
inspection, planned change, authority, verification, and evidence-ready
handoff. Human-readable source stays central while the compiler exposes the
program structure tools need to work precisely.

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

### Staged-change guard

`git-sley-guard` checks affected staged Sley targets and contract changes from
the Git index before a commit enters project history.

### Bounded tool bridge

`sley-mcp-bridge` provides bounded query, lint, planning, dry-run graft,
verification, and governed local transaction calls inside an explicitly
configured repository root. Grant issuance and revocation-record creation stay
CLI-only operator actions.

### Structural comparison

`sley graph-diff` produces a three-way semantic report for human review. It
classifies structural overlap and keeps merge authority explicit.

### Persistent worker clients

The prerelease Python and Node reference clients under `clients/` implement
the stable `sley.worker.v1` JSONL stdio protocol. Their typed models and
protocol constants are generated from the canonical worker schemas. Run
`make worker-clients` to build both local packages in clean offline
environments and exercise handshake, invoke, cancellation, crash recovery,
health, and shutdown. Neither client retries automatically, and neither
package is published by this repository workflow.

## Public Evidence

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

## License

Apache-2.0. See [LICENSE](LICENSE) and [NOTICE](NOTICE).

Autonomy, Engineered.
