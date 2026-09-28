---
format: aep.planning-md/3
id: story:transitions-record-executor-and-correlation
kind: story
status: draft
title: A transition records the executor and correlation of the command that made it
relations:
- serves: vision:O2
revision: 1
---
## Problem

A `/5` transition holds `at`, `actor`, `revision`, `from`, `to`, `decided_on` and `imported`
(`crates/plan/aep-backend-markdown/src/journal.rs`, `struct Transition`). The command context
carries an executor distinct from the actor and a correlation id
(`crates/plan/aep-contract/src/command.rs`, `CommandContext`), and neither is written. An agent's
move made on a person's behalf therefore reads as the person's own.

## Outcome

`transitions` entries carry `executor` when it differs from `actor`, and `correlation`; old entries
without them stay valid.

## Acceptance

- A move with `--executor agent:x` records `executor: "agent:x"`; `history` and `explain` print it.
- A `/5` document written by 0.64.0 still parses and validates.
