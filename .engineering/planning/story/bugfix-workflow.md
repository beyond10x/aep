---
format: aep.planning-md/3
id: story:bugfix-workflow
kind: story
status: implemented
title: A bug fix runs as a workflow with model calls only at diagnose and fix
relations:
- decomposes: epic:deterministic-delivery-workflows
- depends_on: story:llm-step-declares-judgement
- depends_on: story:command-step-expects-failure
- depends_on: story:concurrent-command-steps
- serves: vision:O2
scope:
- confidence: cited
  path: crates/edge/aep-cli/tests/bugfix_workflow.rs
- confidence: cited
  path: drivers/bugfix/standard.yaml
- confidence: cited
  path: generated/instructions/README.md
- confidence: cited
  path: generated/instructions/bugfix/standard.md
- confidence: cited
  path: workflows/bugfix/standard.yaml
revision: 9
transitions:
- {from: "draft", to: "proposed", at: "2026-10-15T06:23:58Z", actor: "human:timo", revision: 6, decided_on: {"recorded":{"review_outcome":9}}}
- {from: "proposed", to: "active", at: "2026-10-15T06:23:58Z", actor: "human:timo", revision: 7, decided_on: {"recorded":{"review_outcome":9}}}
- {from: "active", to: "implemented", at: "2026-10-16T01:52:58Z", actor: "human:timo", revision: 9, decided_on: {"recorded":{"test_result":1,"review_outcome":9}}}
---
# Story: A bug fix runs as a workflow with model calls only at diagnose and fix

## Outcome

A `bugfix/standard` workflow (reproduce → diagnose → fix → verify, with verify routing back to fix
on failing evidence) and its step map `drivers/bugfix/standard.yaml` ship. The run names its
inputs with `--input`: `repro` (the reproducing command), `logs` and `traces` (files),
`artifact` (the planning artifact id) and `gate` (the package gate command).

Steps per state, with the evidence each command step mints:

| state | step | evidence |
|---|---|---|
| reproduce | command `[sh, -c, "{input.repro}"]`, `expect: failure`, `output: repro.txt`; the only run of the command before verify | `test_result`, suite `reproduction`, verifier `test-runner` |
| diagnose (`concurrent: true`, `story:concurrent-command-steps`) | commands, at once: `[cat, "{input.logs}"]` to `logs.txt`, `[cat, "{input.traces}"]` to `traces.txt`, `[aep, plan, artifact, show, "{input.artifact}"]` to `artifact.txt` | none: a pass is `Nothing`, and a failed read is no verdict, retried and then `budget-exhausted` |
| diagnose | one `llm` step, a barrier, with `context` naming `{run_directory}/repro.txt` and the three collected files | none (an `llm` step has none) |
| fix | one `llm` step | none |
| verify | command `[sh, -c, "{input.repro}"]` (expect success), then command `[sh, -c, "{input.gate}"]` | `test_result`, suite `regression`; then `test_result`, suite `gate`, both verifier `test-runner` |

Transitions of `workflows/bugfix/standard.yaml`, tried in document order. `fixed` is terminal.

| from | to | `when:` |
|---|---|---|
| reproduce | diagnose | `tests.reproduction.failed == 0` (the failure was reproduced) |
| diagnose | fix | none: collection mints no evidence, and the hypothesis is judgement |
| fix | verify | none: whatever the fix achieved is observed at verify |
| verify | fixed | `all: [tests.regression.failed == 0, tests.gate.failed == 0]` |
| verify | fix | `any: [tests.regression.failed > 0, tests.gate.failed > 0]` |

reproduce has no other edge, so a reproduction that exits 0 leaves no permitted move.

## Context

No bug-fix workflow exists. `incident/standard` has a diagnose state but no step map
(`workflows/incidents/standard.yaml`). The loop runs only as the `aep:diagnosing` skill's prose,
in the agentplugins repository.

A reproduction that exits 0 is a failing verdict with the reason `nothing reproduced`
(`CommandStep::verdict`, `NOTHING_REPRODUCED`). Because the step declares evidence, it is
submitted as a failing `test_result` for suite `reproduction`, with the reason noted
(`command_verdict`, drive.rs). Without `evidence:` it would be no verdict and end the run
`budget-exhausted`. The `reproduce -> diagnose` guard is then false, so the run stops `blocked`
there.

`aep drive run` refuses a map with `llm` steps (`CommandOnlyHost`). So the fixture drives the
map through `aep_cli::drive::start_with_host`, with a test `ExecutionHost` whose `llm` executor is
a stub. Command steps run in the real CLI host. Inputs are given as `RunArgs::input`, the parsed
form of `--input <name>=<value>`.

## Acceptance

- `aep govern validate --root .` exits 0 with the new workflow and map.
  `StepMap::llm_steps_declared` is 2 for the map: the diagnose and fix steps, each with a
  `judgement:`.
- `aep govern workflow instruct --out generated/instructions` leaves no diff. This covers the
  committed `generated/instructions/bugfix/standard.md` and its line in
  `generated/instructions/README.md` (`crates/edge/aep-cli/tests/instructions.rs` passes).
- **Nothing reproduced.** The fixture gives `repro=true`. The driving call exits 1, and
  `cursor.json` holds `"status": "blocked"` and `"state": "reproduce"`. The printed report carries
  the note ending `exited 0, and the step expects it to fail: nothing reproduced`. No `logs.txt`
  exists in the run directory, and the stub `llm` executor was never called.
- **Driven to completion.** The fixture gives `repro=test -f fixed`, `gate=true`, `logs` and
  `traces` naming two fixture files, and `artifact` naming a fixture story.
  - The run directory holds `repro.txt`, `logs.txt`, `traces.txt` and `artifact.txt`. The stub
    `llm` executor sees all four as the diagnose step's context, with their content.
  - The stub fix step creates `fixed` only on its second call. verify fails once and routes back to
    fix. The report's transitions are, in order: `reproduce -> diagnose`, `diagnose -> fix`,
    `fix -> verify`, `verify -> fix`, `fix -> verify`. The move into the terminal `fixed` is reported
    as the completion (`TransitionResult::Completed`).
  - The run then ends `completed` and the driving call exits 0. `cursor.json` records two `llm`
    steps declared and three run (diagnose once, fix twice).

## Scope

Inferred:

- `workflows/bugfix/standard.yaml` (new)
- `drivers/bugfix/standard.yaml` (new; diagnose declares `concurrent: true`)
- `generated/instructions/bugfix/standard.md` (new) and `generated/instructions/README.md`
  (one line), both rendered by `aep govern workflow instruct`
- `crates/edge/aep-cli/tests/bugfix_workflow.rs` (new; distinct from every other story's test
  file)
- `CHANGELOG.md` (Unreleased)

No ESS change: the story adds workflow and map data only. No ESS release newer than 0.60.0 is
needed.

## Depends on

- `story:llm-step-declares-judgement` (`judgement:`)
- `story:command-step-expects-failure` (`expect: failure`, `output:`, `--input`)
- `story:concurrent-command-steps` (`concurrent: true` on diagnose). This story lands second, so the
  map ships with the opt-in from its first commit and no later edit to `drivers/bugfix/` is owed.
