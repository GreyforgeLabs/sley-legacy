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
  <img alt="Sley v1 Gate" src="https://img.shields.io/badge/Sley%20v1%20Gate-23%2F23-22c55e" />
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

## Current Proof

The 2026-08-22 checkpoint passed the complete local v1 gate:

| Evidence | Verified count |
| --- | ---: |
| Report schemas | 42 |
| Contract fixtures | 128 |
| Accepted corpus cases | 23 |
| Rejected corpus cases | 43 |
| Integration checks | 199 |
| Local v1 gate checks | 23 / 23 |

The self-hosted Sley compiler owns its parser, checker, lint, runtime,
bootstrap, and report semantics in Sley source. The public command layer turns
that language-owned implementation into a practical local workflow and
verification surface.

## Start Locally

Prerequisites: Bash, `jq`, and Python 3 with `jsonschema`.

```bash
export PATH="$(pwd)/bin:$PATH"
make v1
sley doctor --json examples/project
sley ast --json examples/hello.sley
sley query --json --kind calls examples/project
sley lint --json examples/empty_for_statement.sley
sley run --json examples/hello.sley
sley verify --json examples/project
```

## Structural Workflow

The public command surface covers four connected lanes:

| Lane | Representative surfaces |
| --- | --- |
| Inspect | `ast`, `graph`, `query`, `lint`, `doctor` |
| Plan | `plan`, `fix --dry-run`, `graft --dry-run`, `graph-diff` |
| Verify | `check`, `verify`, `trace`, `seal`, contract and conformance reports |
| Operate | `run`, staged-change guard, read-only tool bridge, local workbench |

Use `sley --help` for the complete command index.

## Developer Integrations

### Staged-change guard

`git-sley-guard` checks affected staged Sley targets and contract changes from
the Git index before a commit enters project history.

### Read-only tool bridge

`sley-mcp-bridge` provides bounded query, lint, planning, dry-run graft, and
verification calls for development tools operating inside an explicitly
configured repository root.

### Structural comparison

`sley graph-diff` produces a three-way semantic report for human review. It
classifies structural overlap and keeps merge authority explicit.

## Public Evidence

- [Sley technical brief](https://sleylang.org/docs)
- [Workflow walkthrough](https://sleylang.org/tutorial)
- [Claim evidence](docs/SleyClaimEvidence.md)
- [Language specification](docs/SleyLanguageSpec.md)
- [Contract reference](docs/contracts.md)
- [Read-only tool bridge](docs/SleyMcpBridge.md)
- [Structural comparison research](docs/SleyGraphDiffResearch.md)

## Repository Map

- `self-hosted/src/loom/` contains the self-hosted Sley compiler.
- `bin/` contains the local command and developer integration surfaces.
- `docs/schemas/` contains machine-checkable report contracts.
- `fixtures/contracts/` contains verified contract fixtures.
- `fixtures/corpus/` contains accepted and rejected language cases.
- `scripts/` contains the deterministic local proof gates.
- `examples/` contains projects and workflow demonstrations.

## License

Apache-2.0. See [LICENSE](LICENSE) and [NOTICE](NOTICE).

Autonomy, Engineered.
