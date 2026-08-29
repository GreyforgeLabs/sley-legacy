# Sley worker client for Node

This prerelease, dependency-free reference package implements the canonical
`sley.worker.v1` JSONL stdio transport. Runtime constants and TypeScript
request, response, event, and session models are generated from the four
stable worker schemas by `clients/generate-worker-clients.py`.

The client preserves stream ordering and exposes every stable lifecycle
operation. Every local request wait has a deadline and accepts an optional
`AbortSignal`. A locally timed-out or aborted wait is removed immediately; a
bounded tombstone set discards its late response without reviving the promise.
Canonical worker cancellation remains a separate control request targeting an
active request ID. The client does not retry automatically or reinterpret
worker failure classes. Framing corruption and unexpected worker exit are
fatal: all pending work is rejected and the worker process group is reaped.

S12-503 validates a packed local package inside a fresh, offline npm project.
The package is not published to a registry by this work package.
