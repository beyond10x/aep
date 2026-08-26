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

## What is done, and the one acceptance line that is not

Done: `MarkdownBackend` implements `CommandService` and `QueryService`; the sixteen suites pass and
are shown to fail it under injected faults; a command reaches the file and the prose survives; the
journal answers without git; and a source scan pins that **nothing inside this crate** writes except
`MarkdownBackend::persist`, which exists only to be called by `execute`
(`the_only_write_path_out_of_this_crate_is_a_command`, with a guard test that plants a write the
scan must refuse).

**Not done: `protocol artifact`'s own verbs still call `MarkdownStore::create` and `update`
directly** — `crates/protocol-cli/src/planning.rs:495, 577, 662, 719`. Four sites, in a different
crate, so the in-crate scan does not see them.

They cannot be routed through the backend yet, and the reason is specific rather than effort:
`persist` copies back **only the frontmatter fields an entity body carries** — status, title,
summary, owner. It cannot create a file that does not exist, and it cannot project a relation into
frontmatter, because an entity's relations live as separate `Relation` records and nothing maps them
back. So `artifact new` and `artifact relate` have no command path that would produce the document
they are for.

**One of the two projections has since landed.** `persist` now creates a document for an entity that
has none, when the entity's address says it belongs in this store —
`ep://planning/store/<kind>/<name>`, which is the address `seed` files an artifact at. Its
frontmatter is what the entity carries; its **body is empty**, because a body this crate invented
would be prose nobody is accountable for in a document that reads as though somebody had thought
about it. An entity addressed anywhere else — a conformance suite's — still gets no file and is
reported by `unprojected` (`a_created_entity_becomes_a_document_this_store_holds`,
`an_entity_this_store_is_not_addressed_for_gets_no_invented_file`).

**Both projections have since landed.** The relation one needed a contract fact stated first:
`CreateRelation` reports **no affected entity**, because an edge is a thing in its own right and
neither endpoint's revision moves. The *document* still changes, since a planning document carries
its edges in frontmatter, so `execute` projects the source explicitly — and deliberately does **not**
bump the document's revision, because claiming the artifact moved when only an edge was added would
be a lie in the journal.

`apply_relations` **adds and does not remove**. Rewriting the list from the contract's view would
delete, on the first status move, every edge written by hand into a document this backend has never
been the author of. Projecting `RemoveRelation` is its own piece of work.

## The decision that blocks the last acceptance line

`persist` can now produce every document shape the CLI's verbs need. Routing them is still blocked,
and on a question rather than on effort.

**`protocol artifact new` writes a body** — `template(&document_root, &kind)`, the kind's starting
document (`planning.rs:490`). A command carries an entity, and this backend's create writes an
**empty** body on purpose: *the prose is the document*, an entity holds none, and a body invented by
this crate would be prose nobody is accountable for. Routing `new` through `execute` as it stands
would silently drop the template.

So one of two things has to be true, and neither is free:

| | what it costs |
|---|---|
| **prose enters the contract as entity data** | every status move carries the whole body through the command path, and the entity becomes the document — which is the distinction this backend was built to keep |
| **the CLI keeps a second write path for bodies** | exactly what invariant 14 forbids, and what D-P1 is |

A third option exists and needs designing rather than choosing: a command whose payload *is* a
document body, so a body change is a command like any other and the entity still holds no prose.

**Until that is decided, all four CLI sites stay as they are.** Routing three and leaving `new`
direct would be two write paths rather than one, which is the defect, not a step toward fixing it.

**D-P1 stays open against the CLI** — now against four named lines and one decision, rather than a
missing capability.

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
