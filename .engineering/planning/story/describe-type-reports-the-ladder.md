---
format: aep.planning-md/1
id: story:describe-type-reports-the-ladder
kind: story
status: draft
title: describe_type reports the ladder the kernel decides with
summary: TypeDescriptor::lifecycle is filled in the adapter from the same EntityDefinition the kernel executes, so every backend reports the states and edges protocol artifact lifecycle prints; D-P5 closes.
owner: store
tags:
- backend
- store
relations:
- decomposes: epic:planning-store-as-backend
- depends_on: story:one-adapter-over-any-store
revision: 4
---
# Story: `describe_type` reports the ladder the kernel decides with

## Outcome

A harness asks the contract which statuses a story may hold and what may follow what, and gets the
answer the kernel will actually enforce — rather than reading `artifacts/lifecycles/story.yaml` and
hoping the two agree. D-P5 closes.

## Context

`TypeDescriptor::lifecycle` is `Option<LifecycleDescriptor>` (`crates/aep-contract/src/registry.rs:78`),
initialised `None` and assigned nowhere; `story:journal-backed-store` ticked this line and a review
struck it through. Meanwhile `aep-backend-markdown::kernel` already builds an `EntityDefinition` from
the kind's `ArtifactLifecycle` to decide every move, and `tests/kernel_equivalence.rs` proves it
agrees with the ladder over 800 status pairs. The descriptor should be *that definition*, rendered —
so the answer a harness reads and the verdict it gets cannot drift.

Wave H, story 2 (`docs/plan/store-waves-f-g-h.md`). After F2 (so it is filled once, in the adapter).

## Acceptance

- `describe_type` for every planning kind returns a `LifecycleDescriptor` with the initial state,
  every state, and every `(from, to)` edge — derived from the same `EntityDefinition` the kernel
  executes, not from a second reading of the YAML.
- An equivalence test: for every kind in this repository's store, the descriptor's edge set equals
  `protocol artifact lifecycle <kind>`'s output.
- Filled in the adapter, so the memory, markdown and SQLite backends report identically — pinned by
  a test that asks all three.
- A kind with `requires:` per rung (phase 3) reports which rungs cost evidence and which kind.
- D-P5 is marked closed in the design (`harness-planning-and-driver-design-v0.1.md:326`), the gap
  register and the limitations page, in the same commit.

## Out of Scope

Reporting the ladder for entity types that are not planning kinds (`aep.design/v1` and friends):
their ladders are domain commands, not lifecycle documents, and describing those is a different
descriptor.

## Open Questions

None outstanding.
