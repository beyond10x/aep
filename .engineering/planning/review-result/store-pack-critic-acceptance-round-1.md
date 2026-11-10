---
format: aep.planning-md/3
id: review-result:store-pack-critic-acceptance-round-1
kind: review-result
status: active
title: Acceptance critic, round 1, store-pack stories
relations:
- reviews: story:lifecycle-declares-closed-states
- reviews: story:pack-format-and-overlay-read
- reviews: story:pack-and-unpack-verbs
- reviews: story:validate-checks-packs
- reviews: story:pack-end-to-end-and-docs
revision: 1
---
needs-revision
story:lifecycle-declares-closed-states — acceptance item 4 does not say the regenerated schema gains a `closed` property, so "`schema-check` passes" is just as true before the work as after it — .engineering/planning/story/lifecycle-declares-closed-states.md:41
story:lifecycle-declares-closed-states — the acceptance checks the closed set only for `story`, `review-result` and `approval-record`, so the Outcome's claim that every ladder with `implemented` or `rejected` declares them closed (epic, task, initiative and the rest) can stay false while every item passes — .engineering/planning/story/lifecycle-declares-closed-states.md:33
story:lifecycle-declares-closed-states — the acceptance is four independent numbered outcomes instead of one statement, so one can pass while another fails — .engineering/planning/story/lifecycle-declares-closed-states.md:31
story:pack-format-and-overlay-read — the acceptance is six independent numbered outcomes instead of one statement, so one can pass while another fails — .engineering/planning/story/pack-format-and-overlay-read.md:34
story:pack-and-unpack-verbs — no acceptance item checks that `project.yaml` moves from `aep.project/5` to `aep.project/6` after `pack`, so a `pack` that never writes `/6` passes items 1 to 6 even though the Outcome promises it at line 21 — .engineering/planning/story/pack-and-unpack-verbs.md:38
story:pack-and-unpack-verbs — the acceptance is six independent numbered outcomes instead of one statement, so one can pass while another fails — .engineering/planning/story/pack-and-unpack-verbs.md:31
story:validate-checks-packs — the preamble requires every case to hold a committed pack and every refusal to name the pack and the path, which item 5's `aep.project/6` store without a pack cannot satisfy — .engineering/planning/story/validate-checks-packs.md:42
story:validate-checks-packs — item 7 does not say which committed version it compares against (the first committed version, or the last one before the pack), so its fixture cannot be told apart from item 2's and a test cannot know whether immutability or provenance is the right code — .engineering/planning/story/validate-checks-packs.md:45
story:validate-checks-packs — the Outcome says `validate` checks every pack's format, but no case refuses a pack that fails to parse or whose file name is not its content digest — .engineering/planning/story/validate-checks-packs.md:31
story:validate-checks-packs — the acceptance is eight independent numbered outcomes instead of one statement, so one can pass while another fails — .engineering/planning/story/validate-checks-packs.md:31
story:pack-end-to-end-and-docs — item 3 says the documents "describe" the verbs, the format and the version without naming any text a check could find, and it names no path for "the website page for the store" although two candidates exist (`website/docs/concepts/planning-store.md`, `website/docs/reference/planning-stores.md`) — .engineering/planning/story/pack-end-to-end-and-docs.md:39
story:pack-end-to-end-and-docs — the acceptance is four independent numbered outcomes instead of one statement, so one can pass while another fails — .engineering/planning/story/pack-end-to-end-and-docs.md:31

What I read: all 6 given ids (1 epic and 5 stories) in full, plus `architecture-design:store-pack` and `architecture-decision-record:packed-store-is-aep-project-6`. Commands: `aep plan artifact show`, `aep plan artifact kinds`, `aep plan artifact lifecycle story|epic|review-result|approval-record`, `git ls-files`/`git grep` for the cited paths and symbols, and a read of `Taskfile.yml` for `schema-check` and `docs-check`.

What I could not establish:
- The epic has no `## Acceptance` section. Store convention requires one only on stories and tasks, and 20 of the 22 epics in this store have none, so I did not count it as a finding.
- Most of the stories' items say only "Checked by a CLI test" without naming the test or the command that runs it. That is common in this store and I did not raise it.
- The "one statement" findings apply the convention in `store-conventions.md:115`. However, 16 implemented, archived or rejected stories in this store already use numbered acceptance lists, so the operator may treat those findings as superseded by that practice.
- Out of my lane (scope): design item 6, which says a packed artifact's status is closed and packed evidence belongs to its directory's artifact, is not in `story:validate-checks-packs`'s Outcome.

```findings
- file: .engineering/planning/story/lifecycle-declares-closed-states.md
  line: 41
  category: acceptance
  severity: warning
  verdict: needs-revision
  origin: introduced
  message: "acceptance item 4 does not say the regenerated schema gains a `closed` property, so \"`schema-check` passes\" is just as true before the work as after it"
- file: .engineering/planning/story/lifecycle-declares-closed-states.md
  line: 33
  category: acceptance
  severity: warning
  verdict: needs-revision
  origin: introduced
  message: "the acceptance checks the closed set only for `story`, `review-result` and `approval-record`, so the Outcome's claim that every ladder with `implemented` or `rejected` declares them closed (epic, task, initiative and the rest) can stay false while every item passes"
- file: .engineering/planning/story/lifecycle-declares-closed-states.md
  line: 31
  category: acceptance
  severity: warning
  verdict: needs-revision
  origin: introduced
  message: "the acceptance is four independent numbered outcomes instead of one statement, so one can pass while another fails"
- file: .engineering/planning/story/pack-format-and-overlay-read.md
  line: 34
  category: acceptance
  severity: warning
  verdict: needs-revision
  origin: introduced
  message: "the acceptance is six independent numbered outcomes instead of one statement, so one can pass while another fails"
- file: .engineering/planning/story/pack-and-unpack-verbs.md
  line: 38
  category: acceptance
  severity: warning
  verdict: needs-revision
  origin: introduced
  message: "no acceptance item checks that `project.yaml` moves from `aep.project/5` to `aep.project/6` after `pack`, so a `pack` that never writes `/6` passes items 1 to 6 even though the Outcome promises it at line 21"
- file: .engineering/planning/story/pack-and-unpack-verbs.md
  line: 31
  category: acceptance
  severity: warning
  verdict: needs-revision
  origin: introduced
  message: "the acceptance is six independent numbered outcomes instead of one statement, so one can pass while another fails"
- file: .engineering/planning/story/validate-checks-packs.md
  line: 42
  category: acceptance
  severity: warning
  verdict: needs-revision
  origin: introduced
  message: "the preamble requires every case to hold a committed pack and every refusal to name the pack and the path, which item 5's `aep.project/6` store without a pack cannot satisfy"
- file: .engineering/planning/story/validate-checks-packs.md
  line: 45
  category: acceptance
  severity: warning
  verdict: needs-revision
  origin: introduced
  message: "item 7 does not say which committed version it compares against (the first committed version, or the last one before the pack), so its fixture cannot be told apart from item 2's and a test cannot know whether immutability or provenance is the right code"
- file: .engineering/planning/story/validate-checks-packs.md
  line: 31
  category: acceptance
  severity: warning
  verdict: needs-revision
  origin: introduced
  message: "the Outcome says `validate` checks every pack's format, but no case refuses a pack that fails to parse or whose file name is not its content digest"
- file: .engineering/planning/story/validate-checks-packs.md
  line: 31
  category: acceptance
  severity: warning
  verdict: needs-revision
  origin: introduced
  message: "the acceptance is eight independent numbered outcomes instead of one statement, so one can pass while another fails"
- file: .engineering/planning/story/pack-end-to-end-and-docs.md
  line: 39
  category: acceptance
  severity: warning
  verdict: needs-revision
  origin: introduced
  message: "item 3 says the documents \"describe\" the verbs, the format and the version without naming any text a check could find, and it names no path for \"the website page for the store\" although two candidates exist (`website/docs/concepts/planning-store.md`, `website/docs/reference/planning-stores.md`)"
- file: .engineering/planning/story/pack-end-to-end-and-docs.md
  line: 31
  category: acceptance
  severity: warning
  verdict: needs-revision
  origin: introduced
  message: "the acceptance is four independent numbered outcomes instead of one statement, so one can pass while another fails"
```
