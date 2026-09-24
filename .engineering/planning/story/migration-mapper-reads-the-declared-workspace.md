---
format: aep.planning-md/1
id: story:migration-mapper-reads-the-declared-workspace
kind: story
status: active
title: The migration mapper builds the artifact graph with the store's declared workspace members
relations:
- serves: vision:O2
scope:
- confidence: cited
  path: CHANGELOG.md
- confidence: cited
  path: crates/edge/aep-cli/src/planning.rs
- confidence: cited
  path: crates/edge/aep-cli/src/store_command.rs
- confidence: cited
  path: crates/govern/aep-domain/src/artifact.rs
- confidence: cited
  path: crates/plan/aep-contract/src/migration/command.rs
- confidence: cited
  path: crates/plan/aep-planning-migration/src/mapping.rs
revision: 11
---
## Outcome

`aep plan store migrate dry-run` and `apply` admit a Markdown store whose `.engineering/workspace.yaml`
declares members and whose artifacts carry relations into those members, exactly as `aep plan
artifact validate` already admits it. A migration refusal that comes from the mapper names the
mapper's coordinate in the receipt instead of collapsing to `semantic_mismatch` at the selector.

## Why

Found 2026-09-21 preparing M4f, the AEP planning store's own cutover (wave-validate-v2-20260920,
`m4f-aep-preflight-20260921.md`). The dry-run on the AEP canonical store (313 artifacts, 325
captured nodes) is refused: `outcome.kind refused`, `refusals[0].code semantic_mismatch`,
`at.kind selector`, path `.`. The same command was admitted on the Eventlog, Entity Runtime,
Service SDK, ESS and Connectors stores.

Cause, reproduced with a probe example (untracked, `aep-planning-migration/examples/probe_mapping.rs`)
that calls `capture_markdown` then `markdown_boundaries_raw` and prints the error whole:

```
MAPPING ERROR (Debug): Invalid { coordinate: "markdown/graph" }
```

Controls on the pre-cutover archives, both Ok: Entity Runtime 113 nodes → 267 histories;
Eventlog 85 nodes → 170 histories.

Source at 763d195be:

- `crates/plan/aep-planning-migration/src/mapping.rs:181` calls `report.graph()`.
- `aep-backend-markdown` `store.rs:360-362`: `graph()` is `graph_in_workspace(std::iter::empty())`,
  the artifact graph with no workspace members.
- Every ordinary command builds the graph with the declaration: `planning.rs:2241, 2862, 3490,
  3697, 5416, 6225` call `graph_in_workspace(declared_members(repository))`.
- The AEP canonical store is the only one of the six with an `.engineering/workspace.yaml`
  declaring members. Neither control archive has one.
- `store_command.rs:1213` maps any `MappingError` to `CommandRefusalCodeV1::SemanticMismatch`, and
  `dry_refusal` (`:1335`, `selector_refusal` `:3574`) attaches it to the selector coordinate, so the
  mapper's coordinate never reaches the receipt.

Reproduced without the probe: on a faithful copy, `plan artifact validate` with `workspace.yaml`
present is valid; with `workspace.yaml` parked aside it reports, verbatim:

```
1 problem(s):
  - [undeclared_reference] artifacts.story:assemble-across-sources.relations[2]:
    story:assemble-across-sources informed_by points at entity-runtime/story:typed-references,
    which the manifest does not declare (hint: declare the target artifact, or drop the relation;
    a dangling edge cannot be checked later)
```

A crossing the workspace resolves is read by the mapper as a dangling edge. The store is correct;
the mapper reads it without the declaration that makes it correct. Deleting `workspace.yaml` to get
the migration through would migrate a store whose relations then dangle for real, so that is not
the fix.

## Acceptance

- Red first: a Markdown fixture with `.engineering/workspace.yaml` declaring one member and one
  artifact carrying a relation into that member. `migrate dry-run` at 763d195be refuses it with
  `semantic_mismatch`; after the change it is admitted, and the migrated store's `validate` passes
  with the same workspace declaration the Markdown store had.
- The mapper builds the graph with `graph_in_workspace(declared_members(...))`, the same call the
  read commands make.
- Decision, 2026-09-21 (pass 1 blocker): a workspace crossing survives migration as the authored
  relation data of its entity, not as an `aep.relation` record — a relation record whose target is
  another workspace member is a model change and is not this story. It is read back identically on
  the Eventlog arm through every read path that lists an artifact's relations (show, list JSON,
  graph, validate), compared as values against the Markdown arm. If a read path answers from
  relation records only and loses the crossing, that is reported and the decision is revisited.
- The receipts do not lie: dry-run and apply count imported relation records and workspace
  crossings separately, and the two sum to the source's declared relations (on the AEP store:
  563 records + 1 crossing = 564, never "564 advertised, 563 delivered"). The user-facing text
  names the crossings.
- Decision, 2026-09-21 (pass 2 blockers F1/F2): the crossing predicate knows which member this
  store is. An edge qualified with the store's own member name is a local edge: resolved against
  this store's identities, imported as a relation record, refused as dangling (naming the document)
  when its target is absent. An edge qualified with another declared member is a workspace
  crossing. Any other member name is a misspelling and a dangling edge. "Own" comes from the same
  reader the read commands use (the member whose source resolves to this repository), read once at
  the command edge. The same predicate serves `ArtifactGraph::build_in_workspace`, so `validate`
  stops printing valid for a dangling self-prefixed edge (pass 2 F3, pre-existing, fixed here).
  Cases assert the side each edge lands on (record or crossing), not only the sum: the sum is one
  if/else over one iteration and always equals the source.
- Every receipt that reports an inventory (dry-run, apply, verify, rebuild, eventlog inspect)
  carries the same two counts from the same computation (pass 2 F4); the inspect reader of the
  declaration has a case (F5).
- An `.engineering/workspace.yaml` that exists and does not parse is refused with the file and the
  parse error, never read as "declares nothing" (pass 1 finding 3).
- The unit's own case asserts on the authority's relation surface as well as the projected
  document, so it cannot pass by body-merge alone (pass 1: a green case, a true assertion, the
  wrong subject).
- Red first: a mapper refusal reaches the receipt with the mapper's coordinate (`markdown/graph`
  or the document path), in `--format json` as a field and in plain output as text; the selector
  coordinate is used only for selector failures.
- A store without `workspace.yaml` maps exactly as before (the two control archives stay Ok; the
  ER and Eventlog migrations are the regression fixtures).
- Repository 16-step gate green; `tests/drift.rs`, `planning_cli.rs`, `wave_derivation.rs` counts
  unchanged.
