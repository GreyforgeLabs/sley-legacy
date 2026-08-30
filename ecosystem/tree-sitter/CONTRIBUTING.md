# Contributing

This repository is publicly visible while the Sley ecosystem stabilizes.

## Local Loop

```bash
make fmt
make test
make smoke
```

## Rules

- Keep Sley compiler JSON contracts explicit.
- Do not duplicate Loom semantic authority.
- Do not add real network, provider, deploy, wallet, spend, or secret calls
  to tests.
- Prefer deterministic fixtures.
- Use `sley fix --dry-run` or `sley graft --dry-run` evidence before any
  write feature.
