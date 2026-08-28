---
format: aep.planning-md/1
id: story:postgres-backend
kind: story
status: draft
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
revision: 4
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
  branch.
- Two concurrent writers to one artifact resolve to one accepted write and one refusal that names the
  revision it lost to — not a silent last-writer-wins.
- Schema creation and upgrade are a command, not a README instruction.
- The dependency and the CI service it requires are recorded in `AGENTS.md` § *Dependencies*.

## Out of Scope

Multi-tenancy, row-level security and anything about who may read which plan. Authorisation is the
protocol's job, not the backend's, and putting it here would create a second place that decides.

## Open Questions

Whether the driver's store lock becomes an advisory lock in this backend rather than a file. Decides:
driver and store owners together. Not blocking: the file lock is per store, and a Postgres store is
still reachable through one project directory.

## Re-scoped for wave H, proposed 2026-08-28

The provider is **`entity-runtime`'s** (`story:postgres-provider` there), and this crate is
`EntityBackend<PostgresStore>` — a type over the adapter `story:one-adapter-over-any-store` extracts.
What stays here: the CI service, the sixteen suites against it, the two-writers acceptance above,
and `AGENTS.md` § *Dependencies*. *"Schema creation and upgrade are a command"* is met by the
provider's idempotent `migrate`, called from `protocol artifact` on open.

Edges: `depends_on: story:sqlite-backend` above points at the story `story:sqlite-backend-adapter`
superseded; the store has no `unrelate`, so `depends_on: story:sqlite-backend-adapter` and
`depends_on: story:one-adapter-over-any-store` are added beside it and this line says which is live.
Plan: `docs/plan/store-waves-f-g-h.md` § Wave H.
