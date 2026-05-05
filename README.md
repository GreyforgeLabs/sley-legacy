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
  `if` expressions, calls, field access, list literals, indexing, record
  literals, map literals, and `?`
- statement-level `if`/`else`, `while`, `for`, and `set` local mutation
- builtin `len` for lists, maps, and text
- static checks for duplicate declarations, unknown types, unknown effects,
  simple return mismatches, `?` result flow, host-effect authority, user
  function call arity/types, called-function effect propagation, lexical
  locals, operator operand types, `if` condition/branch types, and typed record
  literal fields, list element types, map key/value types, indexing,
  control-flow conditions, `for` loop collections, and `set` mutation types
- runtime evaluation for zero-argument pure `main`, literal values, pure
  function calls, lexical locals, `set`, operators, `if` expressions,
  statement-level `if`/`else`, `while`, `for`, list literals, map literals,
  indexing, `len`, record literals, and record field access
- structural patch operations for adding/removing parameters, replacing
  function bodies, adding imports/effects/types/functions, and renaming
  declarations

Known v0 limits:

- Expression parsing still falls back to raw nodes for unsupported syntax such
  as lambdas, pattern matching, and multi-statement expression blocks.
- Comment attachment and durable provenance storage are not complete. Accepted
  patch provenance is returned by the patch command, but not written into a
  sidecar store.
- `run` only supports pure execution. Host calls and capability-backed runtime
  objects are still represented as raw values unless a future host boundary is
  supplied.
- `UpdateCallSites`, `InsertStatement`, `ReplaceExpression`, `MoveNode`, and
  `DeleteNode` are declared but return explicit unsupported-operation
  diagnostics.

The useful next step is to add richer dispatch and module semantics: `match`,
multi-statement expression blocks, module resolution, broader map key support,
and a capability-backed host boundary.
