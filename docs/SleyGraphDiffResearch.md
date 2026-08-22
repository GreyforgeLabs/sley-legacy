# Sley Three-way Graph Diff Research

`sley graph-diff` compares compiler-owned semantic projections for a base,
ours, and theirs target:

```bash
sley graph-diff --json \
  --base path/to/base.sley \
  --ours path/to/ours.sley \
  --theirs path/to/theirs.sley
```

The v0 result uses `sley.graph_diff.report.v0`, has `mode=report_only`, and
always has `merge_permitted=false`. A `passed` result means that this bounded
comparison found no overlapping or unsupported changes. It is not proof that a
source merge is safe.

## Identities and ordering

Modules use `module:<module_name>`. Imports and declarations reuse compiler
IDs. Calls use their expression ID qualified as `call:<expression_id>`. Task
bodies come from AST projections with source spans removed, preventing line
movement from changing semantic comparison while retaining statement and
expression structure.

Changes sort by declaration-kind rank, semantic identity, and side. Output is
deterministic for identical compiler projections.

## Fail-closed conflicts

The contract reserves explicit classes for:

- `same_node_edit`
- `rename_delete`
- `call_site_rewrite`
- `effect_change`
- `gate_change`
- `projection_drift`
- `unsupported_ambiguous`

The current prototype detects same-identity edits, call-site rewrites, effect
changes, Gate-take changes, projection/schema drift, and non-resolved calls.
Rename inference remains deliberately unsupported because declaration deletion
plus addition does not prove semantic identity. Take identity is also
provisional outside its owning task projection.

## Non-goals

This command does not merge, format projected output, write source, install a
Git merge driver, or accept changes on behalf of Git. Automatic structural
merge remains blocked until rename and take identity are proven, adversarial
corpus coverage exists, and projected results pass round-trip format, check,
lint, and verify gates against ordinary textual-merge comparisons.

## Validation

```bash
make graph-diff
```

The focused gate covers no-op comparison, deterministic ordering, same-node
body edits, call-site rewrites, effect changes, Gate-take changes, nonzero exit
on conflict, and JSON Schema draft 2020-12 validation.
