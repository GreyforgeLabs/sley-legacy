# Sley Prior-Art Source Pack

Status: public comparison source pack
Last verified: 2026-08-22

## Purpose

This packet turns the working comparison note behind the current Sley review
into an auditable public source pack. It is not a global proof that no other
language exists. It is the current primary-source basis for comparing Sley with
the candidate projects named in the review note and for deciding which public
claims are safe.

Use this packet with `docs/SleyClaimEvidence.md`. Do not use it to post public
issues, public comments, release copy, or title claims until the exact outgoing
text is operator-approved.

## Sley Criteria Used Here

Sley's comparison category is intentionally narrow:

1. Independent language substrate: source form, semantic model, checked command
   surface, and runtime or execution contract.
2. Compiler-owned structural model: program structure is exposed through
   first-class machine contracts, not inferred from raw text.
3. Native agent edit path: inspection, diagnostics, planned repairs, structural
   grafts, verification, traces, and seals are compiler-mediated surfaces.
4. Deterministic authority gates: host effects are explicit, bounded, and
   reproducible without live network, shell, secret-store, deployment, payment,
   spend, wallet, market, or provider calls.
5. Independent auditability: a clean public checkout includes exact commands
   that verify the claimed stage without private infrastructure.

This category is stricter than ordinary "AI-native" marketing. A project can be
AI-native, AI-focused, or useful for agents while still failing Sley's
"agent-native structural programming" criteria.

## Summary Matrix

| Project | Official self-description | Strong fit | Sley-criteria gap | Classification |
| --- | --- | --- | --- | --- |
| Sley | Self-hosted, agent-native structural programming for compiler-mediated, human-reviewed software change. | Sley-owned parser, checker, lint, runtime, bootstrap, and report semantics; compiler-mediated JSON reports; checked graft previews; deterministic gates; public v1 gate. | No global-firstness claim is made by this packet. | Self-hosted agent-native structural language with a reproducible public proof surface. |
| Dana | Adaptive programming language focused on runtime component hot-swapping. | Independent language and strong runtime adaptation model. | Official docs center live component composition/hot-swap, not LLM-native compiler-mediated structural editing. | Adaptive/runtime-native language, not proven agent-native structural language. |
| Jac | AI-native full-stack language with Meaning Typed Programming, Object-Spatial Programming, and multi-target compilation. | Strong AI language claim; LLMs are first-class through `by llm()`/MTP; native codespace exists. | Official docs center AI app/full-stack semantics and ecosystem interop, not a compiler-owned agent edit/graft/seal path. | Serious AI-native language, but not the same category as Sley's structural edit contract. |
| Codong | Claims "world's first AI native programming language" and one correct way to write everything. | Explicit AI-native positioning, independent `.cod` syntax, AI-oriented spec, structured errors. | Official public proof centers token savings, reduced choice, and Go-backed implementation/runtime requirements, not compiler-mediated structural edits with deterministic authority gates. | AI-oriented/AI-native claimant; not shown to satisfy current Sley structural criteria. |
| Codon | High-performance Python implementation/compiler. | Independent compiler frontend and LLVM backend for Python-like code. | Official goals are performance, Python compatibility, hardware support, and Python ecosystem interop; not an AI-native language claimant in the reviewed sense. | Name-collision note; not the Codong claimant. |
| Agentis | TypeScript framework/toolkit for autonomous AI agents. | Real agent orchestration features: memory, planning, swarms, tools, platform connectors. | No independent programming-language substrate; it is a framework over TypeScript/JavaScript and LLM providers. | Agent framework, not a programming language in this comparison category. |
| Mojo | AI-native systems language for high-performance AI infrastructure and heterogeneous hardware. | Strong compiler and AI-infrastructure story; MLIR foundation; Python interop; agentic-programming positioning. | Official docs center performance-portable compute and AI kernel development, not compiler-mediated agent editing of application source with structural grafts/seals. | AI-infrastructure/system language, not Sley-style agent-native structural language. |

## Source Notes

### Sley

Primary local/public evidence:

- `README.md`
- `llms.txt`
- `docs/SleyClaimEvidence.md`
- `scripts/check-self-hosted-code.sh`
- `scripts/self-hosted-test.sh`
- `Makefile`
- `docs/PublicReleaseChecklist.md`

Current public audit commands:

```bash
export PATH="$(pwd)/bin:$PATH"
./scripts/check-self-hosted-code.sh
scripts/self-hosted-test.sh
make v1
bin/sley self-hosting-status --json
make public-release-check
```

Current classification: Sley currently has the strongest public fit among the
reviewed candidates for the agent-native structural category because the proof
surface is a self-hosted local compiler and contract suite rather than a prompt,
framework, or hardware-performance claim. The exact reproducible release gates
are recorded in `docs/PublicReleaseChecklist.md`.

### Dana

Official sources:

- https://projectdana.com/
- https://www.projectdana.com/dana/api/pacdoc/composition/

Observed from official sources:

- The homepage describes Dana as an adaptive programming language that can
  hot-swap components in microseconds, update programs without restarting them,
  and support live code changes.
- The composition package documentation describes runtime component loading,
  dependency inspection/wiring, and hot-swapping by re-wiring dependencies.

Sley comparison:

- Dana appears to satisfy the independent-language and runtime-adaptation parts
  of the broad language test.
- The current official public materials found for this packet do not show an
  LLM-native or agent-native compiler edit path equivalent to Sley's
  inspection/diagnostic/graft/verify/trace/seal command surface.

Safe public wording: Dana is a strong adaptive/runtime language. Under Sley's
criteria, it is not yet documented as an agent-native structural programming
language.

### Jac

Official sources:

- https://docs.jaseci.org/reference/language/
- https://docs.jaseci.org/reference/language/foundation/
- https://docs.jaseci.org/reference/language/native-pathway/
- https://docs.jaseci.org/reference/plugins/byllm/

Observed from official sources:

- Jac describes itself as an AI-native full-stack programming language.
- Jac's foundation docs say it has Python-like syntax and compiles to Python
  bytecode, JavaScript, and native machine code.
- Jac lists LLMs as first-class citizens through Meaning Typed Programming.
- Jac's native pathway docs say `.na.jac` files can compile to self-contained
  binaries without the Python runtime for that native codespace.

Sley comparison:

- Jac is a serious AI-native language claimant and should not be dismissed as
  merely a wrapper.
- Jac still differs from Sley's claim category: the official docs found here
  emphasize full-stack AI development, Object-Spatial Programming, MTP, byLLM,
  multi-target compilation, and ecosystem access. They do not present the same
  compiler-mediated agent edit contract with structural graft previews,
  deterministic authority gates, and trace/seal proof surfaces.

Safe public wording: Jac is AI-native by its own language-design definition.
Under Sley's stricter "agent-native structural programming" criteria, the
missing evidence is not AI integration; it is Sley-style structural edit
mediation and deterministic proof commands.

### Codong

Official sources:

- https://github.com/brettinhere/Codong
- https://raw.githubusercontent.com/brettinhere/Codong/main/README.md
- https://raw.githubusercontent.com/brettinhere/Codong/main/SPEC.md
- https://raw.githubusercontent.com/brettinhere/Codong/main/SPEC_FOR_AI.md

Observed from official sources:

- The repository description says: "AI native programming language one correct
  way to write everything."
- The README claims "The world's first AI native programming language."
- The README frames Codong around AI writing, human review, token savings,
  reduced framework choice, structured JSON errors, and bundled modules.
- `SPEC.md` defines a `.cod` language specification with syntax, types,
  functions, modules, concurrency, and infrastructure modules.
- The README states that `codong run` and `codong build` require Go 1.22+.
- GitHub language metadata reported Go as the dominant implementation language
  when checked on 2026-05-11.

Sley comparison:

- Codong is the closest direct public title claimant in this source pack.
- The official materials support a claim that Codong is designed to be easy for
  LLMs to generate and cheaper in token/context terms.
- The official materials found here do not establish Sley-style
  compiler-mediated structural editing, checked graft previews, deterministic
  authority gates, trace/seal proof surfaces, or a foreign-language-free
  public bootstrap gate.

Safe public wording: Codong is an AI-native claimant focused on one-correct-way
syntax and token reduction. Under Sley's criteria, the missing evidence is a
compiler-owned structural edit/audit path, not merely the phrase "AI native."

### Codon

Official sources:

- https://docs.exaloop.io/
- https://docs.exaloop.io/language/overview/
- https://docs.exaloop.io/developers/compilation/
- https://github.com/exaloop/codon

Observed from official sources:

- Codon is described as a high-performance Python implementation that compiles
  to native machine code.
- The docs say Codon strives to stay close to CPython syntax, semantics, and
  libraries outside performance/static-compilation differences.
- The compilation docs describe a custom frontend, AST, type checker, Codon IR,
  and LLVM backend.

Sley comparison:

- Codon is not the Codong project referenced by the review note.
- Codon has serious compiler technology, but the official public position is
  performance and Python acceleration, not AI-native structural editing.

Safe public wording: Codon should not be treated as the "world's first AI native"
competitor unless a source specifically identifies it that way. The direct
claimant is Codong.

### Agentis

Official sources:

- https://www.agentislabs.ai/
- https://www.agentislabs.ai/blog/agentis
- https://github.com/AgentisLabs/agentis-framework
- https://raw.githubusercontent.com/AgentisLabs/agentis-framework/main/README.md

Observed from official sources:

- The Agentis site describes a CLI agent, an Omni agent, and an Agentis
  Framework for coordinated agent swarms, memory systems, and integrations.
- The GitHub repository describes Agentis Framework as a TypeScript framework
  for autonomous AI agents with memory, planning, tool usage, task
  decomposition, goal-based reasoning, and integrations.
- GitHub language metadata reported TypeScript and JavaScript when checked on
  2026-05-11.

Sley comparison:

- Agentis is relevant to agent systems, but the official materials show a
  framework/toolkit rather than an independent programming language substrate.
- It therefore is not shown to satisfy Sley's first criterion before reaching
  the structural edit criteria.

Safe public wording: Agentis is an AI agent framework, not a programming
language in Sley's comparison category.

### Mojo

Official sources:

- https://mojolang.org/
- https://mojolang.org/docs/vision/
- https://mojolang.org/docs/manual/
- https://mojolang.org/docs/manual/python/

Observed from official sources:

- The Mojo homepage says Mojo is "AI native" and built for performance on the
  diverse hardware that powers modern AI systems.
- Mojo's vision says it targets diverse hardware using Python's intuitive syntax
  with modern systems programming capabilities.
- Mojo's Python interoperability docs state a plan for full compatibility with
  the Python ecosystem and describe importing Python modules through CPython.
- The vision docs explain Mojo's MLIR-based compiler foundation and its focus
  on accelerator and kernel development.

Sley comparison:

- Mojo is a strong AI-infrastructure language and should be described with
  precision.
- Its official AI-native claim is about high-performance AI systems,
  heterogeneous hardware, and agentic programming suitability. The source set
  found here does not show Sley's narrower compiler-mediated agent edit path:
  structural source inspection, graft previews, deterministic authority gates,
  local verification, trace, and seal surfaces for human-reviewed software
  change.

Safe public wording: Mojo is AI-native as an AI-infrastructure/system language.
Under Sley's criteria, it is a different category than agent-native structural
programming.

## Public Issue Gate

Public issues on competitor repositories are not authorized by this packet
alone. Before opening any issue, prepare the exact issue body and get operator
approval for:

- target repository;
- issue title;
- issue body;
- links included;
- tone and requested maintainer action.

The safe framing is a classification question, not an accusation:

```text
I am maintaining a public comparison source pack for Sley, an agent-native
structural programming language. Your project appears to satisfy X and Y from
your official docs, but I do not yet see evidence for Z under the comparison
criteria. Is there an official source I should cite or a correction you want me
to make?
```

Do not post broad authenticity attacks as standalone claims. They are too broad
and weaker than the evidence. Use the narrower, auditable wording:

```text
I do not yet see public evidence that this project satisfies Sley's
agent-native structural programming criteria: compiler-owned structural edit
contracts, checked graft previews, deterministic authority gates, and local
trace/seal proof commands.
```

## Current Confidence

- High confidence: Agentis is a framework, not a language substrate.
- High confidence: Codon is not the Codong claimant.
- High confidence: Dana's public proof is runtime/adaptation-centered.
- High confidence: Mojo's public AI-native claim is hardware/infrastructure and
  performance-centered.
- Moderate confidence: Jac does not expose Sley-equivalent structural
  edit/graft/seal proof surfaces in the official docs reviewed here.
- Moderate confidence: Codong is the closest direct public "world's first AI
  native programming language" title claimant in this packet.
- Low confidence: global "first in the world" is fully proven. This packet is a
  primary-source comparison against named candidates, not a complete census of
  every language project.
