---
format: aep.planning-md/1
id: story:a-dropped-stream-does-not-end-a-run
kind: story
status: draft
title: A dropped model stream is retried, not fatal
summary: One live run died on turn 2 when the provider closed the event stream mid-turn; the loop refuses to retry once text has been emitted, so a transient drop costs the whole run.
owner: harness
tags:
- harness
relations:
- decomposes: epic:self-evaluation
revision: 1
---
# Story: a dropped stream is not a dead run

## Outcome

A run that has done twenty minutes of work survives the provider closing a connection. Today it ends,
and everything it did is thrown away.

## Context

On 2026-08-24 a live run died on turn 2 with `reading the event stream: request or response body
error` — the provider closed the stream mid-turn after a series of keepalives. The loop retries a
turn up to four times, but never once anything has been emitted, and text had been emitted. That rule
is right as far as it goes: replaying a turn whose partial output the caller has already seen would
duplicate it.

What is missing is the case where the *turn* can be abandoned and the *conversation* cannot. The items
before the failed turn are intact and cacheable; only the half-turn is lost. That attempt cost $0.01
and re-running from scratch cost a full run.

## Acceptance

- A stream that drops mid-turn does not end the run: the partial turn is discarded and retried, and
  the model is not shown two copies of anything.
- The retry is visible in the record — a warning naming the transport failure, so a run that limped is
  never mistaken for one that did not.
- A drop that repeats past the attempt ceiling still ends the run, with the transport's own words.
- One test drives a sink that fails after emitting text and asserts the conversation is unharmed.

## Out of Scope

Resuming a run from a written transcript after the process has exited. Different problem, larger.

## Open Questions

- Is a partially-emitted assistant message safe to discard on every wire we speak, or only on the
  Responses wire? Whoever picks this up answers it from the wire's own documentation, not by
  inference.
