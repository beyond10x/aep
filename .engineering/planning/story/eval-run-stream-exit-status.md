---
format: aep.planning-md/1
id: story:eval-run-stream-exit-status
kind: story
status: active
title: aep eval run --stream exits non-zero when the replay is not conformant or undecided
summary: The stream replay prints not conformant / undecided (exit 3) and exits 0; callers reading the status take a contradicted replay as green.
relations:
- serves: vision:O3
scope:
- confidence: inferred
  path: crates/edge/aep-cli/src/eval.rs
- confidence: inferred
  path: crates/edge/aep-cli/tests
revision: 4
---
# Story: `aep eval run --stream` exits non-zero when the replay is not conformant or undecided

## Context

Two independent measurements on 2026-09-03 with `protocol 0.50.0` (agentplugins adversary and
implementor, `crates/agentplugins-check/src/evals.rs`): `aep eval run --stream` over a recorded
transcript prints `not conformant: the run contradicted 2 expectation(s)` and exits **0**; with a
gating `order` row nothing decides it prints `undecided: … (exit 3)` and exits **0**. A caller that
reads the exit status, which is what the agentplugins gate did until that day, reports a
contradicted replay as a replayed transcript. The trace-report record carries the truth
(`verdict != "ok"`); the process status does not.

Reference: `crates/edge/aep-cli/src/eval.rs` (stream mode), `aep trace check` exit codes.

## Acceptance

`aep eval run --stream` exits 1 when the written `trace-report/1` has `verdict: not-conformant`
and 3 when `verdict: undecided`, matching what its own last line prints; a test drives both cases
through the binary and asserts the status and the printed line agree.

## Notes

Found during `epic:area-layout` review; the agentplugins gate now reads the report record and no
longer depends on this status, but every other caller still does.
