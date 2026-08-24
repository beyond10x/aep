---
format: aep.planning-md/1
id: task:ova-table-shape
kind: task
status: draft
title: One table, the header the checks parse by, two verdict values, a row floor
summary: 'R3, R4 and R11: exactly one table with the seven named columns in order, Verdict holding only open or closed, and a row count that cannot be satisfied by an empty table.'
owner: protocol
tags:
- adoption
- checks
relations:
- decomposes: specification:open-vocabulary-audit
- derived_from: task:w4-2-open-vocabulary-audit
- depends_on: task:ova-checks-runner
- depends_on: task:ova-audit-corpus
- depends_on: task:ova-scan-declarations
revision: 4
---
# Task: one table, one header, two verdicts, a floor

## What

**R3, R4, R11.** The audit contains **exactly one** markdown table, whose header row is, in this
order:

| Declaration | Invited at | Verdict | Decided by | Guarantee | Reason for adopters at | Follow-up |

`Verdict` holds exactly one of `open` or `closed`. There is no third value and no hedge. **Open**
means: an adopter can introduce a new value without modifying a file under `crates/` in this
repository. Everything else is `closed`.

And the floor: at least one `open` row, at least one `closed` row, and a row count of at least the
number of candidates `scan-declarations.sh` emits.

## Why

Every sibling check parses this table by its header. A second table in the file, a renamed column or
a seventh-and-a-half cell breaks them all at once, so the shape is decided in one place. The floor
exists because every quantified requirement in the specification is trivially true of the empty
table — without it the suite can be made green by deleting rows.

## Done When

| # | Acceptance |
|---|---|
| T1 | The audit contains exactly one markdown table. Two or more is red, with the line number of each named. |
| T2 | Its header row is the seven column names above, in that order, compared cell for cell after trimming. A renamed or reordered column is red and names the mismatch. |
| T3 | Every data row has exactly seven cells. A row with six or eight is red and named by line. |
| T4 | Every `Verdict` cell is exactly `open` or `closed`. Empty, `partial`, `mostly open`, `open*` or any other value is red and names the row. |
| T5 | At least one row has `open` and at least one has `closed`. |
| T6 | The data row count is at least the line count of `bash .engineering/checks/scan-declarations.sh`. |
| T7 | Emptying the table on a scratch copy turns T5 and T6 red — shown, so the quantified rows are demonstrably non-vacuous. |
| T8 | The parsing helper the sibling checks use is defined once, beside this check, and each sibling reads the table through it rather than re-implementing the split. |

## Notes

- T8 is the reason this task exists separately rather than being folded into each row-level check:
  seven checks parsing a markdown table seven ways is seven chances to disagree about what a cell is.
- Cell values are compared after trimming surrounding spaces and nothing else. Escaped pipes inside a
  cell are out of scope: no cell in this table needs one, and a check that handles them silently
  invites one to be written.
- The floor in T6 is a floor, not an equality. Rows found by reading the corpus have no scan
  candidate behind them, so the table is expected to be longer than the scan's output.

## Verifier

`.engineering/checks/check-table-shape.sh`. T1–T8 are its rows.

T7 runs against a copy under `${TMPDIR:-$HOME/.cache/claude-tmp}` with the data rows deleted, and
requires the check to exit non-zero on that copy — a green result there is a failed row here.
