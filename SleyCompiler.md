# Sley Compiler Notes

Date: 2026-05-05
Status: working design note
Scope: compiler difficulty, Rust bootstrap rationale, self-hosting path

## Short Answer

Writing our own compiler is possible. It is not mystical.

A toy compiler is fairly reachable. A trustworthy compiler for an agent-native,
agent-writable language is much harder because it must be a strict referee, not
just a translator.

The goal is not merely:

```text
source text -> executable behavior
```

The real Sley compiler goal is:

```text
source projection -> typed graph -> checked grafts -> stable diagnostics ->
human review -> safe execution
```

That extra responsibility is where the difficulty lives.

## Explain It Simply

A simple compiler is like a referee for tic-tac-toe. It only needs to know a few
rules.

Sley's target compiler is more like a referee for a whole legal system where
agents are allowed to propose edits, refactors, migrations, and eventually new
compiler pieces.

That referee must be boring, strict, consistent, and hard to trick.

## What Is Easy Enough

These parts are normal compiler work and are already in the Sley v0 direction:

- parse simple Sley syntax
- build an AST or typed graph
- format source consistently
- check basic types
- check simple return values
- evaluate explicit `Ok(value)`/`Err(error)` result values and `?`
  propagation for Sley-level results
- run pure zero-take `main`
- expose JSON diagnostics
- expose AST or graph JSON
- apply simple structural grafts

This is not trivial, but it is tractable.

## What Is Actually Hard

The hard parts are the parts that make Sley useful for agents instead of just
humans typing source text.

Hard compiler requirements:

- stable graph node identities
- lossless or near-lossless source projection
- formatter round trips
- precise diagnostics with stable IDs
- repair hints that agents can use
- module namespace resolution
- import aliases and exported names
- type checking across modules with semantic type identity
- effect and capability propagation across module boundaries
- runtime gate values for host authority
- rejection of unauthorized side effects
- stale graft rejection
- structural migrations
- provenance trace receipts, local trace sidecars, and content-addressed seals
- ZJX graph/graft/trace envelopes
- compatibility across compiler versions
- conformance tests for both accepted and rejected programs

These are hard because agents will push directly on the boundaries. A loose
compiler lets bad edits through. A vague compiler gives agents useless feedback.

## Why Rust First

Rust is not the soul of Sley. Rust is the first referee.

The Rust Loom gives Sley:

- a stable bootstrap implementation
- memory safety
- mature parsing/checking/runtime tooling
- a known ecosystem
- good test infrastructure
- predictable binaries
- a recovery oracle while Sley is young

This does not mean Sley "lives in Rust" forever.

Correct model:

```text
Sley semantics -> defined by spec, graph model, tests, examples, diagnostics
Rust Loom -> first executable implementation of those semantics
Future Sley passes -> self-hosted implementation pieces
```

The language lives in the semantics. Rust is the first implementation.

## Why Not Self-Host Immediately

Immediate self-hosting is attractive but unsafe.

If Sley rewrites its own compiler before the rules are stable, then bad rules,
bad diagnostics, or bad graph encodings can become self-reinforcing. The system
could learn to pass its own weak tests instead of becoming correct.

Self-hosting should be earned, not assumed.

## Safe Self-Hosting Ladder

Recommended path:

1. **Rust-only oracle**
   Rust parses, checks, formats, runs, and applies grafts.

2. **Sley examples and standard library**
   Real Sley code is validated by Rust.

3. **Sley helper tools**
   Write non-authoritative helpers in Sley: lints, migration suggestions, docs
   examples, and fixture generators. Rust now exposes checked query and lint
   reports for those helpers to consume.

4. **Shadow compiler passes**
   Write a Sley version of a compiler pass, but run it beside the Rust pass.
   It has no authority yet.

5. **Conformance matching**
   The Sley pass must match Rust across a large suite of accepted and rejected
   examples.

6. **Promoted pass**
   One narrow Sley pass becomes authoritative.

7. **Self-hosted core**
   Sley eventually builds and evolves large compiler pieces.

8. **Recovery oracle remains**
   Keep a small Rust or frozen reference implementation for recovery and
   compatibility checks.

## Training Data Problem

The training-data problem is not solved by Rust.

Rust helps because Codex already knows Rust, so Codex can help build the first
Loom. But Sley itself still needs agent-readable affordances:

- compact docs
- examples
- non-destructive project scaffolds
- JSON AST or graph output
- stable diagnostics
- repair hints
- graft schemas
- accepted/rejected fixtures
- formatter output
- conformance tests

The strategy is to make Codex learn Sley through tools instead of relying on
pretraining.

Codex does not need to be born knowing Sley. It needs a tight loop:

```text
inspect graph -> propose graft -> check -> read diagnostics -> repair -> repeat
```

## Compiler Quality Bar

For Sley, the compiler is not just implementation plumbing. It is the safety
boundary for agent-written software.

A Sley compiler must:

- reject malformed syntax
- reject unknown names
- reject type mismatches
- reject unauthorized effects
- reject stale grafts
- explain rejections clearly
- preserve human-reviewable output
- keep stable machine-readable structure
- produce evidence that a change was accepted for the right reason

The compiler should be stricter than a normal early hobby language compiler
because Sley's primary editor may be an agent.

## Practical Next Compiler Milestones

Near-term:

1. Keep expanding parser/checker/runtime coverage in Rust.
2. Harden capability-backed host adapters on top of typed fallibility:
   database write has the first deterministic `db.try_insert` slice, secrets
   have deterministic seeded `secrets.try_get`, deploy has deterministic seeded
   `deploy.try_stage`, spend has deterministic seeded
   `spend.try_authorize`, network has deterministic seeded
   `http.try_get_text`, shell has deterministic seeded `shell.try_run`, and
   model calls have deterministic seeded `model.try_complete`.
3. Keep expanding the manifest-backed accepted/rejected gold corpus and CLI
   smoke conformance suite. They now cover runtime authority for the seeded
   host adapter surface, stable JSON roots, project scaffolding, graph/ZJX
   output, graft dry runs, checked graph query reports, and private-task lint
   reports.
4. Consume `sley.query.report.v0` and `sley.lint.report.v0` from helper
   passes. `sley doctor` is the first deterministic readiness helper on top of
   those surfaces, and `sley plan --graft-templates` now turns them into ranked
   edit surfaces plus starter graft operation payloads, rename-plus-call-site
   transactions, add-take-plus-call-arg transactions, and safe
   remove-take-plus-call-arg transactions for unused takes. Next broaden
   authority, style, and migration lints.
5. Extend graph-slice graft planning around checked move/delete operations.
6. Harden project graft writeback beyond existing-module edits.

Medium-term:

1. Extend graph-slice grafts beyond call-site, statement, expression, move, and
   delete edits.
2. Move trace seals into a compressed binary `.zjx` handoff.
3. Grow capability-backed runtime host values beyond seeded v0 adapters:
   deploy beyond seeded stage results, spend beyond seeded authorization text,
   secret beyond seeded values, model beyond seeded completions, shell beyond
   seeded command output, network beyond seeded text, and database write beyond
   per-run inserts.
4. Move selected Sley lint/helper passes onto the checked graph query and lint
   report surfaces.

Long-term:

1. Write Sley lint/helper passes.
2. Shadow Rust passes with Sley equivalents.
3. Promote self-hosted passes after conformance evidence.
4. Preserve a recovery oracle.

## Bottom Line

We can write our own compiler.

The simple version is not the hard part. The hard part is making the compiler
strict, stable, explainable, and safe enough for agents to use as their main
editing surface.

Rust first is not surrender. It is scaffolding.

The path is:

```text
Rust bootstrap -> strict Sley compiler -> agent graft loop -> gold corpus ->
Sley helper passes -> shadow self-hosted passes -> promoted self-hosting
```

That path lets Sley grow without asking an immature language to protect itself
too early.
