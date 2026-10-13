---
format: aep.planning-md/3
id: story:llm-step-declares-judgement
kind: story
status: draft
title: Every model-backed step says why it needs judgement
relations:
- decomposes: epic:deterministic-delivery-workflows
revision: 2
---
# Story: Every model-backed step says why it needs judgement

## Outcome

A `kind: llm` step in an `aep.driver-steps/1` map carries a `judgement:` line that says what no
command could decide. `aep drive` prints, per map, the number of llm steps declared, and per run the
number of llm steps run.

## Context

A step declares its nature with `kind: command | llm | operator`
(`crates/drive/aep-driver-spec/src/map.rs:230-238`). An llm step has `prompt, context, scope, skills,
harness, description` (`map.rs:649-670`), and none of them says why a model is needed. Both shipped
maps have six llm-backed states (`drivers/development/default.yaml:57,88,111,133,185,315`).

## Acceptance

- A map with an llm step without `judgement:` is refused at validation, with a stable code and the
  step's path.
- Both shipped maps declare `judgement:` on every llm step and validate.
- `aep drive status --run <run>` prints the number of llm steps declared and the number run. The
  run's report carries both counts.
- `cargo xtask schema` regenerates `schemas/generated/driver-steps.schema.json`, and
  `schema-check` passes.

## Scope

Cited:

- `crates/drive/aep-driver-spec/src/map.rs`
- `drivers/development/*.yaml`
- `schemas/generated/driver-steps.schema.json`
- `crates/edge/aep-cli/src/drive.rs`

Inferred: `crates/drive/aep-driver/src/run.rs`, for the run count.

Three later stories touch these same files: `story:command-step-expects-failure`,
`story:concurrent-command-steps` and `story:evidence-records-task-spend`. Each depends on this one.
