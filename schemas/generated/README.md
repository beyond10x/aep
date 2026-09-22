# Generated schemas

**Do not edit these files.** They are generated from the Rust types by `cargo xtask schema`, and CI
fails if they differ from what the types produce.

They are the interoperability contract: anything that produces or consumes these documents can
validate them without linking the Rust crates.

| file | Rust type | describes |
| --- | --- | --- |
| [`protocol.schema.json`](protocol.schema.json) | `RawProtocol` | a protocol declaration |
| [`principle.schema.json`](principle.schema.json) | `RawPrinciple` | a principle |
| [`workflow.schema.json`](workflow.schema.json) | `RawWorkflow` | a workflow state machine |
| [`profile.schema.json`](profile.schema.json) | `RawProfile` | a profile |
| [`task.schema.json`](task.schema.json) | `RawTask` | a task |
| [`project.schema.json`](project.schema.json) | `RawProjectConfig` | what an adopting project says about itself |
| [`workspace.schema.json`](workspace.schema.json) | `RawWorkspace` | the repositories one command answers across |
| [`artifact-manifest.schema.json`](artifact-manifest.schema.json) | `RawArtifactManifest` | a project's artifact manifest |
| [`artifact-lifecycle.schema.json`](artifact-lifecycle.schema.json) | `ArtifactLifecycle` | the statuses one artifact kind may hold |
| [`evidence.schema.json`](evidence.schema.json) | `Evidence` | one piece of submitted evidence |
| [`action-request.schema.json`](action-request.schema.json) | `ActionRequest` | an action put to the engine |
| [`event.schema.json`](event.schema.json) | `EventEnvelope` | one audit event |
| [`planning-document.schema.json`](planning-document.schema.json) | `RawPlanningFrontmatter` | the frontmatter of one markdown planning document |
| [`raw-capture-observation.schema.json`](raw-capture-observation.schema.json) | `RawCaptureObservationV1` | one complete, unstable, or refused raw planning-store observation |
| [`planning-inspection.schema.json`](planning-inspection.schema.json) | `InspectionResultV1` | one planning-store inspection result |
| [`planning-migration-dry-run.schema.json`](planning-migration-dry-run.schema.json) | `DryRunResultV1` | one planning-store migration assessment |
| [`planning-migration-dry-run-v2.schema.json`](planning-migration-dry-run-v2.schema.json) | `DryRunResultV2` | one provider-assigned planning-store migration assessment |
| [`planning-migration-apply.schema.json`](planning-migration-apply.schema.json) | `ApplyResultV1` | one durable planning-store migration result |
| [`planning-verification.schema.json`](planning-verification.schema.json) | `VerificationResultV1` | one selected planning-authority verification |
| [`planning-projection-rebuild.schema.json`](planning-projection-rebuild.schema.json) | `RebuildResultV1` | one planning projection rebuild result |
| [`planning-mutation.schema.json`](planning-mutation.schema.json) | `PlanningMutationEnvelopeV1` | one ordinary planning mutation and its durable receipts |
| [`planning-projection-watermark.schema.json`](planning-projection-watermark.schema.json) | `ProjectionWatermarkV1` | one authority-owned projection watermark |
| [`planning-migration-intent.schema.json`](planning-migration-intent.schema.json) | `MigrationIntentV1` | one immutable planning migration intent |
| [`planning-migration-intent-v2.schema.json`](planning-migration-intent-v2.schema.json) | `MigrationIntentV2` | one immutable provider-assigned planning migration request |
| [`planning-store-ownership.schema.json`](planning-store-ownership.schema.json) | `OwnershipMarkerV1` | one explicit-path planning ownership marker |
| [`planning-store-ownership-v2.schema.json`](planning-store-ownership-v2.schema.json) | `OwnershipMarkerV2` | one provider-assigned explicit-path planning ownership marker |
| [`planning-migration-phase.schema.json`](planning-migration-phase.schema.json) | `PhaseRecordV1` | one durably established planning migration phase |
| [`planning-migration-phase-v2.schema.json`](planning-migration-phase-v2.schema.json) | `PhaseRecordV2` | one provider-assigned durable planning migration phase |
| [`planning-migration-current.schema.json`](planning-migration-current.schema.json) | `CurrentPhaseV1` | one durable planning migration phase pointer |
| [`planning-migration-current-v2.schema.json`](planning-migration-current-v2.schema.json) | `CurrentPhaseV2` | one provider-assigned durable planning migration phase pointer |
| [`trace-spec.schema.json`](trace-spec.schema.json) | `RawTraceSpec` | what an agent run must have looked like |
| [`driver-steps.schema.json`](driver-steps.schema.json) | `RawStepMap` | what a harness does in each state of a workflow |
