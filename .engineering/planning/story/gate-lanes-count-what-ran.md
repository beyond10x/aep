---
format: aep.planning-md/1
id: story:gate-lanes-count-what-ran
kind: story
status: draft
title: A gate lane reports how many cases ran, and a unit's own contract is run against its code
summary: A lane that selects 8 of 58 cases is green; a unit that ships a vector and code that breaks it is green; six review documents name the class. Count what ran; run the unit's own contract.
tags:
- gate
- process
relations:
- decomposes: epic:evidence-gated-completion
- serves: vision:O6
scope:
- confidence: inferred
  path: .engineering/checks
- confidence: cited
  path: Taskfile.yml
- confidence: inferred
  path: conformance
- confidence: cited
  path: docs/reviews/2026-08-20-guard-efficacy-review.md
- confidence: cited
  path: docs/status.md
revision: 6
---
# Story: A gate lane reports how many cases ran, and a unit's own contract is run against its code

## Outcome

A green exit with fewer executed cases than before fails the gate, and a unit that ships a contract
vector and the code it describes in one commit has that vector run against the code before merge.

## Context

The recurring theme of six review documents (`docs/reviews/2026-08-20-guard-efficacy-review.md:16-18`,
`2026-08-26-post-release-review.md:110-115`, `2026-08-20-full-repo-review.md:64-69`, Pre-Wave-4 `:291-299`,
`2026-08-24-scope-cache-and-the-native-arm.md:111-112`, `2026-08-30-wave-3-retro.md:160-173`) and four sessions:

- `6f1be0d9#172`: `scripts/delegated-lane.sh` selected host cases by substring; the lane ran 8 of 58; one of the
  50 that never ran was red.
- `6f1be0d9` addendum 1: a vector asserting `state: cancelled` shipped with code returning `exited`, every
  gate step green — `check-bundle` proves the bundle is a fixed point of its source, not that the daemon obeys it.
- `b9f1e943#777`: four of six `check-brand.sh` guards "printed 'clean' and exited 0 against any input".
- `ed007513#1959`: `cargo xtask claims` — "1 claim(s) checked, 21 unverifiable, 0 finding(s)".
- GUARD decision D3 (`:367-373`) — a rejection test per primitive mapping row — has no answer recorded.

## Acceptance

- `task test` prints `executed: <n>` per lane (`cargo test` summary parsed, not estimated) and
  `test-count-check` fails when the count is below the number recorded in `docs/status.md`'s generated
  region; a drop is accepted by editing the record in the same commit that removes the tests.
- A `contract-check` step runs every vector under `examples/*/contracts/**` or `suites/**` that the
  commit adds or changes against the implementation the suite names, and fails on a mismatch.
- D3 is answered in `docs/reviews/2026-08-20-guard-efficacy-review.md` § Decisions with the option taken
  and the commit that took it.

## Out of Scope

Mutation testing as a gate step (`.engineering/checks/check-mutation-proof.sh` stays out of the gate).
