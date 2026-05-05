# WeaveLang v0

This is the first executable implementation slice for `../WeaveLangSpec.md`.

Implemented now:

- `weave parse`
- `weave format`
- `weave check`
- `weave run`
- `weave ast`
- `weave patch`
- module, import, type, effect, and function declarations
- record type declarations
- explicit function parameters and return types
- raw-expression statements with literal classification
- static checks for duplicate declarations, unknown types, unknown effects,
  simple return mismatches, `?` result flow, and host-effect authority
- structural patch operations for adding/removing parameters, replacing
  function bodies, adding imports/effects/types/functions, and renaming
  declarations

Known v0 limits:

- Expression parsing is intentionally shallow. The compiler preserves expression
  source and classifies literals and identifiers, but calls and record literals
  are not fully typed yet.
- Comment attachment and durable provenance storage are not complete. Accepted
  patch provenance is returned by the patch command, but not written into a
  sidecar store.
- `run` only supports a zero-argument pure `main` and literal-ish returns.
- `UpdateCallSites`, `InsertStatement`, `ReplaceExpression`, `MoveNode`, and
  `DeleteNode` are declared but return explicit unsupported-operation
  diagnostics.

The useful next step is to replace raw expressions with a real expression AST
while keeping the current CLI and patch contracts stable.
