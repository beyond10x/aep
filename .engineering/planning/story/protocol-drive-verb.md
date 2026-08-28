---
format: aep.planning-md/1
id: story:protocol-drive-verb
kind: story
status: proposed
title: 'protocol drive: the run that touches the world'
summary: The command, llm and operator executors, the run directory, the store lock, and the flags that resume, restart or take it.
owner: driver
tags:
- cli
- driver
relations:
- decomposes: epic:reference-driver
- depends_on: story:driver-router
- depends_on: story:default-step-map
revision: 3
---
# Story: `protocol drive` — the run that touches the world

## Outcome

An operator starts a run in their repository, walks away, and comes back to either a completed
workflow or a run that stopped somewhere specific with the engine's own words for why — and a
directory they can resume from.

## Context

This is the impure half, deliberately in `protocol-cli`: the three executors that touch the world,
the run directory, the store lock and the pid-liveness probe. It is where the driver's decisions
either survive contact or do not. The store's graph is rebuilt **between steps**, not per state,
because a `command` step can create an artifact and the next step in the same state would otherwise
evaluate against a store one write behind.

## Acceptance

- The lock lives at one fixed path per store, taken with `create_new` **before** a run id is
  allocated — two invocations racing cannot both succeed.
- A second `drive` against a locked store exits non-zero, prints the holder's run id, pid, host and
  the state the cursor says it is in, and **writes nothing** — asserted by an unchanged run directory
  and a clean tree.
- A store whose report is not clean stops the run and prints the accumulated errors verbatim; a
  single unparseable file stops it, asserted by the fact store being *unchanged* rather than silently
  shrunk.
- An artifact created by a step changes `artifact.<kind>.count` in the next evaluation, asserted on
  the fact store; a mutation is not observable through a previously built graph.
- A run that reaches an approval under `--pause-on-approval` persists, exits 0, releases the lock, and
  resumes.
- Two `Engine` values in one process do not collide on a run directory.

## Re-scoped on evidence — 2026-08-28

The verb ships and two real runs have used it (`W4-1/1`, `W4-2/1`). `cargo test -p protocol-cli
--test drive_cli` → 11 passed; `cargo test -p protocol-cli --bin protocol drive::` → 24 passed;
`cargo test -p aep-driver --test driving` → 12 passed.

| line | state | what remains |
|---|---|---|
| the lock is taken with `create_new` before a run id is allocated | **holds** — `crates/protocol-cli/src/drive.rs:923`, order at `:474-475`; `a_second_driver_is_refused_by_name_and_writes_nothing` | note: the lock is one path per **project** (`.engineering/runs/lock.json`), not per store — `--store` does not move it. Either the story says project, or the path follows the store |
| a second `drive` exits non-zero and prints run id, pid, host **and the cursor's state**, writing nothing | **partial** — the refusal names run id, pid and both routes; it can**not** name the state, because `LockState` has no state field (`crates/aep-driver/src/lock.rs:66-75`) and `take_lock` bails before reading a cursor. Host is unasserted | read the holder's cursor and put its state in the refusal; assert host |
| an unclean store report stops the run, errors verbatim, fact store unchanged | **holds** — `crates/aep-driver/src/run.rs:858-866`; `a_store_with_one_unparseable_file_stops_the_run_with_its_fact_base_unchanged` | — |
| an artifact created by a step changes `artifact.<kind>.count` in the next evaluation | **partial** — the graph is rebuilt at the top of every loop iteration (`run.rs:498-500`) and one iteration is one step, so the behaviour is there; no test has a step **create** an artifact and re-read the count | one test that does exactly that |
| `--pause-on-approval` persists, exits 0, releases the lock, resumes | **partial** — exit 0 and resume are asserted; the lock release **on the pause path** is not | one assertion on an existing test |
| two `Engine` values in one process do not collide on a run directory | **partial** — run ids are allocated by counting up and never reused (`drive.rs:801-820`); no test constructs two | one test |

`--restart` does not exist, which takes its Open Question's default by omission: a run directory is
never reused.

## Out of Scope

The hooks, which are the plugin's side of the same enforcement and ship after this. Retry accounting
and the lock-refusal wording are their own stories.

## Open Questions

Whether `--restart` should carry the previous run's cursor forward for reading. Decides: driver
owner. Default if nobody answers: it does not — a run directory is never reused, and `--take-lock`
already records what it superseded.
