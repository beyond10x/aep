---
format: aep.planning-md/3
id: review-result:detflow-acceptance-round-2
kind: review-result
status: active
title: 'Plan critic (acceptance), round 2: deterministic delivery workflows'
relations:
- reviews: epic:deterministic-delivery-workflows
- reviews: story:task-spend-is-modelled
- reviews: story:evidence-records-task-spend
- reviews: story:spend-against-baseline
- reviews: story:llm-step-declares-judgement
- reviews: story:command-step-expects-failure
- reviews: story:release-cutting-commands
- reviews: story:release-step-map
- reviews: story:bugfix-workflow
- reviews: story:concurrent-command-steps
revision: 1
---
needs-revision
story:release-cutting-commands — the Outcome lists `tag` and `pin` as subcommands that are each idempotent and refuse with a named reason, but no acceptance bullet exercises either, so only `prepare` and `record` are checked and the last bullet is met by any real release that used them; add a fixture bullet for `tag` (annotated tag at HEAD, second run changes nothing, refusal naming the reason) and one for `pin` — .engineering/planning/story/release-cutting-commands.md:31-39

What I read: 9 of 9 drafted stories plus the epic and review-result:detflow-acceptance-round-1, each in full with `aep plan artifact show`. In the tree I read `workflows/releases/source.yaml`, `StatusArgs` in `crates/edge/aep-cli/src/drive.rs:323`, and `run.rs` for blocked status and duration fields.

Round 1 findings, each checked against the revised text:
- **release-step-map:** fixed. The acceptance now says "leaves the run at `qualify`, reported blocked, with no tag created and nothing pushed: `publish` is never entered". That is checkable, because `qualify` and `publish` exist in the workflow and `run.rs:939-944` returns `RunStatus::Blocked`.
- **spend-against-baseline:** fixed. There is an `under`/`over`/`unknown` column, with the 12 versus 10 and 12 versus 15 cases in the acceptance.
- **llm-step-declares-judgement:** fixed. The acceptance now uses `aep drive status --run <run>`, which matches `StatusArgs`.
- **concurrent-command-steps:** fixed. The 3, 1 and 2 second fixture finishes in under 4 seconds and is listed in map order. Completion order would be 1, 2, 3, so the two orders can be told apart.

What I could not establish:
- Whether the "under 4 seconds" bound is stable on a loaded CI machine. It is observable, so I did not count it.
- Out of my lane (design or scope): `story:release-step-map` lists version, changelog, status, PR and merge steps, but `workflows/releases/source.yaml` has no states for them.
- Out of my lane (design or scope): the epic describes the baseline as the first recorded tasks, while `story:spend-against-baseline` reads it from project config.

```findings
- file: .engineering/planning/story/release-cutting-commands.md
  line: 31
  category: acceptance
  severity: warning
  verdict: needs-revision
  origin: introduced
  message: "the Outcome lists `tag` and `pin` as subcommands that are each idempotent and refuse with a named reason, but no acceptance bullet exercises either, so only `prepare` and `record` are checked and the last bullet is met by any real release that used them"
```
