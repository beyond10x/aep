---
format: aep.planning-md/3
id: story:a-killed-publication-stage-is-swept
kind: story
status: archived
title: A stage left by a killed process is removed
relations:
- serves: vision:O2
- decomposes: epic:zero-open-issues-2026-11-09
revision: 3
transitions:
- {from: "draft", to: "archived", at: "2026-11-09T22:07:31Z", actor: "human:timo", revision: 3}
---
# A stage left by a killed process is removed

## Outcome

A `planning.aep-stage-*` directory left by a process that was killed during publication is removed the next time
a publication runs, without ever removing a stage another live process owns.

## Why

aep#40 removes a stage on every error return and on unwinding, but a killed process runs no destructor. Its stage
stays beside the projection: recovery re-renders into a fresh directory and never removes the old one (test
`a_stage_a_killed_process_left_does_not_stop_recovery_and_is_not_removed`). Stage names carry pid and counter
(`stage_directory()`), which is what a sweep can test for liveness, with the pid-namespace caveat the aep#40
adversary noted.

## Closed 2026-11-09

Obsolete. `stage_directory()` and the `planning.aep-stage-*` stages lived in `crates/plan/aep-planning-migration/src/projection.rs`. Commit `742bb0713` ("refactor: remove Eventlog from AEP", released in 0.62.0) deleted the `aep-planning-migration` and `aep-backend-eventlog` crates; `CHANGELOG.md` 0.62.0 *Removed* lists it. A search for `aep-stage` and `stage_directory` over `crates/` and `xtask/` at `824869fec` finds nothing.
