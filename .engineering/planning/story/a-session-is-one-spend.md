---
format: aep.planning-md/1
id: story:a-session-is-one-spend
kind: story
status: implemented
title: A session's terminal records are one spend, however many it wrote
relations:
- decomposes: epic:self-evaluation
- serves: vision:O3
revision: 4
---
# Story: a session's terminal records are one spend, however many it wrote

## Outcome

`aep eval run` reports what a run spent, on a stream where one session wrote more than one terminal
record.

## Context

The reader totals `total_cost_usd`, `usage` and `duration_ms` over every `session.ended`. That is right
across the sessions of a driven run — `story:absent-rows-decide-on-a-closed-stream`'s neighbour finding
of 2026-08-23 showed a reader taking the *last* record reporting `$1.135363` for a walk that cost
`$15.014604` over six sessions — and wrong within one session.

`--max-budget-usd` stops a session *after* it has written its `result`, so the stream carries a second
terminal record saying `error_max_budget_usd`, and both restate the same running counters. The
golden-path recording of 2026-09-03 reported `cost_micro_usd: 30002816` for a session that spent
`$15.00140784`. The manifest's `cost-within-the-per-case-budget` row read a $15 run as a $30 one against
its own $15 cap — a contradiction produced by the reader, not by the run.

## Acceptance

- Terminal records are grouped by `session.started`; the fold is the largest figure within a session and
  the sum across sessions, on all three columns.
- The largest and not the last: the stopping record restates the cost and the wall clock but zeroes its
  `usage`, so *last* would report a run of no tokens.
- The driven doubling test is unchanged and still passes; a new test appends a stopping record to the
  committed driven fixture and asserts all three columns are unmoved.
- A stream that opens with a terminal record before any `session.started` is still read, and one with no
  terminal record at all still refuses `NoTerminalEvent`.

## Out of Scope

- Refusing a stream whose terminal records disagree. They do not disagree here; the second states less.
