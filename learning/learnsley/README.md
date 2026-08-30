# LearnSley

LearnSley is the local Linux-native MVP for the Sley learning game described in
`LearnSley.md`.

It is a web app UX backed by a localhost Node server. The server runs the real
Sley compiler binary for `Run`, `Seal`, `Format`, `Graph`, and `Seal Digest`
actions. It does not download packages and does not expose a public listener by
default.

## Run

```bash
cd /path/to/learnsley
npm start
```

Open:

```text
http://127.0.0.1:4179
```

Optional environment:

```bash
SLEY_BIN=<path-to-sley-compiler> PORT=4179 npm start
```

## Current Slice

- Five starter trials from the spec.
- Real Sley `check`, `run`, `format`, `graph`, and `seal` subprocess calls.
- Local JSON progress in `workspaces/progress.json`.
- Clean Seal Chain, Proof Armor, XP, shards, level, and Daily Pulse.
- Loopback-only bind and Host validation for `127.0.0.1`, `localhost`, and `::1`.
- Same-origin `Origin`, per-process CSRF token, and `application/json` checks on every mutation.
- Per-request temporary compiler workspaces with cleanup and bounded stale-crash pruning.
- POSIX compiler process-group termination on timeout, cancellation, and output flood.

`package.json` keeps `"private": true` to prevent accidental npm publication.
That is not a GitHub visibility control; this repository may still be public.

## Security Boundary

This service is safe to publish as source code, but it is not a public hosted
compiler service. The default server accepts only loopback hosts, rejects
non-loopback binds, and treats every compile/progress endpoint as a local-only
mutation guarded by same-origin `Origin` and a per-process CSRF token.

Health responses intentionally report only whether a compiler is configured.
They do not disclose local compiler paths.

## Hosting Decision

`learnsley.example.invalid` can represent a future public product target, but
the current compiler-backed grader is intentionally local. A hosted version
needs either:

1. a static demo shell with no arbitrary code execution;
2. a WASM Sley compiler path; or
3. a locked server sandbox for submitted programs.

The local repo is the source of truth until one of those hosting paths is
implemented and approved.
