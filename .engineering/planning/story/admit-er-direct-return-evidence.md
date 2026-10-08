---
format: aep.planning-md/3
id: story:admit-er-direct-return-evidence
kind: story
status: draft
title: Admit Entity Runtime direct-return evidence and earn conformance from current coverage
revision: 1
---
# Story: Admit Entity Runtime direct-return evidence and earn conformance from current coverage

## Outcome

`aep plan artifact evidence --from` admits Entity Runtime's direct-return ESS suites, and an
`executable-system-specification` moves from `validated` to `conforming` on admitted current
coverage.

## Context

The work exists, unmerged, on branch `feat/er-direct-return-evidence` (head
`ba0d7ec714b662963ac1a77a45ca4b8e2751da69`), from draft pull request 61, closed on 2026-10-08
unmerged. Read on 2026-10-08 against `main` at 0.69.1: 4 of its 7 commits are not on `main`;
+1960 −54 lines outside fixtures and logs (12 source `.rs` files), plus a 107k-line fixture
`suite.json`; 8 paths conflict with `main`; its base is 83 commits behind.

What the branch adds:

- the closed ER return profile for `suite/28` and the inventory `suite/29`
  (`crates/observe/aep-ess-evidence/src/direct_response.rs`);
- retention of the exact report and suite originals, and re-admitting them when lifecycle
  eligibility is evaluated (`crates/observe/aep-ess-evidence/src/planning_coverage.rs`);
- only the latest complete, whole-system, all-origin, nonempty passing coverage for the current
  model qualifies; a newer failed or partial run vetoes an older success.

The branch names two stories, `story:admit-er-direct-return-evidence` and
`story:bind-current-coverage-to-specification-lifecycle`, that are not in this store; this story
holds both.

## Acceptance

- The ER release pair's suite and report are admitted by `aep plan artifact evidence --from`.
- A specification moves `validated -> conforming` on that admitted coverage, and a newer failed or
  partial run blocks the move.
- Delivered by replaying the branch onto current `main` as new commits, not by merging it.
