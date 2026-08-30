---
format: aep.planning-md/1
id: story:command-failure-atomicity
kind: story
status: implemented
title: A refused command changes no semantic state
summary: Stage memory mutations and record exactly one refusal audit.
relations:
- derived_from: epic:architecture-hardening
- serves: vision:O2
revision: 4
---
## Finding

`crates/aep-backend-memory/src/command.rs` mutates an ADR before checking that its superseded target exists, so a refused `AcceptAdr` can advance state and revision.

## Acceptance

Every command executes against candidate state. A refusal publishes no entity, relation, event, history, revision or idempotency change and appends exactly one rejection audit. A regression reaches the missing-superseded-ADR path and a mutation removing the candidate-state boundary makes it fail.

## Scope

- `crates/aep-backend-memory/` — cited from the review.
- shared backend conformance tests — inferred; confirm before editing.
