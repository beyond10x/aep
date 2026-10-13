---
format: aep.planning-md/3
id: review-result:detflow-scope-round-1
kind: review-result
status: active
title: 'Plan critic (scope), round 1: deterministic delivery workflows'
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

story:release-step-map — the parent lists "record" as the last release step and says "yes, every step", but the acceptance covers "version, changelog heading, tag, status, gate, push, PR, merge and verify" and names no record step; add it to the covered steps or say it is left out — .engineering/planning/epic/deterministic-delivery-workflows.md:60 (story .engineering/planning/story/release-step-map.md:29)
story:release-step-map — the parent's done-when is "A release of this repository is cut by `aep drive run`", but the story proves only a fixture repository and never says where this repository's own release commands are declared, so the narrowing is unrecorded; name the story that makes this repository's release run on the map, or say the epic's done-when is checked at epic close — .engineering/planning/epic/deterministic-delivery-workflows.md:110
story:task-spend-is-modelled — the parent says "These are recorded per task" and lists "llm steps run, beside llm steps declared by the map", but the record holds only calls, tokens, cost and wall time, and `story:llm-step-declares-judgement` only prints the counts per run, so these two figures are recorded nowhere per task; add them to the record or say they are print-only — .engineering/planning/epic/deterministic-delivery-workflows.md:74 (story .engineering/planning/story/task-spend-is-modelled.md:16)
story:spend-against-baseline — the parent defines "A baseline is the measure of the first recorded tasks of one kind", but the story reads a baseline the project declares in `project.yaml` and nothing says how it is taken from those first tasks; the body should say it is hand-copied from the verb's output, or add the derivation — .engineering/planning/epic/deterministic-delivery-workflows.md:76 (story .engineering/planning/story/spend-against-baseline.md:18)
story:bugfix-workflow — the parent says for diagnose that "Collecting state (logs, traces, store reads) is a set of commands that can run at once", but the body declares diagnose only as an llm step and no command steps for collecting state, and `story:concurrent-command-steps` only says it "covers" them; add the collection command steps to the map or say they are left out — .engineering/planning/epic/deterministic-delivery-workflows.md:62 (story .engineering/planning/story/bugfix-workflow.md:19)

**What I read.** 8 artifacts: the parent and 7 stories, via `aep plan artifact show` on each. I also ran `aep plan artifact graph` and `aep plan artifact validate`, which printed `valid` plus review-outcome reminders unrelated to this set, and a grep of `.engineering/planning` for other artifacts on the same outcomes. I extracted 17 promises from the parent: 6 Scope bullets, 3 Done When bullets, 7 steps-table rows with a stated replacement, and the measures and baseline definition. I traced 15 to a story. The 2 untraced are in the findings above (record step, first-recorded-tasks baseline), and the rest of the findings are narrowings inside promises that are traced. I found no reach beyond the parent and no two stories claiming one outcome.

**What I could not establish.**
- The epic's table also marks wave selection as "yes: the wave is derived", and marks specify, scope-a-story and adversarial-verify as "partly" deterministic. None of these is in the epic's Scope list or Done When, so I treated them as descriptive and did not call them gaps. If the operator reads them as promises, they are uncovered.
- Out of my lane, for `plan-critic-acceptance`: `story:spend-against-baseline` refuses a target "above" its baseline, which admits equality where the parent says "set below the baseline". The cache-read and cache-write tokens appear only as "cache" in `story:task-spend-is-modelled`. `story:evidence-records-task-spend` has no acceptance for extracting wall time.

```findings
- file: .engineering/planning/epic/deterministic-delivery-workflows.md
  line: 60
  category: scope
  severity: warning
  verdict: needs-revision
  origin: introduced
  message: "story:release-step-map — the parent lists 'record' as the last release step and says 'yes, every step', but the acceptance covers version, changelog heading, tag, status, gate, push, PR, merge and verify and names no record step; add it or say it is left out"
- file: .engineering/planning/epic/deterministic-delivery-workflows.md
  line: 110
  category: scope
  severity: warning
  verdict: needs-revision
  origin: introduced
  message: "story:release-step-map — the parent's done-when is a release of this repository cut by aep drive run, but the story proves only a fixture repository and does not say where this repository's release commands are declared, so the narrowing is unrecorded; name the story that does it or say the done-when is checked at epic close"
- file: .engineering/planning/epic/deterministic-delivery-workflows.md
  line: 74
  category: scope
  severity: blocker
  verdict: needs-revision
  origin: introduced
  message: "story:task-spend-is-modelled — the parent says the measures are recorded per task, including 'llm steps run, beside llm steps declared by the map', but the record holds only calls, tokens, cost and wall time and story:llm-step-declares-judgement only prints the counts per run, so these figures are recorded nowhere per task; add them to the record or say they are print-only"
- file: .engineering/planning/epic/deterministic-delivery-workflows.md
  line: 76
  category: scope
  severity: warning
  verdict: needs-revision
  origin: introduced
  message: "story:spend-against-baseline — the parent defines a baseline as the measure of the first recorded tasks of one kind, but the story reads a project-declared baseline and nothing says how it is taken from those tasks; say it is hand-copied from the verb's output or add the derivation"
- file: .engineering/planning/epic/deterministic-delivery-workflows.md
  line: 62
  category: scope
  severity: warning
  verdict: needs-revision
  origin: introduced
  message: "story:bugfix-workflow — the parent says collecting state (logs, traces, store reads) is a set of commands that can run at once at diagnose, but the body declares diagnose only as an llm step with no collection command steps; add them to the map or say they are left out"
```
