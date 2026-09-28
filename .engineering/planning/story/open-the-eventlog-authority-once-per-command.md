---
format: aep.planning-md/3
id: story:open-the-eventlog-authority-once-per-command
kind: story
status: implemented
title: Open the Eventlog authority once per command
relations:
- serves: vision:O2
- informed_by: story:batch-and-transaction-verbs
scope:
- confidence: cited
  path: crates/edge/aep-cli/src/planning.rs
- confidence: cited
  path: crates/plan/aep-backend-eventlog/src/lib.rs
revision: 6
transitions:
- {from: "draft", to: "proposed", at: "2026-09-25T09:05:53Z", actor: "human:timo", revision: 4, decided_on: {"recorded":{"test_result":1}}, imported: true}
- {from: "proposed", to: "active", at: "2026-09-25T09:06:04Z", actor: "human:timo", revision: 5, decided_on: {"recorded":{"test_result":1}}, imported: true}
- {from: "active", to: "implemented", at: "2026-09-25T09:06:16Z", actor: "human:timo", revision: 6, decided_on: {"recorded":{"test_result":1}}, imported: true}
---
# Open the Eventlog authority once per command

## Outcome

One `aep plan artifact` command opens the planning authority once and reuses that handle for the decision, the
commit and the projection, instead of opening and verifying the store again for each phase.

## Why

On the `/2` Eventlog stores every write re-opened and re-verified the whole authority several times; an ESS `move`
took 541 s.

## Delivered

aep#16, merged 2026-09-23 as `d85adc863`, released in AEP 0.58.0: ESS `move` 541 s → 245 s.

## Scope

- `crates/edge/aep-cli/src/planning.rs`
- `crates/plan/aep-backend-eventlog/src/lib.rs`
- `crates/plan/aep-planning-migration/src/{mutation,projection}.rs`
