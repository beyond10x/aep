---
format: aep.planning-md/3
id: story:bugfix-workflow
kind: story
status: draft
title: A bug fix runs as a workflow with model calls only at diagnose and fix
relations:
- decomposes: epic:deterministic-delivery-workflows
- depends_on: story:llm-step-declares-judgement
- depends_on: story:command-step-expects-failure
revision: 3
---
# Story: A bug fix runs as a workflow with model calls only at diagnose and fix

## Outcome

A `bugfix` workflow (reproduce → diagnose → fix → verify) and its step map ship.

- **reproduce** is a command step declared to expect failure (`story:command-step-expects-failure`).
  It runs the reproducing command the run names, and it passes when that command fails.
- **diagnose** first runs command steps that collect state: the reproducing command's output, the
  logs and traces the run names, and the `aep plan artifact show` of the task. Then it runs one
  llm step over that state.
- **fix** is an llm step.
- **verify** is command steps: the reproducing command passes, then the package gates.

The collection steps run one after another until `story:concurrent-command-steps` lands. After that
they run at once, with no change to this map.

## Context

No bug-fix workflow exists. `incident/standard` has a diagnose state but no step map
(`workflows/incidents/standard.yaml:27`). The loop runs only as the `aep:diagnosing` skill's prose,
in the agentplugins repository.

## Acceptance

- The workflow and the map validate. The map declares exactly two llm steps, diagnose and fix, each
  with `judgement:`.
- A driven run over a fixture is refused at reproduce, with the reason "nothing reproduced", when
  the named command passes before any change.
- With the model steps stubbed:
  - the collection steps write `output:` files, which the diagnose step names in its `context`
    (`story:command-step-expects-failure`), and that step is launched with their content;
  - verify routes back to fix while the reproducing command still fails;
  - the run completes when that command passes.

## Scope

Inferred:

- `workflows/bugfix/` (new)
- `drivers/bugfix/` (new)
- a new test file under `crates/drive/aep-driver/tests/`

## Depends on

- `story:llm-step-declares-judgement`
- `story:command-step-expects-failure`
