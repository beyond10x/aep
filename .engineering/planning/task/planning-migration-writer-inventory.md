---
format: aep.planning-md/1
id: task:planning-migration-writer-inventory
kind: task
status: implemented
title: Inventory inspectable planning writers and remaining control facts
relations:
- decomposes: story:eventlog-planning-authority-migration
- serves: vision:O2
revision: 4
---
## Approved outcome and existing blocker

Approved ESS evolution M3/M4 requires an actual writer fence before apply and all six real-store
cutovers. The existing writer-fleet question is unanswered. Current local config, lock and old-reader
evidence cannot establish which CI, services, schedulers or other machines can still write.
This task supplies the inspectable local configuration portion of that existing requirement; it
does not introduce a new prerequisite, approve quiescence or claim a complete fleet from absence.

## Bounded deliverable and scope

Read only the six participating repositories' project/store selections, CI workflows, task/build
definitions and documented writer startup/control references, plus the selected AEP/ESS orchestration
checkouts. Produce a six-row inventory separating observed configuration, actual control evidence,
and unknown external authority. Cite exact files/lines and identify which operator facts remain.
Do not inspect unrelated home secrets, raw database contents or provider implementation. Do not
stop/restart a writer, change configuration, run SQL, migrate a store or make network calls.

The sole artifact is a local evidence report, not source code or a new framework. Existing source
and all planning stores remain unchanged by the worker. Root remains the only AEP writer.

## Stopping condition

Close after the bounded six-store configuration inventory, concrete control gaps and a concise
list of exact external facts still required, including inability to find a configured mechanism.
No follow-on design, code, probes, tests or live fleet operation is implicitly assigned. Missing
external facts remain an open blocker; a repository search is not proof that no other writer exists.
