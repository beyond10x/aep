---
format: aep.planning-md/3
id: review-result:zero-open-issues-critic-acceptance-1
kind: review-result
status: active
title: Plan critic, acceptance, round 1 (zero-open-issues epic)
relations:
- reviews: story:relate-and-unrelate-write-the-revision-they-print
- reviews: story:an-artifact-id-can-be-renamed
- reviews: story:a-directory-and-a-file-inside-it-read-as-disjoint
- reviews: story:history-shows-a-removed-edge
- reviews: story:relations-order-is-the-same-in-every-store
- reviews: story:published-pattern-residue
- reviews: story:agents-dependency-rule-matches-the-manifests
- reviews: story:discarded-protocol-source-tree-is-removed
- reviews: story:legacy-ess-report-calendar-refusal
- reviews: story:checks-map-evidence-producers
revision: 1
---
needs-revision
story:relate-and-unrelate-write-the-revision-they-print — the acceptance is three sentences covering three independent outcomes (`unrelate`, then "`relate` likewise writes the revision it prints", then the refused-`unrelate` case), and `relate` has no before/after state or history check, so one can pass while another fails — .engineering/planning/story/relate-and-unrelate-write-the-revision-they-print.md:73
story:published-pattern-residue — the acceptance says "Each of the four" but its only observable check, an empty `OPEN_DIVERGENCES`, never evaluates `ArtifactId` (the test pins three rows and `ArtifactId` is "not decided anywhere"), so the check reads the same before and after for `ArtifactId` and "derived from its own validator rather than paraphrasing it" has no check at all; the body does not say the test gains an `ArtifactId` case — .engineering/planning/story/published-pattern-residue.md:80
story:agents-dependency-rule-matches-the-manifests — "A count like the table above, taken after the change, matches the sentence" does not name which entries are counted or what match means, and the example rule ("a new external dependency") turns on "new", which a count of manifest entries cannot see — .engineering/planning/story/agents-dependency-rule-matches-the-manifests.md:51
story:legacy-ess-report-calendar-refusal — "at the calendar boundary" names no value, "return a located error" names no exit status or message, and "retain exact original source time" has no referent on a refused import, so the acceptance does not say what a caller observes instead of today's exit 101 panic at `crates/plan/aep-backend-entity/src/lib.rs:265-267` — .engineering/planning/story/legacy-ess-report-calendar-refusal.md:34

What I read: all 10 ids with `aep plan artifact show <id>`, whole body each. I read the acceptance paragraphs in the store files for line numbers. In the tree I checked `OPEN_DIVERGENCES` and the `ArtifactId` notes in `published_pattern_evaluated.rs`, `ArtifactId::PATTERN` in `artifact.rs:135`, the `macos` job in `ci.yml:109`, commit `04da2ef2f` as an ancestor of HEAD, and `coverage.rs:174-201`.

What I could not establish:
- I did not run `aep plan artifact kinds` or `lifecycle`. Nothing here depended on a status name.
- `story:relations-order-is-the-same-in-every-store` says "both backends", but its scope also lists Postgres and `store_selection.rs` compares two stores. Which two is a scope question, out of my lane, and it did not set my verdict.
- `story:checks-map-evidence-producers` acceptance 4 still has `CHANGELOG.md` carrying the entry, while triage makes it the coordinator's file. That is a surface and scope question, out of my lane.
- `story:an-artifact-id-can-be-renamed`, `story:a-directory-and-a-file-inside-it-read-as-disjoint`, `story:history-shows-a-removed-edge`, `story:relations-order-is-the-same-in-every-store`, `story:discarded-protocol-source-tree-is-removed` and `story:checks-map-evidence-producers` have observable acceptances with a before state. Their multi-part lists are post-conditions of one change, so I did not flag them.

```findings
- file: .engineering/planning/story/relate-and-unrelate-write-the-revision-they-print.md
  line: 73
  category: acceptance
  severity: warning
  verdict: needs-revision
  origin: introduced
  message: "the acceptance is three sentences covering three independent outcomes (unrelate, then \"`relate` likewise writes the revision it prints\", then the refused-unrelate case), and relate has no before/after state or history check, so one can pass while another fails"
- file: .engineering/planning/story/published-pattern-residue.md
  line: 80
  category: acceptance
  severity: blocker
  verdict: needs-revision
  origin: introduced
  message: "the acceptance says \"Each of the four\" but its only observable check, an empty OPEN_DIVERGENCES, never evaluates ArtifactId (the test pins three rows and ArtifactId is not decided anywhere), so the check reads the same before and after for ArtifactId and \"derived from its own validator rather than paraphrasing it\" has no check at all; the body does not say the test gains an ArtifactId case"
- file: .engineering/planning/story/agents-dependency-rule-matches-the-manifests.md
  line: 51
  category: acceptance
  severity: warning
  verdict: needs-revision
  origin: introduced
  message: "\"A count like the table above, taken after the change, matches the sentence\" does not name which entries are counted or what match means, and the example rule (\"a new external dependency\") turns on \"new\", which a count of manifest entries cannot see"
- file: .engineering/planning/story/legacy-ess-report-calendar-refusal.md
  line: 34
  category: acceptance
  severity: blocker
  verdict: needs-revision
  origin: introduced
  message: "\"at the calendar boundary\" names no value, \"return a located error\" names no exit status or message, and \"retain exact original source time\" has no referent on a refused import, so the acceptance does not say what a caller observes instead of today's exit 101 panic at crates/plan/aep-backend-entity/src/lib.rs:265-267"
```
