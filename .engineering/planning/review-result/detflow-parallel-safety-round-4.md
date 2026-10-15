---
format: aep.planning-md/3
id: review-result:detflow-parallel-safety-round-4
kind: review-result
status: active
title: 'Plan critic (parallel-safety), round 4: deterministic delivery workflows'
relations:
- reviews: epic:deterministic-delivery-workflows
- reviews: story:concurrent-command-steps
- reviews: story:bugfix-workflow
- reviews: story:release-step-map
- reviews: story:spend-against-baseline
revision: 1
---
approve

Plan-critic round 4, parallel-safety lens, on story:concurrent-command-steps (C), story:bugfix-workflow (B), story:release-step-map (R) and story:spend-against-baseline (S) of epic:deterministic-delivery-workflows, read in the tree aep-detflow3 (head 56100faa6, release 0.73.0). All five round-3 findings are fixed. B now has a depends_on edge to C and says it lands second so the map ships with `concurrent: true` (bugfix-workflow.md "Depends on" and Outcome). C's scope now lists ess/domains/drive.yaml and website/docs/reference/documents.md. R's test is `release_source_map.rs`, B's is `bugfix_workflow.rs`, and C's is `concurrent.rs`; all are new and distinct, and none extends `driving.rs`. R now says `generated/instructions/` is unchanged, which holds: `aep govern workflow instruct` overlays a map only under `--map` with `--id` (render.rs `instruct`), so a new `drivers/releases/source.yaml` does not alter `generated/instructions/release/source.md`. B alone owns `generated/instructions/bugfix/standard.md` and the README line. S's path is now `crates/govern/aep-domain/src/project.rs` and its scope lists ess/domains/evidence.yaml. I found no new collision.

Pair table (cited unless marked; CHANGELOG.md, generated/ess, schemas/generated and the ess_gate.rs counts excluded as instructed):

| pair | shared surface | outcome |
|---|---|---|
| C - B | `drivers/bugfix/standard.yaml` is B's new file; B reads C's `concurrent` opt-in | ordered by B depends_on C, which records the reason; not concurrent |
| C - R | none (C: ess/domains/drive.yaml, aep-driver-spec map.rs/cursor.rs, aep-driver executor.rs/run.rs, aep-cli drive.rs, documents.md, tests concurrent.rs and drive_cli.rs; R: drivers/releases/source.yaml, AGENTS.md, tests release_source_map.rs). R's scripted executor implements `CommandStepExecutor`, and C adds only a default method, so R builds unchanged | independent |
| C - S | none (S: ess plan.yaml and evidence.yaml, project.rs, aep-cli planning.rs and planning/spend.rs, cli.md; same crate aep-cli but different files) | independent |
| B - R | none (workflows/bugfix and drivers/bugfix against drivers/releases; distinct test files; generated/instructions is B's alone) | independent |
| B - S | none | independent |
| R - S | none | independent |

Reader beside producer: the one reader-producer pair inside the set is B reading C's `concurrent: true`, and a depends_on edge records it (1 of 1). Reads of producers outside the set (the `judgement:`, `expect: failure` and `--input` stories, release-cutting-commands, evidence-records-task-spend) are 6 traced, all with depends_on edges, and all those producers are `implemented` in this tree.

Startable together now: C, R and S. B starts only after C has landed on the integration branch.

What I read: the four stories, round 3's review-result, the four dependency stories' status, and the code and tests behind each shared surface (`git grep` over crates, xtask, ess, website, drivers, workflows). Surfaces established: 4 cited (B's scope heading says "Inferred", but every path is named in its body and is a new file), 0 inferred, 0 unplaced. Acceptance reads traced to a producer in the set: 1, with 1 recorded by a depends_on edge.

What I could not establish, and out of my lane (none of it sets the verdict): (1) B's scope omits `crates/govern/aep-engine/tests/documents.rs:61`, which asserts `registry.workflows().count() == 6`, so adding `bugfix/standard` turns it red; no other story in the set touches it, so it is a scope completeness point, not a collision. (2) S's scope omits `website/docs/reference/project-file.md`, where the project keys are documented; again S alone, so it is not a collision. (3) B's step runs `aep` from PATH, so which binary a test uses is an acceptance matter.

```findings
[]
```
