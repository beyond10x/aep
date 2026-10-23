---
format: aep.planning-md/3
id: story:checks-map-evidence-producers
kind: story
status: draft
title: The checks map produces every record the driven plan demands
summary: 'A governed run under drivers/development/checks.yaml launches without --allow-evidence-gap: the map gains command steps producing the verification and specification records the plan demands, as default.yaml already does.'
tags:
- driver
relations:
- decomposes: epic:evidence-gated-completion
revision: 2
---
# Story: The checks map produces every record the driven plan demands

## Outcome

A governed run under `drivers/development/checks.yaml` launches without `--allow-evidence-gap`:
the coverage pre-flight (`aep drive run`, `crates/edge/aep-cli/src/drive.rs`, the
`evidence_coverage` call before the lock) finds a producer in the map for every evidence kind the
task's plan will demand, as it does for `drivers/development/default.yaml` today.

## Context

The pre-flight compares the plan against the map before a run id exists
(`story:plan-map-coverage`). For `development/checks` it names two kinds no step of the map
declares, `verification` and `specification`, and two more when the task does not declare
`change.code: false`, `contract_result` and `property_test_result`
(`crates/drive/aep-driver/tests/coverage.rs`,
`the_checks_map_still_reports_the_gap_the_governed_run_measured_and_nothing_more`, which records
the gap as a fact about the tree). `default.yaml` closed the same gap in
`story:evidence-producers-for-the-driven-map`: `aep govern validate --root . --evidence <file>`
produces the `verification` record (verifier `aep`, claim `document-tree-valid`) and `aep observe
specification evidence --task {task} --snapshot … --out <file>` produces the `specification`
record. Neither step runs a build, so both fit a map written for documents-and-shell work.

metaharness reported on 2026-10-23, through the orchestrator, that `metaharness aep drive` on
`development/checks` needs `--allow-evidence-gap` and that no AEP story covered it. This story is
that record.

## Acceptance

1. `drivers/development/checks.yaml` gains a `command` step producing `verification` through
   `aep govern validate --root . --evidence {run_directory}/verification.yaml` (verifier `aep`)
   and one producing `specification` through `aep observe specification evidence` with the run's
   `{task}` (verifier `aep`), each in the state whose guard first wants the kind; the map's
   `commands:` declarations are unchanged by it, because a `command` step is the driver's.
2. A decision, recorded in the map's header comment, on `contract_result` and
   `property_test_result`: either the map produces them the way `default.yaml` does, or the tasks
   this map drives declare `change.code: false` and the pre-flight's applicability rule removes the
   two principles. Whichever is taken, `aep drive run --map development/checks` on a task of the
   kind this map drives launches without `--allow-evidence-gap`.
3. `crates/drive/aep-driver/tests/coverage.rs`: the test that records the gap as a fact is
   replaced by one asserting `is_covered()` for `checks.yaml` under the task shape decided in 2,
   and the applicability test keeps passing.
4. `CHANGELOG.md` carries an Unreleased entry; `aep govern validate --root .` loads the map;
   `AEP_BIN=<target>/release/aep task check` exits 0.

## Out of scope

- metaharness's own launch flags and prompt text; it is told when this ships.
- Changing what the `development` protocol demands.
