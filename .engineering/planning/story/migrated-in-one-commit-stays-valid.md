---
format: aep.planning-md/3
id: story:migrated-in-one-commit-stays-valid
kind: story
status: implemented
title: A /1 store committed for the first time together with its migration still validates
relations:
- serves: vision:O2
revision: 4
transitions:
- {from: "draft", to: "proposed", at: "2026-09-28T14:02:57Z", actor: "human:timo", revision: 2}
- {from: "proposed", to: "active", at: "2026-09-28T14:02:57Z", actor: "human:timo", revision: 3}
- {from: "active", to: "implemented", at: "2026-09-28T18:39:07Z", actor: "human:timo", revision: 4, decided_on: {"recorded":{"test_result":1}}}
---
## Problem

`validate` accepts a never-moved artifact whose status is not its kind's initial state only when
its Git history starts in an older format (`aep.planning-md/1` or `/2`)
(`crates/edge/aep-cli/src/planning/git_record.rs`, 0.64.0). A `/1` store whose files were first
committed in the same commit as `aep plan store migrate git` has no older version in history, so its
never-moved, non-initial artifacts are refused.

## Outcome

The migration records what it found for such an artifact (for example one imported transition from
the initial state, or a front-matter key naming the migrated status) so the rule no longer depends
on Git history.

## Acceptance

- A fixture: `/1` store, never committed, migrated and committed in one commit, `validate` → valid.
- A hand-edited status on an artifact born in `/5` is still refused.
