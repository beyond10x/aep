---
format: aep.planning-md/1
id: story:agentplugins-grouped-spellings
kind: story
status: active
title: agentplugins teaches the grouped spellings
summary: 275 flat call sites across the plugin skills, agents and website; recorded eval bytes stay flat.
relations:
- decomposes: epic:consumers-use-grouped-spellings
- serves: vision:O2
scope:
- confidence: cited
  path: agentplugins:plugins
- confidence: cited
  path: agentplugins:website/docs
revision: 4
---
# Story: agentplugins teaches the grouped spellings

## Context

275 flat `aep`/`protocol`/`ess` call sites at `origin/main` 249db09 (2026-09-04). The largest are
`plugins/aep-plan/skills/planning/SKILL.md` 69, `plugins/aep-drive/skills/wave/SKILL.md` 22,
`plugins/aep-plan/skills/story-migration/SKILL.md` 16 and
`plugins/aep-plan/skills/planning/references/store-conventions.md` 13, then six agent files at 4-12
each. The skills are the documents that teach every agent in the organisation how to invoke the CLI,
so they are the highest-value sites and the reason this is the largest unit.

`website/docs/install.md` pins AEP 0.51.0 and ESS 0.11.1; the pin moves only to a release that has
actually published.

## Acceptance

Every skill, agent and website document that teaches an invocation uses the grouped spelling, the
repository gate exits 0, and any flat spelling left is a recorded-bytes predicate or dated history.
