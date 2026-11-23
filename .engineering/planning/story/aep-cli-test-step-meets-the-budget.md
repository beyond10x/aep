---
format: aep.planning-md/3
id: story:aep-cli-test-step-meets-the-budget
kind: story
status: draft
title: The aep-cli test step meets the budget
summary: aep-cli tests run 84 s to 970 s each and two whole-crate runs hit the 30-minute slot limit; the step must end in 120 s with no test over 60 s.
owner: aep
tags:
- test-budget
relations:
- serves: vision:O2
revision: 1
---
# Story: The aep-cli test step meets the budget

## Outcome

`cargo test -p aep-cli` ends inside the 120 s step budget of `~/beyond10x/AGENTS.md` § Tests stay
fast, on a loaded host as on an idle one, and a test that would run past 60 s is killed by the
runner instead of holding a build slot until the slot limit.

## Context

Measured 2026-11-23 on a host with load average about 130-197 (the planning-store review,
`aep-planning-store-review`). Two whole-crate `cargo test -p aep-cli` runs were killed at the
30-minute slot limit on the second row.

| test or binary | time |
|---|---|
| `sqlite_plan::this_repositorys_plan_round_trips_through_sqlite_and_answers_the_markdown_backends_history` | 517 s and 682 s |
| `store_pack_end_to_end` `a_thousand_closed_stories_pack_and_unpack_with_every_answer_unchanged` | 970 s |
| `store_pack_end_to_end_adversary` `every_compared_answer_is_a_real_answer…` | 190 s |
| `planning_cli` binary | 117-146 s for 116-117 tests |
| `store_pack` binary | 94 s |
| `grouped_and_flat_spellings` binary | 84 s |

The shape is the one the rule names: an expensive product is made inside each test. Two of the
products are a store holding this repository's own plan and a store of 1000 closed stories.

## Acceptance

- `cargo test -p aep-cli` on a clean checkout ends within 120 s of wall time, and no single test
  runs over 60 s; the runner kills a test at 60 s and the step prints its ten slowest tests.
- The store of this repository's plan and the 1000-story store are each built once per run and
  read by every test that needs them (a cache on disk keyed by the digest of its sources, or a
  setup step), or the test that needs one moves to a lane named `slow` that runs only when its
  sources changed.

## Out of Scope

Raising a timeout, adding shards or waiting longer: the rule says none of them is a fix. Tests of
other crates, unless the same product is built per test there.

## Open Questions

- Which of the six rows is the cost of the product and which is the cost of the host load? Whoever
  picks this up times each on an idle host first, and names both times.
