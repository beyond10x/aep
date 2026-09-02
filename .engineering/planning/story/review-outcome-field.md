---
format: aep.planning-md/1
id: story:review-outcome-field
kind: story
status: implemented
title: 'A review-result records what happened to it: no-op, fixed or escalated'
summary: A review_outcome evidence record referencing the review-result, refused when the review does not review the artifact.
owner: protocol
tags:
- evidence
- review
relations:
- decomposes: epic:review-facts
- serves: vision:O2
revision: 4
---
# Story: A review-result records what happened to it: no-op, fixed or escalated

## Outcome

Every critic verdict and every adversary finding ends with a recorded outcome, so a later question — did this lens ever change anything — has data behind it.

## Context

A `review-result` is immutable, so the outcome is a separate record that references it. The wave skill and the critic step (in `agentplugins`) write it when they act on a review: `no-op` when nothing changed, `fixed` when the implementor addressed it, `escalated` when it went to the operator.

## Acceptance

- `aep artifact evidence <reviewed-id> --kind review_outcome --review <review-result-id> --outcome no-op|fixed|escalated [--ref …]` records it; the kind is added to the closed evidence-kind list and its help text.
- A `review_outcome` naming a `review-result` that does not review the artifact is refused.
- `aep artifact show <review-result-id>` prints its outcomes.
- `validate` reports (without failing) a `review-result` older than N days with no outcome, N configurable, default 14.

## Out of Scope

Inferring the outcome from a diff.

## Ambiguities

- `inferable` — the evidence-kind list is closed and printed on refusal (`aep artifact evidence --help`), so the new kind must be declared where the others are.

## Open Questions

None.
