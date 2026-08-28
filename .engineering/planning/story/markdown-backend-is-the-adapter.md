---
format: aep.planning-md/1
id: story:markdown-backend-is-the-adapter
kind: story
status: draft
title: MarkdownBackend is the adapter over that provider, and the hand-written write path leaves
summary: MarkdownBackend becomes EntityBackend<MarkdownProvider> plus the plan-shaped projection; persist, latch and journal::append are deleted; a golden test over this repository's store proves nobody can tell.
owner: store
tags:
- backend
- store
relations:
- decomposes: epic:planning-store-as-backend
- depends_on: story:markdown-documents-as-a-store
- depends_on: story:events-reach-the-store
revision: 5
---
# Story: `MarkdownBackend` is the adapter over that provider, and the hand-written write path leaves

## Outcome

There is one place in this workspace that turns a contract command into a durable write, and the
markdown store is not an exception to it. Nobody using `protocol artifact` can tell.

## Context

`MarkdownBackend` (`crates/aep-backend-markdown/src/backend.rs`) wraps `MemoryBackend`, applies,
then `persist`s: writes the document, projects relations into frontmatter, appends the journal,
latches on failure. `EntityBackend<S>` (F2) does all of that except the plan-shaped projection, and
`MarkdownProvider` (G1) is the `S`. What remains ours is the projection: the kind's template body on
create, relations into frontmatter (`apply_relations` adds and does not remove — that stays true),
and `unprojected` for an entity addressed outside this store.

Wave G, story 2 (`docs/plan/store-waves-f-g-h.md`). After G1 and F3.

## Acceptance

- `MarkdownBackend` is `EntityBackend<MarkdownProvider>` plus a projection hook; `persist`, the
  latch and `journal::append` in `backend.rs` are **deleted**, not kept beside the new path.
- The sixteen suites pass at `Level::Full` and the faulty-backend guard fails it, unchanged.
- The D-P1 scans — `the_only_write_path_out_of_this_crate_is_a_command` and
  `no_planning_verb_writes_to_the_store_except_through_a_command` — pass **without being edited**.
- A golden test over this repository's own `.engineering/planning/`: `list`, `board`, `graph`,
  `validate`, `history` and `lifecycle` are byte-identical before and after, in `text` and `json`.
- `protocol artifact new/relate/body/move/evidence` each still produce exactly the document they
  produce today (the existing per-verb tests are the pin).
- The crate's module documentation and `docs/guide/backend.md` describe one shape — adapter over
  provider — and the line count of `backend.rs` is stated in the changelog beside what left.

## Out of Scope

Reading history from the event log (G3). Drift detection (G4).

## Open Questions

None outstanding.
