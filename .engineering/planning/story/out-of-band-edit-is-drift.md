---
format: aep.planning-md/1
id: story:out-of-band-edit-is-drift
kind: story
status: implemented
title: An out-of-band edit, and a deleted document, are reported as drift
summary: protocol artifact validate compares each document with its last event and reports drift, deletion or pre-provider; D-P2 and D-P4 close by detection, prevention refused on the record.
owner: store
tags:
- backend
- store
relations:
- decomposes: epic:planning-store-as-backend
- depends_on: story:history-from-the-event-log
revision: 8
---
# Story: An out-of-band edit, and a deleted document, are reported as drift

## Outcome

Somebody who edits `status:` with a text editor, or `rm`s a story, is told so by
`protocol artifact validate` — naming the document, the field and the last event that disagrees.
D-P2 and D-P4 close by detection.

## Context

D-P2 (*an out-of-band file edit is not tracked*) and D-P4 (*`rm` deletes an artifact, and nothing
prevents it*) were opened with the store and stayed open because there was nothing to compare a file
against. After G1 there is: runtime R-89 says an event records the state before and after and the
fields written, so the events of an instance say what its document should contain.

Prevention was considered and is refused here: a `PreToolUse` hook is bypassed by `Bash` (the
design's own § 3.3 says so), and a lock on a directory of markdown files is a lock somebody deletes.
A check that runs in the gate cannot be routed around.

Wave G, story 4 (`docs/plan/store-waves-f-g-h.md`). After G3.

## Acceptance

- A document whose frontmatter fields differ from what its last event's `to_state`/`changed` and the
  fold of its events say is reported as **drift**, per document, naming each field and the event id.
  **Done** — `crates/aep-backend-markdown/src/drift.rs`: `status` against the last event's
  `to_state`, `revision` against its revision, and every field any event wrote (latest value
  winning) against the document; the body is a person's prose and is not compared.
  `crates/protocol-cli/tests/drift.rs`: `a_document_edited_in_an_editor_is_drift_naming_the_field_and_the_event`.
- A document that has events and no file is reported as **deleted**, naming the last event. **Done**
  — `a_document_removed_with_rm_is_reported_as_deleted`; the journal's older *orphan* finding for
  the same artifact is said once, as a deletion.
- A document with no events at all is reported as **pre-provider**, not as drift. **Done** —
  `N document(s) predate the event log` in text, `pre_provider` in JSON; this repository's own store
  on the day this lands.
- `protocol artifact validate` exits 1 on drift or deletion and 0 on pre-provider; `--format json`
  carries the three categories separately. **Done** — `drift`, `deleted`, `pre_provider`;
  `a_document_that_matches_its_log_is_not_drift_and_a_plan_before_the_log_is_not_either`.
- Both deviations are marked closed by detection in the design, the gap register and the
  limitations page, with the refused prevention recorded as the reason. **Done.**

## Decision taken

A field no event ever wrote — a title given by `protocol artifact new` before the provider — is not
the log's to judge and is left alone; the fold is authoritative only for what it holds.

## Out of Scope

Repairing drift. A repair is a write, and a write is a command somebody issues — `protocol artifact
move` with the ladder consulted, not a verb that copies the file's claim into the log.

## Open Questions

None outstanding.
