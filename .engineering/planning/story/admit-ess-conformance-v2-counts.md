---
format: aep.planning-md/1
id: story:admit-ess-conformance-v2-counts
kind: story
status: implemented
title: Admit exact ESS report v2 counts through both AEP readers
relations:
- serves: vision:O2
- informed_by: story:executable-system-specification-is-governed
scope:
- confidence: cited
  path: CHANGELOG.md
- confidence: cited
  path: Cargo.lock
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
- confidence: cited
  path: docs/design/ess-conformance-v2-evidence.md
- confidence: cited
  path: generated/instructions
- confidence: cited
  path: principles/verification/ess-conformance-v2.yaml
- confidence: cited
  path: profiles/development-ess-conformance-v2.yaml
- confidence: cited
  path: protocols/adp-ess-conformance/1.yaml
- confidence: cited
  path: schemas/generated/artifact-lifecycle.schema.json
- confidence: cited
  path: schemas/generated/artifact-manifest.schema.json
- confidence: cited
  path: schemas/generated/driver-steps.schema.json
- confidence: cited
  path: schemas/generated/event.schema.json
- confidence: cited
  path: schemas/generated/evidence.schema.json
- confidence: cited
  path: schemas/generated/principle.schema.json
- confidence: cited
  path: schemas/generated/profile.schema.json
- confidence: cited
  path: schemas/generated/protocol.schema.json
- confidence: cited
  path: schemas/generated/task.schema.json
- confidence: cited
  path: schemas/generated/workflow.schema.json
- confidence: cited
  path: website/docs/reference/cli.md
revision: 50
---
## Problem and owner

ESS review F03 is tracked by ESS story:a-skipped-scenario-is-not-a-failed-one and its accepted docs/design/review-conformance-coverage.md. AEP00c742e4179593738a2e8aa69e2ecc07d3c89402 still has two report/1-only readers: the optional aep-ess-evidence adapter and the planning CLI's independent --from parser. Neither establishes report/2 readiness. This companion owns the AEP side of the coordinated count migration and serves O2. Existing generic subject-binding, persisted-facts and horizon stories retain their separate ownership.

## Acceptance

Both original-byte reader paths admit the same valid report/2 plus its exact original suite/1–4 JSON, preserve exact category counts and completed time through typed evidence input/readback and diagnostic planning history, and refuse malformed or unpaired inputs consistently. Legacy unknown coverage never qualifies a complete-selection conformance requirement, including through the engine's evidence.missing path. Report/1, its historical aggregate semantics, existing default profiles and all frozen writers remain unchanged. Publish one fully gated reader commit before the new ESS writer is enabled even as an opt-in.

## Binding prerequisite

AEP docs/design/ess-conformance-v2-evidence.md and a dedicated Atlas ADR must bind original report/suite byte transport, re-admission at every typed readback/submission edge, exact integer envelope timestamps, independently supplied expectations and shared same-record qualification before implementation dispatch. The complete binding is now accepted for implementation, with the bounded review correction recorded below; it is not deployed. Existing Evidence/EvidenceEnvelope and the accepted ESS report wire are the typed owners; no new runtime entity or unrelated ESS modeling dependency is introduced.

Names selected for the additive opt-in are evidence kind and fact namespace ess_conformance_v2, principle principles/verification/ess-conformance-v2.yaml, protocol protocols/adp-ess-conformance/1.yaml with id adp-ess-conformance version 1 extending adp/1, and profile profiles/development-ess-conformance-v2.yaml extending development.standard. The profile must not inherit the legacy critical profile's additional ESS requirement. Validate actual protocol/profile composition before accepting these leaves; defaults stay untouched. Exact byte/readback and qualification details are bound in the accepted design; implementation must follow them.

## Implementation boundary

Keep the five affected packages together: observe/aep-ess-evidence, govern/aep-domain, govern/aep-engine, edge/aep-schema and edge/aep-cli. The optional adapter remains the closed standalone ESS boundary and introduces no compiled ESS model crate. Add the permitted edge-to-observe dependency where necessary. New counts project as canonical decimal Text and checked typed booleans; never usize/f64, parse_literal, truthiness of text zero or numeric ordering on strings. No global Number/Node migration, backend or journal fact schema migration, default policy switch or release tag.

Reader strictness covers duplicate keys, unknown fields/versions/profiles, exact unsigned decimal tokens, checked arithmetic, category-list lengths and disjoint sorted IDs, original-byte suite digest and selected membership. Suite/5, complete inventory and qualifying passed conformance remain refused until the separate coverage stage. Zero total may be diagnostic v2 input; it never qualifies. Preserve all valid legacy reader controls including the planning reader's existing v1 zero-total refusal.

The typed byte/readback contract must not trust a serialized verified:true claim. Wire time admits the full u64 range; any narrower planning date API performs an explicit checked adaptation refusal while retaining the meaning of the valid source bytes. A narrow shared ObservedAt unsigned visitor is preferred only with calendar-date compatibility evidence; broader legacy numeric semantics are outside this story.

## Validation

Use runner-derived before/red/after counts and mutation checks for guards. Cover 0, 9007199254740993, i64::MAX, i64::MAX+1 and u64::MAX without allocating enormous lists; signs, fraction/exponent and overflow refuse. Pairing must distinguish original whitespace/newline bytes. Exercise skip-only, unsupported-only, errors, mixed and empty selections with truthful profile/status derivation. Exercise one-record qualification against wrong subject/model/suite/selection/verifier, stale/future time and cross-record fact mixing; unknown coverage never qualifies in either decision path. JSON/YAML typed input, serialized readback, engine submission, driver ingestion, planning --from and both binary aliases must agree.

Run package tests/format/strict Clippy, generate schemas only with cargo xtask schema, then the entire task check. Retain exact generated changes and authoritative exits; no zero-selected lane is proof. Atlas records relying parties, reader-first move order and actual shipping evidence. Installed external binaries and already-generated Go runtimes stay explicitly unverified until separately observed.

## Scope

Derived from read-only source at advertised AEP00c742e4179593738a2e8aa69e2ecc07d3c89402 and the refreshed ESS handoff; code owners are cited, new leaves inferred. Confidence is high for the bounded source surfaces following the complete binding and independent design review; runtime correctness still requires implementation and tests.

- crates/observe/aep-ess-evidence — cited; original-byte shared admission and adapter tests.
- crates/govern/aep-domain — cited; closed evidence, exact time/facts and requirements.
- crates/govern/aep-engine — cited; consistent submission/qualification and evidence.missing.
- crates/edge/aep-schema — cited; exact typed input/readback.
- crates/edge/aep-cli — cited; both live readers and app/driver edges.
- docs/design/ess-conformance-v2-evidence.md — inferred; accepted binding before production.
- principles/verification/ess-conformance-v2.yaml, protocols/adp-ess-conformance/1.yaml and profiles/development-ess-conformance-v2.yaml — inferred; additive opt-in leaves, composition to validate.
- schemas/generated/{artifact-lifecycle,artifact-manifest,driver-steps,event,evidence,principle,profile,protocol,workflow,task}.schema.json — inferred; exact generator-owned propagation, never hand edits.
- Cargo.lock — inferred; intentional hashing/adapter edges only.
- CHANGELOG.md — cited; user-visible Unreleased record.
- website/docs/reference/cli.md — cited; scoped report1/report2 paired-input and evidence-inspection documentation.

Collides with domain evidence/time/requirement, engine execution, typed evidence parsing, CLI readers and these exact policy/generated leaves. No backend, general persisted facts, whole profile tree or whole numeric model scope is reserved.

Confirmed implementation and review scope:

- cited — the five assigned crates own shared original-byte report/suite admission, closed raw transport, exact counts/time, engine re-admission, independent expectations, same-record qualification and both CLI readers. Core AEP compiles against no ESS modeling crate.
- cited — the new principle, `adp-ess-conformance/1` leaf and `development.ess-conformance-v2` profile are additive opt-ins. Default policy source and generic supported protocol majors remain unchanged.
- cited — ten schemas are exact `cargo xtask schema` output, with the raw type's derived new-kind closure. Six instruction files are canonical output from the expanded available-principle inventory; the index is unchanged.
- cited — manifest/lock changes are narrow hashing and optional adapter dependencies. No backend, global Number/Node change, journal-facts schema migration, ESS writer, suite/5 or detailed run/2 reader was added.
- cited — CLI documentation and changelog describe pairing, exact inspection, checked recording and unknown coverage. Installed external binaries and generated Go runtimes are not declared upgraded.

Initial inferred scope above remains historical. Implementation passed 1,313 cases with 32 measured guard mutations; first review added seven and passed all 1,320. The integrated gate passed 2,209 cases, zero failures or ignored cases, plus applicable checks including MSRV and Website. PostgreSQL was explicitly skipped without a connection. The first full-suite attempt exposed an existing outside-project fixture under a project-local TMPDIR; unchanged source passed after moving gate scratch into an assigned external cache directory. The installed old CLI refused the new opt-in vocabulary before store access; completion uses the exact freshly built candidate CLI, without changing the installed binary.

## Preparation record

A dedicated managed checkout was created at the advertised source after reading its AGENTS and linked-tree inventory. Its AGENTS references a removed .agents/skills/planning/SKILL.md; that exact object path is absent, so the available aep-plan:planning0.7.0 skill supplies the planning CLI workflow. Artifact list, kinds, relations and story lifecycle were read before this first store mutation. The primary checkout is dirty and stale and remains untouched. No implementation, test, acceptance or release claim follows from preparing this draft.

## Accepted implementation decision

On2026-09-06 the coordinator accepted docs/design/ess-conformance-v2-evidence.md and Atlas ADR0039 under the user's standing instruction to implement all ESS remediation waves. This is the cross-repository prerequisite for ESS F03 count production. Record exact source/API shipping separately; no version/tag, default switch or consumer deployment is inferred.

The typed value additions remain owned by existing Evidence/EvidenceEnvelope and Task.constraints. Raw original report/suite strings confer no trust; an explicitly installed pure reader re-admits at every mutation and restore boundary. A checked direct-record Result and an explicit reading instant preserve refusal-before-mutation. Independent task subject/model/suite/selection and one per-record qualifier are required in both decision paths. All count-stage coverage remains unknown; no positive complete-conformance fixture or dormant completeness switch is permitted.

Immutable review-result:ess-conformance-v2-binding-review-pass-1 preserves the complete warning that a shared unsigned/date visitor could silently narrow legacy numeric spellings. The accepted correction keeps actual integer tokens exact, preserves established integral floating/exponent/negative-floating-zero legacy admission and rejection behavior separately, and retains strict lexical integers inside embedded report JSON. Outer v2 time must equal the original report instant; rounding cannot freshen it. B04 explicitly requires compatibility, full-u64 transport and rounded-envelope mismatch controls. This is a bounded prerequisite document review, not a full implementation adversary pass or executed test evidence.

The implementation wave has one unit spanning the five established packages because admission, typed replay and both CLI readers form one coherent seam. Actual policy composition, complete frozen suite-vocabulary transcription, source API compilation, generated propagation and full-u64 backend/history behavior remain reason-specific test obligations, not assertions from this decision. The coordinator owns the shared binding, changelog, planning and publication; the implementor works in its own managed tree and leaves source/test changes for review.

## Real composition correction: protocol major versus ESS evidence major

The actual new-policy composition test rejected the initially proposed adp-ess-conformance/2 before Registry resolution. Domain protocol.rs:39,252 and engine registry.rs:205 independently admit supported major1. No adp-ess-conformance protocol exists at the frozen00c742e baseline. Therefore create the new leaf protocols/adp-ess-conformance/1.yaml with id adp-ess-conformance, version1, extending adp/1; the development.ess-conformance-v2 profile references that leaf. Its protocol-definition major is independent of report2 and ess_conformance_v2. The new evidence vocabulary and explicit profile still separate the opt-in contract; no old protocol file is rewritten. SUPPORTED_MAJORS, is_supported_major and both existing checks remain unchanged, and generic major2 documents remain refused. No ESS-specific core version exception is introduced. Retain the original /2 test failure as a proposal-feasibility defect, then measure actual corrected composition/default and generic-major-refusal controls. No /2 leaf was published or relied on by an installed consumer.

## Generated instruction scope correction

The package instruction tests observed that adding the opt-in principle also changes the repository-wide workflow instruction projection, which inventories all declared principles. The owning test is crates/edge/aep-cli/tests/instructions.rs:17–22; its canonical writer is protocol govern workflow instruct --out generated/instructions. The resumed implementor retained the failed package run and exact generated patch under target/ess-conformance-v2-counts. Add generated/instructions as cited scope and regenerate its six changed workflow documents with that writer. No workflow or default policy source changes are authorized by this generated propagation.

## Frozen implementation and independent review

The resumed implementation is frozen as `fc58d0fb365f04f2c92e0a7cc7a278f85b55ec8e`. Root verified all 52 changed/new source and generated paths against `target/ess-conformance-v2-counts/source-manifest.json`, verified the report SHA-256 `edaa883c03ccf116fba536b00e9600e096f4d7723f02c16084b7d9a1d15a2571`, and verified both direct commit identities. The complete immutable implementor report is `target/ess-conformance-v2-counts/handoff.md` in the existing unit.

Final five-package execution passed 1,313 cases, zero failures or ignored cases, across 59 runner summaries. Strict Clippy, package formatting and actual schema drift checking exited zero. The report retains 32 observed guard mutations and all prior failures, including schema closure, canonical instruction propagation and dependency-feature numeric compatibility. Six instruction files were regenerated by the coordinator after recording their scope; ten schemas were emitted by the canonical schema writer. No default profile or protocol source changed.

The author relinquished all writes. `expression_review_resume`, which did not author this unit, now performs the first bounded implementation review under the installed 0.8.0 adversary charter in `target/ess-conformance-v2-counts/adversary-pass-1`. The general collaboration harness substitutes for an unavailable plugin-role selector. This is review readiness; the full repository gate, publication and actual ESS producer correspondence remain required.
