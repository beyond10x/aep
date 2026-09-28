---
title: How AEP fits together
sidebar_label: How it fits together
sidebar_position: 1
description: The two halves of AEP — the governed planning store and the governed-task engine — the documents both read, and the crates behind them.
---

# How AEP fits together

AEP has two halves. Both read the same YAML document tree, and both decide deterministically from
what they are given.

| Half | Question it answers | Where the state lives | Commands |
|---|---|---|---|
| **planning store** | what work exists, what status each item is at, and whether a move is legal and earned | `.engineering/planning/` and `.engineering/evidence/` in your repository | `aep plan artifact …`, `aep plan serve`, `aep plan workspace …` |
| **governed tasks** | on this task, which capabilities are allowed, what is owed, and whether the task is complete | a task file, an artifact manifest and evidence documents you pass in | `aep govern …`, `aep drive …`, `aep observe …` |

Most adopters start with the planning store. The engine half matters once an agent works on a task
under a profile and you want its permissions and its completion decided by rules, not by its own
report.

## The document tree

Every rule is a YAML document, validated before it is used. `project.yaml` names where the tree
comes from (`protocols:`): a directory, or a Git repository pinned to a 40-hex commit.

| Directory | Holds | Read by |
|---|---|---|
| `artifacts/lifecycles/` | one [lifecycle](./lifecycles.md) per artifact kind: statuses, legal moves, rung costs | the planning store |
| `artifacts/kinds/`, `artifacts/relations/`, `artifacts/templates/` | kind descriptions, the relation vocabulary, body templates | the planning store |
| `protocols/` | the vocabulary: capabilities, evidence kinds, verifiers, phases, observable facts | both |
| `principles/`, `profiles/`, `workflows/` | rules, bundles of rules, and state machines guarded by evidence | governed tasks |
| `drivers/` | step maps: what runs in each workflow state | `aep drive` |

The tree AEP ships covers software development (`adp/1`) and operations (`aop/1`) over a shared base
(`aep/1`). A project adds its own principles and profiles under `.engineering/principles/` and
`.engineering/profiles/`, and its own lifecycles in its own document tree.

## Properties both halves keep

- **Deterministic.** The same documents, evidence and supplied instant give the same answer. The
  clock is read at the command-line edge and passed in; no decision reads it.
- **Refusals change nothing.** A refused move or command writes no file.
- **Unknown is not false.** A fact nobody observed is `Unknown`, which never satisfies a guard and is
  reported apart from a fact observed to be wrong.
- **Default deny.** A capability no document grants is not granted, and a denial cannot be granted
  back by a later document.
- **Errors accumulate.** Validation reports every problem it finds, each with a stable code, not
  just the first.

## The crates

| Area | Crates | Responsibility |
|---|---|---|
| `crates/govern/` | `aep-domain`, `aep-engine` | the typed vocabulary; resolution, evaluation, authorization and transitions |
| `crates/plan/` | `aep-contract`, `aep-conformance`, `aep-client`, `aep-backend-*` | the storage contract, the suites a backend is held to, and the backends (Markdown/Git, memory, SQLite, PostgreSQL, Entity Runtime) |
| `crates/drive/` | `aep-driver-spec`, `aep-driver`, `aep-render` | step maps, the reference driver, and drawing a workflow or a run |
| `crates/observe/` | `trace-domain`, `trace-spec`, `aep-ess-evidence` | transcript normalization and checking, and the optional ESS report adapter |
| `crates/profile/` | `aep-profile-development`, `aep-profile-operations` | development and operations vocabulary |
| `crates/edge/` | `aep-schema`, `aep-project`, `aep-cli` | published JSON Schemas, project discovery and protocol-source acquisition, and the `aep` command |

A crate depends only on its own area and the areas below it: `edge` → `{profile, drive, observe}` →
`{govern, plan}` → `aep-domain`. Lifecycle moves are decided by `entity-core` from
[Entity Runtime](https://github.com/beyond10x/entity-runtime), an IO-free kernel AEP pins as a
dependency.

## What lives elsewhere

- **Agent plugins** for Claude Code and Codex are in
  [beyond10x/agentplugins](https://beyond10x.github.io/agentplugins/). This repository ships no
  plugin, and no AEP command picks one for you.
- **Model execution** (paid runs, native harness hooks, live evaluation) belongs to
  `metaharness aep drive`, which uses AEP's libraries. `aep drive` runs command and operator steps,
  and it refuses a step map with a model step before it starts a run.
- **Executable system specifications** belong to [ESS](https://github.com/beyond10x/ess), which has
  no AEP dependency. AEP reads ESS's standalone conformance report as evidence.

## Where to next

- [Artifacts and kinds](./artifacts.md), [lifecycles](./lifecycles.md) and
  [the planning store](./planning-store.md) cover the planning half.
- [Governed tasks](./governance.md) and [design principles](./design-principles.md) cover the
  engine half.
- [Evidence](./evidence.md) covers both.
