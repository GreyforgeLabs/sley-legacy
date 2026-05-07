# Sley VS Code Shim

Status: local development shim for the in-tree `sley-lsp` server. This package
is private and unpublished until public release metadata is approved.

## Use Locally

Build the server:

```bash
cargo build --bin sley-lsp
```

Install extension dependencies:

```bash
cd editors/vscode-sley
npm install
```

Launch VS Code with this extension folder, then set `sley.languageServer.path`
to the built server path if `sley-lsp` is not already on `PATH`:

```bash
code --extensionDevelopmentPath="$PWD" /home/greyforge/sley
```

Example setting:

```json
{
  "sley.languageServer.path": "/home/greyforge/sley/target/debug/sley-lsp"
}
```

The extension contributes `.sley` language metadata, a TextMate grammar for
basic highlighting, and a stdio LSP bridge to `sley-lsp`. The server remains
the semantic authority for diagnostics, symbols, hover, range-scoped code
actions, rename, and surface-pinned non-mutating repair previews.

## Validate

```bash
npm run validate
```
