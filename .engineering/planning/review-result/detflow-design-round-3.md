---
format: aep.planning-md/3
id: review-result:detflow-design-round-3
kind: review-result
status: active
title: 'Plan critic (design), round 3: deterministic delivery workflows'
relations:
- reviews: story:bugfix-workflow
- reviews: story:concurrent-command-steps
- reviews: story:release-step-map
- reviews: story:spend-against-baseline
- reviews: epic:deterministic-delivery-workflows
revision: 1
---
Plan-critic round 3, design lens, on the four remaining stories of
`epic:deterministic-delivery-workflows`, read against wave detflow-2's head (870a25919). Every round 1
and 2 finding is fixed in the text; the new defects come from what shipped (the driver cursor,
`expect: failure`, `output:`, the project file and the release command order).

Verdict: needs-revision. The dependency graph has no cycle and no edge leaving the set except to
implemented siblings. No story needs an ESS release newer than the pinned 0.59.0;
`story:concurrent-command-steps` and `story:spend-against-baseline` change ESS surface and need it
modelled first.

```findings
[
{"file":".engineering/planning/story/concurrent-command-steps.md","line":17,"category":"design","severity":"blocker","verdict":"NEEDS-CHANGE","origin":"pre-existing","message":"the Outcome does not say how a concurrent state meets the cursor's single step pointer and single in_flight attempt (cursor.rs:281,292), nor what an llm or operator step in a concurrent state does; say the command batch is one cursor unit and that llm and operator steps are barriers after every earlier command step"},
{"file":".engineering/planning/story/concurrent-command-steps.md","line":48,"category":"design","severity":"warning","verdict":"NEEDS-CHANGE","origin":"pre-existing","message":"concurrent, id and after are new aep.drive.Step fields and the unknown-after refusal a new outcome, but the scope names no ess/domains/drive.yaml, generated/ess or ess_gate counts; spec first needs them in Scope"},
{"file":".engineering/planning/story/bugfix-workflow.md","line":27,"category":"design","severity":"warning","verdict":"NEEDS-CHANGE","origin":"pre-existing","message":"'with no change to this map' is false once concurrent is opt-in per state; say diagnose gains concurrent: true and nothing else"},
{"file":".engineering/planning/story/bugfix-workflow.md","line":21,"category":"design","severity":"blocker","verdict":"NEEDS-CHANGE","origin":"pre-existing","message":"the collection step 'the reproducing command's output' reruns the command reproduce already ran, and under the default expect: success its nonzero exit fails the state; give reproduce output: repro.txt, name it in diagnose's context and drop the rerun"},
{"file":".engineering/planning/story/release-step-map.md","line":42,"category":"design","severity":"blocker","verdict":"NEEDS-CHANGE","origin":"pre-existing","message":"'no tag created' contradicts the shipped order: version-check needs the tag and pin runs before the gate, so a local tag exists when the gate fails; assign prepare, tag and pin to qualify before the gate and change the acceptance to no tag pushed and nothing pushed"},
{"file":".engineering/planning/story/release-step-map.md","line":22,"category":"design","severity":"warning","verdict":"NEEDS-CHANGE","origin":"pre-existing","message":"the Outcome omits the release branch and commit, the record branch with its status commit and second pull request, and {input.date} for prepare --date, none of which a shipped command performs; list them as ordered command steps and say what 'declares its own release in that map' means"},
{"file":".engineering/planning/story/spend-against-baseline.md","line":16,"category":"design","severity":"blocker","verdict":"NEEDS-CHANGE","origin":"pre-existing","message":"one row per task kind reads a kind no SpendRecord carries (it has artifact, not kind); state that the kind is the kind of the artifact the record names, or add a field to the record"},
{"file":".engineering/planning/story/spend-against-baseline.md","line":48,"category":"design","severity":"warning","verdict":"NEEDS-CHANGE","origin":"pre-existing","message":"the scope names crates/plan/aep-contract but the project file is read in crates/govern/aep-domain/src/project.rs, omits ess/domains/plan.yaml although the verb and the baseline and target keys are new ESS surface, and gives no shape for baseline and target"}
]
```
