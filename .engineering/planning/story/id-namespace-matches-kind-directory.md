---
format: aep.planning-md/3
id: story:id-namespace-matches-kind-directory
kind: story
status: draft
title: An artifact whose id namespace differs from its kind directory is refused or written
summary: doc:zz filed under story/ validates but every write to it is refused, because writes resolve by namespace and seeding by kind.
owner: aep
tags:
- planning-store
relations:
- serves: vision:O2
revision: 1
---
# Story: An artifact whose id namespace differs from its kind directory is refused or written

## Outcome

A store never holds an artifact that validates and then refuses every write.

## Context

`planning.rs` resolves an artifact's locator from the namespace of its id, while seeding places it
by its kind. A file `doc:zz` filed under `story/` passes `aep plan artifact validate`, and then every
write to it (`body`, `set`, `relate`, `move`) is refused because the write looks for `doc:zz` where
its namespace says, and nothing is there.

Found by the planning-store review of 2026-11-23 (`aep-planning-store-review`).

## Acceptance

- Either `aep plan artifact validate` refuses a store holding `doc:zz` under `story/`, naming the
  file and the two places, or `aep plan artifact body doc:zz` on that store writes the file where
  it stands. A test holds the chosen outcome; no store both validates and refuses the write.

## Out of Scope

Moving files between kind directories. Retiring an artifact stays `move --to archived`.

## Open Questions

- Refuse or resolve? Refusing keeps the id, the kind and the directory one fact, and is the
  default of this story. Resolving needs the write path to read the kind, which seeding already
  does.
