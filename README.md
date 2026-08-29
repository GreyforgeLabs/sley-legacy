    # Sley Conformance

    Static coverage and release-readiness reports for Sley schemas, fixtures, contracts, and CLI smokes.

Status: public Sley ecosystem scaffold. This repository is intended for public use with a stable, versioned contract surface.

Implementation reality: Sley-native source-of-truth is now in `src/tool.sley`; current JS report engine is temporary until a Sley emit target is ready.

    ## Why This Exists

    Sley is an agent-native structural language. Source remains the human review
    projection, while the compiler exposes stable JSON surfaces for graph,
    lint, query, edit planning, verification, trace receipts, and ZJX handoff.

    `sley-conformance` exists to make that loop easier for compiler maintainers.

    ## Current Scope

    - Priority: `P0`
    - Utility class: `conformance dashboard`
    - Default mode: local and deterministic
    - Write mode: disabled unless explicitly documented by the command
    - Network calls: none in tests or examples
    - Provider, deploy, spend, wallet, and secret access: not used

    ## Quick Start

    ```bash
    make smoke
    ```

    Useful commands:

    - `sley-conformance report --root ../sley --json`
- `sley-conformance report --root ../sley --markdown`
- `sley-conformance coverage --root ../sley --require-tag cli:verify`

    Mandatory corpus, smoke, schema, and contract evidence must be non-empty,
    parse successfully, and contain unique identifiers. Reports expose
    discovered, parsed, malformed, duplicate, and validated counts. Any
    malformed mandatory artifact makes `report`, `coverage`, and `guard` fail.

    ## Consumed Sley Contracts

    This tool treats Loom, the Sley compiler, as the oracle. It consumes these
    Sley surfaces instead of duplicating compiler logic:

    - `sley.cli_smoke.manifest.v0`
- `sley.ast.program.v0`
- `sley.diagnostics.report.v0`
- `sley.query.report.v0`
- `sley.lint.report.v0`
- `sley.verify.report.v0`
- `sley.zjx.envelope.v0`

    Details live in [`docs/contracts.md`](docs/contracts.md).

    ## Repository Layout

    - `assets/branding/` - repo mark, social card, banner, and generated PNGs
    - `docs/` - architecture, contract, brand, and SEO notes
    - `examples/` - minimal Sley fixtures for local smoke work
    - `test/` - smoke tests that avoid network and external systems
    - `Makefile` - `fmt`, `test`, and `smoke` entry points

    Includes a report CLI that reads manifests, schema files, contract snapshots, and coverage tags.

    ## Release Policy

    This repository is public once:

    - consumed Sley schema versions are declared;
    - deterministic local tests pass;
    - examples work against the current Sley compiler;
    - public-use branding is reviewed;
    - docs avoid private local paths;
    - write paths, if any, preview through `sley fix --dry-run` or
      `sley graft --dry-run` before mutation.

    ## SEO Surface

    SEO title: `Sley Conformance - agent-native report coverage`

    SEO description: Generate conformance reports for Sley coverage, schema-to-contract mappings, smoke tags, and release-readiness gaps.

    Keywords: `Sley conformance`, `contract coverage`, `agent-native validation`, `schema mapping`, `release readiness`, `smoke reports`

    Canonical URL: `https://sleylang.org/tools/sley-conformance`
    - Geo metadata:
      - Region: United States (US)
      - Language: English
      - Audience: agent-native language tooling teams and operators

    GitHub URL: `https://github.com/GreyforgeLabs/sley-conformance`

    ## License

    Apache-2.0. See [`LICENSE`](LICENSE).

    Autonomy, Engineered.
