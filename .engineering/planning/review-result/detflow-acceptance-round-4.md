---
format: aep.planning-md/3
id: review-result:detflow-acceptance-round-4
kind: review-result
status: active
title: 'Plan critic (acceptance), round 4: deterministic delivery workflows'
relations:
- reviews: epic:deterministic-delivery-workflows
- reviews: story:concurrent-command-steps
- reviews: story:bugfix-workflow
- reviews: story:release-step-map
- reviews: story:spend-against-baseline
revision: 1
---
needs-revision. All twelve round 3 acceptance findings are fixed: the concurrent story now declares the ESS change first, makes llm and operator steps barriers and drops id/after, and gives the shared-dependency bullet a threshold and an expected log and note (this matches `Session::step` and the `<state>-<step>-<attempt>.log` naming). The bugfix story opts diagnose into `concurrent: true`, names the blocked observable and the `--input` fixture values. The release story now stops at `--pause-on-approval` with a recorded human approval and has operator merge steps. The spend story defines task kind, the project.yaml shape, the median rule, the filters and the ESS scope. Four new defects remain, two of which make an acceptance bullet unsatisfiable as written. The release story's gate-failure bullet contradicts its own table: the driver runs every remaining step of a state before it asks the engine to move (`next_step` in route.rs, `submit` in run.rs), so the `release-inputs` check after the gate is recorded. The concurrent story's determinism bullet cannot hold because every evidence record carries a wall-clock `observed_at` from `observed_now()`. The other two new findings are a release argv list that cannot be written from the table, and concurrent outcome clauses with no acceptance. The bugfix and spend stories have no new acceptance defects.

What I read: 4 of 4 given ids in full via `aep plan artifact show`, plus review-result:detflow-acceptance-round-3, the release/source workflow, and the driver code the acceptances name (run.rs, route.rs, map.rs `verdict`, drive.rs).

What I could not establish: the bugfix story does not name the evidence kinds its steps mint or the guards on its transitions (reproduce to diagnose, verify back to fix), though its blocked and verify-to-fix observables depend on them; that is design and scope, out of my lane. I did not run a driven fixture, so the `observed_at` finding rests on reading the evidence envelope and `observed_now()`, not on a diff of two snapshots.

```findings
[
{"file":".engineering/planning/story/release-step-map.md","line":86,"category":"acceptance","severity":"blocker","verdict":"needs-revision","origin":"introduced","message":"the gate-failure bullet says 'no argv after the gate is recorded', but the table puts the release-inputs check after the gate in qualify and the driver runs every remaining step of a state after a failing verdict (route.rs next_step, run.rs submit), so that argv is recorded and the bullet cannot pass; say that no argv of publish is recorded, or move the check before the gate"},
{"file":".engineering/planning/story/release-step-map.md","line":76,"category":"acceptance","severity":"warning","verdict":"needs-revision","origin":"introduced","message":"'the recorded argv list equals the command steps of the table above' cannot be written for the rows that hold two commands (fetch then check origin/main; build then task check; record then a bot commit; status then a bot commit) because the story does not say whether each is one step (an sh -c argv) or two, nor the bot commit's argv or the build target path"},
{"file":".engineering/planning/story/concurrent-command-steps.md","line":80,"category":"acceptance","severity":"blocker","verdict":"needs-revision","origin":"introduced","message":"'Ten runs of the concurrency fixture give byte-identical snapshot.json' cannot pass: each step's evidence is minted with the wall-clock observed_at from observed_now() (drive.rs:1902) and carries an evidence id, so the bytes differ between runs; compare the snapshot with those fields removed, or compare the order of evidence only"},
{"file":".engineering/planning/story/concurrent-command-steps.md","line":24,"category":"acceptance","severity":"warning","verdict":"needs-revision","origin":"introduced","message":"the Outcome states that an interruption before the batch ends submits nothing and --retry-in-flight reruns the batch, that a step's retries are spent inside its batch, and that an operator step is a barrier, but the Acceptance holds no bullet for any of the three"}
]
```
