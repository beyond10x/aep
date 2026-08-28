---
format: aep.planning-md/1
id: story:sqlite-hydrates-on-open
kind: story
status: proposed
title: A populated database is read back on open, and the foreign-row refusal retires
summary: open enumerates and folds what the store holds, installing stored identities; needs entity-runtime's store enumeration; the written-set refusal is deleted.
owner: store
tags:
- backend
- store
relations:
- decomposes: epic:planning-store-as-backend
- depends_on: story:events-reach-the-store
revision: 6
---
# Story: A populated database is read back on open, and the foreign-row refusal retires

## Outcome

`protocol artifact list` against a SQLite plan prints the plan. Today it prints nothing, and the
second write against that file is refused by name.

## Context

`SqliteBackend::open` builds a fresh `MemoryBackend` and reads nothing back
(`crates/aep-backend-sqlite/src/lib.rs:60-76`). Identities are minted from a per-process counter, so
run two's first entity collides with run one's; the `written` set turns what would be silent
overwriting into a refusal — *"Hydration is P5; point this at an empty database until then."*
That refusal was the right call and it is also why the backend has no CLI surface: a store that must
be empty is not a store a plan can live in.

Two things are needed. From the runtime, a way to ask a store what it holds —
`entity_store::Store` has `load`, `revision_of`, `events`, `commit` and no enumeration
(`entity-runtime/crates/entity-store/src/lib.rs:147-190`); that is `story:store-enumeration`
there. From here, a hydration path that installs the **stored** identity rather than minting one.

Wave F, story 4 (`docs/plan/store-waves-f-g-h.md`). After F3 and the runtime story.

## Acceptance

- `open` enumerates every `aep.entity` instance, folds each from its events (or loads its state and
  checks the fold agrees), and installs it in `MemoryBackend` with its stored `EntityId`, relations
  and audit records — through the contract, as `seed` does, not through a side door.
- A second process against a populated file sees every entity, relation and audit record the first
  wrote, with the same identities; `history()` for an entity is identical in both processes.
- The `written` set and its refusal are **deleted**, with the test that asserted the refusal
  replaced by one that asserts the hydration — and a guard that plants an unhydrated row and fails.
- `protocol artifact history` over a SQLite store equals the same command over the markdown store of
  the same plan (a fixture plan seeded into both).
- Open time over this repository's own plan (113 artifacts) is measured and written into the story.

## Two facts F3 established that shape this story

- **A plan's history does not fold in general.** `entity_core::rehydrate` holds a creation event to
  `lifecycle.initial`; the contract creates an entity at whatever `status` the command carries
  (`seed` files an implemented story as `implemented`). So hydration takes the acceptance's second
  road — *load its state* — and the fold is a check only where the history begins at the initial
  status. `story:events-reach-the-store` § *Decisions* has the evidence.
- **Who/when/what-decided-on are in the event's `payload`**, not in an envelope: the runtime's
  providers store bare `DomainEvent`s. Hydrating audit records reads them from there until
  `entity-runtime` `story:the-store-keeps-the-envelope` gives them a home.
- **The pin moves here.** `store-enumeration` is not in `0.9.1`; this story bumps the one tag in
  `[workspace.dependencies]` to the release that carries it, and `dep-check` keeps it one.

## Out of Scope

Choosing the store from `project.yaml` (H1). Migration between stores — moving a plan from markdown
to SQLite is a seed, and a seed is a command sequence the contract already has.

## Open Questions

How a hydrated entity gets its stored identity. Decides: store owner. Default if nobody answers:
`MemoryBackend` gains an install-with-identity path used only by hydration, and the adapter derives
nothing — the store's id *is* the identity.
