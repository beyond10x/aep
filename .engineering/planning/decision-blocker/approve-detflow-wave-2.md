---
format: aep.planning-md/3
id: decision-blocker:approve-detflow-wave-2
kind: decision-blocker
status: cleared
title: The second deterministic delivery wave waits for approval
relations:
- blocks: story:command-step-expects-failure
- blocks: story:evidence-records-task-spend
revision: 2
transitions:
- {from: "open", to: "cleared", at: "2026-10-14T19:07:08Z", actor: "human:timo", revision: 2}
---
# Decision: approve the second deterministic delivery wave

Two units of `epic:deterministic-delivery-workflows`, plus the store's opt-in to required findings:

- `story:command-step-expects-failure`: `expect: failure`, run inputs (`--input <name>=<value>`,
  `{input.<name>}`) and `output: <file>` on command steps. Spec first in `ess/domains/drive.yaml`,
  then `aep-driver-spec`, `aep-driver` and `aep drive run --input`.
- `story:evidence-records-task-spend`: the `task_spend` evidence kind and
  `aep plan artifact evidence <id> --kind task_spend --from <file>`, reading a transcript, an
  `eval.run-manifest/1` or a driven run's report into the record `ess/domains/evidence.yaml`
  declares.
- `.engineering/project.yaml` gains `findings_required_since: 2026-10-14`. The key exists since
  0.71.0, and 0.72.0 is released.

Scopes are disjoint except the regenerated `schemas/generated/`, `generated/ess/` and the counts in
`xtask/tests/ess_gate.rs`, which the integration branch regenerates once after both units merge.
`story:evidence-records-task-spend` depended on `story:concurrent-command-steps` only to order those
regenerations; that edge is removed, and its dependencies on `story:task-spend-is-modelled` and
`story:llm-step-declares-judgement` (both implemented) stay.

Width: 2. The epic's other stories (`story:concurrent-command-steps`, `story:bugfix-workflow`,
`story:release-step-map`) need the expected-failure, input and output steps this wave adds, and
`story:spend-against-baseline` needs the spend evidence. The other drafts `aep plan artifact waves`
lists as ready are owned by other planes (eval, harness, protocol), carry open questions that need
a design first, or are obsolete as written.

Package gates of the touched crates per unit; an adversary pass on each unit alongside the build;
one full gate on the integrated head; one pull request; a minor release, since a step map gains
fields and the evidence kind list grows.

## Decided

The open question on `story:evidence-records-task-spend`: the spend record gains an optional
`total_tokens` figure, modelled in `ess/domains/evidence.yaml` before any code. It holds the total
the source states (`tokens` of an `eval.run-manifest/1`), or, for a transcript or a driven run's
report, the sum of the five split figures when all five are known; otherwise it is absent and
printed `unknown`. A manifest's split figures stay absent. This lies inside the epic's approved
measures ("tokens", read as the source states them) and keeps unknown apart from zero.

Approved as proposed: two units in one wave, adversary passes alongside, pull request on green CI,
then a minor release.
