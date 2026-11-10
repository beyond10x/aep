---
format: aep.planning-md/3
id: review-result:store-pack-critic-acceptance-round-2
kind: review-result
status: active
title: Acceptance critic, round 2, store-pack stories
relations:
- reviews: story:lifecycle-declares-closed-states
- reviews: story:pack-format-and-overlay-read
- reviews: story:pack-and-unpack-verbs
- reviews: story:validate-checks-packs
- reviews: story:pack-end-to-end-and-docs
revision: 1
---
needs-revision
story:pack-format-and-overlay-read — the round-1 finding is partly fixed, because there is now one lead sentence, but that sentence uses "and" to join two independent outcomes (every read verb answers the same over packed records; every write to a packed artifact is refused), so the reads can pass while the write refusals fail — .engineering/planning/story/pack-format-and-overlay-read.md:36
story:pack-and-unpack-verbs — the round-1 finding is partly fixed, because there is now one lead sentence, but that sentence uses "and" to join two independent outcomes (the round trip gives back the same bytes; each step does what its dry run said), and the merge, outside-Git and `--help` checks test neither of them — .engineering/planning/story/pack-and-unpack-verbs.md:35
story:pack-end-to-end-and-docs — the lead sentence covers only the 1000-story round trip, so the documentation check at line 41 is a second outcome the statement does not include, and the story can satisfy its sentence with the docs still unwritten — .engineering/planning/story/pack-end-to-end-and-docs.md:33
story:lifecycle-declares-closed-states — the lead sentence covers only what `lifecycle <kind>` prints, so three checks fall outside it: every ladder lists `implemented`/`rejected` in `closed:`, the protocol tree refuses an undeclared `closed:` state when it loads, and the schema gains a `closed` property; each can fail while the sentence holds — .engineering/planning/story/lifecycle-declares-closed-states.md:34

What I read: all 6 given ids in full (the epic and 5 stories), plus the round-1 acceptance review-result and its findings. Commands: `aep plan artifact show` (x7), `aep plan artifact kinds`, `aep plan artifact lifecycle story|epic|review-result|approval-record`, `aep plan artifact findings`, `aep plan artifact --help`, `aep plan store --help`, and `git grep`/`ls` on every path and symbol the acceptances cite. All of them exist, and none of the documentation strings, the `closed` keys or the schema property exist yet.

What I could not establish:
- Whether the operator accepts "one lead sentence plus a Checked-by list" as meeting the one-statement convention (store-conventions.md, section Body). Only these 5 stories in the store use that form.
- The 10 other round-1 findings are fixed: the schema check now says the `closed` property is absent before the work, every ladder is checked, `pack` must leave `/6`, the version case names `project.yaml`, the immutability case names which committed version it compares, the format and name cases are present, and the docs check names its files and strings.
- `story:validate-checks-packs` is fixed in full, because every check is a case of its one sentence.
- The epic has no `## Acceptance` section. The convention requires one only for stories and tasks, so I did not count it.
- Out of my lane: none seen.

```findings
- file: .engineering/planning/story/pack-format-and-overlay-read.md
  line: 36
  category: acceptance
  severity: warning
  verdict: needs-revision
  origin: introduced
  message: "the round-1 finding is partly fixed, because there is now one lead sentence, but that sentence uses \"and\" to join two independent outcomes (every read verb answers the same over packed records; every write to a packed artifact is refused), so the reads can pass while the write refusals fail"
- file: .engineering/planning/story/pack-and-unpack-verbs.md
  line: 35
  category: acceptance
  severity: warning
  verdict: needs-revision
  origin: introduced
  message: "the round-1 finding is partly fixed, because there is now one lead sentence, but that sentence uses \"and\" to join two independent outcomes (the round trip gives back the same bytes; each step does what its dry run said), and the merge, outside-Git and `--help` checks test neither of them"
- file: .engineering/planning/story/pack-end-to-end-and-docs.md
  line: 33
  category: acceptance
  severity: warning
  verdict: needs-revision
  origin: introduced
  message: "the lead sentence covers only the 1000-story round trip, so the documentation check at line 41 is a second outcome the statement does not include, and the story can satisfy its sentence with the docs still unwritten"
- file: .engineering/planning/story/lifecycle-declares-closed-states.md
  line: 34
  category: acceptance
  severity: warning
  verdict: needs-revision
  origin: introduced
  message: "the lead sentence covers only what `lifecycle <kind>` prints, so three checks fall outside it: every ladder lists `implemented`/`rejected` in `closed:`, the protocol tree refuses an undeclared `closed:` state when it loads, and the schema gains a `closed` property; each can fail while the sentence holds"
```
