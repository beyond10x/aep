---
format: aep.planning-md/3
id: review-result:zero-open-issues-critic-parallel-safety-2
kind: review-result
status: active
title: Plan critic, parallel-safety, round 2 (zero-open-issues epic; round 1 was run by the coordinator, not recorded)
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

story:a-directory-and-a-file-inside-it-read-as-disjoint — its own triage says the `waves --help` text changes with the fix, and that text is a doc comment in `crates/edge/aep-cli/src/planning.rs:1117-1120` and `:1153-1155` (cited by the body, found by grep). Its scope leaves `planning.rs` out, so `waves` puts it in wave 0 with no collision, even though relate, rename, relations-order and legacy-ess all claim that file. Remedies: an ordering edge naming `planning.rs`, or splitting the help-text change out. `website/docs/concepts/waves.md:42-44` is also missing from scope (inferred). — .engineering/planning/story/a-directory-and-a-file-inside-it-read-as-disjoint.md:63
story:history-shows-a-removed-edge — the body names `change_of_event`, which greps to nothing. The code that reads a stored event back is `entry_from_event` at `crates/edge/aep-cli/src/planning.rs:8224`, where `remove-relation` falls to `_ => return None` at `:8306`. The removal wording lives in `crates/plan/aep-backend-markdown/src/journal.rs:356`, which the relate story claims. Neither file is in this story's scope, so `waves` puts it in wave 0, apart from every other story on `planning.rs` (inferred). Remedies: an ordering edge naming those files, or splitting the surface. — .engineering/planning/story/history-shows-a-removed-edge.md:29
story:history-shows-a-removed-edge — its acceptance puts the removed-edge assertion in the shared conformance suite. That suite also runs against the Git-native markdown backend (`crates/plan/aep-backend-markdown/tests/conformance.rs:30-37`). That backend's `history` goes through the entity backend (`backend.rs:191-192`), and story:relate-and-unrelate-write-the-revision-they-print produces its removal record. No `depends_on` edge runs from this story to relate, and `waves` schedules history first, in wave 0 against relate's wave 16 (inferred). Remedy: add that edge, or merge the two stories. — .engineering/planning/story/history-shows-a-removed-edge.md:38-40, .engineering/planning/story/relate-and-unrelate-write-the-revision-they-print.md:91-93
story:legacy-ess-report-calendar-refusal — scope declares the whole directory `crates/edge/aep-cli/tests` next to its own file `tests/ess_report_calendar.rs`. Once story:a-directory-and-a-file-inside-it-read-as-disjoint lands its prefix matching, that directory collides with the test files of five other stories in the set: `git_store.rs`, `rename.rs`, `wave_derivation.rs`, `store_selection.rs` and `sqlite_plan.rs` (inferred). Remedy: drop the directory entry. — .engineering/planning/story/legacy-ess-report-calendar-refusal.md:14-15

**What I read:**
- All 10 story files, in full.
- `aep plan artifact waves --kind story --format json`, filtered to pairs inside the set: 11 collisions, 0 unassessed, 0 cycles.
- `git grep` and `sed` over `planning.rs`, `aep-backend-entity/src/lib.rs`, `aep-backend-markdown/{backend,journal}.rs`, the conformance crate and test runners, `store_selection.rs`, `waves.md`, `cli.md` and `ci.yml`.
- Surfaces placed: 10 cited, 2 with extra inferred surfaces I added (history, a-directory), 0 unplaceable.
- Acceptance reads traced to a producer in the set: 3.
  - rename depends on relate's record mechanism: a `depends_on` edge exists.
  - history's conformance test depends on relate: no edge.
  - relations-order's Git-native order depends on relate's write path: kept apart only by the scope collision on `planning.rs` and the design doc, not by an edge.
- So 1 of 3 reads has a `depends_on` edge.

**What I could not establish:**
- Whether relate's fix to the write path moves where a re-added edge lands in the Git-native file. relations-order pins that file as the authority. No finding, because the scope collision already keeps the two out of one wave.
- Whether legacy-ess ends up editing `aep-backend-entity/src/lib.rs:224`. Its body says read-only "unless" a backend policy is chosen, and the typed scope leaves the file out. If it does, it collides with history and relations-order.
- Rename's acceptance (history lists the rename) almost certainly needs `journal.rs` (`history_git`), which is not in its scope. Not a finding, because its `depends_on` edge to relate already orders it after the story that claims `journal.rs`.
- Out of my lane, design: relate's title and history's title both claim "history shows a removed edge".
- Out of my lane, acceptance/design: history's body names a function, `change_of_event`, that does not exist.
- `published-pattern-residue`'s crate-wide entries (`crates/govern/aep-domain`, `crates/drive/aep-driver-spec`) add no collision inside this set.

```findings
- file: .engineering/planning/story/a-directory-and-a-file-inside-it-read-as-disjoint.md
  line: 63
  category: parallel-safety
  severity: warning
  verdict: needs-revision
  origin: introduced
  message: "its own triage says the waves --help text changes with the fix; that text is in crates/edge/aep-cli/src/planning.rs:1117-1120 and :1153-1155 (cited by body, found by grep), scope omits planning.rs, so waves places it in wave 0 with no collision against relate, rename, relations-order and legacy-ess, which all claim that file; remedies: an ordering edge naming planning.rs, or splitting the help-text change out (website/docs/concepts/waves.md:42-44 also unscoped, inferred)"
- file: .engineering/planning/story/history-shows-a-removed-edge.md
  line: 29
  category: parallel-safety
  severity: warning
  verdict: needs-revision
  origin: introduced
  message: "inferred surface: the event reader is entry_from_event at crates/edge/aep-cli/src/planning.rs:8224 (remove-relation falls to None at :8306; the change_of_event the body names greps to nothing) and the removal wording is in aep-backend-markdown/src/journal.rs:356, which relate claims; scope names neither, so waves puts it in wave 0 apart from every planning.rs story; remedies: an ordering edge naming those files, or splitting the surface"
- file: .engineering/planning/story/history-shows-a-removed-edge.md
  line: 38
  category: parallel-safety
  severity: warning
  verdict: needs-revision
  origin: introduced
  message: "inferred: its acceptance puts the assertion in the shared conformance suite, which also runs against the Git-native markdown backend (aep-backend-markdown/tests/conformance.rs:30-37, history via backend.rs:191-192), whose removal record story:relate-and-unrelate-write-the-revision-they-print produces; no depends_on edge runs from this story to relate, and waves schedules history first (wave 0 against relate's wave 16)"
- file: .engineering/planning/story/legacy-ess-report-calendar-refusal.md
  line: 14
  category: parallel-safety
  severity: warning
  verdict: needs-revision
  origin: introduced
  message: "inferred: scope declares the directory crates/edge/aep-cli/tests beside its own file tests/ess_report_calendar.rs; once story:a-directory-and-a-file-inside-it-read-as-disjoint lands prefix matching, that directory collides with five other stories' test files (git_store.rs, rename.rs, wave_derivation.rs, store_selection.rs, sqlite_plan.rs); remedy: drop the directory entry"
```
