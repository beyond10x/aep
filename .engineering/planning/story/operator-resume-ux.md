---
format: aep.planning-md/1
id: story:operator-resume-ux
kind: story
status: draft
title: A refused run tells the operator which of two things to type
summary: The lock refusal names the holder and the two routes out of it; --take-lock supersedes rather than erases; --resume re-acquires before it writes.
owner: driver
tags:
- driver
- operator
relations:
- decomposes: epic:reference-driver
- depends_on: story:protocol-drive-verb
revision: 2
---
# Story: A refused run tells the operator which of two things to type

## Outcome

An operator whose run is refused by a lock does not go and read a design document. The refusal names
who holds it and the exactly two commands that resolve it, and stealing a lock is something a person
did on purpose, recorded in the run that took it.

## Context

A paused run holds no lock, because an `operator` step waiting for a person has no bound and any age
threshold would break exactly the runs that paused correctly. That makes re-acquisition on resume
load-bearing: a resume that writes without re-taking the lock is how two live runs happen. The
refusal follows the shape `artifact move` already uses for an illegal transition — refuse, and name
where you can actually go.

## Acceptance

- A lock whose pid is alive is refused; the message carries run id, pid, host and the cursor's state,
  and names both `--resume` and `--take-lock`.
- A lock whose pid is **not** alive on the same host is reported stale and **still refused** without
  `--take-lock`.
- A lock naming a different host is never stale, whatever the local pid table says.
- `--take-lock` writes the stolen lock's contents into the new run's cursor, so *this run took the
  lock from pid 4711 of run `<task>/2`* is in the record.
- `--resume` against a store whose lock another run now holds refuses.
- The lock is absent after an approval pause while `current` still points at the run.

## Re-scoped on evidence — 2026-08-28

`cargo test -p aep-driver --test routing` → 7 passed; `cargo test -p protocol-cli --test drive_cli`
→ 11 passed, 2026-08-28. Staleness and host rules hold exactly as written. What is left is small,
mechanical and in one crate — which is why this story is the candidate for the first driven run
(`story:governed-dogfood-run`).

| line | state | what remains |
|---|---|---|
| a live pid is refused, naming run id, pid, host, **the cursor's state**, and both routes | **partial** — everything but the state; `LockState` has no state field (`crates/aep-driver/src/lock.rs:66-75`) | read the holder's cursor in the refusal path and print its state |
| a dead pid on the same host is stale and **still refused** | **holds** — `a_dead_holder_is_stale_and_still_refused_until_a_person_says_take_it`, `tests/routing.rs:168` | — |
| a lock naming another host is never stale | **holds** — `a_lock_held_on_another_host_is_never_stale_whatever_the_local_pid_table_says`, `tests/routing.rs:189` | — |
| `--take-lock` writes the stolen lock into the new run's **cursor** | **missing.** `StolenLock` is built (`crates/protocol-cli/src/drive.rs:948-954`) and only **printed** (`:505-511`); the cursor field `took_lock_from` is assigned `None` at `crates/aep-driver/src/run.rs:972` and `cursor.rs:338` and nowhere else, so `protocol drive status` has a printer for a field that is always empty. **The theft is not in the record** | thread the `StolenLock` into the cursor; one test that steals a lock and reads it back from the run directory |
| `--resume` against a store whose lock another run holds is refused | **partial** — the code re-takes the lock through the same path; no test | one test |
| the lock is absent after an approval pause while `current` still points at the run | **partial** — asserted for a finished run and an iteration stop, not for the pause path | one assertion |

One of these is code (`took_lock_from`), the rest are a message field and three tests.

## Out of Scope

Waiting. There is no queue and no blocking acquire — a driver that waits on a lock is a driver
holding a session open for an unbounded time.

## Open Questions

None. The age-threshold question was asked and answered: there is deliberately no threshold.
