---
format: aep.planning-md/3
id: review-result:detflow-acceptance-round-3
kind: review-result
status: active
title: 'Plan critic (acceptance), round 3: deterministic delivery workflows'
relations:
- reviews: story:bugfix-workflow
- reviews: story:concurrent-command-steps
- reviews: story:release-step-map
- reviews: story:spend-against-baseline
- reviews: epic:deterministic-delivery-workflows
revision: 1
---
Plan-critic round 3, acceptance lens, on the four remaining stories of
`epic:deterministic-delivery-workflows`, read against wave detflow-2's head (870a25919), which shipped
`expect: failure`, run inputs, `output:` and the `task_spend` record.

Verdict: needs-revision. Round 1 and 2 fixes still hold; the new findings are acceptance gaps the
shipped work exposes.

ESS needs: `story:concurrent-command-steps` changes the `aep.drive.Step` noun (needs
`ess/domains/drive.yaml` first); `story:spend-against-baseline` changes a command, an outcome and the
project configuration (needs `ess/domains/evidence.yaml` and `plan.yaml` first);
`story:bugfix-workflow` and `story:release-step-map` add workflow and map data only. None needs an ESS
release newer than the pinned 0.59.0.

```findings
[
{"file":".engineering/planning/story/concurrent-command-steps.md","line":17,"category":"acceptance","severity":"warning","verdict":"NEEDS-CHANGE","origin":"pre-existing","message":"llm or operator steps in a concurrent state, an after cycle, a duplicate id, and id/after outside a concurrent state have no acceptance; add a bullet refusing them at validation, naming the offender"},
{"file":".engineering/planning/story/concurrent-command-steps.md","line":41,"category":"acceptance","severity":"warning","verdict":"NEEDS-CHANGE","origin":"pre-existing","message":"the shared depends_on bullet names no circuit_breaker threshold and no expected result, so a concurrent run and a serial run both satisfy it"},
{"file":".engineering/planning/story/concurrent-command-steps.md","line":48,"category":"acceptance","severity":"warning","verdict":"NEEDS-CHANGE","origin":"pre-existing","message":"scope and acceptance omit ess/domains/drive.yaml and its conformance scenarios, so the spec-first order is unrecorded for new step fields and refusals"},
{"file":".engineering/planning/story/bugfix-workflow.md","line":27,"category":"acceptance","severity":"warning","verdict":"NEEDS-CHANGE","origin":"pre-existing","message":"'run at once, with no change to this map' is false: concurrent is opt-in per state, so the map needs concurrent: true on diagnose"},
{"file":".engineering/planning/story/bugfix-workflow.md","line":40,"category":"acceptance","severity":"warning","verdict":"NEEDS-CHANGE","origin":"pre-existing","message":"'refused at reproduce' names no observable (reported blocked at the state, exit status, evidence reason), and an expect:failure step yields a failing verdict, not a refusal"},
{"file":".engineering/planning/story/bugfix-workflow.md","line":19,"category":"acceptance","severity":"warning","verdict":"NEEDS-CHANGE","origin":"pre-existing","message":"the acceptance does not say how a fixture names the reproducing command, logs and task id (--input values spliced into sh -c), so the fixture cannot be built from the text"},
{"file":".engineering/planning/story/release-step-map.md","line":38,"category":"acceptance","severity":"blocker","verdict":"NEEDS-CHANGE","origin":"pre-existing","message":"reaching complete needs a human approval at await_approval (approval.release-create.by_human) and the acceptance names no operator step or answer; state the --pause-on-approval stop, the recorded approval and the resume"},
{"file":".engineering/planning/story/release-step-map.md","line":40,"category":"acceptance","severity":"warning","verdict":"NEEDS-CHANGE","origin":"pre-existing","message":"the merge bullet is a disjunction (operator step, or a command gated on green checks) so either outcome passes; choose one and name what the stub observes"},
{"file":".engineering/planning/story/spend-against-baseline.md","line":38,"category":"acceptance","severity":"blocker","verdict":"NEEDS-CHANGE","origin":"pre-existing","message":"the project.yaml shape for baseline and target (keys, per kind, which figure under/over compares, behaviour at equality) is unspecified, so the fixture and the validate refusal cannot be written"},
{"file":".engineering/planning/story/spend-against-baseline.md","line":37,"category":"acceptance","severity":"warning","verdict":"NEEDS-CHANGE","origin":"pre-existing","message":"--baseline 2 takes a median of two integers with no stated rule for an even count, and partly absent figures are not said to be skipped"},
{"file":".engineering/planning/story/spend-against-baseline.md","line":16,"category":"acceptance","severity":"warning","verdict":"NEEDS-CHANGE","origin":"pre-existing","message":"'task kind' is undefined (the kind of the record's artifact?) and --kind and --since have no acceptance bullet"},
{"file":".engineering/planning/story/spend-against-baseline.md","line":43,"category":"acceptance","severity":"warning","verdict":"NEEDS-CHANGE","origin":"pre-existing","message":"scope omits ess/domains/evidence.yaml and plan.yaml and the ESS scenarios for the new command and config key; it also cites aep-contract where ProjectConfig lives in aep-domain project.rs:349"}
]
```
