---
format: aep.planning-md/3
id: review-result:detflow-parallel-safety-round-3
kind: review-result
status: active
title: 'Plan critic (parallel-safety), round 3: deterministic delivery workflows'
relations:
- reviews: story:bugfix-workflow
- reviews: story:concurrent-command-steps
- reviews: story:release-step-map
- reviews: story:spend-against-baseline
- reviews: epic:deterministic-delivery-workflows
revision: 1
---
Plan-critic round 3, parallel-safety lens, on the four remaining stories of
`epic:deterministic-delivery-workflows`, read against wave detflow-2's head (870a25919).
`generated/ess`, `schemas/generated` and the `xtask/tests/ess_gate.rs` counts are regenerated once on
the integration branch and are not counted as overlap.

Verdict: needs-revision.

Pairs (C concurrent-command-steps, B bugfix-workflow, R release-step-map, S spend-against-baseline):
C and B share `drivers/bugfix/*.yaml` (C's `concurrent: true` opt-in); C and R share nothing; B and R
share only unnamed new tests under `crates/drive/aep-driver/tests/` and `generated/instructions/`; S
shares nothing with the others beyond the routine `CHANGELOG.md` append.

```findings
[
{"file":".engineering/planning/story/bugfix-workflow.md","line":28,"category":"parallel-safety","severity":"blocker","verdict":"NEEDS-CHANGE","origin":"pre-existing","message":"'no change to this map' is false once story:concurrent-command-steps lands: the diagnose state must opt in with concurrent: true, an edit to drivers/bugfix/*.yaml that neither scope names; say the second to land makes that edit, or add an ordering edge recording the file"},
{"file":".engineering/planning/story/concurrent-command-steps.md","line":48,"category":"parallel-safety","severity":"warning","verdict":"NEEDS-CHANGE","origin":"pre-existing","message":"scope omits ess/domains/drive.yaml (aep.drive.Step gains concurrent, id, after; spec first) and website/docs/reference/documents.md; add both"},
{"file":".engineering/planning/story/release-step-map.md","line":49,"category":"parallel-safety","severity":"warning","verdict":"NEEDS-CHANGE","origin":"pre-existing","message":"both this and story:bugfix-workflow create an unnamed new test file under crates/drive/aep-driver/tests/; name distinct files so neither extends driving.rs"},
{"file":".engineering/planning/story/release-step-map.md","line":45,"category":"parallel-safety","severity":"warning","verdict":"NEEDS-CHANGE","origin":"pre-existing","message":"scope omits generated/instructions/release/source.md, which tests/instructions.rs compares to a fresh render; story:bugfix-workflow adds generated/instructions/bugfix and a README line likewise; list generated/instructions in both or regenerate once on the integration branch"},
{"file":".engineering/planning/story/spend-against-baseline.md","line":48,"category":"parallel-safety","severity":"warning","verdict":"NEEDS-CHANGE","origin":"pre-existing","message":"project.yaml schema is cited as crates/plan/aep-contract but ProjectConfig is in crates/govern/aep-domain/src/project.rs:349; replace the path and add ess/domains/evidence.yaml (spec first) to the scope"}
]
```
