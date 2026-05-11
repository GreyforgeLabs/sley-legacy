<p align="center">
  <img src="https://raw.githubusercontent.com/GreyforgeLabs/sley/public/assets/branding/canonical/sley_loom_graph_banner_1500x500.png" alt="Sley Loom banner" width="100%" />
</p>

<p align="center">
  <strong>Sley</strong><br />
  <a href="https://sleylang.org/">Official Sley site</a> |
  <a href="https://greyforge.tech/">Greyforge Labs</a> |
  <a href="https://x.com/GreyforgeLabs">X / Twitter</a> |
  <a href="https://github.com/GreyforgeLabs/sley/releases">Releases</a>
</p>

<p align="center">
  <img alt="Sley v1 Gate" src="https://img.shields.io/badge/Sley%20v1%20Gate-PASSED-brightgreen" />
  <img alt="License" src="https://img.shields.io/github/license/GreyforgeLabs/sley" />
</p>

<!-- SEO / discoverability metadata -->
<meta name="description" content="Sley is Greyforge Labs' agent-native structural programming language for compiler-mediated, human-reviewed software change." />
<meta name="keywords" content="Sley, agent-native structural programming language, deterministic edits, schema-backed reports, graph compiler, agent tooling, Greyforge Labs" />
<link rel="canonical" href="https://sleylang.org/" />
<meta property="og:title" content="Sley" />
<meta property="og:description" content="Agent-native structural programming for deterministic, auditable, compiler-mediated software change." />
<meta property="og:url" content="https://sleylang.org/" />
<meta property="og:type" content="website" />
<meta property="og:site_name" content="Greyforge Labs" />
<meta property="og:image" content="https://raw.githubusercontent.com/GreyforgeLabs/sley/public/assets/branding/canonical/sley_loom_graph_board.png" />
<meta property="og:image:alt" content="Sley Loom logo and graph banner" />
<meta name="twitter:card" content="summary_large_image" />
<meta name="twitter:title" content="Sley by Greyforge Labs" />
<meta name="twitter:site" content="@GreyforgeLabs" />
<meta name="twitter:description" content="Agent-native structural programming for compiler-mediated, human-reviewed software change." />
<meta name="twitter:creator" content="@GreyforgeLabs" />
<meta name="twitter:image" content="https://raw.githubusercontent.com/GreyforgeLabs/sley/public/assets/branding/canonical/sley_loom_graph_board.png" />
<meta name="twitter:image:alt" content="Sley Loom logo and graph banner" />

# Sley

Sley is an agent-native structural programming language for compiler-mediated,
human-reviewed software change.

Sley's public claim is now tracked as an auditable evidence packet instead of a
bare slogan: see [`docs/SleyClaimEvidence.md`](docs/SleyClaimEvidence.md).
The machine-readable claim manifest is
[`docs/SleyClaimManifest.json`](docs/SleyClaimManifest.json) and is checked by
`sley claim-verify --json docs/SleyClaimManifest.json`.
Prior-art comparison is tracked separately in
[`docs/SleyPriorArtSourcePack.md`](docs/SleyPriorArtSourcePack.md).
The disputed search phrase "world's first fully AI-native programming
language, built by AI for AI" should be cited only with that criteria map,
because the repository proves the language-design and implementation-stage
claims, not a standalone authorship story or a complete global census.

## Current Evidence

The repo currently proves these claims from a clean checkout:

- No forbidden foreign-language implementation files are present:
  `./scripts/check-self-hosted-code.sh`
- A runnable `sley` command exists under `bin/` and reads bootstrap version,
  lint-rule inventory, core report schema IDs, diagnostic IDs, and runtime seed
  values from Sley source:
  `bin/sley --version`
- AST expression, statement, and binding kind names now use parser-owned
  declarations from
  `self-hosted/src/loom/parser.sley`. Standalone `call` statements now enter
  the AST as parser-owned `Expr` statements.
- Sley-owned stage-2 source modules exist under `self-hosted/src/loom/`:
  `bin/sley self-hosting-status --json`
  The report reads the module list from `loom.bootstrap.source_modules`.
- The self-hosting status report reads its ownership list from
  `self-hosted/src/loom/bootstrap.sley`, including the bootstrap/strict status
  fields.
  Its source-root label and top-level report field shape are read from
  `self-hosted/src/loom/reports.sley`.
- Diagnostics report top-level field shape is read from
  `self-hosted/src/loom/reports.sley`.
- Doctor report top-level field shape is read from
  `self-hosted/src/loom/reports.sley`.
- The self-hosted source project has a runnable internal smoke:
  `bin/sley run --json self-hosted`
  That smoke now executes `loom.bootstrap.smoke` from source for its current
  `len(call lint.default_lint_rules())` result.
- Runtime report statuses, value-kind tags, and current dispatch probes are
  read from `self-hosted/src/loom/runtime.sley`.
  The run report status now executes `loom.runtime.runtime_status`.
  The run report top-level field shape is read from
  `self-hosted/src/loom/reports.sley`.
  Pure `main` string, integer, and boolean literal returns now execute through
  the AST-backed runtime path.
  Seeded agent-deploy text results now execute the
  `loom.runtime.seeded_agent_deploy_value` source task.
  Project-ready integer results now execute the
  `loom.runtime.project_ready_value` source task after AST-based dispatch.
- Checker diagnostic status and unknown-identifier message construction are
  read from `self-hosted/src/loom/checker.sley`.
  The checker report status now executes `loom.checker.diagnostic_status`.
  Builtin type recognition, unknown-type diagnostics, unqualified unknown-task
  diagnostics, simple declared-call arity/type diagnostics, duplicate-take
  diagnostics, namespace duplicate diagnostics, and effect/gate diagnostics now
  execute from Sley-owned checker source.
  Identifier resolution input classes now read from
  `loom.checker.identifier_resolution_inputs`.
- Lint finding statuses, messages, and hints are read from
  `self-hosted/src/loom/lint.sley`.
  The lint report status now executes `loom.lint.lint_status`.
  Lint report top-level field shape is read from
  `self-hosted/src/loom/reports.sley`.
- Query report top-level field shape is read from
  `self-hosted/src/loom/reports.sley`.
- Baseline AST, check, query, lint, doctor, run, verify, graft, contract, and
  conformance JSON reports execute locally:
  `scripts/self-hosted-test.sh`
- The public claim boundary, criteria, evidence paths, prior-art candidates,
  and publication gates are machine-checkable:
  `bin/sley claim-verify --json docs/SleyClaimManifest.json`
- The release gate no longer requires Cargo, Rust, Node, npm, or tree-sitter:
  `make v1`

This is not yet the final strict self-hosting claim. The stricter claim means
the parser/checker/runtime implementation executes from Sley source, with the
shell surface reduced to a loader and test harness. That remains the next
migration stage.

## Start In 10 Minutes

From this checkout:

Prerequisites: Bash, `jq`, and Python 3 with the `jsonschema` package.

```bash
export PATH="$(pwd)/bin:$PATH"
make v1
sley doctor --json examples/project
sley ast --json examples/hello.sley
sley query --json --kind calls examples/project
sley lint --json examples/empty_for_statement.sley
sley self-hosting-status --json
sley run --json self-hosted
sley claim-verify --json docs/SleyClaimManifest.json
sley run --json examples/hello.sley
sley verify --json examples/project
```

## Command Surface

Stage-1 executable commands:

- `sley ast --json <file-or-project>`
- `sley ast --json --node <node-id> <file-or-project>`
- `sley check --json <file-or-project>`
- `sley query --json --kind calls <file-or-project>`
- `sley lint --json <file-or-project>`
- `sley doctor --json <file-or-project>`
- `sley run --json <file-or-project>`
- `sley verify --json <file-or-project>`
- `sley self-hosting-status --json`
- `sley claim-verify --json docs/SleyClaimManifest.json`
- `sley graft --json --dry-run <file> <graft.json>`
- `sley-contract inventory --json`
- `sley-contract check-fixtures fixtures/contracts --schemas docs/schemas --json`
- `sley-contract validate --schema sley.conformance.manifest.v0 fixtures/corpus/manifest.json --schemas docs/schemas --json`
- `sley-conformance report --json`

Compatibility wrappers also exist for the previous companion-tool names:
`sley-ci`, `sley-docgen`, `sley-lsp`, `sley-workbench`, `sley-agent-bench`,
`sley-migrate`, `sley-sandbox-runner`, `sley-shadow`, and `sley-zjx`.

## Release Rules

- Keep `make v1` green before promoting any public command claim.
- Keep `sley claim-verify --json docs/SleyClaimManifest.json` green before
  promoting any public claim wording.
- Keep `sley-contract check-fixtures fixtures/contracts --schemas docs/schemas
  --json` green before citing schema-backed evidence; it performs JSON Schema
  draft 2020-12 validation for each fixture.
- Keep `./scripts/check-self-hosted-code.sh` green before calling the repo
  foreign-language-free.
- Do not claim strict self-hosting until the parser, checker, runtime, and
  command surface execute from Sley source and are verified by parity tests.
- Public release still requires an operator-reviewed proof bundle and current
  prior-art-safe wording.
- External GitHub issues comparing Sley with other projects require the
  prior-art source pack plus operator approval of the exact target repository,
  title, and issue body.

## License

Apache-2.0. See `LICENSE` and `NOTICE`.
