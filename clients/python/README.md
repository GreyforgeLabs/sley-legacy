# Sley worker client for Python

This prerelease reference package implements the canonical
`sley.worker.v1` JSONL stdio transport. Its typed models are generated from the
four stable worker schemas by `clients/generate-worker-clients.py`.

The client starts an explicitly selected `sley worker start --json` command,
preserves event and response ordering, exposes every stable lifecycle
operation, and returns worker failure classes without reinterpreting them.
It never retries automatically. Every request receives a local deadline;
`PendingResponse.cancel()` cancels a local wait. Timed-out and cancelled waits
are removed immediately, with bounded tombstones that discard late responses.
Use the separate `cancel` control request to cancel work in the worker itself.
Malformed framing and unexpected worker exit reject all pending work and reap
the worker process group.

The package has no runtime dependencies. S12-503 validates a locally built
wheel inside a fresh virtual environment without network access. Public
package-registry publication is outside this work package.
