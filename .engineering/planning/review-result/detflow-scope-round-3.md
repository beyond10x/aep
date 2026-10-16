---
format: aep.planning-md/3
id: review-result:detflow-scope-round-3
kind: review-result
status: active
title: 'Plan critic (scope), round 3: deterministic delivery workflows'
relations:
- reviews: story:bugfix-workflow
- reviews: story:concurrent-command-steps
- reviews: story:release-step-map
- reviews: story:spend-against-baseline
- reviews: epic:deterministic-delivery-workflows
revision: 1
---
Plan-critic round 3, scope lens, on the four remaining stories of
`epic:deterministic-delivery-workflows`, read against wave detflow-2's head (870a25919). All 13
promises extracted from the epic trace to an item; no gap and no double claim. Rounds 1 and 2 held;
the new defects come from what wave 2 shipped.

Verdict: needs-revision. Out of lane, noted: stale line citations in the concurrent and spend stories
(`map.rs`, `run.rs`, `planning.rs`) should be re-cited by symbol.

```findings
[
 {"file":".engineering/planning/story/concurrent-command-steps.md","line":48,"category":"scope","severity":"blocker","verdict":"NEEDS-CHANGE","origin":"pre-existing","message":"Scope names no ESS change although the story adds state `concurrent`, step `id`/`after` and an unknown-`after` refusal; add ess/domains/drive.yaml, generated/ess/ and the ess_gate.rs counts to Scope, specified first"},
 {"file":".engineering/planning/story/spend-against-baseline.md","line":43,"category":"scope","severity":"blocker","verdict":"NEEDS-CHANGE","origin":"pre-existing","message":"Scope names no ESS change although the story adds the `aep plan artifact spend` verb, project.yaml baseline/target keys and a validate refusal; add ess/domains/plan.yaml and evidence.yaml to Scope, specified first"},
 {"file":".engineering/planning/story/spend-against-baseline.md","line":48,"category":"scope","severity":"warning","verdict":"NEEDS-CHANGE","origin":"pre-existing","message":"the project.yaml schema is read in crates/govern/aep-domain/src/project.rs and schemas/generated/project.schema.json, not crates/plan/aep-contract; correct the Scope line"},
 {"file":".engineering/planning/story/bugfix-workflow.md","line":28,"category":"scope","severity":"warning","verdict":"NEEDS-CHANGE","origin":"pre-existing","message":"'with no change to this map' contradicts the per-state `concurrent: true` opt-in, and no story claims adding it to the diagnose state; say so and give it an owner"},
 {"file":".engineering/planning/story/bugfix-workflow.md","line":48,"category":"scope","severity":"warning","verdict":"NEEDS-CHANGE","origin":"pre-existing","message":"Scope omits generated/instructions/bugfix/*.md, which crates/edge/aep-cli/tests/instructions.rs requires for every workflow the tree declares"},
 {"file":".engineering/planning/story/release-step-map.md","line":48,"category":"scope","severity":"warning","verdict":"NEEDS-CHANGE","origin":"pre-existing","message":"Scope lists workflows/releases/source.yaml as changed but no Outcome or Acceptance line says what changes; remove it, or state the edit and add generated/instructions/release/source.md to Scope"},
 {"file":".engineering/planning/story/concurrent-command-steps.md","line":18,"category":"scope","severity":"warning","verdict":"NEEDS-CHANGE","origin":"pre-existing","message":"step `id` and `after` ordering is not asked for by the epic's 'Concurrent command steps, opted into per state' and no epic loop needs it; drop it and its two acceptance bullets, or cite a parent sentence"}
]
```
