# Migrating from Sley 1.1 to 1.2

Sley 1.2 preserves the 1.1 command and source behavior covered by the corpus,
examples, compatibility fixtures, and authoritative release gate. No automatic
source rewrite is required for accepted 1.1 programs.

## Required operator changes

1. Use version `1.2.0` for the CLI and Python/Node worker clients.
2. Run `sley doctor --toolchain --json` after installing or extracting the
   supported Linux archive.
3. Keep existing read-only commands. For governed writes, adopt the explicit
   `sley change` inspect, plan, preview, approval, authorization, apply, verify,
   review, seal, recovery, and rollback lifecycle.
4. Update worker integrations to the stable `sley.worker.v1` request, session,
   event, and response contracts. Do not retry consequential requests without
   an idempotency design.
5. Use manifest-backed `sley test` and the quick, changed, core, or release
   validation profile appropriate to the boundary.
6. Treat unsupported executable `Raw`, foreign Boolean spelling, expression
   control flow, module control flow, and inline task parameters as actionable
   compiler errors. Use `sley explain --diagnostic-id <ID>` for the checked
   spelling.

## Compatibility boundary

Existing v0 report producers remain available where 1.2 registered stable v1
minimum compatibility roots rather than cutting over the producer. W4
transaction and local-adapter contracts remain strict extensible v0 surfaces;
worker, testing, validation, operational replay, and W7 release roots carry the
explicit stable v1 commitments recorded in `CONTRACT_STABILITY.json`.

External adapters, HTTP tenancy, broad translation, package-manager channels,
and production promotion are not migration prerequisites and are not granted
by installing 1.2.
