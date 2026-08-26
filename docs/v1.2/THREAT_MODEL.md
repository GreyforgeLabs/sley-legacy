# Sley 1.2 release threat model

Status: GA release boundary
Updated: 2026-08-26

## Assets and trust boundary

The protected assets are source identity, release payload integrity, contract
identity, user filesystem safety, secrets, and the operator's external release
authority. Git-tracked source and Sley-owned release vocabulary are build
inputs. Repository content is never publication authority.

## Addressed threats

| Threat | Control |
|---|---|
| Dirty, ambiguous, or stale source | Release builds require a clean committed tree; verification binds both metadata roots to the current clean checkout `HEAD`; the RC target builds before verifying. Bounded tests use explicit dual test hooks and emit `test_only`. |
| Path traversal or link escape | Verification rejects absolute paths, `..`, alternate roots, symlinks, hard links, devices, and FIFOs before extraction. |
| Artifact or metadata tampering | SHA-256 checksum and provenance bind the archive, manifest, license inventory, and SBOM. Embedded metadata must equal its external projection. |
| Payload substitution | The manifest records every payload path, digest, byte count, and executable mode plus a deterministic tree digest. |
| Host identity leakage | The archive excludes the development checkpoint and scans for Greyforge host paths. |
| Credential leakage | The verifier scans text payloads for private-key headers and common token prefixes; release inputs do not include host credentials. |
| Dependency or PATH confusion | `sley doctor --toolchain` checks minimum versions, required contracts, local replay availability, and conflicting `sley` resolution. |
| Contract drift | Five W7 v1 contracts are strict fixtures and stable-registry entries. Clean-install verification runs the bundled contract gate. |
| Authority escalation | Manifest, license inventory, and provenance hard-code publication, tag, upload, signing, external-adapter, and provider authority as absent. |

## Residual risk

- SHA-256 provenance is unsigned and supplies integrity, not publisher
  authentication.
- The supported artifact is Linux x86_64 source tooling over host dependencies,
  not a hermetic native binary.
- Dependency provenance belongs to the host installation; dependencies are not
  redistributed in the archive.
- Secret-pattern scanning is a defense in depth check, not a substitute for
  repository secret scanning or independent release review.
- Public hosting, signatures, channel metadata, and platform-specific install
  hardening remain unimplemented and unauthorized.

These residual risks are explicit release facts. They do not widen the local
release candidate into a public or production claim.
