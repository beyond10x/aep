---
format: aep.planning-md/2
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
