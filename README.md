    # Tree-sitter Sley

    Tree-sitter grammar, highlight queries, and editor parser scaffolding for Sley source.

Status: public Sley ecosystem scaffold. This repository is intended for public use with a stable, versioned contract surface.

Implementation reality: Sley-native source-of-truth is now in `src/tool.sley`; Tree-sitter grammar and queries remain temporary until a Sley parser emitter exists.

    ## Why This Exists

    Sley is an agent-native structural language. Source remains the human review
    projection, while the compiler exposes stable JSON surfaces for graph,
    lint, query, edit planning, verification, trace receipts, and ZJX handoff.

    `tree-sitter-sley` exists to make that loop easier for editor users, documentation authors, and review tool builders.

    ## Current Scope

    - Priority: `P0`
    - Utility class: `syntax grammar`
    - Default mode: local and deterministic
    - Write mode: disabled unless explicitly documented by the command
    - Network calls: none in tests or examples
    - Provider, deploy, spend, wallet, and secret access: not used

    ## Quick Start

    ```bash
    make smoke
    ```

    Useful commands:

    - `tree-sitter generate`
- `tree-sitter test`
- `npm run smoke`

    ## Consumed Sley Contracts

    This tool treats Loom, the Sley compiler, as the oracle. It consumes these
    Sley surfaces instead of duplicating compiler logic:

    - `human-reviewable .sley source`

    Details live in [`docs/contracts.md`](docs/contracts.md).

    ## Repository Layout

    - `assets/branding/` - repo mark, social card, banner, and generated PNGs
    - `docs/` - architecture, contract, brand, and SEO notes
    - `examples/` - minimal Sley fixtures for local smoke work
    - `test/` - smoke tests that avoid network and external systems
    - `Makefile` - `fmt`, `test`, and `smoke` entry points

    Includes `grammar.js`, highlight queries, corpus fixtures, and smoke checks that can run before Tree-sitter is installed.

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

    SEO title: `Tree-sitter Sley - agent-native parser surface`

    SEO description: Parse and highlight Sley source with Tree-sitter so agent-oriented editors can work with modules, tasks, authority, and checked-call structures.

    Keywords: `Tree-sitter Sley`, `Sley parser`, `syntax highlighting`, `editor integration`, `query grammar`, `agent-native source`

    Canonical URL: `https://sleylang.org/tools/tree-sitter-sley`
    - Geo metadata:
      - Region: United States (US)
      - Language: English
      - Audience: agent-native language tooling teams and operators

    GitHub URL: `https://github.com/GreyforgeLabs/tree-sitter-sley`

    ## License

    Apache-2.0. See [`LICENSE`](LICENSE).

    Autonomy, Engineered.
