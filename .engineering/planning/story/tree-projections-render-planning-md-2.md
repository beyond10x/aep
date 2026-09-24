---
format: aep.planning-md/2
id: story:tree-projections-render-planning-md-2
kind: story
status: implemented
title: A tree store renders aep.planning-md/2, and validate skips V2 against a base without one
relations:
- decomposes: epic:planning-on-entity-runtime
- serves: vision:O2
revision: 4
---
## Outcome

`validate --against` a revision without a tree store skips V2 with one line; an `aep.project/3`
tree store renders `aep.planning-md/2` and an `aep.project/2` store keeps `/1`, with S5 comparing
each document against the render of its own tag.

Design: `docs/design/planning-on-entity-runtime-v0.1.md` § 8 and § 13 A5.

## Decisions

- `/2` is the tag change only. § 3.2's `head` key has no defined value and is not written.
- A `/1` projection of a tree store stays valid until its next write re-renders it; each
  repository gets one `aep plan artifact render` commit when its planning check moves to the
  release that renders `/2`.
