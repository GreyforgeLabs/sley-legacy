# ADR-011: First-class manifest-backed testing

Status: accepted for S12-504
Owner: Sley maintainers
Date: 2026-08-25

## Target state

S12-504 makes `sley test` a first-class GA command without inventing
language-native test syntax. The accepted input is the strict
`sley.test.manifest.v1` contract discovered through `sley.test.json` and
`*.sley-test.json`. Discovery is repository-confined, symlink-rejecting,
byte-sorted, and deterministic.

The command reuses existing checked execution surfaces:

- table and bounded property cases invoke pure tasks through `sley machine`;
- effect mocks invoke `sley run` with explicit local capabilities and seeds;
- adapter cases invoke exact `sley adapter replay` evidence;
- expected diagnostics invoke `sley check`;
- differential cases compare two bounded pure Sley endpoints by exact
  canonical JSON.

No case kind permits arbitrary shell execution, provider calls, repository
mutation, deploy, spend, or external authority.

## Deterministic bounds

Every discovered manifest in a suite must agree on source, package, authority,
bounds, and coverage inventory. Case identifiers are unique across manifests.
Property generation uses bounded cartesian enumeration in declared field and
value order. The runner checks projected total and generated execution counts
before execution, then enforces wall-time and report-output ceilings.

Reports contain digests of inputs, expected and actual values, and nested
subreports. They do not copy raw effect seeds or private runtime values.
Identical source, manifests, selection, and bounded outcomes produce identical
ordered JSON and `report_digest` values.

## Coverage truth

`sley.test.report.v1` separates four evidence states for task, branch, effect,
and capability coverage:

- `observed` records entrypoint, runtime-authority, or adapter-authority
  evidence;
- `declared` records passing manifest witnesses that are not instrumented;
- `unsupported` records an inventory class the runner cannot credit;
- `unknown` records claims outside the exact checked inventory.

Task, effect, and capability evidence is observed through existing execution
surfaces. Branch coverage is explicitly labeled
`passing_manifest_witness`; it is never presented as runtime instrumentation.
Credited and uncovered inventories are retained separately.

## Changed-node focus

Repeatable `--changed-node` arguments select cases by exact structural node
coverage. Task changes expand through the reverse caller graph emitted by
`sley query`; module nodes expand to their checked tasks. A requested node
with no selected evidence produces a typed failing report instead of a false
pass. S12-505 may derive these nodes from changed-file reports, but it must not
replace this graph-derived selection contract with file-family guesses.

## Review evidence binding

The frozen `sley.change.review.v0` and
`sley.change.transaction_seal.v0` contracts are not weakened or overloaded.
`sley test bind-review` validates an exact passing test report, review, and
seal, then emits the stable read-only `sley.review.packet.v1` successor
envelope. The packet binds transaction, review, seal, final source, suite, and
test report identities. It grants no mutation authority and publishes no
transaction artifact by itself.

## Contract floor

The following roots are stable for Sley 1.2 GA:

- `sley.test.manifest.v1`
- `sley.test.report.v1`
- `sley.review.packet.v1`

Any field or semantic change requires a new schema identifier.

## Validation and recovery

`make user-tests` proves two-manifest discovery order, all six case kinds,
bounded property generation, effect and adapter evidence, expected
diagnostics, translation differentials, exact coverage classification,
changed-node focus and failure, deterministic reports, human output, contract
validation, and review-packet binding.

Recovery is removal of the three contracts, `loom.testing`, the command and
runner wiring, fixture suite, focused gate, and validation routing. Existing
machine, runtime, adapter, worker, and transaction contracts remain unchanged.

## Exclusions

Language-native test syntax, arbitrary host test commands, live provider
testing, parallel execution, changed-file derivation, subsystem validation
reports, deployment, package publication, and public release remain outside
S12-504.
