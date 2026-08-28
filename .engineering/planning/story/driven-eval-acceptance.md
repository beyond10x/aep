---
format: aep.planning-md/1
id: story:driven-eval-acceptance
kind: story
status: archived
title: One real task, driven end to end, with a denial on purpose
summary: A paid run under adp/default whose transcripts are checked and submitted as trace_conformance, including a case that trips a hook deny so permission.denied audits something.
owner: eval
tags:
- driver
- eval
relations:
- decomposes: epic:self-evaluation
- depends_on: story:plugin-enforcement-hooks
revision: 5
---
# Story: One real task, driven end to end, with a denial on purpose

## Outcome

The loop stops being a design: a real task in this repository is driven by `protocol drive` against a
real model, its transcripts are checked, and the resulting record is admitted by the engine. And the
question nobody could answer by reading — whether a hook deny shows up in the transcript's
`permission_denials` — has an observed answer.

## Context

This is the acceptance for the whole driver epic, and it runs outside `task check` for the same
reason the existing plugin eval does: it needs a `claude` binary, credentials and a network. The
denial case is not decoration. The enforcement audit reads `permission_denials`, and an ambiguous `0`
reads as *nothing was denied* when it may mean *denials are counted elsewhere* — a gap-register row
that only a run can close.

## Acceptance

- One task is driven end to end under `adp/default`, reaching a state the protocol calls complete.
- Every transcript passes `protocol trace check`, and the `trace_conformance` records are accepted by
  `protocol evaluate --evidence`.
- A case deliberately trips a `PreToolUse` deny; the transcript is read and the answer — whether
  `permission_denials` incremented — is written into the design's audit column **either way**.
- The gap-register row is closed by naming what was observed, not by disappearing.
- The run is reproducible from what is committed: the step map, the specification and the task, with
  only credentials supplied by the operator.

## Superseded, both halves — 2026-08-28

**Neither half of this story is this story's any more, and one of them is not this repository's.**

- *One real task driven end to end, transcripts checked, records admitted* is
  `story:governed-dogfood-run`, which has since been attempted twice — `W4-1/1` (2026-08-21, blocked
  in `establish_verifiers`) and `W4-2/1` (blocked in `adversarial_verify`), both recorded on
  `docs/plan/harness-wave-4-governed-dogfood.md`.
- *A case that deliberately trips a `PreToolUse` deny* went with the hooks. `epic:metaharness-migration`
  (implemented) deleted `integrations/claude-code/hooks/` and moved the agent evals, their recorded
  transcripts and the deliberate-denial case to metaharness `evals/engineering-protocols/` on
  2026-08-22. The `run.sh` this story's Context names retired with its subject; `task plugin-eval`
  and `task driven-eval` were removed on 2026-08-28 for pointing at a deleted script.

Its Open Question — *do plugin-supplied hooks need a per-invocation consent step?* — is answered by
the architecture rather than by a run: the driver ships no hooks and no settings file and is itself
the per-call decider, in-process, through `metaharness --decisions ask`.

**`story:governed-dogfood-run` still carries `depends_on: story:driven-eval-acceptance`, and that
edge is stale.** It cannot be removed: `protocol artifact` has `relate` and no `unrelate`, which
invariant 16 makes deliberate. It is recorded here instead, and it is a live example for
`story:entity-runtime-mapping`'s register row about mistyped edges.

## Out of Scope

Making this a step of `task check`. A paid run in the ordinary gate is a bill and a false red; the
bounds it establishes are checked against committed transcripts instead.

## Open Questions

If plugin-supplied hooks turn out to require a per-invocation consent step, this run is where it
shows. Decides: driver owner, on what the run does — and the hook layer degrades to advisory if it
goes the wrong way, which is named in the design rather than assumed.
