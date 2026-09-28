---
title: project.yaml
sidebar_position: 2
description: Every field of .engineering/project.yaml, the file that makes a repository an AEP project.
---

# `project.yaml`

`.engineering/project.yaml` makes a directory an AEP project. Every command looks for it from the
working directory upwards, up to twelve levels. `AEP_PROJECT_DIR` renames `.engineering`. Unknown
keys are refused.

```yaml
version: aep.project/5                 # the store format; see below
protocol: adp/1                        # the protocol the project runs under
profile: development.standard          # the profile whose rules apply
protocols: git+https://github.com/beyond10x/aep#88836a30f28ab2fddc3ab63d1ac54956973fa25e
planning_scope: shop                   # required by aep.project/5
store:
  git: {}                              # the Git-native store; the only store /5 accepts
summary: The web shop.                 # optional, for people
providers:                             # optional: link patterns for external references
  jira: https://tracker.example/browse/{key}
```

## Fields

| Field | Required | Default | Meaning |
|---|---|---|---|
| `version` | no | `aep.project/1` | `aep.project/5` for the Git-native store. `aep.project/2`–`/4` are refused with the migration path. |
| `protocol` | yes | | the protocol reference, such as `adp/1` |
| `profile` | yes | | the profile, such as `development.standard` |
| `protocols` | no | `..` | where the governing documents come from: a path relative to `.engineering/`, or a `git+ssh://`, `git+https://` or `git+file://` URL pinned to a 40-hex commit after `#` |
| `planning_scope` | with `/5` | | the store's name, 1–255 bytes; `reverse init` and `migrate git` set it to the repository directory's name |
| `store` | no | `git: {}` under `/5` | where the plan is kept; see below |
| `summary` | no | | one line for people; nothing reads it |
| `providers` | no | | a URL pattern per external system, each containing `{key}`; a pattern without `{key}` is refused |
| `artifacts` | no | `artifacts.yaml` | the artifact manifest for [governed tasks](../concepts/governance.md) |
| `task` | no | `task.yaml` | the task document `govern` and `drive` use when `--task` is absent |
| `state` | no | `state.yaml` | where execution state is kept |
| `principles` | no | `principles` | project-local principles, merged over the tree's |
| `profiles` | no | `profiles` | project-local profiles, merged over the tree's |
| `schemas` | no | `schemas` | project-owned JSON Schema contracts |

Paths other than `protocols` are relative to `.engineering/`, and absolute paths are refused, so the
repository can be cloned anywhere.

## `protocols`

| Form | Example | Notes |
|---|---|---|
| a path | `..`, `../governance` | a tree in the repository, or beside it; use this to add your own lifecycles |
| a pinned Git URL | `git+https://github.com/beyond10x/aep#<40-hex>` | fetched once into `~/.cache/aep/protocol-sources/` (or `AEP_CACHE_DIR`), verified against the commit, and read from there |

An unpinned Git URL is refused. A governing tree that can move without a commit in your repository
would be a gate whose meaning changes silently.

## `store`

| `version` | Accepted `store` | Plan lives in |
|---|---|---|
| `aep.project/5` | `git: {}` (or absent) | `.engineering/planning/` and `.engineering/evidence/`, see [the planning store](../concepts/planning-store.md) |
| `aep.project/1` | absent or `markdown` | `.engineering/planning/` plus `journal.jsonl`; migrate with `aep plan store migrate git --verify` |
| `aep.project/1` | `sqlite: <file>`, `postgres: <url>`, `hybrid: {…}` | a database backend, or Markdown with a replica under a divergence policy |

`planning_tenant` and `planning_identity` named event-log identities. Both are refused today.

[Planning stores](./planning-stores.md) lists every supported and removed backend, generated from
the code, with each removed store's migration path.

## Written by

- `aep plan reverse init --protocols … --profile … [--protocol adp/1] [--summary …]` writes a new
  `/5` file.
- `aep plan store migrate git` rewrites a `/1` file as `/5` and keeps every other key.

## Related files

`.engineering/workspace.yaml` (`version: aep.workspace/1`) lists the repositories that
[workspace](../concepts/workspaces.md) commands answer across. The JSON Schemas for both files are
published in
[`schemas/generated/`](https://github.com/beyond10x/aep/tree/main/schemas/generated) as
`project.schema.json` and `workspace.schema.json`.
