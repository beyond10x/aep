---
format: aep.planning-md/3
id: story:session-command-lifecycle-registry
kind: story
status: implemented
title: Carry startup lifecycles into command sessions
summary: Use the pinned lifecycle registry and provider-reserved identity floor in one fresh PostgreSQL command session.
relations:
- serves: vision:O2
revision: 6
transitions:
- {from: "draft", to: "proposed", at: "2026-08-31T10:06:12Z", actor: "human:operator", revision: 3, imported: true}
- {from: "proposed", to: "active", at: "2026-08-31T10:06:12Z", actor: "human:operator", revision: 4, imported: true}
- {from: "active", to: "implemented", at: "2026-08-31T10:14:40Z", actor: "human:operator", revision: 6, decided_on: {"recorded":{"test_result":1}}, imported: true}
---
## Outcome

A service process evaluates every fresh PostgreSQL command session against the same pinned lifecycle registry it advertises through type discovery.

## Context

The fresh session introduced in 0.36.0 reserves provider-scoped identities through `Identity::with_sequence_floor`, while configured lifecycles enter through `Identity::with_lifecycles`. A central authority needs both inputs in the same projection; choosing one must not silently discard the other.

## Acceptance

`Identity` can carry a lifecycle registry and a sequence floor together, `SessionPostgresBackend` exposes constructors that retain the registry for every command, and tests prove that hydration advances identities without losing the configured ladders.

## Out of Scope

Dynamic bundle activation, migration between lifecycle versions, and service-side identity or authorization policy remain separate work.

## Open Questions

None.

## Implementation

`Identity::with_lifecycles_and_sequence_floor` now retains both inputs in one projection. `SessionPostgresBackend` exposes schema-scoped and default constructors that retain the pinned registry, and every transaction combines that registry with its newly reserved identity range before semantic evaluation. A regression test hydrates the combined projection, verifies the registry remains present, and proves the next identity starts above the provider floor.
