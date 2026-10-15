---
format: aep.planning-md/3
id: review-result:detflow2-spend-adversary-pass-1
kind: review-result
status: active
title: detflow-2 task spend adversary pass 1
relations:
- reviews: story:evidence-records-task-spend
revision: 1
---
Adversary pass 1 on wave detflow-2's unit `story:evidence-records-task-spend`, tree
`aep-detflow2-spend` on `4e2c1197e`.

Header as returned: verdict NEEDS-CHANGE; red 9, all introduced by the unit. The cases are kept as
`crates/edge/aep-cli/tests/evidence_task_spend_adversary.rs`. All nine were fixed in the unit
(`7f88671c1`); the total-tokens rule they raised was decided inside the approved design and is
written on `decision-blocker:approve-detflow-wave-2`.

```findings
[{"file":"crates/edge/aep-cli/src/planning.rs","line":1,"category":"boundary","severity":"warning","verdict":"NEEDS-CHANGE","origin":"introduced","message":"a source reached through a directory link out of the repository was recorded instead of refused"},{"file":"crates/edge/aep-cli/src/planning.rs","line":1,"category":"boundary","severity":"warning","verdict":"NEEDS-CHANGE","origin":"introduced","message":"a .. after a link names a file other than the one the record read and digested"},{"file":"crates/edge/aep-cli/src/planning.rs","line":1,"category":"property","severity":"warning","verdict":"NEEDS-CHANGE","origin":"introduced","message":"a total_cost_usd written as text was recorded as an unknown cost instead of refused"},{"file":"crates/edge/aep-cli/src/planning.rs","line":1,"category":"property","severity":"warning","verdict":"NEEDS-CHANGE","origin":"introduced","message":"a session that states no cost did not leave the task's cost unknown"},{"file":"crates/edge/aep-cli/src/planning.rs","line":1,"category":"contract-drift","severity":"warning","verdict":"NEEDS-CHANGE","origin":"introduced","message":"a transcript's total_tokens differed from the tokens its run manifest states (thinking tokens added)"},{"file":"crates/edge/aep-cli/src/planning.rs","line":1,"category":"property","severity":"warning","verdict":"NEEDS-CHANGE","origin":"introduced","message":"a session with no terminal record did not leave the task's figures unknown"},{"file":"crates/edge/aep-cli/src/planning.rs","line":1,"category":"acceptance","severity":"warning","verdict":"NEEDS-CHANGE","origin":"introduced","message":"--kind task_spend without --from was not refused before the store was opened"},{"file":"crates/edge/aep-cli/src/planning.rs","line":1,"category":"boundary","severity":"warning","verdict":"NEEDS-CHANGE","origin":"introduced","message":"a JSON file no reader recognises was recorded as a transcript of nothing"},{"file":"crates/edge/aep-cli/src/planning.rs","line":1,"category":"property","severity":"warning","verdict":"NEEDS-CHANGE","origin":"introduced","message":"a token figure that overflows across sessions was recorded saturated instead of unknown"}]
```
