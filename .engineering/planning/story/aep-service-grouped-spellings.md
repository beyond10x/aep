---
format: aep.planning-md/1
id: story:aep-service-grouped-spellings
kind: story
status: active
title: aep-service teaches the grouped spellings
summary: 5 flat call sites; the six aep-* library pins do not move for a CLI-only release.
relations:
- decomposes: epic:consumers-use-grouped-spellings
- serves: vision:O2
scope:
- confidence: cited
  path: aep-service:AGENTS.md
- confidence: cited
  path: aep-service:README.md
- confidence: cited
  path: aep-service:docs
revision: 4
---
# Story: aep-service teaches the grouped spellings

## Context

5 flat call sites at `origin/main` cda05bc (2026-09-04). The repository pins six `aep-*` crates at
0.51.0; 0.52.0 changed the command surface only, no crate name and no API, so the pin does not move
as part of this story.

## Acceptance

Every authored document teaching an invocation uses the grouped spelling and the gate exits 0.
