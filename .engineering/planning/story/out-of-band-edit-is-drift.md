---
format: aep.planning-md/1
id: story:out-of-band-edit-is-drift
kind: story
status: draft
title: An out-of-band edit, and a deleted document, are reported as drift
summary: protocol artifact validate compares each document with its last event and reports drift, deletion or pre-provider; D-P2 and D-P4 close by detection, prevention refused on the record.
owner: store
tags:
- backend
- store
relations:
- decomposes: epic:planning-store-as-backend
- depends_on: story:history-from-the-event-log
revision: 4
---
# Story: An out-of-band edit, and a deleted document, are reported as drift

## Outcome

Somebody who edits `status:` with a text editor, or `rm`s a story, is told so by
`protocol artifact validate` — naming the document, the field and the last event that disagrees.
D-P2 and D-P4 close by detection.

## Context

D-P2 (*an out-of-band file edit is not tracked*) and D-P4 (*`rm` deletes an artifact, and nothing
prevents it*) were opened with the store and are still open
(`harness-planning-and-driver-design-v0.1.md:302,320`). Until G1 there was nothing to compare a
file against. Now there is: runtime R-89 says an event records the state before and after and the
fields written, so the last event of an instance says what its document should contain.

Prevention was considered and is refused here: a `PreToolUse` hook is bypassed by `Bash`
(the design's own § 3.3 says so), and a lock on a directory of markdown files is a lock somebody
deletes. A check that runs in the gate cannot be routed around.

Wave G, story 4 (`docs/plan/store-waves-f-g-h.md`). After G3.

## Acceptance

- A document whose frontmatter fields differ from what its last event's `to_state`/`changed` and the
  fold of its events say is reported as **drift**, per document, naming each field and the event id.
- A document that has events and no file is reported as **deleted**, naming the last event.
- A document with no events at all is reported as **pre-provider**, not as drift (this repository's
  own store on the day this lands).
- `protocol artifact validate` exits 1 on drift or deletion and 0 on pre-provider; `--format json`
  carries the three categories separately.
- Both deviations are marked closed by detection in the design, the gap register and the
  limitations page, with the refused prevention recorded as the reason.

## Out of Scope

Repairing drift. A repair is a write, and a write is a command somebody issues — `protocol artifact
move` with the ladder consulted, not a verb that copies the file's claim into the log.

## Open Questions

None outstanding.
