---
format: aep.planning-md/2
id: story:a-failed-publication-leaves-no-stage-directory
kind: story
status: draft
title: A failed publication leaves no stage directory beside the projection
relations:
- serves: vision:O2
- decomposes: epic:planning-on-entity-runtime
revision: 1
---
## Outcome

A publication that fails after staging removes its stage directory, so no
`.engineering/planning.aep-stage-*` directory is ever left beside the projection by a refused
render, write or validate.

## Why

0.59.0's refused `render` left a full stage of the projection (33 to 467 files) in each of the
six cut-over repositories; a `git add -A` would have committed it. 0.59.2 removes the stage on
the fall-through it added, but the other error returns after `stage()` in `publish_current`
(`crates/plan/aep-planning-migration/src/projection.rs`) still return without removing it.
