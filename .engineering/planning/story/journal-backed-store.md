---
format: aep.planning-md/1
id: story:journal-backed-store
kind: story
status: implemented
title: 'P3: the markdown store writes through CommandService'
summary: The store's two write functions reroute through command envelopes, the journal becomes the history it does not have, and the sixteen conformance suites run against it.
owner: store
tags:
- backend
- store
relations:
- decomposes: epic:planning-store-as-backend
revision: 4
---
# Story: P3 — the markdown store writes through `CommandService`

## Outcome

Anyone asking *"is there a durable backend?"* gets an answer the sixteen conformance suites support,
and anyone asking *"what did this story look like three revisions ago, and who moved it?"* gets an
answer from the store rather than from `git log`.

## Context

Deviation **D-P1** was taken on the way in and recorded: the store writes through its own
`create`/`update` rather than through the contract's one write path, which is why the suites do not
run against it and why it has no journal, no audit join and no history (**D-P3**). The mitigation was
that every write funnels through exactly two functions. This is the story that spends that
mitigation.

## Acceptance

- Both write functions route through command envelopes; a source scan finds no other write path in
  the crate.
- `protocol conformance` runs all sixteen `aep-conformance` suites against the markdown store and
  passes, against the same `FaultyBackend` baseline that proves the suites catch injected defects.
- The journal answers what an artifact looked like at a given revision, and who moved it, without
  reading git.
- `describe_type` reports the kind's lifecycle, closing **D-P5**.
- An out-of-band file edit is still indistinguishable in the file (**D-P2**) and the store says so
  rather than pretending otherwise.

## Out of Scope

A new store trait. The seam is the existing `aep-contract` traits; a second trait would be a second
write path and `crates/aep-contract/tests/write_surface.rs` would fail on the declaration.

## Open Questions

Where the journal lives on disk — beside the artifacts or in one file per store. **Answered by what
shipped**: one append-only `journal.jsonl` per store, which is what `journal::append` writes.

## Decisions, 2026-08-26

| question | decision | why |
|---|---|---|
| `CommandService::execute` is async; this store is synchronous file IO | **Wrap the sync store; do not make the store async.** The impl body completes without awaiting, exactly as `aep-backend-memory` does (`crates/aep-backend-memory/src/command.rs:40` carries the same `unused_async_trait_impl` allow and the same one-line reason) | The async belongs to the contract, not to this backend. Making file IO async would pull a runtime into a crate that needs none, and `aep-backend-memory` has already established that a sync body behind an async signature is the shape this contract expects |
| P4: a second hand-written store, or an adapter | **An adapter over `entity-runtime`'s `entity-sqlite`.** It ships one `BEGIN`, both writes, one `COMMIT`, a busy timeout, and a conformance suite of its own | ADR 0002 already points the dependency arrow this way — `aep-backend-markdown` takes `entity-core` today. Writing a second transactional store by hand would be building, badly, the thing next door that is already tested against a torn write |

## Found while reading, not yet fixed

`MarkdownStore::write` (`crates/aep-backend-markdown/src/store.rs:262`) derives its temporary file
name from the document's filename alone — `.{name}.tmp` — so two writers of one document share it,
and one writer's `rename` can install an inode the other is still filling. There is no `fsync`
either. This is the same defect `entity-runtime` 0.8.0 fixed in its own `FileStore`, and P3 rewrites
this exact write path, so it is fixed there rather than in a patch that would have to be written
twice.
