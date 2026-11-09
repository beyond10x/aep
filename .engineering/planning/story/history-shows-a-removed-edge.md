---
format: aep.planning-md/3
id: story:history-shows-a-removed-edge
kind: story
status: active
title: A removed edge leaves a trace in the entity stores
summary: Identity::before never sees RemoveRelation, so history over SQLite and Postgres shows nothing for an unrelate.
relations:
- serves: vision:O2
- decomposes: epic:zero-open-issues-2026-11-09
- depends_on: story:relate-and-unrelate-write-the-revision-they-print
scope:
- confidence: inferred
  path: crates/edge/aep-cli/src/planning.rs
- confidence: inferred
  path: crates/edge/aep-cli/tests/sqlite_plan.rs
- confidence: cited
  path: crates/plan/aep-backend-entity/src/lib.rs
- confidence: inferred
  path: crates/plan/aep-backend-markdown/src/journal.rs
- confidence: inferred
  path: crates/plan/aep-conformance/src
revision: 9
transitions:
- {from: "draft", to: "proposed", at: "2026-11-09T22:07:27Z", actor: "human:timo", revision: 6}
- {from: "proposed", to: "active", at: "2026-11-09T22:07:27Z", actor: "human:timo", revision: 7}
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

## Triage 2026-11-09

Still present at `824869fec`. `crates/plan/aep-backend-entity/src/lib.rs:543-554`: `Identity::before` matches `CreateRelation` and `RecordEvidence` only; `RemoveRelation` falls to `_ => None`.

Reproduced with aep 0.79.0 on a SQLite store: `relate a depends_on b`, `relate a depends_on c`, `unrelate a depends_on b`, `relate a depends_on b`; `history task:a` then lists `created`, `depends_on task:b`, `depends_on task:c`, `depends_on task:b` and no removal. The markdown wording for a removal is at `crates/plan/aep-backend-markdown/src/journal.rs:356`.
