---
format: aep.planning-md/1
id: task:ova-scan-loop
kind: task
status: draft
title: 'Completeness and provenance: the two checks that close the loop on the scan'
summary: 'R13: every scan candidate has a row, every non-scanned row carries a resolving corpus citation, and the audit states in its own words what the scan cannot find.'
owner: protocol
tags:
- adoption
- checks
relations:
- decomposes: specification:open-vocabulary-audit
- derived_from: task:w4-2-open-vocabulary-audit
- depends_on: task:ova-checks-runner
- depends_on: task:ova-scan-declarations
- depends_on: task:ova-citations
revision: 4
---
# Task: completeness and provenance, and the limit stated out loud

## What

**R13.** The two checks that close the loop between the scan and the table:

| | Rule | What it catches |
|---|---|---|
| Completeness | every candidate `scan-declarations.sh` emits has a row in the table | a document-declared vocabulary the audit forgot |
| Provenance | every row the scan does **not** emit satisfies R6 — a corpus citation that resolves | a row invented rather than found |

Plus the sentence that keeps the pair honest: the scan **cannot** discover a closed surface, because
a closed surface is precisely one with no document key to find. The audit says that in its own
section. A reader who takes the completeness check for proof of completeness has been misled by it.

## Why

Without completeness, the table is whatever somebody happened to notice. Without provenance, it is
whatever somebody happened to invent. The two rules together mean every row came from one of exactly
two places, and each place has its own evidence.

The stated limit is not a caveat; it is the load-bearing part. A completeness check named
"completeness" that is complete only over the open half is the same failure the audit exists to
name — a claim that reads stronger than what is behind it.

## Done When

| # | Acceptance |
|---|---|
| P1 | Every candidate `scan-declarations.sh` emits has a row whose `Declaration` or `Decided by` names it. A candidate with no row is red and named. (Acceptance criterion 2.) |
| P2 | Deleting such a row on a scratch copy turns P1 red, naming the orphaned candidate. |
| P3 | Every row the scan does **not** emit carries an `Invited at` satisfying R6 — path in the corpus, fragment present in that file. |
| P4 | The two checks print the partition: how many rows are scan-backed and how many reading-backed, and the two sum to the table's data row count. |
| P5 | The audit carries a named section stating that the scan cannot discover a closed surface, and that the completeness check is therefore not a proof of completeness. |
| P6 | Neither check is vacuous: P1 is red if the scan emits zero candidates, and P3 is red if the reading-backed partition is empty. Both shown. |
| P7 | Every candidate is accounted for exactly once — a candidate matched by two rows is reported, since it usually means one row is a duplicate rather than a layer. |

## Notes

- P7 has one legitimate exception, the layered case of R5, where a single candidate legitimately
  backs two rows with different verdicts. The check reports the multiplicity and passes when the two
  rows differ in verdict; two rows with the same verdict is red.
- The matching rule between a candidate and a row is decided here and stated in the check's header,
  because a loose match makes completeness meaningless and an exact one makes it brittle. Match on
  the candidate string appearing in `Decided by`, falling back to `Declaration`.
- The specification's open question about failing on candidates that are *new since the last round*
  is answered at its default: no. A new candidate with no row already fails P1, and a separate check
  would need a stored baseline this run has nowhere to keep.

## Verifier

`.engineering/checks/check-completeness.sh` and `.engineering/checks/check-provenance.sh`. P1, P2,
P6 and P7 are the first's rows; P3, P4 and P6 are the second's. P5 is asserted by whichever runs
first and named in both.

Two scripts, one task, because R13 states them as a pair and neither is meaningful without the
other: completeness alone can be satisfied by inventing rows, provenance alone by deleting them.
