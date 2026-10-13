---
format: aep.planning-md/3
id: review-result:detflow-parallel-safety-round-1
kind: review-result
status: active
title: 'Plan critic (parallel-safety), round 1: deterministic delivery workflows'
relations:
- reviews: epic:deterministic-delivery-workflows
- reviews: story:task-spend-is-modelled
- reviews: story:evidence-records-task-spend
- reviews: story:spend-against-baseline
- reviews: story:llm-step-declares-judgement
- reviews: story:release-step-map
- reviews: story:bugfix-workflow
- reviews: story:concurrent-command-steps
revision: 1
---
needs-revision
story:llm-step-declares-judgement — it and story:concurrent-command-steps both land on `crates/edge/aep-cli/src/drive.rs` (cited in both bodies: the `aep drive status` counts here, the command host there) and neither body names the other nor is an ordering edge recorded; add an ordering edge that records the shared file as its reason, or split the surface so they no longer share it — .engineering/planning/story/llm-step-declares-judgement.md:38, .engineering/planning/story/concurrent-command-steps.md:35
story:evidence-records-task-spend — adding the `task_spend` kind to the closed list (`evidence.rs:1309-1330`) changes the evidence-kind enum that `schemas/generated/driver-steps.schema.json` publishes, which `story:llm-step-declares-judgement` also regenerates, and this scope lists no `schemas/generated/` file; the schema side is cited and the evidence-records side is inferred, so name the file and either order the pair or split the surface — .engineering/planning/story/evidence-records-task-spend.md:40, .engineering/planning/story/llm-step-declares-judgement.md:32
story:concurrent-command-steps — its acceptance orders steps by `depends_on`, but `map.rs:544-569` shows that field is a named outside dependency keyed by a circuit breaker, not a step reference, so the story needs a new or changed map field its scope does not list; `crates/drive/aep-driver-spec/src/map.rs` and `schemas/generated/driver-steps.schema.json` are then shared with `story:llm-step-declares-judgement` (inferred), so name both files and either order the pair or split the surface — .engineering/planning/story/concurrent-command-steps.md:29, crates/drive/aep-driver-spec/src/map.rs:544

**What you read.** Eight artifacts: the epic and the seven stories, via `aep plan artifact show` for each and `aep plan artifact graph` for the edges. I also checked the cited lines in `map.rs`, `evidence.rs` and `drive.rs`, the `drivers/` and `workflows/` trees, the aep-driver tests, `website/docs/reference/cli.md` and `schemas/generated/driver-steps.schema.json`.

Surfaces established:
- 4 cited (`task-spend-is-modelled`, `llm-step-declares-judgement`, `release-step-map`, `concurrent-command-steps`).
- 3 inferred (`evidence-records-task-spend` beyond the `evidence` verb, `spend-against-baseline`, `bugfix-workflow`).
- 0 unplaceable.

Reader and producer: 4 acceptance reads traced to a producer in the set, and all 4 have a `depends_on` edge.
- `evidence-records-task-spend` reads the record from `task-spend-is-modelled`.
- `spend-against-baseline` reads the `task_spend` records from `evidence-records-task-spend`.
- `bugfix-workflow` reads `judgement:` from `llm-step-declares-judgement`.
- The fourth is the evidence-records read of the modelled record, counted with the first.

**What you could not establish**
- Where `release-step-map` and `bugfix-workflow` put their "driver tests". Both name only `crates/drive/aep-driver/tests/`, a directory, and `concurrent-command-steps` and `release-step-map` may land there too. If they create separate new files there is no collision, and I did not report it.
- How `release-step-map` declares its forge stub (the body marks it inferred). If the stub is in the command host, it touches `drive.rs` too.
- Where `llm-step-declares-judgement` records the "llm steps run" count. Its scope names no `run.rs` or cursor file, and `concurrent-command-steps` rewrites `run.rs`.
- Every story is user-visible and so adds a `CHANGELOG.md` Unreleased entry, which is a routine append and not reported.
- Outside my lane, the design and acceptance critics should check whether `depends_on` is the right field for step ordering (see the third finding).
- Outside my lane, `evidence-records-task-spend` cites `story:a-session-is-one-spend`, which is outside this set.

```findings
- file: .engineering/planning/story/llm-step-declares-judgement.md
  line: 38
  category: parallel-safety
  severity: blocker
  verdict: needs-revision
  origin: introduced
  message: "it and story:concurrent-command-steps both land on crates/edge/aep-cli/src/drive.rs (cited in both bodies) and neither names the other nor is an ordering edge recorded; add an ordering edge that records the shared file as its reason, or split the surface so they no longer share it"
- file: .engineering/planning/story/evidence-records-task-spend.md
  line: 40
  category: parallel-safety
  severity: blocker
  verdict: needs-revision
  origin: introduced
  message: "adding the task_spend kind to the closed list (evidence.rs:1309-1330) changes the evidence-kind enum in schemas/generated/driver-steps.schema.json, which story:llm-step-declares-judgement also regenerates, and this scope lists no schemas/generated file; the schema side is cited and the evidence-records side is inferred, so name the file and either order the pair or split the surface"
- file: .engineering/planning/story/concurrent-command-steps.md
  line: 29
  category: parallel-safety
  severity: blocker
  verdict: needs-revision
  origin: introduced
  message: "its acceptance orders steps by depends_on, but map.rs:544-569 shows that field is a named outside dependency keyed by a circuit breaker, not a step reference, so the story needs a new or changed map field its scope does not list; crates/drive/aep-driver-spec/src/map.rs and schemas/generated/driver-steps.schema.json are then shared with story:llm-step-declares-judgement (inferred), so name both files and either order the pair or split the surface"
```
