---
format: aep.planning-md/3
id: story:spend-against-baseline
kind: story
status: draft
title: Spend per task kind is reported against a baseline and a target
relations:
- decomposes: epic:deterministic-delivery-workflows
- depends_on: story:evidence-records-task-spend
revision: 2
---
# Story: Spend per task kind is reported against a baseline and a target

## Outcome

A read verb, `aep plan artifact spend [--kind k] [--since <date>]`, prints one row per task kind.
Each row has:

- the number of tasks with a `task_spend` record;
- the median model calls, tokens and cost;
- the baseline and the target the project declares;
- a column `under`, `over` or `unknown` saying where the median stands against the target.

`--baseline <n>` prints the medians of the first `n` recorded tasks of each kind, by record instant.
A person copies that row into `.engineering/project.yaml` as the baseline. The verb never writes
the baseline itself.

## Context

The `review-value` table reads cost only from an eval manifest that a review's `--ref` points to
(`crates/edge/aep-cli/src/planning.rs:5628-5637`). No view aggregates spend over tasks.

## Acceptance

- Over a fixture store of three `task_spend` records, the table prints their medians.
- A target of 10 model calls against a median of 12 prints `over`. A target of 15 prints `under`.
- `--baseline 2` over that store prints the medians of the two earliest records.
- The baseline and target are read from `.engineering/project.yaml`. `validate` refuses a target
  that is not strictly below its baseline, naming both values.
- A row whose figures are all unknown prints `unknown`, never `0`.
- Two runs over one store give identical bytes: the verb takes no clock.

## Scope

Inferred:

- `crates/edge/aep-cli/src/planning.rs`
- the `.engineering/project.yaml` schema in `crates/plan/aep-contract`

## Depends on

`story:evidence-records-task-spend`.
