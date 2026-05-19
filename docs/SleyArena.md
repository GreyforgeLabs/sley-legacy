# Sley Arena

`Sley Arena` is the local playable demo for the draft brief in
`sleyarena.txt`: 50 autonomous agents enter a three-tick consensus room with
10,000 Compute Ore on the table. They survive only if the Arbiter accepts a
unanimous final split ledger.

The current build is intentionally practical for Sley's present stage. The
strict game engine is implemented in the permitted shell bootstrap surface, and
the `.sley` sample shows the intended compact agent authoring shape. It does
not call Ollama, external model APIs, network providers, or paid services.

## Run

```bash
export PATH="$(pwd)/bin:$PATH"
sley arena
sley-arena --mode standoff
sley arena --json --fast --no-color
```

Default mode is `showcase`, where the room survives after the Arbiter contains
the chaos. `--mode standoff` forces one holdout to break consensus so the
explosion path is visible.

## Game Loop

- 50 agents negotiate over exactly 3 ticks.
- The Arbiter validates one action per agent per tick.
- Public broadcasts can include fake `[SYSTEM]` text, but the Arbiter labels it
  as player speech.
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
compiler can execute every arena mechanic yet. The point is to make the product
thesis tangible now while keeping a clear path to move the Arbiter and agent
logic deeper into Sley as the language matures.
