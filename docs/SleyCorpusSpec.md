# Sley Corpus Specification

Status: evaluation-evidence capture enabled for the approved 2026-08-23 baseline; corpus admission disabled
Version: 0.3
Sley compatibility: 1.1
Owner: Sley maintainers
Review trigger: language, compiler, benchmark, corpus schema, license policy, or retention-policy change

## 1. Purpose

This specification governs any Sley material retained for retrieval, model
adaptation, synthetic generation, or reusable engineering research. It turns
verified development evidence into an eligible corpus only through explicit
admission:

```text
candidate evidence
  -> provenance and authority check
  -> license and privacy classification
  -> sanitization and secret scan
  -> compiler-backed verification
  -> exact and semantic deduplication
  -> benchmark-contamination check
  -> partition assignment
  -> immutable manifest record
  -> approved retention or release
```

Normal development logs, model conversations, benchmark runs, and downstream
artifacts are not training data by default.

## 2. Normative Language

The words **MUST**, **MUST NOT**, **SHOULD**, **SHOULD NOT**, and **MAY** are
normative requirements in this document.

## 3. Current Boundary

This repository contains checked source, accepted and rejected compiler
fixtures, deterministic SleyBench smoke evidence, and a sanitized downstream
numerology workload. Those assets are possible source material, not an admitted
training corpus.

Reusable model-trajectory capture MUST remain disabled until:

1. the record and corpus-manifest schemas described here exist and reject
   unknown fields;
2. admission, exclusion, deletion, and audit tooling is tested;
3. the SleyBench public/private split and semantic-lineage ledger are pinned;
4. an operator approves the exact capture or import;
5. provider terms and source licenses permit the intended use.

Current implementation status:

- Complete: strict record, exclusion-ledger, manifest, audit-report,
  deletion-plan, audit-event input, chained audit-event, and mutation-audit
  report schemas exist and reject unknown fields.
- Complete: `sley-corpus admission-check` and `sley-corpus audit` validate
  provenance paths, content pins, admission evidence, exclusion coverage,
  benchmark collisions, manifest counts, release state, and split lineage.
- Complete: `sley-corpus deletion-plan` deterministically enumerates removal,
  revocation, and tombstone work without performing mutation.
- Complete: `sley-corpus audit-log` verifies append-only event sequence,
  digest chain, canonical content digests, chronology, identity, state
  continuity, and transition semantics. `append-event` writes only a confirmed
  governance event after locking and revalidating the complete existing log.
- Complete: `sleybench-split` separates public commitment checks from full
  private audits and rejects private evidence inside the public repository.
- Complete: the real 96 public and 24 private SleyBench cases, approved private
  custody, globally disjoint case IDs, and reviewed semantic-lineage ledgers
  pass the production full audit.
- Approved only for the 2026-08-23 K1/K3 baseline: local evaluation evidence
  capture with explicit retention and deletion ownership. Captured benchmark
  trajectories remain ineligible for training, retrieval, export, release, or
  public Git.
- Pending: exact operator approval for any corpus admission, export, release,
  unrelated capture, deletion, or audit-event append. No corpus-operation
  executor is authorized by this specification.

This specification authorizes no provider call by itself, model training,
external data collection, public release, or retention of an unrelated
conversation. The operator-approved baseline is a narrow execution exception,
not standing authority for later provider calls or corpus use.
`sleybench-split verify-public` does not satisfy precondition 3 because it does
not inspect private material. Only a passed full `sleybench-split audit` over
real public and private manifests can prove the split boundary, and that audit
still does not authorize capture.

## 4. Corpus Partitions

Every admitted record MUST belong to exactly one partition:

| Partition | Intended use | Benchmark access |
| --- | --- | --- |
| `train` | retrieval, supervised adaptation, or continued pretraining | MUST exclude all evaluation lineages |
| `development` | prompt, retrieval, and tool-loop development | MUST exclude private held-out lineages |
| `evaluation_public` | disclosed SleyBench development cases and results | MUST NOT be used to claim unexposed generalization |
| `evaluation_private` | private held-out prompts, fixtures, solutions, and tests | MUST NOT enter training, retrieval, public Git, or generation seeds |
| `quarantine` | unresolved provenance, privacy, license, quality, or contamination | MUST NOT be consumed by models or released |
| `rejected` | permanently ineligible material with a reason and tombstone | MUST NOT be restored without a new reviewed admission |

Partition assignment MUST occur by semantic lineage before individual records
are written. Renaming identifiers, reformatting source, translating a prompt,
or mutating constants does not create an independent lineage.

## 5. Record Contract

An implementation SHOULD use `sley.corpus.record.v0` for individual records
and `sley.corpus.manifest.v0` for an immutable corpus release. Each record MUST
contain at least:

- stable record, task, and semantic-lineage IDs;
- partition, visibility, lifecycle state, and admission timestamp;
- Sley, compiler, formatter, schema, and evaluator versions;
- origin type: human-authored, model-authored, synthetic, translated,
  downstream-derived, compiler-generated, or mixed;
- exact source URI or repository path and immutable source revision;
- author/owner authority, provenance chain, and collection method;
- license identifier, license evidence, intended-use decision, and reviewer;
- privacy class, sanitization method, secret-scan result, and redaction log;
- prompt, allowlisted context IDs, knowledge mode, and tool policy when a model
  interaction is present;
- model/provider identity, parameters, seed, and terms snapshot when applicable;
- original source, candidate, patch, diagnostics, tool events, tests, oracle
  result, and final verdict as applicable;
- exact content digests, normalized structural digest, and semantic-lineage
  digest;
- benchmark exclusion IDs and contamination decision;
- quality labels, known limitations, retention class, and deletion authority.

Large blobs MAY be content-addressed sidecars. The record MUST pin each sidecar
digest and media type. A missing or mismatched sidecar fails admission closed.

## 6. Provenance And Licensing

Each candidate MUST identify who created it, where it came from, how it was
obtained, and who has authority to admit it. “Found online,” model output
without terms evidence, and unknown repository origin are insufficient.

Admission rules are:

- Greyforge-authored public Sley material MAY be admitted when ownership and
  privacy checks pass.
- Third-party material MUST have an identified license compatible with the
  exact internal, redistribution, retrieval, or training use being proposed.
- Copyleft, attribution, notice, share-alike, non-commercial, dataset-specific,
  and model-output terms MUST be recorded and enforced, not flattened into a
  generic “open” label.
- Material with ambiguous ownership or incompatible terms goes to
  `quarantine` or `rejected`.
- Generated material MUST retain the generator identity, input provenance,
  provider terms snapshot, and human/automated verification evidence.
- Translation does not erase the source license or provenance chain.

Public corpus releases MUST include required notices and a machine-readable
license inventory. A source's public accessibility alone does not establish
permission for model training or redistribution.

## 7. Privacy And Secrets

Admitted records MUST contain only the minimum information needed for the
declared use. They MUST NOT contain credentials, tokens, private keys, personal
health or financial data, private messages, private endpoints, customer data,
unsanitized production payloads, proprietary prompts, or unrelated repository
content.

Sanitization MUST occur before reusable storage. The pipeline MUST run secret
and high-risk identifier scans and record their versions and results. Redaction
MUST preserve a typed placeholder and a redaction reason without preserving the
secret value in metadata, diffs, or logs.

Downstream product evidence MUST be allowlisted field by field. A sanitized
oracle input/output pair MAY be eligible; raw databases, request logs, user
profiles, and ambient worktree contents are not.

## 8. Collection And Trajectory Capture

Collection MUST be explicit, scoped, and reproducible. A capture plan MUST name:

- source repositories and owned paths;
- permitted record types and partitions;
- knowledge mode and tool allowlist;
- model/provider and terms basis, if any;
- resource and spend ceilings;
- privacy, license, and exclusion gates;
- retention duration and deletion owner;
- expected output and validation commands.

Model trajectories MUST record the complete allowed interaction needed to
interpret the result: task prompt, injected context IDs and digests, responses,
tool requests, schema-validated tool reports, candidate changes, compiler
diagnostics, independent oracles, and verdict. Hidden platform prompts,
provider internals, credentials, and unrelated conversation history MUST NOT be
captured.

Failed attempts MAY be valuable, but failure does not waive admission gates.
Rejected, timed-out, scope-violating, or hallucinated attempts MUST retain the
failure label and MUST NOT be presented as successful instruction examples.

## 9. Compiler Verification And Quality

Every source-bearing record MUST pin the compiler version and run the strongest
applicable deterministic checks. Depending on record type, these include:

- parse and canonical format;
- check and lint diagnostics;
- unit, property, or differential oracle;
- exact output comparison;
- project gates and scope precision;
- deterministic replay;
- regression and minimality checks.

Compiler acceptance is necessary but not sufficient. A record can compile and
still be unlicensed, private, contaminated, semantically wrong, non-idiomatic,
or misleading. Quality labels MUST distinguish syntax-valid, check-valid,
oracle-valid, human-reviewed, and independently replayed evidence.

When a language or compiler change invalidates old evidence, affected records
MUST be reverified or marked incompatible; they MUST NOT silently inherit the
new version label.

## 10. Synthetic And Translated Data

Synthetic generation SHOULD begin from coverage gaps, typed contracts, and
independent oracles rather than paraphrasing existing benchmark cases. A
synthetic record MUST identify its seed lineage and generator configuration.

Retention requires compiler verification plus an independent semantic oracle
when the task claims behavior. Self-consistency from the generating model is
not an independent oracle. Near-identical variants SHOULD be collapsed so
quantity does not masquerade as coverage.

Translations MUST preserve the source license and include executable or
manually reviewed equivalence evidence. Unsupported library behavior, guessed
Sley syntax, or unverifiable semantics fails admission.

## 11. Deduplication And Lineage

Admission MUST compute at least:

1. an exact byte digest;
2. a canonical-format digest for Sley source;
3. a normalized structural digest that ignores non-semantic naming and layout
   where safely possible;
4. a semantic-lineage ID spanning prompts, solutions, tests, translations, and
   generated variants.

Exact duplicates MUST be collapsed. Structural or semantic near-duplicates
MUST remain in one lineage and one split. Deduplication decisions and thresholds
MUST be versioned so a corpus release can be reproduced.

## 12. Benchmark Contamination Controls

SleyBench exclusion IDs are denylist inputs to corpus admission, retrieval
indexing, synthetic generation, and fine-tuning export. Private held-out
prompts, workspaces, solutions, tests, oracle details, semantic variants, and
tool traces MUST remain outside public Git and all consumable corpora.

Before each corpus release or model run, the pipeline MUST compare exact,
structural, and semantic-lineage digests against every evaluation partition.
Suspected overlap goes to `quarantine`. If contamination is discovered after a
run, the result MUST be labeled and reported separately; it MUST NOT be silently
removed to improve a score.

## 13. Retention, Deletion, And Audit

Raw captures SHOULD be ephemeral. Approved normalized records MAY use a longer
retention class only after admission. Each class MUST specify duration,
location, access boundary, backup behavior, and deletion owner.

Deletion MUST remove or tombstone the record, sidecars, derived retrieval
entries, and future export eligibility. Immutable releases MAY retain a
non-sensitive tombstone containing the record ID, digests, removal date, and
reason. If a source license, consent, privacy, or provider-terms decision
changes, affected lineages MUST be traceable and removable.

Every admission, reclassification, export, release, quarantine decision, and
deletion MUST produce an append-only audit event without embedding removed
sensitive content.

## 14. Release And Versioning

Corpus releases MUST be immutable and content-addressed. A release manifest
MUST pin:

- corpus and schema versions;
- Sley/compiler compatibility range;
- partition counts and lineage counts;
- source/license/privacy summaries;
- verification and deduplication tool versions;
- exclusion-ledger digest;
- every record or shard digest;
- known limitations and removal history;
- intended uses and prohibited uses.

Adding, removing, relabeling, re-splitting, or re-verifying a record creates a
new corpus version. Training and evaluation reports MUST name the exact corpus
release; a mutable directory name is not adequate evidence.

## 15. Admission Gates

A candidate is reusable only when all applicable gates pass:

1. authority and immutable provenance are known;
2. license permits the exact intended use;
3. privacy minimization, sanitization, and secret scanning pass;
4. record and sidecars validate against pinned schemas and digests;
5. compiler and independent semantic checks pass at the claimed quality level;
6. exact, structural, and semantic deduplication completes;
7. benchmark exclusions and split-lineage checks pass;
8. retention and deletion owners are assigned;
9. a reviewer approves admission to a non-quarantine partition.

A failed or unavailable gate fails closed. The material remains ephemeral,
enters `quarantine`, or is rejected with a reason.

## 16. Deterministic Tooling

The local `sley-corpus` companion CLI implements read-only governance checks
and one confirmation-gated append-only audit surface:

```bash
sley-corpus admission-check \
  --record fixtures/corpus_governance/record.json \
  --exclusions fixtures/corpus_governance/exclusions.json \
  --root fixtures/corpus_governance \
  --json

sley-corpus audit \
  --manifest fixtures/corpus_governance/manifest.json \
  --root fixtures/corpus_governance \
  --json

sley-corpus deletion-plan \
  --record fixtures/corpus_governance/record.json \
  --requested-at 2026-08-23T00:00:00Z \
  --requested-by sley-maintainers \
  --reason reviewed-removal-reason \
  --json

sley-corpus audit-log \
  --log <audit-log.jsonl> \
  --log-id <logical-log-id> \
  --json

sley-corpus append-event \
  --log <audit-log.jsonl> \
  --event <strict-event-input.json> \
  --log-id <logical-log-id> \
  --expected-head <sha256-digest-or-none> \
  --confirm-append-only-audit-event \
  --json
```

These commands make no provider or network call. `append-event` is the only
write surface, requires an explicit confirmation flag and expected head, uses
an append lock, and records identifiers, state transitions, approvals, and
digests without raw corpus content. It does not collect, admit, retain, export,
release, delete, or tombstone corpus data. The other commands are read-only. A
passed report is eligibility or audit evidence only, not authority to perform
the consequential corpus action. Training or development admission also fails
when the referenced exclusion ledger does not declare a pinned benchmark
split.

## 17. Anti-Goals

This specification does not authorize:

- bulk scraping or importing third-party code because it is accessible;
- retaining all agent sessions by default;
- using benchmark solutions as training examples;
- treating compiler success as proof of license, privacy, or semantic quality;
- hiding failed trajectories or contaminated evaluations;
- publishing private held-out material;
- training a model before SleyBench baselines establish a comparison point;
- weakening Greyforge approval gates for provider spend, public release, or
  external data use.
