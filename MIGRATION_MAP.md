# Sley Legacy Migration Map

Migration date: 2026-08-30

The imports below use unsquashed `git subtree` merges of each source default
branch. Every listed source commit is an ancestor of the consolidated `public`
branch. Satellite tags and non-default branches remain in their original
archived repositories and in the verified migration mirrors because global tag
names collide across projects.

| Original repository | Repository ID | Default branch | Final source commit | Destination path | License | Historical repository disposition |
| --- | ---: | --- | --- | --- | --- | --- |
| [`GreyforgeLabs/sley`](https://github.com/GreyforgeLabs/sley-legacy) | 1232246165 | `public` | `f9d0722e9238c189aedb057f2ad4d8022d478492` | `/` | Apache-2.0 | Renamed to `GreyforgeLabs/sley-legacy`; active consolidated legacy home |
| [`GreyforgeLabs/sley-contract-kit`](https://github.com/GreyforgeLabs/sley-contract-kit) | 1230559649 | `main` | `437b72edb951edf08515ec1810dd2aeae2b38fb8` | `ecosystem/contract-kit/` | Apache-2.0 | Migration notice added; archived without rename |
| [`GreyforgeLabs/sley-ci`](https://github.com/GreyforgeLabs/sley-ci) | 1230559673 | `main` | `b795851d2fc92b3f0a53bd9c9b16ce71dfc8d70b` | `ecosystem/ci/` | Apache-2.0 | Migration notice added; archived without rename |
| [`GreyforgeLabs/tree-sitter-sley`](https://github.com/GreyforgeLabs/tree-sitter-sley) | 1230559712 | `main` | `18bca6a457979bb9d2a10af8d68b2a5265e7a05c` | `ecosystem/tree-sitter/` | Apache-2.0 | Migration notice added; archived without rename |
| [`GreyforgeLabs/sley-lsp`](https://github.com/GreyforgeLabs/sley-lsp) | 1230559760 | `main` | `2ad3e0956bfab85a025fb6d3713b6ff058f3d175` | `ecosystem/lsp/` | Apache-2.0 | Migration notice added; archived without rename |
| [`GreyforgeLabs/sley-workbench`](https://github.com/GreyforgeLabs/sley-workbench) | 1230559791 | `main` | `824d990e932acbd74758c6b709c295fbff557739` | `ecosystem/workbench/` | Apache-2.0 | Migration notice added; archived without rename |
| [`GreyforgeLabs/sley-conformance`](https://github.com/GreyforgeLabs/sley-conformance) | 1230559823 | `main` | `6876a81b60acc1d836a06c1151e0f31d16e46984` | `ecosystem/conformance/` | Apache-2.0 | Migration notice added; archived without rename |
| [`GreyforgeLabs/learnsley`](https://github.com/GreyforgeLabs/learnsley) | 1350119333 | `main` | `ad9d8c6e0e67c70c0dd96b40cc8e9df44df3ce52` | `learning/learnsley/` | No source license grant | Migration notice added; archived without rename |
| [`GreyforgeLabs/sley-audit`](https://github.com/GreyforgeLabs/sley-audit) | 1350119383 | `master` | `dac1292246e1b6a1d0bfae67b6472f6bfdc965c2` | `research/audit/` | Source file claims Apache-2.0; GitHub `NOASSERTION`; preserved verbatim | Migration notice added; archived without rename |

## Omitted refs and GitHub metadata

- `sley-ci`, `sley-lsp`, `sley-conformance`, and `sley-audit` each have an
  `audit/githubaudit-20260828` branch that was not merged into the consolidated
  default branch. The branches remain in the archived source repositories and
  verified mirrors.
- Tags `sley-contract-kit:v0.1.1`, `tree-sitter-sley:v0.1.1`, and
  `sley-workbench:v0.1.1` were not copied into the shared tag namespace because
  their names collide. They remain in the archived source repositories and
  verified mirrors.
- None of the eight satellites has a GitHub release or release asset at the
  migration snapshot.
- Sley 1.x tags and GitHub releases remain attached to repository ID
  `1232246165` after its rename to `sley-legacy`. No Sley 1.x tag or release is
  copied to the active Sley 2 repository.
- Open issues, pull requests, forks, Pages configurations, and submodules were
  empty for all eight satellites at the migration snapshot.

## Migration evidence

The machine-readable inventory, GitHub API snapshots, verified mirrors,
portable bundles, release assets, and SHA-256 manifest are stored locally under
`/home/greyforge/archive/github-migrations/sley-consolidation-20260830T043735Z/`.
