---
title: "0.18.0 — the typed blocker, and a vocabulary that says why it is closed"
description: >
  Two rungs, and the value is entirely in the type: a blocker is typed by what would clear it, so
  "blocked" stops being a label on a card. Plus an audit of every adopter-facing vocabulary — open
  or closed, and what each closure buys.
slug: the-typed-blocker
tags: [release, aep, planning]
date: 2026-08-25T23:48:17+02:00
release_tag: "0.18.0"
release_commit: 6409587e6cec97e300e53566acc00c3e2393d2ca
---

```shell-session
$ protocol artifact lifecycle blocker
blocker starts at open
  cleared -> nothing
  open -> cleared
```

Two rungs. The value is entirely in the **type**: a blocker is **typed by what would clear it**, so
*blocked* stops being a label on a card and becomes a statement with a named exit.

{/* truncate */}

## The other half: an audit of what is closed

The same release published an audit of **every adopter-facing vocabulary** — which are open, which
are closed, and **what each closure buys**.

Opening a vocabulary is not automatically the better answer. `evidence_kinds` stays **closed on
purpose**: an open evidence vocabulary would let a caller invent the kind of proof a gate is asking
for. That is not extensibility, it is a hole.

The output of the audit is not *open everything*. It is that **no vocabulary is closed by accident,
and the reason is written where an adopter reads it**.

## Why that is a deliverable

The failure mode it prevents is specific and common. An adopter hits a closed vocabulary, cannot
tell whether the closure is a decision or an oversight, and either works around it — building
something on a shape that was never intended — or files an issue and waits.

A table saying *closed, and here is what that buys you* turns both of those into a one-minute read.
