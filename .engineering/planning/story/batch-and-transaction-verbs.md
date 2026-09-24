---
format: aep.planning-md/2
id: story:batch-and-transaction-verbs
kind: story
status: draft
title: 'Batch and transaction verbs: N store writes under one validation'
summary: Every write re-validates the whole store in its own process (~1 s at 330 artifacts); a migration of 1,700 writes takes 25 minutes. Add a batch/transaction verb.
tags:
- adopter-feedback
- performance
relations:
- decomposes: epic:adopter-feedback-round-1
revision: 2
---
## Context

Migrating an adopter's backlog into the store (the excluded adopter-app, 2026-09-10: 253 stories, 54 epics,
16 designs, 330 artifacts, 853 relation lines) took two passes of roughly 1,700 `aep plan artifact`
invocations and about 25 minutes of wall clock. Every write is a fresh process that reads the whole
store, re-validates it, and writes one document; there is no verb that takes N writes under one
validation.

Measured on that store (`time`, store copy, `aep` protocol 0.54.0):

| operation | cost | note |
|---|---|---|
| `artifact list` | 0.07 s | reads all 330 files |
| `artifact validate` | 1.74 s, 99 % CPU | 36 artifacts: 0.12 s · 162: 0.61 s · 214: 0.73 s · 330: 1.74 s |
| `artifact move` / `relate` / `set` (one write) | 0.96–1.0 s | journal length irrelevant (0 vs 1,099 lines: same) |
| refused `artifact new` on an existing id | 1.26 s | validation runs before the refusal |
| syscalls per write | 18,106 in 13 ms | I/O is not the cost |
| files opened per write | 3,405 — of which 466 are the pinned protocol snapshot's own `.engineering/planning/story/*.md`, `website/blog/*.md`, `docs/plan/*.md` | the snapshot's unrelated markdown is walked on every command |

Hypothesis (not verified in the source): validation time grows faster than the artifact count above
~200, so the cost is in the relation / objective graph checks rather than parsing.

## Acceptance

A batch verb — `aep plan artifact batch --from <ndjson|yaml>` or a transaction pair
(`begin` … `commit`) — applies N creates / relates / moves / evidence records under **one** load and
**one** validation, writes all-or-nothing, and journals each entry as if issued singly; re-running the
same batch is refused per entry exactly as single verbs are today. On the the excluded adopter store the
migration's 1,700 writes complete in under two minutes (measured, in the story's Progress), and a batch
with one invalid entry leaves the store byte-identical to before.

## Notes

- Secondary finding, same measurement: each command opens the protocol snapshot's blog, docs and its
  own planning store (466 files) — none of which the adopter's store needs. Worth its own story if it
  is not a side effect of the same loader.
- Adopter record: `the excluded adopter-app` `migration-plan:track-to-aep` (its *Result* section carries
  the numbers above).

