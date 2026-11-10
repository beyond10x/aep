---
format: aep.planning-md/3
id: review-result:store-pack-critic-scope-round-1
kind: review-result
status: active
title: Scope critic, round 1, store-pack stories
relations:
- reviews: story:lifecycle-declares-closed-states
- reviews: story:pack-format-and-overlay-read
- reviews: story:pack-and-unpack-verbs
- reviews: story:validate-checks-packs
- reviews: story:pack-end-to-end-and-docs
revision: 1
---
needs-revision
story:pack-and-unpack-verbs — The operator's request includes "Git merge behaviour of packs", and the design promises that `aep plan store unpack <id>` fixes a merge that leaves a path "packed twice with different bytes" ("loose copy kept, packed copies removed"), but no outcome in the set claims this; the outcome names no merge case and acceptance 4 covers only a path that is already loose — .engineering/planning/architecture-design/store-pack.md:154-156; .engineering/planning/story/pack-and-unpack-verbs.md:43
story:pack-and-unpack-verbs — The design says loose evidence added to an already-packed artifact is picked up by the next pack ("the next `pack` folds it in") and that a loose copy of a record a pack already holds byte for byte is removed, but the outcome claims only "every closed artifact whose bytes Git has committed", so nothing claims either case — .engineering/planning/architecture-design/store-pack.md:125,133-134; .engineering/planning/story/pack-and-unpack-verbs.md:19-20

What I read: 9 artifacts (the epic, `architecture-design:store-pack`, `architecture-decision-record:packed-store-is-aep-project-6`, the 5 drafted stories, and `story:review-results-stay-active-after-their-subject-closes` to confirm it is excluded). Commands: `aep plan artifact show` on each, `aep plan artifact graph`, plus `grep`/`sed` on the store files. I listed 17 promises from the epic, the operator's request and the design the epic points to. 15 are claimed in full by exactly one item. 2 are claimed only in part: merge behaviour and the second pack run picking up leftovers, both in the findings above. Nothing in the set goes beyond what the parent asked for, and no two items claim the same outcome.

What I could not establish:
- Out of my lane (acceptance): nothing tests that an older `aep` refuses a packed store by name. The ADR's measurement on 0.79.0 is the only evidence, and `/6` is set only in the pack story's outcome.
- Out of my lane (acceptance): `unpack --dry-run` is in the pack story's outcome but has no acceptance.
- Out of my lane (design): the design does not say which bytes `unpack <id>` writes when two packs hold different bytes for one path and no loose copy exists.
- Not a gap: the ADR says the `/6` row for the skills' version tables and telling the conductor and metaharness owners are not done here, and the epic's exclusion at .engineering/planning/epic/store-pack-closed-artifacts.md:55 covers the skill text.

```findings
- file: .engineering/planning/story/pack-and-unpack-verbs.md
  line: 43
  category: scope
  severity: blocker
  verdict: needs-revision
  origin: introduced
  message: "The operator's request includes \"Git merge behaviour of packs\", and the design promises that `aep plan store unpack <id>` fixes a merge that leaves a path \"packed twice with different bytes\" (\"loose copy kept, packed copies removed\"), but no outcome in the set claims this; the outcome names no merge case and acceptance 4 covers only a path that is already loose"
- file: .engineering/planning/story/pack-and-unpack-verbs.md
  line: 19
  category: scope
  severity: warning
  verdict: needs-revision
  origin: introduced
  message: "The design says loose evidence added to an already-packed artifact is picked up by the next pack (\"the next `pack` folds it in\") and that a loose copy of a record a pack already holds byte for byte is removed, but the outcome claims only \"every closed artifact whose bytes Git has committed\", so nothing claims either case"
```
