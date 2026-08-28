---
format: aep.planning-md/1
id: story:history-from-the-event-log
kind: story
status: draft
title: History and audit are answered from the event log, not from a per-process memory
summary: history(), audit() and protocol artifact history read EventProvider::events; a move's provenance is on the event; D-P3 closes in full.
owner: store
tags:
- backend
- store
relations:
- decomposes: epic:planning-store-as-backend
- depends_on: story:markdown-backend-is-the-adapter
revision: 4
---
# Story: History and audit are answered from the event log, not from a per-process memory

## Outcome

`protocol artifact history story:x` and the contract's `history()` give the same answer, and a
reviewer three months later gets *what made this done* from the store — D-P3 closes in full.

## Context

`history()`, `audit()` and `describe_type()` are delegated to the in-memory backend in both durable
backends (`backend.rs:870-882`, `sqlite lib.rs:311-335`), so they answer from what *this process*
seeded. `protocol artifact history` reads `journal.jsonl` by a separate path. After F3 and G1 the
store holds sealed events with the envelope's `actor`, `recorded_at`, `correlation`, `causation`;
after F4 hydration folds them. What is not on the event is the **decision basis** — the evidence a
`MoveStatus` rested on, `{"recorded": {"test_result": 1}}` — because `DomainEvent` has
`entity, version, id, revision, type, from_state, to_state, changed` and no field for it
(`entity-runtime/crates/entity-core/src/runtime.rs:51-78`). That is the runtime story beside this
one: `story:events-carry-what-they-were-decided-on`.

Wave G, story 3 (`docs/plan/store-waves-f-g-h.md`). After G2 and the runtime story.

## Acceptance

- `EntityBackend::history()` and `audit()` read `EventProvider::events` and answer identically from a
  fresh process; the in-memory `RevisionRecord`s are a cache of the log, not the source.
- `protocol artifact history` reads the same log through the contract, and its output over this
  repository's store is unchanged for every artifact whose history predates the provider (the
  pre-provider journal lines stay readable — G1's default).
- A `MoveStatus`'s provenance is on the event, and `journal-reconciliation`'s **asserted** versus
  **recorded** distinction is decided from it.
- `story:completion-audit-join`'s first acceptance line — the admitted record joined to the artifact
  through the journal, retrievable by artifact id — is met or the story says exactly what is missing.
- The `harness-planning-and-driver-design-v0.1.md` D-P3 entry, `docs/plan/gap-register.md` and
  `website/docs/status/limitations.md` say closed, in the same commit.

## Out of Scope

`protocol explain` answering *what made this done* — that is `story:completion-audit-join`'s third
line and the engine's, not the store's.

## Open Questions

None outstanding.
