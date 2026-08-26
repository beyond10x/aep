---
format: aep.planning-md/1
id: story:journal-reconciliation
kind: story
status: implemented
title: A status in a file with no journal entry behind it is drift, and the store says so
summary: protocol artifact validate reports a frontmatter status the journal does not account for, and a journal entry naming an artifact the store does not hold.
relations:
- decomposes: epic:evidence-gated-completion
revision: 4
---
# Story: A status in a file with no journal entry behind it is drift

## Outcome

Somebody asking *how did this story reach `implemented`?* gets an answer or a finding — never a file
that says `implemented` with a journal that says the artifact was only ever created.

## Context

Both halves of this happened on 2026-08-26, in one day, and neither was caught:

* Six epic status moves ran through a `protocol` predating the journal. Each printed
  `moved draft -> proposed (revision 2)`; each wrote nothing. Two epics shipped at
  `status: implemented, revision: 4` with a journal holding one `created` entry apiece.
* Six journal entries in `entity-runtime` recorded the creation of *`engineering-protocols`*
  artifacts, written a minute before the same six were created correctly in that repository.
  `protocol artifact validate` reported `34 file(s) … valid` with those entries in the journal,
  because it reads the files and the files were fine.

The journal is the store's answer to *what happened*. An entry pointing at nothing, and a status
nothing accounts for, are the two ways it can be wrong, and neither was visible.

## Acceptance

`protocol artifact validate` reports, as findings rather than errors:

- an artifact whose `status` or `revision` the journal does not account for — the journal's last
  entry for it disagrees with the file;
- a journal entry naming an artifact the store does not hold;
- and it counts them, so a store with drift cannot read as a store without any.

Both are **findings**, not refusals: a store that refused to be read because its history is
incomplete would be a store nobody could repair.

## Out of Scope

Repairing drift. Reporting it is this story; a `protocol artifact reconcile` is a later one, and
writing history to match a file is exactly the thing append-only forbids doing casually.

## Open Questions

Whether a store predating the journal should report every artifact as drifted. Decides: store owner.
Default if nobody answers: **no** — an artifact with *no* journal entries at all is a store that
predates the journal, which is a known state rather than a defect; drift is a journal that
disagrees, not a journal that is absent.
