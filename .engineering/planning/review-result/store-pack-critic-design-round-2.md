---
format: aep.planning-md/3
id: review-result:store-pack-critic-design-round-2
kind: review-result
status: active
title: Design critic, round 2, store-pack stories
relations:
- reviews: story:lifecycle-declares-closed-states
- reviews: story:pack-format-and-overlay-read
- reviews: story:pack-and-unpack-verbs
- reviews: story:validate-checks-packs
- reviews: story:pack-end-to-end-and-docs
revision: 1
---
needs-revision
story:pack-format-and-overlay-read — its line-44 `validate` run over a packed fixture is only independent of story:validate-checks-packs when the pack move is committed, because `git_record::changed_evidence` diffs `HEAD` with `--diff-filter=MTD` and reports every uncommitted packed evidence deletion as a rewritten record; the Out of scope sentence's "commit loose files only" option must go, so the fixture commits the pack — .engineering/planning/story/pack-format-and-overlay-read.md:44, .engineering/planning/story/pack-format-and-overlay-read.md:72, crates/edge/aep-cli/src/planning/git_record.rs:192, crates/edge/aep-cli/src/planning.rs:6450
story:pack-format-and-overlay-read — its packing test does not say the fixture holding a pack is `version: aep.project/6`, so the `validate` pass on line 44 breaks once story:validate-checks-packs refuses "a pack in an `aep.project/5` store", and the version-agreement rule is split across the two stories with no owner for the fixture — .engineering/planning/story/pack-format-and-overlay-read.md:40, .engineering/planning/story/validate-checks-packs.md:52

What I read: 6 set artifacts plus `architecture-design:store-pack`, `architecture-decision-record:packed-store-is-aep-project-6` and `review-result:store-pack-critic-design-round-1`, using `aep plan artifact show`, `aep plan artifact relations`, `aep plan artifact graph`, `aep plan artifact validate` (`valid`, nothing on this set), and `git_record.rs` / `planning.rs` (`git_findings`, `reviews_recorded_at`, `print_validation`). Edges walked: 47, going outside the set (27 structural edges including the finding story's 2, plus 20 `reviews`). The 6 `depends_on` edges form no cycle and no serialising chain: lifecycle and format come first, then verbs and validate in parallel, then end-to-end. I traced 12 acceptance reads to a producer in the set. Edges record all 12, 4 of them transitively through story 5. All three round-1 findings are fixed: the CLI rows are now in pack-and-unpack-verbs.md:66, the `depends_on` lifecycle edge is at validate-checks-packs.md:12, and the changelog lines are back with their stories (pack-end-to-end-and-docs.md:29, :49).

What I could not establish:
- Out of my lane (scope): no story makes `changed_evidence` pack-aware, so running `pack` then `validate` before committing would report every packed evidence file as rewritten. The design's reader table (store-pack.md:47-55) does not list `changed_evidence`.
- Out of my lane (acceptance): story 5 asserts that `validate` prints byte-identical output before and after packing. Story 2's comparison list (pack-format-and-overlay-read.md:40-43) leaves `validate` out, and story 4 asserts only `valid`. No story before story 5 checks whether the `N file(s) … N artifact(s)` line (planning.rs:6890, `files_read`) counts packed records.
- I checked `first_committed`: it filters on `A` only, so it still dates packed review-results after a pack commit. Not a finding.

```findings
- file: .engineering/planning/story/pack-format-and-overlay-read.md
  line: 72
  category: design
  severity: warning
  verdict: needs-revision
  origin: introduced
  message: "its line-44 `validate` run over a packed fixture is only independent of story:validate-checks-packs when the pack move is committed, because `git_record::changed_evidence` diffs `HEAD` with `--diff-filter=MTD` and reports every uncommitted packed evidence deletion as a rewritten record; the Out of scope sentence's \"commit loose files only\" option must go, so the fixture commits the pack"
- file: .engineering/planning/story/pack-format-and-overlay-read.md
  line: 40
  category: design
  severity: warning
  verdict: needs-revision
  origin: introduced
  message: "its packing test does not say the fixture holding a pack is `version: aep.project/6`, so the `validate` pass on line 44 breaks once story:validate-checks-packs refuses \"a pack in an `aep.project/5` store\", and the version-agreement rule is split across the two stories with no owner for the fixture"
```
