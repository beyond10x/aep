---
format: aep.planning-md/3
id: story:command-failure-atomicity
kind: story
status: implemented
title: A refused command changes no semantic state
summary: Stage memory mutations and record exactly one refusal audit.
relations:
- derived_from: epic:architecture-hardening
- serves: vision:O2
revision: 4
transitions:
- {from: "draft", to: "proposed", at: "2026-08-30T22:08:50Z", actor: "human:operator", revision: 2, imported: true}
- {from: "proposed", to: "active", at: "2026-08-30T22:08:50Z", actor: "human:operator", revision: 3, imported: true}
- {from: "active", to: "implemented", at: "2026-08-30T23:50:40Z", actor: "human:operator", revision: 4, decided_on: {"recorded":{"test_result":1}}, imported: true}
---
## Finding

`crates/aep-backend-memory/src/command.rs` mutates an ADR before checking that its superseded target exists, so a refused `AcceptAdr` can advance state and revision.

## Acceptance

Every command executes against candidate state. A refusal publishes no entity, relation, event, history, revision or idempotency change and appends exactly one rejection audit. A regression reaches the missing-superseded-ADR path and a mutation removing the candidate-state boundary makes it fail.

## Scope

- `crates/aep-backend-memory/` — cited from the review.
- shared backend conformance tests — inferred; confirm before editing.
