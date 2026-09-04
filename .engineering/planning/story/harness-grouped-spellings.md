---
format: aep.planning-md/1
id: story:harness-grouped-spellings
kind: story
status: active
title: harness teaches the grouped spellings
summary: 15 flat call sites around the drive-transition seam and docs; drive did not move.
relations:
- decomposes: epic:consumers-use-grouped-spellings
- serves: vision:O2
scope:
- confidence: cited
  path: harness:AGENTS.md
- confidence: cited
  path: harness:README.md
- confidence: cited
  path: harness:docs
- confidence: cited
  path: harness:website/docs
revision: 4
---
# Story: harness teaches the grouped spellings

## Context

15 flat call sites at `origin/main` 1d84c6e (2026-09-04). The repository's seam into AEP is
`protocol drive transition` (ADR 0004), and `drive` is an area name that did not move, so several
hits need no change at all.

## Acceptance

Every authored document teaching an invocation uses the grouped spelling, the gate exits 0, and each
unchanged hit is named with its reason.
