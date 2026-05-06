# Sley ZJX Runtime Lock Spec

Date: 2026-05-06
Status: draft engineering spec
Owner: Greyforge Labs
Public status: internal until Sley/ZJX licensing is finalized

## Purpose

This spec defines the lock boundary for the future `zjx-sley` runtime.

The free Sley ZJX runtime must compress only valid Sley structural artifacts.
It must not become a free generic ZJX archive compressor through spoofed JSON
metadata, oversized opaque fields, base64 smuggling, or file-extension tricks.

The core rule is:

```text
zjx-sley does not trust a header.
zjx-sley accepts only artifacts that Sley Core can reconstruct, validate, and
hash as canonical Sley graph structure.
```

## Current System Boundary

Current Sley state:

- Sley semantic identity is the canonical typed graph bytes.
- `sley zjx` emits a preview JSON envelope.
- The current envelope uses `schema=sley.zjx.envelope.v0`,
  `format=zjx-preview-json`, and `compression=none`.
- The current envelope includes `graph_digest=sha256:...` computed from the
  emitted symbol graph JSON.
- The current command is not a compressed `.zjx` archive writer.

Current ZJX state:

- Generic ZJX archive compression lives outside Sley.
- Generic ZJX remains separately licensed.
- The future `zjx-sley` runtime is a Sley-artifact runtime, not ZJX Pro.

Therefore the first compressed integration must preserve this boundary:

```text
Sley Core validates and canonicalizes structure.
zjx-sley packs only validated Sley structure.
ZJX Pro packs arbitrary files, logs, checkpoints, and directories.
```

## Threat Model

The primary abuse case is a spoofed envelope:

```json
{
  "zjx_scope": "sley_artifact",
  "sley_schema": "sley.zjx.envelope.v0",
  "payload": "massive generic data disguised as Sley"
}
```

The attacker wants the free Sley runtime to compress generic enterprise data
without a ZJX Pro license. Likely inputs include CSV logs, JSONL logs, model
checkpoints, backup trees, binary blobs, parquet exports, database dumps, or
large base64 strings hidden inside a Sley-looking object.

Attack classes:

- metadata spoofing;
- schema-name spoofing;
- extension spoofing;
- unknown-field smuggling;
- large string literal smuggling;
- base64/blob smuggling;
- graph-node kind spoofing;
- graph-edge relationship spoofing;
- trace receipt spoofing;
- manifest tampering;
- replaying an old valid scope manifest over a new payload.

The lock is considered failed if a normal engineer can use `zjx-sley` as a
generic compressor by wrapping arbitrary bytes in JSON.

## Non-Goals

This spec does not claim:

- that local binaries cannot be patched;
- that the lock is unbreakable DRM;
- that Sley artifacts are secret;
- that Sley Core needs network access;
- that Sley Core contains the protected ZJX encoder;
- that `zjx-sley` replaces ZJX Pro.

The lock is a product-scope and license-enforcement boundary. It reduces casual
and contractual misuse. It is not the only IP defense.

## Package Responsibilities

### Sley Core

License target:

```text
Apache-2.0
```

Responsibilities:

- parse `.sley` files and projects;
- check task, type, effect, binding, authority, module, and graph semantics;
- build canonical typed graph bytes;
- compute the semantic graph hash;
- emit `sley.zjx.envelope.v0` preview envelopes;
- expose a strict validation report that `zjx-sley` can consume;
- avoid including generic ZJX encoder code.

Sley Core is the source of truth for whether something is a Sley artifact.

### zjx-sley

License target:

```text
Separate Sley-use license or commercial addendum.
```

Responsibilities:

- call or link the Sley Core validator;
- accept only validated Sley structural artifacts;
- write `zjx_scope=sley_artifact` only after validation;
- pack, test, inspect, and unpack Sley `.zjx` artifacts;
- reject generic archive inputs;
- reject unknown or opaque payload surfaces;
- never expose a generic archive mode.

`zjx-sley` may use protected compression logic internally, but its public
permission is limited to Sley artifacts.

### ZJX Pro

License target:

```text
Commercial agreement only.
```

Responsibilities:

- pack arbitrary directories and files;
- handle generic logs, CSV, JSONL, checkpoints, backups, and data lakes;
- run storage audits;
- expose enterprise benchmark and integration surfaces.

Any input rejected by `zjx-sley` because it is not Sley structure belongs here.

## Accepted Inputs

`zjx-sley pack` may accept only these input classes.

### Sley Source File

Example:

```bash
zjx-sley pack app.sley out.sley.zjx
```

Required validation:

- parse source;
- check source successfully;
- build symbol graph;
- build canonical typed graph bytes;
- compute graph hash;
- generate internal Sley envelope;
- pack only the canonical Sley artifact set.

### Sley Project

Example:

```bash
zjx-sley pack ./project out.sley.zjx
```

Required validation:

- load `sley.toml`;
- load only project-owned `.sley` modules under the configured source root;
- reject unexpected binary payloads in the Sley artifact set;
- check the project successfully;
- build the project graph;
- compute graph hash;
- generate internal Sley envelope;
- pack only the canonical Sley artifact set.

### Sley ZJX Preview Envelope

Example:

```bash
sley zjx --json ./project > envelope.json
zjx-sley pack envelope.json out.sley.zjx
```

Required validation:

- parse JSON;
- require `schema=sley.zjx.envelope.v0`;
- require `format=zjx-preview-json`;
- require `compression=none`;
- reject additional top-level fields;
- validate the graph object against the Sley graph contract;
- validate optional slices and trace receipts;
- recompute canonical graph bytes from the submitted graph;
- compute graph hash;
- compare the supplied `graph_digest` against the recomputed value;
- pack only after structural validation succeeds.

### Future Canonical Graph Bytes

Allowed only after canonical graph encoding is frozen.

Required validation:

- decode canonical graph bytes;
- validate graph invariants;
- compute graph hash from decoded canonical bytes;
- ensure the artifact set has no opaque generic payload section.

## Rejected Inputs

`zjx-sley` must reject:

- arbitrary directories without a Sley project manifest;
- arbitrary single files that are not `.sley` source or valid Sley envelopes;
- CSV files;
- JSONL files;
- parquet files;
- model checkpoints;
- ordinary backups;
- media files;
- generic binary blobs;
- generic `.zjx` archives;
- generic JSON objects;
- Sley-looking JSON with unknown top-level fields;
- Sley-looking JSON with opaque payload fields;
- Sley-looking JSON with malformed graph nodes;
- Sley-looking JSON with invalid graph edges;
- Sley-looking JSON whose hash does not match recomputed graph structure;
- requests to set `generic_archive_enabled=true`;
- requests to strip, rewrite, or override `zjx_scope`.

Error message:

```text
Error: ZJX Free Runtime only supports validated Sley structural artifacts.
For generic binary, log, checkpoint, backup, or directory compression, use
ZJX Pro.
```

## Validation Pipeline

The required pack pipeline is:

```text
input
  -> classify input kind
  -> parse using the input-kind parser
  -> run Sley structural validation
  -> build or reconstruct canonical typed graph bytes
  -> compute graph hash
  -> run anti-smuggling checks
  -> build scoped manifest
  -> compress Sley artifact set
  -> test archive before returning success
```

No compression work should begin before validation and anti-smuggling checks
complete.

## Canonical Graph Hash Rule

The graph hash must be recomputed by Sley Core or a byte-for-byte equivalent
validator. It must not be trusted from the input file.

Correct:

```text
hash = sha256(canonical_graph_bytes(reconstructed_graph))
```

Incorrect:

```text
hash = input_json["sley_graph_hash"]
hash = sha256(input_file_bytes)
hash = sha256(input_json["payload"])
hash = sha256(any user-supplied opaque field)
```

The hash proves only that the artifact corresponds to the canonical graph
representation accepted by Sley. It does not grant permission to attach
arbitrary side payloads.

## Structural Validation Contract

The validator must confirm graph invariants, not just JSON shape.

Minimum required checks:

- known schema ID;
- known graph root shape;
- known node kinds only;
- stable node IDs;
- no duplicate node IDs;
- all graph edges point to existing nodes;
- module paths are valid;
- task/type/effect declarations are valid;
- imports resolve according to Sley rules;
- exported declarations obey visibility rules;
- task calls resolve;
- task call arity and types are valid;
- effect propagation is valid;
- authority gates are explicit;
- binding kinds are known;
- immutable bindings are not mutated;
- mutable bindings use allowed mutable forms;
- trace receipts, when present, match known receipt schema;
- graph slice, when present, references nodes in the full graph;
- no unknown top-level fields;
- no unknown graph-node fields unless explicitly versioned.

Validation should prefer the existing Sley checker and graph builder over a
parallel handwritten validator. Duplicated validators drift.

## Anti-Smuggling Rules

The validator must reject Sley-looking artifacts that carry generic data.

Top-level forbidden field names:

```text
payload
data
blob
bytes
archive
file
files
file_contents
base64
raw
raw_bytes
binary
checkpoint
log
csv
jsonl
parquet
```

Nested forbidden opaque-field behavior:

- unknown object keys;
- untyped byte arrays;
- arbitrary attachment arrays;
- raw base64 blocks;
- strings above configured structural limits;
- arrays above configured structural limits unless they are known graph arrays;
- maps whose keys or values do not match Sley graph contracts.

String-literal limits:

- Sley source may contain ordinary string literals.
- A Sley artifact must not use string literals as a generic data container.
- `zjx-sley` should enforce a default per-string literal cap.
- `zjx-sley` should enforce a default total string-literal byte cap.
- Larger caps require a paid/pro mode or an explicit enterprise policy file.

Initial conservative defaults:

```text
max_single_string_literal_bytes = 65536
max_total_string_literal_bytes = 1048576
max_unknown_json_fields = 0
max_embedded_trace_receipt_bytes = 1048576
```

These are product defaults, not language limits. Sley Core can allow larger
programs later, but the free ZJX runtime should not become a blob compressor.

Base64 suspicion heuristic:

```text
if a string is large, mostly [A-Za-z0-9+/=_-], and decodes cleanly as base64,
reject it unless the field is a known Sley field whose semantics permit it.
```

This heuristic is not the main lock. The main lock is structural validation
and absence of opaque payload fields.

## Scoped Manifest Contract

`zjx-sley` writes scope after validation. It does not accept user-authored
scope as authority.

Required manifest fields:

```json
{
  "zjx_scope": "sley_artifact",
  "zjx_scope_version": "0",
  "sley_schema": "sley.zjx.envelope.v0",
  "sley_format": "zjx-preview-json",
  "sley_graph_hash": "sha256:...",
  "sley_validator": "sley-core",
  "sley_validator_version": "...",
  "zjx_runtime": "zjx-sley",
  "zjx_runtime_version": "...",
  "generic_archive_enabled": false
}
```

Optional fields:

```json
{
  "sley_project_manifest_hash": "sha256:...",
  "sley_trace_seal_hash": "sha256:...",
  "sley_source_projection_hash": "sha256:...",
  "validated_at_unix": 0,
  "policy_id": "free-sley-artifact-v0"
}
```

Manifest tampering rules:

- `zjx_scope` is immutable after pack.
- `generic_archive_enabled` must be false for `zjx-sley`.
- unpack/test/inspect must report scope.
- unpack/test/inspect must fail if the manifest claims Sley scope but graph
  validation fails.

## CLI Contract

Initial commands:

```bash
zjx-sley validate input --json
zjx-sley pack input out.sley.zjx --json
zjx-sley test out.sley.zjx --json
zjx-sley inspect out.sley.zjx --json
zjx-sley unpack out.sley.zjx restored --json
```

Future Sley hook:

```bash
sley pack-zjx --runtime zjx-sley project out.sley.zjx
```

Required behavior:

- `pack` runs validation before compression.
- `test` validates archive integrity and Sley scope.
- `inspect` reports lock scope, graph hash, validator version, runtime version,
  and whether generic archive mode is disabled.
- `unpack` restores only the Sley artifact representation, not arbitrary hidden
  files.
- `validate` performs the same structural and anti-smuggling checks without
  writing an archive.

## Error Codes

Use stable machine-readable error codes.

```text
SLEY_ZJX_UNSUPPORTED_INPUT
SLEY_ZJX_SCHEMA_MISMATCH
SLEY_ZJX_UNKNOWN_FIELD
SLEY_ZJX_GRAPH_INVALID
SLEY_ZJX_GRAPH_HASH_MISMATCH
SLEY_ZJX_TRACE_INVALID
SLEY_ZJX_SLICE_INVALID
SLEY_ZJX_OPAQUE_PAYLOAD_REJECTED
SLEY_ZJX_STRING_LIMIT_EXCEEDED
SLEY_ZJX_BASE64_SMUGGLING_REJECTED
SLEY_ZJX_GENERIC_ARCHIVE_REJECTED
SLEY_ZJX_SCOPE_TAMPERED
```

Human-facing generic-compression rejection:

```text
ZJX Free Runtime only supports validated Sley structural artifacts. For generic
binary, log, checkpoint, backup, or directory compression, use ZJX Pro.
```

## Test Matrix

### Accept Tests

- valid single `.sley` file packs;
- valid Sley project packs;
- valid `sley.zjx.envelope.v0` packs;
- valid envelope with graph slice packs;
- valid envelope with trace receipts packs;
- packed archive tests successfully;
- inspected archive reports `zjx_scope=sley_artifact`;
- unpacked artifact reconstructs the same canonical graph hash.

### Reject Tests

- arbitrary directory without `sley.toml`;
- arbitrary `.txt` file;
- arbitrary `.csv` file;
- arbitrary `.jsonl` file;
- generic JSON object;
- envelope with `payload` field;
- envelope with `data` field;
- envelope with unknown top-level field;
- envelope with malformed graph node;
- envelope with duplicate graph node IDs;
- envelope with edge to missing node;
- envelope with invalid graph slice reference;
- envelope with malformed trace receipt;
- envelope with user-supplied hash that does not match recomputed graph;
- envelope with giant string literal;
- envelope with giant base64-looking string;
- request to set `generic_archive_enabled=true`;
- request to rewrite `zjx_scope`;
- generic `.zjx` archive passed to `zjx-sley pack`.

### Release Tests

- public Sley binary contains no generic ZJX encoder symbols;
- public Sley source package contains no protected ZJX encoder files;
- `zjx-sley` rejects generic corpus fixtures by default;
- docs do not call `zjx-sley` a generic compressor;
- docs do not call restricted ZJX runtime open source;
- package contains the correct license files;
- release notes name the Sley-only runtime scope.

## Implementation Guidance

Prefer this dependency direction:

```text
zjx-sley -> Sley Core validation API or Sley Core CLI
Sley Core -> no protected ZJX encoder dependency
```

Acceptable first implementation:

```text
zjx-sley shells out to:
  sley verify --json input
  sley zjx --json input
then validates the envelope and graph hash before compression.
```

Better later implementation:

```text
zjx-sley links a small Sley validation library that exposes:
  validate_source_or_project(input) -> ValidatedSleyArtifact
  validate_envelope(json) -> ValidatedSleyArtifact
```

Do not implement a second loose JSON-only validator inside ZJX. That creates a
spoofing surface and will drift from Sley semantics.

## Open Questions

- What is the exact canonical graph byte encoding for v0?
- Should `sley zjx --json` add source or trace digests, or should those remain
  seal/runtime-manifest fields only?
- What are the first free-runtime string and trace-size caps?
- Should enterprise Sley projects be allowed larger string caps through a
  signed policy file?
- Should `zjx-sley` support signed offline leases for larger Sley-only packs?
- Should `zjx-sley inspect` revalidate the full graph every time or expose a
  fast mode plus a strict mode?

## Decision

The file lock is not a JSON header. The file lock is a successful Sley
structural validation pipeline followed by a scoped manifest written by the
runtime.

The safe rule is:

```text
If Sley Core cannot prove it is Sley structure, zjx-sley does not compress it.
```
