---
slug: /
title: Overview
sidebar_label: Overview
sidebar_position: 1
description: What AEP is, what it decides, and where to start.
---

# AEP

AEP (Agentic Engineering Protocol) keeps a repository's engineering plan as Markdown files that a
program governs. It is one command, `aep`, and a set of YAML documents that say what the command
may do.

```text
.engineering/
  project.yaml                          which rules apply, and where they come from
  planning/story/pay-by-card.md         one artifact: YAML front matter + Markdown body
  evidence/story/pay-by-card/*.json     what was observed about it, one file per record
```

## What it decides

- **Which moves are legal.** Each artifact kind has a lifecycle, a ladder of statuses declared in
  YAML. `aep plan artifact move` walks it, and a refused move lists the statuses you *can* move to.
- **Which moves are earned.** A rung can require evidence: `implemented` on a story needs at least
  one `test_result`. `move` counts the evidence records held for that artifact, and refuses and
  names the missing kind when the count is short.
- **What a plan says about itself.** `validate` checks every file, edge and status, and checks that
  each artifact's recorded transitions are a continuous walk that ends at its status. `explain`
  shows what each move rested on and what the next rung costs.
- **What an agent may do on a governed task.** Principles and profiles resolve into capabilities and
  obligations. The engine answers *allowed*, *denied* or *needs approval*, and names the rule behind
  the answer. This is the [governed-task](./concepts/governance.md) half of AEP.

The same answers hold for a person at a terminal, a coding agent and a CI job. AEP does not run
models, hold credentials or choose plugins.

## The shape of it

| Area | Command | Answers |
|---|---|---|
| plan | `aep plan …` | what work exists, what state it is in, what it needs next |
| govern | `aep govern …` | what the rule documents say, and what they decide for a task |
| drive | `aep drive …` | walking a workflow step by step, with the engine deciding each transition |
| observe | `aep observe …` | what an agent run or a check actually did, turned into evidence |

`aep doctor` reports whether a checkout is in a state the other commands will accept.

## Start

| If you want to | Read |
|---|---|
| try it in ten minutes | [Quickstart](./getting-started.md) |
| understand the model | [How AEP fits together](./concepts/overview.md) |
| plan real work | [Plan work](./guides/plan-work.md) |
| gate a status on a check | [Gate a move on evidence](./guides/gate-a-move-on-evidence.md) |
| upgrade a store written by an older release | [Migrate an older store](./guides/migrate-an-older-store.md) |
| look up a command | [CLI reference](./reference/cli.md) |

Agent plugins for Claude Code and Codex are published separately, from
[beyond10x/agentplugins](https://beyond10x.github.io/agentplugins/). They work through this CLI.

Source: [github.com/beyond10x/aep](https://github.com/beyond10x/aep).
