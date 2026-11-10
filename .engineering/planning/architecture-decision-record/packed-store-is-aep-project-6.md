---
format: aep.planning-md/3
id: architecture-decision-record:packed-store-is-aep-project-6
kind: architecture-decision-record
status: accepted
title: A store that holds a pack is aep.project/6
relations:
- decides: epic:store-pack-closed-artifacts
- informed_by: architecture-design:store-pack
revision: 2
transitions:
- {from: "proposed", to: "accepted", at: "2026-11-10T10:41:09Z", actor: "human:timo", revision: 2}
---
# Decision: a store that holds a pack is `aep.project/6`

**This changes `aep.project/N`.** A store becomes `aep.project/6` when `aep plan store pack` first
writes a pack, and returns to `aep.project/5` when `aep plan store unpack --all` leaves no pack. A
store that never packs stays `aep.project/5`, byte for byte.

## Context

`architecture-design:store-pack` moves closed artifacts and their evidence out of the loose layout
into `.engineering/packs/*.pack`. An `aep` that does not know packs reads only the loose files, so on a
packed store it would answer `list`, `graph`, `validate` and `history` over fewer artifacts and report
dangling relations, without saying why. The operator's constraint: an older `aep` meeting a packed
store must refuse by name, not silently see fewer artifacts.

`project.yaml` is the one file every verb reads first, and both of its closed fields refuse unknown
content in aep 0.79.0. Measured 2026-11-10 (session scratch, aep 0.79.0):

- `version: aep.project/6`:
  `[unsupported_protocol_version] project.version: this build reads `aep.project/5`, not `aep.project/6` (hint: upgrade the tooling rather than reinterpreting the document)`
- `store: {git: {packs: {}}}` under `aep.project/5`:
  `[type_mismatch] project.store: store.git: unknown field `packs`, there are no fields`

## Options

1. **`aep.project/6`, written by `pack`** (taken).
2. A feature key inside `/5` (`store: {git: {packs: {}}}`).
3. No marker; rely on `validate` reporting dangling relations.

## Decision and why

Option 1. Both 1 and 2 are refused by name by every released `aep`, so neither needs the operator's
stop. Option 1 is taken because:

- the refusal it produces is the existing version refusal, which names the version and says to
  upgrade; option 2's refusal names an unknown field and says nothing about upgrading;
- a packed store *is* a different on-disk layout: a `/5` reader of any kind (other tools that read
  `.engineering/planning/` and check `version`, such as conductor's repository collector) would
  undercount it, and the version is what such a reader checks;
- the change is confined to stores that pack: `pack` writes `/6`, `unpack --all` writes `/5` back, and
  a `/5` store is untouched, so adopters that do not pack see no change.

Option 3 is refused: a packed artifact nobody relates to disappears with no dangling edge to report.

## Consequences

- `aep` reads `/5` and `/6`; every verb that accepts `/5` accepts `/6` (`ProjectVersion` in
  `crates/govern/aep-domain/src/project.rs`).
- `validate` refuses a `/5` store holding a pack and a `/6` store holding none.
- The `aep:planning` and `aep:upgrade` skills (sibling `agentplugins` repository) carry a version
  table that lists `/5` as current; they need a `/6` row. Not edited here.
- Downstream readers of the files (conductor, metaharness) will see `/6` on a packed store; their
  owners are told, not changed here.
