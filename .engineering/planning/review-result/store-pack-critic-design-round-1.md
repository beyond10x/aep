---
format: aep.planning-md/3
id: review-result:store-pack-critic-design-round-1
kind: review-result
status: active
title: Design critic, round 1, store-pack stories
relations:
- reviews: story:lifecycle-declares-closed-states
- reviews: story:pack-format-and-overlay-read
- reviews: story:pack-and-unpack-verbs
- reviews: story:validate-checks-packs
- reviews: story:pack-end-to-end-and-docs
revision: 1
---
needs-revision
story:pack-and-unpack-verbs — it adds the `aep plan store pack` and `unpack` verbs, but the rows for them in `website/docs/reference/cli.md` belong to story:pack-end-to-end-and-docs. Its acceptance has to include those rows, otherwise the story cannot land with `cargo test -p aep-cli` and `docs-check` passing. — .engineering/planning/story/pack-and-unpack-verbs.md:49, .engineering/planning/story/pack-end-to-end-and-docs.md:29, crates/edge/aep-cli/src/app.rs:2925
story:validate-checks-packs — its Context takes on design items 1–8, and item 6 needs the packed artifact's status to be closed for its kind. That closed set comes from story:lifecycle-declares-closed-states, and no edge records it. Fix by adding `depends_on story:lifecycle-declares-closed-states` or by putting item 6's closed-status check out of scope. — .engineering/planning/story/validate-checks-packs.md:26, .engineering/planning/architecture-design/store-pack.md:175
story:pack-end-to-end-and-docs — it holds the Unreleased CHANGELOG entries for the lifecycle `closed:` key, `aep.project/6` and `pack`/`unpack`. The rule is an entry for every user-visible change, so each entry belongs in the story that makes that change (lifecycle-declares-closed-states, pack-format-and-overlay-read, pack-and-unpack-verbs), not in a final docs story. — .engineering/planning/story/pack-end-to-end-and-docs.md:42, AGENTS.md:187

**What I read:** all 6 artifacts in the set, plus `architecture-design:store-pack` and `architecture-decision-record:packed-store-is-aep-project-6`. Commands: `aep plan artifact show` for each, `aep plan artifact relations`, `aep plan artifact graph`, `aep plan artifact validate` (prints `valid`, with nothing on this set), `aep plan artifact waves`. I also read Taskfile.yml (`docs-check`), the `cli_reference` test in app.rs, journal.rs, and the `git_findings` caller of `committed_versions`.

I walked 26 edges, including the two edges from the finding outside the set. The only ordering edges are the 5 `depends_on` edges, and they form no cycle: lifecycle and format/overlay come first, then verbs and validate, then end-to-end. That is not a serialising chain. I traced 8 acceptance or context reads to a producer in the set, and an edge records 7 of them. One of the 7 is transitive: the end-to-end story's CHANGELOG line reaches the lifecycle story through pack-and-unpack-verbs. The one with no edge is validate-checks-packs reading the closed set.

**What I could not establish:**
- Draft stories do not appear in `aep plan artifact waves`, so I could not see the wave placement directly.
- Out of my lane (acceptance): validate-checks-packs takes on design item 6, but no acceptance line tests it.
- Unease, not a finding: pack-format-and-overlay-read acceptance 2 runs `validate` over a packed store before validate-checks-packs extends the Git-history walk. If that fixture commits packed review-results, it may report false immutability problems.
- Out of my lane (size): pack-format-and-overlay-read is large (pack module, overlay for every reader, write refusals, `/6`). It is one abstraction, not two.

```findings
- file: .engineering/planning/story/pack-and-unpack-verbs.md
  line: 49
  category: design
  severity: blocker
  verdict: needs-revision
  origin: introduced
  message: "it adds the `aep plan store pack` and `unpack` verbs, but the rows for them in `website/docs/reference/cli.md` belong to story:pack-end-to-end-and-docs. Its acceptance has to include those rows, otherwise the story cannot land with `cargo test -p aep-cli` and `docs-check` passing."
- file: .engineering/planning/story/validate-checks-packs.md
  line: 26
  category: design
  severity: warning
  verdict: needs-revision
  origin: introduced
  message: "its Context takes on design items 1–8, and item 6 needs the packed artifact's status to be closed for its kind. That closed set comes from story:lifecycle-declares-closed-states, and no edge records it. Fix by adding `depends_on story:lifecycle-declares-closed-states` or by putting item 6's closed-status check out of scope."
- file: .engineering/planning/story/pack-end-to-end-and-docs.md
  line: 42
  category: design
  severity: warning
  verdict: needs-revision
  origin: introduced
  message: "it holds the Unreleased CHANGELOG entries for the lifecycle `closed:` key, `aep.project/6` and `pack`/`unpack`. The rule is an entry for every user-visible change, so each entry belongs in the story that makes that change (lifecycle-declares-closed-states, pack-format-and-overlay-read, pack-and-unpack-verbs), not in a final docs story."
```
