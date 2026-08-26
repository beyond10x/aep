---
format: aep.planning-md/1
id: story:sqlite-backend-adapter
kind: story
status: implemented
title: 'P4: aep-backend-sqlite, an adapter over entity-sqlite rather than a second hand-written store'
summary: 'The first database backend: one file, no server, and the transactional store next door rather than a second one written here.'
relations:
- decomposes: epic:planning-store-as-backend
revision: 4
---
# Story: P4 — `aep-backend-sqlite`

## Outcome

An adopter who wants the contract on a database gets one: a single file, no server, holding whatever
the contract holds — including entities a conformance suite invents, which the markdown store cannot
shape because it is shaped like a plan.

## Context

`story:sqlite-backend` proposed writing this. It is written as an **adapter** instead, over
`entity-runtime`'s `entity-sqlite`, on the decision recorded in `story:journal-backed-store`: that
store already writes an instance and its events inside one transaction, with a busy timeout so a
second writer waits rather than being told the system is broken, and it has a test that tears a
write. A second transactional store written here would be building, badly, the thing next door that
is already tested against the case that matters.

The dependency arrow is the one `atlas/architecture/adr/0002` already points.

## Acceptance

- `SqliteBackend` implements `CommandService` and `QueryService`; the sixteen suites pass at
  `Level::Full`.
- The suites are shown to **fail** it under injected faults, so passing is evidence rather than
  decoration.
- What the contract accepted is read back out of the file through a **second** handle, so nothing in
  the backend under test is asked whether its own write happened.
- The contract logic is not written twice: commands go to `aep-backend-memory` and this crate adds
  durability, exactly as `aep-backend-markdown` does.

## Two things this got wrong first, and what they cost

- The durable expectation was **derived** from the contract's revision — *the store must be at mine
  minus one*. That assumes every command bumps by exactly one, which is already false for a command
  touching an entity twice; 15 of 31 checks failed. The expectation is **read** now. What `Expect`
  still catches is the database moving underneath this process between the read and the write, which
  is what it is for.
- A replay was persisted. Nothing changed, so the write pushed a revision the store already held,
  its own check rightly refused, and the backend latched over a command that did no harm. A replay
  now writes nothing.

## Out of Scope

Hydrating from the database on open. This backend persists what the contract accepts; reading a
populated file back into a fresh process is P5's problem and needs the audit and relation tables
this story does not add.

## Open Questions

None outstanding.
