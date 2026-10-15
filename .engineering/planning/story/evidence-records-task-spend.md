---
format: aep.planning-md/3
id: story:evidence-records-task-spend
kind: story
status: implemented
title: Evidence records a task's model calls, tokens and cost
relations:
- decomposes: epic:deterministic-delivery-workflows
- depends_on: story:task-spend-is-modelled
- depends_on: story:llm-step-declares-judgement
- serves: vision:O2
scope:
- confidence: cited
  path: crates/edge/aep-cli/src/planning.rs
- confidence: cited
  path: crates/govern/aep-domain/src/evidence.rs
- confidence: inferred
  path: crates/observe/trace-domain
- confidence: cited
  path: ess/domains/evidence.yaml
- confidence: cited
  path: schemas/generated/driver-steps.schema.json
revision: 9
transitions:
- {from: "draft", to: "proposed", at: "2026-10-14T19:07:13Z", actor: "human:timo", revision: 7, decided_on: {"recorded":{"review_outcome":2}}}
- {from: "proposed", to: "active", at: "2026-10-14T19:07:13Z", actor: "human:timo", revision: 8, decided_on: {"recorded":{"review_outcome":2}}}
- {from: "active", to: "implemented", at: "2026-10-15T02:18:31Z", actor: "human:timo", revision: 9, decided_on: {"recorded":{"test_result":1,"review_outcome":3}}}
---
# Story: Evidence records a task's model calls, tokens and cost

## Outcome

`aep plan artifact evidence <id> --kind task_spend --from <file>` records every figure that
`story:task-spend-is-modelled` declares against the artifact. The file may be a transcript, an
`eval.run-manifest/1` or a driven run's report. The record keeps the source's digest and a path
relative to the repository root.

## Context

`trace_conformance` attests that a transcript was checked, "including… at what cost", but it keeps
only counts and digests (`crates/govern/aep-domain/src/evidence.rs:920-970`, `:1245-1250`). The
closed kind list (`evidence.rs:1309-1330`) has no kind that carries a spend figure.

## Acceptance

- A recorded Claude Code transcript fixture yields a record with these figures:
  - model calls equal to the fixture's API requests;
  - each of the five token figures and the cost equal to its terminal usage;
  - wall time equal to its `duration_ms`.
- An `eval.run-manifest/1` fixture yields its `tokens`, `cost_micro_usd` and `wall_time_ms`.
- A driven run's report yields the llm steps declared and the llm steps run.
- A transcript with no usage records each figure as unknown. The command prints `unknown`, never `0`.
- A session that writes several terminal records is folded the way
  `story:a-session-is-one-spend` folds it (`eval.rs:2800-2815`).
- A source outside the repository root is refused, naming the path, so that no absolute local path
  is stored.

## Scope

Cited:

- `crates/govern/aep-domain/src/evidence.rs`
- `crates/edge/aep-cli/src/planning.rs` (`evidence`)
- `schemas/generated/driver-steps.schema.json`: it publishes the evidence-kind enum.

Inferred:

- `crates/observe/trace-domain`
- fixtures under `crates/edge/aep-cli/tests/`

## Depends on

- `story:task-spend-is-modelled`
- `story:concurrent-command-steps`: the last of the three other stories that regenerate
  `schemas/generated/driver-steps.schema.json`. It is ordered after
  `story:command-step-expects-failure` and `story:llm-step-declares-judgement`.
- `story:llm-step-declares-judgement`: it also regenerates `schemas/generated/driver-steps.schema.json`,
  and that story produces the per-run llm step counts.

## Open question

The spend record of `story:task-spend-is-modelled` (`ess/domains/evidence.yaml`, `SpendRecord`)
holds the five split token figures only. An `eval.run-manifest/1` states one total, `tokens`, so the
fixture line above has no field to land in. Decide before building: add a total-tokens figure to
the record (a specification change first), or record a manifest's total as unknown split figures
and drop that part of the line.
