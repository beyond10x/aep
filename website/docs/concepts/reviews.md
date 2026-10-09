---
title: Reviews and findings
sidebar_position: 6
description: A review is an immutable review-result artifact whose findings are data. What became of it is a review_outcome record, and two rounds can be compared.
---

# Reviews and findings

A review in AEP is an artifact of kind `review-result`, joined to what it reviewed by a `reviews`
edge. It has three properties the rest of the store relies on:

1. **It is immutable.** Its lifecycle is `active → archived` and nothing else. `body` and every
   other edit are refused, so a review cannot be improved after the fact to make a gate pass. A
   second look is a second `review-result`.
2. **Its findings are data.** The body may carry one fenced `findings` block: a list of entries
   that `new` validates on the way in.
3. **What became of it is recorded separately.** A `review_outcome` evidence record on the reviewed
   artifact says whether the review was `fixed`, `escalated` or a `no-op`.

## A finding

| Field | Required | Values |
|---|---|---|
| `file` | yes | the path the finding is about |
| `line` | no | a line number |
| `category` | yes | free text, such as `correctness` or `tests` |
| `severity` | yes | `blocker`, `warning`, `note` |
| `verdict` | no | an adversary's `CONFIRMED`, `NEEDS-CHANGE`, `INFEASIBLE`, or a critic's `approve`, `needs-revision` |
| `origin` | no | `introduced`, `pre-existing`, `undecided` (the default) |
| `message` | yes | one sentence |

Because the body is immutable, the findings have to arrive with the review. Pass them as a JSON
array with `--findings`, and `new` writes them into the body as a JSON `findings` block:

```shell-session
$ aep plan artifact new review-result pay-by-card-r1 --title "Adversary pass 1" --owner adversary \
    --relate reviews:story:pay-by-card --from review.md --findings findings-1.json
created review-result:pay-by-card-r1 (active) at …/planning/review-result/pay-by-card-r1.md
```

A malformed block is refused. The refusal names the body line and quotes it. A body that has its own
`findings` block and is also given `--findings` is refused as ambiguous. A review that found nothing
records `[]`, and that is different from a review with no block at all. `validate` reports the
second as a review whose findings are prose only.

## Requiring a findings block

A project opts in by naming the day from which every review must carry a block:

```yaml
findings_required_since: 2026-10-01   # in .engineering/project.yaml, read as midnight UTC
```

Once the key is set, whatever the date, even one still to come, `new review-result` refuses a body
with no `findings` block. It names the two ways forward: give the findings (a block, `[]` for a
review that found nothing, or `--findings`), or record why there are none with `--prose-only
<reason>`. The reason is written into the review's front matter as `prose_only`, and `show`
returns it. The flag works in every store, and it is refused on any other kind, beside a body
that carries a block, and with a blank reason. A key written with no value (empty, `~` or `null`)
is refused naming the key; it is not read as no opt-in.

The date is what `validate` reads. It counts a review with no block as a problem, unless one of
three exemptions holds. It checks them in this order, and lists each exempt review with the
exemption that applies:

| Standing | When |
|---|---|
| `exempt_prose_only` | it was recorded with `--prose-only`; the reason is listed |
| `exempt_superseded` | a review that carries a block names it with `supersedes` |
| `exempt_before_opt_in` | the store recorded it before midnight UTC of the date: in a Git-native store, the commit that added its file; in an SQLite or Postgres store, its creation entry. One in a Git work tree but not yet committed is being recorded now |
| `missing` | none of these: a problem, exit `1` |

A review the store cannot date is undated, and undated is never before the date. In a shallow
clone, a review the boundary commit holds looks added by that commit, so it is left undated rather
than dated by the clone; a store outside a Git work tree, such as a `git archive` export, dates no
review. Such a review is `missing` unless another exemption holds, and its problem says why it is
undated and what dates it: the full history (`fetch-depth: 0` for `actions/checkout`), or running
`validate` in the repository. An undated review is not reported as overdue for an outcome either.

An existing review is never edited. To settle one, record a new review with its findings and
`--relate supersedes:<old review>`; outcomes are still recorded with `evidence --kind
review_outcome`. Without `findings_required_since`, nothing changes: a review with no block is
listed as `not_required`, not counted, and `--strict` refuses it. With the key set, `--strict`
refuses no exempt review.

## Comparing two rounds

`findings` compares the two most recent reviews of an artifact, or the two you name with `--from`
and `--to`:

```shell-session
$ aep plan artifact findings story:pay-by-card
story:pay-by-card: review-result:pay-by-card-r1 (adversary) -> review-result:pay-by-card-r2 (adversary)
carried 1:
  - src/checkout.rs:90  warning  tests  No test covers a declined card  (was line 88)
new 1:
  - src/receipt.rs:12  note  correctness  the receipt omits the currency
resolved 1:
  - src/checkout.rs:41  blocker  correctness  a declined card empties the basket
```

Two findings are the same finding when their file and category match, their messages match after
lowercasing and collapsing whitespace, and their lines are within three of each other. The reviewer
is printed but never matched on. If two reviewers find one defect, that is one defect, not new work.
`findings` always exits `0`.

## Recording what happened next

```shell-session
$ aep plan artifact evidence story:pay-by-card --kind review_outcome \
    --review review-result:pay-by-card-r1 --outcome fixed
story:pay-by-card: review_outcome recorded — review-result:pay-by-card-r1 was fixed
  on hand: test_result=1, review_outcome=1
```

The record is written on the **reviewed** artifact, because the review itself cannot change. It is
refused when the named review has no `reviews` edge to that artifact. `unrelate` refuses to remove a
`reviews` edge that a recorded outcome relies on.

`validate` reports any review older than `--outcome-within` days (14 by default) that no
`review_outcome` names. It reports this without failing, because a review nobody has acted on yet is
outstanding work, not a broken store. `--strict` fails on it.

## Is a lens worth its cost?

```shell-session
$ aep plan artifact review-value
reviewer   reviews  findings  no-op  fixed  escalated  cost
adversary  2        4         0      1      0          unknown
2 review(s). Counts, never a score: no ranking and no percentage — which of these lenses earns its cost is the operator's judgement, and this table is what it is made on.
```

One row per reviewer: the review's `owner`, or a `reviewer:` key in its document. The cost comes from
run manifests named by a review's `--ref` values. A cost nobody recorded prints as `unknown`, never
as `0`.

See [Review with findings](../guides/review-with-findings.md) for the full loop.
