---
title: "0.21.0 — four mechanisms the engine and driver were missing"
description: >
  Evidence names its subject, so a green run for one story can no longer discharge another story's
  gate. An advisory tier that reports and gates nothing. A circuit breaker on step dependencies. And
  the driver writes down what a step was not allowed to do.
slug: the-engines-mechanisms
tags: [release, aep, driver, evidence]
---

Four mechanisms, each closing a gap where the rule existed and nothing enforced it.

{/* truncate */}

## Evidence names its subject

A record may declare what it is about. If the declared subject does not match the task it is
submitted against, it is refused — **before the record is constructed**, not after.

In a sentence: **a green test run for one story can no longer discharge a different story's gate.**
That was possible before, it was silent, and the audit trail would have looked correct.

Placing the guard before construction is deliberate. A refused record that got built is a refused
record that can be logged, cached, or passed on by something that did not check the verdict.

## An advisory enforcement tier

A requirement may be declared **advisory**, with an owner and an exit criterion. It is evaluated,
reported and counted — and it **gates nothing**.

This exists because the realistic alternative to a soft rule is not a hard rule; it is **prose in a
wiki**. A team can now measure how often a proposed rule *would* have fired, on real work, before
anybody's build turns red.

The owner and the exit criterion are **required at parse time**, so an advisory requirement cannot
quietly become permanent through nobody remembering to revisit it. A rule with no exit criterion is
a rule that never graduates and never dies.

## A dependency that keeps failing stops being called

A step map may declare `depends_on` and a circuit breaker. A repeatedly failing dependency **opens
the breaker** instead of being retried, and the run says the breaker is open rather than producing
the tenth identical timeout.

Retry and circuit-breaking had been prose in every step map — which is to say, enforced by whoever
read it.

## The driver writes what a step was **not** allowed to do

Refusals are emitted as their own `trace-spec/1` document, with `tool.absent` expectations. **The
absence of a tool call becomes a checkable fact rather than a silence.**

This closes a real hole in reasoning about a governed run: *"the agent did not shell out"* and
*"the transcript contains no record either way"* look identical in a log, and only one of them is a
finding.

If a run refused nothing, **no document is written at all** — an empty specification is refused by
the format itself, and a file asserting nothing is worse than no file.
