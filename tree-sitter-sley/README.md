# tree-sitter-sley

Status: in-tree bootstrap for Sley syntax review and editor tooling.

This package provides the first Tree-sitter grammar for the human-readable Sley
source projection. It is intentionally parser-only: it does not call providers,
read secrets, deploy, spend, or mutate Sley source files.

## Commands

```sh
npm install
npm test
```

`npm test` regenerates the parser, runs the checked corpus under
`test/corpus/`, and parses current `.sley` examples plus accepted compiler
corpus fixtures from the parent repo.

## Current Scope

- module and import declarations
- exported and unexported type, effect, and task declarations
- task takes, authority gates, binding statements, set/return/control flow
- call, try, list, map, record, field/index, unary, binary, and if expressions
- `//` and `#` line comments

The grammar is for editor tokenization and structural review. The Rust compiler
remains the source of truth for validation, checking, runtime semantics, grafts,
and deployment gates.
