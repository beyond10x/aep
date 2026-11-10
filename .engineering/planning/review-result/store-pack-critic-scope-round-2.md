---
format: aep.planning-md/3
id: review-result:store-pack-critic-scope-round-2
kind: review-result
status: active
title: Scope critic, round 2, store-pack stories
relations:
- reviews: story:lifecycle-declares-closed-states
- reviews: story:pack-format-and-overlay-read
- reviews: story:pack-and-unpack-verbs
- reviews: story:validate-checks-packs
- reviews: story:pack-end-to-end-and-docs
- reviews: architecture-design:store-pack
revision: 1
---
needs-revision
story:pack-and-unpack-verbs — The design says a store pinned to an older protocol tree "packs terminal states only (`archived`); that is stated in the dry run", but the outcome does not say this and the dry-run acceptance bullet leaves that statement out of the list of what the dry run prints, so no item claims it — .engineering/planning/architecture-design/store-pack.md:91-92; .engineering/planning/story/pack-and-unpack-verbs.md:39
story:pack-and-unpack-verbs — The design says `unpack` "writes only paths that pass the same check, inside `.engineering`, and never follows a symbolic link", but this story covers only the path-grammar refusal, and `story:validate-checks-packs` claims item 3 only as a `validate` check, so no item claims the confinement to `.engineering` or the symlink refusal — .engineering/planning/architecture-design/store-pack.md:168-169; .engineering/planning/story/pack-and-unpack-verbs.md:52

What I read: 9 artifacts (the epic, `architecture-design:store-pack`, `architecture-decision-record:packed-store-is-aep-project-6`, the 5 drafted stories, `review-result:store-pack-critic-scope-round-1`) plus `artifacts/lifecycles/*.yaml`. Commands: `aep plan artifact show` on each, `aep plan artifact graph`, `aep plan artifact findings epic:store-pack-closed-artifacts`, and `rg`/`cat -n` on the store files. I took 19 promises from the epic, the operator's request and the design the epic points to. 17 are claimed by exactly one item. 2 are only partly claimed (the findings above). No item reaches beyond the parent, and no two items claim the same outcome. The ADR `rejected` state is terminal already, so `story:lifecycle-declares-closed-states` saying "every ladder" adds nothing the design does not cover.

Both round-1 findings are fixed. The outcome now says the story "resolves a path a merge left in two places" (pack-and-unpack-verbs.md:24), with merge cases in acceptance (:55). It also now covers "loose evidence recorded against an artifact that is already packed" and "removes a loose copy a pack already holds byte for byte" (:20-22), tested at :46.

What I could not establish:
- Out of my lane (acceptance): nothing tests that `aep` reads two packs written on different branches at once (the design's first merge case). The overlay story says it reads "every record of every pack", but no fixture has more than one pack.
- Out of my lane (acceptance): the end-to-end store is "closed stories with evidence". Its acceptance does not say whether it has archived review-results or `rejected`/`archived` stories, so it is unclear what `findings <id>` compares there.
- Not a gap: the epic's out-of-scope list (epic/store-pack-closed-artifacts.md:50-56) excludes the skill text, packing AEP's own store, and active review-results whose subjects have closed. The ADR's notes for downstream readers (ADR :60-63) are also explicitly not done here.

```findings
- file: .engineering/planning/architecture-design/store-pack.md
  line: 92
  category: scope
  severity: warning
  verdict: needs-revision
  origin: introduced
  message: "The design says a store pinned to an older protocol tree \"packs terminal states only (`archived`); that is stated in the dry run\", but the outcome does not say this and the dry-run acceptance bullet leaves that statement out of the list of what the dry run prints, so no item claims it"
- file: .engineering/planning/architecture-design/store-pack.md
  line: 169
  category: scope
  severity: warning
  verdict: needs-revision
  origin: introduced
  message: "The design says `unpack` \"writes only paths that pass the same check, inside `.engineering`, and never follows a symbolic link\", but this story covers only the path-grammar refusal, and `story:validate-checks-packs` claims item 3 only as a `validate` check, so no item claims the confinement to `.engineering` or the symlink refusal"
```
