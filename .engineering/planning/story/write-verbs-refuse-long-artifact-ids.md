---
format: aep.planning-md/3
id: story:write-verbs-refuse-long-artifact-ids
kind: story
status: implemented
title: 'Write verbs refuse two long artifact ids: the derived idempotency key exceeds 200 characters'
relations:
- serves: vision:O2
scope:
- confidence: cited
  path: crates/edge/aep-cli/src/planning.rs
- confidence: cited
  path: crates/edge/aep-cli/tests/long_artifact_ids.rs
- confidence: cited
  path: crates/govern/aep-domain/src/ids.rs
- confidence: cited
  path: crates/plan/aep-backend-memory/src/seed.rs
revision: 7
transitions:
- {from: "draft", to: "proposed", at: "2026-11-10T18:04:10Z", actor: "human:timo", revision: 3}
- {from: "proposed", to: "active", at: "2026-11-10T18:04:11Z", actor: "human:timo", revision: 4}
- {from: "active", to: "implemented", at: "2026-11-10T20:16:06Z", actor: "human:timo", revision: 7, decided_on: {"recorded":{"test_result":1,"review_outcome":1,"verification":1}}}
---
## Problem

Some write verbs refuse two legal artifact ids when the ids are long. `aep plan artifact relate` is one. It builds its idempotency key as `rel-<source>-<relation>-<target>-<epoch millis>`, with `:` and `/` turned into `-`, in `crates/edge/aep-cli/src/planning.rs` (the `envelope` block of the relate verb, about line 3118). It builds the request id as `req-<name>` and the command id as `cmd-<name>`. All three are `Loose` identifiers capped at `MAX_LENGTH` = 200 characters (`crates/govern/aep-domain/src/ids.rs:45`, checked at :90). An artifact id may itself be up to 200 characters. So a pair whose combined length goes over about 173 characters cannot be related, and the CLI refuses with "invalid idempotency key" (the length reason is read from the code: `validate` in ids.rs).

`new-<id>`, `move-<id>`, `evidence-<id>-<kind>-<millis>`, `redact-<id>-<millis>`, `crossings-<id>-…` and `unrelate` are built the same way (`envelope_for`, about line 2096), so a long enough id hits the same refusal on those verbs.

## Reproduction (observed 2026-11-10, aep 0.79.0, ess repository store)

- Source: `review-result:zero-issues-w1-a-create-or-update-writer-is-no-proof-a-stored-reference-is-present-adversary-1` (108 characters).
- Target: `story:a-create-or-update-writer-is-no-proof-a-stored-reference-is-present` (73 characters).
- The key is `rel-` + source + `-reviews-` + target + `-` + 13 digits = 208 characters, over the cap of 200.
- `aep plan artifact relate <source> reviews:<target>` refuses with "invalid idempotency key".
- The review-result is committed and archived, and it has no `reviews` relation, so `validate` exits 1 with `[empty_declaration] … does not say what it reviews`.
- No verb repairs it: `relate` refuses, `new --relate` only acts at creation, `redact` does not touch relations, and `archived` is terminal for a review-result. The only remaining fix is a hand edit, which guardrail 2 forbids.

## Fix

- One helper derives every envelope identifier from its name. It strips `:` and `/` as today. When the result would go over `MAX_LENGTH`, it keeps a readable prefix and appends `-` plus the first 16 hex characters of the SHA-256 of the full name.
- This keeps the identifier deterministic, so a replay is still recognised, and keeps it under the cap.
- Every verb that builds `req-`, `cmd-` or a key from artifact ids uses it.
- The test comes first: a CLI test relating two ids of 120 and 100 characters, which is red today.

## Acceptance

- `relate`, `unrelate`, `new`, `move`, `evidence` and `redact` succeed on artifact ids up to the 200-character id limit, and a replay of the same command is still recognised.
- No identifier changes for names at or under the cap, so existing stores and replays are unaffected.
