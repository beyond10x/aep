---
format: aep.planning-md/1
id: story:sqlite-hydrates-on-open
kind: story
status: implemented
title: A populated database is read back on open, and the foreign-row refusal retires
summary: open enumerates and folds what the store holds, installing stored identities; needs entity-runtime's store enumeration; the written-set refusal is deleted.
owner: store
tags:
- backend
- store
relations:
- decomposes: epic:planning-store-as-backend
- depends_on: story:events-reach-the-store
revision: 9
---
# Story: A populated database is read back on open, and the foreign-row refusal retires

## Outcome

A SQLite plan, opened by a second process, is the plan: every entity, relation and audit record the
first process wrote, under the same identities, with the same history. Before this the second
process saw nothing, and its first write was refused by name.

## Context

`SqliteBackend::open` built a fresh `MemoryBackend` and read nothing back. Identities were minted
from a per-process counter, so run two's first entity collided with run one's; the `written` set
turned what would be silent overwriting into a refusal — *"Hydration is P5; point this at an empty
database until then."* That refusal was the right call and it is also why the backend had no CLI
surface: a store that must be empty is not a store a plan can live in.

Two things were needed. From the runtime, a way to ask a store what it holds —
`entity-runtime` `story:store-enumeration`, shipped as `StateProvider::ids` in **0.10.0**
(2026-08-28). From here, a hydration path that installs the **stored** identity rather than
minting one.

Wave F, story 4 (`docs/plan/store-waves-f-g-h.md`). After F3 and the runtime story.

## Acceptance

- `open` enumerates every `aep.entity` instance, folds each from its events (or loads its state and
  checks the fold agrees), and installs it in `MemoryBackend` with its stored `EntityId`, relations
  and audit records — through the contract, as `seed` does, not through a side door. **Done, the
  second road** — `hydrate` in `crates/aep-backend-entity/src/lib.rs` loads each instance's state
  and rebuilds its history from the stored events' seals; it does not fold, because a plan's
  history does not fold in general (F3's finding: the runtime holds a creation event to
  `lifecycle.initial`, and the contract creates at whatever status the command carries). Relations
  (`aep.relation`), audit records (`aep.audit`) and the idempotency memory (`aep.applied`) are
  persisted per command and hydrated with the entities; the install path is `MemoryBackend`'s own
  public store surface (`insert_entity`, `insert_relation`, `record_revision`, `record_audit`,
  `remember`), which invariant 14's `write_surface.rs` does not count as a contract write path —
  it is the store the contract's one write path lands in.
- A second process against a populated file sees every entity, relation and audit record the first
  wrote, with the same identities; `history()` for an entity is identical in both processes.
  **Done** — `tests/hydration.rs`:
  `a_second_process_sees_everything_the_first_wrote_with_the_same_identities` compares the whole
  view (entities with metadata, relations, six audit records including a refusal's, history)
  between the two processes; `a_replay_across_processes_is_recognised`;
  `a_removed_relation_stays_removed_after_reopen`.
- The `written` set and its refusal are **deleted**, with the test that asserted the refusal
  replaced by one that asserts the hydration — and a guard that plants an unhydrated row and fails.
  **Done** — `a_second_run_reads_the_first_back_and_continues_past_it` (SQLite crate) and
  `a_second_process_mints_identities_past_the_first_ones`;
  `a_row_this_backend_cannot_read_back_refuses_the_open_by_name` plants an `aep.entity` row with
  no `$aep` metadata and the open is refused naming the row.
- `protocol artifact history` over a SQLite store equals the same command over the markdown store of
  the same plan (a fixture plan seeded into both). **Done through the contract, not the verb** —
  `crates/protocol-cli/tests/sqlite_plan.rs` seeds this repository's own plan into SQLite with the
  same `seed::from_manifest` the markdown backend uses (made generic over `CommandService` for it),
  reopens the file, and asserts `history()` and `metadata` identical to the markdown backend's for
  all 124 artifacts. The *verb* over a SQLite store is wave H, story 1 — every `protocol artifact`
  verb opens through `store:` — and this line is corrected here rather than ticked on a Rust test
  it called a command.
- Open time over this repository's own plan (113 artifacts) is measured and written into the story.
  **Done** — 124 artifacts and 241 relations on 2026-08-28: **reopened (hydrated) in 119 ms**;
  seeded through the contract in 11.5 s (debug build, three SQLite transactions per command —
  entity+event, audit, applied; one transaction per command is a runtime `Store` change, noted
  for wave H).

## Decisions taken

- **Identities: the store's, unparsed.** D-F1's default. `MemoryBackend` mints past whatever is
  held — `next_entity_id`, `next_relation_id`, `next_audit_id` skip an id already in the store —
  so nothing reads structure out of an id (invariant 13) and nothing persists the counter.
- **Metadata rides in the instance** under the reserved `$aep` field (the `EntityMetadata` and the
  archived flag), so one `commit` carries the whole entity and `changed` records that metadata
  moved. The seal in each event's `payload` gained `at` (epoch milliseconds, exact) and
  `executor`, so a rebuilt `RevisionRecord` equals the original byte for byte.
- **Records carry no events.** R-83 is about an instance whose state moves; a relation, an audit
  record and an idempotency entry are records of something that already happened.
- **A row that cannot be read back refuses the open.** Skipping it would answer about part of the
  store and say nothing.

## Out of Scope

Choosing the store from `project.yaml` (H1). Migration between stores — moving a plan from markdown
to SQLite is a seed, and a seed is a command sequence the contract already has (and
`seed::from_manifest` now accepts any `CommandService`).

## Open Questions

None outstanding.
