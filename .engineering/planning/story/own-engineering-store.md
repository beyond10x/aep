---
format: aep.planning-md/1
id: story:own-engineering-store
kind: story
status: active
title: The repository's own .engineering/, holding this backlog
summary: A project.yaml pointing at this repository as its own protocol tree, and the real roadmap as artifacts the driver can evaluate a gate against.
owner: protocol
tags:
- dogfooding
- store
relations:
- decomposes: epic:reference-driver
revision: 4
---
# Story: The repository's own `.engineering/`, holding this backlog

## Outcome

Somebody who clones this repository and types `protocol artifact list` — with no flags, from anywhere
inside it — sees the actual roadmap, in the store the repository publishes, governed by the
lifecycles the repository publishes.

## Context

The wave-1 store was built and pointed at a fixture. The repository's own plan stayed in hand-written
wave pages that nothing validates, which is the protocol's methodology not applied to the protocol's
own backlog. It is also a prerequisite rather than a gesture: a driver with no store has no artifacts
to evaluate a gate against, so the store has to exist before a real story can be driven.

## Acceptance

- `protocol artifact list` run anywhere inside this repository with **no** `--store` answers from
  `.engineering/planning/` — through project discovery, not through a flag.
- `protocol artifact validate` is green over the store, and the command and its output are recorded
  rather than the claim.
- `project.yaml` names this repository as its own protocol tree, and the path is right for the reason
  the file states: paths are resolved against `.engineering`, so the tree is `..`.
- The store holds this wave's own stories, so the first thing the repository governs with it is the
  wave that built it.

## Re-scoped on evidence — 2026-08-28

The store is real and is the repository's own: 125 artifacts, 0 problems, `protocol artifact list`
from `crates/aep-domain/` with no `--store` answers 125 lines, exit 0. Two things are not true yet.

**1. `validate` is red from exactly the position this story promises.** With no `--store`,
`repository_root()` (`crates/protocol-cli/src/planning.rs:97-106`) returns the *current directory*
rather than the discovered project root, so `declared_members` (`:1113`) looks for
`.engineering/workspace.yaml` beside the cwd and misses it. Measured 2026-08-28:

```console
$ cd crates/aep-domain && protocol artifact validate
[undeclared_reference] artifacts.story:assemble-across-sources.relations[2] …
  entity-runtime/story:typed-references, which the manifest does not declare
exit 1
```

From the root the same command exits 0 and prints `valid`. Discovery finds the store from anywhere
and the manifest is resolved against the cwd, so *anywhere inside it* and *green* cannot both hold
today. The fix is one function: resolve the manifest against the discovered project root, plus a
test that runs `validate` from a subdirectory. Until then the acceptance line **the command and its
output are recorded rather than the claim** is recorded here, red.

**2. The Open Question's default was not taken.** *Does `artifact validate` join the project gate?*
— default **yes**. `Taskfile.yml` has 18 steps and none of them is `artifact validate`; `AGENTS.md`
§ *Gate* still says eighteen and does not list it. Taking the default is: the step, the AGENTS.md
row, in one change — and it cannot be taken before defect 1 is fixed, because the gate runs from the
repository root but a contributor does not.

Also recorded: the output this story cites (`harness-wave-4-governed-dogfood.md:77-91`) is over
**33** artifacts; the store is now **125**.

## Out of Scope

Migrating the wave pages into the store. The pages carry reasoning, which is what they are for; the
store carries state, which is what it is for. Duplicating one into the other creates two answers to
*what is the status of this*.

## Open Questions

Whether `artifact validate` joins the project gate. Decides: protocol owner. Default if nobody
answers: **yes** — it is local, clock-free and sub-second, the same argument that placed
`status-check` there — and `AGENTS.md` § *Gate* gains its row in the same change, because a gate whose
step list disagrees with the Taskfile is the drift invariant 1 exists to prevent.
