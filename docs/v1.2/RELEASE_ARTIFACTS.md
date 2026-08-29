# Sley 1.2 release artifacts

Status: local release-candidate procedure
Supported target: Linux x86_64
Updated: 2026-08-26

Sley 1.2 ships one supported, self-contained source distribution:
`sley-1.2.1-linux-x86_64.tar.gz`. macOS, Windows, containers, package
managers, signatures, and public distribution channels are post-GA work.

## Build

Build only from a clean committed source tree:

```bash
make release-archive
```

The build is deterministic for one source commit. It uses the commit timestamp,
sorted archive members, normalized ownership and modes, and a gzip header with
no host filename. Existing outputs are never overwritten in release mode.

`dist/` contains:

- the `.tar.gz` archive;
- a `.tar.gz.sha256` checksum;
- the external `manifest.json` projection;
- the license inventory;
- an SPDX 2.3 JSON SBOM;
- unsigned provenance binding the source, archive, and metadata digests.

The manifest, license inventory, and SBOM are also embedded under `release/`
inside the archive. The archive manifest is the Sley 1.2 version manifest.

## Verify

```bash
make release-security-check
```

Verification rejects missing or symlinked artifacts, unsafe archive members,
checksum or provenance mismatch, a source commit other than the current clean
checkout `HEAD`, invalid contracts, dirty release-candidate
metadata, payload inventory drift, forbidden host paths, secret patterns, and
unpacked toolchain failures. It then runs the unpacked CLI/version, toolchain
doctor, checker, contract fixtures, and Python/Node worker-client gate.

For a clean extracted archive:

```bash
export PATH="$(pwd)/sley-1.2.1-linux-x86_64/bin:$PATH"
sley doctor --toolchain --json
sley --version
```

## Authority

The artifact and provenance are unsigned and explicitly record
`local_release_candidate_only`. Building or verifying them grants no authority
to tag, push, upload, publish, deploy, announce, or sign. Those irreversible
actions require separate exact operator approval.
