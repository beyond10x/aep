---
format: aep.planning-md/3
id: story:relations-order-is-the-same-in-every-store
kind: story
status: active
title: Two stores agree on the order of an artifact's relations
summary: markdown appends, SQLite orders by relation id, so show disagrees after a remove and re-relate.
relations:
- serves: vision:O2
- decomposes: epic:zero-open-issues-2026-11-09
scope:
- confidence: inferred
  path: crates/edge/aep-cli/src/planning.rs
- confidence: cited
  path: crates/edge/aep-cli/tests/store_selection.rs
- confidence: inferred
  path: crates/plan/aep-backend-entity/src/lib.rs
- confidence: inferred
  path: crates/plan/aep-backend-postgres/src
- confidence: inferred
  path: crates/plan/aep-backend-sqlite/src
- confidence: inferred
  path: docs/design/git-native-planning-store-v0.1.md
revision: 10
transitions:
- {from: "draft", to: "proposed", at: "2026-11-09T22:07:27Z", actor: "human:timo", revision: 9}
- {from: "proposed", to: "active", at: "2026-11-09T22:07:28Z", actor: "human:timo", revision: 10}
---
# Story: Two stores agree on the order of an artifact's relations

## Context

After `relate`, `unrelate`, then `relate` again, the markdown backend lists the edge where the line
was appended and the SQLite backend lists it where its new relation id falls. `aep plan artifact
show <id>` therefore prints the relations in a different order depending on which store answered,
for the same sequence of commands.

Found while implementing `unrelate` (0.53.0).
`crates/edge/aep-cli/tests/store_selection.rs` currently asserts each store separately at that
point, with a comment saying why, rather than comparing one store's answer with the other's.

## Acceptance

One ordering rule is decided and written down, both backends produce it, and
`store_selection.rs` compares the two stores' `show` output directly instead of asserting them
apart.

## Triage 2026-11-09

Still present at `824869fec`. `crates/edge/aep-cli/tests/store_selection.rs:562-568` asserts each store on its own. Reproduced with aep 0.79.0: after `relate a depends_on b`, `relate a depends_on c`, `unrelate a depends_on b`, `relate a depends_on b`, `show task:a` lists `task:b, task:c` on SQLite and `task:c, task:b` on the Git-native store.

The rule, decided at triage (coordinator): relations are listed in the order they were added, and an edge that was removed and added again is listed where it was added again. That is what the Git-native file, the authority of an `aep.project/5` store, already holds; the entity stores are brought to it. The rule is written in `docs/design/git-native-planning-store-v0.1.md` § 4.2.
