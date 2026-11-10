---
format: aep.planning-md/3
id: review-result:store-pack-critic-parallel-safety-round-1
kind: review-result
status: active
title: Parallel-safety critic, round 1, store-pack stories
relations:
- reviews: story:lifecycle-declares-closed-states
- reviews: story:pack-format-and-overlay-read
- reviews: story:pack-and-unpack-verbs
- reviews: story:validate-checks-packs
- reviews: story:pack-end-to-end-and-docs
revision: 1
---
needs-revision

story:lifecycle-declares-closed-states — both this story and story:pack-format-and-overlay-read change `crates/edge/aep-cli/src/planning.rs`, and neither body says so, yet no edge orders them so they can run at once. This story changes the `aep plan artifact lifecycle` printer: the body cites the command and `fn lifecycle` is at planning.rs:7082. The overlay story's cited reader table moves `evidence_problems` and the redact listing in the same file onto the overlay. Two fixes are open: add a depends_on edge that names the shared file, or split the surface so the two no longer share it. — .engineering/planning/story/lifecycle-declares-closed-states.md:19, crates/edge/aep-cli/src/planning.rs:7082, .engineering/planning/architecture-design/store-pack.md:52

story:validate-checks-packs — acceptance 8 checks a fixture store that is "packed and committed", which is the state the `pack` verb of story:pack-and-unpack-verbs writes. No depends_on edge runs to that story, and unlike story:pack-format-and-overlay-read:39 the body does not say the test writes the pack with the pack module's renderer. Both stories depend only on wave-1 items, so they land in the same wave. — .engineering/planning/story/validate-checks-packs.md:46, .engineering/planning/story/pack-and-unpack-verbs.md:19

**What I read:** 7 artifacts: the epic, the 5 stories, `architecture-design:store-pack`, and the opening of the ADR. Commands: `aep plan artifact show`, `aep plan artifact graph`, `aep plan artifact relations`, `aep plan artifact waves --help`, and `git grep` on the symbols the bodies and design cite (planning.rs, git_record.rs, store.rs, journal.rs, store_command.rs, artifact.rs, project.rs) plus AGENTS.md.
- **Surfaces:** 5 items placed from what their bodies cite, 0 placed by inference, 0 unplaced.
- **Acceptance reads:** 7 traced to a producer in the set. 6 are covered by a depends_on edge; one of those (end-to-end on lifecycle) only indirectly, through story:pack-and-unpack-verbs. The one without an edge is validate on the pack verb.

**What I could not establish:**
- **validate and lifecycle (out of my lane, acceptance or design):** validate's Context includes the design's item 6, which requires a packed artifact's status to be closed for its kind (store-pack.md:175). That reads the closed set story:lifecycle-declares-closed-states produces, but no edge records it. No acceptance line exercises item 6, so I did not count it as a reader beside its producer. Deriving waves from the edges still runs validate after overlay, so it never shares a wave with lifecycle.
- **CHANGELOG.md (unease, not a finding):** AGENTS.md:187 requires an Unreleased entry for every user-visible change, so implementors of stories 1 to 4 may each add one, colliding on that file in both waves. The plan routes the changelog to story:pack-end-to-end-and-docs, but no earlier story's body says to leave it alone.
- **Wave 2 (inferred, not established):** story:pack-and-unpack-verbs and story:validate-checks-packs may both extend the pack module that story:pack-format-and-overlay-read owns: the digest check, path checks and pack listing. They may also share Git helpers in `planning/git_record.rs`. The pack verb is more likely to use the separate `git` helper in `store_command.rs:202`.
- **Out of my lane (scope or design):** `crates/edge/aep-project/src/load.rs` also has a `collect_documents`. It appears to load the protocol tree, not the store.

```findings
- file: .engineering/planning/story/lifecycle-declares-closed-states.md
  line: 19
  category: parallel-safety
  severity: warning
  verdict: needs-revision
  origin: introduced
  message: "both this story and story:pack-format-and-overlay-read change crates/edge/aep-cli/src/planning.rs, and neither body says so, yet no edge orders them so they can run at once. This story changes the `aep plan artifact lifecycle` printer: the body cites the command and fn lifecycle is at planning.rs:7082. The overlay story's cited reader table moves evidence_problems and the redact listing in the same file onto the overlay. Two fixes are open: add a depends_on edge that names the shared file, or split the surface so the two no longer share it."
- file: .engineering/planning/story/validate-checks-packs.md
  line: 46
  category: parallel-safety
  severity: warning
  verdict: needs-revision
  origin: introduced
  message: "acceptance 8 checks a fixture store that is 'packed and committed', which is the state the pack verb of story:pack-and-unpack-verbs writes (pack-and-unpack-verbs.md:19). No depends_on edge runs to that story, and unlike story:pack-format-and-overlay-read:39 the body does not say the test writes the pack with the pack module's renderer. Both stories depend only on wave-1 items, so they land in the same wave."
```
