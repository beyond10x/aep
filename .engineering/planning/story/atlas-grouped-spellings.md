---
format: aep.planning-md/1
id: story:atlas-grouped-spellings
kind: story
status: active
title: atlas teaches the grouped spellings, and the catalog records the new releases
summary: 36 flat call sites in AGENTS.md, README and skills, plus a catalog refresh for the released versions.
relations:
- decomposes: epic:consumers-use-grouped-spellings
- serves: vision:O2
scope:
- confidence: cited
  path: atlas:AGENTS.md
- confidence: cited
  path: atlas:README.md
- confidence: cited
  path: atlas:catalog
- confidence: cited
  path: atlas:skills
revision: 4
---
# Story: atlas teaches the grouped spellings, and the catalog records the new releases

## Context

36 flat call sites at `origin/main` (2026-09-04) in `AGENTS.md`, `README.md` and the skills Atlas
owns. Atlas is also the catalog: the component records still name the versions from before this
wave, so the same unit refreshes them for AEP 0.52.0, ESS's current release, workspace 0.2.14 and
aep-service 0.1.7.

## Acceptance

Every Atlas-owned document teaching an invocation uses the grouped spelling, `catalog validate`
passes, the rendered catalog names the current released versions, and the gate exits 0.
