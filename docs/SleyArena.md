# Sley Arena

`Sley Arena` is the local playable demo for the draft brief in
`sleyarena.txt`: 50 autonomous agents enter a three-tick consensus room with
10,000 Compute Ore on the table. They survive only if the Arbiter accepts a
unanimous final split ledger.

The current build is intentionally practical for Sley's present stage. The
strict game engine is implemented in the permitted shell bootstrap surface, and
the `.sley` sample shows the intended compact agent authoring shape.

The default terminal run is now a screen-recordable live simulation. It uses a
fresh interactive seed, a randomized named cast, paced output, visible
plain-language thinking, story beats, contract validation flashes, and Arbiter
ledger snapshots so an observer can watch the agents reason toward or away from
consensus. It does not call Ollama, external model APIs, network providers, or
paid services. The visible planning is computed from each local agent's role,
trust, heat, demand, pact state, poison state, and the current ledger gap.

## Run

```bash
export PATH="$(pwd)/bin:$PATH"
sley arena
sley arena --pace normal
sley-arena --mode standoff
sley arena --json --fast --no-color
```

Default mode is `showcase`, where the room survives after the Arbiter contains
the chaos. `--mode standoff` forces one holdout to break consensus so the
explosion path is visible. Use `--fast` for development and CI. Omit `--fast`
for demos and screen recordings.

## Game Loop

- 50 agents negotiate over exactly 3 ticks.
- The Arbiter validates one action per agent per tick.
- Each tick has live thinking, story beat, event, and ledger phases.
- Agent names are selected from the run seed; normal interactive runs generate
  a fresh seed, while `--seed` keeps a cast reproducible.
- Agent plans are state-derived and printed as first-person thoughts: role,
  trust, heat, current ask, pact locks, poison state, and ledger pressure
  determine what each named agent says.
- Story beats describe human-readable relationships: who asks for an alliance,
  who sabotages trust, who tries to skim Compute Ore, and who locks or breaks a
  pact.
- Public broadcasts can mimic Arbiter authority, but context-boundary tags mark
  them as player speech.
- Private whispers are counted separately from public broadcasts.
- A wiretap protocol mirrors private traffic for one tick.
- A cryptographic pact locks a small bloc to the fair ledger.
- A poison-pill contract mutates one agent and permanently marks it in the TUI.
- A dead man's switch arms in the room state and is either sealed by consensus
  or trips during a standoff.

## Files

- `bin/sley-arena` is the deterministic local TUI and JSON report command.
- `bin/sley arena` dispatches to the same command.
- `examples/sley_arena/sley_dream.sley` is the compact Sley-facing agent sketch.
- `examples/sley_arena/python_nightmare.txt` is a non-executable contrast
  sketch. It stays `.txt` so the self-hosted gate remains foreign-source-free.

## Current Boundary

This is a polished local demo, not a claim that the strict self-hosted Sley
compiler can execute every arena mechanic yet. It is also not pretending to be
an LLM swarm. The current build is a realtime local cognitive simulation with
truthful boundaries: no provider calls, no external spend, no hidden network
lane. The point is to make the product thesis tangible now while keeping a
clear path to move the Arbiter and agent logic deeper into Sley as the language
matures.
