---
format: aep.planning-md/1
id: story:admit-ess-conformance-coverage
kind: story
status: active
title: Admit exact ESS coverage and qualify the declared selection
relations:
- serves: vision:O2
- depends_on: story:admit-ess-conformance-v2-counts
scope:
- confidence: inferred
  path: CHANGELOG.md
- confidence: cited
  path: crates/edge/aep-cli
- confidence: cited
  path: crates/edge/aep-schema
- confidence: cited
  path: crates/govern/aep-domain
- confidence: cited
  path: crates/govern/aep-engine
- confidence: cited
  path: crates/observe/aep-ess-evidence
- confidence: inferred
  path: docs/design/ess-conformance-coverage-evidence.md
- confidence: inferred
  path: principles/verification/ess-conformance-coverage.yaml
- confidence: inferred
  path: profiles/development-ess-conformance-coverage.yaml
- confidence: inferred
  path: protocols/adp-ess-conformance-coverage/1.yaml
- confidence: inferred
  path: schemas/generated/artifact-lifecycle.schema.json
- confidence: inferred
  path: schemas/generated/artifact-manifest.schema.json
- confidence: inferred
  path: schemas/generated/driver-steps.schema.json
- confidence: inferred
  path: schemas/generated/event.schema.json
- confidence: inferred
  path: schemas/generated/evidence.schema.json
- confidence: inferred
  path: schemas/generated/principle.schema.json
- confidence: inferred
  path: schemas/generated/profile.schema.json
- confidence: inferred
  path: schemas/generated/protocol.schema.json
- confidence: inferred
  path: schemas/generated/task.schema.json
- confidence: inferred
  path: schemas/generated/workflow.schema.json
- confidence: inferred
  path: website/docs/reference/cli.md
revision: 26
---
## Outcome

Admit ESS suite/5 coverage as a separately versioned AEP evidence carrier, and qualify only a nonempty complete declared selection matching independent task expectations. This is the reader prerequisite for ESS story:review-conformance-coverage under the standing remediation authorization, serving O2.

## Existing typed home

These are value representations under existing Evidence, EvidenceEnvelope, Task.constraints and checked Execution recording/snapshot admission. The existing Rust owners are crates/govern/aep-domain/src/{evidence.rs,ess_conformance_v2.rs,task.rs,requirement.rs}, crates/govern/aep-engine/src/{engine.rs,execution.rs}, the optional crates/observe/aep-ess-evidence reader and edge schema/CLI routes. No new independently identified runtime entity or authored ESS business domain is introduced. Core AEP continues to compile without ESS modeling crates.

## Acceptance

An independently expected, admitted, nonempty suite/5 selection with complete inventory, no in-scope refusal and all-pass terminal outcomes satisfies the same per-record decision in requirement evaluation and evidence.missing. Missing, stale, contradictory, incomplete or differently selected evidence cannot qualify, including after serialized source/snapshot replay.

## Binding prerequisite and current limits

ESS docs/design/review-conformance-coverage.md owns the accepted suite/5 inventory, selection, ordered refusal occurrences, original-byte identity and complete-selection/1 policy. Its count writer is published at ESS87d9945; AEP count readers are published at30aeef2, with current62ef3a7 containing only later internal records. Both old and count-stage carriers and their nonqualifying meanings remain frozen.

Before implementation, accept a new docs/design/ess-conformance-coverage-evidence.md binding and a current Atlas migration ADR. Resolve exact parent-chain byte transport and offline replay, stable authored-source identity, the separate carrier/kind/constraint vocabulary, all actual reader/restore paths and precise expected-selection equality. A proposed original-byte input carrier and a new ess_conformance_coverage_v1 kind are preparation choices, not accepted source declarations. Adding parent fields to existing closed ess_conformance_v2 is not selected.

## Required verification

Use independently authored complete/partial/zero selection fixtures, retained old report/count controls, all suite/5 structural and source/parent/refusal invariants, exact-u64 transport and planning calendar refusal. Exercise both adapter and independent planning reader, typed JSON/YAML, evidence inspection, direct checked recording, engine submission, driver resume and snapshot restore. Missing readers and every refused mutation preserve prior state. Never reconstruct expectations from the report or combine facts from different records. Prove each guard with a failing mutation and actual named cases.

Publish the reviewed and fully gated AEP reader source before enabling any ESS suite/5 opt-in writer. After that writer is frozen, execute actual Rust and Go report/suite input pairs through both AEP routes and typed replay before ESS writer publication. Source publication is separate from installed/generated adopter readiness. No release, tag, installation or default change is authorized.

## Scope

Refreshed 2026-09-06 by story-scoper from AEP62ef3a7, draft revision1, the implemented count dependency and the actual admission graph — cited. Coordinator adds the separately proposed coverage binding; no implementation is active.

- crates/govern/aep-domain — cited; evidence.rs:1196,1432 owns kind/value/facts dispatch; task.rs:286 owns Constraints; requirement.rs:63,354,387,507,654 owns context, positive-count requirements and shared qualification. ess_conformance_v2.rs:492,505,565 retains frozen count reader/carrier/expectation. Add separate coverage values/port/constraint/qualifier without widening those count APIs.
- crates/govern/aep-engine — cited; engine.rs:186,212,228,279 configures readers and handles initialization/submission/restore; execution.rs:222,291,303,313,667,701,733 owns direct recording, preparation before mutation and evidence.missing. Re-admit before publishing evidence, links, events, observation or execution state.
- crates/observe/aep-ess-evidence — cited; counts.rs:18,43,153,194, count_json.rs and count_suite.rs own count adaptation, original scalars and fixed suite1–4 vocabulary. Add closed suite5/input/parent/report transcription alongside these owners.
- crates/edge/aep-schema — cited; parse.rs:333,350,359,370,387–419 and schema.rs:67,107 own raw input, reader-bearing parsing, closed carrier preflight and generated schema treatment. Preserve old reader-free/count-only signatures through additive entry points.
- crates/edge/aep-cli — cited; app.rs:1789–1823,2312,2735–2772 owns evaluation, typed input and inspection; planning.rs:5914 owns report reading and pre-store time refusal; drive.rs:718,948,3501,5050–5071 owns start/resume/consultation and typed record admission. Install both optional readers on every applicable route.
- Driver boundary — cited; aep-driver already restores raw snapshots through Engine and submits observed records through Engine::submit_evidence. Its four-kind mintable list excludes the new kind and requires typed input. Actual CLI driver replay tests are required; no driver production package edit is established.
- Integration shape — inferred; concrete optional count and coverage ports with additive APIs, immutable admitted values and reader-free refusal. A configured count reader never implies coverage readiness.
- Qualification — cited; one independent record must match task/requirement subject, conformance-runner producer, specifically named current model/digest, exact suite/Selection/IDs, known exact time and freshness, nonempty complete all-pass coverage. No cross-record facts can fill a missing condition.
- docs/design/ess-conformance-coverage-evidence.md — inferred; root's proposed separate carrier/input/expectation/admission/qualification/API binding, requiring review before implementation.
- principles/verification/ess-conformance-coverage.yaml — inferred; proposed new explicit principle, with the shared qualifier and no count-only inheritance.
- protocols/adp-ess-conformance-coverage/1.yaml — inferred; proposed separate supported-major1 protocol, extending ADP with only the new vocabulary.
- profiles/development-ess-conformance-coverage.yaml — inferred; proposed explicit development.standard extension and new protocol/principle.
- Generated schema leaves — inferred; artifact-lifecycle, artifact-manifest, driver-steps, event, evidence, principle, profile, protocol, task and workflow under schemas/generated currently propagate count vocabulary. The generator must establish actual final bytes and scope; no handwritten schema edits.
- website/docs/reference/cli.md — inferred; current lines96–97,365,393 need accepted planning/inspection/policy and raw-versus-admitted behavior.
- CHANGELOG.md — inferred; record the new opt-in public capability and preserved legacy/count behavior.
- Verification — cited; package-local cases cover complete/partial/zero independent expectations, original bytes, full-u64/calendar, every parent/source/refusal invariant, JSON/YAML, inspection, planning, direct recording, submission, snapshots and actual driver replay. Missing-reader and refused-mutation cases preserve prior state; prove guards by mutation.
- No additional reservation established — inferred; no compiled ESS dependency, driver production, dependency/lockfile, Taskfile, workflow or generator executable change is required by current source evidence. Expand only from a concrete discovered need.
- Confidence: medium — inferred; five owners and all replay routes are known, but the proposed transport and versioned contracts have not passed binding review or execution.
- Would collide with: those five packages, binding, policy leaves, generated schema leaves, CLI docs and changelog — inferred. The planning journal is serialized by root and is outside implementor ownership.


## Current preparation

Root drafted the proposed coverage binding after the independent scope pass. ESS input/1 parent transport, stable source identity and browser/impact choices remain proposed. Allocate and review the new Atlas ADR and both bindings before implementation. The repository-referenced local planning skill is absent; the installed aep-plan:planning0.8.0 and actual CLI remain the store authority.

## Accepted binding decision — 2026-09-06

Under the standing ESS remediation implementation/publication authorization, root accepts the reviewed coverage/transport/AEP contracts and Atlas ADR0040 for implementation. This section supersedes earlier preparation wording that calls them proposals. Both immutable binding reviews are recorded; pass1 exact fact vocabulary and strict/diagnostic wording are corrected, and pass2 Text truthiness guidance was checked against the unchanged AEP facts/predicate owner. No implementation test or compatibility result is inferred from those document checks. No third binding review was opened.

- docs/design/review-conformance-coverage.md — SHA256 ef8028fce04685a4b5936c87bab37f4b05bf674ccbbb0d2fcb5d0dd55da00742.
- docs/design/review-conformance-coverage-transport.md — SHA256 2b06be326986a9f1dd76029d6802d9c63bc601bcaecce138828c10159f5e31c2.
- docs/design/ess-conformance-coverage-evidence.md — SHA256 d5ee4f43d8527134997eaf7fc2ca30e9f4922dba1df2bebd38779d98029db873.
- architecture/adr/0040-ess-complete-selection-evidence.md — SHA256 54931ffb2e52beb09e7ba7b952ec16f996a681161692227e5cdc25b1f3d337ac.

Original review reports are035d8b8fd665457e387a73edfd47fec9b40fbdc85bc6c93a774b48f3f4f8ada3 andf13642c04236967cd71ff28096abb1907aa8e1139687243546f3192c345b058a. Actual AEP source baseline package tests separately passed1320cases0failed/ignored61summaries; that is existing-source resource/control evidence, not new reader implementation. Reader publication remains required before the ESS writer, followed by frozen actual Rust/Go correspondence before ESS writer publication.
