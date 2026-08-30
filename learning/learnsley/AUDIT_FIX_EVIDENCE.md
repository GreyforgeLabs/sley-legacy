# Audit Fix Evidence

Remediations: GF-AUD2-018, GF-AUD2-027

Version: 0.2.0

## Security Boundary

LearnSley remains a public-source, loopback-only compiler-backed local service.
It is not safe to deploy as a public hosted compiler service without a separate
sandbox design.

`package.json` keeps `"private": true` only to prevent accidental npm package
publication. It is not a GitHub visibility control.

## GF-AUD2-018

- Host validation accepts only loopback names: `127.0.0.1`, `localhost`, and
  `::1`.
- Mutating endpoints require same-origin `Origin`, `application/json`, and a
  per-process CSRF token sent as `x-learnsley-csrf`.
- GET endpoints remain read-only.
- The bundled UI fetches `/api/session` and attaches the token to mutations.
- `/api/health` does not expose compiler paths.

## GF-AUD2-027

- Compiler runs use per-request temporary workspaces and cleanup in `finally`.
- Stale crash workspaces are pruned by bounded age, count, and byte limits.
- Compiler subprocesses run in a POSIX process group and are terminated on
  timeout, cancellation, or output flood.
- Cleanup failures are logged with path-redacted error details and are not
  returned to remote callers.

## Validation

Run locally:

```bash
npm test
npm run check
git diff --check
```
