# Sley First for Sley 1.2

Status: active bounded doctrine
Owner: Sley maintainers
Review trigger: candidate assessment or promotion proposal

Sley First means **evaluate Sley first where its semantic strengths fit**. It
does not mean Sley Everywhere and does not grant migration or production
authority.

Prefer a Sley candidate when the workload is deterministic, structured,
rule/validation/transformation heavy, frequently agent-maintained, bounded by
an implementation-neutral contract, backed by an independent oracle, and
improved by explicit provenance or receipts.

Prefer the incumbent when the workload is framework-bound, ecosystem-heavy,
interactive-latency critical, a native/SIMD kernel, a cryptographic primitive,
or lacks stable semantics and an independent oracle.

Every candidate follows:

```text
identified -> assessed -> contracted -> candidate
  -> differential parity -> non-authoritative shadow -> promotion review
```

Valid terminal states include `deferred`, `rejected`, `capability_blocked`, and
`rolled_back`. These are evidence, not failures of the program.

For 1.2, the existing Siglum numerology candidate is the default operational
reference. It has exact deterministic parity evidence but no production
authority, latency gate, exercised fallback, production shadow, or promotion.
No additional Siglum domain or Greyforge-wide rollout is authorized by this
document.

Promotion requires independent correctness, runtime, operability, security,
maintainability, and agent-efficiency evidence. Internal use alone proves only
that Sley can be used.
