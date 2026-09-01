---
title: "0.23.0 — a claim that left the boundary"
description: >
  Every other ladder here models evidence flowing inward. An outbound claim runs the other way — a
  number already in a customer's inbox — and sending is not undoable. Added as a YAML file with no
  Rust change at all, which is what the whole entity-runtime programme was for.
slug: a-claim-that-left-the-boundary
tags: [release, aep, planning]
date: 2026-08-26T00:49:54+02:00
release_tag: "0.23.0"
release_commit: bba1a15eeab9f653d6150c9ee568b256b9945801
---

Every ladder in this repository models evidence flowing **inward**: something was observed, and an
artifact's status records what we now know.

An outbound claim runs the other way. A number in a customer's inbox. A status page saying
"resolved". An availability figure in a renewal deck. Those are assertions that **already left the
boundary**, and nothing here modelled them.

{/* truncate */}

## The ladder

```shell-session
$ protocol artifact lifecycle outbound-claim
outbound-claim starts at draft
  cleared -> sent
  corrected -> nothing
  correction-owed -> corrected
  draft -> cleared
  sent -> correction-owed, standing
  standing -> correction-owed
```

**Sending is not undoable, and the ladder says so:**

```shell-session
$ protocol artifact move outbound-claim:q3-uptime --to sent
outbound-claim:q3-uptime moved cleared -> sent (revision 3)

$ protocol artifact move outbound-claim:q3-uptime --to draft
outbound-claim:q3-uptime is sent; an outbound-claim may move to: correction-owed, standing
```

There is no path from `sent` back to `draft`. A ladder that allowed one would model retraction as an
**edit** — the claim would simply stop having been made — and the customer would still have the
email.

So a wrong claim moves **forward**, never back. `correction-owed`, then `corrected`, which is a *new
outbound act* with its own evidence, costing **two** approvals where the original cost one. **Fixing
a claim costs a second claim.** That is true in the world, and the ladder should not be kinder than
the world.

## Why `correction-owed` is a rung and not a flag

The rung names the state precisely: **sent, known wrong, audience not yet told.**

It is the most expensive state an organisation can be in and the easiest one to leave undocumented,
because nobody wants to write it down. A rung is a column `protocol artifact board` prints; a flag
is a field nobody queries.

It is deliberately **not terminal** and deliberately **not counted as approved** — a claim in
`correction-owed` is a live obligation, and anything treating it as finished would let the most
important case disappear from a report.

There is no `retracted`. A retraction is itself an outbound claim — **you have to tell somebody** —
so it is a `corrected` whose content is a withdrawal, not a rung that quietly makes the original
stop counting.

## The part that is actually the headline

**A whole new kind of artifact, with six rungs and two evidence requirements, arrived as one YAML
file and no Rust change at all.**

That is the sentence the entity-runtime programme was for. Ten releases earlier it would have been a
pull request against this repository, a review, a release, and an upgrade for everybody. Now it is a
file you write in your own tree.
