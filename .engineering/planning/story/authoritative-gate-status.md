---
format: aep.planning-md/1
id: story:authoritative-gate-status
kind: story
status: implemented
title: CI and prose derive from the executable gate and durable status sources
summary: Remove a second CI gate and hand-maintained current-state claims.
relations:
- derived_from: epic:architecture-hardening
- serves: vision:O6
revision: 4
---
## Finding

The workflow and Taskfile independently enumerate verification, while reader-facing prose repeats versions, component delivery and gate claims that have drifted from tags, the store and the tree.

## Acceptance

CI invokes `task check` after provisioning its declared environment, and release verification reuses that entry point. `cargo xtask status --check` owns every generated volatile status region. Hand-maintained current-state claims become links to annotated-tag status, the planning store or gap register. A drift test fails if CI stops invoking the gate or a generated region changes.

## Scope

- `.github/workflows/`, `Taskfile.yml`, `xtask/` and generated status regions — cited.
- README, VISION, reconciliation and website status prose — inferred from the review search; confirm each claim before editing.
