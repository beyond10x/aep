---
format: aep.planning-md/2
id: migration-plan:aep-runtime-extraction
kind: migration-plan
status: implemented
title: Move concrete AEP execution above the foundation
relations:
- informed_by: epic:reference-driver
- serves: vision:O3
revision: 4
---
## Decision
The operator approved runtime extraction on 2026-09-09. AEP retains the neutral governor, run state, command/operator driving and offline evidence ingestion. Concrete model execution belongs in Metaharness, with no upward runtime dependency from the foundation (Atlas ADR 0047).

## Repository scope
Expose the AEP command implementation and neutral execution-host interface; refuse model-backed starts and resumes before effects; preserve opaque launch metadata for compatible external hosts. The execution host supplies the continuation command printed for its paused runs. Preserve frozen frame, cursor, snapshot and evidence contracts.

## Repository acceptance
The full AEP gate passes, including governance, strict linting, workspace tests, documentation/schema checks, MSRV and website. Refusal tests leave project files unchanged and produce no live-evaluation output. The interface permits Metaharness to preserve paused-run authority and print its actual resume command. Publication uses the bot and exact-commit admission.

## Coordination
Metaharness owns concrete executor, native hooks, event translation and live evaluation; Agentplugins owns callers. Atlas's migration-plan:aep-runtime-extraction and task:foundation-composition-evidence own their integration order, actual catalog direction, the final exact-hash composition receipt and ER planning validation. This record's implemented status describes the AEP source implementation; Atlas cannot close the coordinated migration until those remaining requirements have evidence.

No tags, deployment, paid run or connectors_v2 enrollment is authorized.
