---
title: Review with findings
sidebar_position: 3
description: Record a review as an immutable review-result with machine-readable findings, compare two rounds, record what became of each review, and see which reviewers change things.
---

# Review with findings

This guide runs one review loop on a story: a first review, a fix, a second review, the comparison,
and the outcome record. The concepts are in [Reviews and findings](../concepts/reviews.md).

## 1. Write the findings as JSON

A reviewer, whether a person or an agent, produces a prose report and a list of findings. Keep them
apart. The prose goes in a Markdown file:

```markdown
# Review: pay by card

Adversarial pass over the guest card payment change.
```

The findings go in a JSON array (`findings-1.json`):

```json
[
  {"file": "src/checkout.rs", "line": 41, "category": "correctness", "severity": "blocker",
   "verdict": "CONFIRMED", "origin": "introduced", "message": "a declined card empties the basket"},
  {"file": "src/checkout.rs", "line": 88, "category": "tests", "severity": "warning",
   "verdict": "CONFIRMED", "origin": "introduced", "message": "no test covers a declined card"}
]
```

JSON is the form to use when a tool writes the block. A serializer quotes every value, so a message
containing `": "` or an apostrophe cannot break the block.

## 2. Record the review

```shell-session
$ aep plan artifact new review-result pay-by-card-r1 --title "Adversary pass 1" --owner adversary \
    --relate reviews:story:pay-by-card --from review.md --findings findings-1.json
created review-result:pay-by-card-r1 (active) at …/planning/review-result/pay-by-card-r1.md
```

The body now ends in a `findings` block:

````markdown
```findings
[
{"file":"src/checkout.rs","line":41,"category":"correctness","severity":"blocker","verdict":"CONFIRMED","origin":"introduced","message":"a declined card empties the basket"},
{"file":"src/checkout.rs","line":88,"category":"tests","severity":"warning","verdict":"CONFIRMED","origin":"introduced","message":"no test covers a declined card"}
]
```
````

The review cannot be edited from now on:

```shell-session
$ aep plan artifact body review-result:pay-by-card-r1 --from review.md
error: conflict: `01MEM0000000000000004` is a aep.review-result/v1, which is immutable: a record that can be edited after the fact is not evidence. Archive it, or supersede it with a new one.
```

## 3. Fix, and review again

After the fix, the second round is a second artifact:

```bash
aep plan artifact new review-result pay-by-card-r2 --title "Adversary pass 2" --owner adversary \
  --relate reviews:story:pay-by-card --from review.md --findings findings-2.json
```

## 4. Compare the rounds

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

The carried finding matched even though its line moved from 88 to 90 and its message changed case.
Use this to decide whether a third round is worth it: a round whose findings are mostly *carried* is
not converging.

## 5. Record what became of the first review

```shell-session
$ aep plan artifact evidence story:pay-by-card --kind review_outcome \
    --review review-result:pay-by-card-r1 --outcome fixed
story:pay-by-card: review_outcome recorded — review-result:pay-by-card-r1 was fixed
  on hand: test_result=1, review_outcome=1
```

The outcomes are `fixed`, `escalated` and `no-op`. A review nobody records an outcome for is
reported by `validate` once it is older than `--outcome-within` days (14 by default).

## 6. Look across reviewers

```shell-session
$ aep plan artifact review-value
reviewer   reviews  findings  no-op  fixed  escalated  cost
adversary  2        4         0      1      0          unknown
2 review(s). Counts, never a score: no ranking and no percentage — which of these lenses earns its cost is the operator's judgement, and this table is what it is made on.
```

`--since <YYYY-MM-DD>` narrows it to recent reviews.

## Gating on a review

A lifecycle cannot require a review directly, because `requires:` counts evidence kinds. What you
can do is record an `approval` or `review` evidence record against the reviewed artifact once the
review is resolved, and require that kind on the rung. See
[Gate a move on evidence](./gate-a-move-on-evidence.md).
