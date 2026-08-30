    # Sley Workbench

    Local graph, lint, plan, trace, seal, and ZJX inspection workbench for Sley projects.

Status: public Sley ecosystem scaffold. This repository is intended for public use with a stable, versioned contract surface.

Implementation reality: Sley-native source-of-truth is now in `src/tool.sley`; current Node + browser host is temporary until a Sley emit target is ready.

    ## Why This Exists

    Sley is an agent-native structural language. Source remains the human review
    projection, while the compiler exposes stable JSON surfaces for graph,
    lint, query, edit planning, verification, trace receipts, and ZJX handoff.

    `sley-workbench` exists to make that loop easier for compiler contributors and agent tool builders.

    ## Current Scope

    - Priority: `P0 scaffold`
    - Utility class: `graph and graft workbench`
    - Default mode: local and deterministic
    - Write mode: disabled unless explicitly documented by the command
    - Network calls: none in tests or examples
    - Provider, deploy, spend, wallet, and secret access: not used

    ## Quick Start

    ```bash
    make smoke
    ```

    Useful commands:

    - `sley-workbench --target ../sley/examples/project`
- `sley doctor --json <target>`
- `sley graph --json --slice <node-id> <target>`

    The server binds to `127.0.0.1` and starts in read-only mode. Browser
    command execution uses JSON `POST`, an exact Origin check, and a
    per-process CSRF token. Request targets must be relative paths that remain
    inside the configured workspace after symlink resolution.

    Use `--allow-mutation` only when the local `format` command should be
    enabled. A non-loopback `--host` additionally requires both
    `--allow-remote` and an explicit `--origin`; those flags do not weaken the
    Origin, CSRF, target-containment, timeout, or output limits.

    ## Consumed Sley Contracts

    This tool treats Loom, the Sley compiler, as the oracle. It consumes these
    Sley surfaces instead of duplicating compiler logic:

    - `sley.ast.program.v0`
- `sley.symbol_graph.v0`
- `sley.symbol_graph.slice.v0`
- `sley.query.report.v0`
- `sley.lint.report.v0`
- `sley.doctor.report.v0`
- `sley.edit_plan.report.v0`
- `sley.verify.report.v0`
- `sley.trace.seal.v0`
- `sley.zjx.envelope.v0`

    Details live in [`docs/contracts.md`](docs/contracts.md).

    ## Repository Layout

    - `assets/branding/` - repo mark, social card, banner, and generated PNGs
    - `docs/` - architecture, contract, brand, and SEO notes
    - `examples/` - minimal Sley fixtures for local smoke work
    - `test/` - smoke tests that avoid network and external systems
    - `Makefile` - `fmt`, `test`, and `smoke` entry points

    Includes a local-only HTTP server and static UI scaffold with write mode disabled by default.

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

    SEO title: `Sley Workbench - agent-native inspection surface`

    SEO description: Inspect agent-oriented readiness, graph slices, query tables, lint findings, checked repair plans, traces, seals, and ZJX envelopes from a local workbench.

    Keywords: `Sley workbench`, `graph slice inspection`, `lint findings`, `repair plans`, `agent-native debugging`, `ZJX envelopes`

    Canonical URL: `https://sleylang.org/tools/sley-workbench`
    - Geo metadata:
      - Region: United States (US)
      - Language: English
      - Audience: agent-native language tooling teams and operators

    GitHub URL: `https://github.com/GreyforgeLabs/sley-workbench`

    ## License

    Apache-2.0. See [`LICENSE`](LICENSE).

    Autonomy, Engineered.
