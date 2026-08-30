---
format: aep.planning-md/1
id: story:refusals-name-what-to-do-next
kind: story
status: implemented
title: A refusal names the next thing to type, in words the reader has
summary: A rule path leaks into operator text, explain stops before the next rung, an evidence-kind refusal names no nearest kind, and move panics on an unknown condition operator.
tags:
- cli
- store
relations:
- decomposes: epic:adopter-feedback-round-1
- serves: vision:O2
revision: 4
---
# Story: A refusal names the next thing to type, in words the reader has

## Outcome

Four refusals stop leaking internals or stopping short.

## Context

| # | today | evidence |
|---|---|---|
| a | "reaching implemented needs at least 1 test_result record(s). Nothing was presented at `$args.evidence.test_result`" — a rule path in operator text (`crates/aep-backend-markdown/src/document.rs:413`) | `ed007513#1138`; `e70b8018 s1#1748` |
| b | `explain` says "no status move is recorded" and nothing about what the next rung needs; the requirement is learnt by being refused twice | `11727595#3402-#3407` |
| c | an evidence-kind refusal lists all 15 kinds and stops; the nearest kind for an observation of a live system or a cross-repo dependency is not named | `431986de#6957` (`measurement`); `e70b8018 s1#694` (`cross_repo_dependency`) |
| d | `move` **panics** with a backtrace when a lifecycle names a condition operator the pinned kernel lacks — `crates/aep-backend-entity/src/kernel.rs:226` `unwrap_or_else(… panic!)` | `9da4f51c#1852` `unknown condition operator 'after'` |

## Acceptance

- (a) reads "no `test_result` record is held for this artifact; `protocol artifact evidence <id> --kind test_result …` records one".
- (b) `explain` ends with `next: <status> needs <n> <kind> record(s); held: <m>` for each legal next rung.
- (c) the refusal ends with "for an observation of a running system use `health_observation`; for a
  relation to another store's artifact use `artifact`".
- (d) the definition is built with `?` and `move` refuses: "this lifecycle uses `<op>`, which the kernel
  this build pins (`entity-core <v>`) does not know; raise the pin or drop the guard".

## Out of Scope

New evidence kinds.
