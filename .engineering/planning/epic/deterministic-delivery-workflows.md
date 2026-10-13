---
format: aep.planning-md/3
id: epic:deterministic-delivery-workflows
kind: epic
status: draft
title: Delivery and bug-fix loops run as deterministic workflows
summary: The engine drives plan, implement, gate, release and the bug-fix loop; a model is called only where judgement is needed, and model calls, tokens and cost are recorded per task against a baseline.
relations:
- serves: vision:O2
- serves: vision:O3
- informed_by: epic:reference-driver
- informed_by: epic:self-evaluation
revision: 3
---
# Epic: Delivery and bug-fix loops run as deterministic workflows

## Outcome

Delivery (plan, implement, gate, release) and the bug-fix loop (reproduce, diagnose, fix, verify)
run as workflows that the engine drives step by step. A model is called only at a step whose
judgement no command can supply. Every such call is counted. For each task, the store records how
many model calls it took, how many tokens and what it cost, against a baseline, and each target is
set lower than the baseline before it.

## Why Now

The cost of a task today depends on how the agent read the prose that drives it. Nothing records
that cost against the task.

- Only `adp/default/2` has step maps (`drivers/development/default.yaml`, `checks.yaml`). In both,
  receive, specify, decompose, establish_verifiers, implement and adversarial_verify are
  `kind: llm` steps (`default.yaml:57,88,111,133,185,315`). Only `verify` is all `kind: command`
  (`default.yaml:216-300`).
- `release/source`, `incident/standard` and the other workflows under `workflows/` have no step
  map. No bug-fix workflow exists at all.
- Decomposition, the critic panel, wave selection, release and diagnosis run only as skills an
  agent follows. `aep plan artifact waves` derives a wave, but choosing and dispatching it is prose
  (`crates/edge/aep-cli/src/planning.rs:1110-1112`). `cargo xtask release` checks a release but does
  not cut one (`xtask/src/main.rs:70-73`).
- Token and cost figures are read from transcripts (`crates/observe/trace-domain/src/ir.rs:473-546`)
  and summed per eval run (`crates/edge/aep-cli/src/eval.rs:629-634`). None reaches the planning
  store.
  - `trace_conformance` evidence keeps counts and digests only
    (`crates/govern/aep-domain/src/evidence.rs:920-970`).
  - No evidence kind carries a token or cost figure (`evidence.rs:1309-1330`).
  - "model calls" is counted nowhere.

## The steps, and which need a model

| loop | step | today | deterministic replacement |
|---|---|---|---|
| plan | receive the request | llm | none: reading intent is judgement |
| plan | specify (ESS model) | llm | partly: `ess specify validate` and regeneration are commands; writing the model is judgement |
| plan | decompose an epic | llm, plus four critic calls | none for the draft. The panel runs only when the set has two or more items, as today |
| plan | select a wave | skill prose over `aep plan artifact waves` | yes: the wave is derived. Only approval is the operator's |
| plan | scope a story | llm (story-scoper) | partly: surfaces cited from symbols and paths are a lookup; inferred ones stay judgement |
| implement | write the failing test, then the change | llm | none |
| implement | adversarial verify | llm | partly: mutation of guarded conditions is a command; finding new cases is judgement |
| gate | package gates, full gate, validate, evidence | command | already deterministic |
| release | version, changelog heading, tag, status pins, gate, PR, verify release, record | skill prose over a numbered list (`AGENTS.md`, Releases and commits) | yes, every step. Only writing changelog prose for a reader may need a model |
| bug fix | reproduce | skill prose (`aep:diagnosing`) | yes, once a failing command is named: the step passes when that command fails before the fix |
| bug fix | diagnose | skill prose | none for hypotheses. Collecting state (logs, traces, store reads) is a set of commands that can run at once |
| bug fix | fix | llm | none |
| bug fix | verify | skill prose | yes: the reproducing command passes, and the package gates pass |

## The measures

These are recorded per task, where a task is the artifact a run or session worked on.

- **model calls**: API requests the transcript records.
- **tokens**: input, output, cache read, cache write and thinking.
- **cost**: in micro-USD, as the transcript or the run manifest states it.
- **wall time.**
- **llm steps run**, beside llm steps declared by the map.

A baseline is the median of the first recorded tasks of one kind. The read verb prints it, and a
person copies it into `.engineering/project.yaml`. A target is a bound set strictly below the
baseline. The read verb reports each task kind against its baseline and target.

The record is a new noun. It is modelled in `ess/domains/evidence.yaml` before any story builds on
it.

## Scope

Each item names the story that owns it.

- The spend record modelled in ESS: `story:task-spend-is-modelled`.
- Per-task spend recorded as evidence from a transcript, a run manifest or a driven run:
  `story:evidence-records-task-spend`.
- A baseline and target report: `story:spend-against-baseline`.
- A declared reason on every `kind: llm` step, with llm steps counted per run:
  `story:llm-step-declares-judgement`.
- A command step that expects failure, run inputs, and step output handed to a later step:
  `story:command-step-expects-failure`.
- The release steps as commands: `story:release-cutting-commands`.
- A step map for the release workflow with no model step: `story:release-step-map`.
- A bug-fix workflow and its step map: `story:bugfix-workflow`.
- Concurrent command steps, opted into per state: `story:concurrent-command-steps`.

The table's other "yes" and "partly" rows are not in this epic's scope. These are deriving wave
selection, citing scope, and mutation in the adversary. They are later stories.

## Out of Scope

- A model judging whether another model's work is good. It stays refused, as `epic:self-evaluation`
  says.
- A score or leaderboard across tasks.
- Running models from `aep`: model execution stays `metaharness aep drive`
  (`crates/edge/aep-cli/src/drive.rs:2851`).
- Changing an agent plugin's prose. That is the agentplugins repository's work, against the step
  maps this epic adds.

## Risks

- Counting what a harness does not report. A transcript without usage records an unknown, never a
  zero (the invariant "Unknown differs from false").
- Goodhart: a target on calls can push judgement into a cheaper but wrong command. Every
  replacement keeps its gate, and a target never removes one.

## Done When

- A release of this repository is cut by `aep drive run` over a release step map that has no
  `kind: llm` step. This is checked at epic close, on the first release after
  `story:release-step-map` lands.
- A bug fix runs through the bug-fix map with model calls only at diagnose and fix.
- `aep plan artifact` reports model calls, tokens and cost per task, against a recorded baseline,
  for at least one task kind.
