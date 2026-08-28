---
format: aep.planning-md/1
id: story:hybrid-backend
kind: story
status: implemented
title: 'P6: aep-backend-hybrid, and what its atomicity actually is'
summary: A composite that writes to the database first and projects to markdown second, compensating through the protocol's own inverse commands when the projection fails.
owner: store
tags:
- backend
- store
relations:
- decomposes: epic:planning-store-as-backend
- depends_on: story:postgres-backend
- depends_on: story:markdown-documents-as-a-store
- depends_on: story:store-selection-in-project-yaml
revision: 8
---
# Story: P6 — `aep-backend-hybrid`, and what its atomicity actually is

## Outcome

A team gets both halves at once: the plan is in a database their tooling can query, and it is also in
markdown their pull requests can review — and when one of the two writes fails, somebody can say
exactly what state the plan is in.

## Context

The shape is settled and the guarantee is not. A composite holds a primary (SQL) and a projection
(markdown); the write goes to the primary first because that is the one with transactions, and the
projection follows. A projection that fails after a committed primary write leaves the two
disagreeing, and the composite compensates through the protocol's own inverse command rather than
through a `DELETE`. What exact atomicity that buys is an open design question, and the honest options
run from *eventually consistent with a repair verb* to *two-phase with a durable intent log*.

## Acceptance

- The atomicity guarantee is **written down first**, as a decision with its rejected alternatives,
  before the crate exists.
- The sixteen suites pass against the composite.
- A projection failure after a committed primary write is compensated by an inverse command, and the
  compensation is in the journal — not a repair somebody ran by hand.
- A divergence between primary and projection is detectable by a verb, and the verb says which side
  is authoritative.

## Out of Scope

Choosing the guarantee without P3's suites to test against, which would be guessing. That is why this
story is last in the epic rather than merely later.

## Open Questions

None outstanding. The guarantee is the runtime's declared policy, cited (below).

## Re-scoped for wave H, proposed 2026-08-28

The composite is **`entity-runtime`'s `Hybrid<L: Store, R: Store>`** (`crates/entity-remote/src/hybrid.rs:168`),
instantiated as `EntityBackend<Hybrid<MarkdownProvider, SqliteStore>>` — the plan in markdown for
pull requests and in SQLite for tooling, both local, no network. The open question above — *the
guarantee itself* — is answered by citing rather than choosing: the runtime's `store-v0.1.md` § 10
declares authority, read path, unreachable and divergence behaviour as required words with no
default (R-106), records a losing write as a divergence rather than swallowing it (R-107), and
`catch_up` merges nothing (R-108). The first acceptance line is met by that citation, and the four
words are typed in `project.yaml` (`story:store-selection-in-project-yaml`). Two verbs are ours:
`protocol artifact divergences` and `protocol artifact catch-up`.

Edges: `depends_on: story:postgres-backend` above is no longer true — a hybrid of two local stores
needs no Postgres. `depends_on: story:markdown-documents-as-a-store` and
`depends_on: story:store-selection-in-project-yaml` are added beside it; this line says which is live.
Plan: `docs/plan/store-waves-f-g-h.md` § Wave H.

## Delivered 2026-08-28

| acceptance line | how it is met |
|---|---|
| guarantee written down first, with rejected alternatives | cited: `store-v0.1.md` § 10 (R-106..R-108); the two-phase commit with an intent log is the alternative the runtime's `OnDivergence::Refuse` doc rejects, and `docs/guide/backend.md` § *The plan kept twice* carries the citation |
| sixteen suites pass against the composite | `crates/aep-backend-hybrid/tests/conformance.rs`: 16 suites with the markdown side as authority and with the replica as authority; the faulty-backend guard catches three injected faults; `protocol conformance --backend hybrid` from the command line |
| a failure on one side after the other committed is compensated, in the record | re-read as the runtime reads it: recorded as a `Divergence` (R-107) — written to `divergences.jsonl` beside the plan by `HybridBackend::execute` — and replayed by `catch_up` (R-108), which merges nothing and writes back what it could not replay. The "inverse command" of the original wording is not what the runtime does; nothing is undone, and the fact is kept. `tests/divergence.rs` runs the two-process round trip and the conflict a person has to settle |
| a divergence is detectable by a verb that says which side is authoritative | `protocol artifact divergences` (exit 1 while outstanding; `authority: local|replica` in text and JSON); `protocol artifact catch-up`; `store_selection.rs::a_hybrid_records_a_write_its_replica_refused_and_the_next_process_catches_it_up` through the real binary |

What it took upstream: `entity-runtime` 0.12.2 (an observation lands at an unchanged revision in
every provider — the plan's evidence and edges are observations) and 0.13.0 (`Divergence`
serialises; `Hybrid::remember`), because one process per verb is a shape the runtime's long-running
`Hybrid` had not been asked about.

What is left where it was: `local` must be `markdown` and `replica` must be `sqlite` or `postgres` —
the projection is the plan's, and a replica of another shape is a store this build does not open;
the refusal names what is supported.
