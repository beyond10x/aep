---
format: aep.planning-md/3
id: dependency-blocker:defect-stories-whose-code-lives-in-sibling-repositories
kind: dependency-blocker
status: open
title: Four defect stories describe code in metaharness and agentplugins
relations:
- blocks: story:two-adapters-two-paths
- blocks: story:a-dropped-stream-does-not-end-a-run
- blocks: story:cold-start-outlives-the-deadline
- blocks: story:wave-skill-defects-found-by-running-it
revision: 1
---
# Dependency blocker: four defect stories describe code in sibling repositories

## What is blocked

Four draft stories in this store describe defects whose code is not in this repository:

- `story:two-adapters-two-paths` — `metaharness run --model-endpoint` and the b10x adapter.
- `story:a-dropped-stream-does-not-end-a-run` — the model turn loop of a driven run.
- `story:cold-start-outlives-the-deadline` — `harness-responses`' `DEFAULT_OPERATION_TIMEOUT`.
- `story:wave-skill-defects-found-by-running-it` — the `wave` skill's charters.

At `824869fec` a search over `crates/` and `xtask/` for `model-endpoint`,
`DEFAULT_OPERATION_TIMEOUT`, `harness-responses` and `reading the event stream` finds nothing
(coordinator, 2026-11-09). `AGENTS.md` § *Agent plugins, Harness and Metaharness*: model execution
belongs to `metaharness aep drive`, and skills and agents live in the `agentplugins` repository;
this repository carries none.

## What would clear it

Each defect is verified against its owning repository (metaharness, agentplugins) and either
filed there or found fixed there; then the story here is archived citing that record. Neither
needs a decision; both need work in a repository this run does not change.

## Why it is not cleared here

The zero-open-issues run of 2026-11-09 changes only `beyond10x/aep`. Archiving these stories here
without verifying the owning repository would claim a fix nobody observed.
