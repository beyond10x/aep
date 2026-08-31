---
format: aep.planning-md/1
id: story:durable-command-batches
kind: story
status: implemented
title: One command is one durable optimistic transaction
summary: Adopt an atomic runtime batch SPI and remove stale overwrite and partial success.
relations:
- derived_from: epic:architecture-hardening
- serves: vision:O2
revision: 5
---
## Finding

`crates/aep-backend-entity/src/lib.rs` refreshes `Expect` from durable state immediately before each independent write. A stale backend can overwrite a newer revision, and a successful command can persist only a prefix of its placements and records.

## Acceptance

`entity-runtime` 0.14.0 exposes an ordered all-or-nothing batch extension implemented by memory, SQLite and Postgres. The adapter stages memory and projection state, derives expectations from the pre-command view, commits every placement and record once, and publishes only after success. Markdown recovers an interrupted batch before reads. Hybrid contract commands and reads use the authority. Two independently hydrated writers prove the loser cannot overwrite; an injected failure proves no partial durable or local state.

## Scope

- sibling `entity-runtime` provider crates — cited prerequisite.
- `crates/aep-backend-entity/`, durable backend wrappers and Markdown provider — cited from the write path.
- `crates/aep-backend-hybrid/` — inferred authority/read adjustment; confirm before editing.

## Release finding — 2026-08-31

The 0.35.0 release gate against PostgreSQL proved the old Postgres test still asserted the pre-batch latch behavior. The atomic contract is: a refused batch publishes neither a durable nor local candidate prefix, the caller receives the provider revision conflict, and an independently hydrated process reads the winning authority. The old story now records this supersession explicitly.
