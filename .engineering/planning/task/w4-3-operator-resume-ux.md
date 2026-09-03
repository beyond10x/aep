---
format: aep.planning-md/1
id: task:w4-3-operator-resume-ux
kind: task
status: draft
title: 'W4-3: operator-resume-ux, driven as a governed run'
summary: 'The request recorded at intake: task W4-3 (kind feature, objective operator-resume-ux) asks for story:operator-resume-ux under protocol adp/1 and profile development.driven.'
relations:
- derived_from: story:operator-resume-ux
revision: 5
---
# Task: W4-3 — operator-resume-ux

Intake record. This is the request as `.engineering/task-w4-3.yaml` states it, not an interpretation
of it. Everything below is either a field copied from that task document or a quotation from it or
from the one artifact it names; nothing here is specified, designed or decomposed, because none of
that has been asked for yet.

## What

What the task document declares, field by field:

| Field | Value |
|---|---|
| task id | `W4-3` |
| task kind | `feature` |
| objective | `operator-resume-ux` |
| protocol | `adp/1` |
| profile | `development.driven` |
| derived from | `story:operator-resume-ux` |

Its own header calls it "The third governed run — the first one whose product is Rust, and therefore
the first that walks `development/default` rather than `development/checks`."

The requester's stated reason for choosing this story, verbatim: "`story:operator-resume-ux` is
chosen against `story:governed-dogfood-run`'s own stated default — *one whose acceptance is already
mechanical and whose blast radius is one crate* — after the 2026-08-28 audit of
`epic:reference-driver` closed the previous candidate. `story:retry-budgets` was the candidate this
plan named; it turned out to be shipped and closed that day on `crates/drive/aep-driver/tests/driving.rs`
and `routing.rs`."

The requester states what it believes is left, verbatim: "What is left in `operator-resume-ux` is one
real defect and three assertions:

> * `--take-lock` builds a `StolenLock`, prints it, and never persists it. The cursor field
>   `took_lock_from` is assigned `None` in two places and nowhere else, so `protocol drive status`
>   has a printer for a field that is always empty and *the theft is not in the record*;
> * the lock refusal cannot name the holder's cursor state, because `LockState` has no state field
>   and `take_lock` bails before reading a cursor;
> * three assertions the behaviour already has and no test names: `--resume` against a store another
>   run holds, the lock's absence on the *pause* path, and the host in the refusal."

And its own summary of the size: "One crate and one CLI module, no published format outside the run
directory, and an acceptance a reader can check without arguing about it."

## Why

The one artifact the request names is `story:operator-resume-ux`, carried here by the task document's
own `derived_from` edge. The requester's note: "The story is `story:operator-resume-ux` in
`.engineering/planning/story/`; its Acceptance section, as re-scoped on 2026-08-28, is the
specification this run is measured against. The re-scoping table in its body names exactly which
lines are already held and by which test."

That story sits in `draft`, decomposing `epic:reference-driver` and depending on
`story:protocol-drive-verb`, and states its own outcome, quoted unedited:

> An operator whose run is refused by a lock does not go and read a design document. The refusal
> names who holds it and the exactly two commands that resolve it, and stealing a lock is something a
> person did on purpose, recorded in the run that took it.

Its own Context, quoted unedited:

> A paused run holds no lock, because an `operator` step waiting for a person has no bound and any
> age threshold would break exactly the runs that paused correctly. That makes re-acquisition on
> resume load-bearing: a resume that writes without re-taking the lock is how two live runs happen.
> The refusal follows the shape `artifact move` already uses for an illegal transition — refuse, and
> name where you can actually go.

## Done When

The requester defers this to the story. Its Acceptance section, quoted unedited:

> - A lock whose pid is alive is refused; the message carries run id, pid, host and the cursor's
>   state, and names both `--resume` and `--take-lock`.
> - A lock whose pid is **not** alive on the same host is reported stale and **still refused**
>   without `--take-lock`.
> - A lock naming a different host is never stale, whatever the local pid table says.
> - `--take-lock` writes the stolen lock's contents into the new run's cursor, so *this run took the
>   lock from pid 4711 of run `<task>/2`* is in the record.
> - `--resume` against a store whose lock another run now holds refuses.
> - The lock is absent after an approval pause while `current` still points at the run.

The story's "Re-scoped on evidence — 2026-08-28" section is the one the requester points at for which
of those lines are already held. It records the observation it was re-scoped from, quoted unedited:
"`cargo test -p aep-driver --test routing` → 7 passed; `cargo test -p protocol-cli --test drive_cli`
→ 11 passed, 2026-08-28. Staleness and host rules hold exactly as written." Its per-line table is in
the story body at `.engineering/planning/story/operator-resume-ux.md`; it is not copied here, because
the story is the record of it. The section closes: "One of these is code (`took_lock_from`), the rest
are a message field and three tests."

The story also draws a boundary — its Out of Scope, quoted unedited: "Waiting. There is no queue and
no blocking acquire — a driver that waits on a lock is a driver holding a session open for an
unbounded time."

## Notes

Declared facts — `constraints.facts` in the task document, asserted by the requester and observed by
nothing:

| Fact | Value |
|---|---|
| `change.public_contract` | `false` |
| `change.architectural` | `false` |
| `change.code` | `true` |

The requester's own comments on those facts, verbatim: "The run directory's `cursor.json` gains a
field that is already declared in the type and already printed; nothing a published schema describes
moves." And on the third: "True, and the opposite of W4-2's: this task's product is Rust. It is
declared so the plan demands the evidence kinds a code change owes — which is what
`story:evidence-producers-for-the-driven-map` exists to make producible without
`--allow-evidence-gap`."

`constraints.notes`, the three that are not the story pointer already quoted above:

- "Implementation surface is `crates/drive/aep-driver/` and `crates/edge/aep-cli/src/drive.rs`. Do not
  modify anything under `website/`, `integrations/`, `drivers/`, `.engineering/planning/`, or the
  workspace `Cargo.toml`."
- "The defect to fix first is the one with a user-visible consequence — a stolen lock leaves no
  record that it was stolen. Thread the `StolenLock` into the cursor and read it back from the run
  directory in a test."
- "The tests belong in `crates/drive/aep-driver/tests/routing.rs` and
  `crates/edge/aep-cli/tests/drive_cli.rs`; both already hold the neighbouring cases."

The story's Open Questions section, quoted unedited: "None. The age-threshold question was asked and
answered: there is deliberately no threshold." So this intake records no unanswered question.

Sources: `.engineering/task-w4-3.yaml`; `.engineering/planning/story/operator-resume-ux.md`.
