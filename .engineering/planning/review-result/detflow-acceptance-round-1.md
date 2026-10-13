---
format: aep.planning-md/3
id: review-result:detflow-acceptance-round-1
kind: review-result
status: active
title: 'Plan critic (acceptance), round 1: deterministic delivery workflows'
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
story:release-step-map — the acceptance "A failing gate step routes back as the workflow declares, and pushes nothing" names no destination state, and `workflows/releases/source.yaml` declares no transition out of `qualify` on a failed gate, so the sentence can be met by any behaviour — .engineering/planning/story/release-step-map.md:33
story:spend-against-baseline — the Outcome promises "whether the median is under the target", but no acceptance bullet checks that verdict, so the table can print medians with no under/over column and every bullet still passes — .engineering/planning/story/spend-against-baseline.md:28
story:llm-step-declares-judgement — the acceptance names `aep drive status <run>`, but `StatusArgs` takes the run as `--run`, with no positional, so the named command form is not one a person can run — .engineering/planning/story/llm-step-declares-judgement.md:31
story:concurrent-command-steps — the outcome says results are listed in map order "whatever order they finish in", but the fixture is three equal one-second steps and no step is made to finish out of map order, so byte-identical output across ten runs cannot tell sorted results from completion order — .engineering/planning/story/concurrent-command-steps.md:30

What I read: 8 of 8 ids (the epic and the 7 stories), each in full with `aep plan artifact show`, plus `aep plan artifact kinds` and `aep plan artifact lifecycle story`. In the tree I read `workflows/releases/source.yaml`, `crates/edge/aep-cli/src/drive.rs` (`StatusArgs` and `status`), `eval.rs:2795-2820`, `.engineering/project.yaml`, and `story:a-session-is-one-spend`.

What I could not establish:
- Whether `.engineering/project.yaml` (a closed `aep.project/5` document with no baseline or target keys today) can take them. That is the schema's job, not the acceptance's, so I did not count it.
- Out of my lane: `story:release-step-map` lists version, changelog, status, PR and merge steps that `workflows/releases/source.yaml` has no states for (scope or design).
- Out of my lane: `story:spend-against-baseline` and the epic describe the baseline differently. The epic calls it the first recorded tasks, while the story reads it from project config (design or scope).
- I left multi-bullet acceptance sections unflagged, since the store's existing stories use them.

```findings
- file: .engineering/planning/story/release-step-map.md
  line: 33
  category: acceptance
  severity: blocker
  verdict: needs-revision
  origin: introduced
  message: "the acceptance 'A failing gate step routes back as the workflow declares, and pushes nothing' names no destination state, and workflows/releases/source.yaml declares no transition out of qualify on a failed gate, so the sentence can be met by any behaviour"
- file: .engineering/planning/story/spend-against-baseline.md
  line: 28
  category: acceptance
  severity: warning
  verdict: needs-revision
  origin: introduced
  message: "the Outcome promises 'whether the median is under the target', but no acceptance bullet checks that verdict, so the table can print medians with no under/over column and every bullet still passes"
- file: .engineering/planning/story/llm-step-declares-judgement.md
  line: 31
  category: acceptance
  severity: warning
  verdict: needs-revision
  origin: introduced
  message: "the acceptance names `aep drive status <run>`, but StatusArgs takes the run as `--run` with no positional, so the named command form is not one a person can run"
- file: .engineering/planning/story/concurrent-command-steps.md
  line: 30
  category: acceptance
  severity: warning
  verdict: needs-revision
  origin: introduced
  message: "the outcome says results are listed in map order whatever order they finish in, but the fixture is three equal one-second steps and no step is made to finish out of map order, so byte-identical output across ten runs cannot tell sorted results from completion order"
```
