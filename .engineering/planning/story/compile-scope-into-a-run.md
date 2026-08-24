---
format: aep.planning-md/1
id: story:compile-scope-into-a-run
kind: story
status: draft
title: The step map's scope and context reach a run without a person retyping them
summary: Compile aep.driver-steps/1 scope and context into the arm's own flags, so the declaration is the source of truth rather than documentation.
owner: eval
tags:
- eval
- harness
relations:
- decomposes: epic:self-evaluation
revision: 1
---
# Story: the declaration reaches the run

## Outcome

Somebody who changes `scope:` or `context:` in `drivers/development/default.yaml` changes what the
next run may do. Today they change a document, and a person has to notice and retype it as flags.

## Context

Four live runs on 2026-08-24 (see `docs/reviews/2026-08-24-scope-cache-and-the-native-arm.md`) used
the declared scope, and every one of them had it hand-translated: six `paths` entries became six
`--write-scope <glob>=<word>` arguments, and one `context:` entry became one `--context <file>`. The
translation is mechanical, which is the argument for doing it in code rather than the argument for
leaving it to a person — a mechanical step a person performs is a step that will one day be skipped.

`aep-driver-spec` already parses and validates both keys, and the b10x arm already takes both flags.
What is missing is the compile between them, and the place for it is the adapter, beside `rendering()`
— the same place that already maps neutral operations onto a harness's own names.

## Acceptance

- A run assembled from a step-map step carries that step's `scope` and `context`, with nobody naming
  a flag.
- Rule order survives the compile: first match wins downstream, so a scope is a list and not a set.
- A `context:` file that is absent refuses the run rather than warning — a run given less context than
  its map declares cannot be reproduced from the map.
- One test compiles the committed `drivers/development/default.yaml` step and asserts the exact argv.

## Out of Scope

The vendor arms. Their scope travels as `Frame.subjects`, which is
`story:frame-subjects-from-the-step-map`.

## Open Questions

- Does the compile belong to the driver (`protocol drive`) or to the eval runner? Both assemble runs.
  Decided by whoever picks this up, recorded in the story before the first commit.
