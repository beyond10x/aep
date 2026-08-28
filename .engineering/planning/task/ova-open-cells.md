---
format: aep.planning-md/1
id: task:ova-open-cells
kind: task
status: implemented
title: An open row's three trailing cells hold an em dash, not a blank
summary: 'R8: for every open row, Guarantee, Reason for adopters at and Follow-up each hold the literal em dash, so no cell is readable as either not-applicable or not-yet-filled.'
owner: protocol
tags:
- adoption
- checks
relations:
- decomposes: specification:open-vocabulary-audit
- derived_from: task:w4-2-open-vocabulary-audit
- depends_on: task:ova-checks-runner
- depends_on: task:ova-table-shape
revision: 6
---
# Task: an open row's trailing cells hold an em dash

## What

**R8.** For every row whose `Verdict` is `open`, each of `Guarantee`, `Reason for adopters at` and
`Follow-up` holds the literal `—`. Not empty, not whitespace, not `-`, not `n/a`, not `TBD`.

## Why

A blank cell must never be readable as either *not applicable* or *not filled in yet*. Those are
opposite states with the same appearance, and an adopter reading the table cannot tell which one they
are looking at. The em dash is the positive statement that the cell is settled by the verdict.

It is also what makes the closed-row checks meaningful: if an open row may leave `Guarantee` empty,
then an empty `Guarantee` cannot be evidence of anything about a closed row either.

## Done When

| # | Acceptance |
|---|---|
| O1 | For every `open` row, `Guarantee` is exactly `—`. |
| O2 | For every `open` row, `Reason for adopters at` is exactly `—`. |
| O3 | For every `open` row, `Follow-up` is exactly `—`. |
| O4 | Each of the empty cell, a whitespace-only cell, `-`, `n/a`, `none` and `TBD` is red in an open row. Shown for each of the six on a scratch copy. |
| O5 | At least one `open` row exists, so O1–O3 are not vacuously true. |
| O6 | Every violation is reported with the row's `Declaration` value and the offending column name — not as a count. |
| O7 | The check examines every open row: the number of rows inspected equals the number of `open` verdicts the table carries. |

## Notes

- `none` is red here and legal on a closed row. That asymmetry is the point: `none` on a closed row
  is a claim ("this closure buys nothing"), and on an open row it would be a category error.
- The character is U+2014 EM DASH, the one the specification's own tables use. A U+2013 en dash or a
  hyphen is red, which is worth stating because the two are indistinguishable at a glance in most
  editors.

## Verifier

`.engineering/checks/check-open-cells.sh`. O1–O7 are its rows.

O4 is the discrimination proof for this unit and runs six scratch mutations, one per rejected value.
A check that accepts any of the six is not enforcing R8, it is enforcing "somebody typed something".
