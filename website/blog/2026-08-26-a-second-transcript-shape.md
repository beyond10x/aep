---
title: "0.22.0 — a second harness, and what it immediately found"
description: >
  One trace-spec/1 specification now decides two transcript shapes, so harness neutrality stops
  being a claim with one case behind it. Plus three prose counts re-derived from the commands that
  print them.
slug: a-second-transcript-shape
tags: [release, trace, portability]
---

Every behavioural document here is published as **harness-neutral**, and until this release exactly
one transcript adapter existed. So *"neutral"* was a claim with **no second case behind it**.

A vocabulary tested against one harness is a vocabulary shaped like that harness, and **nobody can
tell which from the inside**.

{/* truncate */}

## The second case

A reader for a second transcript shape, so one `trace-spec/1` specification now decides both. The
specification did not change to accommodate it, which is the result the exercise was for.

Two things stated plainly rather than implied, because the difference matters:

* the committed fixtures are **synthetic**, and every place they are used says so;
* the **verified** reader for that harness's transcripts lives in
  [metaharness](https://github.com/beyond10x/metaharness), checked against thousands of real
  rollout files. This one is a **neutrality probe on constructed bytes** — it answers the vocabulary
  question and does not claim to read anybody's transcripts in production.

The claim this release earns is precise: **the vocabulary is neutral.** The *driving* path still has
never met a second harness, and the limitations page says so.

## Three counts, re-derived

Three prose literals had drifted from the counts their own gates print. Each was corrected by
**running the command beside it** rather than by picking the number that looked right:

| claim | was | is |
|---|---|---|
| billing suite scenarios | 27 | **29** |
| browser boundary claims held | 17 | **counted at run time** |
| revision-pair scenarios | nine | **ten** |

The middle one is the fix worth copying: `smoke.mjs` now **increments a counter in its own check
function** and prints what it actually checked. It cannot drift again, because there is no second
copy of the number to drift from.

For the first, a new guard reads the count **out of the suite's own source** and fails if the prose
disagrees — because correcting two copies without tying them together only resets the clock.
