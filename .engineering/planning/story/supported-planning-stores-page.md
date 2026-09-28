---
format: aep.planning-md/3
id: story:supported-planning-stores-page
kind: story
status: implemented
title: The website lists the supported planning store backends, generated from the code
relations:
- serves: vision:O2
revision: 4
transitions:
- {from: "draft", to: "proposed", at: "2026-09-28T13:49:22Z", actor: "human:timo", revision: 2}
- {from: "proposed", to: "active", at: "2026-09-28T13:49:22Z", actor: "human:timo", revision: 3}
- {from: "active", to: "implemented", at: "2026-09-28T18:39:08Z", actor: "human:timo", revision: 4, decided_on: {"recorded":{"test_result":1}}}
---
## Problem

The public website does not say, in one place that is checked, which planning store backends a
release supports: the Git-native `aep.project/5` store, SQLite and Postgres, and which ones are
removed with their migration path (`/1` journal layout, hybrid, the `/2`–`/4` event-log stores).
Pages describe stores in prose, and nothing fails when the code and the prose disagree.

## Outcome

- A catalog of planning store backends lives in code (kind, selector syntax, project version,
  status: supported / deprecated / removed, since/until release, migration path), and a test
  fails when a store variant has no catalog entry.
- `cargo xtask` generates the website page `reference/planning-stores` from it; the gate fails while
  the page lags the code, so the next release publishes an accurate page.

## Acceptance

- Removing or adding a store variant without updating the catalog fails `cargo test`.
- Editing the catalog without regenerating the page fails `task check`.
- The page lists every supported and removed store with its selector and migration path.
