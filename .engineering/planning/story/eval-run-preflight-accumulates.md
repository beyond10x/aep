---
format: aep.planning-md/1
id: story:eval-run-preflight-accumulates
kind: story
status: implemented
title: aep eval run preflight reports every refusal, not the first
summary: preflight_child_path returns the first refusal so EVAL-RUN-017 masks EVAL-RUN-018; accumulate and name both.
relations:
- serves: vision:O3
- depends_on: story:eval-run-stream-exit-status
scope:
- confidence: inferred
  path: crates/edge/aep-cli/src/eval.rs
revision: 5
---
# Story: `aep eval run` preflight reports every refusal, not the first

## Context

`preflight_child_path` (`crates/edge/aep-cli/src/eval.rs`, around line 3567) returns the first
refusal it finds, so `EVAL-RUN-017` (stale child `aep`) masks `EVAL-RUN-018` (no `ess` on the
child's PATH), and an operator with both faults pays two live round trips to learn both.
`declared_plugins` 400 lines above accumulates its refusals and cites AGENTS.md invariant 3
(validation accumulates) for doing so. Found by the round-2 adversary on 2026-09-04.

## Acceptance

A case that trips both EVAL-RUN-017 and EVAL-RUN-018 is refused once with both codes named, one
line each, before anything is spawned; a test drives the preflight with both faults present and
asserts both codes appear.

## Notes

Same file as `story:eval-run-stream-exit-status`; implemented together as one unit.
