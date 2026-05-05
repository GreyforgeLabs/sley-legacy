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
- structured expressions for literals, identifiers, unary/binary operators,
  `if` expressions, calls, field access, record literals, and `?`
- static checks for duplicate declarations, unknown types, unknown effects,
  simple return mismatches, `?` result flow, host-effect authority, user
  function call arity/types, called-function effect propagation, lexical
  locals, operator operand types, `if` condition/branch types, and typed record
  literal fields
- runtime evaluation for zero-argument pure `main`, literal values, pure
  function calls, lexical locals, operators, `if` expressions, record literals,
  and record field access
- structural patch operations for adding/removing parameters, replacing
  function bodies, adding imports/effects/types/functions, and renaming
  declarations

Known v0 limits:

- Expression parsing still falls back to raw nodes for unsupported syntax such
  as collection literals, lambdas, pattern matching, and multi-statement
  expression blocks.
- Comment attachment and durable provenance storage are not complete. Accepted
  patch provenance is returned by the patch command, but not written into a
  sidecar store.
- `run` only supports pure execution. Host calls and capability-backed runtime
  objects are still represented as raw values unless a future host boundary is
  supplied.
- `UpdateCallSites`, `InsertStatement`, `ReplaceExpression`, `MoveNode`, and
  `DeleteNode` are declared but return explicit unsupported-operation
  diagnostics.

The useful next step is to add real block control flow and data collections:
`while`/`for`, `match`, list/map literals, indexing, and multi-statement branch
blocks.
