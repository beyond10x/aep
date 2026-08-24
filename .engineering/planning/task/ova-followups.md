---
format: aep.planning-md/1
id: task:ova-followups
kind: task
status: draft
title: Every unsettled closed row names a follow-up artifact that resolves
summary: 'R10: each unsettled closed row carries a story or architecture-decision-record id that protocol artifact list resolves, a settled one carries an em dash, and the named artifacts are created in this run.'
owner: protocol
tags:
- adoption
- checks
relations:
- decomposes: specification:open-vocabulary-audit
- derived_from: task:w4-2-open-vocabulary-audit
- depends_on: task:ova-checks-runner
- depends_on: task:ova-closed-cells
revision: 3
---
# Task: every unsettled closure names an artifact that resolves

## What

**R10.** Every **unsettled** closed row names a planning artifact id in `Follow-up` — a `story:` or
an `architecture-decision-record:` recording that the vocabulary stays closed. A settled row holds
`—`. Each named id resolves in `protocol artifact list --format json`.

The follow-up artifacts are **created in this run**, in the planning store, in their initial status.
The work they name is not done in this run.

## Why

The story's third bullet: *a closed vocabulary with no stated reason does not survive the audit
unremarked*. Without this, an unsettled row is a note in a table nobody is accountable for; with it,
every unexplained closure has an id, an owner and a place in the backlog.

The distinction matters more than it looks: the audit's job is not to open anything. It is to make
sure that every closure is either explained or tracked.

## Done When

| # | Acceptance |
|---|---|
| F1 | Every unsettled closed row's `Follow-up` cell holds an id matching `story:<slug>` or `architecture-decision-record:<slug>`. Empty, `—`, `TBD` or a prose sentence is red and names the row. |
| F2 | Each such id appears in `protocol artifact list --format json`. |
| F3 | Every **settled** closed row's `Follow-up` is exactly `—`. |
| F4 | Pointing a `Follow-up` cell at an id that is not in the store turns F2 red, naming the id. (Acceptance criterion 4.) |
| F5 | Each named artifact's body quotes the `Declaration` cell of the row that produced it, verbatim, so the artifact and the row can be joined without the table. |
| F6 | Each named artifact carries a relation to `story:open-vocabulary-audit`, asserted from `protocol artifact list --format json` rather than by reading the file. |
| F7 | Each named artifact is in its kind's **initial** status — none was moved. The work they name is out of scope here. |
| F8 | The unsettled partition is read from `check-closed-cells.sh`'s output, not recomputed, so the two checks cannot disagree about which rows need a follow-up. |
| F9 | The check inspects every closed row: rows examined equals closed rows in the table. |

## Notes

- F7 is the boundary with the rest of the backlog. Creating the artifact is this task; answering the
  question it holds open is each artifact's own future work, and moving one through its lifecycle is
  the operator's call, not this run's.
- `architecture-decision-record` is the right kind when the answer is already known and is *it stays
  closed*; `story` is the right kind when the migration question is still open. The audit's row picks
  one; this check does not judge which.
- F2 uses the CLI, not a directory listing: an id that resolves to a misfiled artifact is not
  resolved, and only the CLI knows that.

## Verifier

`.engineering/checks/check-followups.sh`. F1–F9 are its rows.

F4 runs against a scratch copy of the audit with one cell repointed at an id the store does not hold.
This is one of the four mutations `task:ova-mutation-proof` audits; running it here too means the
check can show its own discrimination without that task existing yet.
