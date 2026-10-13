---
format: aep.planning-md/3
id: story:concurrent-command-steps
kind: story
status: draft
title: Independent command steps run at once, where a state opts in
relations:
- decomposes: epic:deterministic-delivery-workflows
- depends_on: story:llm-step-declares-judgement
- depends_on: story:command-step-expects-failure
revision: 4
---
# Story: Independent command steps run at once, where a state opts in

## Outcome

A state in a step map may declare `concurrent: true`. Its command steps then run at once, unless
ordered by a new step-ordering field, `after: [<id>]`. That field names the new optional `id` that
a command step may carry. A state without `concurrent` runs its steps one at a time, as today.

This covers evidence collection while diagnosing, such as reading logs, traces and store state.
The run report lists results in map order, whatever order the steps finish in.

## Context

- A command step's `depends_on` names an outside dependency that a circuit breaker keys on. It is
  not a step reference (`crates/drive/aep-driver-spec/src/map.rs:376-387`, `:544-569`;
  `crates/drive/aep-driver/src/run.rs:983-1028`).
- A step has no id: it is addressed by its index in `steps_for(state)` (`map.rs:364-381`).
- The driver runs steps one at a time: `run.rs` has no concurrency (grep for `parallel` and
  `concurren`, 2026-10-13).

## Acceptance

- In a `concurrent: true` state, three fixture command steps sleep 3, 1 and 2 seconds, in that map
  order, with no `after`.
  - They finish in under 4 seconds of wall time.
  - The report lists them in map order 3, 1, 2, not in completion order.
- A step with `after: [a]` starts only after the step with `id: a` has ended.
- An `after` naming an id that does not exist in the state is refused at validation, naming it.
- Two steps that name the same `depends_on` dependency run one at a time, so the breaker counts
  their failures in map order.
- The evidence submitted and the report are byte-identical across ten runs.
- A failed step does not cancel its siblings. Each result is reported.
- Both shipped maps (`drivers/development/*.yaml`) declare no `concurrent`, and their existing
  driver tests pass unchanged.

## Scope

Cited:

- `crates/drive/aep-driver-spec/src/map.rs` (`concurrent`, `id`, `after`)
- `schemas/generated/driver-steps.schema.json`
- `crates/drive/aep-driver/src/run.rs`
- `crates/edge/aep-cli/src/drive.rs` (the command host)

## Depends on

- `story:llm-step-declares-judgement`
- `story:command-step-expects-failure`

Both edges are for the shared files `map.rs`, `drive.rs` and the driver-steps schema.

## Open Question

Whether the deterministic core may keep its banned-token scan (`tests/determinism.rs` in
`aep-driver`) once the command host runs threads. The default is to keep concurrency in the `edge`
host and leave the core sequential over results already collected.
