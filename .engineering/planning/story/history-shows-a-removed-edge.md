---
format: aep.planning-md/1
id: story:history-shows-a-removed-edge
kind: story
status: draft
title: A removed edge leaves a trace in the entity stores
summary: Identity::before never sees RemoveRelation, so history over SQLite and Postgres shows nothing for an unrelate.
relations:
- serves: vision:O2
revision: 1
---
# Story: A removed edge leaves a trace in the entity stores

## Context

`aep artifact unrelate` landed in 0.53.0. Over the markdown backend the removal is a journal entry
a reader can follow. Over SQLite and Postgres the edge is removed correctly, but nothing lands on
the source artifact, so `aep plan artifact history <id>` there shows no sign that an edge was ever
taken back.

The cause is narrow and named: `Identity::before` in `crates/plan/aep-backend-entity/src/lib.rs`
takes its inner command as `_inner` and so never sees `RemoveRelation`. After the edge is gone,
`change_of_event` cannot recover the relation kind or the target from the stored
`{"command":"remove-relation","relation":"rel-…"}`, because the id it names no longer resolves. The
note has to be written at removal time, and `aep-backend-entity` cannot reach the vocabulary that
would describe it — it does not depend on `aep-backend-markdown`.

## Acceptance

`aep plan artifact history <id>` over a SQLite store names the removed edge's relation kind and
target after an `unrelate`, and the assertion is a test in the backend conformance suite so both
entity backends are held to it.
