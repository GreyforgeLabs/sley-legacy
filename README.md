    # Sley CI

    Reusable local, pre-commit, and GitHub Actions gates for Sley projects.

    Status: private Sley ecosystem scaffold. This repository is intentionally
    built as a detailed starting point before public release.

    ## Why This Exists

    Sley is an agent-native structural language. Source remains the human review
    projection, while the compiler exposes stable JSON surfaces for graph,
    lint, query, edit planning, verification, trace receipts, and ZJX handoff.

    `sley-ci` exists to make that loop easier for repo maintainers and application developers.

    ## Current Scope

    - Priority: `P0`
    - Utility class: `CI and pre-commit gate pack`
    - Default mode: local and deterministic
    - Write mode: disabled unless explicitly documented by the command
    - Network calls: none in tests or examples
    - Provider, deploy, spend, wallet, and secret access: not used

    ## Quick Start

    ```bash
    make smoke
    ```

    Useful commands:

    - `sley-ci check .`
- `sley-ci verify --deny-warnings .`
- `sley-ci smoke fixtures/cli_smokes/manifest.json`

    ## Consumed Sley Contracts

    This tool treats Loom, the Sley compiler, as the oracle. It consumes these
    Sley surfaces instead of duplicating compiler logic:

    - `sley.diagnostics.report.v0`
- `sley.lint.report.v0`
- `sley.verify.report.v0`
- `sley.trace.seal.v0`
- `sley.zjx.envelope.v0`
- `sley.cli_smoke.manifest.v0`

    Details live in [`docs/contracts.md`](docs/contracts.md).

    ## Repository Layout

    - `assets/branding/` - repo mark, social card, banner, and generated PNGs
    - `docs/` - architecture, contract, brand, and SEO notes
    - `examples/` - minimal Sley fixtures for local smoke work
    - `test/` - smoke tests that avoid network and external systems
    - `Makefile` - `fmt`, `test`, and `smoke` entry points

    Includes `bin/sley-ci`, `action.yml`, a pre-commit hook definition, and reproducible local smoke commands.

    ## Release Policy

    This repository stays private until:

    - consumed Sley schema versions are declared;
    - deterministic local tests pass;
    - examples work against the current Sley compiler;
    - public-use branding is reviewed;
    - docs avoid private local paths;
    - write paths, if any, preview through `sley fix --dry-run` or
      `sley graft --dry-run` before mutation.

    ## SEO Surface

    Draft SEO title: `Sley CI - Sley developer tooling`

    Draft description: Run one obvious Sley quality gate for format, check, lint, verify, seal, ZJX handoff, and manifest-backed CLI smoke checks.

    Future canonical URL: `https://sley.greyforge.tech/tools/sley-ci`

    GitHub URL while private: `https://github.com/GreyforgeLabs/sley-ci`

    ## License

    MIT. See [`LICENSE`](LICENSE).

    Autonomy, Engineered.
