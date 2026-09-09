---
format: aep.planning-md/1
id: migration-plan:aep-runtime-extraction
kind: migration-plan
status: active
title: Move concrete AEP execution above the foundation
relations:
- informed_by: epic:reference-driver
- serves: vision:O3
revision: 2
---
## Decision
The operator approved extraction on 2026-09-09: model-backed runs move to `metaharness aep drive`; migrate eval and Agentplugins callers now; preserve compatible paused runs. AEP retains neutral governor, run machinery, command/operator driving and offline evidence ingestion. No foundation runtime dependency on installed Metaharness or Harness.

## Implementation
Make AEP CLI importable, share neutral run-host services, move concrete executor, frame/event translation, native hooks and live evaluation into Metaharness. Pin all AEP dependencies to one published commit. Keep existing authorization, spend, frozen wire and resume integrity rules. Record Atlas ADR 0047 and actual catalog direction, then complete fresh foundation composition and final planning validation.

## Acceptance
Foundation gates pass without tooling executables. Offline adapter tests retain denial, frame, budget, plugin and legacy-resume behavior. Callers invoke the real replacement command. Exact-hash composition receipt and final ER planning evidence exist before completion. Retire only reviewed recoverable worktrees; preserve unrelated changes. No tags, deployment, paid run or connectors_v2 enrollment.
