---
format: aep.planning-md/1
id: task:ova-closed-cells
kind: task
status: implemented
title: A closed row states its guarantee and where an adopter reads the reason
summary: 'R9: Guarantee is a statement or the literal none, Reason for adopters at is a corpus path that exists or the literal none, and settled versus unsettled is decided from those two cells.'
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
# Task: a closed row states what the closure buys, and where an adopter reads why

## What

**R9.** For every row whose `Verdict` is `closed`:

- `Guarantee` states the semantics the closure buys, or the literal `none`;
- `Reason for adopters at` holds a corpus path, optionally with an anchor, where that reason is
  written **for adopters**, or the literal `none`;
- the row is **settled** when `Guarantee` is not `none` *and* `Reason for adopters at` resolves to a
  corpus file. Otherwise it is **unsettled**.

The check prints the settled/unsettled partition. `task:ova-followups` consumes it.

## Why

The story's second acceptance bullet, made decidable: the reason has to be somewhere an adopter
reads, not only in the audit's own cell. A guarantee stated in the table and nowhere else is a
guarantee the adopter finds only after they have already hit the wall.

A closed verdict is not a defect — `evidence_kinds` being closed is correct, because it is the seam
whose semantics are guaranteed. A closed verdict with nothing behind it is.

## Done When

| # | Acceptance |
|---|---|
| K1 | For every `closed` row, `Guarantee` is either the literal `none` or a non-empty statement. Empty, whitespace-only or `—` is red and names the row. |
| K2 | For every `closed` row, `Reason for adopters at` is either the literal `none` or a path — optionally `path#anchor` — that is a member of the R1 corpus and exists. |
| K3 | A `Reason for adopters at` naming a file that exists but is **not** in the corpus is red. Shown. |
| K4 | The check prints, for each closed row, `settled` or `unsettled` by R9's rule, plus the count of each. |
| K5 | Changing a settled row's `Guarantee` to `none` on a scratch copy flips that row to `unsettled` in K4's output. |
| K6 | At least one `closed` row exists, so K1–K4 are not vacuous. |
| K7 | The number of rows partitioned equals the number of `closed` verdicts the table carries — no closed row is skipped for being unparseable. |
| K8 | No check in this unit treats `closed` as failing. A table of entirely settled closed rows exits 0. |
| K9 | An **anchor** in `Reason for adopters at` resolves to a heading in the file it names, by the GitHub slug rule. A renamed heading is red. Added by the adversarial pass. |
| K10 | No closed row points its reason at the audit itself, and the settled rule refuses that path. Added by the adversarial pass. |

## What the adversarial pass found

K2 resolved the *file* half of the path and dropped the anchor. Two mutations ran green before K9
and K10 existed:

| Mutation | What the reader would have got |
|---|---|
| the heading `## Capabilities` renamed in `website/docs/reference/vocabulary.md` | the reason link landing at the top of the page, where the reason is not |
| a closed row's reason set to `docs/guide/open-vocabulary.md` | the audit citing its own cell as the place an adopter reads why |

The second matters more than it looks. `docs/guide/*.md` is corpus and this audit is a
`docs/guide/*.md`, so a self-citation *resolved* — one cell edit per row turned every unsettled
closure into a settled one and dissolved its follow-up. R9's "not only in the audit's own cell" is
now enforced by `reason_resolves`, so the partition cannot be argued into existence.

## Notes

- K8 is stated as acceptance because it is the invariant most likely to be violated by accident: a
  check that goes red on `closed` would turn the audit into pressure to open vocabularies the story
  explicitly does not ask to open.
- K3 is the difference between "the reason is written down" and "the reason is written down where an
  adopter reads". A design document under `docs/design/` is not in the corpus, and citing one is the
  failure this row catches.
- The partition is this check's output contract; `check-followups.sh` reads it rather than
  recomputing it, so the two cannot disagree about which rows are unsettled.

## Verifier

`.engineering/checks/check-closed-cells.sh`. K1–K10 are its rows.

K5 is half of acceptance criterion 3; the other half — that an unsettled row with `—` in `Follow-up`
turns a check red — belongs to `task:ova-followups`, because that is the check that owns the
`Follow-up` column.
