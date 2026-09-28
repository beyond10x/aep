---
format: aep.planning-md/3
id: story:invariant-citations-match-agents-md
kind: story
status: implemented
title: Code comments cite AGENTS.md invariants by name, not by a number that has moved
relations:
- serves: vision:O2
revision: 4
transitions:
- {from: "draft", to: "proposed", at: "2026-09-28T14:22:54Z", actor: "human:timo", revision: 2}
- {from: "proposed", to: "active", at: "2026-09-28T14:22:54Z", actor: "human:timo", revision: 3}
- {from: "active", to: "implemented", at: "2026-09-28T18:39:08Z", actor: "human:timo", revision: 4, decided_on: {"recorded":{"test_result":1}}}
---
## Problem

0.64.0 deleted invariant 10 and renumbered 11–15 to 10–14. Code comments across `crates/**` cite
numbers from older lists (up to 16), for example `write_surface.rs` calls invariant 14
"one write path", which no current invariant is.

## Outcome

Every code comment that cites an AGENTS.md invariant cites it by name; a test fails on
`invariant <number>` in a comment.

## Acceptance

- `git grep -n 'invariant [0-9]' -- crates` returns nothing, and a gate check keeps it that way.
