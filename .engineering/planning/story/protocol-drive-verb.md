---
format: aep.planning-md/1
id: story:protocol-drive-verb
kind: story
status: active
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
- serves: vision:O3
- depends_on: story:unreadable-lock-refuses-its-own-escape-hatch
revision: 6
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
- Run directories are allocated by counting up and are **never reused** — including after one has
  been deleted, which is the expected case rather than the exotic one, because run directories hold
  transcripts and are the largest thing a driven repository accumulates.

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

### Re-verified — 2026-08-30

`cargo test -p protocol-cli --test drive_cli` → **47 passed**, exit 0 (11 on 2026-08-28);
`cargo test -p aep-driver --test driving` → 12 passed; `--test routing` → 12 passed (7 on
2026-08-28). **Two of the five open rows closed** — both by work that landed under
`story:operator-resume-ux`, which is now `implemented`, rather than by anything filed here.

| row | 2026-08-30 |
|---|---|
| the refusal names the holder's **cursor state** | **shipped.** `LockState` now carries `state: Option<String>` (`crates/aep-driver/src/lock.rs:78-84`), supplied and never discovered, with `state unknown` as the worded absence; asserted by `a_refusal_names_what_the_holding_run_is_doing` (`crates/protocol-cli/tests/drive_cli.rs:1422`) |
| `--pause-on-approval` **releases the lock** | **shipped.** `a_pause_is_the_lock_released_and_the_pointer_kept` (`crates/protocol-cli/tests/drive_cli.rs:1806`) asserts released lock, kept `current` pointer and `awaiting_operator` in one test, for the reason its doc comment gives |

Three rows are what this story now owes, and none is more than a test:

| row | state on 2026-08-30 | what remains |
|---|---|---|
| the refusal asserts **host** | still unasserted. `a_second_driver_is_refused_by_name_and_writes_nothing` (`crates/protocol-cli/tests/drive_cli.rs:360`) writes `host()` into the lock and then asserts only the run id, the pid and `--take-lock` | one `contains` on the host it wrote |
| an artifact **created by a step** changes `artifact.<kind>.count` in the next evaluation | still no test. `artifact.story.count` is read only through `story_count` (`crates/aep-driver/tests/driving.rs:393`), and its three call sites (`:727`, `:755`, `:800`) are the F7 shrink assertions — a store that got *smaller* outside the run, never a step that made it bigger | one test whose `command` step creates an artifact and whose next evaluation reads the higher count |
| two `Engine` values in one process do not collide on a run directory | still no test. A tree-wide search for `collide`, `collision`, `two engine`, `concurrent` over `crates/aep-driver/tests/` and `drive_cli.rs` returns nothing | one test that constructs two |

The lock-path wording is unchanged and still owed: `crates/aep-driver/src/lock.rs:9` documents *one
fixed path per store*, and `crates/protocol-cli/src/drive.rs:99` puts `lock.json` under the
project's `.engineering/runs/`, which `--store` does not move. One line of prose, in whichever
direction the driver owner picks.

### The last acceptance line was rewritten — 2026-08-30

It read *"Two `Engine` values in one process do not collide on a run directory"*, and it was not a
proposition about `Engine`. Established by the adversarial pass rather than argued:

- `Engine` has three fields (`Registry`, `C: Clock`, `AtomicU64`) and six methods, and **none takes
  or returns a path** (`crates/aep-engine/src/engine.rs:185-206`).
- Every `std::fs` call in `aep-engine` is in a `load.rs`/`project.rs` free function; all six
  `RunDirectory::at` sites are in `protocol-cli`.
- There is no process-global state in the drive path — the one `OnceLock` at `drive.rs:4711` is
  inside `#[cfg(test)]`.
- No in-process multi-run caller exists: `eval` refuses to launch a driven arm itself and spawns
  `protocol drive run` (`crates/protocol-cli/src/eval.rs:1783`, `:2842`).

So *in one process* distinguished nothing, and a test written to it could only have asserted that
two values can be constructed. The implementor deleted such a test in correction round 1 and was
right to; the line is now what it always meant, and what it means is `allocate_run`.

**And what it means is not yet true.** `crates/protocol-cli/src/drive.rs:1258-1276` reads
`highest + 1` off a directory listing, so "never reused" holds only while nothing is deleted — its
own doc comment states the property as a fact rather than enforcing it. Remove the highest run
directory and its id is handed to a different run **while `current` still names it**. Red case:
`adversary_a_run_directory_that_was_removed_is_not_handed_out_to_a_second_run`
(`crates/protocol-cli/tests/drive_cli.rs:2670`).

## Out of Scope

The hooks, which are the plugin's side of the same enforcement and ship after this. Retry accounting
and the lock-refusal wording are their own stories.

## Open Questions

Whether `--restart` should carry the previous run's cursor forward for reading. Decides: driver
owner. Default if nobody answers: it does not — a run directory is never reused, and `--take-lock`
already records what it superseded.
