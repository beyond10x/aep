---
format: aep.planning-md/1
id: story:drive-transition-verb
kind: story
status: implemented
title: A native flow is governed by the engine at every section boundary
summary: protocol drive transition answers the loop transition hook from evaluate and transition, so b10x-harness workflow run is governed without walking here
owner: driver
tags:
- driver
- harness
relations:
- decomposes: epic:cross-harness-portability
- serves: vision:O3
revision: 5
---
# Story: A native flow is governed by the engine at every section boundary

## Outcome

`b10x-harness workflow run` walks a flow that `protocol workflow flow` projected from `adp/default`,
and that projection is an ordering, not a government: no guard travels. The loop asks a `transition`
hook before a section is entered and after it leaves. `protocol drive transition` answers it from
the engine — `evaluate` for `enter`, `transition` on a copy of the execution for `leave` — so a
native walk is governed by the same documents that govern a driven run, with no crate dependency in
either direction. Harness design 0003 § 7 named this E2; atlas ADR 0004 records the boundary.

## Acceptance

- Reads the loop's `transition` document on stdin; exit `0` proceeds, `2` refuses with
  `{"reason": …}` in the engine's words; anything unreadable exits `1`, which the loop reads fail
  closed.
- `--run <id>` positions the engine on that run's snapshot over the store as it is now; without it,
  on the state the flow path names (a step node after its state; a retreat group
  `<first>-to-<last>` at its first state on `enter`, its last on `leave`).
- A section that came out failed is left alone.
- Decides only: writes nothing, takes no lock; a consultation leaves a run's cursor byte-identical.
- Documented on the website (integrate-a-harness, CLI reference) and in `docs/guide/harness.md`.

## Evidence

`crates/protocol-cli/tests/drive_cli.rs`: nine `transition_*` tests over the real binary and a
fixture store; the full gate.
