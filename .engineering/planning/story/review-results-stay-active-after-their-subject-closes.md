---
format: aep.planning-md/3
id: story:review-results-stay-active-after-their-subject-closes
kind: story
status: draft
title: Review-results stay active after the work they reviewed closes
tags:
- from-finding
relations:
- informed_by: epic:store-pack-closed-artifacts
- serves: vision:O2
revision: 1
---
# Story: review-results stay `active` after the work they reviewed closes

Status: draft, filed from an operator observation. Not accepted, not scheduled.

## Observation

Operator's structure review (2026-11-10, not re-verified here): in one adopter repository 68 of 68
review-results are `active` while 154 of 253 stories are `implemented`. The question asked: is a
lifecycle step missing, or does the wave flow skip one?

## What was found (2026-11-10)

1. **The lifecycle says this is intended.** `artifacts/lifecycles/review-result.yaml`: "a review is
   recorded `active` … `archived` exists only to retire a review whose subject is gone". An
   `implemented` story is not gone, so a review of it is meant to stay `active`. No rung is missing:
   the ladder is `active -> archived`, and nothing in it ties a review-result's status to its
   subject's.
2. **The wave flow has no step that touches review-results' status.** `aep:implementing` 0.26.0,
   `references/wave.md`, *Close it*, step 4 moves "each story to its terminal status"; review-results
   are created (*Record each adversary pass as one `review-result`*) and never moved. A text search of
   that skill's references for an archiving step for review-results finds none. The planning skill
   0.26.0 says "`move --to archived` is the only way one is retired" and gives no occasion for it.
3. **This repository shows the same shape.** `aep plan artifact list --format json` on
   `origin/main` 26544d326: 61 review-results, all `active`; 44 of them review only artifacts that are
   `implemented`, `archived` or `rejected`, 17 review at least one open artifact.

Conclusion (inferred from 1–3): neither a missing rung nor a skipped step. The store has no notion of
a review that is *settled* because its subject closed, and nobody is told to archive one. The
consequence that matters now: `architecture-design:store-pack` packs archived review-results only, so
in that adopter none of the 68 would be packed.

## To decide before this leaves draft

One of: (a) the wave close archives a review-result once every artifact it `reviews` is closed (a
skill change in `agentplugins`, plus the move cost per review); (b) `pack` treats a review-result as
closed when every artifact it `reviews` is closed, without moving it (a design change to
`architecture-design:store-pack`, item 3); (c) leave it, and say in the lifecycle comment that
reviews of closed work stay `active`. `findings` and `review-value` read active and archived
review-results alike, so neither (a) nor (b) changes their answers.

## Acceptance

To be written with the decision.
