# GitHub audit remediation evidence

Status: implementation evidence for `GF-AUD-002` and `GF-AUD-039`.

## Starting state

- Branch: `main`
- Commit: `af467c5a1a9cf13f43780da1c6b62f391f541bd0`
- Version: `0.0.0-private`
- Baseline: stdio was decoded into a JavaScript string and sliced by character
  count despite byte-valued `Content-Length`; compiler calls had no timeout or
  output ceiling; temporary cleanup was not in `finally`; help registered the
  server before printing; close and unsupported-method semantics were absent.

## Remediation

- The receive buffer remains a `Buffer` through header parsing and byte slicing.
- ASCII headers, unique decimal length, 8 KiB headers, 4 MiB bodies, strict
  UTF-8, and deterministic malformed-frame behavior are enforced.
- Compiler subprocesses use fixed timeout, output, and kill bounds; temporary
  work is removed in `finally`.
- LSP file reads stay inside initialized workspace roots, file URIs use
  `fileURLToPath`, byte offsets become UTF-16 positions, close clears
  diagnostics, and unknown requests return `-32601`.
- Version advanced to `0.0.1-private`.

## Regression coverage

`npm test` covers emoji/CJK/combining/astral text, one-byte and randomized
chunks, multiple frames, malformed/duplicate/negative/missing/oversized length,
malformed JSON recovery, UTF-16 positions, workspace escapes, Windows-style
file URLs, unknown methods, close diagnostics, compiler hangs/output floods,
and help-mode termination.

Rollback is a code-only revert with no persisted state migration. The byte
framer can be retained independently if later method changes are reverted.
