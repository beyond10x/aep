---
format: aep.planning-md/3
id: review-result:detflow-design-round-2
kind: review-result
status: active
title: 'Plan critic (design), round 2: deterministic delivery workflows'
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
story:bugfix-workflow — its acceptance says the collection steps' output is in the diagnose step's context, but an llm step's `context` is a list of literal files with no placeholders (so it cannot name the per-run `{run_directory}`), no command output reaches it, and no item in the set adds that, so a story (or a scope line on story:command-step-expects-failure) letting a command's output file be named in a later llm step's context must exist, with a `depends_on` edge from this story — .engineering/planning/story/bugfix-workflow.md:43, crates/drive/aep-driver-spec/src/map.rs:401-405
story:release-step-map — its steps run `prepare <version>` and `record --run <url>`, which take per-run values, and the only way a map can name one is the `{input.<name>}` placeholder that story:command-step-expects-failure adds (today's placeholders are `run_directory`, `task`, `transcript`), so a `depends_on` edge to that story is missing, and the body must say where the Release run URL comes from, since it does not exist when the run starts — .engineering/planning/story/release-step-map.md:17-21, crates/drive/aep-driver-spec/src/map.rs:599
story:concurrent-command-steps — `after: [<step id>]` names a step id, but a step has no id field (a step is addressed by its index in `steps_for(state)`), so the body must say whether it adds an `id` to command steps or `after` takes indices — .engineering/planning/story/concurrent-command-steps.md:17, crates/drive/aep-driver-spec/src/map.rs:364-381
story:concurrent-command-steps — it makes concurrency the default for every command state, which also changes the shipped `verify` states (`default.yaml:216-300`, six sequential command steps), yet its scope names no `drivers/development/*.yaml` change and no acceptance says existing maps run as before, so it must either make concurrency opt-in per state or put `after` on the shipped maps in scope — .engineering/planning/story/concurrent-command-steps.md:17, drivers/development/default.yaml:216

What I read: 11 artifacts in the worktree (the epic, nine stories and review-result:detflow-design-round-1), with `aep plan artifact show`, `relations`, `graph` and `validate` (`valid`). I also read the driver map and run sources, the `release/source` and `incident/standard` workflows, and the shipped verify state in `default.yaml`.

Round 1: the three findings are revised and none holds as stated. The reproduce step now depends on story:command-step-expects-failure, and story:release-cutting-commands exists with an edge from story:release-step-map. story:concurrent-command-steps now names `after`. The findings above are new defects in the revised text, not repeats.

Graph and acceptance reads: I walked all 14 `depends_on` and `decomposes` edges among the nine stories, plus the epic's `serves` and `informed_by` edges that leave the set. There is no cycle, and the longest chain is three (llm-step, then command-step, then bugfix or concurrent). Each shared-file ordering edge is stated with its file, so I do not count the chains as serialising. I traced 12 acceptance reads to a producer in the set, and 9 have an edge. The 3 without an edge are the findings above: the diagnose context, the release version and run URL, and the step-id read.

What I could not establish:
- Whether the bugfix `reproduce` and `verify` steps need a second `{input.*}` capability beyond `--input` is a detail for the acceptance critic.
- Where the release `record` step gets its run URL is partly a design question for story:release-cutting-commands (it could discover the run from the tag). I named only the artifact whose body has to say it.
- Out of my lane, and not setting the verdict: the forge stub is still marked "Inferred" in story:release-step-map, which is for acceptance and parallel-safety.

```findings
- file: .engineering/planning/story/bugfix-workflow.md
  line: 43
  category: design
  severity: blocker
  verdict: needs-revision
  origin: introduced
  message: "its acceptance says the collection steps' output is in the diagnose step's context, but an llm step's context is a list of literal files with no placeholders (so it cannot name the per-run run_directory), no command output reaches it, and no item in the set adds that, so a story (or a scope line on story:command-step-expects-failure) letting a command's output file be named in a later llm step's context must exist, with a depends_on edge from this story"
- file: .engineering/planning/story/release-step-map.md
  line: 17
  category: design
  severity: blocker
  verdict: needs-revision
  origin: introduced
  message: "its steps run prepare <version> and record --run <url>, which take per-run values, and the only way a map can name one is the {input.<name>} placeholder that story:command-step-expects-failure adds, so a depends_on edge to that story is missing, and the body must say where the Release run URL comes from, since it does not exist when the run starts"
- file: .engineering/planning/story/concurrent-command-steps.md
  line: 17
  category: design
  severity: blocker
  verdict: needs-revision
  origin: introduced
  message: "after: [<step id>] names a step id, but a step has no id field (a step is addressed by its index in steps_for(state)), so the body must say whether it adds an id to command steps or after takes indices"
- file: .engineering/planning/story/concurrent-command-steps.md
  line: 17
  category: design
  severity: warning
  verdict: needs-revision
  origin: introduced
  message: "it makes concurrency the default for every command state, which also changes the shipped verify states (six sequential command steps), yet its scope names no drivers/development/*.yaml change and no acceptance says existing maps run as before, so it must either make concurrency opt-in per state or put after on the shipped maps in scope"
```
