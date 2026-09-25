---
format: aep.planning-md/2
id: epic:planning-on-entity-runtime
kind: epic
status: implemented
title: Planning stores are typed entities on a tree Git merges
relations:
- serves: vision:O2
revision: 4
---
## Outcome

Every Beyond10x planning store is `aep.project/3`: typed Entity Runtime entities on an
`eventlog-tree` store under `.engineering/state/`, which Git merges, with `.engineering/planning/`
a rendered projection and `planning validate` a required check.

Design: `docs/design/planning-on-entity-runtime-v0.1.md` (accepted). Atlas ADR 0063.

## Delivered

| unit | where |
|---|---|
| tree provider, fold cache, V1–V5 | Eventlog 0.4.0 |
| lineage, `Forked`, `BatchAction::Merge`, recorded refusals | Entity Runtime 0.21.0 |
| typed kinds, export, resolve, render, tree rules of validate, pre-push hook | AEP 0.58.0 (#18, #20, #21) |
| cutovers | eventlog #27, entity-runtime #31, aep (this store's own), ess #67 |

## Open

Filed as stories under this epic as they are scheduled: file-store stamps across processes
(B1/B2), V3 `copy`, per-artifact reads (R2), the `.md` format tag (A5), evidence as observation
(N7), V2 skipped for a base without a tree store, forked reads, and the connectors and
service-sdk cutovers.
