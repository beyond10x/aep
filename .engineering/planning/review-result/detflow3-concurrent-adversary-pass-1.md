---
format: aep.planning-md/3
id: review-result:detflow3-concurrent-adversary-pass-1
kind: review-result
status: active
title: detflow-3 concurrent adversary pass 1
relations:
- reviews: story:concurrent-command-steps
revision: 1
---
Adversary pass 1 on wave detflow-3's unit `story:concurrent-command-steps`, tree
`aep-df3-concurrent` on `13302200a`.

Header as returned: verdict NEEDS-CHANGE; every finding introduced by the unit. The red cases
are kept as `crates/drive/aep-driver/tests/adversary_concurrent.rs`. All but one were
fixed in the unit (`ad7a683e1`): attempts are charged per round after the circuit breaker is
read, the cursor records `batch_charged` and `batch_ended` so a resume after a budget stop never
resubmits a sibling, the in-flight marker is rewritten to the step actually in flight, two
outputs differing only in case are one file, and the determinism test strips only minted ids and
times. One stays open and is named on the wave's pull request: a command executor that
returns `Paused` inside a batch (possible only with a harness-supplied executor) leaves the
cursor one step past the paused step.

```findings
[{"file":"crates/drive/aep-driver/src/run.rs","line":1,"category":"property","severity":"warning","verdict":"NEEDS-CHANGE","origin":"introduced","message":"a step skipped by an open circuit inside a batch was charged an attempt, unlike a serial run (fixed)"},{"file":"crates/drive/aep-driver/src/run.rs","line":1,"category":"property","severity":"warning","verdict":"NEEDS-CHANGE","origin":"introduced","message":"a step left behind a spent sibling on its lane was charged an attempt (fixed)"},{"file":"crates/drive/aep-driver-spec/src/cursor.rs","line":1,"category":"property","severity":"warning","verdict":"NEEDS-CHANGE","origin":"introduced","message":"resuming after a batch spent its budget reran a sibling already submitted (fixed)"},{"file":"crates/drive/aep-driver/src/run.rs","line":1,"category":"contract-drift","severity":"warning","verdict":"NEEDS-CHANGE","origin":"introduced","message":"the in-flight marker named the batch's first step instead of the step actually in flight (fixed)"},{"file":"crates/drive/aep-driver/src/run.rs","line":1,"category":"property","severity":"note","verdict":"NEEDS-CHANGE","origin":"introduced","message":"a command returning Paused inside a batch leaves the cursor one step past it (open; harness executors only)"},{"file":"crates/drive/aep-driver-spec/src/map.rs","line":1,"category":"boundary","severity":"warning","verdict":"NEEDS-CHANGE","origin":"introduced","message":"two outputs differing only in case were admitted as distinct files in a concurrent state (fixed)"}]
```
