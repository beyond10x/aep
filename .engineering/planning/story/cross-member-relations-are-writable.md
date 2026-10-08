---
format: aep.planning-md/3
id: story:cross-member-relations-are-writable
kind: story
status: implemented
title: A store can write the cross-member edge the workspace reads
relations:
- decomposes: epic:one-cli-many-repositories
- serves: vision:O2
revision: 4
transitions:
- {from: "draft", to: "proposed", at: "2026-10-08T07:35:36Z", actor: "human:timo", revision: 2}
- {from: "proposed", to: "active", at: "2026-10-08T07:35:36Z", actor: "human:timo", revision: 3}
- {from: "active", to: "implemented", at: "2026-10-08T08:33:22Z", actor: "human:timo", revision: 4, decided_on: {"recorded":{"test_result":1,"review_outcome":1}}}
---
# Story: A store can write the cross-member edge the workspace reads

## Outcome

An owner whose story waits on another member's story records that as a `depends_on` (or any other
relation) naming `<member>/<kind>:<name>`, with the same verbs that write a local edge, and
`aep plan workspace crossings` then lists it.

## Context

aep 0.69.0, reproduced 2026-10-08 in a two-member workspace (`alpha`, `beta`, each declaring both):

- `aep plan artifact relate story:source-a depends_on beta/story:target-b` refuses:
  "… does not hold `beta/story:target-b`, so `story:source-a depends_on beta/story:target-b` would be
  an edge to nothing".
- `aep plan artifact new story source-c --relate depends_on:beta/story:target-b` refuses:
  "`beta/story:target-b` cannot be given an address: invalid locator identifier
  \"ep://planning/store/beta/story/target-b\": the kind contains disallowed character '/'".
- `aep plan workspace crossings` reads such edges (`crates/edge/aep-cli/src/workspace.rs` `crossings`),
  and `ArtifactGraph::build_in_workspace` already admits a target naming another declared member
  (`ArtifactRelation::crosses_to_a_declared_member`, `crates/govern/aep-domain/src/artifact.rs`).

So a store cannot write what the workspace and the validator read. Owners record an
`upstream-blocker` naming the other member's story until this lands.

Where the refusals live: the pre-check in `relate` (`crates/edge/aep-cli/src/planning.rs`), the
target resolution in `write_through_a_command`, and `relate_through_a_command`, which issues a
`CreateRelation` whose target must be a local entity. A migrated crossing is already kept in the
entity body's `relations` field and merged on projection (`document_from_entity`,
`crates/plan/aep-backend-markdown/src/projection.rs`); the write path has no equivalent.

## Specification

No ESS specification exists in this repository. The change adds no noun, command, outcome or
configuration: the relation kinds, the `<member>/<kind>:<name>` spelling and the crossing rule are
already defined; the write verbs are brought in line with the validator.

## Acceptance

- In a two-member workspace, `relate <id> depends_on <other-member>/story:<name>` exits 0, writes the
  edge into the artifact's `relations`, and `aep plan workspace crossings` lists it as resolved.
- `new … --relate depends_on:<other-member>/story:<name>` does the same at creation.
- `unrelate` takes the same edge back.
- A target naming a member the workspace does not declare is still refused, and so is a target naming
  this store's own member that the store does not hold.
- `aep plan artifact validate` accepts the store afterwards; a SQLite/Postgres rebuild of the
  projection neither drops nor duplicates the crossing.
