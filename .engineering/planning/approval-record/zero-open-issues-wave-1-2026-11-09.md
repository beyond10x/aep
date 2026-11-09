---
format: aep.planning-md/3
id: approval-record:zero-open-issues-wave-1-2026-11-09
kind: approval-record
status: draft
title: Wave plan and wave 1 of the zero-open-issues epic approved with no operator present
tags:
- non-interactive
relations:
- decides: story:a-directory-and-a-file-inside-it-read-as-disjoint
- decides: story:agents-dependency-rule-matches-the-manifests
- decides: story:checks-map-evidence-producers
- decides: story:discarded-protocol-source-tree-is-removed
revision: 1
---
# Wave plan and wave 1 of the zero-open-issues epic approved with no operator present

## Which stop

The stage-1 wave proposal of the `aep:implementing` skill (wave mode, skill version 0.26.0), for
the plan of `epic:zero-open-issues-2026-11-09`: six waves, derived once with
`aep plan artifact waves --kind story --status active` (aep 0.79.0). The plan page is
`docs/plan/zero-open-issues-2026-11-09.md`, which quotes the verb's answer verbatim.

## Why it was not asked: non-interactive

The session runs under a Stop hook that holds it until the goal is met, and its instructions say
not to stop and not to ask. Nothing it prints reaches the operator before it ends.

## What the operator said

Goal, 2026-11-09, verbatim: "zero remaining issues on github, all bugs fixed from planning store".
The run's brief states that this goal is the acceptance of these stories and the approval of each
wave.

## What was decided

- The plan, six waves: 1 `story:a-directory-and-a-file-inside-it-read-as-disjoint`,
  `story:agents-dependency-rule-matches-the-manifests`, `story:checks-map-evidence-producers`,
  `story:discarded-protocol-source-tree-is-removed`; 2 `story:legacy-ess-report-calendar-refusal`,
  `story:published-pattern-residue`; 3 `story:relate-and-unrelate-write-the-revision-they-print`;
  4 `story:relations-order-is-the-same-in-every-store`; 5 `story:an-artifact-id-can-be-renamed`;
  6 `story:history-shows-a-removed-edge`.
- Wave 1 runs now. Each later wave gets its own record when it starts, after the store is re-checked.
- Check path: the verb (typed scope entries); 0 unassessed; 20 collisions, all between waves.
- Units run one at a time (budget: one sub-agent at once), each in its own managed worktree with
  its own `target/`, dispatched as `aep:implementor` then `aep:adversary`, both on model opus.
- Commits authorised: one commit per unit on `impl/<slug>`, one merge per unit into
  `wave/zero-open-issues-2026-11-09`, the coordinator's store, plan-page and changelog commits on
  that branch, the push of that branch, one pull request into `main` and its merge once the gate is
  green. Nothing else: no tag, no version bump, no release.

## What stays a stop

A red unit after two attacks leaves its wave. A gate that is not green is not green. A move whose
evidence the run did not produce stays refused. The release is not part of this run.

## What the operator would have been asked

Whether to run these six waves in this order, one unit at a time, with these commits.
