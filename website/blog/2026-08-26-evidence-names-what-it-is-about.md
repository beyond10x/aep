---
title: "0.20.0 — evidence you record, and provenance for a decision"
description: >
  protocol artifact evidence records an observation against an artifact so a later move can be
  decided on it. And a move now separates evidence that was recorded from evidence that was
  asserted — saying so at the moment it leans on the weaker one.
slug: evidence-names-what-it-is-about
tags: [release, aep, planning, evidence]
---

`0.15.0` let a rung cost evidence. This release is how the evidence gets there.

```console
$ protocol artifact evidence outbound-claim:q3-uptime \
    --kind approval --source "legal review" --ref https://example.invalid/approvals/814
outbound-claim:q3-uptime: approval recorded from legal review
  on hand: approval=1

$ protocol artifact move outbound-claim:q3-uptime --to cleared
outbound-claim:q3-uptime moved draft -> cleared (revision 2)
```

{/* truncate */}

## The record carries when somebody looked

`--at` defaults to now, **read at the edge** and written into the record — so the observation
carries when somebody *looked*, not when the file was parsed. That is the same `observed_at` rule
the engine has used since `0.10.0`, now available from the planning side.

`--source` and `--ref` are how a record stops being an assertion: *where it came from* and *where to
go and look*.

## Provenance: recorded versus asserted

A move now records the **provenance** of what decided it, separating evidence that was **recorded**
from evidence that was **asserted** at the moment of the move — and **a move leaning on an assertion
says so as it happens**.

The consequence is about a specific future moment: somebody auditing, months later, why a gate
opened. Without this, both cases look identical in the trail. With it, the weaker claim is visible
**at the point of decision** — where somebody could still have objected — rather than discoverable
afterwards, when the only options are accept or unwind.
