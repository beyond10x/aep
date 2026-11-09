---
format: aep.planning-md/3
id: approval-record:zero-open-issues-acceptance-2026-11-09
kind: approval-record
status: draft
title: Stories of the zero-open-issues epic accepted with no operator present
tags:
- non-interactive
relations:
- decides: epic:zero-open-issues-2026-11-09
- decides: story:relate-and-unrelate-write-the-revision-they-print
- decides: story:an-artifact-id-can-be-renamed
- decides: story:a-directory-and-a-file-inside-it-read-as-disjoint
- decides: story:history-shows-a-removed-edge
- decides: story:relations-order-is-the-same-in-every-store
- decides: story:published-pattern-residue
- decides: story:agents-dependency-rule-matches-the-manifests
- decides: story:discarded-protocol-source-tree-is-removed
- decides: story:legacy-ess-report-calendar-refusal
- decides: story:checks-map-evidence-producers
- decides: story:a-guarded-rung-counts-records-not-verdicts
- decides: task:artifact-root-follows-project
- decides: task:decomposes-edge-examples
- decides: story:a-killed-publication-stage-is-swept
- decides: story:eventlog-lifecycle-refusal-names-the-legal-moves
- decides: story:strict-advisory-classes-on-an-eventlog-plan
- decides: story:unowned-document-in-eventlog-projection-is-reported
- decides: story:migration-receipt-identity-under-symlinked-selector
- decides: story:writer-control-socket-path-budget
revision: 1
---
# Approval record: the epic's stories are accepted, and its stale defect records closed, with no operator present

## The stop

Two stops the `aep:planning` and `aep:implementing` skills leave to the operator: accepting a story
(moving it out of `draft` to `active`), and retiring an artifact (`move --to archived`).

## Why it was not asked

The run is non-interactive: the session was dispatched as a sub-agent with the instruction to run
to the goal, and a Stop hook holds it until the goal is met. Nothing it prints reaches a person
before it ends.

## What the operator said

Goal, 2026-11-09, verbatim: "zero remaining issues on github, all bugs fixed from planning store". The dispatch states that this goal is the acceptance of the
stories under `epic:zero-open-issues-2026-11-09` and the approval of each wave.

## What was decided

1. `epic:zero-open-issues-2026-11-09` and the ten stories below are moved `draft -> proposed -> active`. Each describes an
   open GitHub issue or a defect a triage pass found still present at `824869fec`, with the
   `file:line` in its body.
   - `story:relate-and-unrelate-write-the-revision-they-print`
   - `story:an-artifact-id-can-be-renamed`
   - `story:a-directory-and-a-file-inside-it-read-as-disjoint`
   - `story:history-shows-a-removed-edge`
   - `story:relations-order-is-the-same-in-every-store`
   - `story:published-pattern-residue`
   - `story:agents-dependency-rule-matches-the-manifests`
   - `story:discarded-protocol-source-tree-is-removed`
   - `story:legacy-ess-report-calendar-refusal`
   - `story:checks-map-evidence-producers`
2. The nine artifacts below are moved `draft -> archived`. Each describes a defect that is fixed on
   main or whose surface was deleted; the proof is the `## Closed 2026-11-09` section in its body.
   They are archived one by one on that proof, not as a tidy-up.
   - `story:a-guarded-rung-counts-records-not-verdicts`
   - `task:artifact-root-follows-project`
   - `task:decomposes-edge-examples`
   - `story:a-killed-publication-stage-is-swept`
   - `story:eventlog-lifecycle-refusal-names-the-legal-moves`
   - `story:strict-advisory-classes-on-an-eventlog-plan`
   - `story:unowned-document-in-eventlog-projection-is-reported`
   - `story:migration-receipt-identity-under-symlinked-selector`
   - `story:writer-control-socket-path-budget`

## What was not decided

- `decision-blocker:which-parent-directories-a-snapshot-trusts` and
  `decision-blocker:how-the-checks-map-finds-its-trace-specification` stay open; their stories stay
  `draft`.
- Four draft stories describe failures of surfaces that moved to other repositories and were not
  verified there: `story:two-adapters-two-paths`, `story:a-dropped-stream-does-not-end-a-run`,
  `story:cold-start-outlives-the-deadline` (metaharness) and
  `story:wave-skill-defects-found-by-running-it` (agentplugins). They are untouched.
- `story:journal-actor-from-configuration` and
  `story:workspace-declaration-has-one-reader-tests-included` read as a policy change and a
  refactor, not a defect; untouched.
- `dependency-blocker:metaharness-source-receipt` stays open.
- No evidence is recorded by this record. A story reaches `implemented` only on a `test_result` the
  run observed.

## What the operator would have been asked

Whether these ten are the defects to fix under one epic, and whether the nine records may be
archived on the proof in their bodies.
