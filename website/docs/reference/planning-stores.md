---
title: Planning stores
description: Which planning store backends AEP supports, how project.yaml selects each one, and the migration path for every store AEP has removed.
---

# Planning stores

Every planning store backend AEP has shipped: the ones a release reads, how
[`project.yaml`](./project-file.md#store) selects each, and how to leave the ones it no longer
reads. The tables are generated from the store catalog in the source, and the gate fails while this
page disagrees with it. A release marked *next release* is not tagged yet.

{/* generated:planning-stores:begin — do not edit; run `cargo xtask status` */}

## Supported

| Store | `version` | `store` selector | Also requires | Since | Notes |
|---|---|---|---|---|---|
| Git-native | `aep.project/5` | `store: { git: {} }` | `planning_scope: <scope>` | 0.62.0 | The default. One Markdown file per artifact under `.engineering/planning/` and one evidence file per record under `.engineering/evidence/`; Git is the history. `store` may be omitted. |
| SQLite | `aep.project/1` | `store: { sqlite: <path> }` | — | 0.30.0 | One database file; the path is relative to `.engineering/`. |
| PostgreSQL | `aep.project/1` | `store: { postgres: <url> }` | — | 0.30.0 | A database reached by a libpq connection string or URL. |

## Removed

| Store | `version` | `store` selector | Removed in | Migration | Notes |
|---|---|---|---|---|---|
| Markdown journal layout | `aep.project/1` | `store: markdown` | 0.65.0 (next release) | `aep plan store migrate git --verify`, which the release that removes the layout still carries. | Markdown artifacts under `.engineering/planning/` plus a `journal.jsonl`; also what an `aep.project/1` file without `store` selected. |
| Hybrid | `aep.project/1` | `store: { hybrid: {…} }` | 0.65.0 (next release) | None. Re-create the plan in a Git-native, SQLite or PostgreSQL store. | A local store and a replica under a declared divergence policy. |
| Event-log stores | `aep.project/2 – aep.project/4` | `store: { eventlog: {…} }` | 0.62.0 | Install the build at commit `9c0f1da44429ff935fa0b2d743457945d51e1c51` (`cargo install --git https://github.com/beyond10x/aep --rev 9c0f1da44429ff935fa0b2d743457945d51e1c51 aep-cli`), then run `aep plan store migrate git --verify`. | Planning state kept in an Eventlog stream. |
{/* generated:planning-stores:end */}

[The Git-native planning store](../concepts/planning-store.md) describes the default store's
layout, and [migrating an older store](../guides/migrate-an-older-store.md) walks through a
migration.
