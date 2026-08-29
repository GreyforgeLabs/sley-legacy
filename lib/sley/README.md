# Sley CLI modules

`bin/sley` is the only public entry point. It loads these internal modules in
dependency order and then calls the compact dispatcher.

| Module | Ownership |
| --- | --- |
| `bootstrap.sh` | self-hosted source evaluation and versioned compiler contracts |
| `lifecycle.sh` | shared dependency, argument, JSON, numeric-bound, and cleanup helpers |
| `core.sh` | parser/checker/runtime report primitives and shared syntax operations |
| `commands-quality.sh` | contracts, CI, analysis, validation, packaging, and read-only commands |
| `commands-change.sh` | governed planning and transactional change commands |
| `commands-execution.sh` | runtime, mutation, formatting, trace, seal, and ZJX commands |
| `commands-machine.sh` | bounded machine JSONL protocol and arena delegation |
| `dispatch.sh` | public command-name routing only |

Modules are sourced, never executed directly. `scripts/test-sley-cli-modules.sh`
checks syntax, function availability, launcher size, module size, and the
pre-extraction golden behavior. New command families should go into the
narrowest existing module or a new documented family rather than growing the
launcher or dispatcher with implementation bodies.
