---
title: "0.16.0 — a rung may open on a date, and the clock is read at the edge"
description: >
  Date and duration operators join the condition language, so a rung can be shut until a date
  arrives. The kernel still has no clock: the instant is read at the edge and passed in, which is
  what makes a recorded decision re-checkable by somebody who was not there.
slug: a-rung-may-open-on-a-date
tags: [release, aep, planning]
---

Some rungs should not open yet. A review window. A cooling-off period. A notice period somebody
agreed to in a contract.

This release adds **date and duration operators** to the condition language, so a ladder can say so.

{/* truncate */}

## The clock stays outside

The interesting part is where the time comes from. The kernel deciding the move **has no clock at
all** — reading one is banned by a scan over its own sources. The instant is read **at the edge** and
handed in as an argument.

That is not fastidiousness. It is what makes the same inputs give the same verdict **forever**, so a
decision recorded today can be re-checked next year by somebody who was not there, and the answer
does not depend on when they asked.

A kernel that read its own clock would give different verdicts at different moments for identical
inputs, and every recorded decision would become unfalsifiable.

## Growing a language one operator at a time

The condition language is data, and the standing rule is that it grows **operator by operator, never
into a language**. Two comparison operators over instants is a step; an expression evaluator would
be a different product.

## The part that had to be got right

Date **parsing** now lives inside a decision path, which means it has to be **total**. A malformed
date is `False` — not a panic, not a default, not "probably fine". Anything that can be reached by a
gate has to have an answer for every input, including the input nobody expected.
