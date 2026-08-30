    # Sley Contract Kit

    Schema inventory, report validation, and contract drift checks for Sley tool authors.

Status: public Sley ecosystem scaffold. This repository is intended for public use with a stable, versioned contract surface.

Implementation reality: Sley-native source-of-truth is now in `src/tool.sley`; current JS host wrapper is temporary until a Sley emit target is ready.

    ## Why This Exists

    Sley is an agent-native structural language. Source remains the human review
    projection, while the compiler exposes stable JSON surfaces for graph,
    lint, query, edit planning, verification, trace receipts, and ZJX handoff.

    `sley-contract-kit` exists to make that loop easier for tool authors and compiler contributors.

    ## Current Scope

    - Priority: `P0`
    - Utility class: `contract validation kit`
    - Default mode: local and deterministic
    - Write mode: disabled unless explicitly documented by the command
    - Network calls: none in tests or examples
    - Provider, deploy, spend, wallet, and secret access: not used

    ## Quick Start

    ```bash
    make smoke
    ```

    Useful commands:

    - `sley-contract inventory ../sley/docs/schemas`
- `sley-contract validate --schema sley.verify.report.v0 report.json`
- `sley-contract check-fixtures ../sley/fixtures/contracts`

    Contract checks require non-empty evidence by default. The narrowly scoped
    `--allow-empty` option is available only for bootstrap inventory, fixture,
    and comparison work. Inputs default to 1 MiB per file, 2,048 files, and 64
    JSON nesting levels; bounded overrides use `--max-bytes`, `--max-files`,
    and `--max-depth`.

    Exit codes are stable: `0` means the requested evidence passed, `1` means
    valid evidence showed contract drift, `2` means usage or structured input
    was invalid, and `3` means an I/O or unexpected runtime failure. Add
    `--json` for a single valid JSON result or error object without stack traces.

    ## Consumed Sley Contracts

    This tool treats Loom, the Sley compiler, as the oracle. It consumes these
    Sley surfaces instead of duplicating compiler logic:

    - `sley.ast.program.v0`
- `sley.diagnostics.report.v0`
- `sley.symbol_graph.v0`
- `sley.symbol_graph.slice.v0`
- `sley.query.report.v0`
- `sley.lint.report.v0`
- `sley.doctor.report.v0`
- `sley.edit_plan.report.v0`
- `sley.verify.report.v0`
- `sley.project.scaffold.v0`
- `sley.graft.outcome.v0`
- `sley.trace.receipt.v0`
- `sley.trace.seal.v0`
- `sley.zjx.envelope.v0`

    Details live in [`docs/contracts.md`](docs/contracts.md).

    ## Repository Layout

    - `assets/branding/` - repo mark, social card, banner, and generated PNGs
    - `docs/` - architecture, contract, brand, and SEO notes
    - `examples/` - minimal Sley fixtures for local smoke work
    - `test/` - smoke tests that avoid network and external systems
    - `Makefile` - `fmt`, `test`, and `smoke` entry points

    Embeds a dependency-light CLI scaffold that validates schema root identity and fixture coverage.

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

    SEO title: `Sley Contract Kit - agent-native contract validation`

    SEO description: Validate Sley JSON contracts, inspect schema versions, and detect report drift before toolchains depend on unstable contract shapes.

    Keywords: `Sley Contract Kit`, `contract validation`, `schema versioning`, `JSON contracts`, `report drift`, `toolchain safety`

    Canonical URL: `https://sleylang.org/tools/sley-contract-kit`
    - Geo metadata:
      - Region: United States (US)
      - Language: English
      - Audience: agent-native language tooling teams and operators

    GitHub URL: `https://github.com/GreyforgeLabs/sley-contract-kit`

    ## License

    Apache-2.0. See [`LICENSE`](LICENSE).

    Autonomy, Engineered.
