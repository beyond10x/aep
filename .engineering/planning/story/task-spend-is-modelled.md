---
format: aep.planning-md/3
id: story:task-spend-is-modelled
kind: story
status: implemented
title: The per-task spend record is modelled in ESS
relations:
- decomposes: epic:deterministic-delivery-workflows
- serves: vision:O3
revision: 5
transitions:
- {from: "draft", to: "proposed", at: "2026-10-14T07:23:46Z", actor: "human:timo", revision: 3, decided_on: {"recorded":{"review_outcome":1}}}
- {from: "proposed", to: "active", at: "2026-10-14T07:23:46Z", actor: "human:timo", revision: 4, decided_on: {"recorded":{"review_outcome":1}}}
- {from: "active", to: "implemented", at: "2026-10-14T18:52:09Z", actor: "human:timo", revision: 5, decided_on: {"recorded":{"test_result":1,"review_outcome":1}}}
---
# Story: The per-task spend record is modelled in ESS

## Outcome

`ess/domains/evidence.yaml` declares the spend record before any code reads or writes it. The record
holds these figures:

- model calls;
- input tokens, output tokens, cache-read tokens, cache-write tokens and thinking tokens;
- cost in micro-USD;
- wall time;
- for a driven run, the llm steps declared by the map and the llm steps run.

It is attributed to one artifact and one source: a transcript, a run manifest or a driven run.

## Context

No type in the store records model calls, tokens or cost. `RunOutcome` and `ModelUsage` hold them
per transcript (`crates/observe/trace-domain/src/ir.rs:473-546`), and `eval.run-manifest/1` holds
them per eval run (`crates/edge/aep-cli/src/eval.rs:629-634`). Nothing holds them per task.

## Acceptance

- `ess/domains/evidence.yaml` declares the record with every figure listed in the Outcome. Each
  figure is an optional field, so an unknown figure is absent and is never stored as zero.
- The domain declares a command that records the record against an artifact, and an error for a
  record that names no source digest.
- `ess specify validate` passes with the newest `ess`.
- `cargo xtask ess` regenerates `generated/ess/`, and `ess-gate` passes with its recorded counts
  updated in `xtask/tests/ess_gate.rs`.

## Scope

Cited from `AGENTS.md` (Normative documents, Gate):

- `ess/domains/evidence.yaml`
- `generated/ess/`
- `xtask/tests/ess_gate.rs`

## Out of Scope

The evidence kind and its CLI, which are `story:evidence-records-task-spend`.
