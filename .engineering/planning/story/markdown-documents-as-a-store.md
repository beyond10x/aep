---
format: aep.planning-md/1
id: story:markdown-documents-as-a-store
kind: story
status: implemented
title: The plan's documents are an entity-store provider
summary: MarkdownProvider implements Store over .engineering/planning/ — frontmatter as instance, journal.jsonl as event log — and passes entity-runtime's provider suite and the Broken check.
owner: store
tags:
- backend
- store
relations:
- decomposes: epic:planning-store-as-backend
- depends_on: story:one-adapter-over-any-store
revision: 8
---
# Story: The plan's documents are an `entity-store` provider

## Outcome

The markdown files under `.engineering/planning/` are held to a storage suite written by somebody who
has never seen them — the runtime's — and pass it. After this, "the markdown store is durable" is a
claim two independent suites support.

## Context

`aep-backend-markdown` was the one storage layer in this workspace written by hand and policed only
by its own tests: `store.rs` (706 lines), `journal.rs` (404) and the durability half of `backend.rs`
(883). `SqliteBackend` proved the other shape — an adapter over a `Store` — and F2 made it generic.
What was missing was the provider: the documents themselves as an `entity_store::Store`.

Wave G, story 1 — the load-bearing one (`docs/plan/store-waves-f-g-h.md`). After F2.

## Acceptance

- `aep_backend_markdown::provider::MarkdownProvider` implements `StateProvider`, `EventProvider` and
  `Store`; `entity_store::conformance` passes against it, and `a_broken_provider_is_caught` passes
  against a deliberately wrong copy of it. **Done** — `crates/aep-backend-markdown/src/provider.rs`;
  `tests/provider.rs`: `the_markdown_provider_conforms` (9 cases, the `ids` cases included) and
  `a_broken_copy_of_the_provider_is_caught`, over a `Broken` written on this provider.
- A refused `commit` changes neither the document nor the journal (R-84); pinned by a test that reads
  both files' bytes before and after. **Done** —
  `a_refused_commit_changes_neither_the_document_nor_the_journal`.
- A torn write is tested the way the runtime tests its own `FileStore`: the event append made to
  fail after the document landed, and the story states in its own module documentation what a reader
  finds afterwards and how `rehydrate` treats it. **Done** —
  `a_torn_write_leaves_the_document_ahead_of_its_log_and_says_so` (the journal path made a
  directory); the module doc § *The order of a commit* says what is found: a document at its new
  revision, a log one revision short, the shape wave G story 4 reports as drift; `rehydrate` rebuilds
  the previous revision and refuses nothing.
- Two writers of one document do not share a temporary path (pid + counter, as today) and the
  document is `sync_all`ed before rename. **Done** — the provider writes through
  `MarkdownStore::create`/`update`, whose `write` is unchanged (`store.rs`, pid + counter,
  `sync_all`); `a_commit_leaves_no_temporary_behind`.
- A document a person wrote by hand, with no journal entry, still loads: `events` returns an empty
  log and `load` returns the instance. **Done** —
  `a_document_a_person_wrote_by_hand_loads_with_an_empty_log`, and
  `this_repositorys_own_store_loads_and_every_document_round_trips_byte_for_byte` over all of this
  repository's documents.

## Decisions taken

- **The mapping repeats nothing the path says.** `format`, `id` and `kind` are fields only where a
  document spells them differently from `<directory>:<stem>` (an alias directory, a misfiled id), so
  an instance committed as `{title}` reads back as `{title}` — the runtime's suite compares the two.
- **Documents go through `PlanningDocument`**, the same parser and renderer every verb uses, so a
  round trip is byte-identical; the open kind and status vocabularies are what let
  `conformance-ticket` in state `open` be a valid document.
- **Journal lines are bare `DomainEvent`s**, as every runtime provider stores them; the seal is in
  the payload (F3). The open question's default is taken: the 0.19.0 entries stay as they are, and
  the log begins at the first event the provider wrote; `journal::read` reads both shapes.
- **Numbers travel as `Node` carries them** — floating point, so `sprint: 42` reads as `42.0`.

## Out of Scope

Replacing `MarkdownBackend`'s body (G2). Relations into frontmatter and kind templates — those are
plan-shaped projections above the provider, and G2 owns them.

## Open Questions

None outstanding; the one that was open is decided above.
