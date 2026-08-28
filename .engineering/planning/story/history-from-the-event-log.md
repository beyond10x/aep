---
format: aep.planning-md/1
id: story:history-from-the-event-log
kind: story
status: implemented
title: History and audit are answered from the event log, not from a per-process memory
summary: history(), audit() and protocol artifact history read EventProvider::events; a move's provenance is on the event; D-P3 closes in full.
owner: store
tags:
- backend
- store
relations:
- decomposes: epic:planning-store-as-backend
- depends_on: story:markdown-backend-is-the-adapter
revision: 8
---
# Story: History and audit are answered from the event log, not from a per-process memory

## Outcome

`protocol artifact history story:x` and the contract's `history()` give the same answer, and a
reviewer three months later gets *what made this done* from the store — D-P3 closes in full.

## Context

`history()`, `audit()` and `describe_type()` were delegated to the in-memory backend in both durable
backends, so they answered from what *this process* seeded. `protocol artifact history` read
`journal.jsonl` by a separate path. After F3 and G1 the store holds sealed events; after F4 hydration
reads them; and `entity-runtime` 0.11.0 put the decision basis on the event
(`story:events-carry-what-they-were-decided-on`, `DomainEvent::args`).

Wave G, story 3 (`docs/plan/store-waves-f-g-h.md`). After G2 and the runtime story.

## Acceptance

- `EntityBackend::history()` and `audit()` read `EventProvider::events` and answer identically from a
  fresh process; the in-memory `RevisionRecord`s are a cache of the log, not the source. **Done** —
  the adapter asks the projection for an entity's coordinates (`Projection::coordinates`) and reads
  the provider's events: `history()` is one record per revision from the seals; `audit()` is this
  process's records plus an accepted-command record for every logged command it did not see, held
  to the same filters. `crates/aep-backend-markdown/tests/history.rs`:
  `a_second_process_answers_the_history_the_first_wrote` — a second process opened at a different
  seed instant answers the first's history byte for byte and its audit trail with the first's command
  ids; `a_document_that_predates_the_provider_still_has_the_history_it_had`.
- `protocol artifact history` reads the same log through the contract, and its output over this
  repository's store is unchanged for every artifact whose history predates the provider. **Done
  through the same log, not through the contract's `history()`** — the verb reads `journal.jsonl`
  via `journal::read`, which since G2 answers an `Entry` for a line written before the provider and
  for an event written after it (the event's payload carries the journal's own `change`); the golden
  test pins the output byte-identical, and `RevisionRecord` has no change description to print, so
  routing the verb through `QueryService::history` would have changed the very output this line
  protects. Two readers, one log; recorded here rather than ticked.
- A `MoveStatus`'s provenance is on the event, and `journal-reconciliation`'s **asserted** versus
  **recorded** distinction is decided from it. **Done** — the event's `args.decided_on` (0.11.0) and
  the `payload.change` note carry the provenance; `validate`'s *closed on an assertion* reads it
  through `journal::read` for both line shapes.
- `story:completion-audit-join`'s first acceptance line — the admitted record joined to the artifact
  through the journal, retrievable by artifact id — is met or the story says exactly what is missing.
  **Met, and the second line too**: `protocol artifact evidence` records an observation as an event
  about the document at its current revision; `protocol artifact history <id>` retrieves it by
  artifact id with the revision it was admitted at. Lines three (`protocol explain`) and four (a
  removed source file) remain that story's.
- The `harness-planning-and-driver-design-v0.1.md` D-P3 entry, `docs/plan/gap-register.md` and
  `website/docs/status/limitations.md` say closed, in the same commit. **Done.**

## Out of Scope

`protocol explain` answering *what made this done* — that is `story:completion-audit-join`'s third
line and the engine's, not the store's.

## Open Questions

None outstanding.
