---
format: aep.planning-md/3
id: story:planning-scope-comes-from-the-repository
kind: story
status: draft
title: planning_scope comes from the repository, not the checkout directory
summary: migrate git and reverse init derive planning_scope from the origin remote or the primary checkout, never a linked worktree's directory name
relations:
- serves: vision:O2
revision: 1
---
# Story: `planning_scope` comes from the repository, not the checkout directory

## Outcome

A store migrated or adopted in a linked worktree records the repository's name as its
`planning_scope`, the same value a migration in the primary checkout would record. The name of the
directory a worktree tool chose for the checkout never reaches `.engineering/project.yaml`.

## Context

`default_planning_scope` (`crates/edge/aep-cli/src/store_command.rs:41`) returns the file name of
the directory holding `.engineering/`. That is the repository name only in a primary checkout. In a
managed worktree named `docs-system-aep-store-v5`, `aep plan store migrate git` (0.69.1) wrote that
name as the scope of the `docs-system` store, and it had to be corrected by hand. `aep plan reverse
init` (`crates/edge/aep-cli/src/reverse.rs:1374`) uses the same function and has the same defect.

## Acceptance

- `aep plan store migrate git` run in a linked worktree whose directory name differs from the
  repository writes `planning_scope: <repository>`; a test creates the primary checkout and a
  differently named linked worktree and asserts the written value. It fails on 0.69.1.
- `aep plan reverse init` in the same layout writes the same value.
- The repository name is, in order: the explicit `--planning-scope <name>` flag; the last path
  segment of the `origin` remote URL without `.git`; the directory holding the Git common directory
  when that directory is named `.git`; the directory holding `.engineering/` when no Git repository
  is found. Each source has a test.
- The text output names which source the scope came from.

## Out of scope

Changing the scope of a store that already has one. `planning_scope` stays required and is never
rewritten by a migration.
