    # Contracts

    `sley-ci` is contract-first. It should consume Sley compiler JSON roots
    and source fixtures, never a private parser fork.

    | Contract | Status | Notes |
    |---|---|---|
    | `sley.diagnostics.report.v0` | consumed | current scaffold input |
| `sley.lint.report.v0` | consumed | current scaffold input |
| `sley.verify.report.v0` | consumed | current scaffold input |
| `sley.trace.seal.v0` | consumed | current scaffold input |
| `sley.zjx.envelope.v0` | consumed | current scaffold input |
| `sley.cli_smoke.manifest.v0` | consumed | current scaffold input |

    ## Rules

    - Treat `sley --json` output as the source of truth.
    - Pin every consumed schema root in release notes.
    - Keep fixture updates in the same change as schema drift.
    - Prefer warning reports and dry-run repairs over implicit writes.
    - Keep tests deterministic and local.
