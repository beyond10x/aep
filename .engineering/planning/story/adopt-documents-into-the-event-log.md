---
format: aep.planning-md/1
id: story:adopt-documents-into-the-event-log
kind: story
status: draft
title: A store adopted into the event log has no documents the drift check must ignore
relations:
- informed_by: story:skill-text-cannot-instruct-a-direct-store-write
scope:
- confidence: inferred
  path: crates/edge/aep-cli/src/reverse.rs
- confidence: cited
  path: crates/plan/aep-backend-markdown/src/drift.rs
- confidence: inferred
  path: crates/plan/aep-backend-markdown/src/journal.rs
revision: 9
---
# Story: A store adopted into the event log has no documents the drift check must ignore

## Outcome

`protocol reverse adopt` writes one genesis event per document that has none, so every document in
the store is drift-checkable and **pre-provider** stops being a status any document holds.

## Context

`crates/plan/aep-backend-markdown/src/drift.rs:16` states the gap in its own table:

| finding | what it means | exit |
|---|---|---|
| **drift** | the frontmatter disagrees with its last event — somebody edited `status:`, `title:`, an edge | 1 |
| **forged revision** | the document claims a revision no logged write produced | 1 |
| **deleted** | the log holds events for a document that is not there | 1 |
| **pre-provider** | the document has **no events at all** — "a normal condition and not a defect" | **0** |

`protocol artifact validate` on this repository reports **76 document(s) predate the event log**
(measured 2026-08-30). Every one of those 76 can be edited by hand — `status:`, `title:`, an edge —
and validate says nothing, because with no event to disagree with there is nothing to compare
against. The check that protects the store protects 114 of its 190 documents.

**This is what closed `story:skill-text-cannot-instruct-a-direct-store-write`.** That story asked
for a source scan over shipped prose to stop a skill *instructing* a hand edit. It was built, and
deleted, because validate already refuses the hand edit itself:

```console
$ sed -i 's/^status: draft$/status: active/' <store>/story/probe.md
$ protocol artifact validate
1 problem(s):
  - story:probe drifted from its log: status disagrees with event story:probe@1#0~584ad5e7de54a3c7
    — an edit made outside a command is a change nothing decided
VALIDATE EXIT=1
```

That is the right instrument, and this story is the one thing it needs to cover the whole store
rather than the newer part of it.

**Why `reverse` and not `artifact`.** `protocol reverse` is already the verb group for *reading a
repository that exists into the protocol's own terms*, and `reverse init` already performs the
adjacent one-time act — writing the `project.yaml` that makes a repository an adopting project. A
store whose documents predate its log is the same situation one level down: real state that the
protocol did not produce and now has to account for. `artifact` is the group whose verbs each
perform one planning act on one artifact; this is neither.

## Acceptance

- `protocol reverse adopt` appends one event per document that has no events, recording the
  document's current frontmatter as the state adoption found — not a state it invented.
- The event is distinguishable from a write: a reader can tell *adopted at this state* from *a
  command set this state*, so adoption never reads as provenance it does not have.
- After it runs on this repository, `protocol artifact validate` reports **0 documents that predate
  the event log**, and a hand edit to any of the 76 is reported as drift.
- It is idempotent: a second run appends nothing.
- It refuses a store that is already fully adopted rather than writing a second genesis event.

## Out of Scope

- **Back-dating.** An adoption event is dated when adoption ran, not when the document was written.
  The document's own history is in `git log`, which is where it stays.
- **Reconstructing the writes that were never logged.** Adoption records one state, not a history.

## Open Questions

**Does an adoption event carry a revision, and which one?** Decides: protocol owner. Default if
nobody answers: **the document's current `revision`**, so the forged-revision check (a document
claiming more than the highest revision recorded) keeps working immediately after adoption rather
than being disarmed by a genesis event at revision 0.

## Not established

- Whether the journal's older entries can hold an event shape that writes nothing. `drift.rs:28`
  notes that *"an evidence record is an event at the current revision that writes nothing"*, so a
  no-op event shape exists in some form — **inferred** from that comment, not read from the event
  type.
- Whether 76 is stable. It was read from `protocol artifact validate` on 2026-08-30 against a store
  of 190 artifacts. It falls as documents are rewritten through the CLI and never rises.
