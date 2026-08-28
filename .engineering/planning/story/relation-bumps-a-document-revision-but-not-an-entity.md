---
format: aep.planning-md/1
id: story:relation-bumps-a-document-revision-but-not-an-entity
kind: story
status: draft
title: An edge moves a document's revision and leaves an entity's alone
summary: 'protocol artifact relate answers revision 3 over markdown and revision 1 over SQLite for one plan: the markdown projection counts a frontmatter write as a revision, the contract does not count a relation as one. Decide which is right and make the other store say it.'
owner: store
tags:
- backend
- store
relations:
- decomposes: epic:planning-store-as-backend
- depends_on: story:store-selection-in-project-yaml
revision: 4
---
# Story: an edge moves a document's revision and leaves an entity's alone

## Outcome

`protocol artifact relate` answers the same revision over every store, because the two stores agree
on what a revision counts.

## Context

Found by `crates/protocol-cli/tests/store_selection.rs`, the test that runs every verb over the same
plan on markdown and on SQLite. After `relate story:x decomposes epic:y`, the markdown plan prints
`revision 2` for `story:x` and the SQLite plan prints `revision 1`; after seven artifacts with their
edges are seeded, `graph --format json` reports `version` 1–3 on files and 1 everywhere in SQLite.

The markdown projection counts a frontmatter write as a revision: the edge is written into the
document, so the document changed. The contract counts a relation as a record of its own: the
memory backend's `CreateRelation` names no affected entity, so the entity's revision does not move,
and `Identity` writes an observation event at the unchanged revision (H1). Both are defensible; they
are not the same, and the golden test (`story:markdown-backend-is-the-adapter`) pins the markdown
behaviour as 0.28.0's bytes. The comparison test writes revision numbers as `#` until this is
decided, and says so in its own doc comment.

## Acceptance

- One answer, recorded as an ADR-sized note in `docs/guide/backend.md`: either a relation moves the
  source's revision in the contract (memory, SQLite, Postgres alike — a conformance suite change)
  or the markdown projection stops counting it (a golden-fixture change, and 0.28.0 compatibility
  said goodbye to in the CHANGELOG).
- `store_selection.rs` compares revision numbers again; `blank_number_after` is deleted.
- `describe_type`/`history` unchanged either way: a `Related` entry carries whatever revision the
  store assigned.

## Out of Scope

Anything about relations other than the revision they cost.
