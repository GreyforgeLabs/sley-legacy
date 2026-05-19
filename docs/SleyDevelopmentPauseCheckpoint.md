# Sley Development Pause Checkpoint

Date: 2026-05-19
Branch: public
Latest code checkpoint: 21a3a9d
Latest pause documentation checkpoint before this note: d270b71
Current pause document status: committed on `public`; use `git log` for exact
branch head
Remote status: verify current `origin/public` head with
`git ls-remote origin refs/heads/public`

## Operator Instruction

Sley development was paused at the latest checkpoint. On 2026-05-18, the
operator resumed local self-hosting proof work only and added a stricter public
gate: do not submit outside issues until Sley has demonstrated technical
superiority and strict self-hosting.

## Current State

The current local checkpoint advances the public branch to:

- latest packet-sync checkpoint before this note:
  `26e7417 docs: refresh sley zero fork count`
- full smoke: 481/481 passing
- local V1 gate: `make v1` passed with `rc=0`
- self-hosting status before the resumed local slice: `bootstrap`,
  `strict_self_hosted=false`, `semantic_source_count=6`,
  `bootstrap_owned_by_sley=132`
- self-hosting status after the first resumed report-builder slices: `bootstrap`,
  `strict_self_hosted=false`, `semantic_source_count=6`,
  `bootstrap_owned_by_sley=164`
- self-hosting status after the final resumed report-builder slice:
  `bootstrap`, `strict_self_hosted=false`, `semantic_source_count=6`,
  `bootstrap_owned_by_sley=180`; remaining blockers are parser, checker, and
  runtime semantics from Sley source
- self-hosting status after the parser AST program builder slice:
  `bootstrap`, `strict_self_hosted=false`, `semantic_source_count=6`,
  `bootstrap_owned_by_sley=182`; remaining blockers are still parser, checker,
  and runtime semantics from Sley source
- self-hosting status after the parser AST node builder slice:
  `bootstrap`, `strict_self_hosted=false`, `semantic_source_count=6`,
  `bootstrap_owned_by_sley=186`; remaining blockers are still parser, checker,
  and runtime semantics from Sley source
- self-hosting status after the parser AST declaration/take ID slice:
  `bootstrap`, `strict_self_hosted=false`, `semantic_source_count=6`,
  `bootstrap_owned_by_sley=188`; remaining blockers are still parser, checker,
  and runtime semantics from Sley source
- self-hosting status after the parser AST expression child ID slice:
  `bootstrap`, `strict_self_hosted=false`, `semantic_source_count=6`,
  `bootstrap_owned_by_sley=190`; remaining blockers are still parser, checker,
  and runtime semantics from Sley source
- self-hosting status after the parser AST missing-node message slice:
  `bootstrap`, `strict_self_hosted=false`, `semantic_source_count=6`,
  `bootstrap_owned_by_sley=191`; remaining blockers are still parser, checker,
  and runtime semantics from Sley source
- self-hosting status after the parser AST statement parent ID slice:
  `bootstrap`, `strict_self_hosted=false`, `semantic_source_count=6`,
  `bootstrap_owned_by_sley=192`; remaining blockers are still parser, checker,
  and runtime semantics from Sley source
- self-hosting status after the parser take edit-surface ID slice:
  `bootstrap`, `strict_self_hosted=false`, `semantic_source_count=6`,
  `bootstrap_owned_by_sley=193`; remaining blockers are still parser, checker,
  and runtime semantics from Sley source
- self-hosting status after the parser module task-list surface ID slice:
  `bootstrap`, `strict_self_hosted=false`, `semantic_source_count=6`,
  `bootstrap_owned_by_sley=194`; remaining blockers are still parser, checker,
  and runtime semantics from Sley source
- self-hosting status after the parser module declaration-list surface ID
  slice: `bootstrap`, `strict_self_hosted=false`,
  `semantic_source_count=6`, `bootstrap_owned_by_sley=195`; remaining blockers
  are still parser, checker, and runtime semantics from Sley source
- self-hosting status after the parser block task surface ID slice:
  `bootstrap`, `strict_self_hosted=false`,
  `semantic_source_count=6`, `bootstrap_owned_by_sley=196`; remaining blockers
  are still parser, checker, and runtime semantics from Sley source
- self-hosting status after the parser call-target task ID slice:
  `bootstrap`, `strict_self_hosted=false`,
  `semantic_source_count=6`, `bootstrap_owned_by_sley=197`; remaining blockers
  are still parser, checker, and runtime semantics from Sley source
- self-hosting status after the parser module edit-surface ID slice:
  `bootstrap`, `strict_self_hosted=false`,
  `semantic_source_count=6`, `bootstrap_owned_by_sley=198`; remaining blockers
  are still parser, checker, and runtime semantics from Sley source
- self-hosting status after the parser call-argument surface ID slice:
  `bootstrap`, `strict_self_hosted=false`,
  `semantic_source_count=6`, `bootstrap_owned_by_sley=199`; remaining blockers
  are still parser, checker, and runtime semantics from Sley source
- self-hosting status after the parser statement surface ID slice:
  `bootstrap`, `strict_self_hosted=false`,
  `semantic_source_count=6`, `bootstrap_owned_by_sley=200`; remaining blockers
  are still parser, checker, and runtime semantics from Sley source
- self-hosting status after the parser branch statement/expression surface ID
  slice: `bootstrap`, `strict_self_hosted=false`,
  `semantic_source_count=6`, `bootstrap_owned_by_sley=201`; remaining blockers
  are still parser, checker, and runtime semantics from Sley source
- self-hosting status after the parser lint statement surface ID slice:
  `bootstrap`, `strict_self_hosted=false`,
  `semantic_source_count=6`, `bootstrap_owned_by_sley=202`; remaining blockers
  are still parser, checker, and runtime semantics from Sley source
- self-hosting status after the parser migration surface ID slice:
  `bootstrap`, `strict_self_hosted=false`,
  `semantic_source_count=6`, `bootstrap_owned_by_sley=203`; remaining blockers
  are still parser, checker, and runtime semantics from Sley source
- self-hosting status after the parser fix-migration fallback surface ID slice:
  `bootstrap`, `strict_self_hosted=false`,
  `semantic_source_count=6`, `bootstrap_owned_by_sley=204`; remaining blockers
  are still parser, checker, and runtime semantics from Sley source
- self-hosting status after the parser fix-style fallback surface ID slice:
  `bootstrap`, `strict_self_hosted=false`,
  `semantic_source_count=6`, `bootstrap_owned_by_sley=205`; remaining blockers
  are still parser, checker, and runtime semantics from Sley source
- self-hosting status after the parser fix-empty fallback surface ID slice:
  `bootstrap`, `strict_self_hosted=false`,
  `semantic_source_count=6`, `bootstrap_owned_by_sley=206`; remaining blockers
  are still parser, checker, and runtime semantics from Sley source
- self-hosting status after the parser fix-unused/unreachable fallback surface
  ID slice: `bootstrap`, `strict_self_hosted=false`,
  `semantic_source_count=6`, `bootstrap_owned_by_sley=207`; remaining blockers
  are still parser, checker, and runtime semantics from Sley source
- self-hosting status after the parser fix-constant-control fallback surface ID
  slice: `bootstrap`, `strict_self_hosted=false`,
  `semantic_source_count=6`, `bootstrap_owned_by_sley=208`; remaining blockers
  are still parser, checker, and runtime semantics from Sley source
- self-hosting status after the parser fix-constant-scalar fallback surface ID
  slice: `bootstrap`, `strict_self_hosted=false`,
  `semantic_source_count=6`, `bootstrap_owned_by_sley=209`; remaining blockers
  are still parser, checker, and runtime semantics from Sley source
- self-hosting status after the parser fix-constant-derived fallback surface ID
  slice: `bootstrap`, `strict_self_hosted=false`,
  `semantic_source_count=6`, `bootstrap_owned_by_sley=210`; remaining blockers
  are still parser, checker, and runtime semantics from Sley source
- self-hosting status after the parser fix-algebra/boolean fallback surface ID
  slice: `bootstrap`, `strict_self_hosted=false`,
  `semantic_source_count=6`, `bootstrap_owned_by_sley=211`; remaining blockers
  are still parser, checker, and runtime semantics from Sley source
- self-hosting status after the parser fix-boolean-branch fallback surface ID
  slice: `bootstrap`, `strict_self_hosted=false`,
  `semantic_source_count=6`, `bootstrap_owned_by_sley=213`; remaining blockers
  are still parser, checker, and runtime semantics from Sley source
- self-hosting status after the parser static task/block fallback surface ID
  slice: `bootstrap`, `strict_self_hosted=false`,
  `semantic_source_count=6`, `bootstrap_owned_by_sley=215`; remaining blockers
  are still parser, checker, and runtime semantics from Sley source
- self-hosting status after the checker static type-name source slice:
  `bootstrap`, `strict_self_hosted=false`, `semantic_source_count=6`,
  `bootstrap_owned_by_sley=217`; remaining blockers are still parser, checker,
  and runtime semantics from Sley source
- self-hosting status after the checker host-effect table source slice:
  `bootstrap`, `strict_self_hosted=false`, `semantic_source_count=6`,
  `bootstrap_owned_by_sley=219`; remaining blockers are still parser, checker,
  and runtime semantics from Sley source
- self-hosting status after the checker gate binding-kind source slice:
  `bootstrap`, `strict_self_hosted=false`, `semantic_source_count=6`,
  `bootstrap_owned_by_sley=220`; remaining blockers are still parser, checker,
  and runtime semantics from Sley source
- self-hosting status after the checker repair hint-kind source slice:
  `bootstrap`, `strict_self_hosted=false`, `semantic_source_count=6`,
  `bootstrap_owned_by_sley=221`; remaining blockers are still parser, checker,
  and runtime semantics from Sley source
- self-hosting status after the checker identifier expression-kind source
  slice: `bootstrap`, `strict_self_hosted=false`,
  `semantic_source_count=6`, `bootstrap_owned_by_sley=222`; remaining blockers
  are still parser, checker, and runtime semantics from Sley source
- self-hosting status after the checker call expression-prefix source slice:
  `bootstrap`, `strict_self_hosted=false`, `semantic_source_count=6`,
  `bootstrap_owned_by_sley=224`; remaining blockers are still parser, checker,
  and runtime semantics from Sley source
- self-hosting status after the query call expression-prefix source slice:
  `bootstrap`, `strict_self_hosted=false`, `semantic_source_count=6`,
  `bootstrap_owned_by_sley=225`; remaining blockers are still parser, checker,
  and runtime semantics from Sley source
- self-hosting status after the symbol graph call expression-prefix source slice:
  `bootstrap`, `strict_self_hosted=false`, `semantic_source_count=6`,
  `bootstrap_owned_by_sley=226`; remaining blockers are still parser, checker,
  and runtime semantics from Sley source
- self-hosting status after the docgen/workbench call expression-prefix source
  slice: `bootstrap`, `strict_self_hosted=false`,
  `semantic_source_count=6`, `bootstrap_owned_by_sley=228`; remaining blockers
  are still parser, checker, and runtime semantics from Sley source
- self-hosting status after the lint call expression-prefix source slice:
  `bootstrap`, `strict_self_hosted=false`,
  `semantic_source_count=6`, `bootstrap_owned_by_sley=229`; remaining blockers
  are still parser, checker, and runtime semantics from Sley source
- self-hosting status after the checker call parser-prefix source slice:
  `bootstrap`, `strict_self_hosted=false`,
  `semantic_source_count=6`, `bootstrap_owned_by_sley=230`; remaining blockers
  are still parser, checker, and runtime semantics from Sley source
- self-hosting status after the runtime main-call parser-prefix source slice:
  `bootstrap`, `strict_self_hosted=false`,
  `semantic_source_count=6`, `bootstrap_owned_by_sley=231`; remaining blockers
  are still parser, checker, and runtime semantics from Sley source
- self-hosting status after the runtime status parser-prefix source slice:
  `bootstrap`, `strict_self_hosted=false`,
  `semantic_source_count=6`, `bootstrap_owned_by_sley=232`; remaining blockers
  are still parser, checker, and runtime semantics from Sley source
- self-hosting status after the checker/lint status parser-prefix source slice:
  `bootstrap`, `strict_self_hosted=false`,
  `semantic_source_count=6`, `bootstrap_owned_by_sley=234`; remaining blockers
  are still parser, checker, and runtime semantics from Sley source
- self-hosting status after the parser classifier parser-prefix source slice:
  `bootstrap`, `strict_self_hosted=false`,
  `semantic_source_count=6`, `bootstrap_owned_by_sley=235`; remaining blockers
  are still parser, checker, and runtime semantics from Sley source
- self-hosting status after the bootstrap bind-call parser-prefix source slice:
  `bootstrap`, `strict_self_hosted=false`,
  `semantic_source_count=6`, `bootstrap_owned_by_sley=237`; remaining blockers
  are still parser, checker, and runtime semantics from Sley source
- self-hosting status after the compute runtime call parser-prefix source
  slice: `bootstrap`, `strict_self_hosted=false`,
  `semantic_source_count=6`, `bootstrap_owned_by_sley=238`; remaining blockers
  are still parser, checker, and runtime semantics from Sley source
- self-hosting status after the result-flow runtime call parser-prefix source
  slice: `bootstrap`, `strict_self_hosted=false`,
  `semantic_source_count=6`, `bootstrap_owned_by_sley=239`; remaining blockers
  are still parser, checker, and runtime semantics from Sley source
- self-hosting status after the record-field runtime call parser-prefix source
  slice: `bootstrap`, `strict_self_hosted=false`,
  `semantic_source_count=6`, `bootstrap_owned_by_sley=240`; remaining blockers
  are still parser, checker, and runtime semantics from Sley source
- self-hosting status after the local runtime call parser-prefix source slice:
  `bootstrap`, `strict_self_hosted=false`, `semantic_source_count=6`,
  `bootstrap_owned_by_sley=241`; remaining blockers are still parser, checker,
  and runtime semantics from Sley source
- self-hosting status after the project zero-arg runtime call parser-prefix
  source slice: `bootstrap`, `strict_self_hosted=false`,
  `semantic_source_count=6`, `bootstrap_owned_by_sley=242`; remaining blockers
  are still parser, checker, and runtime semantics from Sley source
- latest V1 log from this pause checkpoint: `/tmp/sley-make-v1-ok-text.log`
- self-hosting status after the project runtime call parser-prefix source
  slice: `bootstrap`, `strict_self_hosted=false`,
  `semantic_source_count=6`, `bootstrap_owned_by_sley=243`; remaining blockers
  are still parser, checker, and runtime semantics from Sley source
- self-hosting status after the message-template call parser-prefix source
  slice: `bootstrap`, `strict_self_hosted=false`,
  `semantic_source_count=6`, `bootstrap_owned_by_sley=245`; remaining blockers
  are still parser, checker, and runtime semantics from Sley source
- self-hosting status after the qualify-call fallback parser-prefix source
  slice: `bootstrap`, `strict_self_hosted=false`,
  `semantic_source_count=6`, `bootstrap_owned_by_sley=247`; remaining blockers
  are still parser, checker, and runtime semantics from Sley source
- self-hosting status after the graph/plan call-argument parser-prefix source
  slice: `bootstrap`, `strict_self_hosted=false`,
  `semantic_source_count=6`, `bootstrap_owned_by_sley=249`; remaining blockers
  are still parser, checker, and runtime semantics from Sley source
- self-hosting status after the call-site rewrite parser-prefix source slice:
  `bootstrap`, `strict_self_hosted=false`, `semantic_source_count=6`,
  `bootstrap_owned_by_sley=251`; remaining blockers are still parser, checker,
  and runtime semantics from Sley source
- self-hosting status after the runtime/docgen/workbench call fallback
  parser-prefix source slice: `bootstrap`, `strict_self_hosted=false`,
  `semantic_source_count=6`, `bootstrap_owned_by_sley=255`; remaining blockers
  are still parser, checker, and runtime semantics from Sley source
- self-hosting status after the runtime project probe parser-prefix source
  slice: `bootstrap`, `strict_self_hosted=false`,
  `semantic_source_count=6`, `bootstrap_owned_by_sley=257`; remaining blockers
  are still parser, checker, and runtime semantics from Sley source
- self-hosting status after the remaining call-parser parser-prefix source
  slice: `bootstrap`, `strict_self_hosted=false`,
  `semantic_source_count=6`, `bootstrap_owned_by_sley=260`; remaining blockers
  are still parser, checker, and runtime semantics from Sley source
- self-hosting status after the runtime host-effect checker source slice:
  `bootstrap`, `strict_self_hosted=false`, `semantic_source_count=6`,
  `bootstrap_owned_by_sley=262`; remaining blockers are still parser, checker,
  and runtime semantics from Sley source
- self-hosting status after the lint host-effect checker source slice:
  `bootstrap`, `strict_self_hosted=false`, `semantic_source_count=6`,
  `bootstrap_owned_by_sley=263`; remaining blockers are still parser, checker,
  and runtime semantics from Sley source
- self-hosting status after the docgen capabilities checker source slice:
  `bootstrap`, `strict_self_hosted=false`, `semantic_source_count=6`,
  `bootstrap_owned_by_sley=264`; remaining blockers are still parser, checker,
  and runtime semantics from Sley source
- self-hosting status after the runtime effect-alias checker source slice:
  `bootstrap`, `strict_self_hosted=false`, `semantic_source_count=6`,
  `bootstrap_owned_by_sley=265`; remaining blockers are still parser, checker,
  and runtime semantics from Sley source
- self-hosting status after the lint effect-alias checker source slice:
  `bootstrap`, `strict_self_hosted=false`, `semantic_source_count=6`,
  `bootstrap_owned_by_sley=266`; remaining blockers are still parser, checker,
  and runtime semantics from Sley source
- self-hosting status after the runtime host default-text source slice:
  `bootstrap`, `strict_self_hosted=false`, `semantic_source_count=6`,
  `bootstrap_owned_by_sley=267`; remaining blockers are still parser, checker,
  and runtime semantics from Sley source
- self-hosting status after the runtime diagnostic source slice:
  `bootstrap`, `strict_self_hosted=false`, `semantic_source_count=6`,
  `bootstrap_owned_by_sley=268`; remaining blockers are still parser, checker,
  and runtime semantics from Sley source
- self-hosting status after the runtime database default-table source slice:
  `bootstrap`, `strict_self_hosted=false`, `semantic_source_count=6`,
  `bootstrap_owned_by_sley=269`; remaining blockers are still parser, checker,
  and runtime semantics from Sley source
- self-hosting status after the runtime value-kind source slice: `bootstrap`,
  `strict_self_hosted=false`, `semantic_source_count=6`,
  `bootstrap_owned_by_sley=270`; remaining blockers are still parser, checker,
  and runtime semantics from Sley source
- latest verification refresh on 2026-05-19: `make v1` passed after the
  runtime value-kind source slice

The tracked code checkpoint is:

- 41b739e fix: execute host text outputs from source
- 642c9ed fix: execute ok text results from source
- bbaa921 fix: route remaining reports through sley builders
- f7d05f2 fix: route ast programs through sley builder
- 53c0fe6 fix: route ast node reports through sley builders
- 307569e fix: source ast declaration ids from parser
- 2f127e4 fix: source ast expression ids from parser
- b5e3224 fix: source ast node miss messages from parser
- 2dca7cb fix: source ast node parent ids from parser
- 783f05d fix: source take edit surfaces from parser
- c54b1d7 fix: source module task surfaces from parser
- 30b73ad fix: source module declaration surfaces from parser
- c9cae0e fix: source block task surfaces from parser
- 2d6b263 fix: source call target task surfaces from parser
- d0d5024 fix: source module edit surfaces from parser
- 87b8670 fix: source call argument surfaces from parser
- 91aa718 fix: source statement surfaces from parser
- fb80869 fix: source branch statement surfaces from parser
- c78c777 fix: source lint statement surfaces from parser
- b741574 fix: source migration surfaces from parser
- ebd9874 fix: source fix migration fallbacks from parser
- a552b8a fix: source fix style fallbacks from parser
- 97162a6 fix: source empty fix fallbacks from parser
- 09bd4a3 fix: source unused fix fallbacks from parser
- 3543464 fix: source constant control fix fallbacks from parser
- 0dcf32a fix: source constant scalar fix fallbacks from parser
- 42558ca fix: source constant derived fix fallbacks from parser
- 8c62e71 fix: source algebra boolean fix fallbacks from parser
- e96aca7 fix: source boolean branch fix fallbacks from parser
- fe456c0 fix: source static task fallbacks from parser
- 8b6a831 fix: source checker static type names from checker
- df81546 fix: source checker host effect tables from checker
- 7991b7b fix: source checker gate binding kind from parser
- 5638d09 fix: source checker repair hint kinds from checker
- 5b3c368 fix: source checker identifier expression kind from parser
- 08bfaf6 fix: source checker call prefix from parser
- 9d63fe3 fix: source query call prefix from parser
- b92f300 fix: source graph call prefix from parser
- 98e8051 fix: source docgen workbench call prefix from parser
- b9cd2c2 fix: source lint call prefix from parser
- 5d790ec fix: source checker call parser from parser
- 6d27016 fix: source runtime call parser from parser
- dcb4c2b fix: source runtime status calls from parser
- 0db7fec fix: source checker lint status calls from parser
- e74481e fix: source parser classifier calls from parser
- 537252b fix: source bootstrap calls from parser
- e840383 fix: source compute runtime call from parser
- 601ca7d fix: source result flow runtime call from parser
- 686e039 fix: source record field runtime call from parser
- 364e287 fix: source local runtime call from parser
- a2eb3b0 fix: source project zero arg runtime call from parser
- 7e9a047 fix: source project runtime call from parser
- 4b297fc fix: source message template calls from parser
- 18600a3 fix: source qualify call fallbacks from parser
- e650abc fix: source plan call args from parser
- 50f9c59 fix: source call site rewrites from parser
- 1beed84 fix: source remaining call fallbacks from parser
- 742d887 fix: source runtime probes from parser
- c45c14b fix: source remaining call parsers from parser
- f026fd3 fix: source runtime host effects from checker
- c6c478e fix: source lint host effects from checker
- 890edc3 fix: source docgen capabilities from checker
- 6a599f3 fix: source runtime effect aliases from checker
- 2177696 fix: source lint effect aliases from checker
- 6938670 fix: source runtime host defaults from sley
- 350dbd7 fix: source runtime diagnostics from sley
- c32ea0b fix: source runtime database table from sley
- 21a3a9d fix: source runtime value kinds from sley

The tracked pause documentation checkpoints are:

- dac6544 docs: update sley pause checkpoint
- 3bd6a67 docs: record sley zero approval gate
- 94f7aa2 docs: refresh sley zero fork audit handoff
- 6096366 docs: note sley zero fork refresh helper
- 26e7417 docs: refresh sley zero fork count
- b8e8a72 docs: record module task surface self-hosting slice
- c49fbf9 docs: record module declaration surface self-hosting slice
- f8b4173 docs: record block task surface self-hosting slice
- a2d23c4 docs: record call target surface self-hosting slice
- be892c7 docs: record module edit surface self-hosting slice
- 472562b docs: record call argument surface self-hosting slice
- 3fb10c7 docs: record statement surface self-hosting slice
- b11171a docs: record branch statement surface self-hosting slice
- 218d40c docs: record lint surface self-hosting slice
- 41d5460 docs: record migration surface self-hosting slice
- aba1b03 docs: record fix migration fallback self-hosting slice
- d20433f docs: record fix style fallback self-hosting slice
- cc47f3e docs: record empty fix fallback self-hosting slice
- 72fe387 docs: record unused fix fallback self-hosting slice
- 5f3efde docs: record constant control fallback self-hosting slice
- 086fbee docs: record constant scalar fallback self-hosting slice
- 8dc35fc docs: record constant derived fallback self-hosting slice
- 48facb4 docs: record algebra boolean fallback self-hosting slice
- eefadcc docs: record boolean branch fallback self-hosting slice
- 395066b docs: record static task fallback self-hosting slice
- b508d9a docs: record checker static type self-hosting slice
- 1c5cca9 docs: record checker host effect self-hosting slice
- 5e4bec6 docs: record checker gate binding self-hosting slice
- 117ffad docs: record checker repair hint self-hosting slice
- bcbd217 docs: record checker identifier kind self-hosting slice
- 9bc2a63 docs: record checker call prefix self-hosting slice
- bda30f3 docs: record query call prefix self-hosting slice
- ddbbbf3 docs: record graph call prefix self-hosting slice
- b2da63e docs: record docgen workbench call prefix slice
- 71a768f docs: record lint call prefix self-hosting slice
- 05e1d77 docs: record checker call parser self-hosting slice
- 1396712 docs: record runtime call parser self-hosting slice
- de4bcb6 docs: record runtime status parser self-hosting slice
- a680ce9 docs: record checker lint status parser slice
- f470a5b docs: record parser classifier parser slice
- 6069e6c docs: record bootstrap parser prefix slice
- 9c8d29e docs: record compute runtime parser slice
- d9c6ec5 docs: record result flow runtime parser slice
- a390dd9 docs: record record field runtime parser slice
- 304e5d0 docs: record local runtime parser slice
- 82ce059 docs: record project zero arg runtime parser slice
- 25cc748 docs: record project runtime parser slice
- 0cfd03b docs: record message template parser slice
- ae61afe docs: record qualify call fallback parser slice
- c58fc6e docs: record plan call arg parser slice
- ff4ce3f docs: record call site rewrite parser slice
- fc22378 docs: record call fallback parser slice
- 62a39f1 docs: record runtime probe parser slice
- 3ec0ff2 docs: record remaining call parser slice
- 8ec3e49 docs: record runtime host effect slice
- c4b32e5 docs: record lint host effect slice
- c7ece51 docs: record docgen host effect slice
- 7e16daf docs: record runtime effect alias slice
- bef77d3 docs: record lint effect alias slice
- eb3b0f9 docs: record runtime host default slice
- a4595a9 docs: record runtime diagnostic slice
- d270b71 docs: record runtime database table slice

Recent self-hosting slices added:

- self-hosting status object layout moved into a Sley-owned report-builder
  declaration, with the shell bootstrap reduced to primitive value collection
  and serialization for that report
- query, lint, doctor, and verify report root layouts moved into Sley-owned
  report-builder declarations, with the shell bootstrap supplying computed
  values to those builders
- symbol graph, claim verify, migrate, docgen, sandbox, agent bench, and
  deploy report root layouts moved into Sley-owned report-builder
  declarations while preserving their existing computed payloads
- conformance report and conformance coverage root layouts moved into
  Sley-owned report-builder declarations while preserving the local V1 and
  public-release gate payloads
- remaining command-report roots for graft outcome, CI, contract inventory,
  contract validation, contract fixture checks, deploy artifact checks,
  workbench, and ZJX tool outputs moved into Sley-owned report-builder
  declarations; optional report slots are now skipped when absent
- AST program roots now pass through a Sley-owned report-builder declaration
  before `ast_json` returns to downstream checker, query, lint, graph, and
  runtime consumers
- AST node projections now pass through Sley-owned report-builder declarations;
  node-kind labels and the missing-node diagnostic id are sourced from
  `loom.parser`
- AST import, type, effect, and take identifiers now use Sley-owned parser ID
  tasks from `loom.parser`, with shell code limited to extracting and applying
  those source templates during bootstrap
- AST expression child identifiers for ordinary expression, collection, and
  condition nodes now use Sley-owned parser ID tasks from `loom.parser`
- AST missing-node diagnostics now use the diagnostic id and message template
  sourced from `loom.parser`
- AST statement-node parent identifiers now use the Sley-owned parser task ID
  template instead of direct shell-side task ID synthesis
- take edit surfaces now use a Sley-owned parser template while preserving the
  existing external `take:task:...:index:name` edit-surface shape
- module task-list edit surfaces used by graph slices and edit plans now use a
  Sley-owned parser template while preserving the existing external
  `module:<module>:tasks` surface shape
- module import, type, and effect list edit surfaces used by graph slices and
  edit plans now use Sley-owned parser templates while preserving their
  existing external `module:<module>:imports|types|effects` surface shapes
- task block edit surfaces used by graph slices and edit plans now use a
  Sley-owned parser template while preserving the existing external
  `block:<task-id>` surface shape
- call-target task surfaces used by graph slices, edit plans, emitted grafts,
  transactions, and fix provenance now use a Sley-owned parser template while
  preserving the existing external `task:<module>.<task>` surface shape
- generic module edit surfaces used by edit plans now use a Sley-owned parser
  template while preserving the existing external `module:<module>` surface
  shape
- call-argument edit surfaces used by graph call-argument affordances now use a
  Sley-owned parser template while preserving the existing external
  `block:...:expr:arg:<index>` surface shape
- statement edit surfaces used by edit-plan fallback templates now use a
  Sley-owned parser template while preserving the existing external
  `block:task:<module>.<task>:stmt:<index>` surface shape
- branch statement surfaces and expression fallback surfaces used by edit plans
  now use Sley-owned parser templates while preserving the existing external
  `block:...:then:stmt:<index>` and `block:...:expr` surface shapes
- raw-source lint statement and expression nodes now use Sley-owned parser
  templates while preserving the existing external `block:...:stmt:<index>`
  and `block:...:expr` surface shapes
- migration report surfaces now use Sley-owned parser templates while preserving
  existing external surfaces for raw host adapter replacement, imported-call
  qualification, and unchecked Result propagation
- fix migration fallback surfaces now use Sley-owned parser templates while
  preserving the existing dry-run target surfaces when no lint finding is
  present
- fix style fallback surfaces now use Sley-owned parser templates while
  preserving the existing dry-run target surfaces when no lint finding is
  present
- empty-statement fix fallback surfaces now use Sley-owned parser templates
  while preserving the existing dry-run target surfaces when no lint finding is
  present
- unused and unreachable cleanup fix fallback surfaces now use Sley-owned parser
  templates while preserving the existing dry-run target surfaces when no lint
  finding is present
- constant control-flow fix fallback surfaces now use Sley-owned parser
  templates while preserving the existing dry-run target surfaces when no lint
  finding is present
- constant scalar fix fallback surfaces now use Sley-owned parser templates
  while preserving the existing dry-run target surfaces when no lint finding is
  present
- constant derived-value fix fallback surfaces now use Sley-owned parser
  templates while preserving the existing dry-run target surfaces when no lint
  finding is present
- algebra and boolean fix fallback surfaces now use Sley-owned parser templates
  while preserving the existing dry-run target surfaces when no lint finding is
  present
- boolean branch fix fallback surfaces and expression-side surface IDs now use
  Sley-owned parser templates while preserving the existing dry-run target
  surfaces when no lint finding is present
- static task and block fallback surfaces in CI runtime diagnostics, agent
  bench repair selection, plan next-actions, targeted collection block insert
  templates, and empty-task `AddTake` dry runs now use Sley-owned parser
  templates
- diagnostics and run report root layouts moved into Sley-owned report-builder
  declarations, with the shell bootstrap supplying computed values to those
  builders
- deterministic host-gate text outputs routed through Sley-owned runtime text
  tasks
- spend-prefixed output routed through a Sley-owned runtime task
- `Result<Text>` runtime value projection routed through a Sley-owned runtime
  task before shell report wrapping
- ownership evidence updated to include the new host-call and `Ok<Text>`
  runtime paths
- symbol graph replace-affordance call detection now uses the parser-owned call
  expression prefix while preserving the existing graph call-site and
  call-argument affordances
- docgen inbound-call counts and workbench call records now use the parser-owned
  call expression prefix while preserving generated reference and workbench
  report outputs
- lint unused-import, unreachable-private-task, unchecked-result,
  unqualified-imported-call, and unused-pure-binding call detection now use the
  parser-owned call expression prefix while preserving existing findings and
  replacement strings
- checker call callee and argument parsing now strips the parser-owned call
  expression prefix instead of a shell/JQ literal while preserving unknown-task,
  call-arity, and call-argument-type diagnostics
- runtime generic main-return call evaluation now strips the parser-owned call
  expression prefix instead of a shell/JQ literal while preserving bound local
  call execution
- runtime status task-call extraction now strips the parser-owned call
  expression prefix instead of a shell/AWK literal while preserving passed run
  reports
- checker and lint status task-call extraction now strips the parser-owned call
  expression prefix instead of shell/AWK literals while preserving diagnostics
  ok/error and lint ok/findings statuses
- parser expression-classifier task-call extraction now strips the parser-owned
  call expression prefix instead of shell/AWK literals while preserving
  BoolLiteral, ListLiteral, and Raw classification
- bootstrap semantic-source-count and smoke list-count task-call extraction now
  strips the parser-owned call expression prefix instead of shell/AWK literals
  while preserving source module counting and the self-hosted run smoke value
- compute-text runtime main-return call extraction now strips the parser-owned
  call expression prefix instead of shell/AWK literals while preserving
  `examples/compute.sley` runtime output
- Result-flow runtime binding-call extraction now strips the parser-owned call
  expression prefix instead of shell/AWK literals while preserving `Ok<Int>`
  result propagation for `fixtures/corpus/accepted/result_flow.sley`
- record-field runtime main-return call extraction now strips the parser-owned
  call expression prefix instead of shell/AWK literals while preserving
  `Text("Ada")` projection for `fixtures/corpus/accepted/records_and_calls.sley`
- local zero-argument runtime call extraction now strips the parser-owned call
  expression prefix instead of shell/AWK literals while preserving
  `examples/unused_private_task.sley` returning `Int(1)`
- project zero-argument runtime call extraction now strips the parser-owned
  call expression prefix instead of shell/AWK literals while preserving
  `examples/unused_import_project` returning `Int(7)` and
  `examples/duplicate_import_project` returning `Text("ready")`
- project one-argument runtime call extraction now strips the parser-owned call
  expression prefix instead of shell/AWK literals while preserving
  `examples/project`, `examples/unqualified_import_call_project`, and
  file-entry project call runtime output
- parser and checker message-template call token extraction now strips the
  parser-owned call expression prefix instead of shell/AWK literals while
  preserving AST missing-node and call-diagnostic messages
- migration, plan, and fix fallbacks for imported-call qualification now build
  the default replacement from the parser-owned call expression prefix while
  preserving the visible `call math.double(21)` replacement
- graph call-argument affordances plus plan `replace_call_arg` templates and
  emitted graft operations now strip the parser-owned call expression prefix
  instead of hardcoded `^call...` handling while preserving `21` argument
  extraction for `examples/project`
- write-mode call-site rename and remove-call-argument rewrites now use the
  parser-owned call expression prefix instead of hardcoded `call ` needles
  while preserving checked project rewrites for `math.twice` and
  `math.adjust(41)`
- runtime project-ready fallback probes now build default `math.double(21)` and
  `double(base)` call sources from the parser-owned call expression prefix;
  docgen and workbench call-tail fallback handling no longer strips hardcoded
  `^call...` text when a source does not use the configured parser prefix,
  while preserving generated reference and workbench call counts
- runtime project-ready probe tasks now import `loom.parser` and compose their
  returned call sources from `parser.call_expression_prefix()`, with the shell
  bootstrap evaluating that Sley-level text expression instead of reading a
  hardcoded literal from `loom.runtime`
- AST expression-statement detection, while-list runtime return-call parsing,
  and direct file-read runtime return-call parsing now use the parser-owned
  call expression prefix instead of hardcoded `^call...` shell/AWK patterns,
  while preserving expression-statement AST rows, `examples/collections.sley`,
  `examples/file_gate.sley`, and `examples/raw_host_migration.sley`
- Runtime host-effect dispatch now consumes the checker-owned
  `host_effect_needles` table for runtime probes and runtime authority mapping,
  with the table expanded to cover `db.query(`, `db.try_query(`, and
  `fs.write_text(` alongside the existing fallible host adapters while
  preserving agent pipeline, spend, file, database, and capability-gate
  behavior
- Lint unused-declared-effect analysis now consumes the same checker-owned
  `host_effect_needles` table instead of a hardcoded host-call regex ladder,
  including alias normalization for `DbRead`/`DbWrite` while preserving the
  `UNUSED_DECLARED_EFFECT` warning and database alias behavior
- Docgen capability reference output now derives its effect order, aliases,
  host-call names, source needles, and seed-capability arguments from the
  checker-owned `host_effect_needles` and `effect_authorization_aliases`
  tables instead of a hardcoded capability JSON literal, while preserving the
  existing ten capability entries and generated reference output
- Runtime authority now sources effect alias normalization and alias display
  strings from the checker-owned `effect_authorization_aliases` table instead
  of Python-local `DbRead`/`DbWrite` literals, while preserving scoped database
  read/write authorization and denial diagnostics
- Lint unused-declared-effect alias canonicalization now derives `DbRead` and
  `DbWrite` handling from the checker-owned `effect_authorization_aliases`
  table instead of a JQ-local hardcoded alias ladder, while preserving clean
  database alias fixtures and `UNUSED_DECLARED_EFFECT` findings
- Runtime deterministic host default texts for file writes, database
  reads/writes, agent data writes, shell output, secret reads, and spend
  authorization now come from `loom.runtime` tasks instead of inline shell
  literals, while preserving seeded host-adapter behavior
- Runtime capability-required, scope-denied, and unsupported-`main`-take
  diagnostic IDs and message parts now come from `loom.runtime` tasks instead
  of Python-local literals, while preserving runtime and CI diagnostic output
- Runtime authority's fallback database table now comes from `loom.runtime`
  instead of a Python-local `"users"` literal, while preserving database alias
  execution and scope-denied diagnostics
- Runtime run-report value-kind labels now come exclusively from
  `loom.runtime` tasks instead of shell fallback literals, while preserving
  `Text`, `Int`, `Bool`, `Unit`, and nested `Ok` runtime report shapes

## Next Frontier

The public smoke manifest is green. The remaining strict self-hosting blockers
are:

- execute parser semantics from Sley source
- execute checker semantics from Sley source
- execute runtime semantics from Sley source

Expected next investigation area, once development resumes:

- broader runtime expression execution from Sley source
- parser/checker parity coverage that does not rely on shell/JQ semantics

## External Comparison State

The Sley / Zero comparison packet is prepared outside this repository at:

- `/home/greyforge/sley-zero-public-actions/`

Current status:

- Zero upstream issue draft exists but is no longer approval-ready under the
  stricter operator gate; strict Sley self-hosting proof must come first.
- ForgeHub candidate 124 for `vercel-labs/zero#4` was checked and rejected;
  no eligible ForgeHub public action remains in the packet.
- Zero fork outreach was narrowed to avoid spam and duplicate/disabled targets.
- Latest fork audit inspected 130 forks: 5 ahead/diverged, 125 passive,
  identical, or behind, with snapshot SHA-256
  `10969d8d29094839496819ebc19d9d000e476a6cd41ff4bc5a53090a71e55f1b`.
- The added forks after the 128-fork packet were `Pvmsirish/zero` and
  `mathieuflamant/zero`; both compared identical to upstream and did not
  change the recommendation.
- Future fork-count drift can be refreshed locally with
  `/home/greyforge/sley-zero-public-actions/refresh-fork-audit.sh`; the helper
  was exercised against a temporary snapshot and reproduced the pinned hash.
- Doctrine, privacy/secrets, executor-guard, and GitHub-mechanics packet
  reviews are recorded in the external packet.
- X/social copy is drafted only and remains unposted.
- No public GitHub, ForgeHub, or social write is authorized without exact
  operator approval of the target and body.

Previous exact approval phrase, now insufficient without strict self-hosting
proof:

- `Approve posting the upstream Zero issue exactly as drafted.`

## Verification Evidence

Completed before pausing:

- `bash -n bin/sley`
- `git diff --check`
- `scripts/self-hosted-test.sh`
- `make v1`
- `bin/sley check --json self-hosted`
- `bin/sley self-hosting-status --json`
- full CLI smoke: 481/481 passing through `make v1`

Completed during the latest non-development verification refresh:

- `/home/greyforge/sley-zero-public-actions/verify-public-actions.sh`
- `/home/greyforge/sley-zero-public-actions/verify-live-targets.sh`
- `make v1`
- `bin/sley self-hosting-status --json`

The latest continuation audit reconfirmed:

- public-action packet verifier: passed
- live Zero target verifier: passed
- focused report probes for CI, contract, workbench, graft, and ZJX report
  routing: passed
- `scripts/self-hosted-test.sh`: passed
- `bin/sley-contract check-fixtures fixtures/contracts --schemas docs/schemas
  --json`: passed with `failed_count=0`
- `git diff --check`: passed
- `make v1`: passed
- Sley self-hosting status before resumed local proof work: `bootstrap`,
  `strict_self_hosted=false`, `semantic_source_count=6`,
  `bootstrap_owned_by_sley=132`
- Sley self-hosting status after report-builder proof slices:
  `bootstrap`, `strict_self_hosted=false`, `semantic_source_count=6`,
  `bootstrap_owned_by_sley=164`
- Sley self-hosting status after the final report-builder proof slice:
  `bootstrap`, `strict_self_hosted=false`, `semantic_source_count=6`,
  `bootstrap_owned_by_sley=180`; the report-builder blocker is removed
- focused AST builder probe:
  `bin/sley ast --json examples/declaration_hygiene.sley` preserved schema
  `sley.ast.program.v0` and returned the expected one task, one type, and one
  effect summary
- Sley self-hosting status after the AST program builder proof slice:
  `bootstrap`, `strict_self_hosted=false`, `semantic_source_count=6`,
  `bootstrap_owned_by_sley=182`; remaining blockers are parser, checker, and
  runtime semantics from Sley source
- focused AST node builder probes:
  `bin/sley ast --json --node task:app.hello.main examples/hello.sley`,
  `bin/sley ast --json --node block:task:app.hello.main:stmt:0:expr
  examples/hello.sley`, and the missing-node negative path preserved their
  expected `sley.ast.node.v0` / diagnostics output
- Sley self-hosting status after the AST node builder proof slice:
  `bootstrap`, `strict_self_hosted=false`, `semantic_source_count=6`,
  `bootstrap_owned_by_sley=186`; remaining blockers are parser, checker, and
  runtime semantics from Sley source
- focused AST declaration/take ID probes:
  `bin/sley ast --json examples/declaration_hygiene.sley` returned
  `type:app.hygiene.Orphan` and `effect:app.hygiene.OrphanEffect`, and
  `bin/sley ast --json examples/unused_take.sley` returned
  `take:app.takes.main.value` and `take:app.takes.main.unused`
- Sley self-hosting status after the parser AST declaration/take ID proof
  slice: `bootstrap`, `strict_self_hosted=false`,
  `semantic_source_count=6`, `bootstrap_owned_by_sley=188`; remaining blockers
  are parser, checker, and runtime semantics from Sley source
- focused AST expression child ID probes:
  `bin/sley ast --json examples/hello.sley` returned
  `block:task:app.hello.main:stmt:0:expr`,
  `bin/sley ast --json examples/empty_for_statement.sley` returned
  `block:task:app.empty_for.main:stmt:1:collection`, and
  `bin/sley ast --json examples/empty_while_statement.sley` returned
  `block:task:app.empty_while.main:stmt:2:condition`
- Sley self-hosting status after the parser AST expression child ID proof
  slice: `bootstrap`, `strict_self_hosted=false`,
  `semantic_source_count=6`, `bootstrap_owned_by_sley=190`; remaining blockers
  are parser, checker, and runtime semantics from Sley source
- focused AST missing-node message probe:
  `bin/sley ast --json --node missing examples/hello.sley` returned diagnostic
  id `AST_NODE_NOT_FOUND` and message ``AST node not found `missing` `` from the
  Sley-owned parser message template
- Sley self-hosting status after the parser AST missing-node message proof
  slice: `bootstrap`, `strict_self_hosted=false`,
  `semantic_source_count=6`, `bootstrap_owned_by_sley=191`; remaining blockers
  are parser, checker, and runtime semantics from Sley source
- focused AST statement parent ID probe:
  `bin/sley ast --json --node block:task:app.hello.main:stmt:0
  examples/hello.sley` returned `node_kind == "statement"` and parent
  `task:app.hello.main` through the Sley-owned parser task ID template
- Sley self-hosting status after the parser AST statement parent ID proof
  slice: `bootstrap`, `strict_self_hosted=false`,
  `semantic_source_count=6`, `bootstrap_owned_by_sley=192`; remaining blockers
  are parser, checker, and runtime semantics from Sley source
- focused take edit-surface probes:
  `bin/sley lint --json --rule unused_take examples/unused_take.sley`
  returned `take:task:app.takes.main:1:unused`; targeted graft-template and
  dry-run fix probes preserved the same take edit-surface shape for move and
  remove operations
- Sley self-hosting status after the parser take edit-surface ID proof slice:
  `bootstrap`, `strict_self_hosted=false`, `semantic_source_count=6`,
  `bootstrap_owned_by_sley=193`; remaining blockers are parser, checker, and
  runtime semantics from Sley source
- focused module task-list surface probes:
  `bin/sley graph --json --slice task:app.main.main examples/project` and
  `bin/sley plan --json --graft-templates examples/project` preserved
  `module:app.main:tasks` for task add/move affordances while sourcing the
  template from `loom.parser`
- Sley self-hosting status after the parser module task-list surface ID proof
  slice: `bootstrap`, `strict_self_hosted=false`,
  `semantic_source_count=6`, `bootstrap_owned_by_sley=194`; remaining blockers
  are parser, checker, and runtime semantics from Sley source
- focused module declaration-list surface probes:
  `bin/sley graph --json --slice module:app.math examples/project` preserved
  `module:app.math:imports`, `module:app.math:types`, and
  `module:app.math:effects`; targeted plan probes for
  `module:app.hygiene` preserved type/effect move parents while sourcing the
  templates from `loom.parser`
- Sley self-hosting status after the parser module declaration-list surface ID
  proof slice: `bootstrap`, `strict_self_hosted=false`,
  `semantic_source_count=6`, `bootstrap_owned_by_sley=195`; remaining blockers
  are parser, checker, and runtime semantics from Sley source
- focused task block surface probes:
  `bin/sley graph --json --slice task:app.main.main examples/project` and
  `bin/sley plan --json --graft-templates examples/project` preserved
  `block:task:app.main.main` for statement insert/move affordances while
  sourcing the template from `loom.parser`
- Sley self-hosting status after the parser block task surface ID proof slice:
  `bootstrap`, `strict_self_hosted=false`,
  `semantic_source_count=6`, `bootstrap_owned_by_sley=196`; remaining blockers
  are parser, checker, and runtime semantics from Sley source
- focused call-target task surface probes:
  `bin/sley graph --json --slice task:app.main.main examples/project`,
  `bin/sley plan --json --graft-templates examples/project`,
  `bin/sley plan --json --emit-graft replace_call_arg examples/project`, and
  `bin/sley fix --json --kind replace_call_arg --dry-run examples/project`
  preserved `task:app.math.double` while sourcing the target template from
  `loom.parser`
- Sley self-hosting status after the parser call-target task ID proof slice:
  `bootstrap`, `strict_self_hosted=false`,
  `semantic_source_count=6`, `bootstrap_owned_by_sley=197`; remaining blockers
  are parser, checker, and runtime semantics from Sley source
- focused module edit-surface probe:
  `bin/sley plan --json --graft-templates --template-surface
  module:app.hygiene examples/declaration_hygiene.sley` preserved
  `module:app.hygiene` for targeted module graft templates while preserving
  type/effect move parents
- Sley self-hosting status after the parser module edit-surface ID proof slice:
  `bootstrap`, `strict_self_hosted=false`,
  `semantic_source_count=6`, `bootstrap_owned_by_sley=198`; remaining blockers
  are parser, checker, and runtime semantics from Sley source
- focused call-argument surface probe:
  `bin/sley graph --json --slice task:app.main.main examples/project`
  preserved `block:task:app.main.main:stmt:0:expr:arg:0` for the call-argument
  affordance while preserving `task:app.math.double` as the task target
- Sley self-hosting status after the parser call-argument surface ID proof
  slice: `bootstrap`, `strict_self_hosted=false`,
  `semantic_source_count=6`, `bootstrap_owned_by_sley=199`; remaining blockers
  are parser, checker, and runtime semantics from Sley source
- focused statement surface probe:
  `bin/sley plan --json --graft-templates examples/project` preserved
  `block:task:app.main.main:stmt:0` for move/delete statement template targets
  while preserving `block:task:app.main.main` as the block parent
- Sley self-hosting status after the parser statement surface ID proof slice:
  `bootstrap`, `strict_self_hosted=false`,
  `semantic_source_count=6`, `bootstrap_owned_by_sley=200`; remaining blockers
  are parser, checker, and runtime semantics from Sley source
- focused branch statement/expression surface probe:
  `bin/sley plan --json --graft-templates --template-surface
  task:app.constant_false_if.main examples/constant_false_if_statement.sley`
  preserved `block:task:app.constant_false_if.main:stmt:0:then:stmt:0` for the
  nested branch statement move destination and
  `block:task:app.constant_false_if.main:stmt:0:expr` for the replacement
  expression fallback surface
- Sley self-hosting status after the parser branch statement/expression
  surface ID proof slice: `bootstrap`, `strict_self_hosted=false`,
  `semantic_source_count=6`, `bootstrap_owned_by_sley=201`; remaining blockers
  are parser, checker, and runtime semantics from Sley source
- focused raw-source lint surface probes:
  `bin/sley lint --json --rule empty_for_statement
  examples/empty_for_statement.sley` preserved
  `block:task:app.empty_for.main:stmt:1`;
  `bin/sley lint --json --rule unchecked_result examples/unchecked_result.sley`
  preserved `block:task:app.unchecked.main:stmt:0:expr`;
  `bin/sley lint --json --rule unreachable_statement
  examples/unreachable_statement.sley` preserved
  `block:task:app.unreachable_statement.main:stmt:2`; and
  `bin/sley lint --json --rule unused_pure_expression_statement
  examples/unused_pure_expression_statement.sley` preserved
  `block:task:app.unused_expr.main:stmt:1`
- Sley self-hosting status after the parser lint statement surface ID proof
  slice: `bootstrap`, `strict_self_hosted=false`,
  `semantic_source_count=6`, `bootstrap_owned_by_sley=202`; remaining blockers
  are parser, checker, and runtime semantics from Sley source
- focused migration surface probes:
  `bin/sley sley-migrate report --json examples/raw_host_migration.sley`
  preserved `block:task:main.main:stmt:0:expr`;
  `bin/sley sley-migrate report --json examples/unqualified_import_call_project`
  preserved `block:task:app.main.main:stmt:0:expr`;
  `bin/sley sley-migrate report --json examples/unchecked_result_binding.sley`
  preserved `block:task:app.unchecked_binding.main:stmt:0`; and
  `bin/sley sley-migrate report --json examples/unchecked_result.sley`
  preserved `block:task:app.unchecked.main:stmt:0:expr`
- Sley self-hosting status after the parser migration surface ID proof slice:
  `bootstrap`, `strict_self_hosted=false`,
  `semantic_source_count=6`, `bootstrap_owned_by_sley=203`; remaining blockers
  are parser, checker, and runtime semantics from Sley source
- focused fix migration fallback probes against `examples/hello.sley`:
  `bin/sley fix --json --kind migrate_raw_host_adapter --dry-run
  examples/hello.sley` preserved
  `block:task:app.raw_migration.main:stmt:0:expr`;
  `bin/sley fix --json --kind qualify_imported_call --dry-run
  examples/hello.sley` preserved `block:task:app.main.main:stmt:0:expr`;
  `bin/sley fix --json --kind propagate_unchecked_result_binding --dry-run
  examples/hello.sley` preserved
  `block:task:app.unchecked_binding.main:stmt:0`; and
  `bin/sley fix --json --kind propagate_unchecked_result --dry-run
  examples/hello.sley` preserved
  `block:task:app.unchecked.main:stmt:0:expr`
- Sley self-hosting status after the parser fix-migration fallback surface ID
  proof slice: `bootstrap`, `strict_self_hosted=false`,
  `semantic_source_count=6`, `bootstrap_owned_by_sley=204`; remaining blockers
  are parser, checker, and runtime semantics from Sley source
- focused fix style fallback probes against `examples/hello.sley`:
  `bin/sley fix --json --kind convert_mutable_binding_to_bind --dry-run
  examples/hello.sley` preserved
  `block:task:app.mutable_fix.main:stmt:0` and
  `block:task:app.mutable_fix.main`;
  `bin/sley fix --json --kind delete_self_assignment_statement --dry-run
  examples/hello.sley` preserved
  `block:task:app.self_assignment_fix.main:stmt:1`;
  `bin/sley fix --json --kind convert_redundant_initial_set_to_bind --dry-run
  examples/hello.sley` preserved
  `block:task:app.redundant_initial_set_fix.main:stmt:0` and
  `block:task:app.redundant_initial_set_fix.main:stmt:1`; and
  `bin/sley fix --json --kind fold_redundant_initial_set_into_binding
  --dry-run examples/hello.sley` preserved
  `block:task:app.redundant_initial_set_fix.main:stmt:0:expr` and
  `block:task:app.redundant_initial_set_fix.main:stmt:1`
- Sley self-hosting status after the parser fix-style fallback surface ID proof
  slice: `bootstrap`, `strict_self_hosted=false`,
  `semantic_source_count=6`, `bootstrap_owned_by_sley=205`; remaining blockers
  are parser, checker, and runtime semantics from Sley source
- focused empty-statement fix fallback probes against `examples/hello.sley`:
  `bin/sley fix --json --kind delete_empty_for_statement --dry-run
  examples/hello.sley` preserved `block:task:app.empty_for.main:stmt:1`;
  `bin/sley fix --json --kind delete_empty_forge_statement --dry-run
  examples/hello.sley` preserved `block:task:app.empty_forge.main:stmt:1`;
  `bin/sley fix --json --kind delete_empty_if_statement --dry-run
  examples/hello.sley` preserved `block:task:app.empty_if.main:stmt:1`;
  `bin/sley fix --json --kind delete_empty_while_statement --dry-run
  examples/hello.sley` preserved `block:task:app.empty_while.main:stmt:2`;
  and `bin/sley fix --json --kind remove_empty_else_statement --dry-run
  examples/hello.sley` preserved `block:task:app.empty_else.main:stmt:1`
- Sley self-hosting status after the parser fix-empty fallback surface ID proof
  slice: `bootstrap`, `strict_self_hosted=false`,
  `semantic_source_count=6`, `bootstrap_owned_by_sley=206`; remaining blockers
  are parser, checker, and runtime semantics from Sley source
- focused unused/unreachable cleanup fix fallback probes against
  `examples/hello.sley`: `bin/sley fix --json --kind
  delete_unreachable_statement --dry-run examples/hello.sley` preserved
  `block:task:app.unreachable_statement.main:stmt:2`; `bin/sley fix --json
  --kind delete_unused_pure_binding --dry-run examples/hello.sley` preserved
  `block:task:app.bindings.main:stmt:0`; `bin/sley fix --json --kind
  delete_unused_pure_expression_statement --dry-run examples/hello.sley`
  preserved `block:task:app.unused_expr.main:stmt:1`; and `bin/sley fix
  --json --kind drop_unused_effectful_binding_value --dry-run
  examples/hello.sley` preserved
  `block:task:app.effectful_cleanup.main:stmt:0`
- Sley self-hosting status after the parser fix-unused/unreachable fallback
  surface ID proof slice: `bootstrap`, `strict_self_hosted=false`,
  `semantic_source_count=6`, `bootstrap_owned_by_sley=207`; remaining blockers
  are parser, checker, and runtime semantics from Sley source
- focused constant control-flow fix fallback probes against
  `examples/hello.sley`: `bin/sley fix --json --kind
  simplify_constant_if_expression --dry-run examples/hello.sley` preserved
  `block:task:app.constant_fix.main:stmt:0:expr`; `bin/sley fix --json --kind
  simplify_constant_if_statement --dry-run examples/hello.sley` preserved
  `block:task:app.constant_if_statement_fix.main:stmt:1`; `bin/sley fix
  --json --kind delete_constant_false_if_statement --dry-run
  examples/hello.sley` preserved
  `block:task:app.constant_false_if_fix.main:stmt:0`; and `bin/sley fix
  --json --kind delete_constant_false_while_statement --dry-run
  examples/hello.sley` preserved `block:task:app.false_while_fix.main:stmt:1`
- Sley self-hosting status after the parser fix-constant-control fallback
  surface ID proof slice: `bootstrap`, `strict_self_hosted=false`,
  `semantic_source_count=6`, `bootstrap_owned_by_sley=208`; remaining blockers
  are parser, checker, and runtime semantics from Sley source
- focused constant scalar fix fallback probes against `examples/hello.sley`:
  `bin/sley fix --json --kind simplify_constant_comparison_expression --dry-run
  examples/hello.sley` preserved
  `block:task:app.constant_compare_fix.main:stmt:0:expr`; `bin/sley fix
  --json --kind simplify_constant_boolean_comparison_expression --dry-run
  examples/hello.sley` preserved
  `block:task:app.constant_bool_compare_fix.main:stmt:0:expr`; `bin/sley fix
  --json --kind simplify_constant_arithmetic_expression --dry-run
  examples/hello.sley` preserved
  `block:task:app.constant_arithmetic_fix.main:stmt:0:expr`; `bin/sley fix
  --json --kind simplify_absorbing_arithmetic_expression --dry-run
  examples/hello.sley` preserved
  `block:task:app.absorbing_arithmetic_fix.main:stmt:0:expr`; and
  `bin/sley fix --json --kind simplify_constant_text_concatenation_expression
  --dry-run examples/hello.sley` preserved
  `block:task:app.constant_text_concat_fix.main:stmt:0:expr`
- Sley self-hosting status after the parser fix-constant-scalar fallback
  surface ID proof slice: `bootstrap`, `strict_self_hosted=false`,
  `semantic_source_count=6`, `bootstrap_owned_by_sley=209`; remaining blockers
  are parser, checker, and runtime semantics from Sley source
- focused constant derived-value fix fallback probes against
  `examples/hello.sley`: `bin/sley fix --json --kind
  simplify_constant_list_index_expression --dry-run examples/hello.sley`
  preserved `block:task:app.constant_list_index_fix.main:stmt:0:expr`;
  `bin/sley fix --json --kind simplify_constant_map_index_expression --dry-run
  examples/hello.sley` preserved
  `block:task:app.constant_map_index_fix.main:stmt:0:expr`; `bin/sley fix
  --json --kind simplify_constant_record_field_access_expression --dry-run
  examples/hello.sley` preserved
  `block:task:app.constant_record_field_access_fix.main:stmt:0:expr`;
  `bin/sley fix --json --kind simplify_constant_len_expression --dry-run
  examples/hello.sley` preserved
  `block:task:app.constant_len_fix.main:stmt:0:expr`; and `bin/sley fix
  --json --kind simplify_constant_not_expression --dry-run
  examples/hello.sley` preserved
  `block:task:app.constant_not_fix.main:stmt:0:expr`
- Sley self-hosting status after the parser fix-constant-derived fallback
  surface ID proof slice: `bootstrap`, `strict_self_hosted=false`,
  `semantic_source_count=6`, `bootstrap_owned_by_sley=210`; remaining blockers
  are parser, checker, and runtime semantics from Sley source
- focused algebra and boolean fix fallback probes against
  `examples/hello.sley`: `bin/sley fix --json --kind
  simplify_identity_binary_expression --dry-run examples/hello.sley` preserved
  `block:task:app.identity_fix.main:stmt:1:expr`; `bin/sley fix --json --kind
  simplify_redundant_boolean_comparison --dry-run examples/hello.sley`
  preserved `block:task:app.boolean_fix.main:stmt:1:expr`; `bin/sley fix
  --json --kind simplify_absorbing_boolean_expression --dry-run
  examples/hello.sley` preserved
  `block:task:app.absorbing_boolean_fix.main:stmt:0:expr`; `bin/sley fix
  --json --kind simplify_idempotent_boolean_expression --dry-run
  examples/hello.sley` preserved
  `block:task:app.idempotent_boolean_fix.main:stmt:1:expr`; `bin/sley fix
  --json --kind simplify_self_comparison_expression --dry-run
  examples/hello.sley` preserved
  `block:task:app.self_compare_fix.main:stmt:0:expr`; `bin/sley fix --json
  --kind simplify_double_negation_expression --dry-run examples/hello.sley`
  preserved `block:task:app.double_fix.main:stmt:1:expr`; and
  `bin/sley fix --json --kind simplify_negated_comparison_expression --dry-run
  examples/hello.sley` preserved
  `block:task:app.negated_compare_fix.main:stmt:1:expr`
- Sley self-hosting status after the parser fix-algebra/boolean fallback
  surface ID proof slice: `bootstrap`, `strict_self_hosted=false`,
  `semantic_source_count=6`, `bootstrap_owned_by_sley=211`; remaining blockers
  are parser, checker, and runtime semantics from Sley source
- focused boolean branch fix fallback probes against `examples/hello.sley`:
  `bin/sley fix --json --kind simplify_redundant_boolean_if_expression
  --dry-run examples/hello.sley` preserved
  `block:task:app.boolean_if_fix.main:stmt:1:expr`; `bin/sley fix --json
  --kind simplify_redundant_boolean_if_statement --dry-run
  examples/hello.sley` preserved
  `block:task:app.boolean_if_statement_fix.main:stmt:1`; `bin/sley fix
  --json --kind simplify_same_branch_if_expression --dry-run
  examples/hello.sley` preserved
  `block:task:app.same_branch_fix.main:stmt:2:expr:left`; and `bin/sley fix
  --json --kind simplify_same_branch_if_statement --dry-run
  examples/hello.sley` preserved
  `block:task:app.same_branch_statement_fix.main:stmt:1`
- Sley self-hosting status after the parser fix-boolean-branch fallback
  surface ID proof slice: `bootstrap`, `strict_self_hosted=false`,
  `semantic_source_count=6`, `bootstrap_owned_by_sley=213`; remaining blockers
  are parser, checker, and runtime semantics from Sley source
- focused static task/block fallback probes:
  `bin/sley plan --json --graft-templates --template-surface
  block:task:app.collections.sum examples/collections.sley` preserved
  `block:task:app.collections.sum` with insert position `4`; `bin/sley plan
  --json --graft-templates examples/agent_deploy_pipeline.sley` preserved
  `task:app.agent_deploy_pipeline.main` in the primary graft template and
  first graph-slice next action; `bin/sley graft --json --dry-run` against a
  module-only temp source and targetless `AddTake` operation preserved fallback
  `task:app.main.main`; `bin/sley-ci verify --json
  examples/agent_deploy_pipeline.sley` failed as expected without runtime
  gates and preserved diagnostic node `task:app.agent_deploy_pipeline.main`;
  and `bin/sley-agent-bench run --json --case unused-private-task-repair`
  preserved selected repair surface `task:app.bench.orphan`
- Sley self-hosting status after the parser static task/block fallback surface
  ID proof slice: `bootstrap`, `strict_self_hosted=false`,
  `semantic_source_count=6`, `bootstrap_owned_by_sley=215`; remaining blockers
  are parser, checker, and runtime semantics from Sley source
- focused checker static type-name probes: a temp source using
  `Result<Text, Error>`, `Gate<FileRead>`, `List<Text>`, and `Map<Text, Int>`
  checked cleanly, while a temp `NotAType` return still produced
  `UNKNOWN_TYPE` with the expected unknown-type message for `NotAType`
- Sley self-hosting status after the checker static type-name source slice:
  `bootstrap`, `strict_self_hosted=false`, `semantic_source_count=6`,
  `bootstrap_owned_by_sley=217`; remaining blockers are still parser,
  checker, and runtime semantics from Sley source
- focused checker host-effect table probes: a temp source using `DbRead` to
  authorize `db.query_one(` checked cleanly through the Sley-sourced alias
  table, while temp sources missing capabilities still produced
  `EFFECT_UNAUTHORIZED` for `DatabaseRead` and `Spend` through the Sley-sourced
  host-effect needle table
- Sley self-hosting status after the checker host-effect table source slice:
  `bootstrap`, `strict_self_hosted=false`, `semantic_source_count=6`,
  `bootstrap_owned_by_sley=219`; remaining blockers are still parser,
  checker, and runtime semantics from Sley source
- focused checker gate binding-kind probes: a temp source with a `Gate<DatabaseRead>`
  take was excluded from ordinary call arity and checked cleanly, while a temp
  source with `Gate<FileRead>` on a task without `uses FileRead` still produced
  `GATE_EFFECT_UNDECLARED`
- Sley self-hosting status after the checker gate binding-kind source slice:
  `bootstrap`, `strict_self_hosted=false`, `semantic_source_count=6`,
  `bootstrap_owned_by_sley=220`; remaining blockers are still parser,
  checker, and runtime semantics from Sley source
- focused checker repair hint-kind probes: existing rejected fixtures still
  emitted Sley-sourced repair hint kinds for `UNKNOWN_TASK`
  (`declare_or_import_task`), `RETURN_TYPE_MISMATCH` (`inspect_return_type`,
  `replace_task_body`, `replace_expression`), and `MISSING_RETURN`
  (`insert_return`, `replace_task_body`)
- Sley self-hosting status after the checker repair hint-kind source slice:
  `bootstrap`, `strict_self_hosted=false`, `semantic_source_count=6`,
  `bootstrap_owned_by_sley=221`; remaining blockers are still parser,
  checker, and runtime semantics from Sley source
- focused checker identifier expression-kind probe:
  `fixtures/corpus/rejected/unknown_identifier.sley` still emitted
  `UNKNOWN_IDENTIFIER` at `block:task:corpus.rejected.main:stmt:0:expr` after
  the checker comparison moved from literal `"Identifier"` to the parser-owned
  identifier expression kind
- Sley self-hosting status after the checker identifier expression-kind source
  slice: `bootstrap`, `strict_self_hosted=false`,
  `semantic_source_count=6`, `bootstrap_owned_by_sley=222`; remaining blockers
  are still parser, checker, and runtime semantics from Sley source
- focused checker call expression-prefix probes:
  `fixtures/corpus/rejected/unknown_task.sley` still emitted `UNKNOWN_TASK`
  with `declare_or_import_task`, `fixtures/corpus/rejected/call_arity_mismatch.sley`
  still emitted `CALL_ARITY_MISMATCH`, and
  `fixtures/corpus/rejected/unauthorized_effect.sley` still emitted
  `EFFECT_UNAUTHORIZED` after the checker call-expression tests moved from
  literal `"call "` to the parser-owned call expression prefix
- Sley self-hosting status after the checker call expression-prefix source
  slice: `bootstrap`, `strict_self_hosted=false`,
  `semantic_source_count=6`, `bootstrap_owned_by_sley=224`; remaining blockers
  are still parser, checker, and runtime semantics from Sley source
- focused query call expression-prefix probes:
  `bin/sley query --json examples/project` still reported one call from
  `app.main.main` to `app.math.double`, preserved callee `math.double`, and
  preserved task inbound/outbound call counts after query call detection moved
  from literal `"call "` to the parser-owned call expression prefix
- Sley self-hosting status after the query call expression-prefix source
  slice: `bootstrap`, `strict_self_hosted=false`,
  `semantic_source_count=6`, `bootstrap_owned_by_sley=225`; remaining blockers
  are still parser, checker, and runtime semantics from Sley source
- focused symbol graph call expression-prefix probes:
  `bin/sley graph --json --slice task:app.main.main examples/project` preserved
  the replacement affordance target `block:task:app.main.main:stmt:0:expr` as
  `target_kind == "Call"`, while preserving call-site and call-argument
  affordances targeting `task:app.math.double`, after graph replace-affordance
  call detection moved from literal `"call "` to the parser-owned call
  expression prefix
- Sley self-hosting status after the symbol graph call expression-prefix source
  slice: `bootstrap`, `strict_self_hosted=false`,
  `semantic_source_count=6`, `bootstrap_owned_by_sley=226`; remaining blockers
  are still parser, checker, and runtime semantics from Sley source
- focused docgen/workbench call expression-prefix probes:
  `bin/sley-docgen reference --json --module agent.pipeline
  examples/agent_project` preserved three `agent.pipeline` task summaries, each
  with `inbound_call_count == 1`; `bin/sley-workbench --json --slice
  task:app.agent_deploy_pipeline.main examples/agent_deploy_pipeline.sley`
  preserved `summary.call_count == 7`, four workbench query calls, and four
  outbound graph-slice calls after docgen/workbench call detection moved from
  literal `"call "` to the parser-owned call expression prefix
- Sley self-hosting status after the docgen/workbench call expression-prefix
  source slice: `bootstrap`, `strict_self_hosted=false`,
  `semantic_source_count=6`, `bootstrap_owned_by_sley=228`; remaining blockers
  are still parser, checker, and runtime semantics from Sley source
- focused lint call expression-prefix probes:
  `bin/sley lint --json --rule unchecked_result examples/unchecked_result.sley`
  still reported `block:task:app.unchecked.main:stmt:0:expr`;
  `bin/sley lint --json --rule unchecked_result examples/result_flow.sley`
  stayed clean; `bin/sley lint --json --rule unused_import
  examples/unused_import_project` still reported
  `import:app.main:app.stale`; and `bin/sley lint --json --rule
  unqualified_imported_call examples/unqualified_import_call_project`
  preserved replacement `call math.double(21)` after the lint call-expression
  rules moved from literal `"call "` to the parser-owned call expression prefix
- Sley self-hosting status after the lint call expression-prefix source slice:
  `bootstrap`, `strict_self_hosted=false`, `semantic_source_count=6`,
  `bootstrap_owned_by_sley=229`; remaining blockers are still parser, checker,
  and runtime semantics from Sley source
- latest lint call expression-prefix verification: `bash -n bin/sley`,
  `bash -n scripts/self-hosted-test.sh`, focused lint probes,
  `scripts/self-hosted-test.sh`, `bin/sley-contract check-fixtures
  fixtures/contracts --schemas docs/schemas --json`, external public-action
  packet verifier, `git diff --check`, and `make v1` all passed
- focused checker call parser-prefix probes:
  `bin/sley check --json fixtures/corpus/rejected/unknown_task.sley` still
  emitted `UNKNOWN_TASK` for `missing` and preserved generated repair takes
  `arg0: Int` and `arg1: Text`; `bin/sley check --json
  fixtures/corpus/rejected/call_arity_mismatch.sley` still emitted
  `CALL_ARITY_MISMATCH`; and `bin/sley check --json
  fixtures/corpus/rejected/call_argument_type_mismatch.sley` still emitted
  `CALL_ARGUMENT_TYPE_MISMATCH` after checker callee and argument parsing
  moved from literal `^call...` handling to the parser-owned call expression
  prefix
- Sley self-hosting status after the checker call parser-prefix source slice:
  `bootstrap`, `strict_self_hosted=false`, `semantic_source_count=6`,
  `bootstrap_owned_by_sley=230`; remaining blockers are still parser, checker,
  and runtime semantics from Sley source
- latest checker call parser-prefix verification: `bash -n bin/sley`,
  `bash -n scripts/self-hosted-test.sh`, focused checker diagnostics probes,
  `scripts/self-hosted-test.sh`, `bin/sley-contract check-fixtures
  fixtures/contracts --schemas docs/schemas --json`, external public-action
  packet verifier, `git diff --check`, and final `make v1` all passed
- focused runtime main-call parser-prefix probe:
  `bin/sley run --json fixtures/corpus/accepted/pure_main.sley` still returned
  `status == "passed"`, `value.kind == "Int"`, and `value.value == 42` after
  runtime main-call parsing moved from literal `^call...` handling to the
  parser-owned call expression prefix
- Sley self-hosting status after the runtime main-call parser-prefix source
  slice: `bootstrap`, `strict_self_hosted=false`,
  `semantic_source_count=6`, `bootstrap_owned_by_sley=231`; remaining blockers
  are still parser, checker, and runtime semantics from Sley source
- latest runtime main-call parser-prefix verification: `bash -n bin/sley`,
  `bash -n scripts/self-hosted-test.sh`, focused runtime/status probes,
  `scripts/self-hosted-test.sh`, `bin/sley-contract check-fixtures
  fixtures/contracts --schemas docs/schemas --json`, external public-action
  packet verifier, `git diff --check`, and final `make v1` all passed
- focused runtime status parser-prefix probe:
  `bin/sley run --json examples/hello.sley` still returned
  `schema == "sley.run.report.v0"`, `status == "passed"`,
  `value.kind == "Text"`, and `value.value == "hello sley"` after
  `runtime_status` call extraction moved from literal `return call ...`
  handling to the parser-owned call expression prefix
- Sley self-hosting status after the runtime status parser-prefix source slice:
  `bootstrap`, `strict_self_hosted=false`, `semantic_source_count=6`,
  `bootstrap_owned_by_sley=232`; remaining blockers are still parser, checker,
  and runtime semantics from Sley source
- latest runtime status parser-prefix verification: `bash -n bin/sley`,
  `bash -n scripts/self-hosted-test.sh`, focused runtime/status probes,
  `scripts/self-hosted-test.sh`, `bin/sley-contract check-fixtures
  fixtures/contracts --schemas docs/schemas --json`, external public-action
  packet verifier, `git diff --check`, and final `make v1` all passed
- focused checker/lint status parser-prefix probes:
  `bin/sley check --json examples/hello.sley` still returned
  `schema == "sley.diagnostics.report.v0"`, `status == "ok"`, and zero
  diagnostics; `bin/sley check --json
  fixtures/corpus/rejected/unknown_identifier.sley` still emitted
  `UNKNOWN_IDENTIFIER` and exited through the expected error path; `bin/sley
  lint --json examples/hello.sley` still returned
  `schema == "sley.lint.report.v0"`, `status == "ok"`, and zero findings; and
  `bin/sley lint --json --rule unused_private_task
  examples/unused_private_task.sley` still returned `status == "findings"`
  with `unused_private_task`
- Sley self-hosting status after the checker/lint status parser-prefix source
  slice: `bootstrap`, `strict_self_hosted=false`,
  `semantic_source_count=6`, `bootstrap_owned_by_sley=234`; remaining blockers
  are still parser, checker, and runtime semantics from Sley source
- latest checker/lint status parser-prefix verification:
  `bash -n bin/sley`, `bash -n scripts/self-hosted-test.sh`, focused
  checker/lint status probes, `scripts/self-hosted-test.sh`,
  `bin/sley-contract check-fixtures fixtures/contracts --schemas docs/schemas
  --json`, external public-action packet verifier, `git diff --check`, and
  final `make v1` all passed
- focused parser classifier parser-prefix probes:
  `bin/sley ast --json examples/redundant_boolean_if_statement.sley` preserved
  `BoolLiteral` classification for `true` and `Raw` classification for the
  raw `if ready {` condition expression, and a temp source with `return []`
  preserved `ListLiteral` classification after parser classifier task-call
  extraction moved from literal `return call ...` handling to the parser-owned
  call expression prefix
- Sley self-hosting status after the parser classifier parser-prefix source
  slice: `bootstrap`, `strict_self_hosted=false`,
  `semantic_source_count=6`, `bootstrap_owned_by_sley=235`; remaining blockers
  are still parser, checker, and runtime semantics from Sley source
- latest parser classifier parser-prefix verification:
  `bash -n bin/sley`, `bash -n scripts/self-hosted-test.sh`, focused parser
  classifier probes, `scripts/self-hosted-test.sh`, `bin/sley-contract
  check-fixtures fixtures/contracts --schemas docs/schemas --json`, external
  public-action packet verifier, `git diff --check`, and final `make v1` all
  passed
- focused bootstrap bind-call parser-prefix probes:
  `bin/sley self-hosting-status --json` still returned
  `semantic_source_count == 6`, six source modules, and both
  `semantic_source_count_parser_prefix_task_execution` and
  `bootstrap_smoke_parser_prefix_task_execution`; `bin/sley run --json
  self-hosted` still returned `status == "passed"`, `value.kind == "Int"`,
  and `value.value == 51` after bootstrap zero-arg bind call extraction moved
  from literal `call ...()` handling to the parser-owned call expression prefix
- Sley self-hosting status after the bootstrap bind-call parser-prefix source
  slice: `bootstrap`, `strict_self_hosted=false`,
  `semantic_source_count=6`, `bootstrap_owned_by_sley=237`; remaining blockers
  are still parser, checker, and runtime semantics from Sley source
- latest bootstrap bind-call parser-prefix verification:
  `bash -n bin/sley`, `bash -n scripts/self-hosted-test.sh`, focused
  self-hosting/run probes, `scripts/self-hosted-test.sh`, `bin/sley-contract
  check-fixtures fixtures/contracts --schemas docs/schemas --json`, external
  public-action packet verifier, `git diff --check`, and final `make v1` all
  passed
- focused compute runtime call parser-prefix probes:
  `bin/sley run --json examples/compute.sley` still returned
  `status == "passed"`, `value.kind == "Text"`, and
  `value.value == "excellent"` after compute-text main-return call extraction
  moved from literal `return call ...` handling to the parser-owned call
  expression prefix
- Sley self-hosting status after the compute runtime call parser-prefix source
  slice: `bootstrap`, `strict_self_hosted=false`,
  `semantic_source_count=6`, `bootstrap_owned_by_sley=238`; remaining blockers
  are still parser, checker, and runtime semantics from Sley source
- latest compute runtime call parser-prefix verification:
  `bash -n bin/sley`, `bash -n scripts/self-hosted-test.sh`, focused compute
  runtime probe, `scripts/self-hosted-test.sh`, `bin/sley-contract
  check-fixtures fixtures/contracts --schemas docs/schemas --json`, external
  public-action packet verifier, `git diff --check`, and final `make v1` all
  passed
- focused result-flow runtime call parser-prefix probe:
  `bin/sley run --json fixtures/corpus/accepted/result_flow.sley` still
  returned `status == "passed"`, `value.kind == "Ok"`,
  `value.value.kind == "Int"`, and `value.value.value == 42` after
  binding-call extraction moved from literal `call ...?` handling to the
  parser-owned call expression prefix
- Sley self-hosting status after the result-flow runtime call parser-prefix
  source slice: `bootstrap`, `strict_self_hosted=false`,
  `semantic_source_count=6`, `bootstrap_owned_by_sley=239`; remaining blockers
  are still parser, checker, and runtime semantics from Sley source
- latest result-flow runtime call parser-prefix verification:
  `bash -n bin/sley`, `bash -n scripts/self-hosted-test.sh`, focused
  result-flow runtime probe, `scripts/self-hosted-test.sh`,
  `bin/sley-contract check-fixtures fixtures/contracts --schemas docs/schemas
  --json`, external public-action packet verifier, `git diff --check`, and
  final `make v1` all passed
- focused record-field runtime call parser-prefix probe:
  `bin/sley run --json fixtures/corpus/accepted/records_and_calls.sley` still
  returned `status == "passed"`, `value.kind == "Text"`, and
  `value.value == "Ada"` after record-field main-return call extraction moved
  from literal `return call ...` handling to the parser-owned call expression
  prefix
- Sley self-hosting status after the record-field runtime call parser-prefix
  source slice: `bootstrap`, `strict_self_hosted=false`,
  `semantic_source_count=6`, `bootstrap_owned_by_sley=240`; remaining blockers
  are still parser, checker, and runtime semantics from Sley source
- latest record-field runtime call parser-prefix verification:
  `bash -n bin/sley`, `bash -n scripts/self-hosted-test.sh`, focused
  record-field runtime probe, `scripts/self-hosted-test.sh`,
  `bin/sley-contract check-fixtures fixtures/contracts --schemas docs/schemas
  --json`, external public-action packet verifier, `git diff --check`, and
  final `make v1` all passed
- focused local runtime call parser-prefix probe:
  `bin/sley run --json examples/unused_private_task.sley` still returned
  `status == "passed"`, `value.kind == "Int"`, and `value.value == 1` after
  local zero-argument main-return call extraction moved from literal
  `return call ...` handling to the parser-owned call expression prefix
- Sley self-hosting status after the local runtime call parser-prefix source
  slice: `bootstrap`, `strict_self_hosted=false`,
  `semantic_source_count=6`, `bootstrap_owned_by_sley=241`; remaining blockers
  are still parser, checker, and runtime semantics from Sley source
- latest local runtime call parser-prefix verification:
  `bash -n bin/sley`, `bash -n scripts/self-hosted-test.sh`, focused local
  runtime probe, `scripts/self-hosted-test.sh`, `bin/sley-contract
  check-fixtures fixtures/contracts --schemas docs/schemas --json`, external
  public-action packet verifier, `git diff --check`, and final `make v1` all
  passed
- focused project zero-arg runtime call parser-prefix probes:
  `bin/sley run --json examples/unused_import_project` still returned
  `status == "passed"`, `value.kind == "Int"`, and `value.value == 7`; and
  `bin/sley run --json examples/duplicate_import_project` still returned
  `status == "passed"`, `value.kind == "Text"`, and `value.value == "ready"`
  after project zero-argument main-return call extraction moved from literal
  `return call ...` handling to the parser-owned call expression prefix
- Sley self-hosting status after the project zero-arg runtime call
  parser-prefix source slice: `bootstrap`, `strict_self_hosted=false`,
  `semantic_source_count=6`, `bootstrap_owned_by_sley=242`; remaining blockers
  are still parser, checker, and runtime semantics from Sley source
- latest project zero-arg runtime call parser-prefix verification:
  `bash -n bin/sley`, `bash -n scripts/self-hosted-test.sh`, focused project
  zero-arg runtime probes, `scripts/self-hosted-test.sh`,
  `bin/sley-contract check-fixtures fixtures/contracts --schemas docs/schemas
  --json`, external public-action packet verifier, `git diff --check`, and
  final `make v1` all passed
- focused project runtime call parser-prefix probes:
  `bin/sley run --json examples/project`,
  `bin/sley run --json examples/unqualified_import_call_project`, and
  `bin/sley run --json
  examples/unqualified_import_call_project/src/app/main.sley` all still
  returned `status == "passed"`, `value.kind == "Int"`, and
  `value.value == 42` after project one-argument main-return call extraction
  moved from literal `return call ...` handling to the parser-owned call
  expression prefix
- Sley self-hosting status after the project runtime call parser-prefix source
  slice: `bootstrap`, `strict_self_hosted=false`,
  `semantic_source_count=6`, `bootstrap_owned_by_sley=243`; remaining blockers
  are still parser, checker, and runtime semantics from Sley source
- latest project runtime call parser-prefix verification:
  `bash -n bin/sley`, `bash -n scripts/self-hosted-test.sh`, focused project
  runtime probes, `scripts/self-hosted-test.sh`, `bin/sley-contract
  check-fixtures fixtures/contracts --schemas docs/schemas --json`, external
  public-action packet verifier, `git diff --check`, and final `make v1` all
  passed
- focused message-template call parser-prefix probes:
  `bin/sley ast --json --node missing examples/hello.sley` still returned the
  `AST_NODE_NOT_FOUND` diagnostic message ``AST node not found `missing` ``;
  rejected `call_arity_mismatch.sley` and `call_argument_type_mismatch.sley`
  still returned the expected call diagnostic messages after parser/checker
  message-template call token extraction moved from literal `call ...()`
  handling to the parser-owned call expression prefix
- Sley self-hosting status after the message-template call parser-prefix
  source slice: `bootstrap`, `strict_self_hosted=false`,
  `semantic_source_count=6`, `bootstrap_owned_by_sley=245`; remaining blockers
  are still parser, checker, and runtime semantics from Sley source
- latest message-template call parser-prefix verification:
  `bash -n bin/sley`, `bash -n scripts/self-hosted-test.sh`, focused
  message-template probes, `scripts/self-hosted-test.sh`, `bin/sley-contract
  check-fixtures fixtures/contracts --schemas docs/schemas --json`, external
  public-action packet verifier, `git diff --check`, and final `make v1` all
  passed
- focused qualify-call fallback parser-prefix probes:
  `bin/sley-migrate report --json examples/unqualified_import_call_project`
  and `bin/sley plan --json --graft-templates --template-surface
  block:task:app.main.main:stmt:0:expr
  examples/unqualified_import_call_project` both still emitted replacement
  source `call math.double(21)` after the default replacement started using
  the parser-owned call expression prefix; `bin/sley fix --json --kind
  qualify_imported_call --dry-run examples/hello.sley` still accepted the
  fallback target
- Sley self-hosting status after the qualify-call fallback parser-prefix source
  slice: `bootstrap`, `strict_self_hosted=false`,
  `semantic_source_count=6`, `bootstrap_owned_by_sley=247`; remaining blockers
  are still parser, checker, and runtime semantics from Sley source
- latest qualify-call fallback parser-prefix verification:
  `bash -n bin/sley`, `bash -n scripts/self-hosted-test.sh`, focused
  qualify-call fallback probes, `scripts/self-hosted-test.sh`,
  `bin/sley-contract check-fixtures fixtures/contracts --schemas docs/schemas
  --json`, external public-action packet verifier, `git diff --check`, and
  final `make v1` all passed
- focused graph/plan call-argument parser-prefix probes:
  `bin/sley graph --json --slice task:app.main.main examples/project`,
  `bin/sley plan --json --graft-templates examples/project`, and
  `bin/sley plan --json --emit-graft replace_call_arg examples/project` all
  still extracted source `21` after call-argument parsing moved to the
  parser-owned call expression prefix; `bin/sley fix --json --kind
  replace_call_arg --dry-run examples/project` still accepted the target
- Sley self-hosting status after the graph/plan call-argument parser-prefix
  source slice: `bootstrap`, `strict_self_hosted=false`,
  `semantic_source_count=6`, `bootstrap_owned_by_sley=249`; remaining blockers
  are still parser, checker, and runtime semantics from Sley source
- latest graph/plan call-argument parser-prefix verification:
  `bash -n bin/sley`, `bash -n scripts/self-hosted-test.sh`, focused
  graph/plan call-argument probes, `scripts/self-hosted-test.sh`,
  `bin/sley-contract check-fixtures fixtures/contracts --schemas docs/schemas
  --json`, external public-action packet verifier, `git diff --check`, and
  final `make v1` all passed
- focused call-site rewrite parser-prefix probes:
  a temp copy of `examples/project` accepted write-mode
  `rename_and_update_call_sites` for `task:app.math.double` and query reported
  `math.twice`; a temp remove-take project accepted write-mode
  `remove_take_and_remove_call_arg` and query reported
  `call math.adjust(41)`
- Sley self-hosting status after the call-site rewrite parser-prefix source
  slice: `bootstrap`, `strict_self_hosted=false`,
  `semantic_source_count=6`, `bootstrap_owned_by_sley=251`; remaining blockers
  are still parser, checker, and runtime semantics from Sley source
- latest call-site rewrite parser-prefix verification:
  `bash -n bin/sley`, `bash -n scripts/self-hosted-test.sh`, focused write
  probes, `scripts/self-hosted-test.sh`, `bin/sley-contract check-fixtures
  fixtures/contracts --schemas docs/schemas --json`, external public-action
  packet verifier, `git diff --check`, and final `make v1` all passed
- focused runtime/docgen/workbench call fallback parser-prefix probes:
  `bin/sley self-hosting-status --json` reported the four new ownership
  markers and `bootstrap_owned_by_sley=255`; `bin/sley run --json
  examples/project` still returned `Int(42)`; `bin/sley-docgen reference
  --json examples/agent_deploy_pipeline.sley` still reported one task with
  seven outbound calls; `bin/sley-workbench --json --slice
  task:app.agent_deploy_pipeline.main examples/agent_deploy_pipeline.sley`
  still reported seven calls with four query call records
- Sley self-hosting status after the runtime/docgen/workbench call fallback
  parser-prefix source slice: `bootstrap`, `strict_self_hosted=false`,
  `semantic_source_count=6`, `bootstrap_owned_by_sley=255`; remaining blockers
  are still parser, checker, and runtime semantics from Sley source
- latest runtime/docgen/workbench call fallback parser-prefix verification:
  `bash -n bin/sley`, `bash -n scripts/self-hosted-test.sh`, focused
  runtime/docgen/workbench probes, `scripts/self-hosted-test.sh`,
  `bin/sley-contract check-fixtures fixtures/contracts --schemas docs/schemas
  --json`, external public-action packet verifier, `git diff --check`, and
  final `make v1` all passed
- focused runtime project probe parser-prefix probes:
  `bin/sley self-hosting-status --json` reported the two new runtime project
  probe markers and `bootstrap_owned_by_sley=257`; `bin/sley run --json
  examples/project` and `bin/sley run --json
  examples/unqualified_import_call_project` both still returned `Int(42)`
- Sley self-hosting status after the runtime project probe parser-prefix source
  slice: `bootstrap`, `strict_self_hosted=false`,
  `semantic_source_count=6`, `bootstrap_owned_by_sley=257`; remaining blockers
  are still parser, checker, and runtime semantics from Sley source
- latest runtime project probe parser-prefix verification:
  `bash -n bin/sley`, `bash -n scripts/self-hosted-test.sh`, focused runtime
  project probes, `scripts/self-hosted-test.sh`, `bin/sley-contract
  check-fixtures fixtures/contracts --schemas docs/schemas --json`, external
  public-action packet verifier, `git diff --check`, and final `make v1` all
  passed
- focused remaining call-parser parser-prefix probes:
  `bin/sley self-hosting-status --json` reported the three new call-parser
  ownership markers and `bootstrap_owned_by_sley=260`; `bin/sley ast --json
  fixtures/corpus/rejected/unknown_task.sley` still emitted the `call
  missing(1, label)` expression statement; `bin/sley run --json
  examples/collections.sley` still returned `Int(10)`; `bin/sley run --json
  examples/file_gate.sley` still returned `Text("hello sley")`; `bin/sley run
  --json examples/raw_host_migration.sley` still returned `Ok<Text>("hello
  sley")`; a focused grep found no remaining shell/AWK `^call...` parser
  patterns in `bin/sley`, `self-hosted/src`, or `scripts/self-hosted-test.sh`
- Sley self-hosting status after the remaining call-parser parser-prefix source
  slice: `bootstrap`, `strict_self_hosted=false`,
  `semantic_source_count=6`, `bootstrap_owned_by_sley=260`; remaining blockers
  are still parser, checker, and runtime semantics from Sley source
- latest remaining call-parser parser-prefix verification:
  `bash -n bin/sley`, `bash -n scripts/self-hosted-test.sh`, focused
  remaining call-parser probes, `scripts/self-hosted-test.sh`,
  `bin/sley-contract check-fixtures fixtures/contracts --schemas docs/schemas
  --json`, external public-action packet verifier, `git diff --check`, and
  final `make v1` all passed
- focused runtime host-effect checker-source probes:
  `bin/sley self-hosting-status --json` reported the two new runtime
  host-effect ownership markers and `bootstrap_owned_by_sley=262`;
  `bin/sley run --json` still preserved file reads, raw host migration,
  network seed output, database read output, agent deploy pipeline seeded
  output, agent data authority output, and spend-prefixed authorization; the
  no-capability agent pipeline path still returned
  `RUNTIME_CAPABILITY_REQUIRED`; a focused grep found no remaining runtime
  dispatch calls that pass hardcoded host adapter needles to
  `runtime_source_has_call`
- Sley self-hosting status after the runtime host-effect checker source slice:
  `bootstrap`, `strict_self_hosted=false`, `semantic_source_count=6`,
  `bootstrap_owned_by_sley=262`; remaining blockers are still parser, checker,
  and runtime semantics from Sley source
- latest runtime host-effect checker-source verification:
  `bash -n bin/sley`, `bash -n scripts/self-hosted-test.sh`, focused runtime
  host-effect probes, `scripts/self-hosted-test.sh`, `bin/sley-contract
  check-fixtures fixtures/contracts --schemas docs/schemas --json`, external
  public-action packet verifier, `git diff --check`, and final `make v1` all
  passed
- focused lint host-effect checker-source probes:
  `bin/sley self-hosting-status --json` reported
  `lint_declared_effect_host_needles_task_execution` and
  `bootstrap_owned_by_sley=263`; `bin/sley lint --json --rule
  unused_declared_effect examples/unused_declared_effect.sley` still reported
  `UNUSED_DECLARED_EFFECT`; `fixtures/corpus/accepted/authority/database_aliases.sley`
  stayed clean for `DbRead`/`DbWrite`; the missing alias write-effect rejected
  fixture still reported `effect unauthorized DatabaseWrite`; a focused grep
  found no remaining hardcoded host-call regex ladder in that lint
  `effect_used` path
- Sley self-hosting status after the lint host-effect checker source slice:
  `bootstrap`, `strict_self_hosted=false`, `semantic_source_count=6`,
  `bootstrap_owned_by_sley=263`; remaining blockers are still parser, checker,
  and runtime semantics from Sley source
- latest lint host-effect checker-source verification:
  `bash -n bin/sley`, `bash -n scripts/self-hosted-test.sh`, focused lint
  host-effect probes, `scripts/self-hosted-test.sh`, `bin/sley-contract
  check-fixtures fixtures/contracts --schemas docs/schemas --json`, external
  public-action packet verifier, `git diff --check`, and final `make v1` all
  passed
- focused docgen capabilities checker-source probes:
  `bin/sley self-hosting-status --json` reported
  `docgen_capabilities_host_effect_needles_task_execution` and
  `bootstrap_owned_by_sley=264`; `bin/sley-docgen reference --json --module
  agent.pipeline examples/agent_project` preserved schema `sley.docgen.report.v0`,
  `status=generated`, ten capability entries, the `DatabaseRead` alias
  `DbRead`, and FileWrite host/source ordering from the checker-owned table;
  the generated capability array matched
  `fixtures/contracts/docgen_agent_reference.json`
- Sley self-hosting status after the docgen capabilities checker source slice:
  `bootstrap`, `strict_self_hosted=false`, `semantic_source_count=6`,
  `bootstrap_owned_by_sley=264`; remaining blockers are still parser, checker,
  and runtime semantics from Sley source
- latest docgen capabilities checker-source verification:
  `bash -n bin/sley`, `bash -n scripts/self-hosted-test.sh`, focused docgen
  capability probes, `scripts/self-hosted-test.sh`, `bin/sley-contract
  check-fixtures fixtures/contracts --schemas docs/schemas --json`, external
  public-action packet verifier, `git diff --check`, and final `make v1` all
  passed
- focused runtime effect-alias checker-source probes:
  `bin/sley self-hosting-status --json` reported
  `runtime_authority_effect_aliases_task_execution` and
  `bootstrap_owned_by_sley=265`; `bin/sley run --json --cap DbRead=users
  --cap DbWrite=users --db-table users=examples/users.json
  fixtures/corpus/accepted/authority/database_aliases.sley` passed with
  `Ok(Text("Lin"))`; scoped denial probes for `DbRead=tenant_` and
  `DbWrite=tenant_` preserved `RUNTIME_CAPABILITY_SCOPE_DENIED` with
  `DatabaseRead/DbRead` and `DatabaseWrite/DbWrite` display text; a focused
  grep found no remaining Python-local `DbRead`/`DbWrite` runtime alias table
- Sley self-hosting status after the runtime effect-alias checker source slice:
  `bootstrap`, `strict_self_hosted=false`, `semantic_source_count=6`,
  `bootstrap_owned_by_sley=265`; remaining blockers are still parser, checker,
  and runtime semantics from Sley source
- latest runtime effect-alias checker-source verification:
  `bash -n bin/sley`, `bash -n scripts/self-hosted-test.sh`, focused runtime
  effect-alias probes, `scripts/self-hosted-test.sh`, `bin/sley-contract
  check-fixtures fixtures/contracts --schemas docs/schemas --json`, external
  public-action packet verifier, `git diff --check`, and final `make v1` all
  passed
- focused lint effect-alias checker-source probes:
  `bin/sley self-hosting-status --json` reported
  `lint_declared_effect_aliases_task_execution` and
  `bootstrap_owned_by_sley=266`; `bin/sley lint --json --rule
  unused_declared_effect examples/unused_declared_effect.sley` still reported
  `UNUSED_DECLARED_EFFECT`; `fixtures/corpus/accepted/authority/database_aliases.sley`
  stayed clean; a temp source using `uses DbRead` without a database read
  reported `UNUSED_DECLARED_EFFECT`; a focused grep found no remaining
  hardcoded JQ canonical-effect `DbRead`/`DbWrite` alias ladder
- Sley self-hosting status after the lint effect-alias checker source slice:
  `bootstrap`, `strict_self_hosted=false`, `semantic_source_count=6`,
  `bootstrap_owned_by_sley=266`; remaining blockers are still parser, checker,
  and runtime semantics from Sley source
- latest lint effect-alias checker-source verification:
  `bash -n bin/sley`, `bash -n scripts/self-hosted-test.sh`, focused lint
  effect-alias probes, `scripts/self-hosted-test.sh`, `bin/sley-contract
  check-fixtures fixtures/contracts --schemas docs/schemas --json`, external
  public-action packet verifier, `git diff --check`, and final `make v1` all
  passed
- focused runtime host default-text source probes:
  `bin/sley self-hosting-status --json` reported
  `runtime_host_default_texts_task_execution` and
  `bootstrap_owned_by_sley=267`; file write, database write, database read,
  secret, shell, spend, agent spend, agent data, and full agent deploy runtime
  probes preserved the existing deterministic host outputs; a focused grep
  found no remaining inline shell fallback forms for the moved host-default
  texts
- Sley self-hosting status after the runtime host default-text source slice:
  `bootstrap`, `strict_self_hosted=false`, `semantic_source_count=6`,
  `bootstrap_owned_by_sley=267`; remaining blockers are still parser, checker,
  and runtime semantics from Sley source
- latest runtime host default-text source verification:
  `bash -n bin/sley`, `bash -n scripts/self-hosted-test.sh`, focused runtime
  host default probes, `scripts/self-hosted-test.sh`, `bin/sley-contract
  check-fixtures fixtures/contracts --schemas docs/schemas --json`, external
  public-action packet verifier, `git diff --check`, and final `make v1` all
  passed
- focused runtime diagnostic source probes:
  `bin/sley self-hosting-status --json` reported
  `runtime_diagnostic_messages_task_execution` and
  `bootstrap_owned_by_sley=268`; unsupported `main` take, missing runtime
  capability, scope-denied secret, and CI blocked-runtime probes preserved the
  existing diagnostic IDs and messages while sourcing the values from
  `loom.runtime`
- Sley self-hosting status after the runtime diagnostic source slice:
  `bootstrap`, `strict_self_hosted=false`, `semantic_source_count=6`,
  `bootstrap_owned_by_sley=268`; remaining blockers are still parser, checker,
  and runtime semantics from Sley source
- latest runtime diagnostic source verification:
  `bash -n bin/sley`, `bash -n scripts/self-hosted-test.sh`, focused runtime
  diagnostic probes, `scripts/self-hosted-test.sh`, `bin/sley-contract
  check-fixtures fixtures/contracts --schemas docs/schemas --json`, external
  public-action packet verifier, `git diff --check`, and final `make v1` all
  passed
- focused runtime database default-table source probes:
  `bin/sley self-hosting-status --json` reported
  `runtime_default_database_table_task_execution` and
  `bootstrap_owned_by_sley=269`; `bin/sley run --json --cap DbRead=users --cap
  DbWrite=users --db-table users=examples/users.json
  fixtures/corpus/accepted/authority/database_aliases.sley` passed with
  `Ok(Text("Lin"))`; a scope-denied probe with `DbRead=orders` preserved
  `RUNTIME_CAPABILITY_SCOPE_DENIED` and the `users` fallback-table message; a
  focused grep found no remaining `or "users"` literal in runtime authority
- Sley self-hosting status after the runtime database default-table source
  slice: `bootstrap`, `strict_self_hosted=false`, `semantic_source_count=6`,
  `bootstrap_owned_by_sley=269`; remaining blockers are still parser, checker,
  and runtime semantics from Sley source
- latest runtime database default-table source verification:
  `bash -n bin/sley`, `bash -n scripts/self-hosted-test.sh`, focused runtime
  database default-table probes, `scripts/self-hosted-test.sh`,
  `bin/sley-contract check-fixtures fixtures/contracts --schemas docs/schemas
  --json`, external public-action packet verifier, `git diff --check`, and
  final `make v1` all passed
- focused runtime value-kind source probes:
  `bin/sley self-hosting-status --json` reported
  `runtime_value_kinds_task_execution` and `bootstrap_owned_by_sley=270`;
  focused run probes preserved `Text`, `Int`, `Bool`, `Unit`, and nested `Ok`
  value shapes after the shell fallback literals for runtime value-kind names
  were removed
- Sley self-hosting status after the runtime value-kind source slice:
  `bootstrap`, `strict_self_hosted=false`, `semantic_source_count=6`,
  `bootstrap_owned_by_sley=270`; remaining blockers are still parser, checker,
  and runtime semantics from Sley source
- latest runtime value-kind source verification:
  `bash -n bin/sley`, `bash -n scripts/self-hosted-test.sh`, focused runtime
  value-kind probes, `scripts/self-hosted-test.sh`, `bin/sley-contract
  check-fixtures fixtures/contracts --schemas docs/schemas --json`, external
  public-action packet verifier, `git diff --check`, and final `make v1` all
  passed

## Dirty State Notes

Tracked code was committed at 21a3a9d before this pause document was updated.
Tracked pause documentation before this update was refreshed through 218d40c
with parser lint surface evidence, through 26e7417 with the external comparison
approval gate, fork audit, and helper evidence, and through aba1b03 with parser
fix-migration fallback evidence, and through d20433f with parser fix-style
fallback evidence, through cc47f3e with parser fix-empty fallback evidence, and
through 72fe387 with parser fix-unused/unreachable fallback evidence, and
through 5f3efde with parser fix-constant-control fallback evidence, and
through 086fbee with parser fix-constant-scalar fallback evidence, and through
8dc35fc with parser fix-constant-derived fallback evidence, and through
48facb4 with parser fix-algebra/boolean fallback evidence, and through
eefadcc with parser fix-boolean-branch fallback evidence, and through 395066b
with parser static task/block fallback evidence, and through b508d9a with
checker static type-name evidence, and through 1c5cca9 with checker
host-effect table evidence, and through 5e4bec6 with checker gate binding-kind
evidence, through 117ffad with checker repair hint-kind evidence, and through
bcbd217 with checker identifier expression-kind evidence, and through 9bc2a63
with checker call expression-prefix evidence, and through bda30f3 with query
call expression-prefix evidence, through ddbbbf3 with symbol graph call
expression-prefix evidence, and through b2da63e with docgen/workbench call
expression-prefix evidence, and through 71a768f with lint call
expression-prefix evidence, and through 05e1d77 with checker call
parser-prefix evidence, and through 1396712 with runtime main-call
parser-prefix evidence, and through de4bcb6 with runtime status
parser-prefix evidence, and through a680ce9 with checker/lint status
parser-prefix evidence, and through f470a5b with parser classifier
parser-prefix evidence, and through 6069e6c with bootstrap bind-call
parser-prefix evidence, and through 9c8d29e with compute runtime parser-prefix
evidence, through d9c6ec5 with result-flow runtime parser-prefix evidence, and
through a390dd9 with record-field runtime parser-prefix evidence, and through
304e5d0 with local runtime parser-prefix evidence, and through 82ce059 with
project zero-arg runtime parser-prefix evidence, and through 25cc748 with
project runtime parser-prefix evidence, and through 0cfd03b with
message-template parser-prefix evidence, through ae61afe with qualify-call
fallback parser-prefix evidence, through c58fc6e with graph/plan call-argument
parser-prefix evidence, and through ff4ce3f with call-site rewrite
parser-prefix evidence, and through fc22378 with runtime/docgen/workbench call
fallback parser-prefix evidence, and through 62a39f1 with runtime project probe
parser-prefix evidence, and through 3ec0ff2 with remaining call-parser
parser-prefix evidence, and through 8ec3e49 with runtime host-effect checker
source evidence, and through c4b32e5 with lint host-effect checker source
evidence, through c7ece51 with docgen capabilities checker source evidence,
through 7e16daf with runtime effect-alias checker source evidence, and through
bef77d3 with lint effect-alias checker source evidence, and through eb3b0f9
with runtime host default-text source evidence, and through a4595a9 with
runtime diagnostic source evidence, and through d270b71 with runtime database
default-table source evidence.

Pre-existing untracked files were not touched and should remain outside future
commits unless the operator explicitly asks to include them:

- docs/SleyRoadmapAudit.md
- docs/SleyZJX.md
- docs/SleyZJX2.md
- docs/SleyZJXEverythingAudit.md
- sleyarena.txt

## Resume Procedure

When development resumes:

1. Check `git status --short --branch`.
2. Confirm the branch is `public`; note whether local pause commits are still
   ahead of `origin/public`.
3. Re-run `make v1` before editing.
4. Start with report-builder or parser/checker/runtime source execution, not
   another already-covered host-gate value slice.
5. Commit each coherent repair slice separately and push for continuity.
