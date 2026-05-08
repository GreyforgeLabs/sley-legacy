# Sley Agent Quickstart

This is the shortest current path from an empty directory to a checked Sley
agent project with dry-run deploy artifacts. It is local-only: the seeded
runtime adapters below do not read real secrets, call live networks, call model
providers, mutate infrastructure, or spend money.

The project is designed for source-repo, release-binary, and shim-based usage.

Ensure the `sley` CLI and companion binaries are available on `PATH` for this
shell (from release packaging or install tooling) before running these commands.

## 1. Create A Project

```bash
sley new --json --template agent-project --name agent-app --module agent.main agent-app
cd agent-app
```

The `agent-project` template creates a multi-module project with explicit
`SecretRead`, `Network`, `ModelCall`, and `Deploy` authority boundaries.

## 2. Inspect Before Editing

```bash
sley check --json .
sley doctor --json .
sley query --json --kind tasks .
sley query --json --kind calls .
sley plan --json --graft-templates .
sley lint --json --deny-warnings .
```

Use the JSON output as the agent contract. Prefer `sley plan` and checked graft
templates over raw source edits when the change can be expressed structurally.

For focused editing, inspect the graph around a node before proposing a graft:

```bash
sley graph --json .
sley graph --json --slice <node-id> .
sley ast --json --node <node-id> .
```

Graph slices expose checked affordance arrays for add, insert, move, delete,
replace, call-site, and call-argument graft starters. Use those operation
payloads, or the matching `sley plan --graft-templates` rows, before falling
back to raw source edits.

## 3. Run With Seeded Authority

The starter intentionally requires seeded capabilities. In bash or zsh, keep the
seed set in one array so the same authority evidence is reused by run, verify,
and deploy:

```bash
SLEY_AGENT_SEEDS=(
  --cap SecretRead
  --secret api_key redacted
  --cap Network
  --http-text https://example.test/profile "profile ready"
  --cap ModelCall
  --model-output deploy-plan "plan approved"
  --cap Deploy
  --deploy-result staging staged
)

sley run --json "${SLEY_AGENT_SEEDS[@]}" .
sley verify --json --deny-warnings "${SLEY_AGENT_SEEDS[@]}" .
```

`sley run` proves deterministic execution. `sley verify` adds check, lint,
runtime, trace, seal, and ZJX handoff next-actions in one readiness report.

## 4. Package A Dry-Run Deploy

```bash
sley deploy --json --dry-run --artifacts-dir .sley/deploy "${SLEY_AGENT_SEEDS[@]}" .
sley-contract inspect-deploy-artifacts .sley/deploy --json
```

A ready package writes:

- `.sley/deploy/deploy-report.json`
- `.sley/deploy/seal.json`
- `.sley/deploy/zjx-envelope.json`
- `.sley/deploy/manifest.json`

`sley-contract inspect-deploy-artifacts` revalidates the manifest, schemas, and
digests after the handoff directory moves between agent sessions.

## 5. Repair Loop

When a gate blocks, use the compiler feedback loop instead of guessing:

```bash
sley doctor --json .
sley plan --json --graft-templates .
sley fix --json --kind <kind> --dry-run .
sley fix --json --kind <kind> --write .
sley verify --json --deny-warnings "${SLEY_AGENT_SEEDS[@]}" .
```

If a graft precondition fails, refresh the AST or graph slice and rebuild the
operation from current node IDs.

## 6. Local V1 Gate

From the Sley repository root:

```bash
make v1
```

For a public release cut, use the stricter metadata gate only after the operator
has approved license and repository values:

```bash
make public-release-check
```

Until those metadata decisions are made, `make public-release-check` is expected
to fail on public packaging blockers even when the executable v1 gate passes.
