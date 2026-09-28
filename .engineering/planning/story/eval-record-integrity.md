---
format: aep.planning-md/3
id: story:eval-record-integrity
kind: story
status: implemented
title: An eval record cannot omit the facts that bind its verdict
summary: Require identity, digests and expectations and always verify them.
relations:
- derived_from: epic:architecture-hardening
- serves: vision:O3
revision: 4
transitions:
- {from: "draft", to: "proposed", at: "2026-08-30T22:08:50Z", actor: "human:operator", revision: 2, imported: true}
- {from: "proposed", to: "active", at: "2026-08-30T22:08:50Z", actor: "human:operator", revision: 3, imported: true}
- {from: "active", to: "implemented", at: "2026-08-30T23:50:47Z", actor: "human:operator", revision: 4, decided_on: {"recorded":{"test_result":1}}, imported: true}
---
## Finding

The eval runner's raw record defaults required producer fields to absence and conditionally checks digests, so a truncated record can bypass the comparisons that bind a verdict to its specification and stream.

## Acceptance

Conversion accumulates stable `EVAL-RECORD-*` refusals for missing, null, empty or malformed specification id, specification digest, transcript digest and expectations. Expectations are non-empty and both digests are always checked. An absent or null verdict remains `Unknown`. One focused case covers each omitted key and a mutation deleting a required-field check turns the suite red.

## Scope

- `crates/protocol-cli/src/eval.rs` and eval fixtures/tests — cited from the review.
