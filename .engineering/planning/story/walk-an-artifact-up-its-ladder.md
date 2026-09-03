---
format: aep.planning-md/1
id: story:walk-an-artifact-up-its-ladder
kind: story
status: draft
title: One command walks the rungs, and reports what each one cost
summary: Reaching a rung two moves away means two hand-written moves, and the alternative a caller reaches for is a jump the ladder should refuse.
scope:
- confidence: cited
  path: crates/edge/protocol-cli
- confidence: inferred
  path: website/docs/reference/cli.md
revision: 5
---
## Context

A migration knows where an artifact ends up — a bug analysis that says it landed is `implemented` —
and it has to get there one `move` at a time, because `move` takes a single `--to` and the ladder
allows one rung per call.

The `sbf/acd` migration walked `draft → proposed → active` by hand for two stories. Nothing is wrong
with the refusal that makes it necessary: guardrail 4 in the `planning` skill is right that jumping
rungs hides which rung's evidence was never paid. What is missing is the verb that walks the rungs
*and says what each one cost*, so a caller does not have to choose between two hand-written moves
and a wrong one.

## Acceptance

- One command walks an artifact from where it is to a named rung, one legal move at a time.
- It prints each move it makes, in order, so the record reads the same as the hand-walked one.
- It stops at the first rung whose `requires:` the store cannot satisfy, and the refusal names that
  rung, the requirement, and what was presented — the message `move` already produces.
- It refuses a target that is not reachable from the current status at all, rather than walking part
  way and leaving the artifact somewhere nobody asked for.
- It is not a bulk verb: it operates on one artifact, so § 4 of the `planning` skill still holds
  that a bulk move is never autonomous.

## Evidence for the gap

`crates/edge/protocol-cli/src/planning.rs` — `move` takes one `--to` and validates it against the
lifecycle. On `sbf/acd`, `story:presence-sync-race` needed two invocations to reach `active`, and
the third to `implemented` was refused for want of a `test_result`.
