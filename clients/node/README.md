# Sley worker client for Node

This prerelease, dependency-free reference package implements the canonical
`sley.worker.v1` JSONL stdio transport. Runtime constants and TypeScript
request, response, event, and session models are generated from the four
stable worker schemas by `clients/generate-worker-clients.py`.

The client preserves stream ordering and exposes every stable lifecycle
operation. Cancellation remains a separate control request targeting an active
request ID, and the final invoke response remains authoritative. The client
does not retry automatically or reinterpret worker failure classes.

S12-503 validates a packed local package inside a fresh, offline npm project.
The package is not published to a registry by this work package.
