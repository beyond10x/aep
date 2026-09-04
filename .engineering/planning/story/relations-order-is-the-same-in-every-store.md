---
format: aep.planning-md/1
id: story:relations-order-is-the-same-in-every-store
kind: story
status: draft
title: Two stores agree on the order of an artifact's relations
summary: markdown appends, SQLite orders by relation id, so show disagrees after a remove and re-relate.
relations:
- serves: vision:O2
revision: 1
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
