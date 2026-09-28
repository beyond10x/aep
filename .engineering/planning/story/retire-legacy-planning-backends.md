---
format: aep.planning-md/3
id: story:retire-legacy-planning-backends
kind: story
status: draft
title: The /1 journal, SQLite, Postgres and hybrid planning backends are deprecated, then removed
relations:
- serves: vision:O2
revision: 1
---
## Problem

Since 0.63.0 the Git-native store (`aep.project/5`) is the default and `aep plan store migrate git`
converts an `aep.project/1` store. The other planning backends remain: the `/1` Markdown layout with
its store-wide `journal.jsonl`, `aep-backend-sqlite`, `aep-backend-postgres` and
`aep-backend-hybrid`. They are the only reason AEP still depends on the Entity Runtime crates
`entity-sqlite`, `entity-postgres`, `entity-query` and `entity-remote` (each crate's Cargo.toml at
`1d8fc1d3`). The Git-native store needs only `entity-core` and `entity-store`.

## Outcome

1. A release deprecates them: opening such a store prints the migration command, as `/1` does today.
2. A later release removes them, and `cargo xtask deps` refuses a lockfile that holds
   `entity-sqlite`, `entity-postgres`, `entity-query` or `entity-remote`.

## Acceptance

- After removal, `cargo tree -i entity-sqlite` finds nothing and every gate step passes.
- `aep-service` (depends on `aep-backend-postgres` at tag 0.53.0) is named in the deprecation note.
