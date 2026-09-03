---
format: aep.planning-md/1
id: story:cold-start-outlives-the-deadline
kind: story
status: draft
title: A driven arm on an autoscaled endpoint holds its request instead of paying for the boot every step
relations:
- decomposes: epic:cross-harness-portability
scope:
- confidence: inferred
  path: crates/drive/aep-driver/src/run.rs
- confidence: inferred
  path: crates/edge/aep-cli/src/drive.rs
- confidence: inferred
  path: crates/observe/trace-spec/src/report.rs
revision: 7
---
# Story: A driven arm on an autoscaled endpoint holds its request instead of paying for the boot every step

## Outcome

A driven run against a gateway that starts a GPU on demand pays that start **once**, not once per
step — and an operator watching it can tell *the model is being started for you* from *the peer has
gone quiet*, because the run says which.

## Context

**Two correct decisions that compose into a loop nobody wrote.** Observed 2026-08-29 while preparing
the first driven run of the b10x arm against `b10x-llmgw`.

- `harness-responses` sets `DEFAULT_OPERATION_TIMEOUT = 180s` per read, with a good reason on the
  constant: *"a streamed turn may legitimately run for minutes; what must not happen is a peer that
  accepts the connection and then says nothing, which would hold the loop open with no way out."*
- `llmgw` starts a RunPod instance on the first request for a model that is not running, holds the
  request while vLLM loads, and reaps a pod it considers idle. Its own config allows
  `start_wait_seconds = 1800`.

A cold boot is exactly the shape the timeout exists to refuse: the peer accepts the connection and
says nothing, for minutes. So the loop gives up at 180 s; the gateway then has no in-flight work and
reaps the pod; the next step's first call starts another boot. **Every step pays a cold start and no
step ever gets a token.**

It was found by reproducing it by hand, badly: probes with `curl -m 8`, `-m 15`, `-m 20`, `-m 90`
each cancelled their request, and the gateway's log shows the consequence rather than the cause —

```
00:18:41  created a pod   xkn2oyhfwfrkle
00:24:49  reaped an idle pod  xkn2oyhfwfrkle
00:27:07  created a pod   ii176z76wsvisa
```

— three pods in ten minutes, each killed by the impatience of the thing waiting for it. A single
request held open with no client deadline was enough to keep one alive.

**Why this is ours and not the gateway's.** The endpoint is doing what it says: it starts a model
and holds the caller. What is missing is on our side — a driven run has no way to say *this endpoint
is autoscaled, wait for it* and no way to distinguish, in its own record, a peer that is silent from
a peer that is starting.

## Acceptance

- A driven run against an autoscaled endpoint **completes its first step** without the endpoint
  being reaped and restarted underneath it. Asserted against a gateway that holds the first request,
  not against a mock — a mock that answers immediately cannot fail this.
- The wait is **declared rather than inherited**: a run says how long it will hold a first call, and
  the number is in the run's launch record with everything else. A default that is silently longer
  would trade this defect for a run that hangs on a peer that really has gone away.
- The run's own record distinguishes **waiting for a cold start** from **a silent peer**. Today both
  are a `NO_TERMINAL_RECORD` warning carrying a transport error, and they are different findings: one
  is worth waiting through and the other is worth stopping for.
- The step's retry budget is not spent on a cold start. Three attempts at a boot are three boots.
- A pre-flight warms the endpoint before the run owns a run id and a lock, or the run's first step
  is knowingly the one that pays. Either is acceptable; being unaware is not.

## Out of Scope

- Changing `harness-responses`' 180-second read timeout for the general case. It is right for what
  it was written for, and this story is about a case it was not written for.
- Making `llmgw` keep a pod warm. Whether an idle GPU is worth paying for is the endpoint owner's
  decision, not a driven run's.
- Any other harness. The Claude Code arm reaches a vendor endpoint that is always warm; this is
  specific to a driven arm on an endpoint somebody scales.

## Open Questions

**Does the run wait, or warm and then run?** Decides: driver owner. Default if nobody answers:
**warm first, in the pre-flight** — it costs one request, it happens before a run id and a lock
exist, and it turns a run that dies three states in into a run that starts a minute later.

**How long is too long?** Decides: whoever holds the eval budget. Default: the endpoint's own
`start_wait_seconds` when it publishes one, and otherwise a declared number rather than an inherited
one — an unbounded wait is a run holding a session open for an unbounded time, which the driver
already refuses elsewhere.
