---
format: aep.planning-md/1
id: story:events-reach-the-store
kind: story
status: implemented
title: A command's record reaches the store as events, so the file holds the history
summary: 'persist commits events: Vec::new(); each accepted command becomes DomainEvents sealed in an Envelope and written with the instance in one commit, so R-83 holds across the seam.'
owner: store
tags:
- backend
- store
relations:
- decomposes: epic:planning-store-as-backend
- depends_on: story:one-adapter-over-any-store
revision: 8
---
# Story: A command's record reaches the store as events, so the file holds the history

## Outcome

A SQLite plan file, opened by a second process, can say who moved a story and when — because the
store holds the events, not because a process that has since exited remembered them.

## Context

`entity-sqlite` writes an instance and its events inside one transaction (runtime R-83) so *"a
state cannot move without the event that explains it"*. `persist` handed it
`Decision { instance, events: Vec::new() }`: the guarantee protected an instance and an empty list.
Every `AuditRecord`, every `RevisionRecord` and the provenance a `MoveStatus` rested on lived only in
the per-process `MemoryBackend`.

The shape the runtime expects is public: `entity_store::Recording { recorded_at, correlation,
causation, actor }` seals `DomainEvent`s into `Envelope`s with derived, clock-free ids (R-86, R-88).
The command envelope carries every one of those four values.

Wave F, story 3 (`docs/plan/store-waves-f-g-h.md`). After F2.

## Acceptance

- Each accepted command produces one `DomainEvent` per affected entity — `from_state`/`to_state`
  from the entity's `status` before and after, `changed` = the fields the command wrote — sealed with
  a `Recording` whose `correlation` is the command's correlation, `causation` the command id, `actor`
  the envelope's actor, `recorded_at` the instant the envelope carried. **Done** —
  `crates/aep-backend-entity/src/lib.rs`: `Provenance::of(&envelope)` is taken before the contract
  consumes the envelope; `persist` builds the event from what the store held a moment ago and what
  the contract holds now; `Recording::seal` mints the id (`aep.entity:<id>@<revision>#0`).
- Read through a **second** handle on the same file: `events(entity, id)` returns exactly one event
  per accepted command against that entity, in order, and zero for a refused one (R-84 across the
  seam). **Done** — `tests/events.rs`:
  `each_accepted_command_is_one_event_in_the_file_read_through_a_second_handle`,
  `a_refused_command_writes_no_event_and_the_file_says_so`.
- A replayed command (`CommandOutcome::Replayed`) writes no event, as it writes no instance. **Done**
  — `a_replayed_command_writes_no_event_as_it_writes_no_instance`, which first asserts the outcome
  *was* `Replayed`.
- An entity's history folds: `entity_core::replay` over the stored events reproduces the stored
  instance (runtime R-97), pinned by a test over a create, an update and a move. **Done** —
  `the_stored_events_fold_back_to_the_stored_instance`: `rehydrate(definition, events) == load()`,
  state, revision and every field.
- The event type is the command's type name (`aep.status.move/v1` and so on). **Done** — asserted
  as `["aep.entity.create/v1", "aep.entity.update/v1", "aep.status.move/v1"]`.

Guard verified by breaking it: with `events: Vec::new()` restored in `persist`, all four tests fail;
reverted, all four pass (2026-08-28).

## Decisions taken, and one finding for the runtime

**Where the seal and the provenance live: the event's `payload`.** The open question's default was
the envelope's `causation`, labelled. That default cannot hold, for a reason that is a fact about the
runtime rather than a preference: **its providers do not store envelopes.** `Store::commit` takes a
`Decision`, whose `events` are bare `DomainEvent`s; `entity-sqlite` writes `decision.events`
(`crates/entity-sqlite/src/lib.rs:205-215` at 0.9.1) and `entity-cli` prints the sealed envelopes and
commits the decision (`crates/entity-cli/src/main.rs:344-362`). `recorded_at`, `correlation`,
`causation` and `actor` never reach the file through the SPI. So the sealed envelope's five fields
are written into the event's `payload`, and a `MoveStatus`'s `decided_on` account beside them —
`null` when the command carried none, never an absent key. G3 reads them from there until the
runtime gives them a home (`story:events-carry-what-they-were-decided-on`, and the finding below).

**Finding, recorded in `entity-runtime` as `story:the-store-keeps-the-envelope` (draft):** an
adopter that keeps who/when/why durable has to smuggle the envelope into `payload`, because the SPI
seals it and then drops it. That is theirs to decide; it is written down there so it is not
re-discovered.

**`changed` is a diff, and it cannot say "removed".** `DomainEvent::changed` is a map of values;
the contract's `UpdateEntity` merges and never removes, so the case does not arise today, and the
note is on `changed_between`.

**Creation into a non-initial status does not fold.** The runtime's `rehydrate` holds a creation
event to `lifecycle.initial` (their 0.8.0 fix). The contract creates an entity at whatever `status`
the command carries — `seed` files an implemented story as `implemented` — so a plan's history does
**not** fold in general, and F4's *"folds each from its events (or loads its state and checks the
fold agrees)"* must take the second road. Recorded here so F4 does not discover it.

**`Timestamp::iso_8601`** moved onto the type in `aep-domain`; the markdown backend's private copy
of the calendar arithmetic now delegates to it, so both backends stamp one spelling.

## Out of Scope

Reading the events back into `history()`/`audit()` — that is G3, and needs the event to carry the
decision basis (`entity-runtime` `story:events-carry-what-they-were-decided-on`). This story writes;
the file holding the history is the claim, and a second handle reading it is the evidence.

## Open Questions

None outstanding; the one that was open is decided above.
