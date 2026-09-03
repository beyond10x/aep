---
format: aep.planning-md/1
id: story:review-lens-value
kind: story
status: draft
title: Each review lens reports whether it earned its cost
relations:
- decomposes: epic:adopter-feedback-round-2
scope:
- confidence: inferred
  path: crates/edge/protocol-cli
- confidence: inferred
  path: crates/govern/aep-domain
revision: 4
---
# Story: Each review lens reports whether it earned its cost

## Outcome

`aep eval` can tabulate, per review lens, how often its verdict was `no-op`, `fixed` or `escalated`,
so a lens that never changes anything can be dropped with a number behind the decision.

## Context

Filed from the 2026-09-02 review of a third-party plugin, which records review value per checkpoint.
Depends on `review-result` artifacts carrying an outcome. Not scheduled.

## Acceptance

- An `aep eval` table over `review-result` artifacts with those three counts per lens.

## Out of Scope

Deciding which lenses to keep.

## Open Questions

Whether outcome is a field on `review-result` or a later evidence record — operator decides.
