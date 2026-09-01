---
title: "0.15.0 — a rung may cost evidence, and the refusal says which kind of no"
description: >
  A ladder that only says what may follow what models permission, not earning. requires: names what
  a rung costs, and the refusal distinguishes "on the ladder and not yet earned" from "not on the
  ladder" — two sentences that lead to different next actions.
slug: a-rung-may-cost-evidence
tags: [release, aep, planning, evidence]
date: 2026-08-25T22:58:08+02:00
release_tag: "0.15.0"
release_commit: c00d27d3ae91207a0e4c34102ed08fe4db775ea5
---

`0.13.0` made the status ladder data. This is the first release that spends it.

A ladder that only says *what may follow what* models **permission**, not **earning**. Anybody could
move a story to `implemented`; the ladder had no opinion about whether anything was implemented.

{/* truncate */}

## What a rung costs

```yaml
requires:
  cleared:   [{ evidence: approval, at_least: 1 }]
  corrected: [{ evidence: approval, at_least: 2 }]
```

And the refusal **says which kind of no it is**, which is the whole design:

```shell-session
$ protocol artifact move outbound-claim:q3-uptime --to cleared
outbound-claim:q3-uptime is draft; cleared is on the ladder and not yet earned: reaching cleared
needs at least 1 approval record(s). Nothing was presented at $args.evidence.approval
```

*On the ladder and not yet earned* is a different sentence from *not on the ladder*, and the two
lead to different next actions. One is "go and get the approval." The other is "you are trying to do
something this kind of work does not do."

A tool that collapses both into `refused` has handed you the job of guessing which.

## Three-valued, like everything else

The condition is `True` / `False` / **`Unknown`**. A rung whose requirement cannot be observed
refuses as `Unknown`, never as `False`.

*Nobody looked* is not the same answer as *somebody looked and it failed*, and neither of them is a
pass.

## What it does not do

The shipped `story` ladder **does not** declare a cost. The mechanism exists; this repository has
not yet spent it on its own most-used kind. That is a choice you can now make in your own tree
without waiting for us.
