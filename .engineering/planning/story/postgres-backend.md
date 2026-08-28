---
format: aep.planning-md/1
id: story:postgres-backend
kind: story
status: implemented
title: 'P5: aep-backend-postgres'
summary: 'The backend an organisation actually runs: concurrent writers, real transactions, the same contract.'
owner: store
tags:
- backend
- store
relations:
- decomposes: epic:planning-store-as-backend
- depends_on: story:sqlite-backend
- depends_on: story:sqlite-backend-adapter
- depends_on: story:one-adapter-over-any-store
revision: 8
---
# Story: P5 — `aep-backend-postgres`

## Outcome

A plan can live where an organisation already keeps things it cares about, with concurrent writers
and the backup story it already has, and nothing above the contract notices the difference.

## Context

This is the backend an organisation actually runs, and it is the first one where two people writing
at once is the normal case rather than the exception. The conformance suites are what decide whether
it is correct; the interesting part is that the same suites now have to be satisfied under real
concurrency rather than under a single-process assumption.

## Acceptance

- The sixteen suites pass against a live server, in CI, without a suite gaining a backend-specific
  branch. **Done** — `crates/aep-backend-postgres/tests/conformance.rs`,
  `the_postgres_backend_conforms_and_the_suites_catch_a_faulty_one`: `Level::Full` over a schema of
  its own on the server `ENTITY_POSTGRES_URL` names, and the three injected faults caught; the
  suites are `aep-conformance`'s unchanged. CI runs them against a `postgres:16` service
  (`.github/workflows/ci.yml`); `task check` gains `postgres-check`, which prints
  `postgres-check: skipped, ENTITY_POSTGRES_URL unset` on a laptop without one. Green against
  `postgres:16` locally on 2026-08-28.
- Two concurrent writers to one artifact resolve to one accepted write and one refusal that names the
  revision it lost to — not a silent last-writer-wins. **Done** —
  `two_processes_writing_one_artifact_resolve_to_one_accepted_and_one_refusal_naming_the_revision`:
  two backends over one database, both hydrated at revision 1, both writing from it on two threads;
  one lands, the other is refused with *expected revision 1, found revision 2* and **latches** — its
  memory and the database disagree, and it says so rather than carrying on; a third process reads
  revision 2. The refusal names the revision because the provider's row lock serialises the two and
  the adapter reads the expectation rather than assuming it (F2's decision, paying off here).
- Schema creation and upgrade are a command, not a README instruction. **Done** — the provider's
  idempotent `migrate`, run by `connect`; `connect_in_schema` keeps several plans in one database.
- The dependency and the CI service it requires are recorded in `AGENTS.md` § *Dependencies*.
  **Done** — the fourth `entity-*` crate at the one tag, `postgres` 0.19 without default features and
  no TLS backend; the network rule names `postgres-check` as its one opted-in exception.

`protocol conformance --backend postgres --store <url>` runs the suites from the command line; the
backend has no scratch server to invent, so `--store` is required and its absence refused by name.

## Decision taken

The open question — whether the driver's store lock becomes an advisory lock here — is not decided
by this story and does not block it: the file lock is per project directory, and a Postgres plan is
still reached through one. It stays open on the story.

## Out of Scope

Multi-tenancy, row-level security and anything about who may read which plan. Authorisation is the
protocol's job, not the backend's, and putting it here would create a second place that decides.
Opening a Postgres plan from `protocol artifact` is `story:store-selection-in-project-yaml`.

## Open Questions

Whether the driver's store lock becomes an advisory lock in this backend rather than a file. Decides:
driver and store owners together. Not blocking: the file lock is per store, and a Postgres store is
still reachable through one project directory.

## Re-scoped for wave H, proposed 2026-08-28

The provider is **`entity-runtime`'s** (`story:postgres-provider` there, shipped in 0.12.0/0.12.1),
and this crate is `EntityBackend<PostgresStore>` — a type over the adapter
`story:one-adapter-over-any-store` extracted. What stayed here: the CI service, the sixteen suites
against it, the two-writers acceptance above, and `AGENTS.md` § *Dependencies*. *"Schema creation
and upgrade are a command"* is met by the provider's idempotent `migrate`, called on connect.

Edges: `depends_on: story:sqlite-backend` points at the story `story:sqlite-backend-adapter`
superseded; the store has no `unrelate`, so `depends_on: story:sqlite-backend-adapter` and
`depends_on: story:one-adapter-over-any-store` are beside it and this line says which is live.
Plan: `docs/plan/store-waves-f-g-h.md` § Wave H.
