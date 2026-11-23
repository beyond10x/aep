---
format: aep.planning-md/3
id: story:write-idempotency-keys-are-injective
kind: story
status: draft
title: Write-command idempotency keys are injective
summary: envelope_for folds ':' and '/' into '-', so two ids that differ only in a separator could share one SQLite idempotency key.
owner: aep
tags:
- planning-store
relations:
- serves: vision:O2
revision: 1
---
# Story: Write-command idempotency keys are injective

## Outcome

Two artifact ids that differ only in `:` against `/` or `-` give two different command names, so no
two writes of the CLI can share one SQLite idempotency key.

## Context

`envelope_for` in `crates/edge/aep-cli/src/planning.rs` builds the command, request and idempotency
names from `name.replace([':', '/'], "-")`. `review-result:x` and `review:result-x` both become
`review-result-x`, and the second would be refused as a reuse of the first's idempotency key. The
same replacement stands at the call sites that build `move-{id}`, `{source}-{relation}-{target}`
and `{correlation}-{id}`.

The CLI cannot create such a pair today: every id it writes is `<kind>:<name>`. The encoding is
still not injective, and a store written by hand or by another tool can hold one.

PR #165 (`fix/artifact-identity-collision`) introduced an injective encoding in
`aep-backend-memory/src/seed.rs` (`token`, with `/` between the parts of a relation name). This
story uses that encoding and does not invent a second one.

## Acceptance

- A test writes to two artifacts whose ids differ only in a separator (`review-result:x` and
  `review:result-x`) and both writes succeed, each with its own idempotency key; before the change
  the second is refused.

## Out of Scope

The seeding path of `aep-backend-memory`, which PR #165 owns.

## Open Questions

- Does the encoding live in one shared function both crates call, or does `aep-cli` keep a copy?
  Whoever picks this up decides after #165 has merged, and prefers the shared function.
