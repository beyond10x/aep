---
format: aep.planning-md/1
id: story:retry-budgets
kind: story
status: implemented
title: Retry budgets per step kind, spent and never reset
summary: A crashed step retries within its kind's budget, a green third attempt does not erase the first two, and exhaustion leaves a resumable snapshot.
owner: driver
tags:
- driver
relations:
- decomposes: epic:reference-driver
- depends_on: story:protocol-drive-verb
revision: 5
---
# Story: Retry budgets per step kind, spent and never reset

## Outcome

A run survives a process that died without pretending it never died: the operator sees *green on the
third try* as three attempts, not as one success.

## Context

A crashed step and a failing suite are different facts and must land on different sides — nothing
observed versus something observed to be false. The budget belongs to the step **kind**, because the
reasons differ: a `command` step retries because a process died, an `llm` step retries once because a
model call errored, and an `operator` step never retries, because re-prompting a person who has not
answered is the driver deciding a human is a transient fault.

## Acceptance

- A `command` step whose executable does not exist submits **zero** evidence and leaves the
  evaluation unchanged.
- A suite that runs and fails submits a `TestResult` with failures, and the next transition moves
  `verify → implement`.
- The cursor's attempt count survives the retry that succeeded, and the run report names it.
- Exhaustion leaves a resumable snapshot, prints `Blocked`'s reasons and the completion explanation
  **verbatim**, and adds exactly one line of the driver's own naming the budget and the step.
- The per-state visit budget and the per-step retry budget are separate bounds — a legitimate
  `verify → implement` cycle is not reported as a wedged command.

## Shipped — read against the code, 2026-08-28

All five acceptance lines hold, with named tests: `crates/aep-driver/tests/driving.rs` (12 passed)
and `crates/aep-driver/tests/routing.rs` (7 passed), run 2026-08-28.

| line | where it holds |
|---|---|
| a crashed `command` step submits zero evidence and changes nothing | `crates/protocol-cli/src/drive.rs` (spawn error → `NoVerdict`); `a_command_step_that_produced_no_verdict_submits_nothing_and_changes_nothing`, `driving.rs:405` |
| a failing suite submits a `TestResult` and the engine routes `verify → implement` | `crates/aep-driver/src/run.rs:692-696`; `a_failing_suite_is_routed_by_the_engine_and_the_visit_budget_ends_the_cycle`, `driving.rs:1032` |
| the attempt count survives the retry that succeeded, and the report names it | `crates/aep-driver-spec/src/cursor.rs:216-221`; `a_retried_step_that_then_succeeded_keeps_its_first_attempt_in_the_cursor`, `driving.rs:526` |
| exhaustion leaves a resumable snapshot with the engine's reasons verbatim | `run.rs:725-738`; `a_step_that_spends_its_retry_budget_leaves_a_resumable_run_and_the_engines_own_reasons`, `driving.rs:459` |
| the visit budget and the retry budget are separate bounds | `crates/aep-driver-spec/src/map.rs:507-512` vs `:765-769`; `driving.rs:1032` and `a_state_entered_past_its_visit_budget_stops_the_run_rather_than_running_its_steps_again`, `routing.rs` |

**The Open Question's default was reversed, deliberately, and this records it.** The default was *no
per-step override — one number per kind, in the map's header*. What shipped is the opposite:
`RawCommandStep.retries: Option<u32>` (`map.rs:374-376`) defaults per step (`map.rs:1118`), and
`retry_budgets_are_per_step_kind` (`map.rs:1467-1483`) pins that `"retries": 0` yields a budget of
zero. The kind default still exists and is what a step gets when it says nothing; the override is
what makes one flaky command declarable without moving every other command's bound. A reader of a
map now has to look at the step as well as the header to know a bound — which is the cost the
default was trying to avoid, and it is accepted rather than unnoticed.

## Out of Scope

Backoff. An unbounded retry with backoff is a bound nobody can state, which is a token budget nobody
can state.

## Open Questions

Whether `command` steps get a per-step override in the map. Decides: driver owner. Default if nobody
answers: no override — one number per kind, in the map's header, is what makes a run's behaviour
readable.
