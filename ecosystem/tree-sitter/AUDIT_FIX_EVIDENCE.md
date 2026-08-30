# Audit Fix Evidence

Version: 0.1.1

Resolved finding: GF-AUD-023.

- `type_alias` is an authoritative root item and accepts the documented
  newline-terminated form.
- Boolean expressions accept either one expression or one comparator/right
  pair; adjacent expressions and comparator chains remain parse errors.
- Corpus coverage includes every top-level grammar construct, standalone and
  compared booleans, grouping, invalid adjacency, and invalid chaining.
- `tree-sitter generate` artifacts are tracked and regenerated in CI before
  corpus tests.

Validation: `npm test`, `npm run fmt`, and `git diff --check`.
