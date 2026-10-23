---
format: aep.planning-md/3
id: story:checks-map-trace-spec-path
kind: story
status: draft
title: The checks map's trace step finds its specification from any project
summary: The implement trace step of drivers/development/checks.yaml names a relative --spec path that exists only in this repository; a run in any other project spends the step's retries on a missing file.
owner: project
tags:
- driver
relations:
- decomposes: epic:reference-driver
revision: 1
---
# Story: The checks map's trace step finds its specification from any project

## Outcome

A governed run under `drivers/development/checks.yaml` in a project other than this repository
reaches the `implement` trace step and the step runs: the trace specification the step names is
found whatever the run's working directory is.

## Context

The `implement` state of `drivers/development/checks.yaml` has a `command` step running `aep
observe trace evidence --spec conformance/trace/expectations.driven-step.trace.yaml --transcript
{transcript} --out {run_directory}/trace-implement.yaml`. The `--spec` path is relative, a
`command` step runs with the adopting project as its working directory, and the file exists only
in this repository. The driver's placeholder expander (`crates/edge/aep-cli/src/drive.rs`,
`expand`) fills `{run_directory}`, `{task}` and `{transcript}` and nothing else; nothing resolves a
path against the protocol root or the map's own directory, at 0.77.0 (`23294974`) as at 0.68.1.

ess-ui-templates reported on 2026-10-23: run `ess-ui-ess-068-crates/2` spent the step's three
attempts on "No such file or directory (os error 2)" and stopped as budget-exhausted in
`implement`. Workaround there: an untracked copy of the file at the same path in the worktree.

## Acceptance

Not yet decided; the fix is one of: a placeholder the map can use for the map's or the protocol
root's directory, `aep observe trace evidence` accepting a specification by a name it resolves
from the protocol root, or the trace specification shipped beside the map. Each is a design
decision for the orchestrator or operator; this story records the defect.
