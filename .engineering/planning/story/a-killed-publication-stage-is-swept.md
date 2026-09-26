---
format: aep.planning-md/2
id: story:a-killed-publication-stage-is-swept
kind: story
status: draft
title: A stage left by a killed process is removed
relations:
- serves: vision:O2
revision: 1
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
