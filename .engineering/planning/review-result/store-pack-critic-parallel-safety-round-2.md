---
format: aep.planning-md/3
id: review-result:store-pack-critic-parallel-safety-round-2
kind: review-result
status: active
title: Parallel-safety critic, round 2, store-pack stories
relations:
- reviews: story:lifecycle-declares-closed-states
- reviews: story:pack-format-and-overlay-read
- reviews: story:pack-and-unpack-verbs
- reviews: story:validate-checks-packs
- reviews: story:pack-end-to-end-and-docs
revision: 1
---
needs-revision

story:validate-checks-packs — this story and story:pack-and-unpack-verbs both have to find every place one path is held (loose and packed, or two packs with different bytes), but story:pack-format-and-overlay-read's overlay only promises "one tree of files" and no way to list every holder of a path; both wave-2 stories will therefore add that lookup to the not-yet-created overlay module in `aep-backend-markdown` (inferred), and neither body names the other or that module under Shared surfaces, so either the overlay story names the lookup as its own, or a depends_on edge between the two names the shared module — .engineering/planning/story/validate-checks-packs.md:48, .engineering/planning/story/pack-and-unpack-verbs.md:57, .engineering/planning/story/pack-format-and-overlay-read.md:17

**What I read:** 7 artifacts: the epic, the 5 stories, `architecture-design:store-pack`, and round-1 `review-result:store-pack-critic-parallel-safety-round-1` and `-design-round-1`. Commands: `aep plan artifact show`, `graph`, `relations`, `validate` (prints `valid`; its only output is stale review-results outside the set), `waves` (all 5 stories `unassessed` because they have no scope yet, as expected before story-scopers run). I also ran `git grep` and `sed` on `planning.rs` (`validate` :6075, `git_findings` :6395, `evidence_problems` :6736, `fn lifecycle` :7082), `store_command.rs`, `error.rs` (`validation_codes!`), `artifact.rs` and the backend-markdown tests.
- **Round 1:** both findings are fixed. The `planning.rs` collision is now named in both bodies and split by symbol (lifecycle-declares-closed-states.md:48-51, pack-format-and-overlay-read.md:60-63). validate-checks-packs now builds its fixtures with the module's `render` and not the pack verb (validate-checks-packs.md:28-30).
- **Surfaces:** I placed all 5 items from what their bodies cite, and none is unplaced. Three parts are inferred, not cited:
  - pack-and-unpack lands in `store_command.rs`, found from the `StoreCommand::Migrate` symbol.
  - validate-checks-packs also lands in `planning.rs` (`validate`, `git_findings`). Its only concurrent peer does not touch that file.
  - The holder lookup in the finding above.
- **Acceptance reads:** I traced 7 to a producer in the set, and a `depends_on` edge records all 7. Six edges are direct. One is transitive: end-to-end's `list --status implemented` reaches lifecycle through pack-and-unpack.
- **Wave pairs that a revision would not change:** in wave 1, lifecycle and pack-format share `planning.rs` (admitted) and `CHANGELOG.md`. In wave 2, pack-and-unpack and validate share `CHANGELOG.md`. Each body lists the changelog under Shared surfaces, and lifecycle-declares-closed-states.md:53-54 says the other stories add their own lines.

**What I could not establish:**
- Whether story:lifecycle-declares-closed-states adds a new `ValidationCode` to `crates/govern/aep-domain/src/error.rs` or reuses `unknown_state` (the existing code fits, artifact.rs:2750). If it adds one and story:pack-format-and-overlay-read puts its parse codes in the same macro list, the wave-1 pair also collides on `error.rs`. This is inferred, so it is not a finding.
- Whether story:pack-and-unpack-verbs reads Git blobs through `git_record.rs` (which story:validate-checks-packs changes) or through the `git` helper in `store_command.rs`. If it uses `git_record.rs`, it is a consumer, not an editor, so this is not counted as a collision.

```findings
- file: .engineering/planning/story/validate-checks-packs.md
  line: 48
  category: parallel-safety
  severity: warning
  verdict: needs-revision
  origin: introduced
  message: "this story and story:pack-and-unpack-verbs both have to find every place one path is held (loose and packed, or two packs with different bytes), but story:pack-format-and-overlay-read's overlay only promises \"one tree of files\" and no way to list every holder of a path; both wave-2 stories will therefore add that lookup to the not-yet-created overlay module in aep-backend-markdown (inferred), and neither body names the other or that module under Shared surfaces, so either the overlay story names the lookup as its own, or a depends_on edge between the two names the shared module"
```
