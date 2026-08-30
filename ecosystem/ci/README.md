    # Sley CI

    Reusable local, pre-commit, and GitHub Actions gates for Sley projects.

Status: public Sley ecosystem scaffold. This repository is intended for public use with a stable, versioned contract surface.

Implementation reality: Sley-native source-of-truth is now in `src/tool.sley`; current CLI/Action wrapper is temporary until a Sley emit target is ready.

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

    Machine mode (`sley-ci --json ...`) always emits one final
    `sley.ci.report.v1` document, including when a step fails or a required
    executable is unavailable. Missing Sley or jq dependencies fail closed.
    Non-gating diagnostics may opt into a recorded skip with
    `--allow-missing`; CI and the GitHub Action never enable it by default.

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

    This repository is public once:

    - consumed Sley schema versions are declared;
    - deterministic local tests pass;
    - examples work against the current Sley compiler;
    - public-use branding is reviewed;
    - docs avoid private local paths;
    - write paths, if any, preview through `sley fix --dry-run` or
      `sley graft --dry-run` before mutation.

## SEO Surface

    SEO title: `Sley CI - agent-native verification gates`

    SEO description: Run agent-native workflow checks for format, lint, verify, seal, schema conformance, and manifest-backed CLI smoke checks.

    Keywords: `Sley CI`, `agent-native verification`, `compiler checks`, `schema conformance`, `token-efficient lint`, `release gating`, `CLI smoke tests`

    Canonical URL: `https://sleylang.org/tools/sley-ci`
    - Geo metadata:
      - Region: United States (US)
      - Language: English
      - Audience: agent-native language tooling teams and operators

    GitHub URL: `https://github.com/GreyforgeLabs/sley-ci`

    ## License

    Apache-2.0. See [`LICENSE`](LICENSE).

    Autonomy, Engineered.
