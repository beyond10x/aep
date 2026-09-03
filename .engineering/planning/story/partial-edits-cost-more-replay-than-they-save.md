---
format: aep.planning-md/1
id: story:partial-edits-cost-more-replay-than-they-save
kind: story
status: draft
title: Writing a body in parts costs more replay than writing it whole
summary: The scoped arm made 15 edits where the unscoped one made 6 writes; the store rule is right, but the way to satisfy it cheaply is unmeasured.
owner: harness
tags:
- eval
- harness
relations:
- decomposes: epic:self-evaluation
scope:
- confidence: inferred
  path: crates/drive/aep-driver
- confidence: cited
  path: crates/drive/aep-driver-spec
- confidence: inferred
  path: docs/reviews/2026-08-24-scope-cache-and-the-native-arm.md
revision: 6
---
# Story: the cheap way to honour the store rule

## Outcome

A run that respects the planning store's frontmatter does not pay twice for it. Today honouring the
rule is what makes the run expensive, which is an argument somebody will eventually use against the
rule.

## Context

Measured on 2026-08-24. The unscoped run wrote six times, three of them whole-file rewrites of
artifacts; the scoped run made fifteen edits and no rewrites. Both produced the same artifacts. The
scoped run's conversation reached 62,680 tokens against 47,990, and cost $0.99 against $0.66.

An edit has to quote enough surrounding text to be unambiguous — the tool refuses a match that appears
zero times or several — so writing a body in five parts carries the neighbourhood five times, and a
stateless loop replays all of it on every later turn. The rule is right; the mechanism for satisfying
it is the crude one.

The skill file says there is no CLI verb for prose and there should not be one, and that stands. What
is unexamined is whether the *tool* can offer a whole-body write that leaves frontmatter alone — a
write below the fence is still not a whole-file replacement, and `partial-only` is a statement about
granularity rather than about which call is used.

## Acceptance

- The cost of the two shapes is measured on the same case, not argued about.
- If a below-the-fence write is added, `partial-only` admits it and a whole-file write is still
  refused, with a test for each.
- The eval re-runs and the difference in replayed tokens is recorded in the review document.

## Out of Scope

Changing what the store rule protects. Frontmatter stays the CLI's.

## Open Questions

- Does a below-the-fence write belong in the tool catalogue or in the planning CLI? The skill says the
  CLI should not grow a prose verb; that argument may or may not cover a tool. Decided before any code
  is written.
