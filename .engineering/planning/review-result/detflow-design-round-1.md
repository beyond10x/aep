---
format: aep.planning-md/3
id: review-result:detflow-design-round-1
kind: review-result
status: active
title: 'Plan critic (design), round 1: deterministic delivery workflows'
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

story:bugfix-workflow — its reproduce step is "a command step that passes when the named reproducing command fails", but the step map has no inverted-verdict or expected-failure mapping and its only placeholders are `run_directory`, `task` and `transcript`, so a per-run reproducing command cannot be named; no item in the set adds that capability and its scope lists no `aep-driver-spec` change, so a story that adds an expected-failure step and names the reproducing command must be added with a `depends_on` edge from this story — .engineering/planning/story/bugfix-workflow.md:18, crates/drive/aep-driver-spec/src/map.rs:690-718, crates/drive/aep-driver-spec/src/map.rs:544-569
story:release-step-map — its outcome is that `aep drive run` cuts a release (version, changelog heading, tag, status), yet no command that performs those steps exists, because the only release verb in `xtask` is the check-only `Release`, and no item in the set adds one, since this scope is YAML and driver tests only, so a story producing the cutting commands in Rust must be added with a `depends_on` edge from this one — .engineering/planning/story/release-step-map.md:15-17, .engineering/planning/story/release-step-map.md:28-30, xtask/src/main.rs:70-73
story:concurrent-command-steps — it defines ordering by "`depends_on` between them" and says "a step with `depends_on` starts only after the step it names has ended", but on a command step `depends_on` is the name of an outside dependency keyed by the circuit breaker, so the body must name a distinct step-ordering field and say how concurrent steps share a breaker's failure count — .engineering/planning/story/concurrent-command-steps.md:15, crates/drive/aep-driver-spec/src/map.rs:376-387, crates/drive/aep-driver/src/run.rs:983-1028

What I read: 8 artifacts (the epic and 7 stories) with `aep plan artifact show`, plus `relations` and `graph`. I ran `validate` once and it printed `valid`. I read `map.rs`, `run.rs` and `xtask/src/main.rs` in the worktree, and `workflows/releases/source.yaml`.

Graph and acceptance reads: I walked all 14 edges in the graph for this set, including the epic's `serves` and `informed_by` edges that leave it. I found no cycle and no serialising chain, since there are two short chains of two and three items. The ESS-first story followed by the evidence story and the report story is a spec-then-implement order that is recorded and demonstrable, not a horizontal slice. I traced 5 acceptance reads to a producer in the set: the `task_spend` record, the ESS record type, the `judgement:` field, and the two driver-map reads. Two have an edge. The three without one are the findings above. `story:a-session-is-one-spend` is an existing implemented artifact outside the set.

Out of lane, not setting the verdict:
- `story:spend-against-baseline` says the project declares the baseline in `.engineering/project.yaml`, while the epic says a baseline is the measure of the first recorded tasks. That is scope.
- `story:task-spend-is-modelled` omits "llm steps run" from the record, although the epic lists it as a measure. That is scope.
- The stubbing mechanism for forge calls and model steps is marked "Inferred" in two stories. Whether it exists is acceptance and parallel-safety.

```findings
- file: .engineering/planning/story/bugfix-workflow.md
  line: 18
  category: design
  severity: blocker
  verdict: needs-revision
  origin: introduced
  message: "its reproduce step is 'a command step that passes when the named reproducing command fails', but the step map has no inverted-verdict or expected-failure mapping and its only placeholders are run_directory, task and transcript, so a per-run reproducing command cannot be named; no item in the set adds that capability and its scope lists no aep-driver-spec change, so a story that adds an expected-failure step and names the reproducing command must be added with a depends_on edge from this story"
- file: .engineering/planning/story/release-step-map.md
  line: 15
  category: design
  severity: blocker
  verdict: needs-revision
  origin: introduced
  message: "its outcome is that aep drive run cuts a release (version, changelog heading, tag, status), yet no command that performs those steps exists, because the only release verb in xtask is the check-only Release, and no item in the set adds one, since this scope is YAML and driver tests only, so a story producing the cutting commands in Rust must be added with a depends_on edge from this one"
- file: .engineering/planning/story/concurrent-command-steps.md
  line: 15
  category: design
  severity: blocker
  verdict: needs-revision
  origin: introduced
  message: "it defines ordering by 'depends_on between them' and says 'a step with depends_on starts only after the step it names has ended', but on a command step depends_on is the name of an outside dependency keyed by the circuit breaker, so the body must name a distinct step-ordering field and say how concurrent steps share a breaker's failure count"
```
