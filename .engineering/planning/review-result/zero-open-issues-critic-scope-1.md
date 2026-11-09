---
format: aep.planning-md/3
id: review-result:zero-open-issues-critic-scope-1
kind: review-result
status: active
title: Plan critic, scope, round 1 (zero-open-issues epic)
relations:
- reviews: epic:zero-open-issues-2026-11-09
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
approve

**What I read:** 20 artifacts (the epic, 10 DRAFT stories, 9 closed artifacts) and the two issue texts.
- Store: `aep plan artifact show` on each, plus `graph`, `relations` and `list`; also `show` on `approval-record:zero-open-issues-acceptance-2026-11-09`, `decision-blocker:how-the-checks-map-finds-its-trace-specification` and four defect-like drafts outside the set.
- Issues: `gh issue view 152`, `gh issue view 153`, `gh issue list --state open`.
- Source checks: `rg` for the symbols the closed artifacts cite, plus the code at `planning.rs:219`, `planning.rs:685`, `conformance_count.rs:48`, `project.rs:556`, `project.rs:584`, `planning.rs:7626` and `aep-backend-entity/src/lib.rs:543`.
- Epic body: I extracted 9 promises and traced all 9 to an item.
  1. #152 closed: `story:an-artifact-id-can-be-renamed`.
  2. #153 closed: `story:relate-and-unrelate-write-the-revision-they-print`.
  3. Each still-present defect is implemented: the eight other drafts.
  4. Each fixed or deleted defect is closed on proof: the nine `## Closed` artifacts.
  5. One pull request carries it all: delivery of the whole set, not a story outcome.
  6. The `gh issue list` acceptance: covered by 1 and 2.
  7. The `decomposes` acceptance: covered by the set.
  8. The `validate` acceptance: covered by the set.
  9. Not in it: the epic's exclusions, which no drafted item reaches into.
- Reach beyond the parent: none found.
- Double claims: `story:history-shows-a-removed-edge` and `story:relate-and-unrelate-write-the-revision-they-print` both concern removal history. The first owns SQLite and Postgres; the second excludes them by name and owns the Git-native store.
- Narrowing: none silent.
  - `story:a-directory-and-a-file-inside-it-read-as-disjoint` records that it does not build the file-and-directory-of-one-name refusal, and why.
  - `story:an-artifact-id-can-be-renamed` names SQLite and Postgres as refused, and names `retire` as out of scope.
- Closed artifacts: the proof supports closing all nine.
  - Obsolete ones: the cited symbols (`aep-stage`, `stage_directory`, `eventlog_invocation`, `SemanticMismatch`, `projection_current`, `read_owned_files`, `receipt_conflict`, `writer-control`, `hold.sock`, `pre_provider`) match nothing under `crates/` or `xtask/`, and `integrations/` does not exist.
  - Fixed ones: `document_root` and `judged_evidence` exist as described, as do their tests `planning_documents_follow_the_protocol_tree_named_by_the_project` and `a_failed_report_1_does_not_move_a_specification_to_conforming` / `a_failed_report_2_does_not_move_a_specification_to_conforming_and_says_why`.
- The new `depends_on` from the rename story to the relate story adds no coverage question.

**What I could not establish:**
- I did not read the drafter's report. The decomposition names its other omissions only in the approval record and the open decision-blocker, not in the epic body.
  - Omissions: `story:checks-map-trace-spec-path` (blocked by an open decision-blocker), the four cross-repository drafts, and `story:journal-actor-from-configuration` plus `story:workspace-declaration-has-one-reader-tests-included` as non-defects.
  - I treated these as named omissions, so they are not gaps. The epic's "Not in it" lists only two exclusions, so adding them there would help a reader. That is not a coverage defect.
- I did not triage the other roughly 70 drafts. I trusted the epic's statement that all 94 were read at `824869fec`, and spot-checked only six: `a-dropped-stream-does-not-end-a-run`, `a-stale-binary-refuses-itself`, `workspace-declaration-has-one-reader-tests-included`, `gate-lanes-count-what-ran`, `cold-start-outlives-the-deadline`, `claim-retirement`.
- Out of my lane, not counted in the verdict:
  - `story:published-pattern-residue` is titled "Three" but its body and acceptance cover four types, and `ArtifactId` was not measured at triage. That belongs to the acceptance critic.
  - `story:agents-dependency-rule-matches-the-manifests` pulls against the operator's global no-comments rule. The story says so itself.
  - `story:discarded-protocol-source-tree-is-removed` acceptance 4 names the macOS CI job. The `macos` job exists in `.github/workflows/ci.yml`; I did not check it runs the new tests.

```findings
[]
```
