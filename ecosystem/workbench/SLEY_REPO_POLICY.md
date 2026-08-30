# Sley Repository Policy (Canonical)

## Defaults

- License: **Apache 2.0** by default for this Sley ecosystem repository.
- Source-of-truth language: **Sley** (`.sley`) for tool contracts/implementation surface.
- Host wrappers (JS/TS/Node/etc.) are temporary until the Sley compiler emit/runtime targets are complete.

## GitHub language stats policy

Use this `.gitattributes` baseline until GitHub Linguist ships native Sley support:

```ini
*.js linguist-vendored
*.mjs linguist-vendored
*.ts linguist-vendored
*.json linguist-vendored
*.md linguist-detectable=false
*.sley linguist-language=Sley
```

## Migration intent

- Keep `src/tool.sley` authoritative for command contracts.
- Update wrappers only as compatibility shims and migration scaffolds.
- Reconcile README/metadata to state current source-of-truth status clearly.
