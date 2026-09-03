---
format: aep.planning-md/1
id: story:trace-evidence-provenance-command
kind: story
status: draft
title: trace evidence records the binary that was invoked
summary: provenance.command is the literal 'protocol trace evidence' whatever binary ran; decide whether evidence names the invoked binary or the canonical spelling.
relations:
- serves: vision:O2
scope:
- confidence: inferred
  path: crates/edge/aep-cli/src/trace.rs
revision: 2
---
# Story: `trace evidence` records the binary that was invoked

## Context

`crates/edge/aep-cli/src/trace.rs:403` writes `provenance.command` as the literal
`protocol trace evidence …` regardless of whether `aep` or `protocol` ran. The guide
`website/docs/guides/check-a-transcript.md` now quotes that output as printed. Found by the
round-2 adversary on 2026-09-04. Changing the literal changes committed evidence bytes, so it is a
decision, not a drive-by edit: either record `current_exe`'s file name, or keep `protocol` as the
canonical provenance spelling and say so in the evidence design.

## Acceptance

An evidence record written by `aep trace evidence` names the command that produced it in a way the
evidence design documents, and a test asserts it for both binary names.

## Notes

Related: `story:eval-run-stream-exit-status` (another `aep`/`protocol` surface found in the same
review).
