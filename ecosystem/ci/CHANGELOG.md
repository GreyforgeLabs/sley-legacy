# Changelog

## 0.0.1-private - 2026-08-28

- Passed action inputs through environment variables instead of interpolating
  caller-controlled values into a shell program.
- Made missing dependencies and failed steps fail closed while preserving one
  valid JSON report for every machine-mode invocation.
- Added hostile-input, control-character, and unavailable-tool regressions.

## 0.0.0-private - 2026-05-06

- Initial private scaffold for `sley-ci`.
- Added documentation, brand assets, examples, and smoke-test entry points.
