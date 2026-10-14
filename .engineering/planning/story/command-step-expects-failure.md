---
format: aep.planning-md/3
id: story:command-step-expects-failure
kind: story
status: active
title: A command step can expect failure, and steps can take inputs and hand on output
relations:
- decomposes: epic:deterministic-delivery-workflows
- depends_on: story:llm-step-declares-judgement
- serves: vision:O2
scope:
- confidence: cited
  path: crates/drive/aep-driver-spec/src/map.rs
- confidence: inferred
  path: crates/drive/aep-driver/src/run.rs
- confidence: cited
  path: crates/edge/aep-cli/src/drive.rs
- confidence: cited
  path: ess/domains/drive.yaml
- confidence: cited
  path: schemas/generated/driver-steps.schema.json
revision: 7
transitions:
- {from: "draft", to: "proposed", at: "2026-10-14T19:07:13Z", actor: "human:timo", revision: 6}
- {from: "proposed", to: "active", at: "2026-10-14T19:07:13Z", actor: "human:timo", revision: 7}
---
# Story: A command step can expect failure, and steps can take inputs and hand on output

## Outcome

A `kind: command` step may declare `expect: failure`, and its result's verdict is then inverted. A
nonzero exit is the pass, and an exit of zero records the reason "nothing reproduced".

A run takes named inputs, `aep drive run --input <name>=<value>`. A step's `run` uses them as
`{input.<name>}`. That is how a bug fix names its reproducing command and how a release names its
version.

A command step may declare `output: <file>`. The driver writes the step's stdout to
`{run_directory}/<file>`. An llm step's `context` may name `{run_directory}/<file>` and
`{input.<name>}`, so a later model step reads what earlier commands collected.

## Context

A step's placeholders are `run_directory`, `task` and `transcript` only
(`crates/drive/aep-driver-spec/src/map.rs:690-718`). An llm step's `context` is a list of literal
files (`map.rs:401-405`). No field maps a failing exit to a pass (`map.rs:544-569`).

## Acceptance

- A step with `expect: failure` whose command exits 1 submits passing evidence.
- The same step whose command exits 0 submits failing evidence with the reason "nothing reproduced".
- A step without `expect` behaves as today: the existing driver tests pass unchanged.
- `{input.repro}` in `run` is replaced with the value given to `--input repro=<value>`.
- A placeholder naming an input the run was not given is refused before any step runs, naming it.
- A command step with `output: collected.txt` writes its stdout to `{run_directory}/collected.txt`.
  A later llm step whose `context` names that path is launched with the file's content.
- `cargo xtask schema` regenerates `schemas/generated/driver-steps.schema.json`, and
  `schema-check` passes.

## Scope

Cited:

- `crates/drive/aep-driver-spec/src/map.rs`
- `schemas/generated/driver-steps.schema.json`
- `crates/edge/aep-cli/src/drive.rs` (`--input`)

Inferred: `crates/drive/aep-driver/src/run.rs`.

## Depends on

`story:llm-step-declares-judgement`, because both change the files above.
