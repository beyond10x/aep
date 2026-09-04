---
slug: /
title: Introduction
sidebar_label: Introduction
sidebar_position: 1
description: Typed, executable rules for agent-performed engineering work.
---

# Agentic Engineering Protocol

AEP is a Rust library, a CLI, and a set of typed document formats for running engineering work
under rules a program can execute.

You write principles such as “tests must pass,” “verification must be independent,” or “production
changes require approval” as validated data. A deterministic engine resolves those rules against a
task and its evidence. The result says what is permitted, what is owed, and why progress is blocked.

## One substrate, two profiles

| Profile | Governs |
|---|---|
| AEP | artifacts, planning, workflows, evidence, permissions, approvals, audit, and completion |
| ADP | specification, decomposition, design, tests, implementation, and review |
| AOP | operational plans, controlled change, verification, rollback, and incidents |

ADP and AOP use AEP's planning and evidence substrate; they do not implement separate engines.

## The command

`aep` is canonical. `protocol` is an exact compatibility alias for existing automation.

```bash
aep govern validate
aep plan artifact board
aep govern explain --action production.write
aep observe trace check --spec expectations.trace.yaml --transcript run.jsonl
```

The CLI also includes a reference driver. It proves the harness contract has a real caller but does
not choose a model, credential, endpoint, or plugin. Harness-specific skills and agents are not in
this repository. They come from the sibling repository `beyond10x/agentplugins`, which publishes the
`beyond10x` marketplace, and they are supplied explicitly.

In Claude Code that is `/plugin marketplace add beyond10x/agentplugins` followed by
`/plugin install aep-plan@beyond10x` (also `aep-drive@beyond10x` and `ess-specify@beyond10x`). In Codex,
add the same GitHub repository as a marketplace from the Plugins surface. The
[agent plugins install page](https://beyond10x.github.io/agentplugins/) carries the current list.

## What lives elsewhere

Executable System Specification tooling is a standalone project. ESS publishes its own conformance
report and has no AEP dependency. The optional AEP-side adapter converts that report into
`ess_conformance` evidence without core AEP compiling against ESS modeling types.

## Continue

| Goal | Read |
|---|---|
| run the CLI | [Getting started](./getting-started.md) |
| understand the architecture | [Architecture overview](./concepts/overview.md) |
| govern a task | [Govern a task](./guides/govern-a-task.md) |
| integrate a harness | [Integrate an agent harness](./guides/integrate-a-harness.md) |
| install the agent plugins | [beyond10x/agentplugins](https://beyond10x.github.io/agentplugins/) |
| inspect a complete example | [A governed task, end to end](./examples/governed-task.md) |
| understand evidence | [Evidence](./concepts/evidence.md) |
| see current delivery state | [Where this stands](./status/where-this-stands.md) |

Source: [github.com/beyond10x/aep](https://github.com/beyond10x/aep).
