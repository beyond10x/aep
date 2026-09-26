---
format: aep.planning-md/2
id: review-result:u37-adversary-pass-1
kind: review-result
status: active
title: u37 adversary pass 1 (writer fence location)
relations:
- reviews: story:planning-writer-fence-leaves-no-lock-file
revision: 1
---
Adversary pass 1 on wave unit u37 (`story:planning-writer-fence-leaves-no-lock-file`, #37). Tree
`wave-20260925-u37` on `b3087b3bb9` plus the implementor's uncommitted diff.

Header as returned: verdict NEEDS-CHANGE; cases executed 661→667, red 2; origin introduced 2,
pre-existing 0, undecided 0. Added `crates/edge/aep-cli/tests/planning_writer_fence_adversary.rs`.

Coordinator routing: finding 1 back to the implementor (also lock the legacy in-tree names when
they already exist, never creating them). Finding 2 no-op (fail-closed, nothing reaches it).
Acceptance line 3 names `evidence/c-rehearsal.sh`, which is not in this repository; not applicable.

```findings
[{"file":"crates/edge/aep-cli/src/planning_writer_fence.rs","line":133,"category":"acceptance","severity":"warning","verdict":"NEEDS-CHANGE","origin":"introduced","message":"Relocating the fence into the git common dir lets a pre-change aep (0.59.3, installed on PATH) holding the in-tree fence and a new aep write the same store at once, which the story's acceptance forbids; locking the legacy in-tree names when they already exist closes it."},{"file":"crates/edge/aep-cli/src/planning_writer_fence.rs","line":123,"category":"judgement","severity":"note","verdict":"INFEASIBLE","origin":"introduced","message":"A read-only .git now refuses every planning write that succeeded on the base; the refusal is fail-closed and mutates nothing, but no configuration on this machine was shown to reach it."}]
```
