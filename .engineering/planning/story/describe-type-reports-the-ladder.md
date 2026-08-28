---
format: aep.planning-md/1
id: story:describe-type-reports-the-ladder
kind: story
status: implemented
title: describe_type reports the ladder the kernel decides with
summary: TypeDescriptor::lifecycle is filled in the adapter from the same EntityDefinition the kernel executes, so every backend reports the states and edges protocol artifact lifecycle prints; D-P5 closes.
owner: store
tags:
- backend
- store
relations:
- decomposes: epic:planning-store-as-backend
- depends_on: story:one-adapter-over-any-store
revision: 8
---
# Story: `describe_type` reports the ladder the kernel decides with

## Outcome

A harness asks the contract which statuses a story may hold and what may follow what, and gets the
answer the kernel will actually enforce — rather than reading `artifacts/lifecycles/story.yaml` and
hoping the two agree. D-P5 closes.

## Context

`TypeDescriptor::lifecycle` was `Option<LifecycleDescriptor>`, initialised `None` and assigned
nowhere; `story:journal-backed-store` ticked this line and a review struck it through. Meanwhile
`kernel::definition_for` already built an `EntityDefinition` from the kind's `ArtifactLifecycle` to
decide every move, and `tests/kernel_equivalence.rs` proves it agrees with the ladder over 800
status pairs. The descriptor is *that definition*, rendered.

Wave H, story 2 (`docs/plan/store-waves-f-g-h.md`). After F2.

## Acceptance

- `describe_type` for every planning kind returns a `LifecycleDescriptor` with the initial state,
  every state, and every `(from, to)` edge — derived from the same `EntityDefinition` the kernel
  executes, not from a second reading of the YAML. **Done** — `aep_backend_entity::kernel::describe`
  reads `definition_for`'s output: its operations, named for the status they reach, are the edges.
- An equivalence test: for every kind in this repository's store, the descriptor's edge set equals
  `protocol artifact lifecycle <kind>`'s output. **Done** —
  `the_descriptor_edges_are_what_protocol_artifact_lifecycle_prints`
  (`crates/protocol-cli/tests/describe_type.rs`), over every named kind the tree declares a ladder
  for.
- Filled in the adapter, so the memory, markdown and SQLite backends report identically — pinned by
  a test that asks all three. **Done** — `every_backend_reports_the_same_ladder_for_every_planning_kind`:
  the adapter over the runtime's `MemoryStore`, over `SqliteStore`, and the markdown backend, one
  set of ladders (`Projection::lifecycles`; `Identity::with_lifecycles`).
- A kind with `requires:` per rung (phase 3) reports which rungs cost evidence and which kind.
  **Done** — `LifecycleDescriptor::requires`, `to -> [(kind, at_least)]`; the story ladder reports
  `implemented -> [(test_result, 1)]`. Read from the lifecycle the definition was built from — the
  same object, not a second reading — because the definition encodes it as a precondition and
  parsing a rule back into a count would be the drift this exists to prevent.
- D-P5 is marked closed in the design, the gap register and the limitations page, in the same
  commit. **Done** for the design and the limitations page; the gap register never carried a D-P5
  row.

## Decision taken

`kernel.rs` moved into `aep-backend-entity` — the adapter both renders the ladder and decides with
it — and `aep-backend-markdown::kernel` re-exports it, so every existing path and the write-path
scan's source list keep working.

## Out of Scope

Reporting the ladder for entity types that are not planning kinds (`aep.design/v1` and friends):
their ladders are domain commands, not lifecycle documents, and describing those is a different
descriptor.

## Open Questions

None outstanding.
