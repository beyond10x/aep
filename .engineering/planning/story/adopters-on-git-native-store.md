---
format: aep.planning-md/3
id: story:adopters-on-git-native-store
kind: story
status: draft
title: Every repository that plans with AEP is on aep.project/5
relations:
- serves: vision:O2
revision: 1
---
## Problem

aep 0.62.0 and later refuse `aep.project/2`–`/4`. On 2026-09-28 these repositories were still on
event-log stores: eventlog, entity-runtime, ess, connectors, service-sdk and
epistemic-knowledge-runtime (EKR migrated a copy for measurement only). The agentplugins
repository is on `/1`. The aep plugin skills in agentplugins still pin aep 0.60.0 in
`verified.json`.

## Outcome

Each repository migrates once (for `/2`–`/4`: the build at `9c0f1da4` and
`aep plan store migrate git --verify`; for `/1`: 0.63.0 or later), pins its `protocols` to a release
commit after 0.62.0, and installs the current release; the plugin skills carry the upgrade steps.

## Acceptance

- Each repository's `.engineering/project.yaml` says `aep.project/5` on its main branch, with a
  green `planning validate` check.
- agentplugins' `verified.json` pins an aep release ≥ 0.63.0.
