---
format: aep.planning-md/1
id: story:markdown-backend-is-the-adapter
kind: story
status: implemented
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
revision: 9
---
# Story: `MarkdownBackend` is the adapter over that provider, and the hand-written write path leaves

## Outcome

There is one place in this workspace that turns a contract command into a durable write, and the
markdown store is not an exception to it. Nobody using `protocol artifact` can tell.

## Context

`MarkdownBackend` (`crates/aep-backend-markdown/src/backend.rs`, 883 lines) wrapped `MemoryBackend`,
applied, then `persist`ed: wrote the document, projected relations into frontmatter, appended the
journal, latched on failure. `EntityBackend<S>` (F2) did all of that except the plan-shaped
projection, and `MarkdownProvider` (G1) is the `S`.

Wave G, story 2 (`docs/plan/store-waves-f-g-h.md`). After G1 and F3.

## Acceptance

- `MarkdownBackend` is `EntityBackend<MarkdownProvider>` plus a projection hook; `persist`, the
  latch and `journal::append` in `backend.rs` are **deleted**, not kept beside the new path. **Done**
  — `backend.rs` is 196 lines: the newtype, `open`, and forwarding. The adapter gained a
  `Projection` seam (`EntityBackend<S, P = Identity>`); `projection.rs` (613 lines) is the plan's
  shape: `place` (prose preserved, ladder checked, relations added), `observe` (an evidence record as
  an event at the document's current revision), `hydrate` (seeding as before).
- The sixteen suites pass at `Level::Full` and the faulty-backend guard fails it, unchanged. **Done**
  — `tests/conformance.rs`, the same tests.
- The D-P1 scans pass **without being edited**. **Partly** —
  `no_planning_verb_writes_to_the_store_except_through_a_command` (protocol-cli) is unedited and
  passes. `the_only_write_path_out_of_this_crate_is_a_command` had to be **widened**, not weakened:
  it lists its sources by name and `provider.rs` did not exist, so it would have passed while blind
  to the module that now holds the write — the exact defect its own `store.rs` comment records. It
  now lists `provider.rs` and `projection.rs`, and the permitted site is `MarkdownProvider::commit`.
  The guard test is unchanged. Recorded here rather than ticked.
- A golden test over this repository's own `.engineering/planning/`: `list`, `board`, `graph`,
  `validate`, `history` and `lifecycle` byte-identical before and after, in `text` and `json`.
  **Done, over a recorded fixture** — `crates/protocol-cli/tests/golden_plan.rs` with
  `fixtures/golden-plan/` recorded by the released 0.28.0 binary: the six read verbs in both
  formats byte-identical (`history` with its per-run instant and user stripped), and — the half
  that matters — `new`, `relate`, `body`, `move`, `evidence` leaving byte-identical documents and
  the same journal entries. Over this repository's own store the read verbs do not touch the
  backend, and the provider test round-trips all 125 documents byte for byte.
- `protocol artifact new/relate/body/move/evidence` each still produce exactly the document they
  produce today. **Done** — the 137 CLI tests and the golden fixture.
- The crate's module documentation and `docs/guide/backend.md` describe one shape — adapter over
  provider — and the line count of `backend.rs` is stated in the changelog beside what left.
  **Done.**

## Decisions taken

- **Every event carries the journal's own word for the change** under `payload.change`
  (`journal::Change`), so `journal::read` answers the same `Entry` for a line written before the
  provider and one written after it, and `protocol artifact history` reads both without knowing.
- **An observation is an event at the document's current revision** — no write to the document,
  an appended line — which is what `evidence_on_hand` counts for a gated move.
- **A no-op writes nothing.** The adapter skips a placement whose fields and state the provider
  already holds; a command that changed nothing does not bump a revision or journal a change.
- **Projection refusals are not latched.** A status off the ladder is refused before anything
  durable disagrees, exactly as before.

## Out of Scope

Reading history from the event log (G3). Drift detection (G4).

## Open Questions

None outstanding.
