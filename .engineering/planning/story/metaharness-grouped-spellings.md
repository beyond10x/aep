---
format: aep.planning-md/1
id: story:metaharness-grouped-spellings
kind: story
status: active
title: metaharness teaches the grouped spellings without moving a recorded byte
summary: 138 flat call sites; only authored prose and executable steps move, fixtures and predicates stay.
relations:
- decomposes: epic:consumers-use-grouped-spellings
- serves: vision:O2
scope:
- confidence: cited
  path: metaharness:AGENTS.md
- confidence: cited
  path: metaharness:README.md
- confidence: cited
  path: metaharness:Taskfile.yml
- confidence: cited
  path: metaharness:docs
- confidence: cited
  path: metaharness:evals
revision: 4
---
# Story: metaharness teaches the grouped spellings without moving a recorded byte

## Context

138 flat call sites at `origin/main` (2026-09-04), in three classes: authored prose and executable
steps, `metaharness.event/1` fixtures recording finished runs, and trace-specification expectation
rows judged against those fixtures. Only the first class moves. A predicate over a finished run is a
claim about that run's bytes, so rewriting one turns a green replay red.

Pre-existing and out of scope: `evals/aep/checks/run-checks.sh` is red at baseline because two
expectation documents were never migrated.

## Acceptance

Authored prose and executable steps use the grouped spelling, every fixture and every predicate over
one is byte-identical to before, and the gate is green or red for exactly its baseline reason.
