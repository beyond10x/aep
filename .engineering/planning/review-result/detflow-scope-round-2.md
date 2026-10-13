---
format: aep.planning-md/3
id: review-result:detflow-scope-round-2
kind: review-result
status: active
title: 'Plan critic (scope), round 2: deterministic delivery workflows'
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
approve

**What I read.** 10 artifacts, with `aep plan artifact show` run from the managed worktree on the parent epic, `review-result:detflow-scope-round-1`, and the nine drafted stories. I read the parent first and listed 17 promises: 9 Scope bullets, 3 Done When bullets, and 5 table rows or measures with a stated replacement. I traced all 17 to a story. I rechecked each of the five round-1 findings against the revised bodies, and all five hold as fixed:

- **Record step:** `story:release-step-map` now lists "verify, then record the release in the planning store". Its acceptance requires "the record step included", and `story:release-cutting-commands` provides `record --run <url>`.
- **Done-when for this repository:** `story:release-step-map` now says "This repository declares its own release in that map. Its first release cut that way is the evidence for the epic's Done When". The parent's Done When says the check happens at epic close.
- **LLM steps per task:** `story:task-spend-is-modelled` now records the llm steps declared and run. `story:evidence-records-task-spend` yields them from a driven run's report.
- **Baseline:** the parent now defines it as the median of the first recorded tasks, copied by a person. `story:spend-against-baseline` matches this with `--baseline <n>` and "the verb never writes the baseline itself".
- **Diagnose collection:** `story:bugfix-workflow` now declares command steps that collect state before the one llm step. They run in sequence until `story:concurrent-command-steps` lands, which covers running them at once.

I found no reach beyond the parent. Every story's outcome traces to a Scope bullet or table row, and none lands in an Out of Scope item. The new `after:` field in `story:concurrent-command-steps` and the run inputs in `story:command-step-expects-failure` are named by the parent. I found no two stories claiming one outcome, and no silent narrowing. I did not run `aep plan artifact graph` this round, because the round-1 grep of the store for other artifacts on these outcomes found none.

**What I could not establish.**
- The parent's table also marks wave selection, scope citing and adversary mutation as "yes" or "partly" deterministic. The parent's Scope now excludes them by name as later stories, so I did not count them as gaps.
- Out of my lane, for the acceptance critic: the Done When items for the bug fix and for the report "for at least one task kind" are proved only over fixture stores and fixtures. They are not proved over a real task.

```findings
[]
```
