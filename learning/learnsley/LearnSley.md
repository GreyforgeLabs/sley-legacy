# LearnSley Game Spec

Date: 2026-05-05
Status: v0 product/game design plus implementation audit
Scope: a gamified Sley learning program for humans, built around the Loom compiler, Sley source, graph inspection, grafts, traces, and seals

## Thesis

LearnSley is the Boot.dev-style learning game for Sley, but the core fantasy is not "learn generic coding with a Sley skin." The core fantasy is:

> Become a human who can read, write, debug, verify, and safely evolve Sley programs with the Loom as referee.

Sley's differentiator is structural programming for humans and agents. The learning game must therefore teach syntax, but it must also teach graph literacy, authority discipline, diagnostics, typed effects, result flow, and graft-first edits. A good player should finish the game able to write real `.sley` source, understand why the compiler accepted or rejected it, inspect graph shards, and review agent-proposed changes without being fooled by plausible text.

LearnSley copies and adapts proven mechanics from Boot.dev and adjacent products, but it must not copy proprietary lesson text, exact challenge content, art, characters, brand names, or course ordering. Game mechanics are adapted into original Sley-native equivalents.

## Design Pillars

1. Write code constantly.
   Reading-only progress is intentionally low-value. Every concept is followed by a real Sley edit, run, diagnostic repair, graph inspection, or explanation challenge.

2. The compiler is the game referee.
   `sley check`, `sley run`, `sley ast`, `sley graph`, `sley graft`, `sley trace`, and `sley seal` are first-class game actions, not hidden implementation details.

3. Reward safe engineering habits.
   Running before sealing, reading diagnostics, using gates deliberately, and writing small verified changes should be rewarded more than guessing.

4. Teach Sley as Sley.
   The course must not smuggle in JavaScript/Python mental models. Sley has `task`, `take`, `bind`, `state`, `tally`, `slot`, `forge`, `uses`, gates, `Result`, `?`, graph shards, grafts, traces, and seals. Those are the language's grammar of thought.

5. Competition is opt-in and mastery-weighted.
   Leaderboards, leagues, and streaks motivate some learners and distort others. LearnSley keeps them available, but scores verified new mastery above repeat grinding.

6. AI helps without replacing thinking.
   The mentor can explain diagnostics, ask questions, and offer hint ladders. It should not dump final code unless the learner deliberately spends a solution item after a real attempt.

## Competitive Mechanics Adapted

| Source pattern | What works | LearnSley adaptation |
|---|---|---|
| Boot.dev structured path, lessons, projects, XP, levels, achievements, leaderboard, community, guilds, local CLI, Run vs Submit, failure armor, sharpshooter sprees, chests, gems, items, AI mentor, interview lessons, Training Grounds | Code-first progression with real consequences for premature submit and a strong daily loop | `Run` is free; `Seal` is final. Failed seals consume Proof Armor or break a Clean Seal Chain. Cache Chests, Shards, Lenses, Trace Keys, mentor interviews, Training Grounds, guild workshops, and portfolio seals are Sley-native |
| CodeCombat real code controlling a visible level, unlockable avatars/items, shareable custom levels, AI hints | Immediate visual feedback makes code feel active | Graph maps, runtime traces, visual module paths, and challenge sandboxes where Sley code moves data through gates and typed graphs |
| Codecademy interactive editor, paths, quizzes, projects, AI assistant, tooltips | Low-friction beginner flow | Browser editor plus local CLI mirror, inline docs, hoverable Sley terms, guided paths, milestone projects |
| Exercism tracks, tests unlock more exercises, mentoring, solution comparison, reputation | Fluency through practice and human review | Sley tracks by concept, mentor review after bosses, compare accepted solutions by graph shape and readability, community reputation for useful help |
| Duolingo streaks, quests, leagues, achievements, personal records, opt-out competition | Strong retention loop, but can distort learning | Daily Pulse, weekly Gates, opt-in leagues, personal records, streak freezes, anti-farm XP rules |
| HackerRank and LeetCode weekly challenges, contests, tags, leaderboard resets, winning solution review | Recurring competitive practice | Weekly Seal Trials, diagnostic repair races, graph-efficiency contests, previous winner reviews |
| freeCodeCamp projects and certifications | Public proof of completed work | Sley certificates require capstone projects, tests, graph inspection, and sealed trace evidence |

Source notes are listed at the end of this document.

## Build Decision

LearnSley should be a web app experience, but the first implementation should
be a Linux-native local repo with a localhost web server that runs the real
Sley compiler. This is the honest Boot.dev-like shape for Sley v0:

1. browser UI for lessons, editor, output, diagnostics, graph view, rewards,
   and progress;
2. local host process for `sley check`, `sley run`, `sley format`,
   `sley graph`, and `sley seal`;
3. content stored as versioned manifests and `.sley` examples;
4. no public arbitrary-code execution until the sandbox model is explicit.

The public target can still be `<learnsley-public-site>`, but public hosting
should initially mean a marketing/demo shell or a browser-only preview unless a
server sandbox exists. A hosted grader that executes user Sley code is a real
runtime surface, not a static page.

Current deployment posture on 2026-05-05:

1. `<learnsley-public-site>` resolves through the existing domain DNS posture,
   but no LearnSley app route is implemented yet;
2. the existing web surface is a separate site repo under
   `<web-surface-repo>`;
3. the safest first build target is `<learnsley-local-repo>`, a standalone
   local repo that can later export a static demo or be integrated into
   WebForge;
4. production hosting requires a named deploy step and live verification.

The implementation rule is:

```text
local first -> compiler-backed MVP -> sandbox design -> static demo -> <learnsley-public-site>
```

## Spec Audit

The game design is strong enough to begin implementation. It already specifies
the core loop, vocabulary, progression, campaign sequence, lesson anatomy,
evaluation dimensions, interface panels, retention loops, social mechanics,
mentor behavior, MVP scope, metrics, risks, and first trial examples.

The main missing pieces before this audit were operational:

| Gap | Decision |
|---|---|
| Is LearnSley a web app or local app? | Web app UX, local Linux runtime first |
| Where does it live first? | Standalone `<learnsley-local-repo>` repo |
| Can it be hosted immediately? | Static demo yes; real Run/Seal grader only after sandbox |
| What is the first executable slice? | Local server, five trials, real `sley` subprocess runner, local progress |
| How are arbitrary submissions contained? | Bind to `127.0.0.1`, cap code size, write to temp workspace, no shell spawning, no default effects |
| How does it stay useful while Sley evolves? | Trial manifests declare supported compiler commands and concept tags |
| What makes it more than docs? | Run, Seal, diagnostics, graph, rewards, chain state, and progress persistence |

Build-grade MVP acceptance criteria:

1. `npm start` launches a localhost-only app without downloading packages;
2. the first trial can be completed by editing Sley and pressing `Run`;
3. `Seal` calls the same compiler path and updates progress only on success;
4. diagnostics from `sley check` are visible without being rewritten into vague
   lesson prose;
5. the graph panel displays `sley graph --json` output for valid code;
6. progress survives a browser refresh;
7. the app can be moved to a hosted shell later without changing lesson
   manifests.

## Product Shape

LearnSley has three product layers:

1. Web game
   The first-class beginner experience: map, lesson text, editor, run output, test panel, graph panel, hints, rewards, profile, guilds, and leaderboards.

2. Local CLI companion
   A `learnsley` wrapper around the real `sley` binary. It runs lesson tests locally, streams results back to the web UI, and teaches the actual developer workflow.

3. Sley-authored course/runtime layer
   Lesson manifests, examples, validators, reward rules, and some game logic are written in Sley as soon as the v0 surface allows it. Until full self-hosting, thin Rust/TypeScript wrappers may host the UI and sandbox, but the canonical learning artifacts should be Sley source plus machine-readable manifests.

## Core Loop

The base loop is:

1. Read one small concept card.
2. Edit a real Sley program.
3. Press `Run`.
4. Inspect output, tests, diagnostics, and optionally graph.
5. Repair until clean.
6. Press `Seal`.
7. Earn XP, shards, chain progress, mastery credit, and possible chest.
8. Unlock next lesson, challenge, or boss.

`Run` never penalizes. `Seal` is equivalent to submitting a reviewed change. If a seal fails, the player loses one Proof Armor if available. If no armor is available, the player's Clean Seal Chain resets.

The intended lesson habit is:

```text
edit -> run -> inspect -> repair -> run -> seal
```

The game should make this habit feel natural long before it teaches large programs.

## Game Vocabulary

| Generic term | LearnSley term | Meaning |
|---|---|---|
| Course | Campaign | A coherent Sley subject area |
| Chapter | Arc | A sequence of related concepts |
| Lesson | Trial | A short interactive unit |
| Exercise | Graft | A required edit or repair |
| Submit | Seal | Final graded attempt |
| XP | Loom XP | Progress currency for levels |
| Gems | Shards | Spendable game currency |
| Armor | Proof Armor | Protects chain from a failed seal |
| Streak | Daily Pulse | Consecutive active learning days |
| Sharpshooter spree | Clean Seal Chain | Consecutive passed seals after at least one run |
| Chest | Cache Chest | Randomized reward bundle |
| Boss | Sentinel Trial | Capstone challenge for an arc |
| Achievement | Sealmark | Permanent badge |
| Training mode | Training Grounds | Infinite targeted practice |
| AI tutor | Loom Guide | Socratic mentor and diagnostic translator |
| Guild | Workshop | Small learner group for quests, help, and projects |
| Community karma | Fellowship Credit | Reputation for useful help |
| Portfolio project | Public Seal | Sealed project artifact with trace proof |

## Progression System

### Levels

Players earn Loom XP for verified work. Levels unlock cosmetics, harder Training Grounds drills, project tracks, and optional competitive modes. Levels should not unlock core syntax; the main curriculum remains open in order.

Base XP:

| Activity | XP |
|---|---:|
| Complete a short trial | 10-25 |
| Complete a diagnostic repair trial | 20-35 |
| Complete a graph inspection trial | 25-40 |
| Complete a Sentinel Trial | 100-250 |
| Complete a public project seal | 300-700 |
| Explain a concept in an interview trial | 25-60 |
| Help another learner with accepted answer | 10-50 Fellowship Credit plus small XP |

No XP is awarded for repeating the exact same solved trial unless it appears in spaced review or Training Grounds with changed inputs.

### Mastery

Every trial has concept tags:

```text
module, task, take, bind, state, tally, slot, list, map, if, while, each,
Result, Error, question-mark, effects, gates, FileRead, DatabaseRead,
DatabaseWrite, import, export, graph, graft, trace, seal, ZJX
```

Each tag has a mastery score from 0 to 100:

```text
mastery_delta = difficulty * correctness * freshness * explanation_bonus
```

Progression gates depend on mastery, not only XP. A learner cannot brute-force a late module by farming easy XP.

### Clean Seal Chain

A chain increments when:

1. the learner has pressed `Run` at least once since the last source edit;
2. all visible tests pass;
3. `Seal` passes; and
4. the lesson is new or due for spaced review.

Every 15 clean seals awards a Cache Chest. Sentinel Trials can grant rare chests.

Failed seals:

1. consume Proof Armor if present;
2. otherwise reset the Clean Seal Chain;
3. never remove core progress already earned.

This is adapted from Boot.dev's Run vs Submit and failure protection loop, but renamed and retuned around Sley's `seal` concept.

### Daily Pulse

Daily Pulse increments when the player completes any meaningful learning action:

```text
one new trial OR one spaced review set OR one Training Grounds drill OR one accepted help answer
```

It does not increment for merely opening the app.

Pulse Freeze items can protect a missed day. They are capped so the streak stays motivational rather than coercive.

### Cache Chests

Cache Chests drop from:

1. daily quests;
2. Clean Seal Chains;
3. Sentinel Trials;
4. weekly events;
5. workshop quests;
6. mentor-review milestones.

Chest rarities:

```text
common, uncommon, rare, sealed, legendary
```

Possible rewards:

| Reward | Use |
|---|---|
| Shards | Spendable currency |
| Proof Armor | Protects a failed seal |
| Lens | Reveals one graph-oriented hint |
| Trace Key | Unlocks a deeper solution trace after completion |
| Mentor Token | Gives an extra AI mentor interaction in limited modes |
| Review Pass | Request human mentor review on a non-boss exercise |
| Theme/cosmetic | Profile and editor customization |
| Pulse Freeze | Protects a missed Daily Pulse |
| XP Catalyst | Temporary XP boost, disabled for leaderboards |

### Shards

Shards buy non-pay-to-win support:

1. cosmetics;
2. extra practice drills;
3. solution peeks after a serious attempt;
4. deeper traces for completed lessons;
5. optional mentor reviews.

Shards must not buy certificates, mastery, boss completion, leaderboard score, or project seals.

## Modes

### 1. Main Path

The canonical guided curriculum. It is linear enough for beginners and branchy enough for experienced developers to test out.

### 2. Training Grounds

Infinite, generated, concept-tagged practice. Examples:

| Drill | Purpose |
|---|---|
| Bind Drill | choose `bind`, `state`, or `tally` correctly |
| Take Drill | add explicit task inputs |
| Result Drill | unwrap `Ok` and propagate `Err` with `?` |
| Gate Drill | repair missing `uses` or missing runtime caps |
| Module Drill | fix private, ambiguous, or unqualified imports |
| Graph Drill | inspect a graph slice and answer structural questions |
| Graft Surgery | choose the correct graft operation for a change |
| Diagnostic Sprint | repair compiler diagnostics under a timer |

Training Grounds awards mastery only when drills are fresh or adaptively difficult.

### 3. Sentinel Trials

Boss-style capstones. A Sentinel Trial combines several concepts, hidden tests, graph checks, and an explanation prompt.

Example Sentinel:

```text
Arc: Result Flow
Goal: build a score loader that reads text, parses a score, returns Ok(label), and propagates file or parse errors.
Checks:
- source parses and formats
- task return type is Result<Text, Error>
- file read uses FileRead gate
- fallible adapter is used
- ? propagates Err
- visible and hidden tests pass
- learner explains why missing FileRead is authority failure, not recoverable Err
```

### 4. Workshop Quests

Small group quests for guild-like play:

1. weekly concept challenge;
2. shared project seal;
3. code review ladder;
4. help forum bounties;
5. team leaderboard.

Workshop score should favor completed mastery and accepted reviews, not raw XP.

### 5. Seal Trials

Weekly opt-in competitions. Categories:

1. fastest correct diagnostic repair;
2. smallest readable Sley solution;
3. best graph-preserving refactor;
4. safest effect/gate design;
5. best explanation of a rejected graft.

Previous winning solutions become review material after the event closes.

### 6. Mentor Interviews

Open-response checks where the Loom Guide asks a question and continues until the learner demonstrates understanding. Wrong or off-topic answers can consume Proof Armor or break a Clean Seal Chain only in graded interview mode. Clarifying questions are never penalized.

Example prompts:

```text
Why is `take gate fs: Gate<FileRead>` not passed as an ordinary call argument?

What is the difference between `bind total = 0` and `tally total = 0` in a loop?

Why should an unauthorized runtime capability fail as a diagnostic instead of `Err(error)`?
```

## Curriculum

Target first complete path:

```text
12 campaigns
48 arcs
220 short trials
36 Training Grounds drill families
18 Sentinel Trials
6 public projects
1 final certification project
```

### Campaign 0: Enter The Loom

Purpose: first contact with Sley and the game loop.

Concepts:

1. what Sley is;
2. source vs typed graph;
3. `sley check`;
4. `sley run`;
5. Run vs Seal;
6. formatting;
7. first Daily Pulse.

First code:

```sley
task main -> Text {
  return "hello, Sley"
}
```

Sentinel: fix a broken hello program, run it, seal it, and explain what the return type means.

### Campaign 1: Tasks And Takes

Purpose: teach Sley's executable unit.

Concepts:

1. `task`;
2. return type;
3. explicit `take`;
4. `call`;
5. simple expressions;
6. `Text`, `Int`, `Bool`, `Float`;
7. call arity diagnostics.

Representative trial:

```sley
task classify -> Text {
  take score: Int

  return if score >= 90 { "high" } else { "steady" }
}

task main -> Text {
  return call classify(95)
}
```

Sentinel: build a grade classifier with three branches and repair an intentional call-arity bug.

### Campaign 2: Binding Ontology

Purpose: replace generic variable thinking with Sley binding kinds.

Concepts:

1. `bind` is immutable;
2. `state` is mutable lifecycle state;
3. `tally` is accumulator state;
4. `set` only works on mutable bindings;
5. `forge` creates an isolated block;
6. hard rule: no generic `var`.

Representative trial:

```sley
task sum -> Int {
  take values: List<Int>

  state index = 0
  tally total = 0
  while index < len(values) {
    set total = total + values[index]
    set index = index + 1
  }
  return total
}
```

Sentinel: repair immutable mutation and explain why `bind` cannot be changed.

### Campaign 3: Collections And Control Flow

Purpose: make Sley feel useful for ordinary logic.

Concepts:

1. `if` expressions;
2. statement-level `if`/`else`;
3. `while`;
4. `each`;
5. list literals;
6. map literals;
7. indexing;
8. `len`;
9. branch type matching.

Sentinel: build a small inventory analyzer using lists, maps, and typed branches.

### Campaign 4: Records, Slots, And Domains

Purpose: teach structural data.

Concepts:

1. `type`;
2. `slot`;
3. record literals;
4. field access;
5. typed record validation;
6. domain modeling.

Representative trial:

```sley
type User = {
  slot id: Text
  slot name: Text
  slot email: Text
}

task display_name -> Text {
  take user: User

  return user.name
}
```

Sentinel: model a `SealReport` record and compute a summary.

### Campaign 5: Modules And Visibility

Purpose: teach multi-file Sley.

Concepts:

1. `module`;
2. `import`;
3. aliases;
4. `export task`;
5. `export type`;
6. `export effect`;
7. private declarations;
8. ambiguous imports;
9. `sley.toml`.

Representative project shape:

```toml
[project]
name = "module-demo"
root = "src"
entry = "app.main"
```

```sley
module app.main

import app.math as math

task main -> Int {
  return call math.double(21)
}
```

Sentinel: split a single-file program into two modules and repair private/ambiguous calls.

### Campaign 6: Result Flow

Purpose: teach recoverable failure.

Concepts:

1. `Result<T, E>`;
2. `Ok(value)`;
3. `Err(error)`;
4. runtime `Error` record;
5. `?` propagation;
6. checker rule: `?` only inside a `Result` returning task;
7. distinction between recoverable errors and authority diagnostics.

Representative trial:

```sley
task parse_score -> Result<Int, Error> {
  take raw: Text

  if raw == "bad" {
    return Err({ code: "BAD_SCORE", message: "score is not valid" })
  }
  return Ok(41)
}

task main -> Result<Int, Error> {
  bind score = call parse_score("41")?
  return Ok(score + 1)
}
```

Sentinel: build a fallible score pipeline and explain each propagation point.

### Campaign 7: Effects And Gates

Purpose: teach Sley's authority model.

Concepts:

1. `uses Effect`;
2. `take gate`;
3. `Gate<Effect>`;
4. gate takes are injected, not ordinary arguments;
5. runtime `--cap`;
6. scope denial;
7. static authority vs runtime authority.

Representative trial:

```sley
task load -> Result<Text, Error> uses FileRead {
  take path: Text

  bind text = fs.try_read_text(path)?
  return Ok(text)
}
```

Run:

```bash
sley run --cap FileRead=/tmp/sley program.sley
```

Sentinel: repair a file reader that has a missing `uses FileRead`, an incorrect raw adapter, and an out-of-scope path.

### Campaign 8: Deterministic Host Adapters

Purpose: teach v0 file and database host boundaries.

Concepts:

1. `FileRead`;
2. `FileWrite`;
3. `DatabaseRead`;
4. `DatabaseWrite`;
5. seeded tables;
6. `DbRow`;
7. row accessors;
8. `db.query_one`;
9. `db.query`;
10. `db.try_insert`.

Representative trial:

```sley
task main -> Result<Text, Error> uses DatabaseWrite, DatabaseRead {
  bind inserted = call db.try_insert("users", { id: "u3", name: "Lin" })?
  bind row = call db.query_one("select * from users where id = ?", inserted.text("id"))
  return Ok(row.text("name"))
}
```

Sentinel: build a deterministic profile service from seeded JSON rows.

### Campaign 9: Graph Literacy

Purpose: make source and graph both visible.

Concepts:

1. source projection;
2. typed graph;
3. AST JSON;
4. symbol graph;
5. graph slices;
6. node IDs;
7. inbound/outbound calls;
8. diagnostics with spans and repair hints.

Commands:

```bash
sley ast --json program.sley
sley graph --json program.sley
sley graph --json --slice task:app.main.main program.sley
```

Sentinel: answer graph questions, then repair a program using only the graph panel and diagnostics.

### Campaign 10: Grafts, Traces, And Seals

Purpose: teach the agent-native edit model.

Concepts:

1. graft operations;
2. dry-run default;
3. `--write`;
4. strict graft JSON;
5. trace receipts;
6. `sley trace`;
7. `sley seal`;
8. content-addressed evidence;
9. why source edits are human projections, not the only editing surface.

Representative graft:

```json
{
  "op": "InsertStatement",
  "target": "task:app.main.main",
  "payload": {
    "position": 1,
    "source": "bind answer = 42"
  }
}
```

Sentinel: apply a dry-run graft, inspect the accepted source, write it, view trace receipts, and seal the artifact.

### Campaign 11: Human-Agent Sley

Purpose: teach humans to collaborate with coding agents safely.

Concepts:

1. read a graph slice before editing;
2. ask an agent for a graft, not a blind text patch;
3. reject unsupported operations;
4. inspect diagnostics;
5. check authority effects;
6. validate traces;
7. compare semantic change vs textual change.

Sentinel: review an agent-proposed change that looks plausible but smuggles in an unauthorized effect.

### Campaign 12: Self-Hosting Ladder

Purpose: orient advanced learners toward Sley's future.

Concepts:

1. Rust Loom as bootstrap oracle;
2. Sley standard library;
3. Sley helper tools;
4. shadow compiler passes;
5. conformance matching;
6. promoted Sley passes;
7. recovery oracle.

Sentinel: write a small Sley helper that inspects a simplified diagnostic record and suggests a repair category.

## Public Projects

Projects are longer guided builds with portfolio pages and sealed trace evidence.

| Project | Concepts |
|---|---|
| Book of Takes | tasks, takes, returns, calls |
| Tally Forge | mutable state, loops, lists, maps |
| Module Cartographer | modules, imports, exports, ambiguity |
| Fallible Ledger | `Result`, `Error`, `?`, record modeling |
| Gatehouse Service | effects, gates, file/database adapters |
| Graph Surgeon | graph slices, grafts, traces, seals |
| Final: LearnSley Mini Engine | learner builds a small Sley lesson validator in Sley |

Each project includes:

1. local CLI tests;
2. hidden tests;
3. graph checks;
4. explanation prompt;
5. sealed trace artifact;
6. optional mentor review;
7. profile badge and shareable proof page.

## Lesson Anatomy

Every trial uses the same contract:

```yaml
id: sley.tasks.first_take
title: First Take
campaign: Tasks And Takes
arc: Explicit Inputs
difficulty: 1
concepts: [task, take, call, Text]
estimated_minutes: 4
starter: lessons/tasks/first_take/starter.sley
solution: lessons/tasks/first_take/solution.sley
visible_tests: lessons/tasks/first_take/tests.visible.json
hidden_tests: lessons/tasks/first_take/tests.hidden.json
graph_assertions: lessons/tasks/first_take/graph.json
hint_ladder: lessons/tasks/first_take/hints.md
interview_prompt: null
rewards:
  xp: 15
  shards: 2
  chain_eligible: true
```

Instruction pattern:

1. one paragraph explaining the concept;
2. one tiny code example;
3. one task statement;
4. one misconception warning;
5. one expected behavior;
6. no long lecture before the learner codes.

Hint ladder:

```text
Hint 1: point to relevant syntax.
Hint 2: point to compiler diagnostic.
Hint 3: show a partial shape.
Hint 4: reveal solution after cost/confirmation.
```

## Evaluation Engine

The grader should combine several checks:

1. parse check;
2. format check;
3. type/effect check;
4. runtime output tests;
5. hidden runtime tests;
6. graph shape assertions;
7. diagnostic expectation checks for repair lessons;
8. explanation/interview acceptance;
9. project-specific trace/seal checks.

Graph assertions are essential because Sley lessons often need to require a concept even when many source strings would produce the same output.

Example graph assertions:

```json
{
  "must_have": [
    {"kind": "task", "name": "classify"},
    {"kind": "take", "task": "classify", "name": "score", "type": "Int"},
    {"kind": "binding", "binding_kind": "tally", "name": "total"}
  ],
  "must_not_have": [
    {"kind": "effect", "name": "FileRead"}
  ]
}
```

## Sley Program Architecture

LearnSley should gradually become a Sley-authored program. The practical architecture:

```text
learnsley/
  sley.toml
  src/
    learnsley/
      main.sley
      course.sley
      lesson.sley
      progress.sley
      rewards.sley
      mastery.sley
      evaluator.sley
      diagnostics.sley
      training.sley
      workshop.sley
      leaderboard.sley
      seal.sley
  lessons/
    ...
  runtime/
    web-shell/
    cli-wrapper/
```

Near-term ownership:

1. Sley source owns lesson metadata models, scoring rules, simple validators, and examples.
2. Rust Loom owns parse/check/run/graft/trace/seal authority.
3. Web shell owns UI, auth, persistence, and sandbox orchestration.
4. CLI wrapper owns local command execution and sync.

Example Sley domain model:

```sley
module learnsley.lesson

export type Lesson = {
  slot id: Text
  slot title: Text
  slot campaign: Text
  slot difficulty: Int
  slot xp: Int
}

export task base_reward -> Int {
  take lesson: Lesson

  return lesson.xp + lesson.difficulty
}
```

Example reward rule:

```sley
module learnsley.rewards

import learnsley.lesson as lesson

export task seal_reward -> Int {
  take trial: lesson.Lesson
  take clean: Bool

  if clean {
    return trial.xp + 5
  }
  return trial.xp
}
```

Example learner exercise:

```sley
module novice.first

task greet -> Text {
  take name: Text

  return "hello " + name
}

task main -> Text {
  return call greet("Sley")
}
```

## User Interface

### Main Lesson View

Panels:

1. instructions;
2. Sley editor;
3. tests/output;
4. diagnostics;
5. graph slice;
6. hint/mentor;
7. reward/chain state.

Required controls:

1. `Run`;
2. `Seal`;
3. `Format`;
4. `Graph`;
5. `Hint`;
6. `Ask Guide`;
7. `Open Locally`.

The `Seal` button should be visually distinct and slightly heavier than `Run`, but never hidden. If the learner has not run since the last edit, the UI warns them but allows the seal.

### Map View

The map is not a decorative fantasy map. It is a structural learning graph:

```text
Campaign -> Arc -> Trial -> Sentinel -> Project
```

Nodes show:

1. completion;
2. mastery gaps;
3. due reviews;
4. locked prerequisites;
5. project seals;
6. optional challenge branches.

### Graph Panel

The graph panel teaches Sley's identity:

1. module nodes;
2. task nodes;
3. type/effect nodes;
4. call edges;
5. import/export edges;
6. authority/effect edges;
7. selected source span;
8. diagnostics attached to nodes.

In early lessons it is read-only and simplified. In advanced lessons it becomes required.

### Profile

Profile shows:

1. level;
2. Daily Pulse;
3. Clean Seal Chain;
4. campaigns completed;
5. public project seals;
6. concept mastery radar;
7. Sealmarks;
8. Fellowship Credit;
9. optional leaderboard placement.

## Social Systems

### Workshops

Workshops are small learner groups. They support:

1. weekly group quests;
2. shared discussion threads;
3. mentor office hours;
4. project review queues;
5. group progress without forcing competition.

### Fellowship Credit

Fellowship Credit rewards useful help:

1. accepted explanation;
2. high-quality code review;
3. diagnostic translation;
4. project feedback;
5. mentoring a lower-level trial.

Credit should require recipient confirmation or moderator/mentor validation. Raw posting volume should not score.

### Leaderboards

Leaderboards are opt-in. Types:

1. weekly XP;
2. weekly mastery;
3. Clean Seal Chain;
4. Training Grounds category;
5. Workshop quest;
6. Seal Trial contest.

Anti-farm rules:

1. repeated solved lessons give no leaderboard XP;
2. XP catalysts do not count for competitive ranking;
3. old easy reviews have capped value;
4. suspicious rapid submissions are flagged;
5. players can opt out without losing friends/workshop features.

## AI Mentor: Loom Guide

The Loom Guide has access to:

1. lesson instructions;
2. starter source;
3. learner source;
4. visible tests;
5. compiler diagnostics;
6. graph slice;
7. hint ladder;
8. accepted solution only when the learner spends a solution reveal.

Allowed behavior:

1. ask clarifying questions;
2. explain diagnostics;
3. point at relevant docs;
4. suggest the next smallest experiment;
5. ask the learner to predict output;
6. compare source and graph;
7. grade interview answers against acceptance criteria.

Disallowed default behavior:

1. dump final solution before a serious attempt;
2. rewrite entire project for the learner;
3. bypass tests;
4. encourage unauthorized effects;
5. hide uncertainty about Sley v0 limitations.

Mentor personality: direct, calm, precise, slightly game-aware, never overly cute. Sley is already unusual; the mentor should reduce cognitive load, not add noise.

## Retention Without Bad Learning Incentives

LearnSley should avoid the main failure mode of heavily gamified learning: optimizing for points instead of competence.

Rules:

1. mastery gates outrank XP gates;
2. repeated easy work gives little or no progress;
3. explanation checks appear at arc boundaries;
4. hidden tests prevent memorizing visible cases;
5. graph assertions prevent output-only hacks;
6. leaderboards are opt-in;
7. streak repair is possible but bounded;
8. solution peeks cost items and mark the lesson as assisted;
9. assisted completion still counts for progression but reduces mastery gain;
10. projects require unassisted final seal for certification.

## Certification

Certificate: Sley v0 Human Operator.

Requirements:

1. complete all core campaigns through Graph Surgeon;
2. pass all Sentinel Trials;
3. finish at least four public projects;
4. finish the final LearnSley Mini Engine project;
5. pass a mentor interview on Sley's authority model;
6. generate a trace/seal artifact for the final project;
7. complete a code review of an agent-proposed graft.

Certificate page includes:

1. project summaries;
2. concept mastery;
3. seal digests;
4. selected source files;
5. trace receipt count;
6. mentor review status.

## MVP

The smallest credible LearnSley release:

1. 40 trials across Campaigns 0-3.
2. 4 Sentinel Trials.
3. 1 project: Tally Forge.
4. Web editor with Run, Seal, diagnostics, and rewards.
5. Local CLI wrapper for the same lessons.
6. Progress JSON or SQLite persistence.
7. XP, levels, Daily Pulse, Clean Seal Chain, Proof Armor, Cache Chests.
8. Hint ladder, no full AI mentor required yet.
9. Basic profile.
10. No public leaderboard until anti-farm scoring exists.

MVP acceptance criteria:

1. a complete beginner can write and run a first Sley task in under 10 minutes;
2. a learner can complete 30 minutes of lessons without leaving the app;
3. failed `Seal` teaches Run-before-Seal behavior;
4. every trial has visible tests and at least one useful diagnostic path;
5. the game never teaches syntax unsupported by Sley v0 unless the lesson is explicitly marked future;
6. the project can export completed source and progress evidence.

## Phase Plan

### Phase 1: Course Spine

Build:

1. lesson manifest schema;
2. 40 starter lessons;
3. visible/hidden test runner;
4. reward engine;
5. progress store;
6. CLI wrapper;
7. beginner web UI.

### Phase 2: Sley-Native Identity

Build:

1. graph panel;
2. graph assertions;
3. diagnostic repair lessons;
4. first Sley-authored reward and lesson metadata modules;
5. trace/seal display;
6. first public project.

### Phase 3: Retention And Practice

Build:

1. Training Grounds;
2. Daily Pulse;
3. Cache Chests;
4. Proof Armor;
5. Clean Seal Chain;
6. spaced review;
7. personal records.

### Phase 4: Social And Competitive

Build:

1. workshops;
2. Fellowship Credit;
3. opt-in leaderboards;
4. weekly Seal Trials;
5. winning solution review;
6. moderation and anti-farm checks.

### Phase 5: Advanced Sley

Build:

1. graft lessons;
2. trace/seal projects;
3. agent-review lessons;
4. final certification;
5. advanced self-hosting ladder content.

## Content Production Standard

Every lesson must pass:

1. `sley format` on all starter and solution files;
2. `sley check` on solution;
3. expected diagnostic snapshot for broken starter, if intentionally broken;
4. visible tests;
5. hidden tests;
6. concept tag review;
7. one misconception check;
8. accessibility review for instruction clarity;
9. no unsupported Sley syntax unless marked future;
10. no copied competitor prose.

## Metrics

North star:

```text
weekly verified Sley seals per active learner
```

Supporting metrics:

1. time to first successful `task main`;
2. Run-before-Seal ratio;
3. diagnostic repair success rate;
4. first-week retention;
5. campaign completion;
6. mastery gain per hour;
7. Sentinel pass rate;
8. project seal rate;
9. mentor hint dependency;
10. assisted vs unassisted completion;
11. leaderboard opt-out rate;
12. community accepted help rate.

Bad metrics to avoid optimizing:

1. raw time in app;
2. repeated easy XP;
3. number of hints consumed;
4. streak length without mastery;
5. leaderboard grind volume.

## Risks

| Risk | Mitigation |
|---|---|
| Sley v0 is still narrow | Keep lessons aligned to implemented features; mark future campaigns clearly |
| Learners copy solutions without understanding | Assisted completions reduce mastery; Sentinel Trials include interviews |
| Game mechanics overpower learning | Mastery gates, anti-farm XP, opt-in leaderboards |
| AI mentor gives away code | Hint ladder, solution reveal item, prompt restrictions, audit logs |
| Graph concepts overwhelm beginners | Delay graph requirements until after source confidence; use simplified visual graph first |
| Local CLI setup causes friction | Browser-first MVP; local CLI introduced gradually with copyable commands |
| Leaderboard cheating | Mastery-weighted scoring, hidden tests, suspicious pattern detection, opt-out |
| Legal risk from "copy Boot.dev" | Adapt mechanics only; original names, text, art, story, lessons, and curriculum |

## Example First Five Trials

### Trial 1: Return Text

Goal: change the return value of `main`.

Starter:

```sley
task main -> Text {
  return "change me"
}
```

Seal condition: returns `"hello, Sley"`.

### Trial 2: Add A Take

Goal: add an explicit input to a task.

Starter:

```sley
task greet -> Text {
  return "hello"
}

task main -> Text {
  return call greet("Ada")
}
```

Expected repair:

```sley
task greet -> Text {
  take name: Text

  return "hello " + name
}
```

### Trial 3: Bind Is Immutable

Goal: fix an immutable mutation diagnostic.

Starter:

```sley
task main -> Int {
  bind total = 0
  set total = total + 1
  return total
}
```

Expected repair: use `state` or `tally`, with lesson discussion explaining which is better.

### Trial 4: Tally A List

Goal: accumulate values with `tally`.

Starter:

```sley
task sum -> Int {
  take values: List<Int>

  tally total = 0
  each value in values {
    set total = total + value
  }
  return total
}
```

Task: complete `main` to call `sum`.

### Trial 5: Explain A Diagnostic

Goal: answer a Loom Guide question.

Prompt:

```text
The compiler says `set` cannot modify `bind total`. In one or two sentences,
explain the rule and name the binding kind you would use for an accumulator.
```

Pass condition: learner mentions immutable `bind` and `tally` or mutable binding.

## Example Sentinel Trial: Gatehouse

Brief:

```text
Build a task that reads a profile file and returns its contents as
Result<Text, Error>. Use the fallible file adapter and propagate errors.
```

Starter:

```sley
task load_profile -> Text {
  take path: Text

  return fs.read_text(path)
}
```

Expected concepts:

1. return type becomes `Result<Text, Error>`;
2. task declares `uses FileRead`;
3. uses `fs.try_read_text(path)?`;
4. returns `Ok(text)`;
5. run command grants `--cap FileRead=<root>`.

Possible accepted shape:

```sley
task load_profile -> Result<Text, Error> uses FileRead {
  take path: Text

  bind text = fs.try_read_text(path)?
  return Ok(text)
}
```

Interview prompt:

```text
Why is a missing FileRead capability a runtime diagnostic instead of an Err value?
```

## Source Notes

Competitive mechanics checked on 2026-05-05:

1. Boot.dev describes code-first backend learning, a game-like curriculum, Boots as a Socratic AI mentor, community, XP, lessons completed, and course catalog depth: https://www.boot.dev/
2. Boot.dev lesson pages document Run vs Submit, failure penalties, armor, sharpshooter sprees, chests, gems, community, guilds, leaderboard, and Discord karma/fellowship: https://www.boot.dev/lessons/142c8a73-5ede-49a6-9460-563890646023 and https://www.boot.dev/lessons/0f4fa755-1ce7-468b-bf34-c1460e97bf28
3. Boot.dev patch notes document chests as primary rewards, daily quests, sprees, boss drops, items, and CLI-based local lessons: https://www.boot.dev/blog/news/bootdev-beat-2024-05/
4. Boot.dev Training Grounds launch notes describe harder practice, AI help, and available solutions: https://www.boot.dev/blog/news/training-grounds-launch/
5. CodeCombat describes real typed code controlling levels and AI League competition: https://codecombat.zendesk.com/hc/en-us/articles/4410481859095-What-is-CodeCombat
6. CodeCombat feature pages describe unlockables, custom level creation, group leaderboards, and AI hints that explain without giving the whole solution: https://codecombat.com/features/
7. Codecademy describes guided paths, lessons, quizzes, projects, interactive browser editor, live output, AI assistant, tooltips, and milestones: https://www.codecademy.com/ and https://help.codecademy.com/hc/en-us/articles/220453248-Picking-Your-Learning-Path
8. Exercism documents language tracks, test-passing exercises that unlock more exercises, mentoring, reputation, and contribution credit: https://exercism.org/docs/using/getting-started
9. HackerRank weekly challenge docs document weekly cadence, reset, leaderboards, previous winning submissions, and scoring rules: https://support.hackerrank.com/articles/7992263058-weekly-challenges
10. Duolingo product posts document leaderboards/leagues, opt-out, anti-cheat monitoring, friends quests, streaks, achievements, personal records, and milestone badges: https://blog.duolingo.com/duolingo-leagues-leaderboards/ and https://blog.duolingo.com/achievement-badges/

Sley local authorities used:

1. `<sley-language-spec>`
2. `<sley-repo>/README.md`
3. `<sley-repo>/docs/SleyLanguageSpec.md`
4. `<sley-repo>/SleyImprove.md`
5. `<sley-repo>/SleyCompiler.md`
6. `<sley-repo>/examples/*.sley`
