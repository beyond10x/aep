---
format: aep.planning-md/1
id: story:one-adapter-over-any-store
kind: story
status: implemented
title: 'aep-backend-entity: one adapter over any entity-store provider'
summary: The contract-over-a-Store adapter is extracted from aep-backend-sqlite and made generic over entity_store::Store; SqliteBackend becomes an instantiation, and every later backend is a type.
owner: store
tags:
- backend
- store
relations:
- decomposes: epic:planning-store-as-backend
- depends_on: story:one-entity-runtime-pin
revision: 8
---
# Story: `aep-backend-entity` — one adapter over any `entity-store` provider

## Outcome

The next durable backend is a type, not a crate of logic: `EntityBackend<PostgresStore>` is a line,
and it passes the sixteen suites because the adapter already does and the provider already passed
the runtime's.

## Context

`SqliteBackend` (`crates/aep-backend-sqlite/src/lib.rs`, 336 lines before this story) was already
the adapter this story names — `MemoryBackend` for the contract logic, a latch, and `persist` over an
`entity_store::Store`. Nothing in it was SQLite-specific except the two constructors. `MarkdownBackend`
(`backend.rs`, 883 lines) is the same arrangement written a second time with a hand-written store
underneath, which wave G addresses; this story does not touch it.

Extracting the generic form now is what lets F3 (events) and F4 (hydration) be written once, and
what makes G2, H3 and H4 instantiations.

Wave F, story 2 (`docs/plan/store-waves-f-g-h.md`). After F1.

## Acceptance

- `crates/aep-backend-entity` exports `EntityBackend<S: entity_store::Store>` implementing
  `CommandService` and `QueryService` exactly as `SqliteBackend` did: apply in `MemoryBackend`,
  then `commit` with a **read** `Expect`, latch on failure, latch covers reads. **Done** —
  `crates/aep-backend-entity/src/lib.rs`; the latch-covers-reads half has its own test now
  (`a_latched_adapter_refuses_reads_as_well_as_writes`, over a provider whose every commit fails),
  which the SQLite crate never had.
- `aep_backend_sqlite::SqliteBackend` is `EntityBackend<entity_sqlite::SqliteStore>` (a type alias
  or a newtype with the two constructors); its public API does not change. **Done** — a newtype,
  because the constructors are inherent methods and an alias cannot carry one from another crate;
  `open`, `in_memory`, `latched`, `len`, `is_empty` unchanged, plus `as_entity_backend()` for a
  caller that wants the generic surface. `the_newtype_forwards_the_whole_contract` runs the suites
  through the forwarding, which a forwarding mistake fails and an alias could not have.
- The sixteen suites pass at `Level::Full` through the generic adapter over `SqliteStore` **and**
  over `entity_store::MemoryStore`, and the faulty-backend guard fails both. **Done** —
  `crates/aep-backend-entity/tests/conformance.rs`: 5 tests, all green 2026-08-28.
- `cargo xtask guards` reports no test body duplicated — the conformance tests move rather than copy.
  **Done** — `2498 test body/bodies compared, 0 duplicated across crates`. The suites left
  `aep-backend-sqlite`; what stays there is the second-handle read-back and the foreign-row refusal.
- The `written` set and the foreign-row refusal move with the code unchanged; retiring them is F4.
  **Done** — same set, same refusal, message now says "store" and "F4" instead of "database" and
  "P5"; `a_second_run_refuses_to_overwrite_what_the_first_one_stored` still passes.

## What else moved with it

- The `entity-*` pins are now declared **once**, in `[workspace.dependencies]`, and the three crates
  say `entity-core.workspace = true` — F1's "one pin" made literal, which F4's bump will thank.
- `docs/guide/backend.md`, `website/docs/status/limitations.md`, `docs/status.md` § *Repository
  layout* and `AGENTS.md` § *Evidence and labelling* describe the adapter. `AGENTS.md`'s "three
  implementors" sentence still holds: the adapter is not a fourth implementor, it is what the third
  one is made of.
- `cargo clippy --workspace --all-targets -- -D warnings` exit 0 with the new member.

## Out of Scope

The markdown backend (G2), events (F3), hydration (F4). This story moves code and adds a type
parameter; a diff that also changes behaviour cannot say which half a finding came from.

## Open Questions

None outstanding.
