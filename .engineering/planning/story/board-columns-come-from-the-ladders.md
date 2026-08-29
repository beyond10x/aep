---
format: aep.planning-md/1
id: story:board-columns-come-from-the-ladders
kind: story
status: draft
title: The board's columns come from the ladders, not from a list compiled into the binary
summary: 'A blocker at open, or any adopter-declared status, appears in no board column: board iterates ArtifactStatus::ALL (planning.rs:2142). Found landing story:blocker-relation.'
tags:
- bug
- store
relations:
- informed_by: story:blocker-relation
revision: 1
---
# Story: The board's columns come from the ladders, not from a list compiled into the binary

## Outcome

An artifact whose status is one an adopter declared — a `blocker` at `open`, a kind of their own at a rung of their own — appears on `protocol artifact board` in a column of that name. Today it appears in no column at all, and the only trace of it is the `[blocked: …]` marker on the items it is holding up.

## Context

Found while landing `story:blocker-relation` (2026-08-30). `protocol artifact board` builds its columns from `ArtifactStatus::ALL` (`crates/protocol-cli/src/planning.rs:2142`), the list of statuses compiled into the binary — while `ArtifactStatus` is an open vocabulary (`Other(String)`, gated by each kind's ladder at `parse_status_in`, see `docs/guide/open-vocabulary.md`, row *Artifact status values a lifecycle document may name*). A `blocker` starts at `open` and ends at `cleared` (`artifacts/lifecycles/blocker.yaml`); neither is in `ALL`, so the blocker itself is invisible on the board although the story it blocks is marked correctly. `list` is unaffected: it prints rows, not columns.

## Acceptance

- `board` derives its columns from the union of the ladders the store's kinds declare (`protocol artifact lifecycle <kind>` for every kind present), in ladder order, and the compiled list is used for nothing but the default ordering of the statuses it knows.
- A store holding one `blocker` at `open` shows an `open` column with that blocker in it, asserted by a test on the passkeys fixture plus one blocker.
- A status a ladder declares and no artifact holds still gets no column — an empty column is noise, not information.
- `--kind` narrows the columns to that kind's ladder.

## Out of Scope

Rendering blockers differently from other artifacts; `story:blocker-relation` already marks the blocked side.

## Open Questions

Whether an artifact whose status is on no ladder at all (a document `validate` already refuses) should still be shown. Decides: protocol owner. Default if nobody answers: **no** — `validate` is the surface for that finding.
