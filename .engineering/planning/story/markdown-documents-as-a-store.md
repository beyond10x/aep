---
format: aep.planning-md/1
id: story:markdown-documents-as-a-store
kind: story
status: draft
title: The plan's documents are an entity-store provider
summary: MarkdownProvider implements Store over .engineering/planning/ — frontmatter as instance, journal.jsonl as event log — and passes entity-runtime's provider suite and the Broken check.
owner: store
tags:
- backend
- store
relations:
- decomposes: epic:planning-store-as-backend
- depends_on: story:one-adapter-over-any-store
revision: 4
---
# Story: The plan's documents are an `entity-store` provider

## Outcome

The markdown files under `.engineering/planning/` are held to a storage suite written by somebody who
has never seen them — the runtime's — and pass it. After this, "the markdown store is durable" is a
claim two independent suites support.

## Context

`aep-backend-markdown` is the one storage layer in this workspace written by hand and policed only by
its own tests: `store.rs` (706 lines), `journal.rs` (404) and the durability half of `backend.rs`
(883). `SqliteBackend` proved the other shape — an adapter over a `Store` — and F2 made it generic.
What is missing is the provider: the documents themselves as an `entity_store::Store`.

The mapping is direct. A document's frontmatter is the instance's fields; `status` is
`lifecycle_state`; `revision` is `revision`; the body is a `body` field. `journal.jsonl` is the event
log, one sealed `Envelope<DomainEvent>` per line. `commit` checks `Expect` against the file's
revision, writes the document through the existing temp-name + `sync_all` path
(`store.rs:271`, `store.rs:289`), appends the events, and says which failure leaves what.

Wave G, story 1 — the load-bearing one (`docs/plan/store-waves-f-g-h.md`). After F2.

## Acceptance

- `aep_backend_markdown::provider::MarkdownProvider` implements `StateProvider`, `EventProvider` and
  `Store`; `entity_store::conformance` passes against it, and `a_broken_provider_is_caught` passes
  against a deliberately wrong copy of it.
- A refused `commit` changes neither the document nor the journal (R-84); pinned by a test that reads
  both files' bytes before and after.
- A torn write is tested the way the runtime tests its own `FileStore`: the event append made to
  fail after the document landed, and the story states in its own module documentation what a reader
  finds afterwards and how `rehydrate` treats it.
- Two writers of one document do not share a temporary path (pid + counter, as today) and the
  document is `sync_all`ed before rename.
- A document a person wrote by hand, with no journal entry, still loads: `events` returns an empty
  log and `load` returns the instance, because a plan that predates the provider is a normal
  condition and refusing it would refuse this repository's own store.

## Out of Scope

Replacing `MarkdownBackend`'s body (G2). Relations into frontmatter and kind templates — those are
plan-shaped projections above the provider, and G2 owns them.

## Open Questions

Whether `journal.jsonl`'s existing lines (the 0.19.0 shape: who, when, artifact, revision, what
changed, provenance) are read as events or left as a pre-provider history the log begins after.
Decides: store owner. Default if nobody answers: **left as is and read by `protocol artifact history`
as before**; the event log begins at the first `commit` this provider makes, and the boundary is a
line the log records.
