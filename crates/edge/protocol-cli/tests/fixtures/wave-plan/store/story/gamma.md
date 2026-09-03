---
format: aep.planning-md/1
id: story:gamma
kind: story
status: draft
title: Gamma
summary: Touches the same domain module alpha does, so the two cannot share a wave.
relations:
- decomposes: epic:wave-fixture
scope:
- path: crates/govern/aep-domain/src/artifact.rs
  confidence: cited
revision: 1
---

# Story: Gamma

## Outcome

Touches the same domain module alpha does, so the two cannot share a wave.
