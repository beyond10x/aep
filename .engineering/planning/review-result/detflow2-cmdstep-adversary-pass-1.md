---
format: aep.planning-md/3
id: review-result:detflow2-cmdstep-adversary-pass-1
kind: review-result
status: active
title: detflow-2 command step adversary pass 1
relations:
- reviews: story:command-step-expects-failure
revision: 1
---
Adversary pass 1 on wave detflow-2's unit `story:command-step-expects-failure`, tree
`aep-detflow2-cmdstep` on `81832d8cf`.

Header as returned: verdict NEEDS-CHANGE; red 7, all introduced by the unit. Four cases are kept in
`crates/drive/aep-driver-spec/tests/adversary_command_step.rs` and three in the `aep-cli` drive
tests. All seven were fixed in the unit (`9f87c65c5`), with `ess/domains/drive.yaml` modelled
first; the 126/127 rule was decided inside the approved design and is written on
`decision-blocker:approve-detflow-wave-2`.

```findings
[{"file":"crates/drive/aep-driver-spec/src/map.rs","line":1,"category":"property","severity":"warning","verdict":"NEEDS-CHANGE","origin":"introduced","message":"input values were re-expanded as placeholders, so two values could spell {task} and an unclosed { hid {run_directory}"},{"file":"crates/drive/aep-driver-spec/src/map.rs","line":1,"category":"acceptance","severity":"warning","verdict":"NEEDS-CHANGE","origin":"introduced","message":"under expect: failure an exit of 126 or 127 (command not executable or not found) counted as a reproduction"},{"file":"crates/drive/aep-driver-spec/src/map.rs","line":1,"category":"boundary","severity":"warning","verdict":"NEEDS-CHANGE","origin":"introduced","message":"an input value could turn an llm step's context entry into a glob or a blank"},{"file":"crates/drive/aep-driver-spec/src/map.rs","line":1,"category":"boundary","severity":"warning","verdict":"NEEDS-CHANGE","origin":"introduced","message":"reserved run-record output names were refused only in their exact case, not on case-insensitive file systems"},{"file":"crates/drive/aep-driver-spec/src/map.rs","line":1,"category":"boundary","severity":"warning","verdict":"NEEDS-CHANGE","origin":"introduced","message":"an output could take a step log's name and overwrite it"},{"file":"crates/drive/aep-driver-spec/src/map.rs","line":1,"category":"boundary","severity":"warning","verdict":"NEEDS-CHANGE","origin":"introduced","message":"an output name holding a NUL byte was accepted at load"},{"file":"ess/domains/drive.yaml","line":1,"category":"contract-drift","severity":"warning","verdict":"NEEDS-CHANGE","origin":"introduced","message":"the specification did not model the new refusals the implementation made"}]
```
