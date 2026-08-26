# Sley MCP Bridge

`sley-mcp-bridge` exposes a bounded Model Context Protocol surface over the
stage-1 Sley compiler and the local governed-change CLI. It speaks
newline-delimited UTF-8 JSON-RPC on stdin and stdout. Stdout is reserved for
protocol messages; operational and compiler logs go to stderr.

## Start

Pass the absolute top level of the Git repository an MCP client may inspect:

```bash
bin/sley-mcp-bridge --root /absolute/path/to/repository
```

The bridge refuses a relative root, a non-Git directory, or a subdirectory of a
Git repository. Each tool path is canonicalized with symlinks resolved and must
remain inside the configured root.

## Tools

- `sley_query`: structural query reports, optionally filtered by kind, module,
  or export visibility.
- `sley_lint`: lint reports, optionally filtered by module or rule.
- `sley_plan`: compiler-generated graft templates without mutation.
- `sley_propose_graft`: validation and dry-run preview of one graft operation.
- `sley_verify`: compiler verification without deployment or writes.

The governed transaction adapters are:

- `sley_change_inspect`, `sley_change_plan`, `sley_change_preview`, and
  `sley_change_approval_request` for the read-only request path;
- `sley_change_apply_authorization` for validating an operator-issued grant and
  deriving its exact bounded apply authorization;
- `sley_change_apply`, `sley_change_recover`, and `sley_change_rollback` for
  explicitly confirmed transaction mutation; and
- `sley_change_review` for explicitly confirmed terminal packet and seal
  publication.

Transaction artifacts are passed as paths to regular non-symlink files inside
the configured repository. The bridge fixes `--repository` to its configured
Git root and delegates every artifact, authority, freshness, replay, recovery,
verification, and seal decision to the matching `sley change` command. It does
not accept inline artifact objects or expose a generic change-command runner.

`sley change approve` and `sley change revocation-record` remain CLI-only
operator actions. An MCP client may request approval and consume operator-issued
grant and revocation evidence, but it cannot self-assert an issuer or create the
active authority record required by apply.

`tools/list` publishes JSON Schema draft 2020-12 input schemas and the existing
compiler-owned output report schemas. Transaction outputs allow either the
command's success schema or `sley.diagnostics.report.v0`. Successful and typed
negative transaction calls return the exact CLI report in both
`structuredContent` and text compatibility content.

The bridge constructs subprocess arguments as arrays and never evaluates tool
arguments as shell source. Mutating transaction tools publish non-read-only,
non-idempotent annotations and require the same explicit boolean confirmations
as the CLI. The bridge does not expose write-mode graft, fix, deploy, run,
arbitrary shell, or arbitrary command execution.

## Bounds and cancellation

The default maximum request frame is 1 MiB, the default compiler response is
2 MiB, and each compiler call has a 120-second timeout. Operators may lower or
raise those positive-integer limits with `SLEY_MCP_MAX_REQUEST_BYTES`,
`SLEY_MCP_MAX_RESPONSE_BYTES`, and `SLEY_MCP_TOOL_TIMEOUT_SECONDS`. Frame limits
may not exceed 64 MiB and the timeout may not exceed 3,600 seconds.

Request frames are read in bounded chunks, and an oversized frame is drained
without retaining the complete frame before the next request is processed.
Compiler stdout and stderr are protected by a process file-size ceiling before
the response-size contract is evaluated, so a tool cannot fill the temporary
workspace with an unbounded report.

This bootstrap bridge processes one stdio request at a time. It recognizes and
logs `notifications/cancelled` for inactive requests, but cannot preempt a
foreground compiler call because it does not read the next frame until that
call returns or times out. Clients should rely on the bounded timeout rather
than assume concurrent cancellation.

## Validation

```bash
make mcp-bridge
```

The focused gate covers lifecycle negotiation, tool discovery and calls,
malformed frames, path and symlink escape attempts, request/response bounds,
timeout behavior, cancellation notification handling, stdout protocol purity,
exact CLI/MCP transaction parity, confirmation denial, operator-only authority
issuance, verified apply, post-apply verification, terminal review publication,
and post-seal rollback denial.
